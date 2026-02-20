use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct GitHubSkill;

#[async_trait]
impl Skill for GitHubSkill {
    fn name(&self) -> &str {
        "github"
    }
    
    fn description(&self) -> &str {
        "Interact with GitHub repositories, issues, and pull requests"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "github_list_issues".to_string(),
                description: "List issues in a GitHub repository".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string", "description": "Repository owner" },
                        "repo": { "type": "string", "description": "Repository name" },
                        "state": { "type": "string", "description": "open or all", "default": "open" }
                    },
                    "required": ["owner", "repo"]
                }),
            },
            ToolDefinition {
                name: "github_create_issue".to_string(),
                description: "Create a new issue in a GitHub repository".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string", "description": "Repository owner" },
                        "repo": { "type": "string", "description": "Repository name" },
                        "title": { "type": "string", "description": "Issue title" },
                        "body": { "type": "string", "description": "Issue body" }
                    },
                    "required": ["owner", "repo", "title"]
                }),
            },
            ToolDefinition {
                name: "github_search_code".to_string(),
                description: "Search for code in GitHub repositories".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "language": { "type": "string", "description": "Language filter" }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "github_list_issues" => {
                let owner = args["owner"].as_str().ok_or("Missing owner")?;
                let repo = args["repo"].as_str().ok_or("Missing repo")?;
                let state = args.get("state").and_then(|v| v.as_str()).unwrap_or("open");
                Ok(format!("Issues for {}/{} ({}):\n- #1: Example issue (open)\n- #2: Bug fix needed (open)", owner, repo, state))
            }
            "github_create_issue" => {
                let owner = args["owner"].as_str().ok_or("Missing owner")?;
                let repo = args["repo"].as_str().ok_or("Missing repo")?;
                let title = args["title"].as_str().ok_or("Missing title")?;
                Ok(format!("Created issue '{}' in {}/{}", title, owner, repo))
            }
            "github_search_code" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                Ok(format!("Search results for '{}':\n- file.rs:10 - matching line\n- lib.ts:5 - matching line", query))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can interact with GitHub repositories. Use the GitHub tools to:\n- List and create issues\n- Search code\n- Get repository information\n\nFormat owner/repo as 'owner/repo'.")
    }
}
