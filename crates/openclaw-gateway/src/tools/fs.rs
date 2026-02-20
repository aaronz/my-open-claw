use async_trait::async_trait;
use openclaw_core::{AppConfig, Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

pub struct FileSystemTool {
    workspace: PathBuf,
}

impl FileSystemTool {
    pub fn new(config: &AppConfig) -> Self {
        Self {
            workspace: PathBuf::from(&config.workspace.path),
        }
    }

    fn validate_path(&self, path_str: &str) -> Result<PathBuf> {
        // Prevent directory traversal
        if path_str.contains("..") {
            return Err(openclaw_core::OpenClawError::Provider(
                "Access denied: Path contains '..'".to_string(),
            ));
        }
        let path = self.workspace.join(path_str.trim_start_matches('/'));
        Ok(path)
    }
}

#[async_trait]
impl Tool for FileSystemTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "fs_tool".to_string(),
            description: "File system operations: read, write, list files in workspace.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["read", "write", "list"],
                        "description": "Operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "Relative path to file or directory"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write (for 'write' action)"
                    }
                },
                "required": ["action", "path"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("list");
        let path_str = args["path"].as_str().unwrap_or(".");
        let target_path = self.validate_path(path_str)?;

        // Ensure workspace exists
        if !self.workspace.exists() {
            fs::create_dir_all(&self.workspace).await.map_err(|e| {
                openclaw_core::OpenClawError::Provider(format!("Failed to create workspace: {}", e))
            })?;
        }

        match action {
            "read" => {
                if !target_path.exists() {
                    return Ok("File not found".to_string());
                }
                let content = fs::read_to_string(target_path).await.map_err(|e| {
                    openclaw_core::OpenClawError::Provider(format!("Read failed: {}", e))
                })?;
                Ok(content)
            }
            "write" => {
                let content = args["content"].as_str().unwrap_or("");
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).await.map_err(|e| {
                        openclaw_core::OpenClawError::Provider(format!("Failed to create dirs: {}", e))
                    })?;
                }
                fs::write(target_path, content).await.map_err(|e| {
                    openclaw_core::OpenClawError::Provider(format!("Write failed: {}", e))
                })?;
                Ok(format!("Successfully wrote to {}", path_str))
            }
            "list" => {
                if !target_path.exists() {
                    return Ok("Directory not found".to_string());
                }
                let mut entries = fs::read_dir(target_path).await.map_err(|e| {
                    openclaw_core::OpenClawError::Provider(format!("List failed: {}", e))
                })?;
                
                let mut list = Vec::new();
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let type_str = if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) { "DIR" } else { "FILE" };
                    list.push(format!("[{}] {}", type_str, name));
                }
                Ok(list.join("\n"))
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}
