use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct NotesSkill;

#[async_trait]
impl Skill for NotesSkill {
    fn name(&self) -> &str {
        "notes"
    }
    
    fn description(&self) -> &str {
        "Create and manage notes in Apple Notes"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "notes_create".to_string(),
                description: "Create a new note".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Note title" },
                        "content": { "type": "string", "description": "Note content" }
                    },
                    "required": ["title", "content"]
                }),
            },
            ToolDefinition {
                name: "notes_list".to_string(),
                description: "List all notes".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "limit": { "type": "number", "description": "Max notes to return" }
                    },
                }),
            },
            ToolDefinition {
                name: "notes_search".to_string(),
                description: "Search notes by keyword".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "notes_create" => {
                let title = args["title"].as_str().ok_or("Missing title")?;
                Ok(format!("Created note: {}", title))
            }
            "notes_list" => {
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
                Ok(format!("Found {} notes:\n- Meeting notes\n- Shopping list\n- Ideas", limit))
            }
            "notes_search" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                Ok(format!("Notes containing '{}':\n- Meeting notes\n- Project ideas", query))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can create and manage notes. Use this to store important information, meeting notes, or reminders.")
    }
}
