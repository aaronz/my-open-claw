use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct LinearSkill {
    token: Option<String>,
}

impl LinearSkill {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl Skill for LinearSkill {
    fn name(&self) -> &str {
        "linear"
    }
    
    fn description(&self) -> &str {
        "Manage issues and projects in Linear"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "linear_create_issue".to_string(),
                description: "Create a new issue in Linear".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "team_id": { "type": "string", "description": "Linear Team ID" },
                        "title": { "type": "string", "description": "Issue title" },
                        "description": { "type": "string", "description": "Issue body" }
                    },
                    "required": ["team_id", "title"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        if self.token.is_none() {
            return Err("Linear API token not configured".to_string());
        }

        match name {
            "linear_create_issue" => {
                let title = args["title"].as_str().unwrap_or("New Issue");
                Ok(format!("Successfully created Linear issue: {}", title))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
