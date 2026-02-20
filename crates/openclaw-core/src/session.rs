use crate::channel::ChannelKind;
use crate::error::{OpenClawError, Result};
use crate::provider::{ToolCall, ToolResult};
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
    pub timestamp: DateTime<Utc>,
    pub channel: ChannelKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

pub struct SessionStore {
    sessions: DashMap<Uuid, Session>,
    peer_index: DashMap<(ChannelKind, String), Uuid>,
    data_dir: Option<std::path::PathBuf>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            peer_index: DashMap::new(),
            data_dir: None,
        }
    }

    pub fn with_persistence(data_dir: std::path::PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let store = Self {
            sessions: DashMap::new(),
            peer_index: DashMap::new(),
            data_dir: Some(data_dir),
        };
        store.load_all()?;
        Ok(store)
    }

    fn load_all(&self) -> Result<()> {
        let dir = match &self.data_dir {
            Some(d) => d,
            None => return Ok(()),
        };
        if !dir.exists() {
            return Ok(());
        }
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        self.peer_index
                            .insert((session.channel.clone(), session.peer_id.clone()), session.id);
                        self.sessions.insert(session.id, session);
                    }
                }
            }
        }
        Ok(())
    }

    fn persist(&self, session_id: &Uuid) {
        if let (Some(dir), Some(session)) = (&self.data_dir, self.sessions.get(session_id)) {
            let path = dir.join(format!("{}.json", session_id));
            if let Ok(json) = serde_json::to_string_pretty(session.value()) {
                let _ = std::fs::write(path, json);
            }
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
        self.persist(&session.id);
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
        drop(session);
        self.persist(session_id);
        Ok(())
    }

    pub fn list(&self) -> Vec<Session> {
        self.sessions.iter().map(|r| r.value().clone()).collect()
    }

    pub fn reset(&self, session_id: &Uuid) -> Result<()> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| OpenClawError::Session(format!("session not found: {session_id}")))?;
        session.messages.clear();
        session.updated_at = Utc::now();
        drop(session);
        self.persist(session_id);
        Ok(())
    }

    pub fn replace_messages(&self, session_id: &Uuid, messages: Vec<ChatMessage>) -> Result<()> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| OpenClawError::Session(format!("session not found: {session_id}")))?;
        session.messages = messages;
        session.updated_at = Utc::now();
        drop(session);
        self.persist(session_id);
        Ok(())
    }

    pub fn compact(&self, session_id: &Uuid, summarized_count: usize, summary: ChatMessage) -> Result<()> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| OpenClawError::Session(format!("session not found: {session_id}")))?;

        if session.messages.len() < summarized_count {
            return Ok(()); // Messages removed, skip compaction
        }

        let mut new_msgs = Vec::with_capacity(session.messages.len() - summarized_count + 1);
        new_msgs.push(summary);
        
        for m in session.messages.iter().skip(summarized_count) {
            new_msgs.push(m.clone());
        }

        session.messages = new_msgs;
        session.updated_at = Utc::now();
        drop(session);
        self.persist(session_id);
        Ok(())
    }

    pub fn update_metadata(&self, session_id: &Uuid, key: String, value: serde_json::Value) -> Result<()> {
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| OpenClawError::Session(format!("session not found: {session_id}")))?;
        session.metadata.insert(key, value);
        session.updated_at = Utc::now();
        drop(session);
        self.persist(session_id);
        Ok(())
    }

    pub fn remove(&self, session_id: &Uuid) {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            self.peer_index
                .remove(&(session.channel.clone(), session.peer_id.clone()));
            if let Some(dir) = &self.data_dir {
                let path = dir.join(format!("{}.json", session_id));
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}
