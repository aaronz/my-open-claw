use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::nodes::NodeManager;

pub struct NodesTool {
    manager: Arc<NodeManager>,
}

impl NodesTool {
    pub fn new(manager: Arc<NodeManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for NodesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "nodes".to_string(),
            description: "Discover and control paired nodes (companion devices like macOS, iOS, Android). Supports camera capture, screen recording, location, and command execution.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["status", "list", "describe", "run", "camera_snap", "camera_clip", "screen_record", "location_get", "notify"],
                        "description": "Action to perform"
                    },
                    "node_id": {
                        "type": "string",
                        "description": "Target node ID (optional, uses first available if not specified)"
                    },
                    "command": {
                        "type": "string",
                        "description": "Command to run (for 'run' action)"
                    },
                    "message": {
                        "type": "string",
                        "description": "Notification message (for 'notify' action)"
                    },
                    "duration": {
                        "type": "integer",
                        "description": "Duration in seconds (for camera_clip, screen_record)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("list");
        let node_id = args["node_id"].as_str();

        match action {
            "status" => {
                let nodes = self.manager.list().await;
                let online = nodes.iter().filter(|n| n.online).count();
                Ok(format!("Nodes: {} online, {} total", online, nodes.len()))
            }
            "list" => {
                let nodes = self.manager.list().await;
                if nodes.is_empty() {
                    return Ok("No paired nodes".to_string());
                }
                let list: Vec<String> = nodes.iter().map(|n| {
                    let status = if n.online { "online" } else { "offline" };
                    format!("- {} ({}) [{}]", n.name, n.platform, status)
                }).collect();
                Ok(format!("Paired nodes:\n{}", list.join("\n")))
            }
            "describe" => {
                let target_id = node_id.unwrap_or("");
                if let Some(node) = self.manager.get(target_id).await {
                    Ok(format!(
                        "Node: {}\nPlatform: {}\nVersion: {}\nCapabilities: {}\nOnline: {}",
                        node.name, node.platform, node.version, 
                        node.capabilities.join(", "), node.online
                    ))
                } else {
                    Ok(format!("Node not found: {}", target_id))
                }
            }
            "run" => {
                let command = args["command"].as_str().unwrap_or("");
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: command.to_string(),
                        params: json!({}),
                    }).await;
                if response.success {
                    Ok(format!("Command executed: {}", 
                        response.data.map(|d| d.to_string()).unwrap_or_default()))
                } else {
                    Ok(format!("Command failed: {}", 
                        response.error.unwrap_or_default()))
                }
            }
            "camera_snap" => {
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: "camera_snap".to_string(),
                        params: json!({}),
                    }).await;
                if response.success {
                    Ok("Camera snapshot captured".to_string())
                } else {
                    Ok(format!("Failed: {}", response.error.unwrap_or_default()))
                }
            }
            "camera_clip" => {
                let duration = args["duration"].as_u64().unwrap_or(10);
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: "camera_clip".to_string(),
                        params: json!({ "duration": duration }),
                    }).await;
                if response.success {
                    Ok(format!("Video clip recorded ({}s)", duration))
                } else {
                    Ok(format!("Failed: {}", response.error.unwrap_or_default()))
                }
            }
            "screen_record" => {
                let duration = args["duration"].as_u64().unwrap_or(30);
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: "screen_record".to_string(),
                        params: json!({ "duration": duration }),
                    }).await;
                if response.success {
                    Ok(format!("Screen recorded ({}s)", duration))
                } else {
                    Ok(format!("Failed: {}", response.error.unwrap_or_default()))
                }
            }
            "location_get" => {
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: "location_get".to_string(),
                        params: json!({}),
                    }).await;
                if response.success {
                    if let Some(data) = response.data {
                        Ok(format!("Location: lat={}, lng={}", 
                            data["lat"].as_f64().unwrap_or(0.0),
                            data["lng"].as_f64().unwrap_or(0.0)))
                    } else {
                        Ok("Location retrieved".to_string())
                    }
                } else {
                    Ok(format!("Failed: {}", response.error.unwrap_or_default()))
                }
            }
            "notify" => {
                let message = args["message"].as_str().unwrap_or("Notification from OpenClaw");
                let response = self.manager.send_command(node_id.unwrap_or(""), 
                    crate::nodes::NodeCommand {
                        command: "notify".to_string(),
                        params: json!({ "message": message }),
                    }).await;
                if response.success {
                    Ok(format!("Notification sent: {}", message))
                } else {
                    Ok(format!("Failed: {}", response.error.unwrap_or_default()))
                }
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}
