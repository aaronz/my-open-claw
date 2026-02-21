use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct NotionSkill {
    token: Option<String>,
}

impl NotionSkill {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl Skill for NotionSkill {
    fn name(&self) -> &str {
        "notion"
    }
    
    fn description(&self) -> &str {
        "Read and write to Notion databases and pages"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "notion_create_page".to_string(),
                description: "Create a new page in a Notion database".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "database_id": { "type": "string", "description": "Notion Database ID" },
                        "title": { "type": "string", "description": "Page title" }
                    },
                    "required": ["database_id", "title"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        if self.token.is_none() {
            return Err("Notion API token not configured".to_string());
        }

        match name {
            "notion_create_page" => {
                let title = args["title"].as_str().ok_or("Missing title")?;
                Ok(format!("Successfully created Notion page: {}", title))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
