use super::gateway::SharedGateway;
use super::types::*;
use super::BackendConfig;
use crate::agent;
use crate::agent::docs::DocsStatus;
use crate::agent::rag::{IndexingProgress, RagStatus, SharedRagManager};
use crate::agent::{
    AgentEvent, AgentEventType, AgentRequest, AgentResponse, ComponentUpdate, DocsManager,
    FileAction, FileChange, Position, Size, WriteTracker,
};
use crate::config::{AppConfig, DeviceConfig, DeviceInfo, EmbeddingMemoryMode, ModelConfig, SandboxConfig, ServerModeInfo};
use crate::constants::paths::DATA_DIR;
use futures_util::StreamExt;
use reqwest::Client;
use rig::agent::MultiTurnStreamItem;
use rig::streaming::{StreamedAssistantContent, StreamedUserContent, StreamingPrompt};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{command, ipc::Channel, AppHandle, Manager, State};
use tauri_plugin_shell::ShellExt;
use tokio::sync::RwLock;

/// Maximum allowed size for base64-encoded images (5MB after decoding)
/// Base64 encoding increases size by ~33%, so we check for ~6.7MB encoded
const MAX_IMAGE_BASE64_LEN: usize = 7 * 1024 * 1024;

/// Shared app configuration
pub type SharedAppConfig = Arc<RwLock<AppConfig>>;

/// Get the project data directory for docs and RAG storage.
/// Uses CARGO_MANIFEST_DIR (src-tauri/) and goes up one level to get project root.
/// This ensures the data directory is at the project root regardless of the
/// current working directory (which varies during `tauri dev`).
fn get_project_data_dir() -> Result<PathBuf, String> {
    // CARGO_MANIFEST_DIR is set at compile time to the directory containing Cargo.toml (src-tauri/)
    // We go up one level to get the actual project root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let project_root = std::path::Path::new(manifest_dir)
        .parent()
        .ok_or_else(|| "Failed to get project root from CARGO_MANIFEST_DIR".to_string())?;

    let data_dir = project_root.join(DATA_DIR);

    // Create the directory if it doesn't exist
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;
    }

    Ok(data_dir)
}

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

#[command]
pub async fn connect_to_server(
    _gateway: State<'_, SharedGateway>,
    _url: String,
) -> Result<LLMStatus, String> {
    // TODO: Implement connect_external through gateway interface
    // For now, this feature is disabled during the gateway migration
    Err("External server connection not yet supported through gateway".to_string())
}

#[command]
pub async fn start_sidecar_llm(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    model_path: String,
    mmproj_path: String,
) -> Result<LLMStatus, String> {
    let config_guard = config.read().await;
    let device = config_guard.device.clone();
    drop(config_guard);

    let backend_config = BackendConfig {
        model_path: Some(std::path::PathBuf::from(&model_path)),
        mmproj_path: Some(std::path::PathBuf::from(&mmproj_path)),
        device: Some(device.device),
        gpu_layers: Some(device.gpu_layers),
        embedding_mode: false,
        ..Default::default()
    };

    gateway
        .start(&backend_config, &app)
        .await
        .map_err(|e| e.to_string())?;

    Ok(LLMStatus {
        ready: gateway.is_ready().await,
        mode: "sidecar_inference".to_string(),
        url: gateway.base_url().await,
    })
}

#[command]
pub async fn get_llm_status(gateway: State<'_, SharedGateway>) -> Result<LLMStatus, String> {
    let ready = gateway.is_ready().await;
    let url = gateway.base_url().await;
    let backend_name = gateway.current_backend_name().await;

    Ok(LLMStatus {
        ready,
        mode: if ready {
            format!("sidecar_{}", backend_name)
        } else {
            "none".to_string()
        },
        url,
    })
}

#[command]
pub async fn stop_llm(gateway: State<'_, SharedGateway>) -> Result<(), String> {
    gateway.stop().await;
    Ok(())
}

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
            data: Some(serde_json::json!({ "message": "Analyzed drawing, generating component..." })),
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
        enricher_registry.register(Box::new(agent::SvelteDocsEnricher::new(rag_manager.inner().clone())));
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
    log::info!("[run_agent] Using sandbox config with import validation mode: {:?}", sandbox_config.import_validation_mode);

    let ui_agent = agent::create_ui_agent(&client, "default", project_root.clone(), enricher_registry, write_tracker.clone(), sandbox_config);

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

    // Run the agent with streaming - RIG handles the tool-calling loop
    // multi_turn(5) allows up to 5 tool-calling rounds before requiring a final response
    // This enables the agent to call tools (like write_gui_file) and handle validation errors
    // Note: Validation errors should include relevant docs automatically, so agent doesn't waste turns searching
    log::info!("[run_agent] Running RIG agent with streaming...");
    let mut stream = ui_agent
        .stream_prompt(&prompt)
        .multi_turn(5)
        .await;

    let mut final_response = String::new();

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
                        log::info!("[run_agent] Tool call: {} with args: {}",
                            tool_call.function.name,
                            tool_call.function.arguments);
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
                    _ => {} // Handle other variants (Final, ToolCallDelta, etc.)
                }
            }
            Ok(MultiTurnStreamItem::StreamUserItem(user_content)) => {
                // Tool results
                let StreamedUserContent::ToolResult(result) = user_content;
                let result_text = result.content.iter()
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
                log::info!("[run_agent] Tool result for {}: {}", result.id, result_text);
            }
            Ok(MultiTurnStreamItem::FinalResponse(response)) => {
                final_response = response.response().to_string();
                log::info!("[run_agent] Final response received");
            }
            Ok(_) => {
                // Handle future/unknown variants
            }
            Err(e) => {
                log::error!("[run_agent] Stream error: {}", e);
                channel
                    .send(AgentEvent {
                        event_type: AgentEventType::Error,
                        data: Some(serde_json::json!({ "error": e.to_string() })),
                    })
                    .ok();
                return Err(format!("Agent stream error: {}", e));
            }
        }
    }

    log::info!("[run_agent] Agent response: {}", final_response);
    let response = final_response;

    // Get files written during this session from the write tracker
    let written_files = write_tracker.lock().unwrap_or_else(|e| e.into_inner()).clone();
    let file_changes: Vec<FileChange> = written_files
        .iter()
        .filter_map(|path| {
            let full_path = project_root.join("src").join("generated").join(path);
            std::fs::read_to_string(&full_path).ok().map(|content| FileChange {
                path: path.clone(),
                action: FileAction::Create,
                content: Some(content),
            })
        })
        .collect();
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

// ============================================================================
// Svelte Documentation Commands
// ============================================================================

#[command]
pub async fn get_svelte_docs_status(_app: AppHandle) -> Result<DocsStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);
    Ok(docs_manager.get_status())
}

#[command]
pub async fn update_svelte_docs(_app: AppHandle) -> Result<DocsStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    log::info!("Downloading Svelte 5 documentation...");
    docs_manager.download_docs().await
        .map_err(|e| format!("Failed to download docs: {}", e))?;

    log::info!("Building search index...");
    docs_manager.build_index().await
        .map_err(|e| format!("Failed to build index: {}", e))?;

    Ok(docs_manager.get_status())
}

// ============================================================================
// RAG (Retrieval Augmented Generation) Commands
// ============================================================================

/// Event sent during document indexing
#[derive(Clone, serde::Serialize)]
pub struct IndexingEvent {
    pub current: usize,
    pub total: usize,
    pub status: String,
    pub done: bool,
    pub error: Option<String>,
}

impl From<IndexingProgress> for IndexingEvent {
    fn from(progress: IndexingProgress) -> Self {
        Self {
            current: progress.current,
            total: progress.total,
            status: progress.status,
            done: false,
            error: None,
        }
    }
}

#[command]
pub async fn get_rag_status(
    _app: AppHandle,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<RagStatus, String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    let mut manager = rag_manager.write().await;
    manager.update_docs_status(&docs_manager);

    // Load existing vectors from disk if not already loaded
    if !manager.status().vectors_indexed {
        if let Err(e) = manager.load_from_disk().await {
            log::warn!("Failed to load vectors from disk: {}", e);
        }
    }

    Ok(manager.status().clone())
}

#[command]
pub async fn check_embedding_server(url: String) -> Result<bool, String> {
    Ok(crate::agent::check_embedding_server(&url).await)
}

#[command]
pub async fn set_embedding_server_url(
    rag_manager: State<'_, SharedRagManager>,
    url: String,
) -> Result<bool, String> {
    let mut manager = rag_manager.write().await;
    manager.set_embedding_url(url);
    let available = manager.check_vectorizer().await;
    Ok(available)
}

#[command]
pub async fn index_rag_documents(
    _app: AppHandle,
    rag_manager: State<'_, SharedRagManager>,
    channel: Channel<IndexingEvent>,
) -> Result<(), String> {
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    // Ensure docs are available
    docs_manager.ensure_docs_available().await
        .map_err(|e| format!("Failed to ensure docs available: {}", e))?;

    // Load the search index
    let index = docs_manager.load_index()
        .map_err(|e| format!("Failed to load search index: {}", e))?;

    let version = docs_manager.get_status().version.unwrap_or_else(|| "unknown".to_string());

    // Create a progress callback that sends to the channel
    let channel_clone = channel.clone();
    let on_progress = move |progress: IndexingProgress| {
        channel_clone.send(IndexingEvent::from(progress)).ok();
    };

    // Index documents
    let mut manager = rag_manager.write().await;
    match manager.index_documents(&index.entries, &version, on_progress).await {
        Ok(()) => {
            channel.send(IndexingEvent {
                current: index.entries.len(),
                total: index.entries.len(),
                status: "Complete".to_string(),
                done: true,
                error: None,
            }).ok();
            Ok(())
        }
        Err(e) => {
            channel.send(IndexingEvent {
                current: 0,
                total: 0,
                status: "Failed".to_string(),
                done: true,
                error: Some(e.to_string()),
            }).ok();
            Err(e.to_string())
        }
    }
}

#[command]
pub async fn load_rag_from_disk(
    rag_manager: State<'_, SharedRagManager>,
) -> Result<bool, String> {
    let mut manager = rag_manager.write().await;
    manager.load_from_disk().await
        .map_err(|e| format!("Failed to load RAG from disk: {}", e))
}

#[command]
pub async fn clear_rag_cache(
    rag_manager: State<'_, SharedRagManager>,
) -> Result<(), String> {
    let mut manager = rag_manager.write().await;
    manager.clear_cache().await
        .map_err(|e| format!("Failed to clear RAG cache: {}", e))
}

#[command]
pub async fn search_rag(
    rag_manager: State<'_, SharedRagManager>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<crate::agent::SvelteDoc>, String> {
    let manager = rag_manager.read().await;
    // Use the backwards-compatible method that returns SvelteDoc
    manager
        .search_as_docs(&query, limit.unwrap_or(3))
        .await
        .map_err(|e| format!("RAG search failed: {}", e))
}

// ============================================================================
// Model Configuration Commands
// ============================================================================

#[command]
pub async fn get_model_config(
    config: State<'_, SharedAppConfig>,
) -> Result<ModelConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.models.clone())
}

#[command]
pub async fn set_model_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    models: ModelConfig,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.models = models;
    config_guard.save(&app_data_dir).await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Model configuration saved");
    Ok(())
}

#[command]
pub async fn get_app_config(
    config: State<'_, SharedAppConfig>,
) -> Result<AppConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.clone())
}

#[command]
pub async fn set_app_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    new_config: AppConfig,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    *config_guard = new_config;
    config_guard.save(&app_data_dir).await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Application configuration saved");
    Ok(())
}

#[command]
pub async fn get_device_config(
    config: State<'_, SharedAppConfig>,
) -> Result<DeviceConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.device.clone())
}

#[command]
pub async fn set_device_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    device: DeviceConfig,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.device = device;
    config_guard.save(&app_data_dir).await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Device configuration saved");
    Ok(())
}

/// List available compute devices by running llama-server --list-devices
#[command]
pub async fn list_devices(app: AppHandle) -> Result<Vec<DeviceInfo>, String> {
    use tauri_plugin_shell::process::CommandEvent;

    log::info!("Listing available devices...");

    // Run llama-server with --list-devices flag
    // Use --device CUDA0 to trigger the CUDA binary which shows all device types
    let (mut rx, _child) = app
        .shell()
        .sidecar("llama-server-wrapper")
        .map_err(|e| format!("Failed to create sidecar: {}", e))?
        .args(["--device", "CUDA0", "--list-devices"])
        .spawn()
        .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

    // Collect output
    let mut output = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => {
                if let Ok(text) = String::from_utf8(line) {
                    output.push_str(&text);
                }
            }
            CommandEvent::Stderr(line) => {
                if let Ok(text) = String::from_utf8(line) {
                    output.push_str(&text);
                }
            }
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }

    log::info!("Device list output: {}", output);

    // Parse the output
    // Format: "  Vulkan0: Intel(R) Graphics (RPL-P) (32003 MiB, 28803 MiB free)"
    let mut devices = Vec::new();

    // Always add CPU option first
    devices.push(DeviceInfo {
        id: "none".to_string(),
        name: "CPU Only".to_string(),
        total_vram_mb: 0,
        free_vram_mb: 0,
    });

    for line in output.lines() {
        let line = line.trim();
        // Look for lines like "Vulkan0: ..." or "CUDA0: ..."
        if let Some(colon_pos) = line.find(':') {
            let id = line[..colon_pos].trim();
            // Skip if it doesn't look like a device ID (e.g., "Available devices")
            if !id.contains(' ') && (id.starts_with("Vulkan") || id.starts_with("CUDA") || id.starts_with("Metal")) {
                let rest = line[colon_pos + 1..].trim();

                // Parse name and VRAM info
                // Format: "NVIDIA GeForce RTX 4060 Laptop GPU (8188 MiB, 547 MiB free)"
                let (name, total_vram, free_vram) = if let Some(paren_pos) = rest.rfind('(') {
                    let name = rest[..paren_pos].trim();
                    let vram_info = &rest[paren_pos + 1..].trim_end_matches(')');

                    // Parse "8188 MiB, 547 MiB free"
                    let parts: Vec<&str> = vram_info.split(',').collect();
                    let total = parts.get(0)
                        .and_then(|s| s.trim().strip_suffix(" MiB"))
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                    let free = parts.get(1)
                        .and_then(|s| s.trim().strip_suffix(" MiB free"))
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);

                    (name.to_string(), total, free)
                } else {
                    (rest.to_string(), 0, 0)
                };

                devices.push(DeviceInfo {
                    id: id.to_string(),
                    name,
                    total_vram_mb: total_vram,
                    free_vram_mb: free_vram,
                });
            }
        }
    }

    log::info!("Found {} devices", devices.len());
    Ok(devices)
}

// ============================================================================
// Server Mode Commands
// ============================================================================

#[command]
pub async fn get_server_mode(
    gateway: State<'_, SharedGateway>,
) -> Result<ServerModeInfo, String> {
    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_inference(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
) -> Result<ServerModeInfo, String> {
    let backend_name = gateway.current_backend_name().await;
    log::info!("Starting sidecar inference with backend: {}", backend_name);

    let config_guard = config.read().await;

    // Extract config values we'll need after dropping the guard
    let embedding_model_path = config_guard.models.embedding_model_path.clone();
    let embedding_memory_mode = config_guard.embedding_memory_mode.clone();

    // Build backend-specific config
    let backend_config = match backend_name.as_str() {
        "Ollama" => {
            // Ollama uses model names, not file paths
            let model_name = config_guard.models.ollama_vlm_model.as_ref()
                .ok_or_else(|| "Ollama VLM model not configured. Set a model like 'llava:13b' or 'qwen2-vl:7b' in Model Configuration.".to_string())?;
            BackendConfig {
                model_name: Some(model_name.clone()),
                embedding_mode: false,
                ..Default::default()
            }
        }
        _ => {
            // llama.cpp and others use file paths
            let model_path = config_guard.models.vlm_model_path.as_ref()
                .ok_or_else(|| "VLM model path not configured".to_string())?;
            let mmproj_path = config_guard.models.vlm_mmproj_path.as_ref()
                .ok_or_else(|| "VLM mmproj path not configured".to_string())?;

            BackendConfig {
                model_path: Some(std::path::PathBuf::from(model_path)),
                mmproj_path: Some(std::path::PathBuf::from(mmproj_path)),
                device: Some(config_guard.device.device.clone()),
                gpu_layers: Some(config_guard.device.gpu_layers),
                embedding_mode: false,
                ..Default::default()
            }
        }
    };
    drop(config_guard);

    // Start the main LLM server
    gateway
        .start(&backend_config, &app)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in inference mode");

    // Start embedding server for parallel modes (if embedding model is configured)
    if let Some(ref emb_path) = embedding_model_path {
        if embedding_memory_mode != EmbeddingMemoryMode::Sequential {
            // Get device info for VRAM checking
            let devices = list_devices(app.clone()).await.unwrap_or_default();

            match gateway.start_embedding_server(emb_path, embedding_memory_mode.clone(), &devices, &app).await {
                Ok(()) => {
                    // Set embedding URL in RAG manager so search() will work
                    if let Some(url) = gateway.embedding_url().await {
                        let mut rag = rag_manager.write().await;
                        rag.set_embedding_url(url);
                        log::info!("Embedding server started and RAG manager configured");
                    }
                }
                Err(e) => {
                    // Log but don't fail - embedding server is optional
                    log::warn!("Failed to start embedding server: {}. Vector search may not work.", e);
                }
            }
        } else {
            log::info!("Sequential embedding mode: embedding server will start on-demand");
        }
    }

    Ok(gateway.mode_info().await)
}

#[command]
pub async fn start_sidecar_embedding(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
) -> Result<ServerModeInfo, String> {
    let config_guard = config.read().await;

    let model_path = config_guard.models.embedding_model_path.as_ref()
        .ok_or_else(|| "Embedding model path not configured".to_string())?;

    let backend_config = BackendConfig {
        model_path: Some(std::path::PathBuf::from(model_path)),
        device: Some(config_guard.device.device.clone()),
        gpu_layers: Some(config_guard.device.gpu_layers),
        embedding_mode: true,
        ..Default::default()
    };
    drop(config_guard);

    gateway
        .start(&backend_config, &app)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Started sidecar in embedding mode");
    Ok(gateway.mode_info().await)
}

/// Index documents with automatic mode switching
/// If in inference mode, switches to embedding mode, indexes, then switches back
#[command]
pub async fn index_docs_with_switch(
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    config: State<'_, SharedAppConfig>,
    rag_manager: State<'_, SharedRagManager>,
    channel: Channel<IndexingEvent>,
) -> Result<(), String> {
    log::info!("========== INDEX_DOCS_WITH_SWITCH CALLED ==========");
    let config_guard = config.read().await;

    // Check we have embedding model configured
    let embedding_model_path = config_guard.models.embedding_model_path.as_ref()
        .ok_or_else(|| "Embedding model path not configured".to_string())?
        .clone();
    log::info!("Embedding model path: {:?}", embedding_model_path);

    // Check if we need to restore VLM mode after
    let restore_vlm = gateway.is_inference_mode().await;
    log::info!("Restore VLM after indexing: {}", restore_vlm);

    // Save the last inference config for potential restoration
    let last_inference_config = gateway.last_inference_config().await;

    let device = config_guard.device.clone();
    drop(config_guard);

    // Send progress: switching to embedding mode
    channel.send(IndexingEvent {
        current: 0,
        total: 0,
        status: "Switching to embedding mode...".to_string(),
        done: false,
        error: None,
    }).ok();

    // Build embedding config based on which backend is active
    let backend_name = gateway.current_backend_name().await;
    log::info!("Current backend for embedding: {}", backend_name);

    let embedding_config = match backend_name.as_str() {
        "Ollama" => {
            // Ollama uses model names, not file paths
            // Default to nomic-embed-text for embeddings
            BackendConfig {
                model_name: Some("nomic-embed-text".to_string()),
                embedding_mode: true,
                ..Default::default()
            }
        }
        "Candle" => {
            // Candle uses local SafeTensors model directories (not GGUF files)
            // Get the path from config (user must download model manually from HuggingFace)
            let config_guard = config.read().await;
            let candle_path = config_guard.models.candle_embedding_model_path.clone()
                .ok_or_else(|| {
                    "Candle embedding model path not configured. \
                     Download a SafeTensors model from HuggingFace (e.g., BAAI/bge-small-en-v1.5) \
                     and set the path in Settings.".to_string()
                })?;
            drop(config_guard);

            BackendConfig {
                model_path: Some(std::path::PathBuf::from(&candle_path)),
                embedding_mode: true,
                ..Default::default()
            }
        }
        _ => {
            // llama.cpp and others use file paths (GGUF format)
            BackendConfig {
                model_path: Some(std::path::PathBuf::from(&embedding_model_path)),
                device: Some(device.device.clone()),
                gpu_layers: Some(device.gpu_layers),
                embedding_mode: true,
                ..Default::default()
            }
        }
    };

    gateway
        .start(&embedding_config, &app)
        .await
        .map_err(|e| format!("Failed to start embedding server: {}", e))?;

    // Update RAG manager with embedding URL from the gateway
    // All backends now expose an HTTP API (llama.cpp sidecar, Ollama daemon, Candle's Axum server)
    let embedding_url = match gateway.base_url().await {
        Some(url) => url,
        None => {
            // Backend has no HTTP API (e.g., Candle)
            // Restore VLM mode if needed and return error
            if restore_vlm {
                if let Some(inference_config) = last_inference_config.clone() {
                    let _ = gateway.start(&inference_config, &app).await;
                }
            }
            return Err(format!(
                "The {} backend does not support RAG indexing through the GUI. \
                 It runs in-process without an HTTP API. \
                 Please use llama.cpp or Ollama for RAG/embedding functionality.",
                backend_name
            ));
        }
    };
    log::info!("Embedding URL set: {:?}", embedding_url);
    {
        let mut rag_guard = rag_manager.write().await;
        rag_guard.set_embedding_url(embedding_url);
    }

    // Load docs and index
    let project_data_dir = get_project_data_dir()?;
    let docs_manager = DocsManager::new(project_data_dir);

    docs_manager.ensure_docs_available().await
        .map_err(|e| format!("Failed to ensure docs available: {}", e))?;

    let index = docs_manager.load_index()
        .map_err(|e| format!("Failed to load search index: {}", e))?;

    let version = docs_manager.get_status().version.unwrap_or_else(|| "unknown".to_string());
    log::info!("Loaded {} documents from search index (version: {})", index.entries.len(), version);

    // Create progress callback
    let channel_clone = channel.clone();
    let on_progress = move |progress: IndexingProgress| {
        channel_clone.send(IndexingEvent::from(progress)).ok();
    };

    // Index documents
    log::info!("Starting index_documents() with {} docs", index.entries.len());
    let index_result = {
        let mut manager = rag_manager.write().await;
        manager.index_documents(&index.entries, &version, on_progress).await
    };

    match index_result {
        Ok(()) => {
            channel.send(IndexingEvent {
                current: index.entries.len(),
                total: index.entries.len(),
                status: "Indexing complete".to_string(),
                done: false,
                error: None,
            }).ok();
        }
        Err(e) => {
            log::error!("Failed to index documents: {:?}", e);
            channel.send(IndexingEvent {
                current: 0,
                total: 0,
                status: "Indexing failed".to_string(),
                done: true,
                error: Some(e.to_string()),
            }).ok();

            // Try to restore VLM mode even on error
            if restore_vlm {
                if let Some(inference_config) = last_inference_config.clone() {
                    let _ = gateway.start(&inference_config, &app).await;
                }
            }

            return Err(e.to_string());
        }
    }

    // Restore VLM mode if we were in it before
    if restore_vlm {
        channel.send(IndexingEvent {
            current: index.entries.len(),
            total: index.entries.len(),
            status: "Switching back to VLM mode...".to_string(),
            done: false,
            error: None,
        }).ok();

        if let Some(inference_config) = last_inference_config {
            gateway
                .start(&inference_config, &app)
                .await
                .map_err(|e| format!("Failed to restore VLM mode: {}", e))?;
        }
    }

    channel.send(IndexingEvent {
        current: index.entries.len(),
        total: index.entries.len(),
        status: "Complete".to_string(),
        done: true,
        error: None,
    }).ok();

    Ok(())
}

// ============================================================================
// Backend Commands
// ============================================================================

use super::backend::BackendInfo;

/// List all available inference backends
#[command]
pub async fn list_backends(gateway: State<'_, SharedGateway>) -> Result<Vec<BackendInfo>, String> {
    let mut backends = gateway.available_backends();
    let current_name = gateway.current_backend_name().await;

    // Mark the active backend
    for backend in &mut backends {
        backend.active = backend.name == current_name;
    }

    Ok(backends)
}

/// Get the currently active backend name
#[command]
pub async fn get_current_backend(gateway: State<'_, SharedGateway>) -> Result<String, String> {
    Ok(gateway.current_backend_name().await)
}

/// Switch to a different inference backend
///
/// Note: This stops the current backend. You'll need to call start_sidecar_inference
/// or start_sidecar_embedding to start the new backend.
#[command]
pub async fn switch_backend(
    gateway: State<'_, SharedGateway>,
    backend_name: String,
) -> Result<(), String> {
    gateway
        .switch_backend(&backend_name)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Switched to backend: {}", backend_name);
    Ok(())
}

/// Get capabilities of the current backend
#[command]
pub async fn get_backend_capabilities(
    gateway: State<'_, SharedGateway>,
) -> Result<super::backend::BackendCapabilities, String> {
    Ok(gateway.capabilities().await)
}

// ============================================================================
// Binary Download Commands
// ============================================================================

/// Get the binaries directory path
fn get_binaries_dir(_app: &AppHandle) -> Result<PathBuf, String> {
    // In dev mode, binaries are in src-tauri/binaries
    // build.rs copies them to target/debug for the sidecar system
    // We check src-tauri/binaries as that's where downloads should go

    // Try to find src-tauri/binaries relative to current exe or working directory
    let candidates = [
        // Dev mode: current working directory is src-tauri
        std::env::current_dir().ok().map(|p| p.join("binaries")),
        // Dev mode: exe is in target/debug, binaries in src-tauri/binaries
        std::env::current_exe().ok().and_then(|p| {
            p.parent() // target/debug
                .and_then(|p| p.parent()) // target
                .and_then(|p| p.parent()) // src-tauri
                .map(|p| p.join("binaries"))
        }),
        // Production: binaries next to exe
        std::env::current_exe().ok().and_then(|p| {
            p.parent().map(|p| p.join("binaries"))
        }),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            log::debug!("Found binaries dir at: {:?}", candidate);
            return Ok(candidate);
        }
    }

    // Fallback: create in current directory
    let fallback = std::env::current_dir()
        .map_err(|e| format!("Failed to get current dir: {}", e))?
        .join("binaries");
    log::warn!("Binaries dir not found, using fallback: {:?}", fallback);
    Ok(fallback)
}

/// Get the directory for downloading binaries (uses app data dir to avoid triggering recompilation)
fn get_download_binaries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;

    // Use app data directory for downloads - this is outside the source tree
    // and won't trigger Tauri's file watcher during dev mode
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let binaries_dir = app_data_dir.join("binaries");

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    log::debug!("Download binaries dir: {:?}", binaries_dir);
    Ok(binaries_dir)
}

/// Required files for llama.cpp backend
const REQUIRED_BINARIES: &[&str] = &[
    "llama-server-x86_64-unknown-linux-gnu",
    "libllama.so",
    "libggml.so",
];

/// Check if llama.cpp binaries are available
#[command]
pub async fn check_llama_binaries(app: AppHandle) -> Result<BinaryStatus, String> {
    let binaries_dir = get_binaries_dir(&app)?;

    let mut missing = Vec::new();

    for file in REQUIRED_BINARIES {
        let path = binaries_dir.join(file);
        if !path.exists() {
            missing.push(file.to_string());
        }
    }

    // Also check for the wrapper script
    let wrapper = binaries_dir.join("llama-server-wrapper-x86_64-unknown-linux-gnu");
    if !wrapper.exists() {
        missing.push("llama-server-wrapper-x86_64-unknown-linux-gnu".to_string());
    }

    Ok(BinaryStatus {
        available: missing.is_empty(),
        missing_files: missing,
    })
}

/// Download llama.cpp binaries from GitHub releases
#[command]
pub async fn download_llama_binaries(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    use futures_util::TryStreamExt;

    let binaries_dir = get_binaries_dir(&app)?;

    // Ensure binaries directory exists
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    // llama.cpp release info
    let release_tag = "b4967";
    let archive_name = format!("llama-{}-bin-ubuntu-x64.zip", release_tag);
    let download_url = format!(
        "https://github.com/ggerganov/llama.cpp/releases/download/{}/{}",
        release_tag, archive_name
    );

    channel.send(DownloadProgress {
        status: "Downloading llama.cpp binaries...".to_string(),
        current: 0,
        total: 0,
        done: false,
        error: None,
    }).ok();

    log::info!("Downloading llama.cpp from: {}", download_url);

    // Download the archive
    let client = Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }

    let total_size = response.content_length().unwrap_or(0);
    let temp_path = binaries_dir.join(&archive_name);

    // Download with progress
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.try_next().await.map_err(|e| format!("Download error: {}", e))? {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        channel.send(DownloadProgress {
            status: "Downloading...".to_string(),
            current: downloaded,
            total: total_size,
            done: false,
            error: None,
        }).ok();
    }

    drop(file);

    channel.send(DownloadProgress {
        status: "Extracting...".to_string(),
        current: total_size,
        total: total_size,
        done: false,
        error: None,
    }).ok();

    // Extract the archive
    log::info!("Extracting archive to: {:?}", binaries_dir);

    let file = std::fs::File::open(&temp_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read archive entry: {}", e))?;

        let name = file.name().to_string();

        // We're interested in specific files from the archive
        // The archive structure is: build/bin/llama-server, build/lib/libllama.so, etc.
        let extract_name: Option<String> = if name.ends_with("llama-server") && !name.contains("llama-server-") {
            Some("llama-server-x86_64-unknown-linux-gnu".to_string())
        } else if name.ends_with("libllama.so") {
            Some("libllama.so".to_string())
        } else if name.ends_with("libggml.so") {
            Some("libggml.so".to_string())
        } else if name.ends_with("libggml-base.so") {
            Some("libggml-base.so".to_string())
        } else if name.ends_with("libggml-cpu.so") {
            // Skip CPU-specific variants, use the base one
            None
        } else if name.contains("libggml-") && name.ends_with(".so") {
            // Extract other ggml libraries (vulkan, cuda, etc.)
            std::path::Path::new(&name)
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
        } else {
            None
        };

        if let Some(ref dest_name) = extract_name {
            let dest_path = binaries_dir.join(dest_name);
            let mut dest_file = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create file {}: {}", dest_name, e))?;

            std::io::copy(&mut file, &mut dest_file)
                .map_err(|e| format!("Failed to extract {}: {}", dest_name, e))?;

            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = dest_file.metadata()
                    .map_err(|e| format!("Failed to get metadata: {}", e))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest_path, perms)
                    .map_err(|e| format!("Failed to set permissions: {}", e))?;
            }

            log::info!("Extracted: {} -> {:?}", name, dest_path);
        }
    }

    // Create the wrapper script
    let wrapper_path = binaries_dir.join("llama-server-wrapper-x86_64-unknown-linux-gnu");
    let wrapper_content = r#"#!/bin/bash
# Wrapper script for llama-server that sets up LD_LIBRARY_PATH

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LD_LIBRARY_PATH="${SCRIPT_DIR}:${LD_LIBRARY_PATH}"

# Check for CUDA device request
for arg in "$@"; do
    if [[ "$arg" == CUDA* ]]; then
        if [[ -d "${SCRIPT_DIR}/cuda" ]]; then
            export LD_LIBRARY_PATH="${SCRIPT_DIR}/cuda:${LD_LIBRARY_PATH}"
        fi
        break
    fi
done

exec "${SCRIPT_DIR}/llama-server-x86_64-unknown-linux-gnu" "$@"
"#;

    std::fs::write(&wrapper_path, wrapper_content)
        .map_err(|e| format!("Failed to write wrapper script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&wrapper_path)
            .map_err(|e| format!("Failed to get wrapper metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&wrapper_path, perms)
            .map_err(|e| format!("Failed to set wrapper permissions: {}", e))?;
    }

    // Clean up the archive
    let _ = std::fs::remove_file(&temp_path);

    channel.send(DownloadProgress {
        status: "Complete".to_string(),
        current: total_size,
        total: total_size,
        done: true,
        error: None,
    }).ok();

    log::info!("llama.cpp binaries downloaded and extracted successfully");
    Ok(())
}

/// Check if Ollama binary is available in our managed location
#[command]
pub async fn check_ollama_binary(app: AppHandle) -> Result<BinaryStatus, String> {
    // First check system PATH (already installed by user)
    if which::which("ollama").is_ok() {
        return Ok(BinaryStatus {
            available: true,
            missing_files: vec![],
        });
    }

    // Check our managed binaries directory
    let binaries_dir = get_binaries_dir(&app)?;
    let ollama_path = binaries_dir.join("ollama");

    if ollama_path.exists() {
        Ok(BinaryStatus {
            available: true,
            missing_files: vec![],
        })
    } else {
        Ok(BinaryStatus {
            available: false,
            missing_files: vec!["ollama".to_string()],
        })
    }
}

/// Download Ollama binary from GitHub releases
#[command]
pub async fn download_ollama_binary(
    app: AppHandle,
    channel: Channel<DownloadProgress>,
) -> Result<(), String> {
    use futures_util::TryStreamExt;

    let binaries_dir = get_download_binaries_dir(&app)?;

    // Ensure binaries directory exists
    std::fs::create_dir_all(&binaries_dir)
        .map_err(|e| format!("Failed to create binaries directory: {}", e))?;

    // Ollama release info - uses tar.zst format
    let release_tag = "v0.14.1";
    let archive_name = "ollama-linux-amd64.tar.zst";
    let download_url = format!(
        "https://github.com/ollama/ollama/releases/download/{}/{}",
        release_tag, archive_name
    );

    channel
        .send(DownloadProgress {
            status: "Downloading Ollama...".to_string(),
            current: 0,
            total: 0,
            done: false,
            error: None,
        })
        .ok();

    log::info!("Downloading Ollama from: {}", download_url);

    // Download the archive
    let client = Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to start download: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    let temp_path = binaries_dir.join(archive_name);

    // Download with progress
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut downloaded: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream
        .try_next()
        .await
        .map_err(|e| format!("Download error: {}", e))?
    {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write chunk: {}", e))?;

        downloaded += chunk.len() as u64;

        channel
            .send(DownloadProgress {
                status: "Downloading...".to_string(),
                current: downloaded,
                total: total_size,
                done: false,
                error: None,
            })
            .ok();
    }

    drop(file);

    channel
        .send(DownloadProgress {
            status: "Extracting...".to_string(),
            current: total_size,
            total: total_size,
            done: false,
            error: None,
        })
        .ok();

    // Extract the tar.zst archive
    log::info!("Extracting Ollama archive to: {:?}", binaries_dir);

    let file = std::fs::File::open(&temp_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;

    // Decompress zstd
    let decoder = zstd::Decoder::new(file)
        .map_err(|e| format!("Failed to create zstd decoder: {}", e))?;

    // Extract tar
    let mut archive = tar::Archive::new(decoder);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read tar entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read tar entry: {}", e))?;

        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?;
        let path_str = path.to_string_lossy();

        // We're looking for the main ollama binary
        // The archive structure is typically: bin/ollama
        if path_str.ends_with("/ollama") || path_str == "ollama" {
            let dest_path = binaries_dir.join("ollama");

            // Extract to destination
            let mut dest_file = std::fs::File::create(&dest_path)
                .map_err(|e| format!("Failed to create ollama binary: {}", e))?;

            std::io::copy(&mut entry, &mut dest_file)
                .map_err(|e| format!("Failed to extract ollama: {}", e))?;

            // Make executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = dest_file
                    .metadata()
                    .map_err(|e| format!("Failed to get metadata: {}", e))?
                    .permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&dest_path, perms)
                    .map_err(|e| format!("Failed to set permissions: {}", e))?;
            }

            log::info!("Extracted ollama binary to: {:?}", dest_path);
        }
    }

    // Clean up the archive
    let _ = std::fs::remove_file(&temp_path);

    // Verify extraction
    let ollama_path = binaries_dir.join("ollama");
    if !ollama_path.exists() {
        return Err("Failed to extract ollama binary from archive".to_string());
    }

    channel
        .send(DownloadProgress {
            status: "Complete".to_string(),
            current: total_size,
            total: total_size,
            done: true,
            error: None,
        })
        .ok();

    log::info!("Ollama binary downloaded and extracted successfully");
    Ok(())
}

// ============================================================================
// Document Chunking Commands
// ============================================================================

use crate::agent::types::{ChunkPreview, DocInfo};
use crate::agent::{preview_chunks, ChunkConfig};
use crate::agent::docs_index::SearchIndex;

/// List all documents available for chunking
#[command]
pub async fn list_chunkable_docs(app: AppHandle) -> Result<Vec<DocInfo>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let docs_dir = app_data_dir.join("svelte-docs");

    if !docs_dir.exists() {
        return Ok(Vec::new());
    }

    // Build index to get document info
    let index = SearchIndex::build_from_docs(&docs_dir)
        .map_err(|e| format!("Failed to build index: {}", e))?;

    let docs: Vec<DocInfo> = index
        .entries
        .iter()
        .map(|entry| DocInfo {
            id: entry.id.clone(),
            title: entry.title.clone(),
            section: entry.section.clone(),
            char_count: entry.content.len(),
        })
        .collect();

    Ok(docs)
}

/// Preview how a document would be chunked
#[command]
pub async fn preview_doc_chunks(
    app: AppHandle,
    doc_id: String,
) -> Result<ChunkPreview, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let docs_dir = app_data_dir.join("svelte-docs");

    if !docs_dir.exists() {
        return Err("Documentation not downloaded yet".to_string());
    }

    // Build index to find the document
    let index = SearchIndex::build_from_docs(&docs_dir)
        .map_err(|e| format!("Failed to build index: {}", e))?;

    // Find the document by ID
    let entry = index
        .entries
        .iter()
        .find(|e| e.id == doc_id)
        .ok_or_else(|| format!("Document not found: {}", doc_id))?;

    // Generate chunk preview
    let config = ChunkConfig::default();
    let preview = preview_chunks(
        &entry.id,
        &entry.title,
        &entry.section,
        &entry.content,
        &config,
    );

    Ok(preview)
}

// ============================================================================
// Embedding Memory Mode Commands
// ============================================================================

/// Get the current embedding memory mode
#[command]
pub async fn get_embedding_memory_mode(
    config: State<'_, SharedAppConfig>,
) -> Result<String, String> {
    let config_guard = config.read().await;
    let mode = match config_guard.embedding_memory_mode {
        EmbeddingMemoryMode::CpuParallel => "cpu_parallel",
        EmbeddingMemoryMode::GpuParallel => "gpu_parallel",
        EmbeddingMemoryMode::Sequential => "sequential",
    };
    Ok(mode.to_string())
}

/// Set the embedding memory mode
/// Note: This saves the config but doesn't restart the embedding server.
/// Call start_sidecar_inference to apply the new mode.
#[command]
pub async fn set_embedding_memory_mode(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    mode: String,
) -> Result<(), String> {
    let new_mode = match mode.as_str() {
        "cpu_parallel" => EmbeddingMemoryMode::CpuParallel,
        "gpu_parallel" => EmbeddingMemoryMode::GpuParallel,
        "sequential" => EmbeddingMemoryMode::Sequential,
        _ => return Err(format!("Invalid embedding memory mode: {}", mode)),
    };

    {
        let mut config_guard = config.write().await;
        config_guard.embedding_memory_mode = new_mode;
    }

    // Save config to disk
    let config_guard = config.read().await;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    config_guard
        .save(&app_data_dir)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Set embedding memory mode to: {}", mode);
    Ok(())
}

/// Check if the embedding server is ready
#[command]
pub async fn is_embedding_server_ready(
    gateway: State<'_, SharedGateway>,
) -> Result<bool, String> {
    Ok(gateway.is_embedding_server_ready().await)
}

/// Get the embedding server URL if available
#[command]
pub async fn get_embedding_server_url(
    gateway: State<'_, SharedGateway>,
) -> Result<Option<String>, String> {
    Ok(gateway.embedding_url().await)
}

// ============================================================================
// Sandbox Configuration Commands
// ============================================================================

/// Get the current sandbox configuration
#[command]
pub async fn get_sandbox_config(
    config: State<'_, SharedAppConfig>,
) -> Result<SandboxConfig, String> {
    let config_guard = config.read().await;
    Ok(config_guard.sandbox.clone())
}

/// Set the sandbox configuration
#[command]
pub async fn set_sandbox_config(
    app: AppHandle,
    config: State<'_, SharedAppConfig>,
    sandbox: SandboxConfig,
) -> Result<(), String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let mut config_guard = config.write().await;
    config_guard.sandbox = sandbox;
    config_guard.save(&app_data_dir).await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    log::info!("Sandbox configuration saved");
    Ok(())
}
