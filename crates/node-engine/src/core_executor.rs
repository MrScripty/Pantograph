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
#[cfg(any(feature = "pytorch-nodes", feature = "audio-nodes"))]
use crate::model_dependencies::ModelRefV2;

mod dependency_preflight;
mod file_io;
#[cfg(feature = "inference-nodes")]
mod inference_nodes;
#[cfg(feature = "inference-nodes")]
mod kv_cache;
mod ollama;
mod pure_nodes;
mod settings;
pub(crate) use dependency_preflight::*;
pub(crate) use file_io::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use inference_nodes::*;
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
// PyTorch handlers (behind pytorch-nodes feature)
// ---------------------------------------------------------------------------

/// Ensure the PyTorch worker module (and its sibling modules) are loaded into
/// the Python interpreter.  Safe to call multiple times -- only the first call
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
