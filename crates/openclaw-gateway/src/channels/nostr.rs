use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result};
use serde_json::json;
use std::sync::{Arc, Weak};

pub struct NostrChannel {
    state: Weak<AppState>,
    private_key: String,
    relays: Vec<String>,
}

impl NostrChannel {
    pub fn new(private_key: String, relays: Vec<String>, state: Weak<AppState>) -> Self {
        Self {
            state,
            private_key,
            relays,
        }
    }

    pub async fn handle_event(&self, event: serde_json::Value) -> Result<()> {
        if event["kind"].as_u64() == Some(4) {
            if let Some(content) = event["content"].as_str() {
                let pubkey = event["pubkey"].as_str().unwrap_or("unknown");

                let msg = ChatMessage {
                    id: uuid::Uuid::new_v4(),
                    role: Role::User,
                    content: content.to_string(),
                    images: vec![],
                    tool_calls: vec![],
                    tool_result: None,
                    timestamp: chrono::Utc::now(),
                    channel: ChannelKind::Api,
                };

                if let Some(state) = self.state.upgrade() {
                    let session = state.sessions.get_or_create(ChannelKind::Api, pubkey);
                    let _ = state.sessions.add_message(&session.id, msg);
                    crate::agent::run_agent_cycle(state, session.id).await;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Channel for NostrChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "nostr"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        tracing::info!("Nostr send to {}: {}", peer_id, content);
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        tracing::info!("Nostr connecting to relays: {:?}", self.relays);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}
