use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use openclaw_core::config::AuthMode;
use std::sync::Arc;

use crate::state::AppState;

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match &state.config.gateway.auth.mode {
        AuthMode::None => Ok(next.run(request).await),
        AuthMode::Password => {
            let expected = state
                .config
                .gateway
                .auth
                .password
                .as_deref()
                .unwrap_or("");
            if check_credential(&request, expected, "Basic") {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        AuthMode::Token => {
            let expected = state
                .config
                .gateway
                .auth
                .token
                .as_deref()
                .unwrap_or("");
            if check_credential(&request, expected, "Bearer") {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

fn check_credential(request: &Request, expected: &str, scheme: &str) -> bool {
    if expected.is_empty() {
        return true;
    }

    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            let prefix = format!("{} ", scheme);
            if let Some(value) = auth_str.strip_prefix(&prefix) {
                return value == expected;
            }
            return auth_str == expected;
        }
    }
    if let Some(query) = request.uri().query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if (key == "token" || key == "password") && value == expected {
                    return true;
                }
            }
        }
    }

    false
}
