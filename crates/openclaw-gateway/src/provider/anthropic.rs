use async_trait::async_trait;
use futures::StreamExt;
use openclaw_core::error::{OpenClawError, Result};
use openclaw_core::provider::Provider;
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use tokio::sync::mpsc;

pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn stream_chat(
        &self,
        messages: &[ChatMessage],
        system_prompt: Option<&str>,
        model: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
        tools: Option<&[openclaw_core::provider::ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<openclaw_core::provider::CompletionResponse> {
        let api_messages: Vec<Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| match m.role {
                Role::User => json!({
                    "role": "user",
                    "content": m.content,
                }),
                Role::Assistant => {
                    if !m.tool_calls.is_empty() {
                        let mut content: Vec<Value> = Vec::new();
                        if !m.content.is_empty() {
                            content.push(json!({
                                "type": "text",
                                "text": m.content
                            }));
                        }
                        for tc in &m.tool_calls {
                            content.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments
                            }));
                        }
                        json!({
                            "role": "assistant",
                            "content": content
                        })
                    } else {
                        json!({
                            "role": "assistant",
                            "content": m.content
                        })
                    }
                }
                Role::Tool => {
                    if let Some(tr) = &m.tool_result {
                        json!({
                            "role": "user",
                            "content": [{
                                "type": "tool_result",
                                "tool_use_id": tr.tool_call_id,
                                "content": tr.content
                            }]
                        })
                    } else {
                        json!({
                            "role": "user",
                            "content": m.content
                        })
                    }
                }
                Role::System => json!({ "role": "user", "content": "" }),
            })
            .collect();

        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
            "max_tokens": max_tokens.unwrap_or(4096),
        });

        if let Some(t) = temperature {
            body["temperature"] = json!(t);
        }

        if let Some(sp) = system_prompt {
            body["system"] = json!(sp);
        }

        if let Some(tools) = tools {
            let anthropic_tools: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters
                    })
                })
                .collect();
            body["tools"] = json!(anthropic_tools);
        }

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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
                "Anthropic API error {status}: {text}"
            )));
        }

        let mut full_response = String::new();
        let mut tool_calls = Vec::new();
        let mut current_tool_id: Option<String> = None;
        let mut current_tool_name: Option<String> = None;
        let mut current_tool_input = String::new();

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
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            match parsed["type"].as_str() {
                                Some("content_block_start") => {
                                    if parsed["content_block"]["type"] == "tool_use" {
                                        current_tool_id = parsed["content_block"]["id"]
                                            .as_str()
                                            .map(String::from);
                                        current_tool_name = parsed["content_block"]["name"]
                                            .as_str()
                                            .map(String::from);
                                        current_tool_input.clear();
                                    }
                                }
                                Some("content_block_delta") => {
                                    if let Some(delta) = parsed.get("delta") {
                                        if delta["type"] == "text_delta" {
                                            if let Some(text) = delta["text"].as_str() {
                                                full_response.push_str(text);
                                                let _ = token_tx.send(text.to_string()).await;
                                            }
                                        } else if delta["type"] == "input_json_delta" {
                                            if let Some(partial) = delta["partial_json"].as_str() {
                                                current_tool_input.push_str(partial);
                                            }
                                        }
                                    }
                                }
                                Some("content_block_stop") => {
                                    if let (Some(id), Some(name)) =
                                        (&current_tool_id, &current_tool_name)
                                    {
                                        if let Ok(args) =
                                            serde_json::from_str(&current_tool_input)
                                        {
                                            tool_calls.push(openclaw_core::provider::ToolCall {
                                                id: id.clone(),
                                                name: name.clone(),
                                                arguments: args,
                                            });
                                        }
                                        current_tool_id = None;
                                        current_tool_name = None;
                                        current_tool_input.clear();
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // Process any remaining data in the buffer
        for line in buffer.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    match parsed["type"].as_str() {
                        Some("content_block_start") => {
                            if parsed["content_block"]["type"] == "tool_use" {
                                current_tool_id = parsed["content_block"]["id"]
                                    .as_str()
                                    .map(String::from);
                                current_tool_name = parsed["content_block"]["name"]
                                    .as_str()
                                    .map(String::from);
                                current_tool_input.clear();
                            }
                        }
                        Some("content_block_delta") => {
                            if let Some(delta) = parsed.get("delta") {
                                if delta["type"] == "text_delta" {
                                    if let Some(text) = delta["text"].as_str() {
                                        full_response.push_str(text);
                                        let _ = token_tx.send(text.to_string()).await;
                                    }
                                } else if delta["type"] == "input_json_delta" {
                                    if let Some(partial) = delta["partial_json"].as_str() {
                                        current_tool_input.push_str(partial);
                                    }
                                }
                            }
                        }
                        Some("content_block_stop") => {
                            if let (Some(id), Some(name)) =
                                (&current_tool_id, &current_tool_name)
                            {
                                if let Ok(args) =
                                    serde_json::from_str(&current_tool_input)
                                {
                                    tool_calls.push(openclaw_core::provider::ToolCall {
                                        id: id.clone(),
                                        name: name.clone(),
                                        arguments: args,
                                    });
                                }
                                current_tool_id = None;
                                current_tool_name = None;
                                current_tool_input.clear();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(openclaw_core::provider::CompletionResponse {
            content: full_response,
            tool_calls,
        })
    }
}
