use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::{CompletionResponse, Provider, ToolCall, ToolDefinition};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<String>, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model,
        }
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        _model: &str,
        max_tokens: Option<u32>,
        _temperature: Option<f32>,
        tools: Option<&[ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<CompletionResponse> {
        let url = format!("{}/api/chat", self.base_url);
        
        let mut ollama_messages: Vec<Value> = messages
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
            ollama_messages.insert(0, json!({
                "role": "system",
                "content": sp
            }));
        }

        let mut body = json!({
            "model": self.model,
            "messages": ollama_messages,
            "stream": true
        });

        if let Some(mt) = max_tokens {
            body["options"]["num_predict"] = json!(mt);
        }

        let response = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Provider(format!("Ollama error {}: {}", status, text)));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e: reqwest::Error| OpenClawError::Provider(e.to_string()))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() { continue; }

                if let Ok(parsed) = serde_json::from_str::<Value>(&line) {
                    if let Some(content) = parsed["message"]["content"].as_str() {
                        full_response.push_str(content);
                        let _ = token_tx.send(content.to_string()).await;
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
