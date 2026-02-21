use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::{CompletionResponse, Provider, ToolDefinition};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct TogetherProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl TogetherProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl Provider for TogetherProvider {
    fn name(&self) -> &str {
        "together"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        _model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        _tools: Option<&[ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<CompletionResponse> {
        let url = "https://api.together.xyz/v1/chat/completions";

        let mut openai_messages: Vec<Value> = messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::System => "system",
                    Role::Tool => "tool",
                };
                json!({
                    "role": role,
                    "content": m.content
                })
            })
            .collect();

        if let Some(sp) = system_prompt {
            openai_messages.insert(0, json!({
                "role": "system",
                "content": sp
            }));
        }

        let body = json!({
            "model": self.model,
            "messages": openai_messages,
            "stream": true,
            "max_tokens": max_tokens.unwrap_or(4096),
            "temperature": temperature.unwrap_or(0.7)
        });

        let response = self.client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Provider(format!("Together error {}: {}", status, text)));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e: reqwest::Error| OpenClawError::Provider(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" { continue; }
                        
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                                full_response.push_str(content);
                                let _ = token_tx.send(content.to_string()).await;
                            }
                        }
                    }
                }
            }
        }

        Ok(CompletionResponse {
            content: full_response,
            tool_calls: vec![],
        })
    }
}
