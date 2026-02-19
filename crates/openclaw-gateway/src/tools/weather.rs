use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::json;

pub struct WeatherTool;

#[async_trait]
impl Tool for WeatherTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                },
                "required": ["location"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String> {
        let location = args["location"].as_str().unwrap_or("unknown");
        // Mock response
        Ok(format!("The weather in {} is sunny and 72°F", location))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weather_tool() {
        let tool = WeatherTool;
        let args = json!({ "location": "London" });
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("London"));
        assert!(result.contains("sunny"));
    }
}
