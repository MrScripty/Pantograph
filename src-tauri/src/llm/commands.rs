use super::server::SharedLlamaServer;
use super::types::*;
use futures_util::StreamExt;
use reqwest::Client;
use tauri::{command, ipc::Channel, AppHandle, State};

#[command]
pub async fn send_vision_prompt(
    _app: AppHandle,
    server: State<'_, SharedLlamaServer>,
    prompt: String,
    image_base64: String,
    channel: Channel<StreamEvent>,
) -> Result<(), String> {
    let server_guard = server.read().await;
    if !server_guard.is_ready() {
        return Err("LLM server not ready".to_string());
    }

    let base_url = server_guard
        .base_url()
        .ok_or_else(|| "No server URL configured".to_string())?;
    drop(server_guard);

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
                            channel
                                .send(StreamEvent {
                                    content: None,
                                    done: true,
                                    error: None,
                                })
                                .ok();
                            return Ok(());
                        }

                        if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                            if let Some(choice) = chunk.choices.first() {
                                if let Some(content) = &choice.delta.content {
                                    channel
                                        .send(StreamEvent {
                                            content: Some(content.clone()),
                                            done: false,
                                            error: None,
                                        })
                                        .ok();
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                channel
                    .send(StreamEvent {
                        content: None,
                        done: true,
                        error: Some(e.to_string()),
                    })
                    .ok();
                return Err(e.to_string());
            }
        }
    }

    // Send done signal if stream ended without [DONE]
    channel
        .send(StreamEvent {
            content: None,
            done: true,
            error: None,
        })
        .ok();

    Ok(())
}

#[command]
pub async fn connect_to_server(
    server: State<'_, SharedLlamaServer>,
    url: String,
) -> Result<LLMStatus, String> {
    let mut server_guard = server.write().await;
    server_guard.connect_external(&url).await?;
    Ok(server_guard.status())
}

#[command]
pub async fn start_sidecar_llm(
    app: AppHandle,
    server: State<'_, SharedLlamaServer>,
    model_path: String,
    mmproj_path: String,
) -> Result<LLMStatus, String> {
    let mut server_guard = server.write().await;
    server_guard
        .start_sidecar(&app, &model_path, &mmproj_path)
        .await?;
    Ok(server_guard.status())
}

#[command]
pub async fn get_llm_status(server: State<'_, SharedLlamaServer>) -> Result<LLMStatus, String> {
    let server_guard = server.read().await;
    Ok(server_guard.status())
}

#[command]
pub async fn stop_llm(server: State<'_, SharedLlamaServer>) -> Result<(), String> {
    let mut server_guard = server.write().await;
    server_guard.stop();
    Ok(())
}
