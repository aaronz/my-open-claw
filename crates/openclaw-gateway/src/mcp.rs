use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use openclaw_core::{Tool, Result as CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub server_cmd: String,
    pub server_args: Vec<String>,
}

impl McpTool {
    pub fn new(name: String, description: String, parameters: Value, cmd: String, args: Vec<String>) -> Self {
        Self {
            name,
            description,
            parameters,
            server_cmd: cmd,
            server_args: args,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters.clone(),
        }
    }

    async fn execute(&self, args: Value) -> CoreResult<String> {
        let mut child = Command::new(&self.server_cmd)
            .args(&self.server_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to spawn MCP server: {}", e)))?;

        let mut stdin = child.stdin.take().ok_or_else(|| openclaw_core::OpenClawError::Provider("Failed to open stdin".to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| openclaw_core::OpenClawError::Provider("Failed to open stdout".to_string()))?;
        let mut reader = BufReader::new(stdout);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": self.name,
                "arguments": args
            }
        });

        let req_str = serde_json::to_string(&request).map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        stdin.write_all(format!("{}\n", req_str).as_bytes()).await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        stdin.flush().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let mut line = String::new();
        reader.read_line(&mut line).await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let response: Value = serde_json::from_str(&line).map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        
        let _ = child.kill().await;

        if let Some(content) = response.get("result").and_then(|r| r.get("content")).and_then(|c| c.as_array()) {
            let text: Vec<String> = content.iter()
                .filter_map(|item| item.get("text").and_then(|t| t.as_str()).map(|s| s.to_string()))
                .collect();
            Ok(text.join("\n"))
        } else {
            Ok(format!("MCP Error: {:?}", response.get("error")))
        }
    }
}

pub struct McpManager {
    pub tools: Arc<Mutex<Vec<McpTool>>>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_server(&self, cmd: String, args: Vec<String>) -> CoreResult<()> {
        let mut child = Command::new(&cmd)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to spawn MCP server for discovery: {}", e)))?;

        let mut stdin = child.stdin.take().ok_or_else(|| openclaw_core::OpenClawError::Provider("Failed to open stdin".to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| openclaw_core::OpenClawError::Provider("Failed to open stdout".to_string()))?;
        let mut reader = BufReader::new(stdout);

        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "tools/list",
            "params": {}
        });

        let req_str = serde_json::to_string(&request).unwrap();
        stdin.write_all(format!("{}\n", req_str).as_bytes()).await.unwrap();
        stdin.flush().await.unwrap();

        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();

        let response: Value = serde_json::from_str(&line).unwrap();
        if let Some(tools_arr) = response.get("result").and_then(|r| r.get("tools")).and_then(|t| t.as_array()) {
            let mut guard = self.tools.lock().await;
            for t in tools_arr {
                let name = t["name"].as_str().unwrap_or("unknown").to_string();
                let desc = t["description"].as_str().unwrap_or("").to_string();
                let params = t["inputSchema"].clone();
                
                guard.push(McpTool::new(name, desc, params, cmd.clone(), args.clone()));
            }
        }

        let _ = child.kill().await;
        Ok(())
    }

    pub async fn get_tools(&self) -> Vec<ToolDefinition> {
        let guard = self.tools.lock().await;
        guard.iter().map(|t| t.definition()).collect()
    }
}
