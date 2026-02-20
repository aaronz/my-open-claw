pub mod anthropic;
pub mod gemini;
pub mod openai;

use openclaw_core::config::ProviderConfig;
use openclaw_core::provider::Provider;
use std::sync::Arc;

pub fn create_provider(config: &ProviderConfig) -> Option<Arc<dyn Provider>> {
    let api_key = config.api_key.as_ref()?;
    match config.name.to_lowercase().as_str() {
        "anthropic" | "claude" => Some(Arc::new(anthropic::AnthropicProvider::new(
            api_key.clone(),
        ))),
        "openai" | "gpt" => Some(Arc::new(openai::OpenAiProvider::new(
            api_key.clone(),
            config.base_url.clone(),
        ))),
        "gemini" | "google" => Some(Arc::new(gemini::GeminiProvider::new(
            api_key.clone(),
            config.model.clone(),
        ))),
        _ => {
            tracing::warn!("unknown provider: {}", config.name);
            None
        }
    }
}
