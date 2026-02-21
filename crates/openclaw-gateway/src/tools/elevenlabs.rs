use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use reqwest::Client;

pub struct ElevenLabsTool {
    client: Client,
    api_key: String,
}

impl ElevenLabsTool {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Tool for ElevenLabsTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "tts_elevenlabs".to_string(),
            description: "Convert text to high-quality audio speech using ElevenLabs.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "The text to convert to speech"
                    },
                    "voice_id": {
                        "type": "string",
                        "description": "Optional ElevenLabs voice ID",
                        "default": "pNInz6obpg8ndclKuztW"
                    }
                },
                "required": ["text"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let text = args["text"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing text".to_string()))?;
        let voice_id = args["voice_id"].as_str().unwrap_or("pNInz6obpg8ndclKuztW");

        let url = format!("https://api.elevenlabs.io/v1/text-to-speech/{}", voice_id);
        
        let body = json!({
            "text": text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.5,
                "similarity_boost": 0.5
            }
        });

        let res = self.client.post(&url)
            .header("xi-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("ElevenLabs error: {}", err)));
        }

        let bytes = res.bytes().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        let b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, bytes);
        
        Ok(format!("data:audio/mpeg;base64,{}", b64))
    }
}
