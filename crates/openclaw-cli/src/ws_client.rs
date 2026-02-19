use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use openclaw_core::WsMessage;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub async fn send_and_receive(
    url: &str,
    msg: WsMessage,
    on_thinking: impl Fn(),
    mut on_token: impl FnMut(&str),
) -> Result<String> {
    let (ws_stream, _) = connect_async(url)
        .await
        .context("failed to connect to gateway WebSocket — is the gateway running?")?;

    let (mut write, mut read) = ws_stream.split();

    let json = serde_json::to_string(&msg)?;
    write.send(Message::Text(json.into())).await?;

    let mut full_response = String::new();

    while let Some(msg_result) = read.next().await {
        let ws_msg = msg_result.context("WebSocket read error")?;
        match ws_msg {
            Message::Text(text) => {
                if let Ok(parsed) = serde_json::from_str::<WsMessage>(&text) {
                    match parsed {
                        WsMessage::AgentThinking { .. } => {
                            on_thinking();
                        }
                        WsMessage::AgentResponse {
                            content, done, ..
                        } => {
                            on_token(&content);
                            full_response.push_str(&content);
                            if done {
                                break;
                            }
                        }
                        WsMessage::Error { message, .. } => {
                            return Err(anyhow!("gateway error: {}", message));
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    let _ = write.close().await;
    Ok(full_response)
}
