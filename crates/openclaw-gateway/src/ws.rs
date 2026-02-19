use crate::state::AppState;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use openclaw_core::{
    session::{ChatMessage, Role},
    ChannelKind, WsMessage,
};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let client_id = Uuid::new_v4();
    let (tx, _rx) = broadcast::channel::<String>(64);
    state.ws_clients.insert(client_id, tx.clone());
    info!(client_id = %client_id, "ws client connected");

    let (mut ws_sink, mut ws_stream) = socket.split();
    let mut broadcast_rx = tx.subscribe();

    let sink_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            if ws_sink.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_stream.next().await {
        match msg {
            Message::Text(text) => {
                let response = handle_text_message(&text, &state, client_id).await;
                if let Some(resp) = response {
                    let _ = tx.send(resp);
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    sink_task.abort();
    state.ws_clients.remove(&client_id);
    state.subscriptions.remove(&client_id);
    info!(client_id = %client_id, "ws client disconnected");
}

async fn handle_text_message(
    text: &str,
    state: &Arc<AppState>,
    client_id: Uuid,
) -> Option<String> {
    let msg: WsMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            warn!(error = %e, "failed to parse ws message");
            let err = WsMessage::Error {
                code: "parse_error".to_string(),
                message: e.to_string(),
            };
            return serde_json::to_string(&err).ok();
        }
    };

    let response = match msg {
        WsMessage::Ping { timestamp } => WsMessage::Pong { timestamp },
        WsMessage::GetSessions => WsMessage::SessionList {
            sessions: state.sessions.list(),
        },
        WsMessage::GetConfig => WsMessage::ConfigResponse {
            config: state.config.clone(),
        },
        WsMessage::Subscribe { channels } => {
            for ch in &channels {
                if let Ok(uuid) = ch.parse::<Uuid>() {
                    state.subscribe(client_id, uuid);
                }
            }
            return None;
        }
        WsMessage::SendMessage {
            session_id,
            content,
            channel,
            peer_id,
        } => {
            let ch = channel.unwrap_or(ChannelKind::Api);
            let pid = peer_id.unwrap_or_else(|| "cli".to_string());
            let session = if let Some(sid) = session_id {
                state
                    .sessions
                    .get(&sid)
                    .unwrap_or_else(|| state.sessions.create(ch.clone(), pid.clone()))
            } else {
                state.sessions.get_or_create(ch.clone(), &pid)
            };

            state.subscribe(client_id, session.id);

            let user_msg = ChatMessage {
                id: Uuid::new_v4(),
                role: Role::User,
                content: content.clone(),
                timestamp: chrono::Utc::now(),
                channel: ch.clone(),
            };
            let _ = state.sessions.add_message(&session.id, user_msg.clone());

            let new_msg = WsMessage::NewMessage {
                session_id: session.id,
                message: user_msg,
            };
            if let Ok(json) = serde_json::to_string(&new_msg) {
                state.send_to_subscribers(&session.id, &json);
            }

            let sid = session.id;
            let spawn_state = Arc::clone(state);
            let model = state.config.models.default_model.clone();
            let system_prompt = state.config.agent.system_prompt.clone();
            let max_tokens = state.config.agent.max_tokens;

            tokio::spawn(async move {
                if let Ok(json) = serde_json::to_string(&WsMessage::AgentThinking {
                    session_id: sid,
                }) {
                    spawn_state.send_to_subscribers(&sid, &json);
                }

                let session_messages = spawn_state
                    .sessions
                    .get(&sid)
                    .map(|s| s.messages.clone())
                    .unwrap_or_default();

                let response_content = match &spawn_state.provider {
                    Some(provider) => {
                        let (token_tx, mut token_rx) = mpsc::channel::<String>(256);

                        let stream_state = Arc::clone(&spawn_state);
                        let stream_sid = sid;
                        let stream_task = tokio::spawn(async move {
                            while let Some(token) = token_rx.recv().await {
                                if let Ok(json) =
                                    serde_json::to_string(&WsMessage::AgentResponse {
                                        session_id: stream_sid,
                                        content: token,
                                        done: false,
                                    })
                                {
                                    stream_state.send_to_subscribers(&stream_sid, &json);
                                }
                            }
                        });

                        let result = provider
                            .stream_chat(
                                &session_messages,
                                system_prompt.as_deref(),
                                &model,
                                max_tokens,
                                token_tx,
                            )
                            .await;

                        let _ = stream_task.await;

                        match result {
                            Ok(full) => full,
                            Err(e) => {
                                error!(error = %e, "provider error");
                                if let Ok(json) = serde_json::to_string(&WsMessage::Error {
                                    code: "provider_error".to_string(),
                                    message: e.to_string(),
                                }) {
                                    spawn_state.send_to_subscribers(&sid, &json);
                                }
                                return;
                            }
                        }
                    }
                    None => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        format!(
                            "I received your message: \"{content}\". \
                             No AI provider configured — run `openclaw onboard` to set one up."
                        )
                    }
                };

                let assistant_msg = ChatMessage {
                    id: Uuid::new_v4(),
                    role: Role::Assistant,
                    content: response_content.clone(),
                    timestamp: chrono::Utc::now(),
                    channel: ch,
                };
                let _ = spawn_state.sessions.add_message(&sid, assistant_msg);

                if let Ok(json) = serde_json::to_string(&WsMessage::AgentResponse {
                    session_id: sid,
                    content: response_content,
                    done: true,
                }) {
                    spawn_state.send_to_subscribers(&sid, &json);
                }
            });

            return None;
        }
        _ => WsMessage::Error {
            code: "unsupported".to_string(),
            message: "message type not handled by server".to_string(),
        },
    };

    serde_json::to_string(&response).ok()
}
