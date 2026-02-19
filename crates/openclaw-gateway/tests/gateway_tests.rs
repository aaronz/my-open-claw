#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use openclaw_core::{AppConfig, WsMessage};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::Message;

    async fn spawn_test_gateway() -> u16 {
        let config = AppConfig::default();
        let state = openclaw_gateway::state::AppState::new_ephemeral(config);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let app = axum::Router::new()
            .route("/ws", axum::routing::get(openclaw_gateway::ws::ws_handler))
            .merge(openclaw_gateway::routes::api_router())
            .with_state(state);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        port
    }

    #[tokio::test]
    async fn health_endpoint() {
        let port = spawn_test_gateway().await;
        let resp = reqwest::get(format!("http://127.0.0.1:{}/health", port))
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["status"], "ok");
    }

    #[tokio::test]
    async fn ws_ping_pong() {
        let port = spawn_test_gateway().await;
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws.split();

        let ping = WsMessage::Ping { timestamp: 42 };
        let json = serde_json::to_string(&ping).unwrap();
        write.send(Message::Text(json.into())).await.unwrap();

        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            read.next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();

        let text = response.into_text().unwrap();
        let msg: WsMessage = serde_json::from_str(&text).unwrap();
        match msg {
            WsMessage::Pong { timestamp } => assert_eq!(timestamp, 42),
            _ => panic!("expected Pong, got {:?}", msg),
        }
    }

    #[tokio::test]
    async fn ws_send_message_gets_agent_response() {
        let port = spawn_test_gateway().await;
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws.split();

        let send = WsMessage::SendMessage {
            session_id: None,
            content: "hello test".to_string(),
            channel: None,
            peer_id: Some("test-user".to_string()),
        };
        let json = serde_json::to_string(&send).unwrap();
        write.send(Message::Text(json.into())).await.unwrap();

        let mut got_thinking = false;
        let mut got_response = false;

        let timeout = tokio::time::Duration::from_secs(5);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout && !got_response {
            if let Ok(Some(Ok(msg))) = tokio::time::timeout(
                tokio::time::Duration::from_secs(2),
                read.next(),
            )
            .await
            {
                if let Ok(text) = msg.into_text() {
                    if let Ok(parsed) = serde_json::from_str::<WsMessage>(&text) {
                        match parsed {
                            WsMessage::AgentThinking { .. } => got_thinking = true,
                            WsMessage::AgentResponse { done: true, .. } => got_response = true,
                            _ => {}
                        }
                    }
                }
            }
        }

        assert!(got_thinking, "should have received AgentThinking");
        assert!(got_response, "should have received AgentResponse with done=true");
    }

    #[tokio::test]
    async fn ws_get_sessions_empty() {
        let port = spawn_test_gateway().await;
        let url = format!("ws://127.0.0.1:{}/ws", port);
        let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let (mut write, mut read) = ws.split();

        let msg = WsMessage::GetSessions;
        let json = serde_json::to_string(&msg).unwrap();
        write.send(Message::Text(json.into())).await.unwrap();

        let response = tokio::time::timeout(
            tokio::time::Duration::from_secs(2),
            read.next(),
        )
        .await
        .unwrap()
        .unwrap()
        .unwrap();

        let text = response.into_text().unwrap();
        let parsed: WsMessage = serde_json::from_str(&text).unwrap();
        match parsed {
            WsMessage::SessionList { sessions } => assert!(sessions.is_empty()),
            _ => panic!("expected SessionList"),
        }
    }

    #[tokio::test]
    async fn api_status_endpoint() {
        let port = spawn_test_gateway().await;
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/status", port))
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["session_count"], 0);
    }

    #[tokio::test]
    async fn api_sessions_endpoint() {
        let port = spawn_test_gateway().await;
        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/sessions", port))
            .await
            .unwrap();
        assert!(resp.status().is_success());
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["sessions"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn api_config_redacts_secrets() {
        let mut config = AppConfig::default();
        config.gateway.auth.password = Some("secret123".to_string());
        config.models.providers.push(openclaw_core::config::ProviderConfig {
            name: "test".to_string(),
            model: "test-model".to_string(),
            api_key: Some("sk-secret".to_string()),
            base_url: None,
        });

        let state = openclaw_gateway::state::AppState::new(config);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        let app = axum::Router::new()
            .merge(openclaw_gateway::routes::api_router())
            .with_state(state);

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let resp = reqwest::get(format!("http://127.0.0.1:{}/api/config", port))
            .await
            .unwrap();
        let body: serde_json::Value = resp.json().await.unwrap();

        assert_eq!(body["gateway"]["auth"]["password"], "***");
        assert_eq!(body["models"]["providers"][0]["api_key"], "***");
    }
}
