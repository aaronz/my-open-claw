use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct GotifyChannel {
    _state: Weak<AppState>,
    api_url: String,
    token: String,
    client: reqwest::Client,
}

impl GotifyChannel {
    pub fn new(api_url: String, token: String, state: Weak<AppState>) -> Self {
        Self {
            _state: state,
            api_url,
            token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for GotifyChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "gotify"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/message?token={}", self.api_url, self.token);
        
        let body = json!({
            "message": content,
            "title": "OpenClaw Notification",
            "priority": 5
        });

        let _ = self.client.post(&url)
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
