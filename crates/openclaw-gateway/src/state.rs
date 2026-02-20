use chrono::{DateTime, Utc};
use dashmap::DashMap;
use openclaw_core::provider::Provider;
use openclaw_core::{AppConfig, Channel, ChannelKind, SessionStore, Tool};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::memory::service::MemoryService;
use crate::provider::create_provider;
use crate::tools::default_tools;

pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionStore,
    pub ws_clients: DashMap<Uuid, broadcast::Sender<String>>,
    pub subscriptions: DashMap<Uuid, HashSet<Uuid>>,
    pub provider: Option<Arc<dyn Provider>>,
    pub tools: HashMap<String, Box<dyn Tool>>,
    pub channels: DashMap<ChannelKind, Arc<dyn Channel>>,
    pub memory: Option<MemoryService>,
    pub workspace_prompt: Option<String>,
    pub start_time: DateTime<Utc>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Arc<Self> {
        let provider = config
            .models
            .providers
            .first()
            .and_then(|p| create_provider(p));

        let workspace_prompt =
            openclaw_core::workspace::load_prompt_files(&config.workspace.path);

        let sessions_dir = std::path::PathBuf::from(&config.workspace.path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("sessions");

        let sessions = SessionStore::with_persistence(sessions_dir)
            .unwrap_or_else(|_| SessionStore::new());

        let tools = default_tools(&config);

        let memory = if config.memory.enabled {
            match MemoryService::new(&config).await {
                Ok(m) => Some(m),
                Err(e) => {
                    tracing::error!("Failed to init memory service: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Arc::new(Self {
            config,
            sessions,
            ws_clients: DashMap::new(),
            subscriptions: DashMap::new(),
            provider,
            tools,
            channels: DashMap::new(),
            memory,
            workspace_prompt,
            start_time: Utc::now(),
        })
    }

    /// In-memory only — no disk persistence or workspace prompt loading.
    pub fn new_ephemeral(config: AppConfig) -> Arc<Self> {
        let provider = config
            .models
            .providers
            .first()
            .and_then(|p| create_provider(p));

        let tools = default_tools(&config);

        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            ws_clients: DashMap::new(),
            subscriptions: DashMap::new(),
            provider,
            tools,
            channels: DashMap::new(),
            memory: None,
            workspace_prompt: None,
            start_time: Utc::now(),
        })
    }

    pub fn effective_system_prompt(&self) -> Option<String> {
        match (&self.config.agent.system_prompt, &self.workspace_prompt) {
            (Some(sp), Some(wp)) => Some(format!("{sp}\n\n{wp}")),
            (Some(sp), None) => Some(sp.clone()),
            (None, Some(wp)) => Some(wp.clone()),
            (None, None) => None,
        }
    }

    pub fn broadcast(&self, msg: &str) {
        for entry in self.ws_clients.iter() {
            let _ = entry.value().send(msg.to_string());
        }
    }

    pub fn send_to_subscribers(&self, session_id: &Uuid, msg: &str) {
        for entry in self.subscriptions.iter() {
            if entry.value().contains(session_id) {
                let client_id = entry.key();
                if let Some(tx) = self.ws_clients.get(client_id) {
                    let _ = tx.value().send(msg.to_string());
                }
            }
        }
    }

    pub fn send_to_client(&self, client_id: &Uuid, msg: &str) {
        if let Some(tx) = self.ws_clients.get(client_id) {
            let _ = tx.value().send(msg.to_string());
        }
    }

    pub fn subscribe(&self, client_id: Uuid, session_id: Uuid) {
        self.subscriptions
            .entry(client_id)
            .or_insert_with(HashSet::new)
            .insert(session_id);
    }

    pub fn uptime(&self) -> chrono::Duration {
        Utc::now() - self.start_time
    }
}
