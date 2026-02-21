use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub name: String,
    pub workspace: Option<String>,
    pub system_prompt: Option<String>,
    pub model: Option<String>,
    pub tools_allow: Option<Vec<String>>,
    pub tools_deny: Option<Vec<String>>,
    pub sandbox_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBinding {
    pub agent_id: String,
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_pattern: Option<String>,
}

pub struct AgentRouter {
    agents: HashMap<String, AgentConfig>,
    bindings: Vec<AgentBinding>,
    default_agent: String,
}

impl AgentRouter {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            bindings: Vec::new(),
            default_agent: "default".to_string(),
        }
    }

    pub fn register_agent(&mut self, config: AgentConfig) {
        self.agents.insert(config.id.clone(), config);
    }

    pub fn add_binding(&mut self, binding: AgentBinding) {
        self.bindings.push(binding);
    }

    pub fn set_default(&mut self, agent_id: &str) {
        self.default_agent = agent_id.to_string();
    }

    pub fn route(&self, channel: &str, account_id: Option<&str>, peer_id: &str) -> Option<&AgentConfig> {
        for binding in &self.bindings {
            let channel_match = binding.channel.as_ref()
                .map(|c| c == "*" || c.to_lowercase() == channel.to_lowercase())
                .unwrap_or(true);

            let account_match = binding.account_id.as_ref()
                .map(|a| account_id.map(|aid| a == aid).unwrap_or(false))
                .unwrap_or(true);

            let peer_match = binding.peer_pattern.as_ref()
                .map(|p| {
                    if p == "*" {
                        true
                    } else if p.starts_with('^') || p.ends_with('$') {
                        let pattern = regex::Regex::new(p).ok();
                        pattern.map(|re| re.is_match(peer_id)).unwrap_or(false)
                    } else {
                        peer_id.contains(p)
                    }
                })
                .unwrap_or(true);

            if channel_match && account_match && peer_match {
                if let Some(agent) = self.agents.get(&binding.agent_id) {
                    return Some(agent);
                }
            }
        }

        self.agents.get(&self.default_agent)
    }

    pub fn get_agent(&self, id: &str) -> Option<&AgentConfig> {
        self.agents.get(id)
    }

    pub fn list_agents(&self) -> Vec<&AgentConfig> {
        self.agents.values().collect()
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}
