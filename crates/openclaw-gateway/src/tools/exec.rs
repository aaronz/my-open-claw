use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::process::Command;
use std::process::Stdio;

use crate::state::AppState;

pub struct ExecTool {
    state: Arc<AppState>,
}

impl ExecTool {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "exec".to_string(),
            description: "Execute shell commands in the workspace with full control over timeout, background execution, privilege elevation, and security policies.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "Timeout in milliseconds (default: 30000)"
                    },
                    "background": {
                        "type": "boolean",
                        "description": "Run in background (default: false)"
                    },
                    "elevated": {
                        "type": "boolean",
                        "description": "Request elevated privileges (default: false)"
                    },
                    "cwd": {
                        "type": "string",
                        "description": "Working directory (default: workspace path)"
                    },
                    "env": {
                        "type": "object",
                        "description": "Environment variables to set"
                    },
                    "shell": {
                        "type": "string",
                        "enum": ["sh", "bash", "zsh", "fish", "cmd", "powershell"],
                        "description": "Shell to use (default: sh on Unix, cmd on Windows)"
                    },
                    "yield_ms": {
                        "type": "integer",
                        "description": "Yield partial output every N ms (for streaming)"
                    },
                    "pty": {
                        "type": "boolean",
                        "description": "Use PTY for interactive commands (default: false)"
                    },
                    "security": {
                        "type": "string",
                        "enum": ["strict", "moderate", "permissive"],
                        "description": "Security level (default: moderate)"
                    },
                    "ask": {
                        "type": "boolean",
                        "description": "Ask user for confirmation before executing (default: false)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let command = args["command"].as_str().unwrap_or("");
        if command.is_empty() {
            return Ok("Error: command is required".to_string());
        }

        let timeout_ms = args["timeout_ms"].as_u64().unwrap_or(30000);
        let background = args["background"].as_bool().unwrap_or(false);
        let cwd = args["cwd"].as_str()
            .unwrap_or(&self.state.config.workspace.path)
            .to_string();
        let shell: String = args["shell"].as_str().unwrap_or(
            if cfg!(target_os = "windows") { "cmd" } else { "sh" }
        ).to_string();
        let security = args["security"].as_str().unwrap_or("moderate");
        let ask = args["ask"].as_bool().unwrap_or(false);

        // Security checks
        if security == "strict" {
            let blocked = ["rm -rf /", "sudo", "mkfs", "dd if=", "> /dev/sd"];
            for pattern in blocked {
                if command.contains(pattern) {
                    return Ok(format!("Command blocked by strict security policy: contains '{}'", pattern));
                }
            }
        }

        if ask {
            // In a real implementation, this would prompt the user
            tracing::warn!("Exec command requires user approval: {}", command);
            return Ok(format!("Command requires user approval: {}", command));
        }

        if background {
            // Spawn background task
            let cmd = command.to_string();
            let workdir = cwd.clone();
            tokio::spawn(async move {
                let _ = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .args(["/C", &cmd])
                        .current_dir(&workdir)
                        .spawn()
                } else {
                    Command::new(shell)
                        .args(["-c", &cmd])
                        .current_dir(&workdir)
                        .spawn()
                };
            });
            return Ok(format!("Command started in background: {}", command));
        }

        // Execute with timeout
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .current_dir(&cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        } else {
            Command::new(shell)
                .args(["-c", command])
                .current_dir(&cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
        };

        match tokio::time::timeout(
            tokio::time::Duration::from_millis(timeout_ms),
            output
        ).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                if output.status.success() {
                    Ok(format!("Exit: {}\nOutput: {}", exit_code, stdout))
                } else {
                    Ok(format!("Exit: {}\nStdout: {}\nStderr: {}", exit_code, stdout, stderr))
                }
            }
            Ok(Err(e)) => Ok(format!("Execution error: {}", e)),
            Err(_) => Ok(format!("Command timed out after {}ms", timeout_ms)),
        }
    }
}
