//! Core task executor with built-in node handlers.
//!
//! `CoreTaskExecutor` handles all node types whose logic is not host-specific.
//! Hosts (Tauri, NIF/Elixir) only need to handle nodes that require platform
//! resources (e.g. RAG manager, UI interaction).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(feature = "inference-nodes")]
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
#[cfg(feature = "inference-nodes")]
use inference::{
    InferenceExecutionInputKind, InferenceGateway, InferenceLifecyclePhase,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceTaskId, OptionCompatibilityDiagnostic,
    OptionSupportState,
};

use crate::engine::TaskExecutor;
use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
#[cfg(feature = "inference-nodes")]
use crate::extensions::extension_keys;
use crate::extensions::ExecutorExtensions;

#[cfg(feature = "audio-nodes")]
mod audio_nodes;
#[cfg(any(
    feature = "inference-nodes",
    feature = "pytorch-nodes",
    feature = "audio-nodes"
))]
mod dependency_preflight;
mod file_io;
#[cfg(feature = "inference-nodes")]
mod inference_nodes;
#[cfg(feature = "inference-nodes")]
mod kv_cache;
#[cfg(feature = "inference-nodes")]
mod llamacpp_nodes;
mod model_nodes;
mod processing_nodes;
mod pure_nodes;
#[cfg(feature = "pytorch-nodes")]
mod pytorch_nodes;
#[cfg(feature = "inference-nodes")]
mod retrieval_nodes;
mod settings;
#[cfg(feature = "audio-nodes")]
pub(crate) use audio_nodes::*;
#[cfg(any(
    feature = "inference-nodes",
    feature = "pytorch-nodes",
    feature = "audio-nodes"
))]
pub(crate) use dependency_preflight::*;
pub(crate) use file_io::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use inference_nodes::*;
#[cfg(feature = "inference-nodes")]
pub(crate) use llamacpp_nodes::*;
pub(crate) use model_nodes::*;
pub(crate) use processing_nodes::*;
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

#[cfg(feature = "inference-nodes")]
fn has_resolved_model_package_facts(inputs: &HashMap<String, serde_json::Value>) -> bool {
    inputs
        .get("resolved_model_package_facts")
        .or_else(|| inputs.get("resolvedModelPackageFacts"))
        .is_some_and(|value| !value.is_null())
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

#[cfg(any(feature = "inference-nodes", feature = "pytorch-nodes"))]
fn retired_inference_node_error(node_type: &str) -> Result<HashMap<String, serde_json::Value>> {
    Err(NodeEngineError::ExecutionFailed(format!(
        "Retired inference node type '{node_type}' is no longer executable. Migrate this workflow to canonical llm-inference with task_kind, backend_key, and a Pumas model reference."
    )))
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

            // Gateway-backed inference nodes (require `inference-nodes` feature)
            #[cfg(feature = "inference-nodes")]
            "embedding" => retired_inference_node_error("embedding"),
            #[cfg(feature = "inference-nodes")]
            "llamacpp-inference" => retired_inference_node_error("llamacpp-inference"),
            #[cfg(feature = "inference-nodes")]
            "reranker" => retired_inference_node_error("reranker"),
            #[cfg(feature = "inference-nodes")]
            "vision-analysis" => retired_inference_node_error("vision-analysis"),
            #[cfg(feature = "inference-nodes")]
            "llm-inference" => {
                let canonical_inputs = inputs_with_model_path_from_ref(&inputs)?;
                let exec_id = self.execution_id.as_deref().unwrap_or("unknown");
                let preferred_backend = preferred_backend_key("llm-inference", &canonical_inputs);
                reject_contract_only_inference_task(
                    &canonical_inputs,
                    extensions,
                    task_id,
                    exec_id,
                    preferred_backend.as_deref(),
                )?;
                match canonical_inference_input_kind(&canonical_inputs) {
                    Some(InferenceExecutionInputKind::TextGeneration) => {
                        execute_llm_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            task_id,
                            self.event_sink.as_ref(),
                            exec_id,
                            extensions,
                        )
                        .await
                    }
                    Some(InferenceExecutionInputKind::Embedding) => {
                        execute_embedding_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            extensions,
                            task_id,
                            exec_id,
                        )
                        .await
                    }
                    Some(InferenceExecutionInputKind::Rerank) => {
                        execute_rerank_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            extensions,
                            task_id,
                            exec_id,
                        )
                        .await
                    }
                    Some(InferenceExecutionInputKind::AudioTranscription) => {
                        execute_audio_transcription_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            extensions,
                            task_id,
                            exec_id,
                        )
                        .await
                    }
                    Some(InferenceExecutionInputKind::ImageGeneration) => {
                        execute_image_generation_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            extensions,
                            task_id,
                            exec_id,
                        )
                        .await
                    }
                    _ if has_resolved_model_package_facts(&canonical_inputs)
                        && canonical_inputs
                            .get("prompt")
                            .and_then(serde_json::Value::as_str)
                            .is_some_and(|prompt| !prompt.trim().is_empty()) =>
                    {
                        execute_llm_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            task_id,
                            self.event_sink.as_ref(),
                            exec_id,
                            extensions,
                        )
                        .await
                    }
                    _ if preferred_backend.as_deref() == Some("llamacpp") => {
                        let resolved_model_ref = enforce_dependency_preflight(
                            "llm-inference",
                            &canonical_inputs,
                            extensions,
                        )
                        .await?;
                        execute_llamacpp_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            task_id,
                            self.event_sink.as_ref(),
                            exec_id,
                            resolved_model_ref,
                            extensions,
                        )
                        .await
                    }
                    _ if preferred_backend.as_deref() == Some("pytorch") => {
                        #[cfg(feature = "pytorch-nodes")]
                        {
                            let preflight_context = dependency_preflight_lifecycle_context(
                                &canonical_inputs,
                                task_id,
                                exec_id,
                                preferred_backend.as_deref(),
                            );
                            let resolved_model_ref = enforce_dependency_preflight_with_lifecycle(
                                "llm-inference",
                                &canonical_inputs,
                                extensions,
                                Some(&preflight_context),
                            )
                            .await?;
                            execute_pytorch_inference(
                                &canonical_inputs,
                                task_id,
                                self.event_sink.as_ref(),
                                exec_id,
                                resolved_model_ref,
                                extensions,
                            )
                            .await
                        }
                        #[cfg(not(feature = "pytorch-nodes"))]
                        {
                            execute_llm_inference(
                                self.gateway.as_ref(),
                                &canonical_inputs,
                                task_id,
                                self.event_sink.as_ref(),
                                exec_id,
                                extensions,
                            )
                            .await
                        }
                    }
                    _ => {
                        execute_llm_inference(
                            self.gateway.as_ref(),
                            &canonical_inputs,
                            task_id,
                            self.event_sink.as_ref(),
                            exec_id,
                            extensions,
                        )
                        .await
                    }
                }
            }
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

#[cfg(feature = "inference-nodes")]
fn reject_contract_only_inference_task(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
    backend_key: Option<&str>,
) -> Result<()> {
    let Some(entry) = canonical_inference_task_entry(inputs) else {
        return Ok(());
    };
    let Some(contract) = entry.request_contract() else {
        return Ok(());
    };
    if contract.execution_supported {
        return Ok(());
    }

    let message = format!(
        "Canonical inference task '{}' is contract-only at this execution boundary: task request contract has execution_supported=false for input kind '{}'.",
        entry.canonical_label(),
        contract.input_kind.canonical_label()
    );
    let option_diagnostics =
        contract_only_task_option_diagnostics(inputs, entry.task_id.clone(), backend_key);
    let artifact_refs = contract_only_task_artifact_refs(inputs);
    record_task_validation_failure_lifecycle(
        extensions,
        task_id,
        execution_id,
        entry.canonical_label(),
        backend_key,
        inference_model_id_from_inputs(inputs),
        message.clone(),
        option_diagnostics,
        artifact_refs,
    );

    Err(NodeEngineError::ExecutionFailed(message))
}

#[cfg(feature = "inference-nodes")]
fn contract_only_task_option_diagnostics(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: InferenceTaskId,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    match task_id {
        InferenceTaskId::DepthEstimation => {
            depth_estimation_task_option_diagnostics(inputs, backend_key)
        }
        InferenceTaskId::VideoUnderstanding => {
            video_understanding_task_option_diagnostics(inputs, backend_key)
        }
        _ => Vec::new(),
    }
}

#[cfg(feature = "inference-nodes")]
fn depth_estimation_task_option_diagnostics(
    inputs: &HashMap<String, serde_json::Value>,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let mut diagnostics = Vec::new();
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "depth_estimation.output_format",
        &["output_format", "outputFormat"],
        "depth_estimation",
    );
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "depth_estimation.include_point_cloud",
        &["include_point_cloud", "includePointCloud"],
        "depth_estimation",
    );
    diagnostics
}

#[cfg(feature = "inference-nodes")]
fn video_understanding_task_option_diagnostics(
    inputs: &HashMap<String, serde_json::Value>,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let mut diagnostics = Vec::new();
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "video_understanding.frame_sample_rate",
        &[
            "frame_sample_rate",
            "frameSampleRate",
            "sample_rate",
            "sampleRate",
        ],
        "video_understanding",
    );
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "video_understanding.max_frames",
        &["max_frames", "maxFrames"],
        "video_understanding",
    );
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "video_understanding.start_time_seconds",
        &["start_time_seconds", "startTimeSeconds"],
        "video_understanding",
    );
    push_contract_only_option_diagnostic(
        inputs,
        &mut diagnostics,
        backend_key,
        "video_understanding.end_time_seconds",
        &["end_time_seconds", "endTimeSeconds"],
        "video_understanding",
    );
    diagnostics
}

#[cfg(feature = "inference-nodes")]
fn push_contract_only_option_diagnostic(
    inputs: &HashMap<String, serde_json::Value>,
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    backend_key: Option<&str>,
    option_path: &str,
    aliases: &[&str],
    task_label: &str,
) {
    if !task_option_present(inputs, aliases) {
        return;
    }

    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: option_path.to_string(),
        state: OptionSupportState::BackendUnavailable,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(
            format!("{task_label} is contract-only at this execution boundary; option support is deferred to an executable backend"),
        ),
    });
}

#[cfg(feature = "inference-nodes")]
fn task_option_present(inputs: &HashMap<String, serde_json::Value>, aliases: &[&str]) -> bool {
    aliases.iter().any(|alias| {
        inputs.get(*alias).is_some_and(|value| !value.is_null())
            || inputs
                .get("task_options")
                .and_then(|task_options| task_options.get(*alias))
                .is_some_and(|value| !value.is_null())
    })
}

#[cfg(feature = "inference-nodes")]
fn record_task_validation_failure_lifecycle(
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
    task_label: &str,
    backend_key: Option<&str>,
    model_id: Option<String>,
    detail: String,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    artifact_refs: Vec<String>,
) {
    let Some(sink) = extensions
        .get::<Arc<dyn InferenceRequestLifecycleEventSink>>(
            extension_keys::INFERENCE_LIFECYCLE_SINK,
        )
        .cloned()
    else {
        return;
    };

    let request_id = Some(format!("{execution_id}:{task_id}:{task_label}"));
    let backend_key = backend_key.map(ToOwned::to_owned);
    let runtime_id = backend_key.clone();
    for (kind, detail) in [
        (InferenceRequestLifecycleEventKind::Started, None),
        (InferenceRequestLifecycleEventKind::Failed, Some(detail)),
        (InferenceRequestLifecycleEventKind::CleanupCompleted, None),
    ] {
        let event_option_diagnostics = if matches!(kind, InferenceRequestLifecycleEventKind::Failed)
        {
            option_diagnostics.clone()
        } else {
            Vec::new()
        };
        let event_artifact_refs = if matches!(kind, InferenceRequestLifecycleEventKind::Failed) {
            artifact_refs.clone()
        } else {
            Vec::new()
        };
        if let Err(error) = sink.record(InferenceRequestLifecycleEvent {
            request_id: request_id.clone(),
            phase: InferenceLifecyclePhase::TaskValidation,
            kind,
            occurred_at_ms: unix_timestamp_ms(),
            task_id: Some(task_label.to_string()),
            backend_key: backend_key.clone(),
            runtime_id: runtime_id.clone(),
            selected_runtime_variant_id: None,
            runtime_instance_id: None,
            selected_device_class: None,
            selected_device_id: None,
            selected_network_node_id: None,
            model_id: model_id.clone(),
            resolved_artifact_kind: None,
            usage: None,
            cache_handle_id: None,
            artifact_refs: event_artifact_refs,
            detail,
            canonical_error_event_id: None,
            compatibility_report: None,
            compatibility_issues: Vec::new(),
            option_diagnostics: event_option_diagnostics,
        }) {
            log::warn!("failed to record inference task validation lifecycle event: {error}");
        }
    }
}

#[cfg(feature = "inference-nodes")]
fn contract_only_task_artifact_refs(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
    let mut refs = Vec::new();
    for value in inputs.values() {
        collect_bounded_artifact_refs_from_value(value, &mut refs);
    }
    refs
}

#[cfg(feature = "inference-nodes")]
fn collect_bounded_artifact_refs_from_value(value: &serde_json::Value, refs: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if is_artifact_ref_key(key) {
                    collect_bounded_artifact_ref_leaf(value, refs);
                } else {
                    collect_bounded_artifact_refs_from_value(value, refs);
                }
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                collect_bounded_artifact_refs_from_value(value, refs);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "inference-nodes")]
fn collect_bounded_artifact_ref_leaf(value: &serde_json::Value, refs: &mut Vec<String>) {
    match value {
        serde_json::Value::String(value) => push_bounded_artifact_ref(value, refs),
        serde_json::Value::Array(values) => {
            for value in values {
                collect_bounded_artifact_ref_leaf(value, refs);
            }
        }
        serde_json::Value::Object(object) => {
            for value in object.values() {
                collect_bounded_artifact_ref_leaf(value, refs);
            }
        }
        _ => {}
    }
}

#[cfg(feature = "inference-nodes")]
fn push_bounded_artifact_ref(value: &str, refs: &mut Vec<String>) {
    let Some(value) = inference::bounded_inference_artifact_ref(value) else {
        return;
    };
    if !refs.iter().any(|existing| existing == &value) {
        refs.push(value);
    }
}

#[cfg(feature = "inference-nodes")]
fn is_artifact_ref_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|character| *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect();
    matches!(
        normalized.as_str(),
        "imageref" | "imagerefs" | "videoref" | "videorefs" | "artifactref" | "artifactrefs"
    )
}

#[cfg(feature = "inference-nodes")]
fn inference_model_id_from_inputs(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
    inputs
        .get("model_name")
        .or_else(|| inputs.get("model"))
        .or_else(|| inputs.get("model_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            inputs
                .get("pumas_model_ref")
                .or_else(|| inputs.get("model_ref"))
                .and_then(|value| {
                    value
                        .get("model_id")
                        .or_else(|| value.get("modelId"))
                        .and_then(serde_json::Value::as_str)
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .or_else(|| {
            inputs
                .get("resolved_model_source")
                .and_then(|value| value.get("model_ref").or_else(|| value.get("modelRef")))
                .and_then(|value| {
                    value
                        .get("model_id")
                        .or_else(|| value.get("modelId"))
                        .and_then(serde_json::Value::as_str)
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
}

#[cfg(feature = "pytorch-nodes")]
fn dependency_preflight_lifecycle_context(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    execution_id: &str,
    backend_key: Option<&str>,
) -> DependencyPreflightLifecycleContext {
    let task_label = canonical_inference_task_entry(inputs)
        .map(|entry| entry.canonical_label().to_string())
        .unwrap_or_else(|| "text_generation".to_string());
    DependencyPreflightLifecycleContext {
        task_id: task_id.to_string(),
        execution_id: execution_id.to_string(),
        task_label,
        backend_key: backend_key.map(ToOwned::to_owned),
        model_id: inference_model_id_from_inputs(inputs),
        resolved_artifact_kind: read_resolved_artifact_kind_from_inputs(inputs),
    }
}

#[cfg(feature = "inference-nodes")]
fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
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
