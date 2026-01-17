//! Agent orchestration and execution commands.

use super::shared::{get_project_data_dir, SharedAppConfig, MAX_IMAGE_BASE64_LEN};
use crate::agent;
use crate::agent::rag::SharedRagManager;
use crate::agent::tools::WriteGuiFileArgs;
use crate::agent::{
    AgentEvent, AgentEventType, AgentRequest, AgentResponse, ComponentUpdate, FileAction,
    FileChange, Position, Size, WriteTracker,
};
use crate::llm::gateway::SharedGateway;
use crate::llm::types::*;
use reqwest::Client;
use rig::agent::MultiTurnStreamItem;
use rig::streaming::{
    StreamedAssistantContent, StreamedUserContent, StreamingPrompt, ToolCallDeltaContent,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{command, ipc::Channel, AppHandle, State};
use futures_util::StreamExt;

#[command]
pub async fn run_agent(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    rag_manager: State<'_, SharedRagManager>,
    config: State<'_, SharedAppConfig>,
    request: AgentRequest,
    channel: Channel<AgentEvent>,
) -> Result<AgentResponse, String> {
    // Validate image size to prevent DoS
    if request.image_base64.len() > MAX_IMAGE_BASE64_LEN {
        return Err(format!(
            "Image too large: {} bytes (max {} bytes)",
            request.image_base64.len(),
            MAX_IMAGE_BASE64_LEN
        ));
    }

    log::info!("[run_agent] Starting agent with prompt: {}", request.prompt);

    // Get the LLM server URL
    if !gateway.is_ready().await {
        log::error!("[run_agent] LLM server not ready");
        return Err("LLM server not ready".to_string());
    }

    let base_url = gateway
        .base_url()
        .await
        .ok_or_else(|| "No server URL configured".to_string())?;
    log::info!("[run_agent] Using LLM server at: {}", base_url);

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
            data: Some(
                serde_json::json!({ "message": "Analyzed drawing, generating component..." }),
            ),
        })
        .ok();

    // Step 2: Create the RIG client and agent for tool execution
    log::info!("[run_agent] Step 2: Creating RIG agent...");

    let client = agent::create_client(&base_url)?;

    // Create write tracker to track files written during this session
    let write_tracker: WriteTracker = Arc::new(Mutex::new(Vec::new()));

    // Ensure RAG manager has the embedding URL before agent runs
    // This is needed for vector search to work when compile errors occur
    let vector_search_available = if let Some(url) = gateway.embedding_url().await {
        let mut rag = rag_manager.write().await;
        rag.set_embedding_url(url.clone());
        log::info!("[run_agent] Set embedding URL in RAG manager: {}", url);
        // Check if vector search is fully available (embedding URL + indexed vectors)
        rag.is_search_available()
    } else {
        log::warn!("[run_agent] No embedding URL available - vector search will not work");
        false
    };

    // Build enricher registry - this provides automatic documentation enrichment for errors
    // The agent does NOT get doc search tools - documentation is served programmatically
    let mut enricher_registry = agent::EnricherRegistry::new();
    if vector_search_available {
        enricher_registry.register(Box::new(agent::SvelteDocsEnricher::new(
            rag_manager.inner().clone(),
        )));
        log::info!("[run_agent] Registered SvelteDocsEnricher for automatic error enrichment");
    } else {
        log::info!("[run_agent] Vector search not available - errors will not include auto-docs");
    }
    let enricher_registry = Arc::new(enricher_registry);

    // Get sandbox config for import validation
    let sandbox_config = {
        let config_guard = config.read().await;
        config_guard.sandbox.clone()
    };
    log::info!(
        "[run_agent] Using sandbox config with import validation mode: {:?}",
        sandbox_config.import_validation_mode
    );

    let ui_agent = agent::create_ui_agent(
        &client,
        "default",
        project_root.clone(),
        enricher_registry,
        write_tracker.clone(),
        sandbox_config,
    );

    // Build the prompt with vision analysis included
    let prompt = format_agent_prompt_with_analysis(&request, &vision_analysis);
    log::info!("[run_agent] Agent prompt: {}", prompt);

    // Send the prompt to the frontend for visibility
    channel
        .send(AgentEvent {
            event_type: AgentEventType::Content,
            data: Some(serde_json::json!({
                "type": "system_prompt",
                "prompt": prompt
            })),
        })
        .ok();

    // Run the agent with streaming
    // multi_turn(5) allows up to 5 tool-calling rounds before requiring a final response
    // This enables the agent to call tools (like write_gui_file) and handle validation errors
    // Note: Validation errors should include relevant docs automatically, so agent doesn't waste turns searching
    log::info!("[run_agent] Running RIG agent with streaming...");

    let mut stream = ui_agent.stream_prompt(&prompt).multi_turn(5).await;

    let mut final_response = String::new();
    // Track the last write_gui_file tool call arguments for early termination
    let mut last_write_gui_file_args: Option<String> = None;

    while let Some(item) = stream.next().await {
        match item {
            Ok(MultiTurnStreamItem::StreamAssistantItem(content)) => {
                match content {
                    StreamedAssistantContent::Text(text) => {
                        // Send streaming text chunk to frontend
                        channel
                            .send(AgentEvent {
                                event_type: AgentEventType::Content,
                                data: Some(serde_json::json!({
                                    "type": "text_chunk",
                                    "chunk": text.text
                                })),
                            })
                            .ok();
                        final_response.push_str(&text.text);
                    }
                    StreamedAssistantContent::ToolCall(tool_call) => {
                        // Send tool call event to frontend
                        channel
                            .send(AgentEvent {
                                event_type: AgentEventType::ToolCall,
                                data: Some(serde_json::json!({
                                    "name": tool_call.function.name,
                                    "arguments": tool_call.function.arguments.to_string()
                                })),
                            })
                            .ok();
                        log::info!(
                            "[run_agent] Tool call: {} with args: {}",
                            tool_call.function.name,
                            tool_call.function.arguments
                        );

                        // Track write_gui_file calls for early termination on success
                        if tool_call.function.name == "write_gui_file" {
                            last_write_gui_file_args =
                                Some(tool_call.function.arguments.to_string());
                        }
                    }
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        // Send reasoning to frontend (for models that support it)
                        let reasoning_text = reasoning.reasoning.join("\n");
                        channel
                            .send(AgentEvent {
                                event_type: AgentEventType::Content,
                                data: Some(serde_json::json!({
                                    "type": "reasoning",
                                    "text": reasoning_text
                                })),
                            })
                            .ok();
                    }
                    StreamedAssistantContent::ToolCallDelta { id, content } => {
                        // Send tool call delta for real-time streaming of tool arguments
                        let content_data = match content {
                            ToolCallDeltaContent::Name(name) => serde_json::json!({
                                "type": "name",
                                "value": name
                            }),
                            ToolCallDeltaContent::Delta(delta) => serde_json::json!({
                                "type": "delta",
                                "value": delta
                            }),
                        };
                        channel
                            .send(AgentEvent {
                                event_type: AgentEventType::ToolCallDelta,
                                data: Some(serde_json::json!({
                                    "id": id,
                                    "content": content_data
                                })),
                            })
                            .ok();
                    }
                    StreamedAssistantContent::ReasoningDelta { id, reasoning } => {
                        // Send reasoning delta for real-time streaming
                        channel
                            .send(AgentEvent {
                                event_type: AgentEventType::Content,
                                data: Some(serde_json::json!({
                                    "type": "reasoning_delta",
                                    "id": id,
                                    "text": reasoning
                                })),
                            })
                            .ok();
                    }
                    _ => {} // Handle other variants (Final, etc.)
                }
            }
            Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => {
                // Tool results
                let StreamedUserContent::ToolResult(result) = user_content;
                let result_text = result
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        rig::message::ToolResultContent::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                channel
                    .send(AgentEvent {
                        event_type: AgentEventType::ToolResult,
                        data: Some(serde_json::json!({
                            "tool_id": result.id,
                            "output": result_text
                        })),
                    })
                    .ok();
                log::info!(
                    "[run_agent] Tool result for {}: {}",
                    result.id,
                    result_text
                );

                // Early termination: if write_gui_file succeeded, send ComponentCreated and stop
                if result_text == "true" {
                    if let Some(args_json) = last_write_gui_file_args.take() {
                        if let Ok(args) = serde_json::from_str::<WriteGuiFileArgs>(&args_json) {
                            // Create ComponentUpdate from the tool args and request context
                            let id = args
                                .path
                                .trim_end_matches(".svelte")
                                .replace('/', "_")
                                .replace('\\', "_");

                            let (x, y, width, height) = if let Some(bounds) = &request.drawing_bounds
                            {
                                (bounds.min_x, bounds.min_y, bounds.width, bounds.height)
                            } else {
                                (100.0, 100.0, 200.0, 100.0)
                            };

                            let component_update = ComponentUpdate {
                                id,
                                path: args.path.clone(),
                                position: Position { x, y },
                                size: Size { width, height },
                                source: args.content,
                            };

                            // Send ComponentCreated event for immediate frontend rendering
                            channel
                                .send(AgentEvent {
                                    event_type: AgentEventType::ComponentCreated,
                                    data: Some(serde_json::to_value(&component_update).unwrap()),
                                })
                                .ok();
                            log::info!(
                                "[run_agent] Component created successfully, stopping agent early"
                            );

                            break;
                        }
                    }
                }
            }
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                final_response = response.response().to_string();
                log::info!("[run_agent] Final response received");
            }
            Ok(_) => {
                // Handle future/unknown variants
            }
            Err(e) => {
                let error_str = e.to_string();
                log::error!("[run_agent] Stream error: {}", error_str);

                channel
                    .send(AgentEvent {
                        event_type: AgentEventType::Error,
                        data: Some(serde_json::json!({ "error": error_str })),
                    })
                    .ok();
                return Err(format!("Agent stream error: {}", error_str));
            }
        }
    }

    log::info!("[run_agent] Agent response: {}", final_response);
    let response = final_response;

    // Get files written during this session from the write tracker
    let written_files = write_tracker
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let file_changes: Vec<FileChange> = written_files
        .iter()
        .filter_map(|path| {
            let full_path = project_root.join("src").join("generated").join(path);
            std::fs::read_to_string(&full_path)
                .ok()
                .map(|content| FileChange {
                    path: path.clone(),
                    action: FileAction::Create,
                    content: Some(content),
                })
        })
        .collect();
    log::info!("[run_agent] Found {} file changes", file_changes.len());

    let component_updates = create_component_updates(&request, &file_changes);
    log::info!(
        "[run_agent] Created {} component updates",
        component_updates.len()
    );

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
async fn analyze_drawing_with_vision(
    base_url: &str,
    request: &AgentRequest,
) -> Result<String, String> {
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
                ContentPart::Text {
                    text: vision_prompt,
                },
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

/// Create component updates based on file changes and request context
fn create_component_updates(
    request: &AgentRequest,
    file_changes: &[FileChange],
) -> Vec<ComponentUpdate> {
    file_changes
        .iter()
        .filter_map(|change| {
            change.content.as_ref().map(|content| {
                let id = change
                    .path
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
