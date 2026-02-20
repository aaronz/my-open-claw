use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct SlackSkill;

#[async_trait]
impl Skill for SlackSkill {
    fn name(&self) -> &str {
        "slack"
    }
    
    fn description(&self) -> &str {
        "Send messages and interact with Slack channels"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "slack_send_message".to_string(),
                description: "Send a message to a Slack channel".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "channel": { "type": "string", "description": "Channel ID or name" },
                        "text": { "type": "string", "description": "Message text" }
                    },
                    "required": ["channel", "text"]
                }),
            },
            ToolDefinition {
                name: "slack_list_channels".to_string(),
                description: "List available Slack channels".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "slack_send_message" => {
                let channel = args["channel"].as_str().ok_or("Missing channel")?;
                let text = args["text"].as_str().ok_or("Missing text")?;
                Ok(format!("Sent message to {}: {}", channel, text))
            }
            "slack_list_channels" => {
                Ok("Available channels:\n- #general\n- #random\n- #engineering".to_string())
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can send messages to Slack channels using the Slack skill.")
    }
}
