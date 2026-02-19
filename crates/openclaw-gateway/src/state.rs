use chrono::{DateTime, Utc};
use dashmap::DashMap;
use openclaw_core::provider::Provider;
use openclaw_core::{AppConfig, SessionStore};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::provider::create_provider;

pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionStore,
    pub ws_clients: DashMap<Uuid, broadcast::Sender<String>>,
    pub subscriptions: DashMap<Uuid, HashSet<Uuid>>,
    pub provider: Option<Arc<dyn Provider>>,
    pub start_time: DateTime<Utc>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Arc<Self> {
        let provider = config
            .models
            .providers
            .first()
            .and_then(|p| create_provider(p));

        Arc::new(Self {
            config,
            sessions: SessionStore::new(),
            ws_clients: DashMap::new(),
            subscriptions: DashMap::new(),
            provider,
            start_time: Utc::now(),
        })
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
