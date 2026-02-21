pub mod anthropic;
pub mod gemini;
pub mod openai;
pub mod ollama;
pub mod vllm;
pub mod litellm;
pub mod bedrock;
pub mod huggingface;
pub mod together;
pub mod moonshot;

use openclaw_core::config::ProviderConfig;
use openclaw_core::provider::Provider;
use std::sync::Arc;
use tokio::sync::mpsc;

pub fn create_provider(config: &ProviderConfig) -> Option<Arc<dyn Provider>> {
    let api_key = config.api_key.as_ref();
    match config.name.to_lowercase().as_str() {
        "anthropic" | "claude" => {
            let key = api_key?;
            Some(Arc::new(anthropic::AnthropicProvider::new(key.clone())))
        }
        "openai" | "gpt" => {
            let key = api_key?;
            Some(Arc::new(openai::OpenAiProvider::new(
                key.clone(),
                config.base_url.clone(),
            )))
        }
        "gemini" | "google" => {
            let key = api_key?;
            Some(Arc::new(gemini::GeminiProvider::new(
                key.clone(),
                config.model.clone(),
            )))
        }
        "ollama" => Some(Arc::new(ollama::OllamaProvider::new(
            config.base_url.clone(),
            config.model.clone(),
        ))),
        "vllm" => {
            let base = config.base_url.clone().unwrap_or_else(|| "http://localhost:8000".to_string());
            Some(Arc::new(vllm::VllmProvider::new(base, config.model.clone())))
        }
        "litellm" => {
            let key = api_key?;
            let base = config.base_url.clone().unwrap_or_else(|| "http://localhost:4000".to_string());
            Some(Arc::new(litellm::LitellmProvider::new(base, key.clone(), config.model.clone())))
        }
        "bedrock" => {
            let key = api_key?;
            let secret = config.base_url.clone().unwrap_or_default();
            let region = "us-east-1".to_string();
            Some(Arc::new(bedrock::BedrockProvider::new(region, config.model.clone(), key.clone(), secret)))
        }
        "huggingface" | "hf" => {
            let key = api_key?;
            let model = if config.model.is_empty() { "microsoft/DialoGPT-large".to_string() } else { config.model.clone() };
            Some(Arc::new(huggingface::HuggingFaceProvider::new(key.clone(), model)))
        }
        "together" | "togetherai" => {
            let key = api_key?;
            let model = if config.model.is_empty() { "meta-llama/Llama-3-70b-chat-hf".to_string() } else { config.model.clone() };
            Some(Arc::new(together::TogetherProvider::new(key.clone(), model)))
        }
        "moonshot" | "kimi" => {
            let key = api_key?;
            let model = if config.model.is_empty() { "moonshot-v1-8k".to_string() } else { config.model.clone() };
            Some(Arc::new(moonshot::MoonshotProvider::new(key.clone(), model)))
        }
        "mock" | "test" => Some(Arc::new(MockProvider)),
        _ => {
            tracing::warn!("unknown provider: {}", config.name);
            None
        }
    }
}

pub struct MockProvider;

#[async_trait::async_trait]
impl openclaw_core::provider::Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn stream_chat(
        &self,
        _messages: &[openclaw_core::session::ChatMessage],
        _system_prompt: Option<&str>,
        _model: &str,
        _max_tokens: Option<u32>,
        _temperature: Option<f32>,
        _tools: Option<&[openclaw_core::provider::ToolDefinition]>,
        token_tx: tokio::sync::mpsc::Sender<String>,
    ) -> openclaw_core::error::Result<openclaw_core::provider::CompletionResponse> {
        let content = "This is a mock response from the OpenClaw test provider. I am running in minimal dependency mode.";
        let _ = token_tx.send(content.to_string()).await;
        
        Ok(openclaw_core::provider::CompletionResponse {
            content: content.to_string(),
            tool_calls: vec![],
        })
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
        messages: &[openclaw_core::session::ChatMessage],
        system_prompt: Option<&str>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tools: Option<&[openclaw_core::provider::ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> openclaw_core::error::Result<openclaw_core::provider::CompletionResponse> {
        let mut last_err = None;
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
                    last_err = Some(e);
                    current_index = (current_index + 1) % self.providers.len();

                    if current_index == 0 {
                        break;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            openclaw_core::OpenClawError::Provider("All providers failed".to_string())
        }))
    }
}
