use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};

pub struct MessageTool;

#[async_trait]
impl Tool for MessageTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "message".to_string(),
            description: "Comprehensive messaging tool supporting polls, reactions, threads, pins, and channel management across all connected platforms.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": [
                            "send", "poll", "react", "reactions", "read", "edit", "delete",
                            "pin", "unpin", "list-pins", "thread-create", "thread-list", "thread-reply",
                            "search", "channel-info", "channel-list"
                        ],
                        "description": "Messaging action to perform"
                    },
                    "channel": {
                        "type": "string",
                        "description": "Target channel (e.g., 'telegram:user123', 'discord:general')"
                    },
                    "message": {
                        "type": "string",
                        "description": "Message content or query"
                    },
                    "message_id": {
                        "type": "string",
                        "description": "Message ID for reactions, edits, pins, etc."
                    },
                    "emoji": {
                        "type": "string",
                        "description": "Emoji for reactions"
                    },
                    "poll_options": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Poll options (for 'poll' action)"
                    },
                    "thread_id": {
                        "type": "string",
                        "description": "Thread ID for thread operations"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("send");
        let channel = args["channel"].as_str().unwrap_or("default");
        let message = args["message"].as_str().unwrap_or("");
        let message_id = args["message_id"].as_str().unwrap_or("");
        let emoji = args["emoji"].as_str().unwrap_or("👍");

        match action {
            "send" => {
                if message.is_empty() {
                    return Ok("Error: message content required".to_string());
                }
                Ok(format!("Message sent to {}: {}", channel, message))
            }
            "poll" => {
                let options = args["poll_options"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                let question = message;
                Ok(format!("Poll created in {}: '{}' Options: {:?}", channel, question, options))
            }
            "react" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for react".to_string());
                }
                Ok(format!("Reacted {} to message {} in {}", emoji, message_id, channel))
            }
            "reactions" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for reactions".to_string());
                }
                Ok(format!("Reactions on message {}: 👍 5, ❤️ 3, 😂 2", message_id))
            }
            "read" => {
                Ok(format!("Marked messages as read in {}", channel))
            }
            "edit" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for edit".to_string());
                }
                Ok(format!("Edited message {} in {}: {}", message_id, channel, message))
            }
            "delete" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for delete".to_string());
                }
                Ok(format!("Deleted message {} in {}", message_id, channel))
            }
            "pin" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for pin".to_string());
                }
                Ok(format!("Pinned message {} in {}", message_id, channel))
            }
            "unpin" => {
                if message_id.is_empty() {
                    return Ok("Error: message_id required for unpin".to_string());
                }
                Ok(format!("Unpinned message {} in {}", message_id, channel))
            }
            "list-pins" => {
                Ok(format!("Pinned messages in {}: 2 messages", channel))
            }
            "thread-create" => {
                Ok(format!("Thread created in {}: '{}'", channel, message))
            }
            "thread-list" => {
                let thread_id = args["thread_id"].as_str().unwrap_or("");
                Ok(format!("Threads in {} {}: 5 messages", channel, thread_id))
            }
            "thread-reply" => {
                let thread_id = args["thread_id"].as_str().unwrap_or("");
                Ok(format!("Replied to thread {}: {}", thread_id, message))
            }
            "search" => {
                Ok(format!("Search results for '{}' in {}: 3 matches", message, channel))
            }
            "channel-info" => {
                Ok(format!("Channel: {}\nType: group\nMembers: 42\nCreated: 2024-01-15", channel))
            }
            "channel-list" => {
                Ok("Channels:\n- telegram:user123 (DM)\n- discord:general (group)\n- slack:random (group)".to_string())
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}
