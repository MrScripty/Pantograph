//! Core task executor with built-in node handlers.
//!
//! `CoreTaskExecutor` handles all node types whose logic is not host-specific.
//! Hosts (Tauri, NIF/Elixir) only need to handle nodes that require platform
//! resources (e.g. RAG manager, UI interaction).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

#[cfg(feature = "inference-nodes")]
use inference::InferenceGateway;

use crate::engine::TaskExecutor;
use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
#[cfg(any(
    feature = "inference-nodes",
    feature = "pytorch-nodes",
    feature = "audio-nodes"
))]
use crate::model_dependencies::ModelRefV2;

mod dependency_preflight;
mod file_io;
#[cfg(feature = "inference-nodes")]
mod kv_cache;
mod ollama;
mod pure_nodes;
mod settings;
pub(crate) use dependency_preflight::*;
pub(crate) use file_io::*;
pub(crate) use ollama::*;
pub(crate) use pure_nodes::*;
pub(crate) use settings::*;

/// Extract the node type from task inputs or infer from the task ID.
///
/// Checks `_data.node_type` first (injected by the graph converter),
/// then falls back to stripping the trailing `-N` suffix from the task ID.
pub fn resolve_node_type(task_id: &str, inputs: &HashMap<String, serde_json::Value>) -> String {
    inputs
        .get("_data")
        .and_then(|d| d.get("node_type"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            let parts: Vec<&str> = task_id.rsplitn(2, '-').collect();
            if parts.len() == 2 {
                parts[1].to_string()
            } else {
                task_id.to_string()
            }
        })
}

/// Core task executor that handles all host-independent node types.
///
/// For nodes requiring host-specific resources, wrap this in a
/// `CompositeTaskExecutor` with a host-specific fallback.
pub struct CoreTaskExecutor {
    /// Optional project root for file I/O nodes (read-file, write-file).
    project_root: Option<PathBuf>,
    /// Inference gateway for LLM nodes (llamacpp, llm-inference, vision, unload-model).
    #[cfg(feature = "inference-nodes")]
    gateway: Option<Arc<InferenceGateway>>,
    /// Optional event sink for streaming tokens during inference.
    event_sink: Option<Arc<dyn EventSink>>,
    /// Execution ID for event correlation.
    execution_id: Option<String>,
}

impl CoreTaskExecutor {
    /// Create a new core executor.
    pub fn new() -> Self {
        Self {
            project_root: None,
            #[cfg(feature = "inference-nodes")]
            gateway: None,
            event_sink: None,
            execution_id: None,
        }
    }

    /// Set the project root directory for file I/O nodes.
    pub fn with_project_root(mut self, root: PathBuf) -> Self {
        self.project_root = Some(root);
        self
    }

    /// Set the inference gateway for LLM nodes.
    #[cfg(feature = "inference-nodes")]
    pub fn with_gateway(mut self, gateway: Arc<InferenceGateway>) -> Self {
        self.gateway = Some(gateway);
        self
    }

    /// Set the event sink for streaming tokens during inference.
    pub fn with_event_sink(mut self, sink: Arc<dyn EventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    /// Set the execution ID for event correlation.
    pub fn with_execution_id(mut self, id: String) -> Self {
        self.execution_id = Some(id);
        self
    }
}

impl Default for CoreTaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Pure node handlers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// File I/O handlers (async, use project_root)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Expand settings and shared input readers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Dependency preflight helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Ollama HTTP inference handler
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// TaskExecutor implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl TaskExecutor for CoreTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &graph_flow::Context,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);
        let _ = extensions;

        log::debug!(
            "CoreTaskExecutor: executing '{}' (type '{}')",
            task_id,
            node_type
        );

        match node_type.as_str() {
            // Input nodes
            "text-input" => execute_text_input(&inputs),
            "number-input" => execute_number_input(&inputs),
            "boolean-input" => execute_boolean_input(&inputs),
            "selection-input" => execute_selection_input(&inputs),
            "vector-input" => execute_vector_input(&inputs),
            "masked-text-input" => execute_masked_text_input(&inputs),
            "linked-input" => execute_linked_input(&inputs),
            "image-input" => execute_image_input(&inputs),
            "audio-input" => execute_audio_input(&inputs),

            // Output nodes
            "text-output" => execute_text_output(&inputs),
            "vector-output" => execute_vector_output(&inputs),
            "image-output" => execute_image_output(&inputs),
            "audio-output" => execute_audio_output(&inputs),
            "point-cloud-output" => execute_point_cloud_output(&inputs),
            "component-preview" => execute_component_preview(&inputs),

            // Model/provider nodes
            "model-provider" => execute_model_provider(&inputs),
            "puma-lib" => execute_puma_lib(&inputs),

            // Control flow nodes
            "conditional" => execute_conditional(&inputs),
            "merge" => execute_merge(&inputs),

            // Processing nodes
            "validator" => execute_validator(&inputs),
            "json-filter" => execute_json_filter(&inputs),
            "expand-settings" => execute_expand_settings(&inputs),

            // File I/O nodes
            "read-file" => execute_read_file(self.project_root.as_ref(), &inputs).await,
            "write-file" => execute_write_file(self.project_root.as_ref(), &inputs).await,

            // Interaction nodes
            "human-input" => execute_human_input(&inputs),
            "tool-executor" => execute_tool_executor(&inputs),

            // Pure HTTP inference
            "ollama-inference" => execute_ollama_inference(&inputs).await,

            // Gateway-backed inference nodes (require `inference-nodes` feature)
            #[cfg(feature = "inference-nodes")]
            "embedding" => execute_embedding(self.gateway.as_ref(), &inputs).await,
            #[cfg(feature = "inference-nodes")]
            "llamacpp-inference" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("llamacpp-inference", &inputs, extensions).await?;
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_llamacpp_inference(
                    self.gateway.as_ref(),
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                    resolved_model_ref,
                    extensions,
                )
                .await
            }
            #[cfg(feature = "inference-nodes")]
            "reranker" => execute_reranker(self.gateway.as_ref(), &inputs).await,
            #[cfg(feature = "inference-nodes")]
            "llm-inference" => {
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_llm_inference(
                    self.gateway.as_ref(),
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                )
                .await
            }
            #[cfg(feature = "inference-nodes")]
            "vision-analysis" => execute_vision_analysis(self.gateway.as_ref(), &inputs).await,
            #[cfg(feature = "inference-nodes")]
            "unload-model" => execute_unload_model(self.gateway.as_ref(), &inputs).await,

            // KV cache operations (require inference-nodes feature)
            #[cfg(feature = "inference-nodes")]
            "kv-cache-save" => kv_cache::execute_save(&inputs, extensions).await,
            #[cfg(feature = "inference-nodes")]
            "kv-cache-load" => {
                kv_cache::execute_load(&inputs, extensions, self.gateway.as_ref()).await
            }
            #[cfg(feature = "inference-nodes")]
            "kv-cache-truncate" => {
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                kv_cache::execute_truncate(
                    &inputs,
                    extensions,
                    self.gateway.as_ref(),
                    task_id,
                    exec_id,
                    self.event_sink.as_ref(),
                )
                .await
            }

            // PyTorch inference (in-process via PyO3)
            #[cfg(feature = "pytorch-nodes")]
            "pytorch-inference" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("pytorch-inference", &inputs, extensions).await?;
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                execute_pytorch_inference(
                    &inputs,
                    task_id,
                    self.event_sink.as_ref(),
                    exec_id,
                    resolved_model_ref,
                    extensions,
                )
                .await
            }

            // Audio generation (in-process via PyO3 + Stable Audio)
            #[cfg(feature = "audio-nodes")]
            "audio-generation" => {
                let resolved_model_ref =
                    enforce_dependency_preflight("audio-generation", &inputs, extensions).await?;
                execute_audio_generation(&inputs, resolved_model_ref).await
            }

            // Unknown — signal that this node requires a host-specific executor
            _ => Err(NodeEngineError::ExecutionFailed(format!(
                "Node type '{}' requires host-specific executor",
                node_type
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Gateway-backed inference handlers (behind feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "inference-nodes")]
fn require_gateway(gateway: Option<&Arc<InferenceGateway>>) -> Result<&Arc<InferenceGateway>> {
    gateway.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "InferenceGateway not configured: requires host-specific executor".to_string(),
        )
    })
}

/// Resolve a model path that may be a directory to the actual `.gguf` file inside.
///
/// pumas-library stores directory paths; llama.cpp needs the `.gguf` file.
#[cfg(feature = "inference-nodes")]
fn resolve_gguf_path(path: &str) -> Result<String> {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let gguf = std::fs::read_dir(p)
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Cannot read model directory '{}': {}",
                    path, e
                ))
            })?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
            })
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(format!(
                    "No .gguf file found in model directory '{}'",
                    path
                ))
            })?;
        Ok(gguf.path().to_string_lossy().into_owned())
    } else {
        Ok(path.to_string())
    }
}

#[cfg(feature = "inference-nodes")]
fn parse_reranker_documents(value: &serde_json::Value) -> Result<Vec<String>> {
    let items = if let Some(items) = value.as_array() {
        items
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input must be a JSON array".to_string(),
        ));
    };

    let mut documents = Vec::with_capacity(items.len());
    for item in items {
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        if let Some(text) = item
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("content").and_then(|v| v.as_str()))
            .or_else(|| item.get("document").and_then(|v| v.as_str()))
        {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents must be strings or objects with text/content/document fields"
                .to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input cannot be empty".to_string(),
        ));
    }

    Ok(documents)
}

#[cfg(feature = "inference-nodes")]
fn parse_reranker_documents_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>> {
    if let Some(value) = inputs.get("documents") {
        return parse_reranker_documents(value);
    }

    if let Some(raw) = inputs
        .get("documents_json")
        .and_then(|value| value.as_str())
    {
        let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Reranker documents_json must be valid JSON: {}",
                e
            ))
        })?;
        return parse_reranker_documents(&parsed);
    }

    Err(NodeEngineError::ExecutionFailed(
        "Missing documents input".to_string(),
    ))
}

#[cfg(feature = "inference-nodes")]
async fn execute_llamacpp_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;

    let model_path = resolve_gguf_path(model_path_raw)?;
    let system_prompt = inputs.get("system_prompt").and_then(|s| s.as_str());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);

    // Read model-specific inference settings
    let extra_settings = build_extra_settings(inputs);

    // Ensure gateway is ready — start if needed
    if !gw.is_ready().await {
        let mut config = inference::BackendConfig {
            model_path: Some(PathBuf::from(&model_path)),
            device: Some("auto".to_string()),
            gpu_layers: Some(-1),
            embedding_mode: false,
            ..Default::default()
        };

        // Apply model-specific settings to backend config
        if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
            config.gpu_layers = Some(v as i32);
        }
        if let Some(v) = extra_settings
            .get("context_length")
            .and_then(|v| v.as_i64())
        {
            config.context_size = Some(v as u32);
        }

        log::info!(
            "LlamaCppInference: starting server with model '{}'",
            model_path
        );
        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start llama.cpp server: {}", e))
        })?;

        // Wait for readiness with timeout
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for llama.cpp server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        log::info!("LlamaCppInference: server is ready");
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "llama.cpp server started but no URL available".to_string(),
        )
    })?;

    let full_prompt = if let Some(sys) = system_prompt {
        format!("{}\n\n{}", sys, prompt)
    } else {
        prompt.to_string()
    };

    let restored_kv_slot = kv_cache::restore_llamacpp_input_handle(
        inputs,
        gw,
        extensions,
        task_id,
        execution_id,
        event_sink,
    )
    .await?;
    let streaming = event_sink.is_some();
    let mut request_body = serde_json::json!({
        "prompt": full_prompt,
        "n_predict": max_tokens,
        "temperature": temperature,
        "stop": ["</s>", "<|im_end|>", "<|end|>"],
        "stream": streaming
    });
    if restored_kv_slot {
        request_body["id_slot"] = serde_json::json!(0);
        request_body["cache_prompt"] = serde_json::json!(true);
    }

    let client = reqwest::Client::new();
    let url = format!("{}/completion", base_url);

    log::debug!(
        "LlamaCppInference: sending request to {} (stream={})",
        url,
        streaming
    );

    let http_response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Failed to connect to llama.cpp server at {}: {}",
                url, e
            ))
        })?;

    if !http_response.status().is_success() {
        let status = http_response.status();
        let error_body = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "llama.cpp API error ({}): {}",
            status, error_body
        )));
    }

    let response_text = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_llamacpp_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        // Process any remaining data in buffer
        let line = buffer.trim().to_string();
        if let Some(token) = parse_llamacpp_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let response_json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse llama.cpp response: {}", e))
        })?;
        response_json["content"].as_str().unwrap_or("").to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    outputs.insert("model_path".to_string(), serde_json::json!(model_path));
    let task_type_primary = infer_task_type_primary("llamacpp-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "llamacpp",
        &model_path,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "llamacpp",
                "modelId": model_path,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    let kv_cache_output = match kv_cache::capture_llamacpp_output_handle(
        task_id,
        execution_id,
        gw,
        extensions,
        event_sink,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "LlamaCppInference: failed to capture KV cache output for '{}': {}",
                task_id,
                error
            );
            serde_json::Value::Null
        }
    };
    outputs.insert("kv_cache_out".to_string(), kv_cache_output);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_reranker(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let query = inputs
        .get("query")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;
    if query.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker query cannot be empty".to_string(),
        ));
    }

    let documents = parse_reranker_documents_input(inputs)?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;
    let model_path = resolve_gguf_path(model_path_raw)?;

    let top_k = inputs
        .get("top_k")
        .and_then(|value| value.as_u64().map(|v| v as usize))
        .or_else(|| {
            inputs
                .get("top_k")
                .and_then(|value| value.as_i64())
                .filter(|v| *v > 0)
                .map(|v| v as usize)
        });
    let return_documents =
        read_optional_input_bool_aliases(inputs, &["return_documents", "returnDocuments"])
            .unwrap_or(true);

    let mut extra_settings = build_extra_settings(inputs);
    let mut config = inference::BackendConfig {
        model_path: Some(PathBuf::from(&model_path)),
        device: Some("auto".to_string()),
        gpu_layers: Some(-1),
        reranking_mode: true,
        ..Default::default()
    };

    if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
        config.gpu_layers = Some(v as i32);
    }
    if let Some(v) = extra_settings
        .get("context_length")
        .and_then(|v| v.as_i64())
    {
        config.context_size = Some(v as u32);
    }
    extra_settings.remove("gpu_layers");
    extra_settings.remove("context_length");

    if !gw.is_ready().await || !gw.is_reranking_mode().await {
        if gw.is_ready().await {
            gw.stop().await;
        }

        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start reranking server: {}", e))
        })?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for reranking server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    let response = gw
        .rerank(inference::RerankRequest {
            model: model_path.clone(),
            query: query.to_string(),
            documents,
            top_n: top_k,
            return_documents,
            extra_options: serde_json::Value::Object(extra_settings.into_iter().collect()),
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Reranker request failed: {}", e)))?;

    let scores = response
        .results
        .iter()
        .map(|result| serde_json::json!(result.score))
        .collect::<Vec<_>>();
    let top_document = response
        .results
        .first()
        .and_then(|result| result.document.clone());
    let top_score = response.results.first().map(|result| result.score);

    let mut outputs = HashMap::new();
    outputs.insert(
        "results".to_string(),
        serde_json::to_value(&response.results).unwrap_or(serde_json::Value::Null),
    );
    outputs.insert("scores".to_string(), serde_json::json!(scores));
    outputs.insert(
        "model_path".to_string(),
        serde_json::json!(model_path.clone()),
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "contractVersion": 2,
            "engine": "llamacpp",
            "modelId": model_path,
            "modelPath": model_path,
            "taskTypePrimary": "reranking"
        }),
    );
    outputs.insert(
        "top_document".to_string(),
        top_document
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "top_score".to_string(),
        top_score
            .map(|value| serde_json::json!(value))
            .unwrap_or(serde_json::Value::Null),
    );
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_embedding(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let text = inputs
        .get("text")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing text input".to_string()))?;
    if text.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Embedding input text cannot be empty".to_string(),
        ));
    }

    let backend_name = gw.current_backend_name().await;
    if !is_llamacpp_backend_name(&backend_name) {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LlamaCpp Embedding blocked execution: active backend '{}' is not supported",
            backend_name
        )));
    }

    let model = read_optional_input_string_aliases(
        inputs,
        &["model", "model_name", "modelName", "model_id", "modelId"],
    )
    .filter(|s| !s.trim().is_empty())
    .unwrap_or_else(|| "default".to_string());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is not ready. Start llama.cpp in embedding mode (`--embeddings`) first".to_string(),
        ));
    }
    if !gw.is_embedding_mode().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is running in inference mode. Restart with `--embeddings`".to_string(),
        ));
    }
    let capabilities = gw.capabilities().await;
    if !capabilities.embeddings {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: active backend does not support embeddings"
                .to_string(),
        ));
    }

    let emit_metadata =
        read_optional_input_bool_aliases(inputs, &["emit_metadata", "emitMetadata"])
            .unwrap_or(false);

    let start = std::time::Instant::now();
    let results = gw
        .embeddings(vec![text.to_string()], &model)
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("LlamaCpp Embedding request failed: {}", e))
        })?;
    let embedding = results.first().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned no vectors for input text".to_string(),
        )
    })?;
    if embedding.vector.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned an empty vector".to_string(),
        ));
    }
    if embedding.vector.iter().any(|v| !v.is_finite()) {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned invalid vector values".to_string(),
        ));
    }

    let mut outputs = HashMap::new();
    outputs.insert("embedding".to_string(), serde_json::json!(embedding.vector));
    if emit_metadata {
        outputs.insert(
            "metadata".to_string(),
            serde_json::json!({
                "backend": "llamacpp",
                "model": model,
                "vector_length": embedding.vector.len(),
                "duration_ms": start.elapsed().as_millis(),
            }),
        );
    }

    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
fn is_llamacpp_backend_name(backend_name: &str) -> bool {
    canonical_backend_key(Some(backend_name)).as_deref() == Some("llamacpp")
}

/// Parse a llama.cpp `/completion` SSE data line into a content token.
///
/// llama.cpp streams `data: {"content": "token", ...}` per line.
#[cfg(feature = "inference-nodes")]
fn parse_llamacpp_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Parse an OpenAI-compatible `/v1/chat/completions` SSE data line into a content token.
///
/// Streams `data: {"choices": [{"delta": {"content": "token"}}]}` per line.
#[cfg(feature = "inference-nodes")]
fn parse_openai_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(feature = "inference-nodes")]
async fn execute_llm_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let system_prompt = inputs.get("system_prompt").and_then(|p| p.as_str());
    let extra_context = inputs.get("context").and_then(|c| c.as_str());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LLM server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No LLM server URL available".to_string())
    })?;

    let full_prompt = if let Some(ctx) = extra_context {
        format!("{}\n\nContext:\n{}", prompt, ctx)
    } else {
        prompt.to_string()
    };

    let mut messages = Vec::new();
    if let Some(sys) = system_prompt {
        messages.push(serde_json::json!({"role": "system", "content": sys}));
    }
    messages.push(serde_json::json!({"role": "user", "content": full_prompt}));

    let streaming = event_sink.is_some();
    let mut request_body = serde_json::json!({
        "model": "gpt-4",
        "messages": messages,
        "stream": streaming
    });

    // Forward model-specific inference settings into the request body
    let extra_settings = build_extra_settings(inputs);
    for (key, value) in &extra_settings {
        request_body[key] = value.clone();
    }

    let client = reqwest::Client::new();
    let http_response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

    if !http_response.status().is_success() {
        let error = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LLM error: {}",
            error
        )));
    }

    let response = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_openai_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        let line = buffer.trim().to_string();
        if let Some(token) = parse_openai_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
        })?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response));
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_vision_analysis(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let image_base64 = inputs
        .get("image")
        .and_then(|i| i.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing image input".to_string()))?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "Vision server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No vision server URL available".to_string())
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&serde_json::json!({
            "model": "gpt-4-vision-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "max_tokens": 4096
        }))
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Vision request failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Vision API error: {}",
            error_text
        )));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
    })?;

    let analysis = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let mut outputs = HashMap::new();
    outputs.insert("analysis".to_string(), serde_json::json!(analysis));
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
async fn execute_unload_model(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_ref_value = inputs.get("model_ref").ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Missing model_ref input. Connect an inference node's Model Reference output."
                .to_string(),
        )
    })?;
    let model_ref =
        ModelRefV2::validate_value(model_ref_value).map_err(NodeEngineError::ExecutionFailed)?;

    let engine = model_ref.engine.as_str();
    let model_id = model_ref.model_id.as_str();

    let trigger_value = inputs
        .get("trigger")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    log::info!(
        "UnloadModel: unloading '{}' from engine '{}'",
        model_id,
        engine
    );

    match engine {
        "llamacpp" => {
            let gw = require_gateway(gateway)?;
            gw.stop().await;
            log::info!(
                "UnloadModel: llama.cpp server stopped for model '{}'",
                model_id
            );
        }
        "ollama" => {
            let client = reqwest::Client::new();
            let url = "http://localhost:11434/api/generate";
            let request_body = serde_json::json!({
                "model": model_id,
                "keep_alive": 0
            });

            match client.post(url).json(&request_body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!(
                        "UnloadModel: Ollama model '{}' unloaded from VRAM",
                        model_id
                    );
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    log::warn!(
                        "UnloadModel: Ollama unload returned {} for model '{}': {}",
                        status,
                        model_id,
                        body
                    );
                }
                Err(e) => {
                    return Err(NodeEngineError::ExecutionFailed(format!(
                        "Failed to connect to Ollama server to unload model '{}': {}",
                        model_id, e
                    )));
                }
            }
        }
        #[cfg(feature = "pytorch-nodes")]
        "pytorch" => {
            use pyo3::types::PyAnyMethods;
            // Unload via PyO3 in-process call to the Python worker
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_torch_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload PyTorch model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: PyTorch model '{}' unloaded", model_id);
        }
        #[cfg(feature = "audio-nodes")]
        "stable_audio" => {
            use pyo3::types::PyAnyMethods;
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_audio_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload audio model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: audio model '{}' unloaded", model_id);
        }
        "onnx-runtime" | "onnxruntime" => {
            log::info!(
                "UnloadModel: onnx-runtime model '{}' does not keep a shared runtime session",
                model_id
            );
        }
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Unknown inference engine '{}'. Supported: llamacpp, ollama, pytorch, stable_audio, onnx-runtime",
                other
            )));
        }
    }

    let status_msg = format!("Model '{}' unloaded from {}", model_id, engine);

    let mut outputs = HashMap::new();
    outputs.insert("status".to_string(), serde_json::json!(status_msg));
    outputs.insert("trigger_passthrough".to_string(), trigger_value);
    Ok(outputs)
}

/// Ensure the PyTorch worker module (and its sibling modules) are loaded into
/// the Python interpreter.  Safe to call multiple times — only the first call
/// actually loads.
#[cfg(feature = "pytorch-nodes")]
fn ensure_torch_worker_initialised(py: pyo3::Python<'_>) -> std::result::Result<(), String> {
    if py.import("pantograph_torch_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling modules first so worker.py's imports resolve
    let bd_code = std::ffi::CString::new(include_str!("../../inference/torch/block_diffusion.py"))
        .map_err(|e| format!("Invalid block_diffusion source: {}", e))?;
    let bd_module =
        pyo3::types::PyModule::from_code(py, &bd_code, c"block_diffusion.py", c"block_diffusion")
            .map_err(|e| format!("Failed to load block_diffusion: {}", e))?;
    modules
        .set_item("block_diffusion", &bd_module)
        .map_err(|e| format!("Failed to register block_diffusion: {}", e))?;

    let ar_code = std::ffi::CString::new(include_str!("../../inference/torch/autoregressive.py"))
        .map_err(|e| format!("Invalid autoregressive source: {}", e))?;
    let ar_module =
        pyo3::types::PyModule::from_code(py, &ar_code, c"autoregressive.py", c"autoregressive")
            .map_err(|e| format!("Failed to load autoregressive: {}", e))?;
    modules
        .set_item("autoregressive", &ar_module)
        .map_err(|e| format!("Failed to register autoregressive: {}", e))?;

    // Now load the worker module (which imports from block_diffusion and autoregressive)
    let code = std::ffi::CString::new(include_str!("../../inference/torch/worker.py"))
        .map_err(|e| format!("Invalid worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_torch_worker",
        c"pantograph_torch_worker",
    )
    .map_err(|e| format!("Failed to load worker: {}", e))?;

    log::info!(
        "PyTorch worker module initialised (with block_diffusion + autoregressive siblings)"
    );
    Ok(())
}

/// Ensure the Stable Audio worker module (and its sibling) are loaded into
/// the Python interpreter.  Safe to call multiple times — only the first call
/// actually loads.
#[cfg(feature = "audio-nodes")]
fn ensure_audio_worker_initialised(py: pyo3::Python<'_>) -> std::result::Result<(), String> {
    if py.import("pantograph_audio_worker").is_ok() {
        return Ok(());
    }

    use pyo3::types::PyAnyMethods;

    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {}", e))?;
    let modules = sys
        .getattr("modules")
        .map_err(|e| format!("Failed to get sys.modules: {}", e))?;

    // Register sibling module first so worker.py's imports resolve
    let sa_code = std::ffi::CString::new(include_str!("../../inference/audio/stable_audio.py"))
        .map_err(|e| format!("Invalid stable_audio source: {}", e))?;
    let sa_module =
        pyo3::types::PyModule::from_code(py, &sa_code, c"stable_audio.py", c"stable_audio")
            .map_err(|e| format!("Failed to load stable_audio: {}", e))?;
    modules
        .set_item("stable_audio", &sa_module)
        .map_err(|e| format!("Failed to register stable_audio: {}", e))?;

    // Now load the worker module (which imports from stable_audio)
    let code = std::ffi::CString::new(include_str!("../../inference/audio/worker.py"))
        .map_err(|e| format!("Invalid audio worker source: {}", e))?;
    pyo3::types::PyModule::from_code(
        py,
        &code,
        c"pantograph_audio_worker",
        c"pantograph_audio_worker",
    )
    .map_err(|e| format!("Failed to load audio worker: {}", e))?;

    log::info!("Audio worker module initialised (with stable_audio sibling)");
    Ok(())
}

#[cfg(feature = "pytorch-nodes")]
async fn execute_pytorch_inference(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    // Detect if the prompt input is a masked prompt JSON object
    let masked_prompt_json = inputs
        .get("prompt")
        .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("masked_prompt"))
        .map(|p| serde_json::to_string(p).unwrap_or_default());

    let prompt = if let Some(p_str) = inputs.get("prompt").and_then(|p| p.as_str()) {
        p_str.to_string()
    } else if let Some(p_obj) = inputs.get("prompt") {
        // For masked prompt objects, concatenate all segment texts as the plain prompt
        if let Some(segments) = p_obj.get("segments").and_then(|s| s.as_array()) {
            segments
                .iter()
                .filter_map(|seg| seg.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        } else {
            return Err(NodeEngineError::ExecutionFailed(
                "Missing prompt input: not a string or masked prompt".to_string(),
            ));
        }
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Missing prompt input".to_string(),
        ));
    };

    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let system_prompt = inputs
        .get("system_prompt")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);
    let device = inputs
        .get("device")
        .and_then(|d| d.as_str())
        .unwrap_or("auto")
        .to_string();
    let model_type = inputs
        .get("model_type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pytorch-model")
        .to_string();

    // Phase 1: Check if model is already loaded, load if needed
    {
        let mp = model_path.clone();
        let dev = device.clone();
        let mt = model_type.clone();

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                // Ensure worker + sibling modules are initialised
                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to import worker: {}", e))?;

                // Check if the correct model is already loaded
                let info = worker
                    .call_method0("get_loaded_info")
                    .map_err(|e| format!("get_loaded_info failed: {}", e))?;

                let needs_load = if info.is_none() {
                    true
                } else {
                    let loaded_path: String = info
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default();
                    loaded_path != mp
                };

                if needs_load {
                    log::info!("PyTorchInference: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", &dev).unwrap();
                    if let Some(ref mt_val) = mt {
                        kwargs.set_item("model_type", mt_val).unwrap();
                    }
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Model load failed: {}", e))?;
                    log::info!("PyTorchInference: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| NodeEngineError::ExecutionFailed(e))?;
    }

    let _restored_kv_cache = kv_cache::restore_pytorch_input_handle(
        inputs,
        extensions,
        task_id,
        execution_id,
        event_sink,
    )
    .await?;

    // Read model-specific inference settings to forward as Python kwargs
    let extra_settings = build_extra_settings(inputs);
    // Keep top_p explicit even when inference_settings schema is missing.
    let top_p = inputs
        .get("top_p")
        .and_then(|v| v.as_f64())
        .or_else(|| extra_settings.get("top_p").and_then(|v| v.as_f64()))
        .unwrap_or(0.95);

    // Phase 2: Generate — streaming or non-streaming
    let response_text = if let Some(sink) = event_sink {
        // Streaming: iterate Python generator via mpsc channel
        // Channel carries (mode, text) tuples: mode is "append" or "replace"
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<std::result::Result<(String, String), String>>(32);
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings.clone();
        let top_p = top_p;

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| {
                use pyo3::types::{PyAnyMethods, PyDictMethods, PyTypeMethods};

                if let Err(e) = ensure_torch_worker_initialised(py) {
                    let _ = tx.blocking_send(Err(e));
                    return;
                }
                let worker = match py.import("pantograph_torch_worker") {
                    Ok(w) => w,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to get worker: {}", e)));
                        return;
                    }
                };

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let generator = match worker.call_method("generate_tokens", (), Some(&kwargs)) {
                    Ok(g) => g,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Failed to create generator: {}", e)));
                        return;
                    }
                };

                let iter = match generator.try_iter() {
                    Ok(it) => it,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(format!("Generator not iterable: {}", e)));
                        return;
                    }
                };

                for item in iter {
                    match item {
                        Ok(token_obj) => {
                            // Try dict first: {"mode": "append"|"replace", "text": "..."}
                            let result = if let Ok(dict) =
                                token_obj.downcast::<pyo3::types::PyDict>()
                            {
                                let mode = dict
                                    .get_item("mode")
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.extract::<String>().ok())
                                    .unwrap_or_else(|| "append".to_string());
                                let text = dict
                                    .get_item("text")
                                    .ok()
                                    .flatten()
                                    .and_then(|v| v.extract::<String>().ok())
                                    .unwrap_or_default();
                                Ok((mode, text))
                            } else if let Ok(text) = token_obj.extract::<String>() {
                                // Backwards compat: plain string → append
                                Ok(("append".to_string(), text))
                            } else {
                                Err(format!(
                                    "Token extraction failed: expected dict or string, got {:?}",
                                    token_obj.get_type().name()
                                ))
                            };
                            if tx.blocking_send(result).is_err() {
                                return;
                            }
                        }
                        Err(e) => {
                            let _ = tx.blocking_send(Err(format!("Generator error: {}", e)));
                            return;
                        }
                    }
                }
            });
        });

        let mut full_response = String::new();
        while let Some(token_result) = rx.recv().await {
            let (mode, text) = token_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("PyTorch generation error: {}", e))
            })?;
            if mode == "replace" {
                full_response = text.clone();
            } else {
                full_response.push_str(&text);
            }
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "stream",
                serde_json::json!({"mode": mode, "text": text}),
            ));
        }

        full_response
    } else {
        // Non-streaming: single blocking call
        let p = prompt.clone();
        let sp = system_prompt.clone();
        let mpj = masked_prompt_json.clone();
        let extra = extra_settings;
        let top_p = top_p;

        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<String, String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_torch_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_torch_worker")
                    .map_err(|e| format!("Failed to get worker: {}", e))?;

                let kwargs = pyo3::types::PyDict::new(py);
                kwargs.set_item("prompt", &p).unwrap();
                if let Some(ref sys) = sp {
                    kwargs.set_item("system_prompt", sys).unwrap();
                }
                kwargs.set_item("max_tokens", max_tokens).unwrap();
                kwargs.set_item("temperature", temperature).unwrap();
                kwargs.set_item("top_p", top_p).unwrap();
                if let Some(ref mpj_val) = mpj {
                    kwargs.set_item("masked_prompt_json", mpj_val).unwrap();
                }

                // Forward model-specific inference settings as kwargs
                for (key, value) in &extra {
                    if let Some(n) = value.as_i64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(n) = value.as_f64() {
                        kwargs.set_item(key.as_str(), n).unwrap();
                    } else if let Some(s) = value.as_str() {
                        kwargs.set_item(key.as_str(), s).unwrap();
                    } else if let Some(b) = value.as_bool() {
                        kwargs.set_item(key.as_str(), b).unwrap();
                    }
                }

                let result = worker
                    .call_method("generate", (), Some(&kwargs))
                    .map_err(|e| format!("Generation failed: {}", e))?;

                result
                    .extract::<String>()
                    .map_err(|e| format!("Failed to extract result: {}", e))
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(|e| NodeEngineError::ExecutionFailed(e))?
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    let task_type_primary = infer_task_type_primary("pytorch-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "pytorch",
        &model_name,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "pytorch",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    let kv_cache_output = match kv_cache::capture_pytorch_output_handle(
        task_id,
        execution_id,
        extensions,
        event_sink,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "PyTorchInference: failed to capture KV cache output for '{}': {}",
                task_id,
                error
            );
            serde_json::Value::Null
        }
    };
    outputs.insert("kv_cache_out".to_string(), kv_cache_output);
    Ok(outputs)
}

// ---------------------------------------------------------------------------
// Audio generation handler (behind audio-nodes feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "audio-nodes")]
async fn execute_audio_generation(
    inputs: &HashMap<String, serde_json::Value>,
    resolved_model_ref: Option<ModelRefV2>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?
        .to_string();

    let duration = inputs
        .get("duration")
        .and_then(|d| d.as_f64())
        .unwrap_or(30.0);
    let steps = inputs
        .get("num_inference_steps")
        .and_then(|s| s.as_i64())
        .unwrap_or(100);
    let guidance_scale = inputs
        .get("guidance_scale")
        .and_then(|g| g.as_f64())
        .unwrap_or(7.0);
    let seed = inputs.get("seed").and_then(|s| s.as_i64()).unwrap_or(-1);

    // Phase 1: Load model if needed
    {
        let mp = model_path.clone();
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(|py| -> std::result::Result<(), String> {
                use pyo3::types::{PyAnyMethods, PyDictMethods};

                ensure_audio_worker_initialised(py)?;
                let worker = py
                    .import("pantograph_audio_worker")
                    .map_err(|e| format!("Failed to import audio worker: {}", e))?;

                let info = worker
                    .call_method0("get_loaded_info")
                    .map_err(|e| format!("get_loaded_info failed: {}", e))?;

                let needs_load = if info.is_none() {
                    true
                } else {
                    let loaded_path: String = info
                        .get_item("model_path")
                        .ok()
                        .and_then(|v| v.extract::<String>().ok())
                        .unwrap_or_default();
                    loaded_path != mp
                };

                if needs_load {
                    log::info!("AudioGeneration: loading model from '{}'", mp);
                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("model_path", &mp).unwrap();
                    kwargs.set_item("device", "auto").unwrap();
                    worker
                        .call_method("load_model", (), Some(&kwargs))
                        .map_err(|e| format!("Audio model load failed: {}", e))?;
                    log::info!("AudioGeneration: model loaded successfully");
                }

                Ok(())
            })
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?;
    }

    // Phase 2: Generate audio
    let mut result = {
        let p = prompt;
        tokio::task::spawn_blocking(move || {
            pyo3::Python::with_gil(
                |py| -> std::result::Result<HashMap<String, serde_json::Value>, String> {
                    use pyo3::types::PyAnyMethods;

                    let worker = py
                        .import("pantograph_audio_worker")
                        .map_err(|e| format!("Failed to get audio worker: {}", e))?;

                    let kwargs = pyo3::types::PyDict::new(py);
                    kwargs.set_item("prompt", &p).unwrap();
                    kwargs.set_item("duration", duration).unwrap();
                    kwargs.set_item("steps", steps).unwrap();
                    kwargs.set_item("guidance_scale", guidance_scale).unwrap();
                    kwargs.set_item("seed", seed).unwrap();

                    let result = worker
                        .call_method("generate_audio_from_text", (), Some(&kwargs))
                        .map_err(|e| format!("Audio generation failed: {}", e))?;

                    // Extract dict fields
                    let audio_base64: String = result
                        .get_item("audio_base64")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or_default();
                    let duration_seconds: f64 = result
                        .get_item("duration_seconds")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(0.0);
                    let sample_rate: i64 = result
                        .get_item("sample_rate")
                        .ok()
                        .and_then(|v| v.extract().ok())
                        .unwrap_or(44100);

                    let mut outputs = HashMap::new();
                    outputs.insert("audio".to_string(), serde_json::json!(audio_base64));
                    outputs.insert(
                        "duration_seconds".to_string(),
                        serde_json::json!(duration_seconds),
                    );
                    outputs.insert("sample_rate".to_string(), serde_json::json!(sample_rate));
                    Ok(outputs)
                },
            )
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Task join error: {}", e)))?
        .map_err(NodeEngineError::ExecutionFailed)?
    };

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("audio-model")
        .to_string();
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "stable_audio",
        &model_name,
        &model_path,
        "text-to-audio",
        inputs,
    );
    result.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "stable_audio",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": "text-to-audio",
            })
        }),
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// KV Cache handlers (behind inference-nodes feature)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "core_executor/tests.rs"]
mod tests;
