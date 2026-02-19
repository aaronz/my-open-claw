use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Telegram,
    Discord,
    Slack,
    #[serde(rename = "whatsapp")]
    WhatsApp,
    Signal,
    #[serde(rename = "webchat")]
    WebChat,
    Cli,
    Api,
}

impl std::fmt::Display for ChannelKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Telegram => write!(f, "telegram"),
            Self::Discord => write!(f, "discord"),
            Self::Slack => write!(f, "slack"),
            Self::WhatsApp => write!(f, "whatsapp"),
            Self::Signal => write!(f, "signal"),
            Self::WebChat => write!(f, "webchat"),
            Self::Cli => write!(f, "cli"),
            Self::Api => write!(f, "api"),
        }
    }
}

#[async_trait]
pub trait Channel: Send + Sync {
    fn kind(&self) -> ChannelKind;
    fn name(&self) -> &str;
    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()>;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}
