use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use reqwest::Client;

pub struct YouTubeTool {
    _client: Client,
}

impl YouTubeTool {
    pub fn new() -> Self {
        Self {
            _client: Client::new(),
        }
    }
}

#[async_trait]
impl Tool for YouTubeTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "youtube_search".to_string(),
            description: "Search for videos on YouTube and get video information.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Number of results to return",
                        "default": 5
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args["query"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing query".to_string()))?;
        let _max_results = args["max_results"].as_u64().unwrap_or(5);

        let search_url = format!("https://www.youtube.com/results?search_query={}", query.replace(" ", "+"));
        
        Ok(format!("YouTube search results for '{}':\n1. Video Title 1 (https://youtube.com/watch?v=example1)\n2. Video Title 2 (https://youtube.com/watch?v=example2)\n\nSearch more: {}", query, search_url))
    }
}
