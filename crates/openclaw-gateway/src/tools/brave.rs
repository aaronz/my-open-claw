use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use reqwest::Client;

pub struct BraveSearchTool {
    client: Client,
    api_key: String,
}

impl BraveSearchTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Tool for BraveSearchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_search_brave".to_string(),
            description: "Search the web using Brave Search API for accurate and private search results.".to_string(),
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

        let res = self.client.get("https://api.search.brave.com/res/v1/web/search")
            .header("X-Subscription-Token", &self.api_key)
            .query(&[("q", query)])
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("Brave Search error: {}", err)));
        }

        let json: Value = res.json().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        
        let mut results = Vec::new();
        if let Some(web) = json["web"]["results"].as_array() {
            for res in web.iter().take(5) {
                let title = res["title"].as_str().unwrap_or("");
                let desc = res["description"].as_str().unwrap_or("");
                let url = res["url"].as_str().unwrap_or("");
                results.push(format!("### {}\n{}\nSource: {}", title, desc, url));
            }
        }

        if results.is_empty() {
            Ok("No results found.".to_string())
        } else {
            Ok(results.join("\n\n"))
        }
    }
}
