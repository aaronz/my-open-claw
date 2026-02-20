use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct OAuthManager {
    configs: HashMap<String, OAuthConfig>,
    tokens: Arc<Mutex<HashMap<(Uuid, String), OAuthToken>>>,
}

impl OAuthManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_config(&mut self, provider: String, config: OAuthConfig) {
        self.configs.insert(provider, config);
    }

    pub fn get_auth_url(&self, provider: &str, session_id: Uuid) -> Option<String> {
        let config = self.configs.get(provider)?;
        let state = format!("{}:{}", provider, session_id);
        Some(format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&state={}",
            config.auth_url, config.client_id, config.redirect_uri, state
        ))
    }

    pub async fn save_token(&self, session_id: Uuid, provider: String, token: OAuthToken) {
        let mut tokens = self.tokens.lock().await;
        tokens.insert((session_id, provider), token);
    }

    pub async fn get_token(&self, session_id: Uuid, provider: &str) -> Option<OAuthToken> {
        let tokens = self.tokens.lock().await;
        tokens.get(&(session_id, provider.to_string())).cloned()
    }
}
