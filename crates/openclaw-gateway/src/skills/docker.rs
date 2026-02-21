use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;
use std::process::Command;

use super::Skill;

pub struct DockerSkill;

#[async_trait]
impl Skill for DockerSkill {
    fn name(&self) -> &str {
        "docker"
    }
    
    fn description(&self) -> &str {
        "Manage Docker containers and images"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "docker_ps".to_string(),
                description: "List running docker containers".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "all": { "type": "boolean", "description": "Show all containers (default only running)" }
                    }
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "docker_ps" => {
                let all = args["all"].as_bool().unwrap_or(false);
                let mut cmd = Command::new("docker");
                cmd.arg("ps");
                if all {
                    cmd.arg("-a");
                }
                
                let output = cmd.output().map_err(|e| e.to_string())?;
                if output.status.success() {
                    Ok(String::from_utf8_lossy(&output.stdout).to_string())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
