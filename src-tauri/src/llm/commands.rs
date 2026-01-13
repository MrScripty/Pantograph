use super::server::SharedLlamaServer;
use super::types::*;
use crate::agent;
use crate::agent::{AgentEvent, AgentEventType, AgentRequest, AgentResponse, ComponentUpdate, FileChange, FileAction, Position, Size};
use futures_util::StreamExt;
use reqwest::Client;
use rig::completion::Prompt;
use std::path::PathBuf;
use tauri::{command, ipc::Channel, AppHandle, Manager, State};

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

#[command]
pub async fn run_agent(
    app: AppHandle,
    server: State<'_, SharedLlamaServer>,
    request: AgentRequest,
    channel: Channel<AgentEvent>,
) -> Result<AgentResponse, String> {
    log::info!("[run_agent] Starting agent with prompt: {}", request.prompt);

    // Get the LLM server URL
    let server_guard = server.read().await;
    if !server_guard.is_ready() {
        log::error!("[run_agent] LLM server not ready");
        return Err("LLM server not ready".to_string());
    }

    let base_url = server_guard
        .base_url()
        .ok_or_else(|| "No server URL configured".to_string())?;
    log::info!("[run_agent] Using LLM server at: {}", base_url);
    drop(server_guard);

    // Get the project root - in dev mode, current_dir is src-tauri, so we go up one level
    // to get to the actual project root where src/generated lives
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    let project_root = current_dir
        .parent()
        .map(PathBuf::from)
        .unwrap_or(current_dir);
    log::info!("[run_agent] Project root: {:?}", project_root);

    // Notify frontend that we're starting
    channel
        .send(AgentEvent {
            event_type: AgentEventType::Content,
            data: Some(serde_json::json!({ "message": "Starting UI generation..." })),
        })
        .ok();

    // Step 1: Use vision API to analyze the drawing first
    log::info!("[run_agent] Step 1: Analyzing drawing with vision API...");
    let vision_analysis = analyze_drawing_with_vision(&base_url, &request).await?;
    log::info!("[run_agent] Vision analysis result: {}", vision_analysis);

    channel
        .send(AgentEvent {
            event_type: AgentEventType::Content,
            data: Some(serde_json::json!({ "message": "Analyzed drawing, generating component..." })),
        })
        .ok();

    // Step 2: Create the RIG client and agent for tool execution
    log::info!("[run_agent] Step 2: Creating RIG agent...");
    let client = agent::create_client(&base_url)?;
    let ui_agent = agent::create_ui_agent(&client, "default", project_root.clone());

    // Build the prompt with vision analysis included
    let prompt = format_agent_prompt_with_analysis(&request, &vision_analysis);
    log::info!("[run_agent] Agent prompt: {}", prompt);

    // Run the agent - RIG handles the tool-calling loop
    // multi_turn(5) allows up to 5 tool-calling rounds before requiring a final response
    // This enables the agent to call tools (like write_gui_file) and handle validation errors
    log::info!("[run_agent] Running RIG agent...");
    let response: String = ui_agent
        .prompt(&prompt)
        .multi_turn(5)
        .await
        .map_err(|e| {
            log::error!("[run_agent] Agent error: {}", e);
            format!("Agent error: {}", e)
        })?;
    log::info!("[run_agent] Agent response: {}", response);

    // Parse the response and extract file changes
    let file_changes = extract_file_changes(&project_root);
    log::info!("[run_agent] Found {} file changes", file_changes.len());

    let component_updates = create_component_updates(&request, &file_changes);
    log::info!("[run_agent] Created {} component updates", component_updates.len());

    // Send completion event
    channel
        .send(AgentEvent {
            event_type: AgentEventType::Done,
            data: Some(serde_json::json!({
                "message": response,
                "files_changed": file_changes.len()
            })),
        })
        .ok();

    Ok(AgentResponse {
        file_changes,
        component_updates,
        message: response,
    })
}

/// Analyze the drawing using vision API
async fn analyze_drawing_with_vision(base_url: &str, request: &AgentRequest) -> Result<String, String> {
    let client = Client::new();

    let vision_prompt = format!(
        "Analyze this UI sketch and describe what UI component the user wants to create. \
        Be specific about the shape, layout, and any text visible. \
        The user's request is: {}",
        request.prompt
    );

    let chat_request = ChatRequest {
        model: "default".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrlData {
                        url: format!("data:image/png;base64,{}", request.image_base64),
                    },
                },
                ContentPart::Text { text: vision_prompt },
            ],
        }],
        stream: false,
        max_tokens: Some(1024),
        temperature: Some(0.3),
    };

    let response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&chat_request)
        .send()
        .await
        .map_err(|e| format!("Vision request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Vision API error {}: {}", status, body));
    }

    // Parse the non-streaming response
    let json_response: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse vision response: {}", e))?;

    let content = json_response["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Unable to analyze drawing")
        .to_string();

    Ok(content)
}

/// Format the agent prompt with vision analysis
fn format_agent_prompt_with_analysis(request: &AgentRequest, vision_analysis: &str) -> String {
    let mut prompt = String::new();

    // Add vision analysis
    prompt.push_str(&format!(
        "## Drawing Analysis\n{}\n\n",
        vision_analysis
    ));

    // Add drawing bounds context
    if let Some(bounds) = &request.drawing_bounds {
        prompt.push_str(&format!(
            "## Drawing Location\nThe user drew at position: x={:.0}, y={:.0}, width={:.0}, height={:.0}\n\n",
            bounds.min_x, bounds.min_y, bounds.width, bounds.height
        ));
    }

    // Add target element context
    if let Some(target_id) = &request.target_element_id {
        prompt.push_str(&format!(
            "## Target Element\nThe user is drawing on/near existing component: {}\n\n",
            target_id
        ));
    }

    // Add component tree context
    if !request.component_tree.is_empty() {
        prompt.push_str("## Existing Components\n");
        for comp in &request.component_tree {
            prompt.push_str(&format!(
                "- {} ({}) at ({:.0}, {:.0})\n",
                comp.name, comp.path, comp.bounds.x, comp.bounds.y
            ));
        }
        prompt.push_str("\n");
    }

    // Add user's request
    prompt.push_str(&format!(
        "## User's Request\n{}\n\n\
        Based on the drawing analysis and user request, use the write_gui_file tool to create the Svelte component.",
        request.prompt
    ));

    prompt
}

/// Format the agent prompt with all context (legacy, kept for reference)
#[allow(dead_code)]
fn format_agent_prompt(request: &AgentRequest) -> String {
    let mut prompt = String::new();

    // Add drawing bounds context
    if let Some(bounds) = &request.drawing_bounds {
        prompt.push_str(&format!(
            "## Drawing Location\nThe user drew at position: x={:.0}, y={:.0}, width={:.0}, height={:.0}\n\n",
            bounds.min_x, bounds.min_y, bounds.width, bounds.height
        ));
    }

    // Add target element context
    if let Some(target_id) = &request.target_element_id {
        prompt.push_str(&format!(
            "## Target Element\nThe user is drawing on/near existing component: {}\n\n",
            target_id
        ));
    }

    // Add component tree context
    if !request.component_tree.is_empty() {
        prompt.push_str("## Existing Components\n");
        for comp in &request.component_tree {
            prompt.push_str(&format!(
                "- {} ({}) at ({:.0}, {:.0})\n",
                comp.name, comp.path, comp.bounds.x, comp.bounds.y
            ));
        }
        prompt.push_str("\n");
    }

    // Add image reference
    prompt.push_str(&format!(
        "## User's Drawing\n[The image shows the user's sketch]\n\n## User's Request\n{}",
        request.prompt
    ));

    prompt
}

/// Extract file changes from the generated directory (recursive)
fn extract_file_changes(project_root: &PathBuf) -> Vec<FileChange> {
    let generated_path = project_root.join("src").join("generated");
    let mut changes = Vec::new();

    fn collect_svelte_files(dir: &PathBuf, base: &PathBuf, changes: &mut Vec<FileChange>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Recurse into subdirectories
                    collect_svelte_files(&path, base, changes);
                } else if path.extension().map_or(false, |ext| ext == "svelte") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let relative_path = path
                            .strip_prefix(base)
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_default();

                        changes.push(FileChange {
                            path: relative_path,
                            action: FileAction::Create,
                            content: Some(content),
                        });
                    }
                }
            }
        }
    }

    collect_svelte_files(&generated_path, &generated_path, &mut changes);
    changes
}

/// Create component updates based on file changes and request context
fn create_component_updates(request: &AgentRequest, file_changes: &[FileChange]) -> Vec<ComponentUpdate> {
    file_changes
        .iter()
        .filter_map(|change| {
            change.content.as_ref().map(|content| {
                let id = change.path
                    .trim_end_matches(".svelte")
                    .replace('/', "_")
                    .replace('\\', "_");

                // Position based on drawing bounds or default
                let (x, y, width, height) = if let Some(bounds) = &request.drawing_bounds {
                    (bounds.min_x, bounds.min_y, bounds.width, bounds.height)
                } else {
                    (100.0, 100.0, 200.0, 100.0)
                };

                ComponentUpdate {
                    id,
                    path: change.path.clone(),
                    position: Position { x, y },
                    size: Size { width, height },
                    source: content.clone(),
                }
            })
        })
        .collect()
}
