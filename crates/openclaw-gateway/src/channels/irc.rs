use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result};
use std::sync::{Arc, Weak};

pub struct IrcChannel {
    state: Weak<AppState>,
    server: String,
    nick: String,
    channels: Vec<String>,
}

impl IrcChannel {
    pub fn new(server: String, nick: String, channels: Vec<String>, state: Weak<AppState>) -> Self {
        Self {
            state,
            server,
            nick,
            channels,
        }
    }
}

#[async_trait]
impl Channel for IrcChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "irc"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        tracing::info!("IRC send to {}: {}", peer_id, content);
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        tracing::info!("IRC connecting to {} as {}", self.server, self.nick);
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}
