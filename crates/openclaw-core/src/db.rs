use crate::channel::ChannelKind;
use crate::error::Result;
use crate::session::{ChatMessage, Session};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

#[derive(Clone)]
pub struct DbStore {
    pool: SqlitePool,
}

impl DbStore {
    pub async fn new(db_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(db_url).await.map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(Self { pool })
    }

    pub async fn create_session(&self, channel: ChannelKind, peer_id: String) -> Result<Session> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let metadata = serde_json::json!({});
        
        sqlx::query(
            "INSERT INTO sessions (id, channel, peer_id, metadata, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id.to_string())
        .bind(channel.to_string())
        .bind(peer_id.clone())
        .bind(serde_json::to_string(&metadata).unwrap())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(Session {
            id,
            channel,
            peer_id,
            messages: vec![],
            created_at: now,
            updated_at: now,
            metadata: std::collections::HashMap::new(),
        })
    }

    pub async fn get_session_by_peer(&self, channel: ChannelKind, peer_id: &str) -> Result<Option<Session>> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT id, channel, peer_id, metadata, created_at, updated_at FROM sessions WHERE channel = ? AND peer_id = ?"
        )
        .bind(channel.to_string())
        .bind(peer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        if let Some(r) = row {
            let id_str: String = r.get("id");
            let id = Uuid::parse_str(&id_str).unwrap();
            let channel_str: String = r.get("channel");
            let channel = serde_json::from_value(serde_json::json!(channel_str)).unwrap_or(ChannelKind::Api);
            let metadata_str: String = r.get("metadata");
            let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            
            let messages = self.get_messages(id).await?;

            Ok(Some(Session {
                id,
                channel,
                peer_id: r.get("peer_id"),
                messages,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                metadata,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn add_message(&self, session_id: Uuid, msg: ChatMessage) -> Result<()> {
        let now = chrono::Utc::now();
        
        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, images, tool_calls, tool_result, timestamp) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(msg.id.to_string())
        .bind(session_id.to_string())
        .bind(serde_json::to_string(&msg.role).unwrap().replace("\"", ""))
        .bind(msg.content)
        .bind(serde_json::to_string(&msg.images).unwrap())
        .bind(serde_json::to_string(&msg.tool_calls).unwrap())
        .bind(msg.tool_result.as_ref().map(|tr| serde_json::to_string(tr).unwrap()))
        .bind(msg.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        sqlx::query(
            "UPDATE sessions SET updated_at = ? WHERE id = ?"
        )
        .bind(now)
        .bind(session_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(())
    }

    pub async fn get_messages(&self, session_id: Uuid) -> Result<Vec<ChatMessage>> {
        use sqlx::Row;
        let rows = sqlx::query(
            "SELECT id, role, content, images, tool_calls, tool_result, timestamp FROM messages WHERE session_id = ? ORDER BY timestamp ASC"
        )
        .bind(session_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| crate::OpenClawError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        let mut msgs = Vec::new();
        for r in rows {
            let id_str: String = r.get("id");
            let role_str: String = r.get("role");
            let images_str: String = r.get("images");
            let tool_calls_str: String = r.get("tool_calls");
            let tool_result_str: Option<String> = r.get("tool_result");

            msgs.push(ChatMessage {
                id: Uuid::parse_str(&id_str).unwrap(),
                role: serde_json::from_str(&format!("\"{}\"", role_str)).unwrap_or(crate::session::Role::User),
                content: r.get("content"),
                images: serde_json::from_str(&images_str).unwrap_or_default(),
                tool_calls: serde_json::from_str(&tool_calls_str).unwrap_or_default(),
                tool_result: tool_result_str.and_then(|tr| serde_json::from_str(&tr).ok()),
                timestamp: r.get("timestamp"),
                channel: ChannelKind::Api,
            });
        }
        Ok(msgs)
    }
}
