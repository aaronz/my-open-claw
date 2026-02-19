use async_trait::async_trait;
use serde_json::Value;
use crate::provider::ToolDefinition;
use crate::error::Result;

#[async_trait]
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, args: Value) -> Result<String>;
}
