use async_trait::async_trait;
use openclaw_core::provider::ToolDefinition;
use serde_json::json;

use super::Skill;

pub struct SpotifySkill;

#[async_trait]
impl Skill for SpotifySkill {
    fn name(&self) -> &str {
        "spotify"
    }
    
    fn description(&self) -> &str {
        "Control Spotify playback and search for music"
    }
    
    fn version(&self) -> &str {
        "1.0.0"
    }
    
    fn tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "spotify_play".to_string(),
                description: "Play a song or playlist".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Song name or artist" }
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "spotify_pause".to_string(),
                description: "Pause current playback".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            },
            ToolDefinition {
                name: "spotify_now_playing".to_string(),
                description: "Get currently playing track".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {},
                }),
            },
            ToolDefinition {
                name: "spotify_search".to_string(),
                description: "Search for songs, albums, or artists".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string", "description": "Search query" },
                        "type": { "type": "string", "description": "track, album, or artist" }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }
    
    async fn execute_tool(&self, name: &str, args: serde_json::Value) -> Result<String, String> {
        match name {
            "spotify_play" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                Ok(format!("Playing: {}", query))
            }
            "spotify_pause" => {
                Ok("Paused playback".to_string())
            }
            "spotify_now_playing" => {
                Ok("Now playing: Bohemian Rhapsody - Queen".to_string())
            }
            "spotify_search" => {
                let query = args["query"].as_str().ok_or("Missing query")?;
                let search_type = args.get("type").and_then(|v| v.as_str()).unwrap_or("track");
                Ok(format!("Search results for '{}' ({})[0]:\n- Song Title by Artist", query, search_type))
            }
            _ => Err("Unknown tool".to_string())
        }
    }
    
    fn system_prompt(&self) -> Option<&str> {
        Some("You can control Spotify playback. Use this to play music, pause, skip, or search for songs.")
    }
}
