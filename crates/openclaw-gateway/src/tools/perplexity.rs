use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use reqwest::Client;

pub struct PerplexityTool {
    client: Client,
    api_key: String,
}

impl PerplexityTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Tool for PerplexityTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search_perplexity".to_string(),
            description: "Search the web using Perplexity AI for real-time information and deep research.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing query".to_string()))?;

        let body = json!({
            "model": "llama-3.1-sonar-small-128k-online",
            "messages": [
                {
                    "role": "system",
                    "content": "Be concise and provide real-time information."
                },
                {
                    "role": "user",
                    "content": query
                }
            ]
        });

        let res = self.client.post("https://api.perplexity.ai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("Perplexity error: {}", err)));
        }

        let json: Value = res.json().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        let content = json["choices"][0]["message"]["content"].as_str().unwrap_or("No content").to_string();
        
        Ok(content)
    }
}
