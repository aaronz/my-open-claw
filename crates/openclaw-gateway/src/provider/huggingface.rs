use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::{CompletionResponse, Provider, ToolDefinition};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct HuggingFaceProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl HuggingFaceProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }
}

#[async_trait]
impl Provider for HuggingFaceProvider {
    fn name(&self) -> &str {
        "huggingface"
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
        let url = format!(
            "https://api-inference.huggingface.co/models/{}",
            self.model
        );

        let mut prompt = String::new();
        if let Some(sp) = system_prompt {
            prompt.push_str(sp);
            prompt.push_str("\n\n");
        }

        for msg in messages {
            let role = match msg.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
                Role::System => "System",
                Role::Tool => "Tool",
            };
            prompt.push_str(&format!("{}: {}\n", role, msg.content));
        }
        prompt.push_str("Assistant:");

        let body = json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": max_tokens.unwrap_or(1024),
                "temperature": temperature.unwrap_or(0.7),
                "return_full_text": false
            }
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Provider(format!("HuggingFace error {}: {}", status, text)));
        }

        let json: Value = response.json().await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        let content = if let Some(arr) = json.as_array() {
            arr.first()
                .and_then(|v| v["generated_text"].as_str())
                .unwrap_or("")
                .to_string()
        } else {
            json["generated_text"].as_str().unwrap_or("").to_string()
        };

        let _ = token_tx.send(content.clone()).await;

        Ok(CompletionResponse {
            content,
            tool_calls: vec![],
        })
    }
}
