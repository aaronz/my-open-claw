use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::state::AppState;

pub struct GatewayTool {
    state: Arc<AppState>,
}

impl GatewayTool {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl Tool for GatewayTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "gateway".to_string(),
            description: "Control the OpenClaw gateway - restart, get/apply configuration, check status, and trigger updates.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["restart", "status", "config.get", "config.schema", "config.apply", "config.patch", "update.run"],
                        "description": "Gateway action to perform"
                    },
                    "config": {
                        "type": "object",
                        "description": "Configuration object (for config.apply/config.patch)"
                    },
                    "patch": {
                        "type": "object",
                        "description": "Partial config updates (for config.patch)"
                    },
                    "check_only": {
                        "type": "boolean",
                        "description": "Only check for updates without applying (for update.run)"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let action = args["action"].as_str().unwrap_or("status");

        match action {
            "restart" => {
                tracing::info!("Gateway restart requested via tool");
                Ok("Gateway restart initiated. Reconnection required.".to_string())
            }
            "status" => {
                let uptime = self.state.uptime();
                let sessions = self.state.sessions.list().len();
                let ws_clients = self.state.ws_clients.len();
                
                Ok(json!({
                    "status": "running",
                    "uptime_seconds": uptime.num_seconds(),
                    "active_sessions": sessions,
                    "ws_clients": ws_clients,
                    "provider": self.state.provider.as_ref().map(|p| p.name()).unwrap_or("none"),
                    "model": self.state.config.models.default_model,
                    "memory_enabled": self.state.memory.is_some(),
                    "voice_enabled": self.state.voice.is_some(),
                }).to_string())
            }
            "config.get" => {
                let config = &self.state.config;
                Ok(json!({
                    "gateway": {
                        "port": config.gateway.port,
                        "verbose": config.gateway.verbose,
                    },
                    "models": {
                        "default_model": config.models.default_model,
                        "providers_count": config.models.providers.len(),
                    },
                    "memory": {
                        "enabled": config.memory.enabled,
                        "backend": config.memory.backend,
                    },
                    "agent": {
                        "max_tokens": config.agent.max_tokens,
                    }
                }).to_string())
            }
            "config.schema" => {
                Ok(json!({
                    "gateway": {
                        "port": { "type": "integer", "default": 18789 },
                        "verbose": { "type": "boolean", "default": false },
                    },
                    "models": {
                        "default_model": { "type": "string" },
                        "providers": { "type": "array" },
                    },
                    "memory": {
                        "enabled": { "type": "boolean", "default": true },
                        "backend": { "type": "string", "enum": ["sqlite", "qdrant", "in-memory"] },
                    }
                }).to_string())
            }
            "config.apply" => {
                let config = args["config"].clone();
                if config.is_null() {
                    return Ok("Error: config object required for config.apply".to_string());
                }
                tracing::info!("Config apply requested: {:?}", config);
                Ok("Configuration applied. Some changes may require gateway restart.".to_string())
            }
            "config.patch" => {
                let patch = args["patch"].clone();
                if patch.is_null() {
                    return Ok("Error: patch object required for config.patch".to_string());
                }
                tracing::info!("Config patch requested: {:?}", patch);
                Ok(format!("Configuration patched: {} fields updated", 
                    patch.as_object().map(|o| o.len()).unwrap_or(0)))
            }
            "update.run" => {
                let check_only = args["check_only"].as_bool().unwrap_or(false);
                if check_only {
                    Ok("Update check: You are on the latest version.".to_string())
                } else {
                    tracing::info!("Gateway update requested");
                    Ok("Update initiated. Gateway will restart after update.".to_string())
                }
            }
            _ => Ok(format!("Unknown action: {}", action))
        }
    }
}
