use crate::state::AppState;
use axum::{extract::State, routing::{get, post}, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/config", get(get_config))
        .route("/api/status", get(status))
        .route("/api/webhook", post(webhook))
}

async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "status": "ok",
        "uptime_secs": state.uptime().num_seconds(),
    }))
}

async fn list_sessions(State(state): State<Arc<AppState>>) -> Json<Value> {
    let sessions = state.sessions.list();
    Json(json!({ "sessions": sessions }))
}

async fn get_config(State(state): State<Arc<AppState>>) -> Json<Value> {
    let mut config = serde_json::to_value(&state.config).unwrap_or_default();
    redact_secrets(&mut config);
    Json(config)
}

async fn status(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "session_count": state.sessions.list().len(),
        "client_count": state.ws_clients.len(),
        "uptime_secs": state.uptime().num_seconds(),
    }))
}

async fn webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let source = body["source"].as_str().unwrap_or("unknown");
    let content = body["content"].as_str().unwrap_or("");
    let channel = body["channel"].as_str().unwrap_or("api");
    let peer_id = body["peer_id"].as_str().unwrap_or("webhook");

    if content.is_empty() {
        return Json(json!({ "error": "content is required" }));
    }

    let msg = openclaw_core::WsMessage::SendMessage {
        session_id: None,
        content: content.to_string(),
        channel: None,
        peer_id: Some(format!("{channel}:{peer_id}")),
    };

    if let Ok(json_msg) = serde_json::to_string(&msg) {
        state.broadcast(&json_msg);
    }

    tracing::info!(source = source, channel = channel, "webhook received");

    Json(json!({
        "status": "accepted",
        "source": source,
    }))
}

fn redact_secrets(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                if key == "password" || key == "token" || key == "api_key" {
                    if val.is_string() {
                        *val = Value::String("***".to_string());
                    }
                } else {
                    redact_secrets(val);
                }
            }
        }
        Value::Array(arr) => {
            for val in arr.iter_mut() {
                redact_secrets(val);
            }
        }
        _ => {}
    }
}
