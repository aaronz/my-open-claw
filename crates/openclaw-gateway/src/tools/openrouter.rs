use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use reqwest::Client;

pub struct OpenRouterTool {
    client: Client,
    api_key: String,
}

impl OpenRouterTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Tool for OpenRouterTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search_openrouter".to_string(),
            description: "Search or query specialized models via OpenRouter for advanced web research and synthesis.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search or research query"
                    },
                    "model": {
                        "type": "string",
                        "description": "Specific OpenRouter model to use (optional)",
                        "default": "perplexity/sonar-reasoning"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing query".to_string()))?;
        let model = args["model"].as_str().unwrap_or("perplexity/sonar-reasoning");

        let body = json!({
            "model": model,
            "messages": [
                {
                    "role": "user",
                    "content": query
                }
            ]
        });

        let res = self.client.post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/openclaw/openclaw")
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("OpenRouter error: {}", err)));
        }

        let json: Value = res.json().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("No content").to_string();
        
        Ok(content)
    }
}
