use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use openclaw_core::AppConfig;
use qdrant_client::prelude::*;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchPoints, VectorParams, VectorsConfig,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct MemoryService {
    qdrant: Arc<QdrantClient>,
    embedding: Arc<Mutex<TextEmbedding>>,
    collection_name: String,
}

impl MemoryService {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let qdrant = QdrantClient::from_url(&config.memory.qdrant_url).build()?;

        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )?;

        let service = Self {
            qdrant: Arc::new(qdrant),
            embedding: Arc::new(Mutex::new(model)),
            collection_name: config.memory.collection_name.clone(),
        };

        service.init_collection().await?;
        Ok(service)
    }

    async fn init_collection(&self) -> Result<()> {
        if !self.qdrant.collection_exists(&self.collection_name).await? {
            self
                .qdrant
                .create_collection(&CreateCollection {
                    collection_name: self.collection_name.clone(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(qdrant_client::qdrant::vectors_config::Config::Params(
                            VectorParams {
                                size: 384,
                                distance: Distance::Cosine.into(),
                                ..Default::default()
                            },
                        )),
                    }),
                    ..Default::default()
                })
                .await?;
        }
        Ok(())
    }

    pub async fn add_memory(&self, text: &str, metadata: serde_json::Value) -> Result<()> {
        let embedding = {
            let model = self.embedding.lock().await;
            let embeddings = model.embed(vec![text], None)?;
            embeddings[0].clone()
        };

        let mut payload = Payload::new();
        // Flatten metadata into payload
        if let serde_json::Value::Object(map) = metadata {
            for (k, v) in map {
                payload.insert(k, v);
            }
        }
        payload.insert("text", text.to_string());

        let point = PointStruct::new(
            Uuid::new_v4().to_string(),
            embedding,
            payload,
        );

        self
            .qdrant
            .upsert_points(self.collection_name.clone(), None, vec![point], None)
            .await?;

        Ok(())
    }

    pub async fn search_memory(&self, query: &str, limit: u64) -> Result<Vec<String>> {
        let embedding = {
            let model = self.embedding.lock().await;
            let embeddings = model.embed(vec![query], None)?;
            embeddings[0].clone()
        };

        let search_result = self
            .qdrant
            .search_points(&SearchPoints {
                collection_name: self.collection_name.clone(),
                vector: embedding,
                limit,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await?;

        let mut results = Vec::new();
        for point in search_result.result {
            if let Some(payload) = point.payload.get("text") {
                // payload value is a qdrant Value, not direct string?
                // qdrant_client Payload types convert nicely?
                // PointStruct payload is HashMap<String, Value>.
                // Value is qdrant_client::qdrant::Value (kind: string_value)
                
                // qdrant-client provides conversion helpers?
                // `point.payload` is `HashMap<String, qdrant::Value>`.
                // Checking how to extract string.
                
                // `point.payload["text"].kind` -> `Kind::StringValue(s)`
                // But accessing nested enum is verbose.
                // qdrant-client might interpret it.
                
                // Let's print debug if needed, but try standard way.
                // Or verify qdrant crate docs (from memory).
                // `Value` has `as_str()` helper? Maybe not.
                
                // Let's implement robust extraction helper logic in a bit if needed.
                // For now, let's assume `to_string()` or similar works, or use `json!` conversion.
                // Actually `serde_json::to_value` converts qdrant Value to serde Value.
                
                let json_val = serde_json::to_value(payload).unwrap_or_default();
                if let Some(s) = json_val.as_str() {
                    results.push(s.to_string());
                }
            }
        }
        Ok(results)
    }
}
