#[cfg(test)]
mod tests {
    use openclaw_core::*;
    use openclaw_core::config::*;
    use openclaw_core::session::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn ws_message_serialize_ping() {
        let msg = WsMessage::Ping { timestamp: 123456 };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"ping""#));
        assert!(json.contains(r#""timestamp":123456"#));
    }

    #[test]
    fn ws_message_deserialize_send_message() {
        let json = r#"{"type":"send_message","content":"hello","session_id":null,"channel":null,"peer_id":null}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        match msg {
            WsMessage::SendMessage { content, .. } => assert_eq!(content, "hello"),
            _ => panic!("expected SendMessage"),
        }
    }

    #[test]
    fn ws_message_roundtrip_agent_response() {
        let msg = WsMessage::AgentResponse {
            session_id: uuid::Uuid::new_v4(),
            content: "test response".to_string(),
            done: true,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: WsMessage = serde_json::from_str(&json).unwrap();
        match parsed {
            WsMessage::AgentResponse { content, done, .. } => {
                assert_eq!(content, "test response");
                assert!(done);
            }
            _ => panic!("expected AgentResponse"),
        }
    }

    #[test]
    fn session_store_compact() {
        let store = SessionStore::new();
        let session = store.create(ChannelKind::Cli, "u1".to_string());
        for i in 0..10 {
            let msg = ChatMessage {
                id: uuid::Uuid::new_v4(),
                role: Role::User,
                content: format!("msg {}", i),
                timestamp: chrono::Utc::now(),
                channel: ChannelKind::Cli,
                tool_calls: vec![],
                tool_result: None,
            };
            store.add_message(&session.id, msg).unwrap();
        }

        let summary = ChatMessage {
            id: uuid::Uuid::new_v4(),
            role: Role::System,
            content: "summary".to_string(),
            timestamp: chrono::Utc::now(),
            channel: ChannelKind::Cli,
            tool_calls: vec![],
            tool_result: None,
        };

        // Compact: remove first 5 messages, replace with summary
        store.compact(&session.id, 5, summary).unwrap();

        let updated = store.get(&session.id).unwrap();
        // Expected: summary + 5 messages (msg 5..9)
        assert_eq!(updated.messages.len(), 6);
        assert_eq!(updated.messages[0].content, "summary");
        assert_eq!(updated.messages[1].content, "msg 5");
        assert_eq!(updated.messages[5].content, "msg 9");
    }

    #[test]
    fn ws_message_deserialize_get_sessions() {
        let json = r#"{"type":"get_sessions"}"#;
        let msg: WsMessage = serde_json::from_str(json).unwrap();
        assert!(matches!(msg, WsMessage::GetSessions));
    }

    #[test]
    fn config_default_port() {
        let config = AppConfig::default();
        assert_eq!(config.gateway.port, 18789);
    }

    #[test]
    fn config_default_model() {
        let config = AppConfig::default();
        assert_eq!(config.models.default_model, "claude-sonnet-4-20250514");
    }

    #[test]
    fn config_load_missing_file_returns_default() {
        let path = std::path::PathBuf::from("/tmp/nonexistent_openclaw_test_config.yaml");
        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.gateway.port, 18789);
    }

    #[test]
    fn config_save_and_load_roundtrip() {
        let mut config = AppConfig::default();
        config.gateway.port = 9999;
        config.models.default_model = "test-model".to_string();
        config.agent.system_prompt = Some("you are helpful".to_string());

        let tmp = NamedTempFile::new().unwrap();
        config.save(tmp.path()).unwrap();

        let loaded = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(loaded.gateway.port, 9999);
        assert_eq!(loaded.models.default_model, "test-model");
        assert_eq!(loaded.agent.system_prompt.as_deref(), Some("you are helpful"));
    }

    #[test]
    fn config_yaml_partial_fields_use_defaults() {
        let yaml = "gateway:\n  port: 12345\n";
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(yaml.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let config = AppConfig::load(tmp.path()).unwrap();
        assert_eq!(config.gateway.port, 12345);
        assert_eq!(config.models.default_model, "claude-sonnet-4-20250514");
        assert!(matches!(config.gateway.auth.mode, AuthMode::None));
    }

    #[test]
    fn session_store_create_and_get() {
        let store = SessionStore::new();
        let session = store.create(ChannelKind::Cli, "user1".to_string());
        let got = store.get(&session.id).unwrap();
        assert_eq!(got.id, session.id);
        assert_eq!(got.peer_id, "user1");
    }

    #[test]
    fn session_store_get_or_create_reuses_session() {
        let store = SessionStore::new();
        let s1 = store.get_or_create(ChannelKind::Cli, "user1");
        let s2 = store.get_or_create(ChannelKind::Cli, "user1");
        assert_eq!(s1.id, s2.id);
    }

    #[test]
    fn session_store_different_channels_different_sessions() {
        let store = SessionStore::new();
        let s1 = store.get_or_create(ChannelKind::Cli, "user1");
        let s2 = store.get_or_create(ChannelKind::Api, "user1");
        assert_ne!(s1.id, s2.id);
    }

    #[test]
    fn session_store_add_message() {
        let store = SessionStore::new();
        let session = store.create(ChannelKind::Cli, "user1".to_string());
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4(),
            role: Role::User,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            channel: ChannelKind::Cli,
            tool_calls: vec![],
            tool_result: None,
        };
        store.add_message(&session.id, msg).unwrap();
        let updated = store.get(&session.id).unwrap();
        assert_eq!(updated.messages.len(), 1);
        assert_eq!(updated.messages[0].content, "hello");
    }

    #[test]
    fn session_store_list() {
        let store = SessionStore::new();
        store.create(ChannelKind::Cli, "a".to_string());
        store.create(ChannelKind::Api, "b".to_string());
        assert_eq!(store.list().len(), 2);
    }

    #[test]
    fn channel_kind_display() {
        assert_eq!(ChannelKind::Telegram.to_string(), "telegram");
        assert_eq!(ChannelKind::WhatsApp.to_string(), "whatsapp");
        assert_eq!(ChannelKind::Cli.to_string(), "cli");
    }

    #[test]
    fn channel_kind_serde_roundtrip() {
        let kind = ChannelKind::Discord;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, r#""discord""#);
        let parsed: ChannelKind = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, kind);
    }

    #[test]
    fn error_display() {
        let err = OpenClawError::Config("bad config".to_string());
        assert_eq!(err.to_string(), "config error: bad config");
        let err = OpenClawError::Provider("timeout".to_string());
        assert_eq!(err.to_string(), "provider error: timeout");
    }

    #[test]
    fn auth_mode_serde() {
        let json = r#""password""#;
        let mode: AuthMode = serde_json::from_str(json).unwrap();
        assert!(matches!(mode, AuthMode::Password));
    }

    #[test]
    fn presence_status_serde() {
        let status = openclaw_core::message::PresenceStatus::Typing;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""typing""#);
    }
}
