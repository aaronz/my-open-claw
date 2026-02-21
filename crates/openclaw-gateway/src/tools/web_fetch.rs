use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};

pub struct WebFetchTool {
    client: reqwest::Client,
}

impl WebFetchTool {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Fetch and extract readable content from a URL. Returns cleaned text suitable for analysis.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to fetch content from"
                    },
                    "extract_mode": {
                        "type": "string",
                        "enum": ["text", "markdown", "html"],
                        "description": "Extraction mode (default: text)"
                    },
                    "max_chars": {
                        "type": "integer",
                        "description": "Maximum characters to return (default: 10000)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let url = args["url"].as_str().unwrap_or("");
        let max_chars = args["max_chars"].as_u64().unwrap_or(10000) as usize;

        if url.is_empty() {
            return Ok("Error: URL is required".to_string());
        }

        let response = self.client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; OpenClaw/1.0)")
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !response.status().is_success() {
            return Ok(format!("HTTP error: {}", response.status()));
        }

        let html = response.text().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let text = extract_text(&html);
        
        if text.len() > max_chars {
            Ok(format!("{}...\n[Truncated at {} characters]", &text[..max_chars], max_chars))
        } else {
            Ok(text)
        }
    }
}

fn extract_text(html: &str) -> String {
    let mut text = String::new();
    let mut in_script = false;
    let mut in_style = false;
    let mut last_was_space = false;

    for line in html.lines() {
        let line = line.trim();
        
        if line.contains("<script") { in_script = true; }
        if line.contains("</script>") { in_script = false; continue; }
        if line.contains("<style") { in_style = true; }
        if line.contains("</style>") { in_style = false; continue; }
        
        if in_script || in_style { continue; }

        let stripped = strip_tags(line);
        let stripped = html_escape::decode_html_entities(&stripped);
        let stripped = stripped.trim();
        
        if !stripped.is_empty() {
            if !last_was_space {
                text.push(' ');
            }
            text.push_str(stripped);
            last_was_space = false;
        } else if !text.is_empty() {
            last_was_space = true;
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_tags(s: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    
    result
}

mod html_escape {
    pub fn decode_html_entities(s: &str) -> String {
        s.replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
    }
}
