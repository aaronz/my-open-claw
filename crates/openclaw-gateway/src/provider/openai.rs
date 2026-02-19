use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::Provider;
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: base_url.unwrap_or_else(|| DEFAULT_BASE_URL.to_string()),
        }
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        model: &str,
        max_tokens: Option<u32>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<String> {
        let mut api_messages: Vec<Value> = Vec::new();

        if let Some(sp) = system_prompt {
            api_messages.push(json!({
                "role": "system",
                "content": sp,
            }));
        }

        for m in messages {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::System => "system",
            };
            api_messages.push(json!({
                "role": role,
                "content": m.content,
            }));
        }

        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
        });

        if let Some(mt) = max_tokens {
            body["max_tokens"] = json!(mt);
        }

        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(OpenClawError::Provider(format!(
                "OpenAI API error {status}: {text}"
            )));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| OpenClawError::Provider(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data.trim() == "[DONE]" {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(content) =
                                parsed["choices"][0]["delta"]["content"].as_str()
                            {
                                full_response.push_str(content);
                                let _ = token_tx.send(content.to_string()).await;
                            }
                        }
                    }
                }
            }
        }

        for line in buffer.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    if let Some(content) = parsed["choices"][0]["delta"]["content"].as_str() {
                        full_response.push_str(content);
                        let _ = token_tx.send(content.to_string()).await;
                    }
                }
            }
        }

        Ok(full_response)
    }
}
