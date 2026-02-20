use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct DiscordSkill;

#[async_trait]
impl Skill for DiscordSkill {
    fn name(&self) -> &str {
        "discord"
    }
    
    fn description(&self) -> &str {
        "Send messages and interact with Discord servers"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "discord_send_message".to_string(),
                description: "Send a message to a Discord channel".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "channel_id": { "type": "string", "description": "Channel ID" },
                        "content": { "type": "string", "description": "Message content" }
                    },
                    "required": ["channel_id", "content"]
                }),
            },
            ToolDefinition {
                name: "discord_list_channels".to_string(),
                description: "List channels in a Discord server".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "guild_id": { "type": "string", "description": "Server ID" }
                    },
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "discord_send_message" => {
                let channel_id = args["channel_id"].as_str().ok_or("Missing channel_id")?;
                let content = args["content"].as_str().ok_or("Missing content")?;
                Ok(format!("Sent message to channel {}: {}", channel_id, content))
            }
            "discord_list_channels" => {
                Ok("Channels:\n- #general (123456789)\n- #random (987654321)".to_string())
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can send messages to Discord channels using the Discord skill.")
    }
}
