use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use tokio::process::Command;

pub struct SandboxTool {
    image: String,
    timeout_secs: u64,
}

impl SandboxTool {
    pub fn new() -> Self {
        Self {
            image: "python:3.11-slim".to_string(),
            timeout_secs: 60,
        }
    }

    pub fn with_image(mut self, image: &str) -> Self {
        self.image = image.to_string();
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    async fn is_docker_available() -> bool {
        Command::new("docker")
            .arg("info")
            .output()
            .await
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl Default for SandboxTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SandboxTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sandbox_execute".to_string(),
            description: "Execute a command in an isolated Docker sandbox for security. Supports Python, shell commands, and scripts. Automatically times out after 60 seconds.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The command or script to run in the sandbox"
                    },
                    "language": {
                        "type": "string",
                        "enum": ["python", "bash", "sh"],
                        "description": "The language/interpreter to use (default: python)"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let command = args["command"].as_str().unwrap_or("");
        let language = args["language"].as_str().unwrap_or("python");

        if command.is_empty() {
            return Ok("Empty command".to_string());
        }

        if !Self::is_docker_available().await {
            return Ok("Docker is not available. Please install Docker to use sandbox execution.".to_string());
        }

        let (cmd, img): (Vec<String>, String) = match language {
            "bash" | "sh" => (vec!["sh".to_string(), "-c".to_string(), command.to_string()], "alpine:latest".to_string()),
            _ => (vec!["python".to_string(), "-c".to_string(), command.to_string()], self.image.clone()),
        };

        let mut docker_args: Vec<String> = vec![
            "run".to_string(),
            "--rm".to_string(),
            "--network".to_string(),
            "none".to_string(),
            "--memory".to_string(),
            "256m".to_string(),
            "--cpus".to_string(),
            "0.5".to_string(),
            "--timeout".to_string(),
            self.timeout_secs.to_string(),
            img,
        ];
        
        for arg in cmd {
            docker_args.push(arg);
        }

        let output = Command::new("docker")
            .args(&docker_args)
            .output()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Docker execution failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            Ok(format!("Status: Failed\nError: {}\nOutput: {}", stderr, stdout))
        } else {
            Ok(format!("Output: {}", stdout))
        }
    }
}
