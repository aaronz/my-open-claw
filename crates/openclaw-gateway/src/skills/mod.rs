use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![]
    }
    
    async fn execute_tool(&self, _name: &str, _args: serde_json::Value) -> Result<String, String> {
        Err("Tool not implemented".to_string())
    }
    
    fn system_prompt(&self) -> Option<&str> {
        None
    }
    
    fn is_enabled(&self) -> bool {
        true
    }
}

pub struct SkillRegistry {
    skills: HashMap<String, Box<dyn Skill>>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, skill: Box<dyn Skill>) {
        self.skills.insert(skill.name().to_string(), skill);
    }
    
    pub fn get(&self, name: &str) -> Option<&Box<dyn Skill>> {
        self.skills.get(name)
    }
    
    pub fn list(&self) -> Vec<&Box<dyn Skill>> {
        self.skills.values().collect()
    }
    
    pub fn enabled_skills(&self) -> Vec<&Box<dyn Skill>> {
        self.skills.values().filter(|s| s.is_enabled()).collect()
    }
    
    pub fn all_tools(&self) -> Vec<ToolDefinition> {
        self.enabled_skills()
            .iter()
            .flat_map(|s| s.tools())
            .collect()
    }
    
    pub fn system_prompts(&self) -> String {
        self.enabled_skills()
            .iter()
            .filter_map(|s| s.system_prompt())
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillConfig {
    pub name: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
}

fn default_enabled() -> bool {
    true
}

pub mod github;
pub mod slack;
pub mod discord;
pub mod weather;
pub mod notes;
pub mod spotify;
pub mod obsidian;
pub mod memory;
pub mod notion;
pub mod onepassword;
pub mod node;
pub mod docker;

pub use github::GitHubSkill;
pub use slack::SlackSkill;
pub use discord::DiscordSkill;
pub use weather::WeatherSkill;
pub use notes::NotesSkill;
pub use spotify::SpotifySkill;
pub use obsidian::ObsidianSkill;
pub use memory::MemorySkill;
pub use notion::NotionSkill;
pub use onepassword::OnePasswordSkill;
pub use node::NodeSkill;
pub use docker::DockerSkill;

pub fn default_skills(github_token: Option<String>, obsidian_path: Option<String>, notion_token: Option<String>) -> SkillRegistry {
    let mut registry = SkillRegistry::new();
    
    registry.register(Box::new(WeatherSkill));
    registry.register(Box::new(GitHubSkill::new(github_token)));
    registry.register(Box::new(SlackSkill));
    registry.register(Box::new(DiscordSkill));
    registry.register(Box::new(NotesSkill));
    registry.register(Box::new(SpotifySkill));
    registry.register(Box::new(ObsidianSkill::new(obsidian_path)));
    registry.register(Box::new(NotionSkill::new(notion_token)));
    registry.register(Box::new(OnePasswordSkill));
    registry.register(Box::new(NodeSkill));
    registry.register(Box::new(DockerSkill));
    registry.register(Box::new(MemorySkill::new(None)));
    
    registry
}
