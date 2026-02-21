use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result};
use serde_json::json;
use std::sync::{Arc, Weak};

pub struct TeamsChannel {
    state: Weak<AppState>,
    tenant_id: String,
    client_id: String,
    client_secret: String,
    client: reqwest::Client,
}

impl TeamsChannel {
    pub fn new(tenant_id: String, client_id: String, client_secret: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            tenant_id,
            client_id,
            client_secret,
            client: reqwest::Client::new(),
        }
    }

    pub async fn handle_webhook(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        let activity_type = body["type"].as_str().unwrap_or("");

        match activity_type {
            "message" => {
                if let Some(text) = body["text"].as_str() {
                    let from_id = body["from"]["id"].as_str().unwrap_or("unknown");

                    let msg = ChatMessage {
                        id: uuid::Uuid::new_v4(),
                        role: Role::User,
                        content: text.to_string(),
                        images: vec![],
                        tool_calls: vec![],
                        tool_result: None,
                        timestamp: chrono::Utc::now(),
                        channel: ChannelKind::Api,
                    };

                    if let Some(state) = self.state.upgrade() {
                        let session = state.sessions.get_or_create(ChannelKind::Api, from_id);
                        let _ = state.sessions.add_message(&session.id, msg);
                        crate::agent::run_agent_cycle(state, session.id).await;
                    }
                }
            }
            "invoke" => {
                if body["name"].as_str() == Some("adaptiveCard/action") {
                    tracing::info!("Teams adaptive card action received");
                }
            }
            _ => {}
        }

        Ok(json!({}))
    }

    async fn get_access_token(&self) -> Result<String> {
        let url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );

        let body = vec![
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("scope", "https://api.botframework.com/.default"),
        ];

        let res = self.client
            .post(&url)
            .form(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let json: serde_json::Value = res.json().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        json["access_token"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| openclaw_core::OpenClawError::Provider("No access token".to_string()))
    }
}

#[async_trait]
impl Channel for TeamsChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "teams"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let service_url = "https://smba.trafficmanager.net/amer/";
        let url = format!("{}v3/conversations/{}/activities", service_url, peer_id);

        let body = json!({
            "type": "message",
            "text": content
        });

        let _ = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
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
