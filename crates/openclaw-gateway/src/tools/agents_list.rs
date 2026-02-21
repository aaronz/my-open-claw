use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};

pub struct AgentsListTool;

#[async_trait]
impl Tool for AgentsListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "agents_list".to_string(),
            description: "List available agent IDs that can be used as targets for spawning sub-agents via sessions_spawn tool.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "include_details": {
                        "type": "boolean",
                        "description": "Include full agent details (default: false)"
                    }
                }
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let include_details = args["include_details"].as_bool().unwrap_or(false);

        let agents = vec![
            ("default", "Default assistant agent", "claude-sonnet-4-20250514"),
            ("coder", "Code-focused agent", "claude-sonnet-4-20250514"),
            ("researcher", "Research and analysis agent", "claude-sonnet-4-20250514"),
            ("writer", "Content creation agent", "claude-sonnet-4-20250514"),
        ];

        if include_details {
            let details: Vec<Value> = agents.iter().map(|(id, desc, model)| {
                json!({
                    "id": id,
                    "description": desc,
                    "model": model,
                    "tools": "all",
                    "sandbox": "off"
                })
            }).collect();
            Ok(json!({ "agents": details }).to_string())
        } else {
            let ids: Vec<&str> = agents.iter().map(|(id, _, _)| *id).collect();
            Ok(json!({ "agent_ids": ids }).to_string())
        }
    }
}
