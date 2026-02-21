use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct SignalChannel {
    state: Weak<AppState>,
    api_url: String, 
}

impl SignalChannel {
    pub fn new(api_url: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            api_url,
        }
    }
}

#[async_trait]
impl Channel for SignalChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Signal
    }

    fn name(&self) -> &str {
        "signal"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let client = reqwest::Client::new();
        let body = json!({
            "message": content,
            "number": peer_id,
            "recipients": [peer_id]
        });

        let _ = client.post(format!("{}/v1/send", self.api_url))
            .json(&body)
            .send()
            .await;

        Ok(())
    }

    async fn start(&self) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn handle_signal_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    if let Some(envelope) = body.get("envelope") {
        let source = envelope["source"].as_str().unwrap_or_default();
        let message = envelope.get("dataMessage");
        
        if let Some(msg) = message {
            let text = msg["message"].as_str().unwrap_or_default();
            if !source.is_empty() && !text.is_empty() {
                let kind = ChannelKind::Signal;
                let session = state.sessions.get_or_create(kind.clone(), source);
                let session_id = session.id;
                drop(session);

                let user_msg = ChatMessage {
                    id: Uuid::new_v4(),
                    role: Role::User,
                    content: text.to_string(),
                    timestamp: chrono::Utc::now(),
                    channel: kind.clone(),
                    images: vec![],
                    tool_calls: vec![],
                    tool_result: None,
                };
                let _ = state.sessions.add_message(&session_id, user_msg.clone());

                let new_msg = WsMessage::NewMessage {
                    session_id,
                    message: user_msg,
                };
                if let Ok(json) = serde_json::to_string(&new_msg) {
                    state.broadcast(&json);
                }

                let spawn_state = Arc::clone(&state);
                tokio::spawn(async move {
                    run_agent_cycle(spawn_state, session_id).await;
                });
            }
        }
    }
    Ok(())
}
