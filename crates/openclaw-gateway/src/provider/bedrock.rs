use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::{CompletionResponse, Provider, ToolDefinition};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct BedrockProvider {
    client: reqwest::Client,
    region: String,
    model_id: String,
    access_key: String,
    secret_key: String,
}

impl BedrockProvider {
    pub fn new(region: String, model_id: String, access_key: String, secret_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            region,
            model_id,
            access_key,
            secret_key,
        }
    }

    fn sign_request(&self, _method: &str, _path: &str, _body: &[u8]) -> (String, String) {
        let amz_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = chrono::Utc::now().format("%Y%m%d").to_string();
        
        (format!("AWS4-HMAC-SHA256 Credential={}/{}/bedrock/aws4_request, SignedHeaders=host;x-amz-date, Signature=dummy", 
            self.access_key, date_stamp), amz_date)
    }
}

#[async_trait]
impl Provider for BedrockProvider {
    fn name(&self) -> &str {
        "bedrock"
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
        let model_id = urlencoding::encode(&self.model_id);
        let url = format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/invoke-with-response-stream",
            self.region, model_id
        );

        let mut converse_messages: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    _ => "user",
                };
                json!({
                    "role": role,
                    "content": [{
                        "text": m.content
                    }]
                })
            })
            .collect();

        let mut body = json!({
            "messages": converse_messages
        });

        if let Some(sp) = system_prompt {
            body["system"] = json!([{ "text": sp }]);
        }

        body["inferenceConfig"] = json!({
            "maxTokens": max_tokens.unwrap_or(4096),
            "temperature": temperature.unwrap_or(0.7)
        });

        let (auth_header, amz_date) = self.sign_request("POST", &url, &[]);

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-Amz-Date", amz_date)
            .header("Authorization", auth_header)
            .header("Accept", "application/vnd.amazon.eventstream")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Provider(format!("Bedrock error {}: {}", status, text)));
        }

        let mut full_response = String::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e: reqwest::Error| OpenClawError::Provider(e.to_string()))?;
            
            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                if let Some(content) = extract_bedrock_content(&text) {
                    full_response.push_str(&content);
                    let _ = token_tx.send(content).await;
                }
            }
        }

        Ok(CompletionResponse {
            content: full_response,
            tool_calls: vec![],
        })
    }
}

fn extract_bedrock_content(data: &str) -> Option<String> {
    for line in data.lines() {
        if line.contains("\"text\":") {
            if let Ok(json) = serde_json::from_str::<Value>(line) {
                if let Some(text) = json["contentBlockDelta"]["delta"]["text"].as_str() {
                    return Some(text.to_string());
                }
            }
        }
    }
    None
}

mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut result = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
                _ => {
                    for b in c.to_string().as_bytes() {
                        result.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        result
    }
}
