use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use super::Skill;

pub struct ObsidianSkill {
    vault_path: Option<PathBuf>,
}

impl ObsidianSkill {
    pub fn new(path: Option<String>) -> Self {
        Self {
            vault_path: path.map(PathBuf::from),
        }
    }
}

#[async_trait]
impl Skill for ObsidianSkill {
    fn name(&self) -> &str {
        "obsidian"
    }
    
    fn description(&self) -> &str {
        "Read and write notes in Obsidian vaults"
    }
    
    fn version(&self) -> &str {
        "1.1.0"
    }

    fn is_enabled(&self) -> bool {
        self.vault_path.is_some()
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "obsidian_read".to_string(),
                description: "Read a note from the vault".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Note path (e.g. 'Daily/2026-02-20')" }
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
                        "append": { "type": "boolean", "description": "Append to existing note", "default": false }
                    },
                    "required": ["path", "content"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        let vault = self.vault_path.as_ref().ok_or("Obsidian vault path not configured")?;
        
        match name {
            "obsidian_read" => {
                let path_str = args["path"].as_str().ok_or("Missing path")?;
                let full_path = vault.join(format!("{}.md", path_str));
                
                let content = fs::read_to_string(full_path).map_err(|e| format!("Read failed: {}", e))?;
                Ok(content)
            }
            "obsidian_write" => {
                let path_str = args["path"].as_str().ok_or("Missing path")?;
                let content = args["content"].as_str().ok_or("Missing content")?;
                let append = args["append"].as_bool().unwrap_or(false);
                
                let full_path = vault.join(format!("{}.md", path_str));
                
                if let Some(parent) = full_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                
                if append {
                    let mut existing = fs::read_to_string(&full_path).unwrap_or_default();
                    if !existing.is_empty() && !existing.ends_with('\n') {
                        existing.push('\n');
                    }
                    existing.push_str(content);
                    fs::write(full_path, existing).map_err(|e| e.to_string())?;
                } else {
                    fs::write(full_path, content).map_err(|e| e.to_string())?;
                }
                
                Ok(format!("Successfully wrote to {}", path_str))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
