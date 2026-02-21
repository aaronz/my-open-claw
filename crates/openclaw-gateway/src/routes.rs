use crate::agent::run_agent_cycle;
use crate::state::AppState;
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use openclaw_core::session::{ChatMessage, Role};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub fn api_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index_html))
        .route("/health", get(health))
        .route("/api/sessions", get(list_sessions))
        .route("/api/config", get(get_config))
        .route("/api/status", get(status))
        .route("/api/webhook", post(webhook))
        .route("/api/webhook/whatsapp", post(whatsapp_webhook))
        .route("/api/webhook/signal", post(signal_webhook))
        .route("/api/webhook/matrix", post(matrix_webhook))
        .route("/api/webhook/mattermost", post(mattermost_webhook))
        .route("/api/webhook/bluebubbles", post(bluebubbles_webhook))
        .route("/api/webhook/feishu", post(feishu_webhook))
        .route("/api/webhook/zulip", post(zulip_webhook))
        .route("/api/memory", post(ingest_memory))
}

async fn feishu_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::feishu::handle_feishu_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn zulip_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::zulip::handle_zulip_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn bluebubbles_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::bluebubbles::handle_bluebubbles_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn mattermost_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::mattermost::handle_mattermost_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn matrix_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::matrix::handle_matrix_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn whatsapp_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::whatsapp::handle_whatsapp_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn signal_webhook(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> impl axum::response::IntoResponse {
    let _ = crate::channels::signal::handle_signal_webhook(state, body).await;
    axum::http::StatusCode::OK
}

async fn index_html() -> impl axum::response::IntoResponse {
    axum::response::Html(include_str!("static/index.html"))
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
    let channel_str = body["channel"].as_str().unwrap_or("api");
    let peer_id = body["peer_id"].as_str().unwrap_or("webhook");

    if content.is_empty() {
        return Json(json!({ "error": "content is required" }));
    }

    let channel_kind: openclaw_core::ChannelKind =
        serde_json::from_value(json!(channel_str)).unwrap_or(openclaw_core::ChannelKind::Api);

    let session = state
        .sessions
        .get_or_create(channel_kind.clone(), peer_id);
    let session_id = session.id;

    let user_msg = ChatMessage {
        id: Uuid::new_v4(),
        role: Role::User,
        content: content.to_string(),
        timestamp: chrono::Utc::now(),
        channel: channel_kind,
        images: vec![],
        tool_calls: vec![],
        tool_result: None,
    };
    let _ = state.sessions.add_message(&session_id, user_msg.clone());

    let new_msg = openclaw_core::WsMessage::NewMessage {
        session_id,
        message: user_msg,
    };
    if let Ok(json_msg) = serde_json::to_string(&new_msg) {
        state.send_to_subscribers(&session_id, &json_msg);
    }

    tracing::info!(
        source = source,
        channel = channel_str,
        session_id = %session_id,
        "webhook received, triggering agent"
    );

    let spawn_state = Arc::clone(&state);
    tokio::spawn(async move {
        run_agent_cycle(spawn_state, session_id).await;
    });

    Json(json!({
        "status": "accepted",
        "source": source,
        "session_id": session_id
    }))
}

async fn ingest_memory(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Some(memory) = &state.memory {
        let content = body["content"].as_str().unwrap_or("");
        if content.is_empty() {
            return Json(json!({ "error": "content required" }));
        }
        let metadata = body.get("metadata").cloned().unwrap_or(json!({}));

        let mem = memory.clone();
        let content_owned = content.to_string();
        tokio::spawn(async move {
            if let Err(e) = mem.add_memory(&content_owned, metadata).await {
                tracing::error!("Ingest failed: {}", e);
            }
        });

        Json(json!({ "status": "ingesting" }))
    } else {
        Json(json!({ "error": "memory disabled" }))
    }
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
