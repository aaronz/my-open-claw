use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::Result;
use crate::session::ChatMessage;

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
