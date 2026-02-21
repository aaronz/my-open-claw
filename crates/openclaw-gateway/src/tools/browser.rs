use async_trait::async_trait;
use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::handler::viewport::Viewport;
use chromiumoxide::Page;
use futures::StreamExt;
use openclaw_core::provider::ToolDefinition;
use openclaw_core::{Tool, Result as CoreResult};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct BrowserTool {
    browser: Arc<Mutex<Option<Arc<Browser>>>>,
}

impl BrowserTool {
    pub fn new() -> Self {
        Self {
            browser: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_browser(&self) -> CoreResult<Arc<Browser>> {
        let mut browser_lock = self.browser.lock().await;
        if let Some(browser) = &*browser_lock {
            return Ok(Arc::clone(browser));
        }

        let config = BrowserConfig::builder()
            .viewport(Viewport::default())
            .build()
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to create browser config: {}", e)))?;

        let (browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to launch browser: {}", e)))?;

        let browser = Arc::new(browser);

        tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });

        *browser_lock = Some(Arc::clone(&browser));
        Ok(Arc::clone(&browser))
    }

    async fn get_page(&self, url: &str) -> CoreResult<Page> {
        let browser = self.get_browser().await?;
        let page = browser
            .new_page(url)
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Failed to create page: {}", e)))?;
        Ok(page)
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "browser_control".to_string(),
            description: "Control a headless browser to navigate websites, click elements, and extract data.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["navigate", "click", "type", "screenshot", "extract_text"],
                        "description": "The action to perform"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for click/type actions"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: serde_json::Value) -> CoreResult<String> {
        let action = args["action"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing action".to_string()))?;

        match action {
            "navigate" => {
                let url = args["url"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing url".to_string()))?;
                let page = self.get_page(url).await?;
                let title = page.get_title().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                Ok(format!("Navigated to {}. Title: {:?}", url, title))
            }
            "extract_text" => {
                let browser = self.get_browser().await?;
                let pages = browser.pages().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                let page = pages.last().ok_or_else(|| openclaw_core::OpenClawError::Provider("No pages open".to_string()))?;
                let content = page.content().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                Ok(content)
            }
            "screenshot" => {
                let browser = self.get_browser().await?;
                let pages = browser.pages().await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                let page = pages.last().ok_or_else(|| openclaw_core::OpenClawError::Provider("No pages open".to_string()))?;
                let screenshot = page.screenshot(chromiumoxide::page::ScreenshotParams::default()).await.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                let b64 = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, screenshot);
                Ok(format!("data:image/png;base64,{}", b64))
            }
            _ => Err(openclaw_core::OpenClawError::Provider(format!("Unsupported browser action: {}", action))),
        }
    }
}

