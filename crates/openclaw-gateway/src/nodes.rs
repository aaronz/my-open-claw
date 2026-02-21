use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub online: bool,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCommand {
    pub command: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResponse {
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

pub struct NodeManager {
    nodes: Arc<Mutex<Vec<NodeInfo>>>,
}

impl NodeManager {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn register(&self, node: NodeInfo) {
        let mut nodes = self.nodes.lock().await;
        nodes.push(node);
    }

    pub async fn unregister(&self, id: &str) {
        let mut nodes = self.nodes.lock().await;
        nodes.retain(|n| n.id != id);
    }

    pub async fn list(&self) -> Vec<NodeInfo> {
        self.nodes.lock().await.clone()
    }

    pub async fn get(&self, id: &str) -> Option<NodeInfo> {
        self.nodes.lock().await.iter().find(|n| n.id == id).cloned()
    }

    pub async fn send_command(&self, _node_id: &str, command: NodeCommand) -> NodeResponse {
        match command.command.as_str() {
            "status" => NodeResponse {
                success: true,
                data: Some(serde_json::json!({ "status": "ok" })),
                error: None,
            },
            "camera_snap" => NodeResponse {
                success: true,
                data: Some(serde_json::json!({ "image": "base64..." })),
                error: None,
            },
            "location_get" => NodeResponse {
                success: true,
                data: Some(serde_json::json!({ "lat": 0.0, "lng": 0.0 })),
                error: None,
            },
            _ => NodeResponse {
                success: false,
                data: None,
                error: Some(format!("Unknown command: {}", command.command)),
            },
        }
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}
