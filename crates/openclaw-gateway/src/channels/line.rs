use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result};
use serde_json::json;
use std::sync::{Arc, Weak};

pub struct LineChannel {
    state: Weak<AppState>,
    channel_token: String,
    channel_secret: String,
    client: reqwest::Client,
}

impl LineChannel {
    pub fn new(channel_token: String, channel_secret: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            channel_token,
            channel_secret,
            client: reqwest::Client::new(),
        }
    }

    pub async fn handle_webhook(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        if let Some(events) = body["events"].as_array() {
            for event in events {
                if event["type"].as_str() == Some("message") {
                    if let Some(message) = event["message"]["text"].as_str() {
                        let user_id = event["source"]["userId"].as_str().unwrap_or("unknown");

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
                            let session = state.sessions.get_or_create(ChannelKind::Api, user_id);
                            let _ = state.sessions.add_message(&session.id, msg);
                            crate::agent::run_agent_cycle(state, session.id).await;
                        }
                    }
                }
            }
        }
        Ok(json!({}))
    }
}

#[async_trait]
impl Channel for LineChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "line"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = "https://api.line.me/v2/bot/message/push";
        let body = json!({
            "to": peer_id,
            "messages": [{
                "type": "text",
                "text": content
            }]
        });

        let _ = self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.channel_token))
            .header("Content-Type", "application/json")
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
