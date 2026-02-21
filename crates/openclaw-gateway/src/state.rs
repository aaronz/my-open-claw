use chrono::{DateTime, Utc};
use dashmap::DashMap;
use openclaw_core::provider::Provider;
use openclaw_core::{AppConfig, Channel, ChannelKind, SessionStore, Tool};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::auth::oauth::OAuthManager;
use crate::auth::pairing::PairingManager;
use crate::cron::CronScheduler;
use crate::mcp::McpManager;
use crate::memory::service::MemoryService;
use crate::provider::create_provider_with_fallback;
use crate::skills::{SkillRegistry, default_skills};
use crate::tools::default_tools;
use crate::voice::service::VoiceService;

pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionStore,
    pub ws_clients: DashMap<Uuid, broadcast::Sender<String>>,
    pub subscriptions: DashMap<Uuid, HashSet<Uuid>>,
    pub provider: Option<Arc<dyn Provider>>,
    pub tools: DashMap<String, Box<dyn Tool>>,
    pub channels: DashMap<ChannelKind, Arc<dyn Channel>>,
    pub memory: Option<MemoryService>,
    pub voice: Option<VoiceService>,
    pub cron: Arc<CronScheduler>,
    pub workspace_prompt: Option<String>,
    pub start_time: DateTime<Utc>,
    pub skills: SkillRegistry,
    pub oauth: Arc<OAuthManager>,
    pub mcp: Arc<McpManager>,
    pub pairing: Arc<PairingManager>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Arc<Self> {
        let provider = create_provider_with_fallback(&config.models.providers);

        let workspace_prompt =
            openclaw_core::workspace::load_prompt_files(&config.workspace.path);

        let data_dir = std::path::PathBuf::from(&config.workspace.path)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("data");
        std::fs::create_dir_all(&data_dir).unwrap_or_default();
        
        let db_path = data_dir.join("openclaw.db");
        let db_url = format!("sqlite://{}", db_path.display());
        
        let sessions = SessionStore::with_sqlite(&db_url).await
            .unwrap_or_else(|e| {
                tracing::error!("Failed to init SQLite: {}. Using in-memory sessions.", e);
                SessionStore::new()
            });

        let cron = Arc::new(CronScheduler::new());

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

        let voice = VoiceService::new(&config);

        let memory_ref = memory.clone();
        let mut skills = default_skills(
            config.agent.github_token.clone(), 
            config.agent.obsidian_path.clone(), 
            config.agent.notion_token.clone(),
            config.agent.google_token.clone(),
            config.agent.linear_token.clone(),
            config.agent.todoist_token.clone()
        );
        if let Some(ref mem) = memory_ref {
            skills.register(Box::new(crate::skills::MemorySkill::new(Some(Arc::new(mem.clone())))));
        }

        let state = Arc::new(Self {
            config: config.clone(),
            sessions,
            ws_clients: DashMap::new(),
            subscriptions: DashMap::new(),
            provider,
            tools: DashMap::new(),
            channels: DashMap::new(),
            memory,
            voice,
            cron: cron.clone(),
            workspace_prompt,
            start_time: Utc::now(),
            skills,
            oauth: Arc::new(OAuthManager::new()),
            mcp: Arc::new(McpManager::new()),
            pairing: Arc::new(PairingManager::new()),
        });

        let tools = default_tools(&config, cron.clone(), state.clone());
        for (name, tool) in tools {
            state.tools.insert(name, tool);
        }

        state
    }

    /// In-memory only — no disk persistence or workspace prompt loading.
    pub fn new_ephemeral(config: AppConfig) -> Arc<Self> {
        let provider = create_provider_with_fallback(&config.models.providers);

        let cron = Arc::new(CronScheduler::new());
        let skills = default_skills(
            config.agent.github_token.clone(), 
            config.agent.obsidian_path.clone(), 
            config.agent.notion_token.clone(),
            config.agent.google_token.clone(),
            config.agent.linear_token.clone(),
            config.agent.todoist_token.clone()
        );

        let state = Arc::new(Self {
            config: config.clone(),
            sessions: SessionStore::new(),
            ws_clients: DashMap::new(),
            subscriptions: DashMap::new(),
            provider,
            tools: DashMap::new(),
            channels: DashMap::new(),
            memory: None,
            voice: None,
            cron: cron.clone(),
            workspace_prompt: None,
            start_time: Utc::now(),
            skills,
            oauth: Arc::new(OAuthManager::new()),
            mcp: Arc::new(McpManager::new()),
            pairing: Arc::new(PairingManager::new()),
        });

        let tools = default_tools(&config, cron.clone(), state.clone());
        for (name, tool) in tools {
            state.tools.insert(name, tool);
        }

        state
    }

    pub fn effective_system_prompt(&self) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(sp) = &self.config.agent.system_prompt {
            parts.push(sp.clone());
        }

        if let Some(wp) = &self.workspace_prompt {
            parts.push(wp.clone());
        }

        let now = chrono::Local::now();
        parts.push(format!("### Current Date & Time\nTime zone: {}\nLocal time: {}", now.format("%Z"), now.format("%Y-%m-%d %H:%M:%S")));

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n\n"))
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
