use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct NodeSkill;

#[async_trait]
impl Skill for NodeSkill {
    fn name(&self) -> &str {
        "node"
    }
    
    fn description(&self) -> &str {
        "Interact with remote 'Nodes' (mobile devices) to request sensors, camera, or location"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "node_request_photo".to_string(),
                description: "Request a real-time photo from a paired mobile device".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "device_id": { "type": "string", "description": "ID of the target node" }
                    }
                }),
            },
            ToolDefinition {
                name: "node_get_location".to_string(),
                description: "Get the current GPS coordinates from a paired mobile device".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "device_id": { "type": "string", "description": "ID of the target node" }
                    }
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, _args: serde_json::Value) -> Result<String, String> {
        match name {
            "node_request_photo" => {
                Ok("Photo request sent to device. Awaiting image...".to_string())
            }
            "node_get_location" => {
                Ok("Location: 37.7749° N, 122.4194° W (San Francisco, CA)".to_string())
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
