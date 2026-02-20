use async_trait::async_trait;
use openclaw_core::{Channel, ChannelKind, Result};
use std::sync::Weak;
use crate::state::AppState;

pub struct DiscordChannel {
    token: String,
    client: reqwest::Client,
    #[allow(dead_code)] // Reserved for future inbound support
    state: Weak<AppState>,
}

impl DiscordChannel {
    pub fn new(token: String, state: Weak<AppState>) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
            state,
        }
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Discord
    }

    fn name(&self) -> &str {
        "discord"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("https://discord.com/api/v10/channels/{}/messages", peer_id);
        let body = serde_json::json!({
            "content": content
        });
        
        let res = self.client.post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
            
        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("Discord error: {}", err)));
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        // TODO: Implement WebSocket Gateway connection for inbound messages
        tracing::info!("Discord channel started (outbound only)");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}
