use crate::channel::ChannelKind;
use crate::error::{OpenClawError, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub channel: ChannelKind,
    pub peer_id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: Role,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub channel: ChannelKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
}

pub struct SessionStore {
    sessions: DashMap<Uuid, Session>,
    peer_index: DashMap<(ChannelKind, String), Uuid>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            peer_index: DashMap::new(),
        }
    }

    pub fn create(&self, channel: ChannelKind, peer_id: String) -> Session {
        let now = Utc::now();
        let session = Session {
            id: Uuid::new_v4(),
            channel: channel.clone(),
            peer_id: peer_id.clone(),
            messages: vec![],
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        };
        self.peer_index
            .insert((channel, peer_id), session.id);
        self.sessions.insert(session.id, session.clone());
        session
    }

    pub fn get(&self, id: &Uuid) -> Option<Session> {
        self.sessions.get(id).map(|s| s.clone())
    }

    pub fn get_or_create(&self, channel: ChannelKind, peer_id: &str) -> Session {
        let key = (channel.clone(), peer_id.to_string());
        if let Some(id) = self.peer_index.get(&key) {
            if let Some(session) = self.sessions.get(&id) {
                return session.clone();
            }
        }
        self.create(channel, peer_id.to_string())
    }

    pub fn add_message(&self, session_id: &Uuid, msg: ChatMessage) -> Result<()> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| OpenClawError::Session(format!("session not found: {session_id}")))?;
        session.updated_at = Utc::now();
        session.messages.push(msg);
        Ok(())
    }

    pub fn list(&self) -> Vec<Session> {
        self.sessions.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
