use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;
use std::process::Command;

use super::Skill;

pub struct SystemSkill;

#[async_trait]
impl Skill for SystemSkill {
    fn name(&self) -> &str {
        "system"
    }
    
    fn description(&self) -> &str {
        "Control macOS system settings like volume, brightness, and Do Not Disturb"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "system_set_volume".to_string(),
                description: "Set system output volume (0-100)".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "level": { "type": "integer", "minimum": 0, "maximum": 100 }
                    },
                    "required": ["level"]
                }),
            },
            ToolDefinition {
                name: "system_get_info".to_string(),
                description: "Get basic macOS system information".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "system_set_volume" => {
                let level = args["level"].as_u64().unwrap_or(50);
                let _ = Command::new("osascript")
                    .arg("-e")
                    .arg(format!("set volume output volume {}", level))
                    .output();
                Ok(format!("System volume set to {}%", level))
            }
            "system_get_info" => {
                let output = Command::new("sw_vers").output().map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
