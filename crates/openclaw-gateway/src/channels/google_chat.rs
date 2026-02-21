use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};

pub struct GoogleChatChannel {
    state: Weak<AppState>,
    webhook_url: String,
    client: reqwest::Client,
}

impl GoogleChatChannel {
    pub fn new(webhook_url: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            webhook_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn handle_webhook(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        let event_type = body["type"].as_str().unwrap_or("");

        match event_type {
            "MESSAGE" | "ADDED_TO_SPACE" => {
                if let Some(message) = body["message"]["text"].as_str() {
                    let sender_id = body["user"]["name"]
                        .as_str()
                        .unwrap_or("unknown");

                    let msg = ChatMessage {
                        id: uuid::Uuid::new_v4(),
                        role: Role::User,
                        content: message.to_string(),
                        images: vec![],
                        tool_calls: vec![],
                        tool_result: None,
                        timestamp: chrono::Utc::now(),
                        channel: ChannelKind::Api,
                    };

                    if let Some(state) = self.state.upgrade() {
                        let session = state.sessions.get_or_create(ChannelKind::Api, sender_id);
                        let _ = state.sessions.add_message(&session.id, msg);

                        crate::agent::run_agent_cycle(
                            state.clone(),
                            session.id,
                        ).await;
                        
                        return Ok(json!({"text": {"text": ["Message received, processing..."]}}));
                    }
                }
                Ok(json!({"text": {"text": ["Received"]}}))
            }
            _ => Ok(json!({})),
        }
    }
}

#[async_trait]
impl Channel for GoogleChatChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "google_chat"
    }

    async fn send_message(&self, _peer_id: &str, content: &str) -> Result<()> {
        let body = json!({
            "text": content
        });

        let _ = self.client
            .post(&self.webhook_url)
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
