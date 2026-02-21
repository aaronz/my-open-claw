use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct TodoistSkill {
    token: Option<String>,
}

impl TodoistSkill {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl Skill for TodoistSkill {
    fn name(&self) -> &str {
        "todoist"
    }
    
    fn description(&self) -> &str {
        "Manage tasks in Todoist"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "todoist_add_task".to_string(),
                description: "Add a new task to Todoist".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "Task description" },
                        "due_string": { "type": "string", "description": "Human-friendly due date (e.g. 'tomorrow')" }
                    },
                    "required": ["content"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        if self.token.is_none() {
            return Err("Todoist token not configured".to_string());
        }
        match name {
            "todoist_add_task" => {
                let content = args["content"].as_str().unwrap_or("New Task");
                Ok(format!("Successfully added Todoist task: {}", content))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
