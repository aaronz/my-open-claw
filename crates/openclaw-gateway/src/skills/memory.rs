use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;
use std::sync::Arc;

use super::Skill;
use crate::memory::service::MemoryService;

pub struct MemorySkill {
    memory: Option<Arc<MemoryService>>,
}

impl MemorySkill {
    pub fn new(memory: Option<Arc<MemoryService>>) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Skill for MemorySkill {
    fn name(&self) -> &str {
        "memory"
    }
    
    fn description(&self) -> &str {
        "Search and recall past conversations and information"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn is_enabled(&self) -> bool {
        self.memory.is_some()
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        if self.memory.is_none() {
            return vec![];
        }
        
        vec![
            ToolDefinition {
                name: "memory_search".to_string(),
                description: "Search past conversations and memories".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "What to search for" },
                        "limit": { "type": "number", "description": "Max results", "default": 5 }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "memory_save".to_string(),
                description: "Save important information to long-term memory".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": { "type": "string", "description": "What to remember" },
                        "context": { "type": "string", "description": "Additional context" }
                    },
                    "required": ["content"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        let memory = self.memory.as_ref().ok_or("Memory service not available")?;
        
        match name {
            "memory_search" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as u64;
                
                match memory.search_memory(query, limit).await {
                    Ok(results) => {
                        if results.is_empty() {
                            Ok("No memories found.".to_string())
                        } else {
                            Ok(results.join("\n---\n"))
                        }
                    }
                    Err(e) => Err(format!("Search failed: {}", e))
                }
            }
            "memory_save" => {
                let content = args["content"].as_str().ok_or("Missing content")?;
                let context = args.get("context")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                let metadata = json!({
                    "context": context,
                    "type": "manual_save"
                });
                
                match memory.add_memory(content, metadata).await {
                    Ok(_) => Ok("Saved to memory.".to_string()),
                    Err(e) => Err(format!("Save failed: {}", e))
                }
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        if self.memory.is_some() {
            Some("You have access to long-term memory. Use memory_search to find past conversations and memory_save to remember important information the user wants to recall later.")
        } else {
            None
        }
    }
}
