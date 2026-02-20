pub mod anthropic;
pub mod gemini;
pub mod openai;

use openclaw_core::config::ProviderConfig;
use openclaw_core::provider::{CompletionResponse, Provider, ToolDefinition};
use openclaw_core::session::ChatMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

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

pub fn create_provider_with_fallback(configs: &[ProviderConfig]) -> Option<Arc<dyn Provider>> {
    if configs.is_empty() {
        return None;
    }

    let providers: Vec<Arc<dyn Provider>> = configs
        .iter()
        .filter_map(|c| create_provider(c))
        .collect();

    if providers.is_empty() {
        return None;
    }

    if providers.len() == 1 {
        return Some(providers.into_iter().next().unwrap());
    }

    Some(Arc::new(FailoverProvider { providers }))
}

pub struct FailoverProvider {
    providers: Vec<Arc<dyn Provider>>,
}

impl FailoverProvider {
    fn get_next_provider(&self, last_index: usize) -> Arc<dyn Provider> {
        let next = (last_index + 1) % self.providers.len();
        Arc::clone(&self.providers[next])
    }
}

#[async_trait::async_trait]
impl Provider for FailoverProvider {
    fn name(&self) -> &str {
        "failover"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tools: Option<&[ToolDefinition]>,
        mut token_tx: mpsc::Sender<String>,
    ) -> openclaw_core::error::Result<CompletionResponse> {
        let mut last_error = None;
        let mut current_index = 0;

        loop {
            let provider = self.get_next_provider(current_index);

            match provider
                .stream_chat(
                    messages,
                    system_prompt,
                    model,
                    max_tokens,
                    temperature,
                    tools,
                    token_tx.clone(),
                )
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    tracing::warn!(
                        "Provider {} failed: {}. Trying next provider...",
                        provider.name(),
                        e
                    );
                    last_error = Some(e);
                    current_index = (current_index + 1) % self.providers.len();

                    if current_index == 0 {
                        break;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            openclaw_core::OpenClawError::Provider("All providers failed".to_string())
        }))
    }
}
