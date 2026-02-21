use async_trait::async_trait;
use openclaw_core::{Result, Tool, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilePatch {
    path: String,
    hunks: Vec<Hunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Hunk {
    start_line: usize,
    end_line: usize,
    content: String,
}

pub struct ApplyPatchTool;

#[async_trait]
impl Tool for ApplyPatchTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "apply_patch".to_string(),
            description: "Apply structured patches across multiple files with multi-hunk support. Safer than direct file writes as it validates context.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "patches": {
                        "type": "array",
                        "description": "List of file patches to apply",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": {
                                    "type": "string",
                                    "description": "File path to patch"
                                },
                                "hunks": {
                                    "type": "array",
                                    "description": "Hunks to apply",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "start_line": {
                                                "type": "integer",
                                                "description": "Start line (1-indexed)"
                                            },
                                            "end_line": {
                                                "type": "integer",
                                                "description": "End line (1-indexed, inclusive)"
                                            },
                                            "content": {
                                                "type": "string",
                                                "description": "New content to replace the lines"
                                            }
                                        },
                                        "required": ["start_line", "end_line", "content"]
                                    }
                                }
                            },
                            "required": ["path", "hunks"]
                        }
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "If true, only validate without applying (default: false)"
                    }
                },
                "required": ["patches"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let dry_run = args["dry_run"].as_bool().unwrap_or(false);
        let patches: Vec<FilePatch> = serde_json::from_value(args["patches"].clone())
            .map_err(|e| openclaw_core::OpenClawError::Provider(format!("Invalid patch format: {}", e)))?;

        let mut results = Vec::new();

        for patch in patches {
            let path = Path::new(&patch.path);
            
            if !path.exists() {
                results.push(format!("❌ {} - File not found", patch.path));
                continue;
            }

            let content = tokio::fs::read_to_string(path).await
                .map_err(|e| openclaw_core::OpenClawError::Io(e))?;

            let mut lines: Vec<&str> = content.lines().collect();
            let mut applied_hunks = 0;

            for hunk in &patch.hunks {
                if hunk.start_line == 0 || hunk.end_line < hunk.start_line {
                    results.push(format!("❌ {} - Invalid line range: {}-{}", 
                        patch.path, hunk.start_line, hunk.end_line));
                    continue;
                }

                let start_idx = hunk.start_line.saturating_sub(1);
                let end_idx = hunk.end_line.saturating_sub(1);

                if end_idx >= lines.len() {
                    results.push(format!("❌ {} - Line {} beyond file length {}", 
                        patch.path, hunk.end_line, lines.len()));
                    continue;
                }

                let new_lines: Vec<&str> = hunk.content.lines().collect();
                lines.splice(start_idx..=end_idx, new_lines);
                applied_hunks += 1;
            }

            if !dry_run && applied_hunks > 0 {
                let new_content = lines.join("\n");
                tokio::fs::write(path, new_content).await
                    .map_err(|e| openclaw_core::OpenClawError::Io(e))?;
                results.push(format!("✅ {} - Applied {} hunks", patch.path, applied_hunks));
            } else if dry_run {
                results.push(format!("🔍 {} - Would apply {} hunks (dry run)", patch.path, applied_hunks));
            }
        }

        Ok(results.join("\n"))
    }
}
