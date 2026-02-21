use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use openclaw_core::AppConfig;
use qdrant_client::{
    qdrant::{
        CreateCollection, Distance, PointStruct, SearchPoints, UpsertPoints, Value, VectorParams,
        VectorsConfig,
    },
    Payload, Qdrant,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

enum MemoryBackend {
    InMemory {
        data: Arc<Mutex<Vec<(String, serde_json::Value)>>>,
    },
    Qdrant {
        client: Arc<Qdrant>,
        collection_name: String,
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

        if config.memory.qdrant_url == "in-memory"
            || config.memory.qdrant_url.is_empty()
            || !config.memory.enabled
        {
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
                tracing::warn!(
                    "Invalid Qdrant URL {}: {}. Falling back to in-memory.",
                    config.memory.qdrant_url,
                    e
                );
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
        if let MemoryBackend::Qdrant {
            client,
            collection_name,
        } = &*self.backend
        {
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
            MemoryBackend::Qdrant {
                client,
                collection_name,
            } => {
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

                let point = PointStruct::new(Uuid::new_v4().to_string(), embedding, payload);

                client
                    .upsert_points(UpsertPoints {
                        collection_name: collection_name.clone(),
                        points: vec![point],
                        ..Default::default()
                    })
                    .await?;
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
            MemoryBackend::Qdrant {
                client,
                collection_name,
            } => {
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
                    if let Some(Value {
                        kind: Some(qdrant_client::qdrant::value::Kind::StringValue(s)),
                        ..
                    }) = point.payload.get("text")
                    {
                        results.push(s.clone());
                    }
                }
                Ok(results)
            }
            MemoryBackend::InMemory { data } => {
                let guard = data.lock().await;
                let results: Vec<String> = guard
                    .iter()
                    .filter(|(text, _)| {
                        text.to_lowercase()
                            .contains(&query.to_lowercase())
                    })
                    .map(|(text, _)| text.clone())
                    .take(limit as usize)
                    .collect();
                Ok(results)
            }
        }
    }
}
