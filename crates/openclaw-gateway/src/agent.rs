use crate::state::AppState;
use chrono::Utc;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{ChannelKind, WsMessage, Tool};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::error;
use uuid::Uuid;

pub async fn run_agent_cycle(state: Arc<AppState>, session_id: Uuid) {
    if let Ok(json) = serde_json::to_string(&WsMessage::AgentThinking {
        session_id,
    }) {
        state.send_to_subscribers(&session_id, &json);
    }

    if state.provider.is_none() {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        let (last_content, channel) = if let Some(s) = state.sessions.get(&session_id) {
            (
                s.messages.last().map(|m| m.content.clone()).unwrap_or_default(),
                s.channel.clone()
            )
        } else {
            return;
        };
            
        let content = format!(
            "I received your message: \"{last_content}\". \
             No AI provider configured — run `openclaw onboard` to set one up."
        );
        
        let assistant_msg = ChatMessage {
            id: Uuid::new_v4(),
            role: Role::Assistant,
            content: content.clone(),
            timestamp: Utc::now(),
            channel,
            tool_calls: vec![],
            images: vec![],
            tool_result: None,
        };
        
        let _ = state.sessions.add_message(&session_id, assistant_msg);

        if let Ok(json) = serde_json::to_string(&WsMessage::AgentResponse {
            session_id,
            content,
            done: true,
        }) {
            state.send_to_subscribers(&session_id, &json);
        }
        return;
    }
    
    let provider = state.provider.as_ref().unwrap();
    
    // Per-session config
    let (model, max_tokens, temp_override) = {
        let default_model = state.config.models.default_model.clone();
        let default_max_tokens = state.config.agent.max_tokens;

        if let Some(session) = state.sessions.get(&session_id) {
            let m = session.metadata.get("model").and_then(|v| v.as_str()).map(String::from).unwrap_or(default_model);
            let t = session.metadata.get("temperature").and_then(|v| v.as_f64()).map(|f| f as f32);
            let mt = session.metadata.get("max_tokens").and_then(|v| v.as_u64()).map(|u| u as u32).or(default_max_tokens);
            (m, mt, t)
        } else {
            (default_model, default_max_tokens, None)
        }
    };

    // Loop
    for _turn in 0..5 {
        let (messages, mut system_prompt, channel) = {
            let session = match state.sessions.get(&session_id) {
                Some(s) => s,
                None => return,
            };
            (
                session.messages.clone(),
                state.effective_system_prompt(),
                session.channel.clone(),
            )
        };

        let skill_prompts = state.skills.system_prompts();
        if !skill_prompts.is_empty() {
            system_prompt = Some(format!("{}\n\n{}", system_prompt.unwrap_or_default(), skill_prompts));
        }

        // RAG Retrieval
        if let Some(memory) = &state.memory {
            if let Some(last_msg) = messages.last() {
                if last_msg.role == Role::User {
                    if let Ok(results) = memory.search_memory(&last_msg.content, 3).await {
                        if !results.is_empty() {
                            let context = results.join("\n---\n");
                            let memory_block = format!("\n\n### Relevant Memories\n{}", context);
                            system_prompt = Some(format!("{}{}", system_prompt.unwrap_or_default(), memory_block));
                        }
                    }
                }
            }
        }

        let typing_state = Arc::clone(&state);
        let typing_session_id = session_id;
        let typing_channel = channel.clone();
        let _typing_handle = tokio::spawn(async move {
            if let Some(chan) = typing_state.channels.get(&typing_channel) {
                if let Some(session) = typing_state.sessions.get(&typing_session_id) {
                    let peer_id = session.peer_id.clone();
                    drop(session);
                    let _ = chan.value().send_typing(&peer_id).await;
                }
            }
        });

        let (token_tx, mut token_rx) = mpsc::channel::<String>(256);
        let stream_state = Arc::clone(&state);
        let stream_sid = session_id;

        let stream_task = tokio::spawn(async move {
            while let Some(token) = token_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&WsMessage::AgentResponse {
                    session_id: stream_sid,
                    content: token,
                    done: false,
                }) {
                    stream_state.send_to_subscribers(&stream_sid, &json);
                }
            }
        });

        let tools: Vec<_> = state.tools.iter().map(|t| t.value().definition()).collect();
        let skill_tools = state.skills.all_tools();
        let mcp_tools = state.mcp.get_tools().await;
        let mut all_tools = tools;
        all_tools.extend(skill_tools);
        all_tools.extend(mcp_tools);
        let tools_slice = if all_tools.is_empty() { None } else { Some(all_tools.as_slice()) };

        let result = provider.stream_chat(
             &messages,
             system_prompt.as_deref(),
             &model,
             max_tokens,
             temp_override,
             tools_slice,
             token_tx
        ).await;
        
        let _ = stream_task.await;
        
        match result {
             Ok(resp) => {
                 let assistant_msg = ChatMessage {
                     id: Uuid::new_v4(),
                     role: Role::Assistant,
                     content: resp.content.clone(),
                     timestamp: Utc::now(),
                     channel: channel.clone(),
                     tool_calls: resp.tool_calls.clone(),
                     images: vec![],
                     tool_result: None,
                 };
                 let _ = state.sessions.add_message(&session_id, assistant_msg);
                 
                 // Save to memory
                 if let Some(memory) = &state.memory {
                     if let Some(last_msg) = messages.last() {
                         if last_msg.role == Role::User && !resp.content.is_empty() {
                             let text = format!("User: {}\nAssistant: {}", last_msg.content, resp.content);
                             let mem = memory.clone();
                             tokio::spawn(async move {
                                 let metadata = json!({
                                     "role": "interaction",
                                     "timestamp": Utc::now().to_rfc3339()
                                 });
                                 if let Err(e) = mem.add_memory(&text, metadata).await {
                                     error!("Failed to save memory: {}", e);
                                 }
                             });
                         }
                     }
                 }
                 
                 if resp.tool_calls.is_empty() {
                      if let Ok(json) = serde_json::to_string(&WsMessage::AgentResponse {
                          session_id,
                          content: String::new(),
                          done: true,
                      }) {
                          state.send_to_subscribers(&session_id, &json);
                      }

                    if let Some(chan) = state.channels.get(&channel) {
                        let peer_id = {
                            if let Some(s) = state.sessions.get(&session_id) {
                                s.peer_id.clone()
                            } else {
                                break;
                            }
                        };
                        
                        let voice_reply = {
                            if let Some(s) = state.sessions.get(&session_id) {
                                s.metadata
                                    .get("voice_reply")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false)
                            } else {
                                false
                            }
                        };

                        let chan_ref = chan.value().clone();
                        drop(chan);
                        let content = resp.content.clone();
                        let voice_service = state.voice.clone();

                        tokio::spawn(async move {
                            if let Err(e) = chan_ref.send_message(&peer_id, &content).await {
                                error!("Failed to send to channel: {}", e);
                            }
                            
                            if voice_reply {
                                if let Some(voice) = voice_service {
                                    if let Ok(audio) = voice.speak(&content).await {
                                        if let Err(e) = chan_ref.send_voice(&peer_id, audio).await {
                                            error!("Failed to send voice: {}", e);
                                        }
                                    }
                                }
                            }
                        });
                    }
                      break;
                 }
                 
                 if let Ok(json) = serde_json::to_string(&WsMessage::AgentThinking {
                     session_id,
                 }) {
                     state.send_to_subscribers(&session_id, &json);
                 }
                 
                for tc in resp.tool_calls {
                    let output = if let Some(tool) = state.tools.get(&tc.name) {
                        let mut args = tc.arguments.clone();
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert("_session_id".to_string(), json!(session_id));
                        }
                        
                        match tool.execute(args).await {
                            Ok(s) => s,
                            Err(e) => format!("Error: {e}"),
                        }
                    } else if let Some(skill) = state.skills.enabled_skills().iter().find(|s| {
                        s.tools().iter().any(|t| t.name == tc.name)
                    }) {
                        match skill.execute_tool(&tc.name, tc.arguments.clone()).await {
                            Ok(s) => s,
                            Err(e) => format!("Error: {e}"),
                        }
                    } else if let Some(mcp_tool) = state.mcp.tools.lock().await.iter().find(|t| t.name == tc.name) {
                        match mcp_tool.execute(tc.arguments.clone()).await {
                            Ok(s) => s,
                            Err(e) => format!("Error: {e}"),
                        }
                    } else {
                        format!("Error: Tool not found: {}", tc.name)
                    };
                      
                      let tool_msg = ChatMessage {
                          id: Uuid::new_v4(),
                          role: Role::Tool,
                          content: String::new(),
                          timestamp: Utc::now(),
                          channel: channel.clone(),
                          tool_calls: vec![],
            images: vec![],
                          tool_result: Some(openclaw_core::provider::ToolResult {
                              tool_call_id: tc.id,
                              content: output,
                          }),
                      };
                      let _ = state.sessions.add_message(&session_id, tool_msg);
                 }
             }
             Err(e) => {
                 error!(error = %e, "provider error");
                 if let Ok(json) = serde_json::to_string(&WsMessage::Error {
                     code: "provider_error".to_string(),
                     message: e.to_string(),
                 }) {
                     state.send_to_subscribers(&session_id, &json);
                 }
                 break;
             }
        }
    }
}

pub async fn compact_session(state: Arc<AppState>, session_id: Uuid) -> Result<String, String> {
    let session_opt = state.sessions.get(&session_id);
    if session_opt.is_none() {
        return Err("Session not found".to_string());
    }
    let session = session_opt.unwrap();
    let msg_count = session.messages.len();
    if msg_count < 10 {
        return Ok("Session too short to compact.".to_string());
    }

    let keep_count = 5;
    let compact_count = msg_count - keep_count;
    let to_summarize: Vec<ChatMessage> = session.messages.iter().take(compact_count).cloned().collect();
    drop(session);

    let context_str = to_summarize.iter()
        .map(|m| format!("{:?}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n\n");

    if let Some(provider) = &state.provider {
        let (tx, _rx) = mpsc::channel(1);
        let summary_prompt = "Summarize the following conversation history into a concise context summary. Focus on key facts, user preferences, and unresolved tasks.";
        let msgs = vec![ChatMessage {
            id: Uuid::new_v4(),
            role: Role::User,
            content: format!("{}\n\nConversation:\n{}", summary_prompt, context_str),
            timestamp: Utc::now(),
            channel: ChannelKind::Api,
            tool_calls: vec![],
            images: vec![],
            tool_result: None,
        }];
        let model = &state.config.models.default_model;

        match provider.stream_chat(&msgs, None, model, None, None, None, tx).await {
            Ok(resp) => {
                let summary = resp.content;
                let summary_msg = ChatMessage {
                    id: Uuid::new_v4(),
                    role: Role::System,
                    content: format!("CONTEXT SUMMARY:\n{}", summary),
                    timestamp: Utc::now(),
                    channel: ChannelKind::Api,
                    tool_calls: vec![],
            images: vec![],
                    tool_result: None,
                };
                if let Err(e) = state.sessions.compact(&session_id, compact_count, summary_msg) {
                    return Err(format!("Failed to compact session: {}", e));
                }
                Ok(format!("Compacted {} messages into summary.", compact_count))
            }
            Err(e) => Err(format!("Provider error: {}", e)),
        }
    } else {
        Err("No provider configured.".to_string())
    }
}
