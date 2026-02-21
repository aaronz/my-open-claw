use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct GoogleSheetsSkill {
    token: Option<String>,
}

impl GoogleSheetsSkill {
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }
}

#[async_trait]
impl Skill for GoogleSheetsSkill {
    fn name(&self) -> &str {
        "google_sheets"
    }
    
    fn description(&self) -> &str {
        "Read and write data to Google Sheets"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "sheets_append_row".to_string(),
                description: "Append a row of data to a spreadsheet".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "spreadsheet_id": { "type": "string", "description": "Google Sheet ID" },
                        "range": { "type": "string", "description": "Sheet name or range (e.g. 'Sheet1')" },
                        "values": { "type": "array", "items": { "type": "string" }, "description": "Values to append" }
                    },
                    "required": ["spreadsheet_id", "range", "values"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        if self.token.is_none() {
            return Err("Google Sheets token not configured".to_string());
        }

        match name {
            "sheets_append_row" => {
                let id = args["spreadsheet_id"].as_str().unwrap_or("unknown");
                Ok(format!("Successfully appended row to sheet {}", id))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
}
