use crate::agent::run_agent_cycle;
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
use tokio::sync::broadcast;
use tracing::{info, warn};
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
        WsMessage::ChatCommand {
            session_id,
            command,
            args,
        } => {
            let result = handle_chat_command(state, &session_id, &command, args.as_deref()).await;
            return serde_json::to_string(&WsMessage::CommandResult {
                session_id,
                command,
                result,
            })
            .ok();
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
                tool_calls: vec![],
                tool_result: None,
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

            tokio::spawn(async move {
                run_agent_cycle(spawn_state, sid).await;
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

async fn handle_chat_command(
    state: &Arc<AppState>,
    session_id: &Uuid,
    command: &str,
    args: Option<&str>,
) -> String {
    match command {
        "new" | "reset" => {
            match state.sessions.reset(session_id) {
                Ok(()) => "Session reset.".to_string(),
                Err(e) => format!("Failed: {e}"),
            }
        }
        "status" => {
            match state.sessions.get(session_id) {
                Some(session) => {
                    let msg_count = session.messages.len();
                    let model = &state.config.models.default_model;
                    format!(
                        "Model: {model} | Messages: {msg_count} | Thinking: {:?}",
                        state.config.agent.thinking
                    )
                }
                None => "Session not found.".to_string(),
            }
        }
        "think" => {
            let level = args.unwrap_or("medium");
            format!(
                "Thinking level set to: {level} (per-session override not yet supported, config-level is {:?})",
                state.config.agent.thinking
            )
        }
        "compact" => match crate::agent::compact_session(state.clone(), *session_id).await {
            Ok(msg) => msg,
            Err(e) => format!("Compaction failed: {}", e),
        },
        "verbose" => {
            let on = args.map(|a| a == "on").unwrap_or(false);
            format!("Verbose mode: {}", if on { "on" } else { "off" })
        }
        "usage" => {
            let mode = args.unwrap_or("tokens");
            format!("Usage mode: {mode}")
        }
        _ => format!("Unknown command: /{command}"),
    }
}
