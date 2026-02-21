use async_trait::async_trait;
use dashmap::DashMap;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};
use uuid::Uuid;

pub struct Process {
    pub id: String,
    pub command: String,
    pub status: String,
    pub output: Vec<String>,
    pub exit_code: Option<i32>,
}

pub struct ProcessManager {
    processes: Arc<DashMap<String, Process>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
        }
    }

    pub async fn spawn(&self, command: &str) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        
        let mut cmd = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg(command)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
        }.map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to spawn: {}", e)))?;

        let process = Process {
            id: id.clone(),
            command: command.to_string(),
            status: "running".to_string(),
            output: vec![],
            exit_code: None,
        };

        self.processes.insert(id.clone(), process);
        
        let processes = self.processes.clone();
        let pid = id.clone();
        tokio::spawn(async move {
            let status = cmd.wait().await;
            if let Some(mut p) = processes.get_mut(&pid) {
                p.status = "completed".to_string();
                p.exit_code = status.ok().and_then(|s| s.code());
            }
        });

        Ok(id)
    }

    pub fn list(&self) -> Vec<Process> {
        self.processes.iter().map(|p| p.value().clone()).collect()
    }

    pub fn get(&self, id: &str) -> Option<Process> {
        self.processes.get(id).map(|p| p.value().clone())
    }

    pub fn remove(&self, id: &str) -> bool {
        self.processes.remove(id).is_some()
    }
}

pub struct ProcessTool {
    manager: Arc<ProcessManager>,
}

impl ProcessTool {
    pub fn new(manager: Arc<ProcessManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "process".to_string(),
            description: "Manage background process sessions - spawn, list, poll output, and kill long-running commands.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["spawn", "list", "poll", "kill", "remove"],
                        "description": "Action to perform"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to spawn (for spawn action)"
                    },
                    "id": {
                        "type": "string",
                        "description": "Process ID (for poll/kill/remove)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("list");

        match action {
            "spawn" => {
                let command = args["command"].as_str().unwrap_or("");
                if command.is_empty() {
                    return Ok("Error: command is required for spawn".to_string());
                }
                let id = self.manager.spawn(command).await?;
                Ok(format!("Spawned process: {}", id))
            }
            "list" => {
                let processes = self.manager.list();
                if processes.is_empty() {
                    return Ok("No running processes".to_string());
                }
                let output: Vec<String> = processes.iter().map(|p| {
                    format!("{}: {} ({})", p.id, p.command, p.status)
                }).collect();
                Ok(output.join("\n"))
            }
            "poll" => {
                let id = args["id"].as_str().unwrap_or("");
                if let Some(p) = self.manager.get(id) {
                    Ok(format!("Process {}: {} ({:?})\nOutput: {}", 
                        p.id, p.status, p.exit_code, p.output.join("\n")))
                } else {
                    Ok(format!("Process not found: {}", id))
                }
            }
            "kill" | "remove" => {
                let id = args["id"].as_str().unwrap_or("");
                if self.manager.remove(id) {
                    Ok(format!("Process {} removed", id))
                } else {
                    Ok(format!("Process not found: {}", id))
                }
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}

impl Clone for Process {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            command: self.command.clone(),
            status: self.status.clone(),
            output: self.output.clone(),
            exit_code: self.exit_code,
        }
    }
}
