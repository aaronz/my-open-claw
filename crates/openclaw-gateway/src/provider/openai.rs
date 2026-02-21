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
        temperature: Option<f32>,
        tools: Option<&[openclaw_core::provider::ToolDefinition]>,
        token_tx: mpsc::Sender<String>,
    ) -> Result<openclaw_core::provider::CompletionResponse> {
        let mut api_messages: Vec<Value> = Vec::new();

        if let Some(sp) = system_prompt {
            api_messages.push(json!({
                "role": "system",
                "content": sp,
            }));
        }

        for m in messages {
            match m.role {
                Role::User => {
                    let mut content = vec![json!({
                        "type": "text",
                        "text": m.content,
                    })];

                    for img in &m.images {
                        content.push(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/jpeg;base64,{}", img),
                            }
                        }));
                    }

                    api_messages.push(json!({
                        "role": "user",
                        "content": content,
                    }));
                }
                Role::Assistant => {
                    let mut msg = json!({
                        "role": "assistant",
                        "content": m.content,
                    });
                    if !m.tool_calls.is_empty() {
                        let tc_json: Vec<Value> = m.tool_calls
                            .iter()
                            .map(|tc| {
                                json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.name,
                                        "arguments": tc.arguments.to_string()
                                    }
                                })
                            })
                            .collect();
                        msg["tool_calls"] = json!(tc_json);
                    }
                    api_messages.push(msg);
                }
                Role::Tool => {
                    if let Some(tr) = &m.tool_result {
                        api_messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tr.tool_call_id,
                            "content": tr.content
                        }));
                    }
                }
                Role::System => {}
            }
        }

        let mut body = json!({
            "model": model,
            "messages": api_messages,
            "stream": true,
        });

        if let Some(mt) = max_tokens {
            body["max_tokens"] = json!(mt);
        }

        if let Some(t) = temperature {
            body["temperature"] = json!(t);
        }

        if let Some(tools) = tools {
            let openai_tools: Vec<Value> = tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect();
            body["tools"] = json!(openai_tools);
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
        let mut pending_tools: std::collections::HashMap<u64, openclaw_core::provider::ToolCall> = std::collections::HashMap::new();
        // Temporary storage for arguments string builder
        let mut pending_args: std::collections::HashMap<u64, String> = std::collections::HashMap::new();

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
                            let choice = &parsed["choices"][0];
                            let delta = &choice["delta"];

                            // Handle content
                            if let Some(content) = delta["content"].as_str() {
                                full_response.push_str(content);
                                let _ = token_tx.send(content.to_string()).await;
                            }

                            // Handle tool calls
                            if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                for tc in tool_calls {
                                    let index = tc["index"].as_u64().unwrap_or(0);
                                    
                                    if let Some(id) = tc["id"].as_str() {
                                        // New tool call starting
                                        pending_tools.insert(index, openclaw_core::provider::ToolCall {
                                            id: id.to_string(),
                                            name: String::new(), // Will be filled below
                                            arguments: serde_json::Value::Null, // Will be filled at end
                                        });
                                        pending_args.insert(index, String::new());
                                    }

                                    if let Some(function) = tc.get("function") {
                                        if let Some(name) = function["name"].as_str() {
                                            if let Some(pt) = pending_tools.get_mut(&index) {
                                                pt.name = name.to_string();
                                            }
                                        }
                                        if let Some(args) = function["arguments"].as_str() {
                                            if let Some(pa) = pending_args.get_mut(&index) {
                                                pa.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process remaining buffer
        for line in buffer.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                    if let Some(choices) = parsed.get("choices") {
                        if let Some(choice) = choices.get(0) {
                            if let Some(delta) = choice.get("delta") {
                                // Handle content
                                if let Some(content) = delta["content"].as_str() {
                                    full_response.push_str(content);
                                    let _ = token_tx.send(content.to_string()).await;
                                }

                                // Handle tool calls
                                if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                    for tc in tool_calls {
                                        let index = tc["index"].as_u64().unwrap_or(0);
                                        
                                        if let Some(id) = tc["id"].as_str() {
                                            pending_tools.insert(index, openclaw_core::provider::ToolCall {
                                                id: id.to_string(),
                                                name: String::new(),
                                                arguments: serde_json::Value::Null,
                                            });
                                            pending_args.insert(index, String::new());
                                        }

                                        if let Some(function) = tc.get("function") {
                                            if let Some(name) = function["name"].as_str() {
                                                if let Some(pt) = pending_tools.get_mut(&index) {
                                                    pt.name = name.to_string();
                                                }
                                            }
                                            if let Some(args) = function["arguments"].as_str() {
                                                if let Some(pa) = pending_args.get_mut(&index) {
                                                    pa.push_str(args);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Finalize tool calls
        let mut tool_calls = Vec::new();
        // Sort by index to maintain order
        let mut indices: Vec<u64> = pending_tools.keys().cloned().collect();
        indices.sort();
        
        for index in indices {
            if let Some(mut tool) = pending_tools.remove(&index) {
                if let Some(args_str) = pending_args.get(&index) {
                    if let Ok(args_json) = serde_json::from_str(args_str) {
                        tool.arguments = args_json;
                        tool_calls.push(tool);
                    } else {
                         tracing::warn!("failed to parse arguments for tool {}: {}", tool.name, args_str);
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
