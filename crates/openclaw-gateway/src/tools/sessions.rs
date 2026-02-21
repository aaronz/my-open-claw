use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::state::AppState;

pub struct SessionsTool {
    state: Arc<AppState>,
}

impl SessionsTool {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for SessionsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "sessions".to_string(),
            description: "Manage conversation sessions - list, inspect history, send messages to other sessions, and spawn sub-agents.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "history", "send", "spawn", "reset", "status"],
                        "description": "Action to perform"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Target session ID"
                    },
                    "message": {
                        "type": "string",
                        "description": "Message content (for 'send' action)"
                    },
                    "agent_id": {
                        "type": "string",
                        "description": "Agent ID to spawn (for 'spawn' action)"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "Prompt for spawned agent (for 'spawn' action)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of messages to return (for 'history' action)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("list");

        match action {
            "list" => {
                let sessions = self.state.sessions.list();
                if sessions.is_empty() {
                    return Ok("No active sessions".to_string());
                }
                let list: Vec<String> = sessions.iter().map(|s| {
                    format!("- {} ({}): {} messages", 
                        s.id, s.channel, s.messages.len())
                }).collect();
                Ok(format!("Active sessions:\n{}", list.join("\n")))
            }
            "history" => {
                let session_id = args["session_id"].as_str()
                    .ok_or_else(|| openclaw_core::OpenClawError::Provider("session_id required".to_string()))?;
                let limit = args["limit"].as_u64().unwrap_or(20) as usize;
                let uuid = Uuid::parse_str(session_id)
                    .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Invalid UUID: {}", e)))?;
                
                if let Some(session) = self.state.sessions.get(&uuid) {
                    let messages: Vec<String> = session.messages.iter().rev().take(limit).rev().map(|m| {
                        let role = match m.role {
                            openclaw_core::session::Role::User => "User",
                            openclaw_core::session::Role::Assistant => "Assistant",
                            openclaw_core::session::Role::System => "System",
                            openclaw_core::session::Role::Tool => "Tool",
                        };
                        format!("[{}] {}", role, m.content.lines().next().unwrap_or(""))
                    }).collect();
                    Ok(format!("Session history:\n{}", messages.join("\n")))
                } else {
                    Ok(format!("Session not found: {}", session_id))
                }
            }
            "send" => {
                let session_id = args["session_id"].as_str()
                    .ok_or_else(|| openclaw_core::OpenClawError::Provider("session_id required".to_string()))?;
                let message = args["message"].as_str()
                    .ok_or_else(|| openclaw_core::OpenClawError::Provider("message required".to_string()))?;
                
                let uuid = Uuid::parse_str(session_id)
                    .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Invalid UUID: {}", e)))?;
                
                let msg = openclaw_core::session::ChatMessage {
                    id: Uuid::new_v4(),
                    role: openclaw_core::session::Role::Assistant,
                    content: message.to_string(),
                    images: vec![],
                    tool_calls: vec![],
                    tool_result: None,
                    timestamp: chrono::Utc::now(),
                    channel: openclaw_core::ChannelKind::Api,
                };
                
                self.state.sessions.add_message(&uuid, msg)?;
                Ok(format!("Message sent to session {}", session_id))
            }
            "spawn" => {
                let agent_id = args["agent_id"].as_str().unwrap_or("default");
                let prompt = args["prompt"].as_str()
                    .ok_or_else(|| openclaw_core::OpenClawError::Provider("prompt required".to_string()))?;
                
                let new_session = self.state.sessions.create(
                    openclaw_core::ChannelKind::Api,
                    format!("spawned:{}", Uuid::new_v4())
                );
                
                Ok(format!(
                    "Spawned agent '{}' in session {}. Prompt: {}",
                    agent_id, new_session.id, prompt
                ))
            }
            "reset" => {
                let session_id = args["session_id"].as_str()
                    .ok_or_else(|| openclaw_core::OpenClawError::Provider("session_id required".to_string()))?;
                
                let uuid = Uuid::parse_str(session_id)
                    .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Invalid UUID: {}", e)))?;
                
                self.state.sessions.reset(&uuid)?;
                Ok(format!("Session {} reset", session_id))
            }
            "status" => {
                let session_id = args["session_id"].as_str();
                if let Some(sid) = session_id {
                    let uuid = Uuid::parse_str(sid)
                        .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Invalid UUID: {}", e)))?;
                    if let Some(session) = self.state.sessions.get(&uuid) {
                        Ok(format!(
                            "Session: {}\nChannel: {}\nMessages: {}\nCreated: {}\nUpdated: {}",
                            session.id, session.channel, session.messages.len(),
                            session.created_at, session.updated_at
                        ))
                    } else {
                        Ok(format!("Session not found: {}", sid))
                    }
                } else {
                    let sessions = self.state.sessions.list();
                    Ok(format!("{} active sessions, {} total messages",
                        sessions.len(),
                        sessions.iter().map(|s| s.messages.len()).sum::<usize>()))
                }
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}
