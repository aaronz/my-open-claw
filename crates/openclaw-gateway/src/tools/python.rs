use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde_json::{json, Value};
use tokio::process::Command;

pub struct PythonTool;

#[async_trait]
impl Tool for PythonTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "python_interpreter".to_string(),
            description: "Execute Python code. Use print() to output results. Note: Running locally.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Python code to execute"
                    }
                },
                "required": ["code"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let code = args["code"].as_str().unwrap_or("");
        if code.is_empty() {
            return Ok("Empty code".to_string());
        }

        // Security Warning: This runs unboxed.
        let output = Command::new("python3")
            .arg("-c")
            .arg(code)
            .output()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Execution failed: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
             Ok(format!("Error: {}\nOutput: {}", stderr, stdout))
        } else if !stderr.is_empty() {
            Ok(format!("Output: {}\nStderr: {}", stdout, stderr))
        } else {
            Ok(format!("Output: {}", stdout))
        }
    }
}
