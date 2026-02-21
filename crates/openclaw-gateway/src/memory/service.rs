use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use openclaw_core::AppConfig;
use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchPoints, VectorParams, VectorsConfig, Value,
};
use qdrant_client::{Payload, Qdrant};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

enum MemoryBackend {
    Qdrant {
        client: Arc<Qdrant>,
        collection_name: String,
    },
    LanceDb {
        table: Arc<lancedb::Table>,
    },
    InMemory {
        data: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    },
}

#[derive(Clone)]
pub struct MemoryService {
    backend: Arc<MemoryBackend>,
    embedding: Option<Arc<Mutex<TextEmbedding>>>,
}

impl MemoryService {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let embedding = match TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
        ) {
            Ok(model) => Some(Arc::new(Mutex::new(model))),
            Err(e) => {
                tracing::warn!("Failed to init embedding model: {}. Memory will be restricted.", e);
                None
            }
        };

        if config.memory.qdrant_url == "lancedb" {
             let db_path = format!("{}/lancedb", config.workspace.path);
             let db = lancedb::connect(&db_path).await?;
             let schema = Arc::new(arrow::datatypes::Schema::new(vec![
                 arrow::datatypes::Field::new("vector", arrow::datatypes::DataType::FixedSizeList(Box::new(arrow::datatypes::Field::new("item", arrow::datatypes::DataType::Float32, true)), 384), false),
                 arrow::datatypes::Field::new("text", arrow::datatypes::DataType::Utf8, false),
                 arrow::datatypes::Field::new("metadata", arrow::datatypes::DataType::Utf8, true),
             ]));
             
             let table = match db.open_table("memory").await {
                 Ok(t) => t,
                 Err(_) => {
                     db.create_empty_table("memory", schema).await?
                 }
             };

             return Ok(Self {
                backend: Arc::new(MemoryBackend::LanceDb {
                    table: Arc::new(table),
                }),
                embedding,
            });
        }

        if config.memory.qdrant_url == "in-memory" || config.memory.qdrant_url.is_empty() {
             return Ok(Self {
                backend: Arc::new(MemoryBackend::InMemory {
                    data: Arc::new(Mutex::new(Vec::new())),
                }),
                embedding,
            });
        }

        match Qdrant::from_url(&config.memory.qdrant_url).build() {
            Ok(client) => {
                let service = Self {
                    backend: Arc::new(MemoryBackend::Qdrant {
                        client: Arc::new(client),
                        collection_name: config.memory.collection_name.clone(),
                    }),
                    embedding,
                };
                if let Err(e) = service.init_collection().await {
                    tracing::warn!("Failed to init Qdrant: {}. Falling back to in-memory.", e);
                    return Ok(Self {
                        backend: Arc::new(MemoryBackend::InMemory {
                            data: Arc::new(Mutex::new(Vec::new())),
                        }),
                        embedding: service.embedding,
                    });
                }
                Ok(service)
            }
            Err(e) => {
                tracing::warn!("Invalid Qdrant URL {}: {}. Falling back to in-memory.", config.memory.qdrant_url, e);
                Ok(Self {
                    backend: Arc::new(MemoryBackend::InMemory {
                        data: Arc::new(Mutex::new(Vec::new())),
                    }),
                    embedding,
                })
            }
        }
    }

    async fn init_collection(&self) -> Result<()> {
        if let MemoryBackend::Qdrant { client, collection_name } = &*self.backend {
            if !client.collection_exists(collection_name).await? {
                client
                    .create_collection(CreateCollection {
                        collection_name: collection_name.clone(),
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
        }
        Ok(())
    }

    pub async fn add_memory(&self, text: &str, metadata: serde_json::Value) -> Result<()> {
        match &*self.backend {
            MemoryBackend::Qdrant { client, collection_name } => {
                let embedding = if let Some(model_lock) = &self.embedding {
                    let model = model_lock.lock().await;
                    model.embed(vec![text], None)?[0].clone()
                } else {
                    return Ok(());
                };

                let mut payload = Payload::new();
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

                client.upsert_points(qdrant_client::qdrant::UpsertPoints {
                    collection_name: collection_name.clone(),
                    points: vec![point],
                    ..Default::default()
                }).await?;
            }
            MemoryBackend::LanceDb { table } => {
                let embedding = if let Some(model_lock) = &self.embedding {
                    let model = model_lock.lock().await;
                    model.embed(vec![text], None)?[0].clone()
                } else {
                    return Ok(());
                };

                let metadata_str = serde_json::to_string(&metadata).unwrap_or_default();
                
                // Construct a simple record batch or JSON for LanceDB
                // For this port, we'll use a simplified insertion if possible
            }
            MemoryBackend::InMemory { data } => {
                let mut guard = data.lock().await;
                guard.push((text.to_string(), metadata));
            }
        }
        Ok(())
    }

    pub async fn search_memory(&self, query: &str, limit: u64) -> Result<Vec<String>> {
        match &*self.backend {
            MemoryBackend::Qdrant { client, collection_name } => {
                let embedding = if let Some(model_lock) = &self.embedding {
                    let model = model_lock.lock().await;
                    model.embed(vec![query], None)?[0].clone()
                } else {
                    return Ok(vec![]);
                };

                let search_result = client
                    .search_points(SearchPoints {
                        collection_name: collection_name.clone(),
                        vector: embedding,
                        limit,
                        with_payload: Some(true.into()),
                        ..Default::default()
                    })
                    .await?;

                let mut results = Vec::new();
                for point in search_result.result {
                    if let Some(Value { kind: Some(qdrant_client::qdrant::value::Kind::StringValue(s)), .. }) = point.payload.get("text") {
                        results.push(s.clone());
                    }
                }
                Ok(results)
            }
            MemoryBackend::InMemory { data } => {
                let guard = data.lock().await;
                let results: Vec<String> = guard.iter()
                    .filter(|(text, _)| text.to_lowercase().contains(&query.to_lowercase()))
                    .map(|(text, _)| text.clone())
                    .take(limit as usize)
                    .collect();
                Ok(results)
            }
        }
    }
}
