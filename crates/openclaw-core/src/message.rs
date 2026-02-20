use crate::channel::ChannelKind;
use crate::config::AppConfig;
use crate::session::{ChatMessage, Session};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    Ping {
        timestamp: i64,
    },
    Pong {
        timestamp: i64,
    },
    Subscribe {
        channels: Vec<String>,
    },
    SendMessage {
        session_id: Option<Uuid>,
        content: String,
        channel: Option<ChannelKind>,
        peer_id: Option<String>,
    },
    ChatCommand {
        session_id: Uuid,
        command: String,
        args: Option<String>,
    },
    GetSessions,
    GetConfig,
    NewMessage {
        session_id: Uuid,
        message: ChatMessage,
    },
    SessionList {
        sessions: Vec<Session>,
    },
    ConfigResponse {
        config: AppConfig,
    },
    CommandResult {
        session_id: Uuid,
        command: String,
        result: String,
    },
    Error {
        code: String,
        message: String,
    },
    AgentThinking {
        session_id: Uuid,
    },
    AgentResponse {
        session_id: Uuid,
        content: String,
        done: bool,
    },
    PresenceUpdate {
        channel: ChannelKind,
        status: PresenceStatus,
    },
    CanvasUpdate {
        session_id: Uuid,
        id: String,
        content: String,
        language: Option<String>,
        title: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PresenceStatus {
    Online,
    Offline,
    Typing,
}
