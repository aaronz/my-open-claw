use async_trait::async_trait;
use chrono::{Local, Utc};
use openclaw_core::provider::ToolDefinition;
use openclaw_core::{Tool, Result as CoreResult};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::state::AppState;

pub struct SessionStatusTool {
    state: Arc<AppState>,
}

impl SessionStatusTool {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for SessionStatusTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "session_status".to_string(),
            description: "Get the current session status, including the current date, time, and session metadata.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    async fn execute(&self, args: Value) -> CoreResult<String> {
        let session_id_str = args["_session_id"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing session_id".to_string()))?;
        let session_id = Uuid::parse_str(session_id_str).map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let now = Local::now();
        let utc_now = Utc::now();
        
        let mut status = format!("Current Time: {}\nUTC Time: {}\n", now.format("%Y-%m-%d %H:%M:%S %Z"), utc_now.to_rfc3339());
        
        if let Some(session) = self.state.sessions.get(&session_id) {
            status.push_str(&format!("Channel: {:?}\n", session.channel));
            status.push_str(&format!("Messages: {}\n", session.messages.len()));
            if !session.metadata.is_empty() {
                status.push_str("Metadata:\n");
                for (k, v) in &session.metadata {
                    status.push_str(&format!("- {}: {}\n", k, v));
                }
            }
        }

        Ok(status)
    }
}
