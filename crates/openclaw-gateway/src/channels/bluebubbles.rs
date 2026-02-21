use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct BlueBubblesChannel {
    _state: Weak<AppState>,
    api_url: String,
    password: String,
    client: reqwest::Client,
}

impl BlueBubblesChannel {
    pub fn new(api_url: String, password: String, state: Weak<AppState>) -> Self {
        Self {
            _state: state,
            api_url,
            password,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for BlueBubblesChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "bluebubbles"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/api/v1/message/text", self.api_url);
        let body = json!({
            "chatGuid": peer_id,
            "message": content,
            "method": "private-api"
        });

        let _ = self.client.post(&url)
            .query(&[("password", &self.password)])
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

pub async fn handle_bluebubbles_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    if let Some(data) = body.get("data") {
        let guid = data["chatGuid"].as_str().unwrap_or_default();
        let text = data["text"].as_str().unwrap_or_default();
        let from_me = data["isFromMe"].as_bool().unwrap_or(false);

        if !guid.is_empty() && !text.is_empty() && !from_me {
            let kind = ChannelKind::Api; 
            let session = state.sessions.get_or_create(kind.clone(), guid);
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
    Ok(())
}
