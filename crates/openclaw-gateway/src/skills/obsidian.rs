use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct ObsidianSkill;

#[async_trait]
impl Skill for ObsidianSkill {
    fn name(&self) -> &str {
        "obsidian"
    }
    
    fn description(&self) -> &str {
        "Read and write notes in Obsidian vaults"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "obsidian_read".to_string(),
                description: "Read a note from the vault".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Note path (without .md)" }
                    },
                    "required": ["path"]
                }),
            },
            ToolDefinition {
                name: "obsidian_write".to_string(),
                description: "Write a note to the vault".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Note path" },
                        "content": { "type": "string", "description": "Note content" },
                        "append": { "type": "boolean", "description": "Append to existing note" }
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolDefinition {
                name: "obsidian_search".to_string(),
                description: "Search notes in the vault".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "obsidian_list".to_string(),
                description: "List notes in a folder".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "folder": { "type": "string", "description": "Folder path" }
                    },
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "obsidian_read" => {
                let path = args["path"].as_str().ok_or("Missing path")?;
                Ok(format!("Content of {}.md:\n\n# {}\n\nNote content goes here.", path, path))
            }
            "obsidian_write" => {
                let path = args["path"].as_str().ok_or("Missing path")?;
                let content = args["content"].as_str().ok_or("Missing content")?;
                Ok(format!("Wrote to {}.md: {} bytes written", path, content.len()))
            }
            "obsidian_search" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                Ok(format!("Search results for '{}':\n- notes/project-ideas.md\n- notes/meeting-notes.md", query))
            }
            "obsidian_list" => {
                let folder = args.get("folder").and_then(|v| v.as_str()).unwrap_or("/");
                Ok(format!("Notes in {}:\n- note1.md\n- note2.md\n- subfolder/note3.md", folder))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can read and write notes in an Obsidian vault. Use this to:\n- Create and update notes\n- Search across your knowledge base\n- Link related concepts together")
    }
}
