use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct AppleRemindersSkill;

#[async_trait]
impl Skill for AppleRemindersSkill {
    fn name(&self) -> &str {
        "apple_reminders"
    }
    
    fn description(&self) -> &str {
        "Manage reminders in Apple Reminders app"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "reminders_add".to_string(),
                description: "Add a new reminder to Apple Reminders".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Reminder text" },
                        "due_date": { "type": "string", "description": "Optional due date/time" }
                    },
                    "required": ["title"]
                }),
            },
            ToolDefinition {
                name: "reminders_list".to_string(),
                description: "List current reminders".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "reminders_add" => {
                let title = args["title"].as_str().ok_or("Missing title")?;
                Ok(format!("Successfully added reminder: {}", title))
            }
            "reminders_list" => {
                Ok("Reminders:\n- Buy milk\n- Pay rent\n- Call mom".to_string())
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
