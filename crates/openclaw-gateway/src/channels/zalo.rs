use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};

pub struct ZaloChannel {
    state: Weak<AppState>,
    oa_id: String,
    secret_key: String,
    client: reqwest::Client,
}

impl ZaloChannel {
    pub fn new(oa_id: String, secret_key: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            oa_id,
            secret_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn handle_webhook(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        let event_name = body["event_name"].as_str().unwrap_or("");

        match event_name {
            "user_send_text" => {
                if let Some(sender_id) = body["sender"]["id"].as_str() {
                    if let Some(message) = body["message"]["text"].as_str() {
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
                        }
                    }
                }
                Ok(json!({"code": 0, "message": "Success"}))
            }
            _ => Ok(json!({"code": 0, "message": "Event ignored"})),
        }
    }

    fn generate_access_token(&self) -> String {
        self.secret_key.clone()
    }
}

#[async_trait]
impl Channel for ZaloChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "zalo"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!(
            "https://business.openapi.zalo.me/v3.0/oa/message/text",
        );

        let body = json!({
            "recipient": {
                "user_id": peer_id
            },
            "message": {
                "text": content
            }
        });

        let _ = self.client
            .post(&url)
            .header("access_token", self.generate_access_token())
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
