use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};

pub struct SearchTool {
    api_key: String,
    client: reqwest::Client,
}

impl SearchTool {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for SearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web for up-to-date information.".to_string(),
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
        let query = args["query"].as_str().unwrap_or("");
        if query.is_empty() {
            return Ok("Empty query".to_string());
        }

        let url = "https://api.tavily.com/search";
        let body = json!({
            "api_key": self.api_key,
            "query": query,
            "search_depth": "basic",
            "include_answer": true,
            "max_results": 5
        });

        let res = self.client.post(url).json(&body).send().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("Tavily error: {}", err)));
        }

        let json: Value = res.json().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        // Extract answer or results
        if let Some(answer) = json.get("answer").and_then(|v| v.as_str()) {
            return Ok(format!("Answer: {}\n\nResults: {:?}", answer, json["results"]));
        }
        
        Ok(format!("Results: {:?}", json["results"]))
    }
}
