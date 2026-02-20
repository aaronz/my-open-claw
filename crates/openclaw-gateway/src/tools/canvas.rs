use openclaw_core::provider::ToolDefinition;
use openclaw_core::{Tool, Result as CoreResult, WsMessage};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use crate::state::AppState;

pub struct CanvasTool {
    state: Arc<AppState>,
}

impl CanvasTool {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait::async_trait]
impl Tool for CanvasTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "update_canvas".to_string(),
            description: "Update the persistent canvas/artifact for this session. Use this to display code snippets, charts, or long-form documents separately from the chat.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Unique identifier for this canvas/artifact"
                    },
                    "title": {
                        "type": "string",
                        "description": "Descriptive title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Markdown or code content"
                    },
                    "language": {
                        "type": "string",
                        "description": "Programming language for syntax highlighting"
                    }
                },
                "required": ["id", "content"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> CoreResult<String> {
        let id = args["id"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing id".to_string()))?.to_string();
        let content = args["content"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing content".to_string()))?.to_string();
        let title = args["title"].as_str().map(|s| s.to_string());
        let language = args["language"].as_str().map(|s| s.to_string());
        
        let session_id_str = args["_session_id"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing session_id".to_string()))?;
        let session_id = Uuid::parse_str(session_id_str).map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let msg = WsMessage::CanvasUpdate {
            session_id,
            id: id.clone(),
            content,
            language,
            title,
        };

        if let Ok(json) = serde_json::to_string(&msg) {
            self.state.broadcast(&json);
        }

        Ok(format!("Canvas '{}' updated.", id))
    }
}
