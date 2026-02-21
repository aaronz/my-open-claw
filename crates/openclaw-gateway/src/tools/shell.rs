use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use tokio::process::Command;

pub struct ShellTool;

#[async_trait]
impl Tool for ShellTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell_execute".to_string(),
            description: "Execute a shell command locally and return the output. Use with caution.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to run (e.g. 'ls -la')"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let cmd_str = args["command"].as_str().unwrap_or("");
        if cmd_str.is_empty() {
            return Ok("Empty command".to_string());
        }

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .arg("/C")
                .arg(cmd_str)
                .output()
                .await
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(cmd_str)
                .output()
                .await
        }.map_err(|e| openclaw_core::OpenClawError::Provider(format!("Command failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
             Ok(format!("Status: Failed\nError: {}\nOutput: {}", stderr, stdout))
        } else {
            Ok(format!("Output: {}", stdout))
        }
    }
}
