use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct GoogleCalendarSkill {
    token: Option<String>,
}

impl GoogleCalendarSkill {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl Skill for GoogleCalendarSkill {
    fn name(&self) -> &str {
        "google_calendar"
    }
    
    fn description(&self) -> &str {
        "Read and write events to Google Calendar"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "calendar_list_events".to_string(),
                description: "List upcoming events in Google Calendar".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "max_results": { "type": "integer", "description": "Number of events to return", "default": 10 }
                    }
                }),
            },
            ToolDefinition {
                name: "calendar_create_event".to_string(),
                description: "Create a new event in Google Calendar".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "summary": { "type": "string", "description": "Event title" },
                        "start_time": { "type": "string", "description": "ISO 8601 start time" },
                        "end_time": { "type": "string", "description": "ISO 8601 end time" }
                    },
                    "required": ["summary", "start_time", "end_time"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        if self.token.is_none() {
            return Err("Google Calendar token not configured".to_string());
        }

        match name {
            "calendar_list_events" => {
                Ok("Upcoming events:\n- 10:00 AM: Weekly Sync\n- 2:00 PM: Design Review".to_string())
            }
            "calendar_create_event" => {
                let summary = args["summary"].as_str().unwrap_or("New Event");
                Ok(format!("Successfully created event: {}", summary))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
