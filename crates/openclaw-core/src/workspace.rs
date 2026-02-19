use std::path::Path;

pub fn load_prompt_files(workspace_path: &str) -> Option<String> {
    let ws = Path::new(workspace_path);
    if !ws.exists() {
        return None;
    }

    let prompt_files = ["AGENTS.md", "SOUL.md", "TOOLS.md"];
    let mut parts = Vec::new();

    for filename in &prompt_files {
        let path = ws.join(filename);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if !content.trim().is_empty() {
                    parts.push(content);
                }
            }
        }
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n---\n\n"))
    }
}
