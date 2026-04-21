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

#[cfg(feature = "audio-nodes")]
mod audio_nodes;
mod dependency_preflight;
mod file_io;
#[cfg(feature = "inference-nodes")]
mod inference_nodes;
#[cfg(feature = "inference-nodes")]
mod kv_cache;
#[cfg(feature = "inference-nodes")]
mod llamacpp_nodes;
mod ollama;
mod pure_nodes;
#[cfg(feature = "pytorch-nodes")]
mod pytorch_nodes;
#[cfg(feature = "inference-nodes")]
mod retrieval_nodes;
mod settings;
#[cfg(feature = "audio-nodes")]
pub(crate) use audio_nodes::*;
pub(crate) use dependency_preflight::*;
pub(crate) use file_io::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use inference_nodes::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use llamacpp_nodes::*;
pub(crate) use ollama::*;
pub(crate) use pure_nodes::*;
#[cfg(feature = "pytorch-nodes")]
pub(crate) use pytorch_nodes::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use retrieval_nodes::*;
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
// Feature-gated Python worker handlers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// KV Cache handlers (behind inference-nodes feature)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "core_executor/tests.rs"]
mod tests;
