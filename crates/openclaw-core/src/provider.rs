use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::error::Result;
use crate::session::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

/// Trait for AI model providers (Anthropic, OpenAI, etc.)
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name (e.g., "anthropic", "openai")
    fn name(&self) -> &str;

    /// Stream a chat completion. Tokens are sent through `token_tx` as they arrive.
    /// Returns the full accumulated response text when complete.
    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        model: &str,
        max_tokens: Option<u32>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<String>;
}
