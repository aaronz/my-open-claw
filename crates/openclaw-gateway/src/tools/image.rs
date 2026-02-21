use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct ImageTool {
    api_key: Option<String>,
    base_url: String,
}

impl ImageTool {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            base_url: "https://api.openai.com/v1/chat/completions".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }
}

#[async_trait]
impl Tool for ImageTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "image".to_string(),
            description: "Analyze images with vision-capable AI models. Supports base64 encoded images or URLs.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "image": {
                        "type": "string",
                        "description": "Image data - either a URL, base64 data URI, or file path"
                    },
                    "prompt": {
                        "type": "string",
                        "description": "What to analyze or describe about the image"
                    },
                    "model": {
                        "type": "string",
                        "description": "Vision model to use (default: gpt-4o)"
                    }
                },
                "required": ["image", "prompt"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let image = args["image"].as_str().unwrap_or("");
        let prompt = args["prompt"].as_str().unwrap_or("Describe this image");
        let model = args["model"].as_str().unwrap_or("gpt-4o");

        let api_key = match &self.api_key {
            Some(k) => k,
            None => return Ok("Error: No API key configured for image analysis".to_string()),
        };

        let image_url = if image.starts_with("data:") {
            image.to_string()
        } else if image.starts_with("http://") || image.starts_with("https://") {
            image.to_string()
        } else {
            let path = std::path::Path::new(image);
            if path.exists() {
                let bytes = tokio::fs::read(path).await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?;
                let base64 = BASE64.encode(&bytes);
                format!("data:image/png;base64,{}", base64)
            } else {
                return Ok(format!("Error: Image file not found: {}", image));
            }
        };

        let client = reqwest::Client::new();
        let body = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": prompt
                    },
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": image_url
                        }
                    }
                ]
            }],
            "max_tokens": 1000
        });

        let response = client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Ok(format!("API error {}: {}", status, text));
        }

        let json: Value = response.json().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("No response")
            .to_string();

        Ok(content)
    }
}
