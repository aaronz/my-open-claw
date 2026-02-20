use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct WeatherSkill;

#[async_trait]
impl Skill for WeatherSkill {
    fn name(&self) -> &str {
        "weather"
    }
    
    fn description(&self) -> &str {
        "Get current weather and forecasts for any location"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get current weather for a location".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name or coordinates"
                    }
                },
                "required": ["location"]
            }),
        }]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "get_weather" => {
                let location = args["location"]
                    .as_str()
                    .ok_or("Missing location")?;
                Ok(format!("Weather for {}: Partly cloudy, 72°F (22°C), humidity 65%", location))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You have access to a weather tool. Use it to check weather conditions when users ask about weather, forecasts, or travel advice.")
    }
}
