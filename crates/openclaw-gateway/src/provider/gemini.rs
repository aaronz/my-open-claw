use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::{CompletionResponse, Provider, ToolCall, ToolDefinition};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    model_name: String, // e.g. "gemini-1.5-pro"
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model_name: model,
        }
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        _model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tools: Option<&[ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<CompletionResponse> {
        let model = if _model.contains("gemini") {
            _model
        } else {
            &self.model_name
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            model, self.api_key
        );

        let mut contents: Vec<Value> = Vec::new();

        for m in messages {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "model",
                Role::System => "user",
                Role::Tool => "user",
            };

            let mut parts = if !m.tool_calls.is_empty() {
                m.tool_calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "functionCall": {
                                "name": tc.name,
                                "args": tc.arguments
                            }
                        })
                    })
                    .collect::<Vec<_>>()
            } else if let Some(tr) = &m.tool_result {
                vec![json!({
                    "functionResponse": {
                        "name": tr.tool_call_id,
                        "response": { "content": tr.content }
                    }
                })]
            } else {
                vec![json!({ "text": m.content })]
            };

            if m.role == Role::User && !m.images.is_empty() {
                for img in &m.images {
                    parts.push(json!({
                        "inlineData": {
                            "mimeType": "image/jpeg",
                            "data": img
                        }
                    }));
                }
            }

            contents.push(json!({
                "role": role,
                "parts": parts
            }));
        }

        let mut body = json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": max_tokens.unwrap_or(8192)
            }
        });

        if let Some(t) = temperature {
            body["generationConfig"]["temperature"] = json!(t);
        }

        if let Some(sp) = system_prompt {
            body["systemInstruction"] = json!({
                "parts": [{ "text": sp }]
            });
        }

        if let Some(tools) = tools {
            let gemini_tools: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    })
                })
                .collect();

            body["tools"] = json!([{
                "function_declarations": gemini_tools
            }]);
        }

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Provider(format!(
                "Gemini API error {}: {}",
                status, text
            )));
        }

        let mut full_response = String::new();
        let mut tool_calls = Vec::new();

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e: reqwest::Error| OpenClawError::Provider(e.to_string()))?;
            let s = String::from_utf8_lossy(&chunk);
            buffer.push_str(&s);

            let mut consumed_bytes = 0;

            loop {
                let slice = &buffer[consumed_bytes..];
                if slice.is_empty() {
                    break;
                }

                let start_offset = if let Some(idx) = slice.find('{') {
                    idx
                } else {
                    break;
                };

                let object_start = consumed_bytes + start_offset;
                let check_slice = &buffer[object_start..];

                let mut balance = 0;
                let mut in_string = false;
                let mut escape = false;
                let mut end_offset = None;

                for (i, c) in check_slice.char_indices() {
                    if escape {
                        escape = false;
                    } else if c == '\\' {
                        escape = true;
                    } else if c == '"' {
                        in_string = !in_string;
                    } else if !in_string {
                        if c == '{' {
                            balance += 1;
                        } else if c == '}' {
                            balance -= 1;
                            if balance == 0 {
                                end_offset = Some(i + 1);
                                break;
                            }
                        }
                    }
                }

                if let Some(len) = end_offset {
                    let json_str = &buffer[object_start..object_start + len];
                    if let Ok(parsed) = serde_json::from_str::<Value>(json_str) {
                        if let Some(candidates) = parsed["candidates"].as_array() {
                            if let Some(candidate) = candidates.first() {
                                if let Some(content) = candidate["content"].as_object() {
                                    if let Some(parts) = content["parts"].as_array() {
                                        for part in parts {
                                            if let Some(text) = part["text"].as_str() {
                                                full_response.push_str(text);
                                                let _ = token_tx.send(text.to_string()).await;
                                            }
                                            if let Some(func_call) = part.get("functionCall") {
                                                let name = func_call["name"]
                                                    .as_str()
                                                    .unwrap_or("")
                                                    .to_string();
                                                let args = func_call["args"].clone();
                                                tool_calls.push(ToolCall {
                                                    id: format!("call_{}", Uuid::new_v4()),
                                                    name,
                                                    arguments: args,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        consumed_bytes = object_start + len;
                    } else {
                        // Should not happen if balance logic is correct and valid JSON
                        consumed_bytes = object_start + 1;
                    }
                } else {
                    break;
                }
            }

            if consumed_bytes > 0 {
                buffer.drain(..consumed_bytes);
            }
        }

        Ok(CompletionResponse {
            content: full_response,
            tool_calls,
        })
    }
}
