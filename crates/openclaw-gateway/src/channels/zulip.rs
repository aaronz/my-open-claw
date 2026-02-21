use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct ZulipChannel {
    _state: Weak<AppState>,
    site_url: String,
    email: String,
    api_key: String,
    client: reqwest::Client,
}

impl ZulipChannel {
    pub fn new(site_url: String, email: String, api_key: String, state: Weak<AppState>) -> Self {
        Self {
            _state: state,
            site_url,
            email,
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for ZulipChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "zulip"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/api/v1/messages", self.site_url);
        
        let mut params = std::collections::HashMap::new();
        params.insert("type", "private");
        params.insert("to", peer_id);
        params.insert("content", content);

        let _ = self.client.post(&url)
            .basic_auth(&self.email, Some(&self.api_key))
            .form(&params)
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

pub async fn handle_zulip_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    if let Some(message) = body.get("message") {
        let sender_email = message["sender_email"].as_str().unwrap_or_default();
        let text = message["content"].as_str().unwrap_or_default();
        
        if !sender_email.is_empty() && !text.is_empty() {
            let kind = ChannelKind::Api;
            let session = state.sessions.get_or_create(kind.clone(), sender_email);
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
