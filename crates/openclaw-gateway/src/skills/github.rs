use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::{json, Value};
use reqwest::Client;

use super::Skill;

pub struct GitHubSkill {
    client: Client,
    token: Option<String>,
}

impl GitHubSkill {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("OpenClaw/0.1.0")
                .build()
                .unwrap_or_default(),
            token,
        }
    }

    async fn api_get(&self, url: &str) -> Result<Value, String> {
        let mut req = self.client.get(url);
        if let Some(token) = &self.token {
            req = req.header("Authorization", format!("Bearer {}", token));
        }
        
        let res = req.send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("GitHub API error {}: {}", res.status(), res.text().await.unwrap_or_default()));
        }
        
        res.json().await.map_err(|e| e.to_string())
    }
}

#[async_trait]
impl Skill for GitHubSkill {
    fn name(&self) -> &str {
        "github"
    }
    
    fn description(&self) -> &str {
        "Interact with GitHub repositories, issues, and pull requests"
    }
    
    fn version(&self) -> &str {
        "1.1.0"
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
                name: "github_get_repo".to_string(),
                description: "Get information about a GitHub repository".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "owner": { "type": "string", "description": "Repository owner" },
                        "repo": { "type": "string", "description": "Repository name" }
                    },
                    "required": ["owner", "repo"]
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
                
                let url = format!("https://api.github.com/repos/{}/{}/issues?state={}", owner, repo, state);
                let issues = self.api_get(&url).await?;
                
                let mut out = format!("Issues for {}/{}:\n", owner, repo);
                if let Some(arr) = issues.as_array() {
                    for issue in arr.iter().take(10) {
                        let number = issue["number"].as_u64().unwrap_or(0);
                        let title = issue["title"].as_str().unwrap_or("No title");
                        let user = issue["user"]["login"].as_str().unwrap_or("unknown");
                        out.push_str(&format!("- #{} {} (by {})\n", number, title, user));
                    }
                    if arr.len() > 10 {
                        out.push_str("... and more\n");
                    }
                }
                Ok(out)
            }
            "github_get_repo" => {
                let owner = args["owner"].as_str().ok_or("Missing owner")?;
                let repo = args["repo"].as_str().ok_or("Missing repo")?;
                
                let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
                let data = self.api_get(&url).await?;
                
                let desc = data["description"].as_str().unwrap_or("No description");
                let stars = data["stargazers_count"].as_u64().unwrap_or(0);
                let lang = data["language"].as_str().unwrap_or("Unknown");
                
                Ok(format!(
                    "Repository: {}/{}\nDescription: {}\nLanguage: {}\nStars: {}\nLink: {}",
                    owner, repo, desc, lang, stars, data["html_url"].as_str().unwrap_or("")
                ))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can interact with GitHub repositories. Use the GitHub tools to:\n- List and create issues\n- Get repository information\n\nFormat owner/repo as 'owner/repo'.")
    }
}
