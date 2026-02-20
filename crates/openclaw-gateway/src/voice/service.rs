use anyhow::Result;
use openclaw_core::AppConfig;
use reqwest::{Client, multipart};

#[derive(Clone)]
pub struct VoiceService {
    client: Client,
    api_key: String,
    stt_model: String,
    tts_model: String,
    tts_voice: String,
}

impl VoiceService {
    pub fn new(config: &AppConfig) -> Option<Self> {
        if !config.audio.enabled {
            return None;
        }
        let api_key = config.audio.openai_api_key.clone().or_else(|| {
            config.models.providers.iter()
                .find(|p| p.name == "openai" || p.name == "gpt")
                .and_then(|p| p.api_key.clone())
        })?;

        Some(Self {
            client: Client::new(),
            api_key,
            stt_model: config.audio.stt_model.clone(),
            tts_model: config.audio.tts_model.clone(),
            tts_voice: config.audio.tts_voice.clone(),
        })
    }

    pub async fn transcribe(&self, audio_bytes: Vec<u8>, filename: &str) -> Result<String> {
        let part = multipart::Part::bytes(audio_bytes).file_name(filename.to_string());
        let form = multipart::Form::new()
            .part("file", part)
            .text("model", self.stt_model.clone());

        let res = self.client.post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;

        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow::anyhow!("STT error: {}", err));
        }

        let json: serde_json::Value = res.json().await?;
        Ok(json["text"].as_str().unwrap_or("").to_string())
    }

    pub async fn speak(&self, text: &str) -> Result<Vec<u8>> {
        let body = serde_json::json!({
            "model": self.tts_model,
            "input": text,
            "voice": self.tts_voice
        });

        let res = self.client.post("https://api.openai.com/v1/audio/speech")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let err = res.text().await?;
            return Err(anyhow::anyhow!("TTS error: {}", err));
        }

        let bytes = res.bytes().await?;
        Ok(bytes.to_vec())
    }
}
