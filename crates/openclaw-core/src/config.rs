use crate::error::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub gateway: GatewayConfig,
    #[serde(default)]
    pub channels: ChannelsConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub models: ModelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub bind: BindMode,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub verbose: bool,
}

fn default_port() -> u16 {
    18789
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BindMode {
    #[default]
    Loopback,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub mode: AuthMode,
    pub password: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    #[default]
    None,
    Password,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    pub telegram: Option<ChannelInstanceConfig>,
    pub discord: Option<ChannelInstanceConfig>,
    pub slack: Option<ChannelInstanceConfig>,
    pub whatsapp: Option<ChannelInstanceConfig>,
    pub signal: Option<ChannelInstanceConfig>,
    pub webchat: Option<ChannelInstanceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInstanceConfig {
    #[serde(default)]
    pub enabled: bool,
    pub token: Option<String>,
    #[serde(default)]
    pub dm_policy: DmPolicy,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DmPolicy {
    #[default]
    Pairing,
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub thinking: ThinkingLevel,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default)]
    pub providers: Vec<ProviderConfig>,
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            gateway: GatewayConfig::default(),
            channels: ChannelsConfig::default(),
            agent: AgentConfig::default(),
            models: ModelsConfig::default(),
        }
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            port: 18789,
            bind: BindMode::Loopback,
            auth: AuthConfig::default(),
            verbose: false,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: AuthMode::None,
            password: None,
            token: None,
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            thinking: ThinkingLevel::Medium,
            max_tokens: None,
        }
    }
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            default_model: default_model(),
            providers: vec![],
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_path() -> PathBuf {
        ProjectDirs::from("ai", "openclaw", "openclaw")
            .map(|dirs| dirs.config_dir().join("config.yaml"))
            .unwrap_or_else(|| PathBuf::from("config.yaml"))
    }
}
