//! Vision prompt handling commands.

use super::shared::MAX_IMAGE_BASE64_LEN;
use crate::llm::gateway::SharedGateway;
use crate::llm::types::*;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::{command, ipc::Channel, AppHandle, State};

#[command]
pub async fn send_vision_prompt(
    _app: AppHandle,
    gateway: State<'_, SharedGateway>,
    prompt: String,
    image_base64: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    // Validate image size to prevent DoS
    if image_base64.len() > MAX_IMAGE_BASE64_LEN {
        return Err(format!(
            "Image too large: {} bytes (max {} bytes)",
            image_base64.len(),
            MAX_IMAGE_BASE64_LEN
        ));
    }

    if !gateway.is_ready().await {
        return Err("LLM server not ready".to_string());
    }

    let base_url = gateway
        .base_url()
        .await
        .ok_or_else(|| "No server URL configured".to_string())?;

    let client = Client::new();

    let request = ChatRequest {
        model: "default".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrlData {
                        url: format!("data:image/png;base64,{}", image_base64),
                    },
                },
                ContentPart::Text { text: prompt },
            ],
        }],
        stream: true,
        max_tokens: Some(2048),
        temperature: Some(0.7),
    };

    let response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&request)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("HTTP error {}: {}", status, body));
    }

    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            let _ = channel.send(StreamEvent {
                                content: None,
                                done: true,
                                error: None,
                            });
                            return Ok(());
                        }

                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    // If channel send fails, the receiver has dropped
                                    // We could break here, but the stream will end naturally
                                    let _ = channel.send(StreamEvent {
                                        content: Some(content.clone()),
                                        done: false,
                                        error: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = channel.send(StreamEvent {
                    content: None,
                    done: true,
                    error: Some(e.to_string()),
                });
                return Err(e.to_string());
            }
        }
    }

    // Send done signal if stream ended without [DONE]
    let _ = channel.send(StreamEvent {
        content: None,
        done: true,
        error: None,
    });

    Ok(())
}
