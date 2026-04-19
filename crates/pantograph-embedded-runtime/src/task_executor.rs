//! Host task executor for Pantograph-specific node types.
//!
//! Only handles node types that require Pantograph host resources
//! (for example RAG search or Python-backed execution). All other nodes are
//! handled by `CoreTaskExecutor` via `CompositeTaskExecutor`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use chrono::Utc;
use node_engine::{
    Context, DependencyState, EventSink, ExecutorExtensions, ModelDependencyRequest,
    ModelDependencyRequirements, ModelDependencyResolver, ModelDependencyStatus, NodeEngineError,
    Result, TaskExecutor, WorkflowEvent, core_executor::resolve_node_type, extension_keys,
};
use pantograph_runtime_identity::canonical_engine_backend_key;

use crate::python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
    PythonStreamHandler,
};
pub use crate::python_runtime_execution::{
    PythonRuntimeExecutionMetadata, PythonRuntimeExecutionRecorder,
};
use crate::rag::RagBackend;
use crate::runtime_health::failed_runtime_health_assessment;

/// Host task executor that handles only Pantograph host-dependent nodes.
///
/// Currently handles:
/// - `rag-search`: requires an injected `RagBackend`
/// - `pytorch-inference`: python sidecar execution
/// - `diffusion-inference`: python sidecar execution
/// - `audio-generation`: python sidecar execution
/// - `onnx-inference`: python sidecar execution
///
/// All other node types should be handled by `CoreTaskExecutor` via
/// `CompositeTaskExecutor`. Unknown types return the sentinel error
/// that `CompositeTaskExecutor` uses for fallthrough.
pub struct TauriTaskExecutor {
    /// Optional host-provided RAG backend for document search.
    rag_backend: Option<Arc<dyn RagBackend>>,
    /// Host adapter for python-backed nodes (pytorch/diffusion/audio/onnx).
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
}

/// Pantograph-specific extension keys used by host executors.
pub mod runtime_extension_keys {
    /// `Arc<dyn node_engine::EventSink>` for streaming host-side events.
    pub const EVENT_SINK: &str = "pantograph_event_sink";
    /// Execution identifier for host-side stream/progress events.
    pub const EXECUTION_ID: &str = "pantograph_execution_id";
    /// Recorder for Python-backed runtime execution metadata captured during a run.
    pub const PYTHON_RUNTIME_EXECUTION_RECORDER: &str =
        "pantograph_python_runtime_execution_recorder";
}

impl TauriTaskExecutor {
    const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV64_PRIME: u64 = 0x0000_0100_0000_01B3;
    const PYTHON_RUNTIME_FAILURE_THRESHOLD: u32 = 3;

    fn canonical_backend_key(value: Option<&str>) -> Option<String> {
        canonical_engine_backend_key(value)
    }

    /// Create a new task executor with the default process Python runtime.
    pub fn new(rag_backend: Option<Arc<dyn RagBackend>>) -> Self {
        Self::with_python_runtime(rag_backend, Arc::new(ProcessPythonRuntimeAdapter))
    }

    /// Create a task executor with a custom python runtime adapter.
    pub fn with_python_runtime(
        rag_backend: Option<Arc<dyn RagBackend>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            rag_backend,
            python_runtime,
        }
    }

    /// Execute a RAG search task
    async fn execute_rag_search(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let query = inputs
            .get("query")
            .and_then(|q| q.as_str())
            .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;

        let limit = inputs
            .get("limit")
            .and_then(|l| l.as_f64())
            .map(|l| l as usize)
            .unwrap_or(5);

        let rag_backend = self.rag_backend.as_ref().ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "rag-search node requires a configured RAG backend".to_string(),
            )
        })?;
        let docs = rag_backend
            .search_as_docs(query, limit)
            .await
            .map_err(|e| NodeEngineError::ExecutionFailed(format!("RAG search failed: {}", e)))?;

        // Build context string
        let context_str = docs
            .iter()
            .map(|doc| format!("## {}\n{}", doc.title, doc.content))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");

        let mut outputs = HashMap::new();
        outputs.insert(
            "documents".to_string(),
            serde_json::to_value(&docs).unwrap(),
        );
        outputs.insert("context".to_string(), serde_json::json!(context_str));
        Ok(outputs)
    }

    fn collect_model_ref_env_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let model_ref = inputs.get("model_ref");
        let Some(bindings) = model_ref
            .and_then(|v| v.get("dependencyBindings"))
            .and_then(|v| v.as_array())
        else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for binding in bindings {
            let env_id = binding
                .get("envId")
                .and_then(|v| v.as_str())
                .or_else(|| binding.get("env_id").and_then(|v| v.as_str()));
            if let Some(env_id) = env_id {
                let trimmed = env_id.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    fn collect_environment_ref_env_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let environment_ref =
            Self::read_optional_input_value_aliases(inputs, &["environment_ref", "environmentRef"]);
        let Some(environment_ref) = environment_ref else {
            return Vec::new();
        };

        let mut out = Vec::new();
        if let Some(env_id) = environment_ref
            .get("env_id")
            .and_then(|v| v.as_str())
            .or_else(|| environment_ref.get("envId").and_then(|v| v.as_str()))
        {
            let trimmed = env_id.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
        if let Some(env_ids) = environment_ref
            .get("env_ids")
            .and_then(|v| v.as_array())
            .or_else(|| environment_ref.get("envIds").and_then(|v| v.as_array()))
        {
            for value in env_ids {
                if let Some(env_id) = value.as_str() {
                    let trimmed = env_id.trim();
                    if !trimmed.is_empty() {
                        out.push(trimmed.to_string());
                    }
                }
            }
        }

        out.sort();
        out.dedup();
        out
    }

    fn collect_runtime_env_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let mut out = Self::collect_model_ref_env_ids(inputs);
        out.extend(Self::collect_environment_ref_env_ids(inputs));
        out.sort();
        out.dedup();
        out
    }

    fn python_runtime_recorder(
        extensions: &ExecutorExtensions,
    ) -> Option<Arc<PythonRuntimeExecutionRecorder>> {
        extensions
            .get::<Arc<PythonRuntimeExecutionRecorder>>(
                runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
            )
            .cloned()
    }

    fn python_runtime_backend_id(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> String {
        if let Some(backend_key) =
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
                .and_then(|value| Self::canonical_backend_key(Some(&value)))
        {
            return backend_key;
        }

        if let Some(engine) = inputs
            .get("model_ref")
            .and_then(|value| value.get("engine"))
            .and_then(|value| value.as_str())
            .and_then(|value| Self::canonical_backend_key(Some(value)))
        {
            return engine;
        }

        match node_type {
            "onnx-inference" => "onnx-runtime".to_string(),
            "audio-generation" => "stable_audio".to_string(),
            _ => "pytorch".to_string(),
        }
    }

    fn python_runtime_model_target(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
        inputs
            .get("model_ref")
            .and_then(|value| value.get("modelPath"))
            .and_then(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
    }

    fn python_runtime_instance_id(runtime_id: &str, env_ids: &[String]) -> String {
        if env_ids.is_empty() {
            return format!("python-runtime:{}:default", runtime_id);
        }

        if env_ids.len() == 1 {
            return format!(
                "python-runtime:{}:{}",
                runtime_id,
                Self::sanitize_key_component(&env_ids[0])
            );
        }

        let env_material = env_ids.join("|");
        format!(
            "python-runtime:{}:{}",
            runtime_id,
            Self::stable_hash_hex(&env_material)
        )
    }

    fn python_runtime_execution_metadata(
        node_type: &str,
        request: &PythonNodeExecutionRequest,
        runtime_reused: bool,
    ) -> PythonRuntimeExecutionMetadata {
        let runtime_id = Self::python_runtime_backend_id(node_type, &request.inputs);
        PythonRuntimeExecutionMetadata {
            snapshot: inference::RuntimeLifecycleSnapshot {
                runtime_id: Some(runtime_id.clone()),
                runtime_instance_id: Some(Self::python_runtime_instance_id(
                    &runtime_id,
                    &request.env_ids,
                )),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(runtime_reused),
                lifecycle_decision_reason: None,
                active: true,
                last_error: None,
            },
            model_target: Self::python_runtime_model_target(&request.inputs),
            health_assessment: None,
        }
    }

    fn apply_inference_setting_defaults(inputs: &mut HashMap<String, serde_json::Value>) {
        let schema = inputs
            .get("inference_settings")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();

        for parameter in &schema {
            let Some(key) = parameter.get("key").and_then(|value| value.as_str()) else {
                continue;
            };
            let key = key.trim();
            if key.is_empty() {
                continue;
            }

            let has_non_null_value = inputs.get(key).is_some_and(|value| !value.is_null());
            let candidate_value = if has_non_null_value {
                inputs.get(key).cloned()
            } else {
                parameter.get("default").cloned()
            };
            let Some(raw_value) = candidate_value else {
                continue;
            };
            let resolved_value =
                Self::resolve_inference_setting_runtime_value(parameter, raw_value);
            if resolved_value.is_null() {
                continue;
            }
            inputs.insert(key.to_string(), resolved_value);
        }
    }

    fn promote_runtime_metadata(inputs: &mut HashMap<String, serde_json::Value>) {
        for key in ["task_type_primary", "model_type", "backend_key"] {
            let nested = inputs.get("_data").and_then(|data| data.get(key)).cloned();
            let Some(value) = nested.or_else(|| Self::read_optional_input_value(inputs, key))
            else {
                continue;
            };
            if value.is_null() {
                continue;
            }

            let should_override = match inputs.get(key) {
                None => true,
                Some(existing) if existing.is_null() => true,
                Some(existing) if existing.as_str() == Some("unknown") => true,
                Some(existing) if existing.as_str() == Some("") => true,
                _ => false,
            };

            if should_override {
                inputs.insert(key.to_string(), value);
            }
        }
    }

    fn resolve_inference_setting_runtime_value(
        parameter: &serde_json::Value,
        value: serde_json::Value,
    ) -> serde_json::Value {
        let normalized = if let serde_json::Value::Object(map) = &value {
            if map.contains_key("label") {
                if let Some(option_value) = map.get("value") {
                    option_value.clone()
                } else {
                    value
                }
            } else {
                value
            }
        } else {
            value
        };

        let Some(candidate_label) = normalized.as_str() else {
            return normalized;
        };

        let allowed_values = parameter
            .get("constraints")
            .and_then(|constraints| constraints.get("allowed_values"))
            .and_then(|values| values.as_array());
        let Some(allowed_values) = allowed_values else {
            return normalized;
        };

        for option in allowed_values {
            if let serde_json::Value::Object(option_map) = option {
                let option_label = option_map
                    .get("label")
                    .or_else(|| option_map.get("name"))
                    .or_else(|| option_map.get("key"))
                    .and_then(|v| v.as_str());
                if option_label == Some(candidate_label) {
                    if let Some(option_value) = option_map.get("value") {
                        return option_value.clone();
                    }
                }
            }
        }

        normalized
    }

    fn read_optional_input_string(
        inputs: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<String> {
        inputs
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                inputs
                    .get("_data")
                    .and_then(|d| d.get(key))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
    }

    fn read_optional_input_value(
        inputs: &HashMap<String, serde_json::Value>,
        key: &str,
    ) -> Option<serde_json::Value> {
        inputs
            .get(key)
            .cloned()
            .or_else(|| inputs.get("_data").and_then(|d| d.get(key)).cloned())
    }

    fn read_optional_input_string_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<String> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_string(inputs, key))
    }

    fn read_optional_input_value_aliases(
        inputs: &HashMap<String, serde_json::Value>,
        aliases: &[&str],
    ) -> Option<serde_json::Value> {
        aliases
            .iter()
            .find_map(|key| Self::read_optional_input_value(inputs, key))
    }

    fn parse_requirements_fallback(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Option<node_engine::ModelDependencyRequirements> {
        let raw = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_requirements", "dependencyRequirements"],
        )?;
        serde_json::from_value(raw).ok()
    }

    fn read_input_dependency_override_patches(
        inputs: &HashMap<String, serde_json::Value>,
    ) -> Vec<node_engine::DependencyOverridePatchV1> {
        let Some(raw) = Self::read_optional_input_value_aliases(
            inputs,
            &[
                "dependency_override_patches",
                "dependencyOverridePatches",
                "manual_overrides",
                "manualOverrides",
            ],
        ) else {
            return Vec::new();
        };

        if raw.is_null() {
            return Vec::new();
        }
        if raw.is_object() {
            return serde_json::from_value::<node_engine::DependencyOverridePatchV1>(raw)
                .map(|single| vec![single])
                .unwrap_or_default();
        }
        serde_json::from_value::<Vec<node_engine::DependencyOverridePatchV1>>(raw)
            .unwrap_or_default()
    }

    fn fallback_platform_context_from_key(platform_key: &str) -> Option<serde_json::Value> {
        let normalized = platform_key.trim();
        if normalized.is_empty() {
            return None;
        }

        let mut parts = normalized.split('-');
        let os = parts.next().unwrap_or_default().trim();
        let arch = parts.next().unwrap_or_default().trim();
        if os.is_empty() || arch.is_empty() {
            return None;
        }

        Some(serde_json::json!({ "os": os, "arch": arch }))
    }

    fn read_input_selected_binding_ids(inputs: &HashMap<String, serde_json::Value>) -> Vec<String> {
        let Some(raw) = Self::read_optional_input_value_aliases(
            inputs,
            &["selected_binding_ids", "selectedBindingIds"],
        ) else {
            return Vec::new();
        };

        raw.as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| !s.trim().is_empty())
            .collect()
    }

    fn puma_lib_task_type_from_pipeline_tag(pipeline_tag: &str) -> String {
        match pipeline_tag.trim().to_lowercase().as_str() {
            "text-to-audio" | "text-to-speech" => "text-to-audio".to_string(),
            "automatic-speech-recognition" => "audio-to-text".to_string(),
            "text-to-image" | "image-to-image" => "text-to-image".to_string(),
            "image-classification" | "object-detection" | "image-to-text" => {
                "image-to-text".to_string()
            }
            "feature-extraction" | "sentence-similarity" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    fn puma_lib_metadata_string(
        metadata: &serde_json::Map<String, serde_json::Value>,
        keys: &[&str],
    ) -> Option<String> {
        keys.iter().find_map(|key| {
            metadata
                .get(*key)
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        })
    }

    fn insert_puma_lib_output_string(
        outputs: &mut HashMap<String, serde_json::Value>,
        key: &str,
        value: Option<String>,
    ) {
        if let Some(value) = value.filter(|value| !value.trim().is_empty()) {
            outputs.insert(key.to_string(), serde_json::json!(value));
        }
    }

    async fn execute_puma_lib(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut model_path =
            Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                .unwrap_or_default();
        let model_id = Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"]);
        let mut model_type =
            Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"]);
        let mut task_type_primary = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        );
        let backend_key =
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"]);
        let mut recommended_backend = Self::read_optional_input_string_aliases(
            inputs,
            &["recommended_backend", "recommendedBackend"],
        );

        if let (Some(model_id), Some(api)) = (
            model_id.as_deref(),
            extensions.get::<Arc<pumas_library::PumasApi>>(extension_keys::PUMAS_API),
        ) {
            match api.get_model(model_id).await {
                Ok(Some(model)) => {
                    if !model.path.trim().is_empty() {
                        model_path = model.path.clone();
                    }
                    if model_type
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                    {
                        model_type = Some(model.model_type.clone());
                    }

                    if let Some(metadata) = model.metadata.as_object() {
                        if task_type_primary
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                        {
                            task_type_primary = Self::puma_lib_metadata_string(
                                metadata,
                                &[
                                    "task_type_primary",
                                    "taskTypePrimary",
                                    "task_type",
                                    "taskType",
                                ],
                            )
                            .or_else(|| {
                                Self::puma_lib_metadata_string(
                                    metadata,
                                    &["pipeline_tag", "pipelineTag"],
                                )
                                .map(|value| Self::puma_lib_task_type_from_pipeline_tag(&value))
                            });
                        }

                        if recommended_backend
                            .as_deref()
                            .is_none_or(|value| value.trim().is_empty())
                        {
                            recommended_backend = Self::puma_lib_metadata_string(
                                metadata,
                                &["recommended_backend", "recommendedBackend"],
                            );
                        }
                    }

                    match api.resolve_model_execution_descriptor(model_id).await {
                        Ok(descriptor) => {
                            if !descriptor.entry_path.trim().is_empty() {
                                model_path = descriptor.entry_path;
                            }
                            if !descriptor.model_type.trim().is_empty() {
                                model_type = Some(descriptor.model_type);
                            }
                            let task = descriptor.task_type_primary.trim();
                            if !task.is_empty() && task != "unknown" {
                                task_type_primary = Some(task.to_string());
                            }
                        }
                        Err(error) => {
                            log::warn!(
                                "Puma-Lib execution descriptor lookup failed for '{}': {}",
                                model_id,
                                error
                            );
                        }
                    }
                }
                Ok(None) => {
                    log::warn!(
                        "Puma-Lib model '{}' was not found during workflow execution; using saved node data",
                        model_id
                    );
                }
                Err(error) => {
                    log::warn!(
                        "Puma-Lib lookup failed for '{}': {}; using saved node data",
                        model_id,
                        error
                    );
                }
            }
        }

        let inference_settings = Self::read_optional_input_value_aliases(
            inputs,
            &["inference_settings", "inferenceSettings"],
        )
        .filter(|value| value.is_array())
        .unwrap_or_else(|| serde_json::json!([]));

        let mut outputs = HashMap::new();
        outputs.insert("model_path".to_string(), serde_json::json!(model_path));
        outputs.insert("inference_settings".to_string(), inference_settings);
        Self::insert_puma_lib_output_string(&mut outputs, "model_id", model_id);
        Self::insert_puma_lib_output_string(&mut outputs, "model_type", model_type);
        Self::insert_puma_lib_output_string(&mut outputs, "task_type_primary", task_type_primary);
        Self::insert_puma_lib_output_string(&mut outputs, "backend_key", backend_key);
        Self::insert_puma_lib_output_string(
            &mut outputs,
            "recommended_backend",
            recommended_backend,
        );

        if let Some(selected_binding_ids) = Self::read_optional_input_value_aliases(
            inputs,
            &["selected_binding_ids", "selectedBindingIds"],
        )
        .filter(|value| value.is_array())
        {
            outputs.insert("selected_binding_ids".to_string(), selected_binding_ids);
        }
        if let Some(platform_context) = Self::read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        )
        .filter(|value| value.is_object())
        {
            outputs.insert("platform_context".to_string(), platform_context);
        }
        if let Some(dependency_bindings) = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_bindings", "dependencyBindings"],
        )
        .filter(|value| value.is_array())
        {
            outputs.insert("dependency_bindings".to_string(), dependency_bindings);
        }
        if let Some(dependency_requirements) = Self::read_optional_input_value_aliases(
            inputs,
            &["dependency_requirements", "dependencyRequirements"],
        )
        .filter(|value| value.is_object())
        {
            outputs.insert(
                "dependency_requirements".to_string(),
                dependency_requirements,
            );
        }
        Self::insert_puma_lib_output_string(
            &mut outputs,
            "dependency_requirements_id",
            Self::read_optional_input_string_aliases(
                inputs,
                &["dependency_requirements_id", "dependencyRequirementsId"],
            ),
        );

        log::debug!("PumaLib: providing model path '{}'", model_path);
        Ok(outputs)
    }

    fn infer_task_type_primary(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> String {
        if let Some(task) = Self::read_optional_input_string_aliases(
            inputs,
            &["task_type_primary", "taskTypePrimary"],
        ) {
            if !task.trim().is_empty() {
                return task;
            }
        }

        let model_type =
            Self::read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
                .unwrap_or_default()
                .to_lowercase();

        if node_type == "audio-generation" || model_type == "audio" {
            return "text-to-audio".to_string();
        }
        if node_type == "diffusion-inference" {
            return "text-to-image".to_string();
        }

        match model_type.as_str() {
            "diffusion" => "text-to-image".to_string(),
            "vision" => "image-to-text".to_string(),
            "embedding" => "feature-extraction".to_string(),
            _ => "text-generation".to_string(),
        }
    }

    fn infer_backend_key(node_type: &str) -> Option<String> {
        match node_type {
            "audio-generation" => Some("stable_audio".to_string()),
            "pytorch-inference" => Some("pytorch".to_string()),
            // Leave diffusion unspecified when the graph does not provide a
            // concrete backend so Pumas can apply the model's recommended
            // execution profile.
            "diffusion-inference" => None,
            "onnx-inference" => Some("onnx-runtime".to_string()),
            _ => Some("pytorch".to_string()),
        }
    }

    fn preferred_backend_key(
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        requirements: Option<&ModelDependencyRequirements>,
    ) -> Option<String> {
        if node_type == "diffusion-inference" {
            if let Some(backend) = Self::canonical_backend_key(
                Self::read_optional_input_string_aliases(
                    inputs,
                    &["recommended_backend", "recommendedBackend"],
                )
                .as_deref(),
            ) {
                return Some(backend);
            }
        }

        Self::canonical_backend_key(
            Self::read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
                .as_deref(),
        )
        .or_else(|| {
            Self::canonical_backend_key(
                requirements.as_ref().and_then(|r| r.backend_key.as_deref()),
            )
        })
    }

    fn build_model_dependency_request(
        node_type: &str,
        model_path: &str,
        inputs: &HashMap<String, serde_json::Value>,
    ) -> ModelDependencyRequest {
        let requirements = Self::parse_requirements_fallback(inputs);
        let backend_key = Self::preferred_backend_key(node_type, inputs, requirements.as_ref())
            .or_else(|| Self::infer_backend_key(node_type));

        let task_type_primary = Self::infer_task_type_primary(node_type, inputs);
        let model_id = Self::read_optional_input_string_aliases(inputs, &["model_id", "modelId"])
            .or_else(|| requirements.as_ref().map(|r| r.model_id.clone()));
        let platform_context = Self::read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        )
        .or_else(|| {
            requirements
                .as_ref()
                .and_then(|r| Self::fallback_platform_context_from_key(&r.platform_key))
        });

        let mut selected_binding_ids = Self::read_input_selected_binding_ids(inputs);
        if selected_binding_ids.is_empty() {
            if let Some(req) = &requirements {
                selected_binding_ids = req.selected_binding_ids.clone();
            }
        }

        ModelDependencyRequest {
            node_type: node_type.to_string(),
            model_path: model_path.to_string(),
            model_id,
            model_type: Self::read_optional_input_string_aliases(
                inputs,
                &["model_type", "modelType"],
            ),
            task_type_primary: Some(task_type_primary),
            backend_key,
            platform_context,
            selected_binding_ids,
            dependency_override_patches: Self::read_input_dependency_override_patches(inputs),
        }
    }

    fn dependency_mode(inputs: &HashMap<String, serde_json::Value>) -> String {
        Self::read_optional_input_string_aliases(inputs, &["mode"])
            .map(|mode| mode.trim().to_lowercase())
            .filter(|mode| mode == "auto" || mode == "manual")
            .unwrap_or_else(|| "auto".to_string())
    }

    fn allows_local_python_fallback(status: &ModelDependencyStatus) -> bool {
        if status.state == DependencyState::Unresolved
            && status.code.as_deref() == Some("no_dependency_bindings")
        {
            return true;
        }

        status.state == DependencyState::Missing
            && !status.bindings.is_empty()
            && status.bindings.iter().all(|binding| {
                binding.state == DependencyState::Missing
                    && binding.code.as_deref() == Some("requirements_missing")
                    && binding.failed_requirements.is_empty()
            })
    }

    fn canonical_requirement_fingerprint(
        requirements: &node_engine::ModelDependencyRequirements,
    ) -> String {
        let mut rows = Vec::new();
        let selected = requirements
            .selected_binding_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        for binding in &requirements.bindings {
            if !selected.is_empty() && !selected.contains(&binding.binding_id) {
                continue;
            }
            for req in &binding.requirements {
                rows.push(format!(
                    "{}|{}|{}|{}",
                    binding.binding_id, req.kind, req.name, req.exact_pin
                ));
            }
        }
        rows.sort();
        rows.join(";")
    }

    fn sanitize_key_component(raw: &str) -> String {
        raw.chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>()
    }

    fn dependency_env_store_root() -> PathBuf {
        let base = dirs::data_local_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(std::env::temp_dir);
        base.join("pantograph").join("dependency_envs")
    }

    fn stable_hash_hex(value: &str) -> String {
        let mut digest = Self::FNV64_OFFSET_BASIS;
        for byte in value.as_bytes() {
            digest ^= *byte as u64;
            digest = digest.wrapping_mul(Self::FNV64_PRIME);
        }
        format!("{:016x}", digest)
    }

    fn resolve_environment_ref(
        status: &ModelDependencyStatus,
    ) -> std::result::Result<serde_json::Value, String> {
        let requirements = &status.requirements;
        let selected = if requirements.selected_binding_ids.is_empty() {
            requirements
                .bindings
                .iter()
                .map(|b| b.binding_id.clone())
                .collect::<Vec<_>>()
        } else {
            requirements.selected_binding_ids.clone()
        };

        let env_ids = status
            .bindings
            .iter()
            .filter_map(|row| row.env_id.clone())
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .collect::<Vec<_>>();
        let primary_env_id = env_ids.first().cloned();

        let mut selected_bindings = requirements
            .bindings
            .iter()
            .filter(|binding| selected.contains(&binding.binding_id))
            .collect::<Vec<_>>();
        if selected_bindings.is_empty() {
            selected_bindings = requirements.bindings.iter().collect::<Vec<_>>();
        }

        let environment_kind = selected_bindings
            .iter()
            .find_map(|binding| binding.environment_kind.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let python_override = selected_bindings
            .iter()
            .find_map(|binding| binding.python_executable_override.clone());

        let state_value = serde_json::to_value(&status.state).map_err(|err| {
            format!(
                "Failed to serialize dependency status state for environment_ref: {}",
                err
            )
        })?;
        let state = state_value
            .as_str()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unresolved".to_string());

        let python_executable = if let Some(override_path) = python_override {
            Some(override_path)
        } else if !env_ids.is_empty()
            && (environment_kind == "python" || environment_kind == "python-venv")
        {
            crate::python_runtime::resolve_python_executable_for_env_ids(&env_ids)
                .ok()
                .map(|path| path.to_string_lossy().to_string())
        } else {
            None
        };

        let backend_key = requirements
            .backend_key
            .clone()
            .unwrap_or_else(|| "any".to_string());
        let requirements_fingerprint = Self::canonical_requirement_fingerprint(requirements);
        let key_material = format!(
            "{}|{}|{}|{}",
            primary_env_id.clone().unwrap_or_else(|| "none".to_string()),
            requirements.platform_key,
            backend_key,
            requirements_fingerprint
        );
        let environment_key =
            Self::sanitize_key_component(&format!("v1:{}", Self::stable_hash_hex(&key_material)));

        let manifest_dir = Self::dependency_env_store_root()
            .join(environment_kind.replace(':', "_"))
            .join(&environment_key);
        std::fs::create_dir_all(&manifest_dir).map_err(|err| {
            format!(
                "Failed to create dependency environment manifest directory '{}': {}",
                manifest_dir.display(),
                err
            )
        })?;
        let manifest_path = manifest_dir.join("manifest.json");
        let manifest = serde_json::json!({
            "contract_version": 1,
            "generated_at": Utc::now().to_rfc3339(),
            "environment_key": environment_key,
            "environment_kind": environment_kind,
            "env_id": primary_env_id,
            "env_ids": env_ids,
            "python_executable": python_executable,
            "state": state,
            "requirements_fingerprint": requirements_fingerprint,
            "platform_key": requirements.platform_key,
            "backend_key": requirements.backend_key,
            "selected_binding_ids": requirements.selected_binding_ids,
            "requirements": requirements,
            "status": status,
        });
        std::fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).map_err(|err| {
                format!(
                    "Failed to serialize dependency environment manifest '{}': {}",
                    manifest_path.display(),
                    err
                )
            })?,
        )
        .map_err(|err| {
            format!(
                "Failed to write dependency environment manifest '{}': {}",
                manifest_path.display(),
                err
            )
        })?;

        Ok(serde_json::json!({
            "contract_version": 1,
            "environment_key": environment_key,
            "environment_kind": environment_kind,
            "env_id": manifest["env_id"],
            "env_ids": manifest["env_ids"],
            "python_executable": python_executable,
            "state": state,
            "requirements_fingerprint": requirements_fingerprint,
            "platform_key": requirements.platform_key,
            "backend_key": requirements.backend_key,
            "manifest_path": manifest_path.to_string_lossy().to_string(),
        }))
    }

    async fn execute_dependency_environment(
        &self,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let Some(resolver) = extensions
            .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
        else {
            return Err(NodeEngineError::ExecutionFailed(
                "Dependency environment node requires dependency resolver extension".to_string(),
            ));
        };

        let model_path =
            Self::read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed(
                        "Missing model_path input. Connect Puma-Lib model_path output.".to_string(),
                    )
                })?;
        let mode = Self::dependency_mode(inputs);
        let request =
            Self::build_model_dependency_request("dependency-environment", &model_path, inputs);
        let requirements = resolver
            .resolve_model_dependency_requirements(request.clone())
            .await
            .map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency environment resolve failed: {}",
                    err
                ))
            })?;

        let mut status = resolver
            .check_dependencies(request.clone())
            .await
            .map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency environment check failed: {}",
                    err
                ))
            })?;
        if mode == "auto" && matches!(status.state, DependencyState::Missing) {
            let install = resolver
                .install_dependencies(request)
                .await
                .map_err(|err| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency environment install failed: {}",
                        err
                    ))
                })?;
            status = ModelDependencyStatus {
                state: install.state,
                code: install.code,
                message: install.message,
                requirements: install.requirements,
                bindings: install.bindings,
                checked_at: install.installed_at,
            };
        }

        let ui_state = if mode == "manual"
            && matches!(
                status.state,
                DependencyState::Missing | DependencyState::Unresolved
            ) {
            "needs_user_input".to_string()
        } else {
            serde_json::to_value(&status.state)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unresolved".to_string())
        };
        let environment_ref = Self::resolve_environment_ref(&status).map_err(|err| {
            NodeEngineError::ExecutionFailed(format!(
                "Dependency environment failed to emit environment_ref: {}",
                err
            ))
        })?;

        let mut outputs = HashMap::new();
        outputs.insert("environment_ref".to_string(), environment_ref);
        outputs.insert(
            "dependency_requirements".to_string(),
            serde_json::to_value(&requirements).map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to serialize dependency requirements output: {}",
                    err
                ))
            })?,
        );
        outputs.insert(
            "dependency_status".to_string(),
            serde_json::json!({
                "mode": mode,
                "ui_state": ui_state,
                "state": status.state,
                "code": status.code,
                "message": status.message,
                "checked_at": status.checked_at,
                "requirements": status.requirements,
                "bindings": status.bindings,
            }),
        );
        Ok(outputs)
    }

    async fn enforce_dependency_preflight(
        &self,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<Option<node_engine::ModelRefV2>> {
        if node_type != "pytorch-inference"
            && node_type != "diffusion-inference"
            && node_type != "audio-generation"
            && node_type != "onnx-inference"
        {
            return Ok(None);
        }

        let environment_ref =
            Self::read_optional_input_value_aliases(inputs, &["environment_ref", "environmentRef"]);
        let environment_gate_enabled = environment_ref.is_some();
        if let Some(environment_ref) = &environment_ref {
            let state = environment_ref
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("unresolved");
            if state != "ready" {
                let payload = serde_json::json!({
                    "kind": "environment_ref_gate",
                    "node_type": node_type,
                    "state": state,
                    "environment_ref": environment_ref,
                });
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight blocked execution: {}",
                    payload
                )));
            }
        }

        let Some(resolver) = extensions
            .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
        else {
            if environment_gate_enabled {
                return Ok(None);
            }
            return Err(NodeEngineError::ExecutionFailed(
                "Dependency preflight blocked execution: dependency resolver is not configured"
                    .to_string(),
            ));
        };

        let model_path = inputs
            .get("model_path")
            .and_then(|m| m.as_str())
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(
                    "Missing model_path input. Connect a Puma-Lib node.".to_string(),
                )
            })?;

        let request = Self::build_model_dependency_request(node_type, model_path, inputs);
        if environment_gate_enabled {
            let resolved = resolver
                .resolve_model_ref(request, None)
                .await
                .map_err(|e| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency preflight failed to resolve model_ref from ready environment_ref: {}",
                        e
                    ))
                })?;
            if let Some(ref model_ref) = resolved {
                model_ref
                    .validate()
                    .map_err(NodeEngineError::ExecutionFailed)?;
            }
            return Ok(resolved);
        }

        let requirements = resolver
            .resolve_model_dependency_requirements(request.clone())
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight requirements resolution failed for '{}': {}",
                    node_type, e
                ))
            })?;

        let status = resolver
            .check_dependencies(request.clone())
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight check failed for '{}': {}",
                    node_type, e
                ))
            })?;

        if Self::allows_local_python_fallback(&status) {
            let resolved = resolver.resolve_model_ref(request, Some(requirements)).await.map_err(
                |e| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Dependency preflight failed to resolve model_ref for local Python fallback: {}",
                        e
                    ))
                },
            )?;
            if let Some(ref model_ref) = resolved {
                model_ref
                    .validate()
                    .map_err(NodeEngineError::ExecutionFailed)?;
            }
            return Ok(resolved);
        }

        if status.state != DependencyState::Ready {
            let payload = serde_json::json!({
                "kind": "dependency_preflight",
                "node_type": node_type,
                "model_path": model_path,
                "validation_state": requirements.validation_state,
                "validation_errors": requirements.validation_errors,
                "selected_binding_ids": requirements.selected_binding_ids,
                "state": status.state,
                "code": status.code,
                "bindings": status.bindings,
                "message": status.message,
            });
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Dependency preflight blocked execution: {}",
                payload
            )));
        }

        let resolved = resolver
            .resolve_model_ref(request, Some(requirements))
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Dependency preflight failed to resolve model_ref: {}",
                    e
                ))
            })?;
        if let Some(ref model_ref) = resolved {
            model_ref
                .validate()
                .map_err(NodeEngineError::ExecutionFailed)?;
        }

        Ok(resolved)
    }

    async fn execute_python_node(
        &self,
        task_id: &str,
        node_type: &str,
        inputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut runtime_inputs = inputs.clone();
        Self::apply_inference_setting_defaults(&mut runtime_inputs);
        Self::promote_runtime_metadata(&mut runtime_inputs);
        if let Some(model_ref) = self
            .enforce_dependency_preflight(node_type, inputs, extensions)
            .await?
        {
            let value = serde_json::to_value(model_ref).map_err(|err| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to serialize resolved model_ref for python runtime adapter: {}",
                    err
                ))
            })?;
            runtime_inputs.insert("model_ref".to_string(), value);
        }

        let request = PythonNodeExecutionRequest {
            node_type: node_type.to_string(),
            inputs: runtime_inputs.clone(),
            env_ids: Self::collect_runtime_env_ids(&runtime_inputs),
        };
        let recorder = Self::python_runtime_recorder(extensions);
        let mut runtime_metadata =
            Self::python_runtime_execution_metadata(node_type, &request, false);
        runtime_metadata.snapshot.lifecycle_decision_reason = runtime_metadata
            .snapshot
            .normalized_lifecycle_decision_reason();

        let streamed_any = Arc::new(AtomicBool::new(false));
        let stream_handler: Option<PythonStreamHandler> = Self::resolve_stream_target(extensions)
            .map(|(sink, execution_id)| {
                let streamed_any = streamed_any.clone();
                let task_id = task_id.to_string();
                Arc::new(move |chunk: serde_json::Value| {
                    streamed_any.store(true, Ordering::Relaxed);
                    let _ = sink.send(WorkflowEvent::task_stream(
                        &task_id,
                        &execution_id,
                        "stream",
                        chunk,
                    ));
                }) as PythonStreamHandler
            });

        let outputs = self
            .python_runtime
            .execute_node_with_stream(request, stream_handler)
            .await
            .map_err(|error| {
                if let Some(recorder) = recorder.as_ref() {
                    let mut failed = runtime_metadata.clone();
                    let previous_failures = recorder.previous_consecutive_failures(
                        failed.snapshot.runtime_instance_id.as_deref(),
                    );
                    failed.snapshot.active = false;
                    failed.snapshot.last_error = Some(error.clone());
                    failed.snapshot.lifecycle_decision_reason =
                        failed.snapshot.normalized_lifecycle_decision_reason();
                    failed.health_assessment = Some(failed_runtime_health_assessment(
                        error.clone(),
                        previous_failures,
                        Self::PYTHON_RUNTIME_FAILURE_THRESHOLD,
                    ));
                    recorder.record(failed);
                }
                NodeEngineError::ExecutionFailed(error)
            })?;
        if let Some(recorder) = recorder.as_ref() {
            runtime_metadata.snapshot.active = false;
            recorder.record(runtime_metadata);
        }
        if !streamed_any.load(Ordering::Relaxed) && Self::supports_buffered_stream_replay(node_type)
        {
            Self::emit_python_stream_events(task_id, &outputs, extensions);
        }
        Ok(outputs)
    }

    fn supports_buffered_stream_replay(node_type: &str) -> bool {
        node_type != "audio-generation"
    }

    fn resolve_stream_target(
        extensions: &ExecutorExtensions,
    ) -> Option<(Arc<dyn EventSink>, String)> {
        let sink = extensions
            .get::<Arc<dyn EventSink>>(runtime_extension_keys::EVENT_SINK)?
            .clone();
        let execution_id = extensions
            .get::<String>(runtime_extension_keys::EXECUTION_ID)
            .cloned()
            .unwrap_or_default();
        Some((sink, execution_id))
    }

    fn emit_python_stream_events(
        task_id: &str,
        outputs: &HashMap<String, serde_json::Value>,
        extensions: &ExecutorExtensions,
    ) {
        let Some(stream_payload) = outputs.get("stream") else {
            return;
        };
        let Some((sink, execution_id)) = Self::resolve_stream_target(extensions) else {
            return;
        };

        let send_stream = |chunk: serde_json::Value| {
            let _ = sink.send(WorkflowEvent::task_stream(
                task_id,
                &execution_id,
                "stream",
                chunk,
            ));
        };

        match stream_payload {
            serde_json::Value::Null => {}
            serde_json::Value::Array(items) => {
                for item in items {
                    send_stream(item.clone());
                }
            }
            other => send_stream(other.clone()),
        }
    }
}

#[async_trait]
impl TaskExecutor for TauriTaskExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        extensions: &node_engine::ExecutorExtensions,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let node_type = resolve_node_type(task_id, &inputs);

        match node_type.as_str() {
            "rag-search" => self.execute_rag_search(&inputs).await,
            "puma-lib" => self.execute_puma_lib(&inputs, extensions).await,
            "dependency-environment" => {
                self.execute_dependency_environment(&inputs, extensions)
                    .await
            }
            "pytorch-inference" | "diffusion-inference" | "audio-generation" | "onnx-inference" => {
                self.execute_python_node(task_id, &node_type, &inputs, extensions)
                    .await
            }
            _ => {
                // Signal to CompositeTaskExecutor that this node type
                // requires host-specific executor (i.e., fall through to core)
                Err(NodeEngineError::ExecutionFailed(format!(
                    "Node type '{}' requires host-specific executor",
                    node_type
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use node_engine::{
        DependencyState, DependencyValidationState, ExecutorExtensions, ModelDependencyBinding,
        ModelDependencyBindingStatus, ModelDependencyInstallResult, ModelDependencyRequest,
        ModelDependencyRequirements, ModelDependencyResolver, ModelDependencyStatus, ModelRefV2,
        VecEventSink, WorkflowEvent, extension_keys,
    };
    use std::sync::Mutex;

    #[test]
    fn canonical_backend_key_accepts_llama_cpp_alias() {
        assert_eq!(
            TauriTaskExecutor::canonical_backend_key(Some("llama_cpp")),
            Some("llamacpp".to_string())
        );
    }

    #[derive(Clone)]
    struct StubDependencyResolver {
        requirements: ModelDependencyRequirements,
        status: ModelDependencyStatus,
        model_ref: Option<ModelRefV2>,
    }

    #[async_trait]
    impl ModelDependencyResolver for StubDependencyResolver {
        async fn resolve_model_dependency_requirements(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyRequirements, String> {
            Ok(self.requirements.clone())
        }

        async fn check_dependencies(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyStatus, String> {
            Ok(self.status.clone())
        }

        async fn install_dependencies(
            &self,
            _request: ModelDependencyRequest,
        ) -> std::result::Result<ModelDependencyInstallResult, String> {
            Err("install not used in task-executor tests".to_string())
        }

        async fn resolve_model_ref(
            &self,
            _request: ModelDependencyRequest,
            _requirements: Option<ModelDependencyRequirements>,
        ) -> std::result::Result<Option<ModelRefV2>, String> {
            Ok(self.model_ref.clone())
        }
    }

    struct RecordingPythonAdapter {
        requests: Arc<Mutex<Vec<PythonNodeExecutionRequest>>>,
        response: HashMap<String, serde_json::Value>,
    }

    #[async_trait]
    impl PythonRuntimeAdapter for RecordingPythonAdapter {
        async fn execute_node(
            &self,
            request: PythonNodeExecutionRequest,
        ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
            self.requests.lock().expect("recording lock").push(request);
            Ok(self.response.clone())
        }
    }

    fn test_executor(
        adapter: Arc<dyn PythonRuntimeAdapter>,
        resolver: Arc<dyn ModelDependencyResolver>,
    ) -> (TauriTaskExecutor, ExecutorExtensions) {
        let executor = TauriTaskExecutor::with_python_runtime(None, adapter);

        let mut extensions = ExecutorExtensions::new();
        extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
        (executor, extensions)
    }

    fn install_python_runtime_recorder(
        extensions: &mut ExecutorExtensions,
    ) -> Arc<PythonRuntimeExecutionRecorder> {
        let recorder = Arc::new(PythonRuntimeExecutionRecorder::default());
        extensions.set(
            runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
            recorder.clone(),
        );
        recorder
    }

    fn make_requirements(state: DependencyValidationState) -> ModelDependencyRequirements {
        ModelDependencyRequirements {
            model_id: "model-a".to_string(),
            platform_key: "linux-x86_64".to_string(),
            backend_key: Some("pytorch".to_string()),
            dependency_contract_version: 1,
            validation_state: state,
            validation_errors: Vec::new(),
            bindings: Vec::new(),
            selected_binding_ids: Vec::new(),
        }
    }

    fn make_status(state: DependencyState, code: Option<&str>) -> ModelDependencyStatus {
        ModelDependencyStatus {
            state,
            code: code.map(|s| s.to_string()),
            message: code.map(|s| format!("status={}", s)),
            requirements: make_requirements(DependencyValidationState::Resolved),
            bindings: Vec::new(),
            checked_at: None,
        }
    }

    fn make_missing_binding_status(binding_code: &str) -> ModelDependencyStatus {
        ModelDependencyStatus {
            state: DependencyState::Missing,
            code: None,
            message: None,
            requirements: make_requirements(DependencyValidationState::Resolved),
            bindings: vec![ModelDependencyBindingStatus {
                binding_id: "binding-a".to_string(),
                env_id: Some("python-venv:test".to_string()),
                state: DependencyState::Missing,
                code: Some(binding_code.to_string()),
                message: None,
                missing_requirements: vec!["diffusers".to_string()],
                installed_requirements: Vec::new(),
                failed_requirements: Vec::new(),
            }],
            checked_at: None,
        }
    }

    fn create_test_env() -> tempfile::TempDir {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
        temp_dir
    }

    fn write_test_diffusers_bundle(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("scheduler")).unwrap();
        std::fs::create_dir_all(root.join("text_encoder")).unwrap();
        std::fs::create_dir_all(root.join("tokenizer")).unwrap();
        std::fs::create_dir_all(root.join("unet")).unwrap();
        std::fs::create_dir_all(root.join("vae")).unwrap();
        std::fs::write(
            root.join("model_index.json"),
            serde_json::json!({
                "_class_name": "StableDiffusionPipeline",
                "scheduler": ["diffusers", "EulerDiscreteScheduler"],
                "text_encoder": ["transformers", "CLIPTextModel"],
                "tokenizer": ["transformers", "CLIPTokenizer"],
                "unet": ["diffusers", "UNet2DConditionModel"],
                "vae": ["diffusers", "AutoencoderKL"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_imported_diffusion_metadata(
        model_dir: &std::path::Path,
        model_id: &str,
        entry_path: &std::path::Path,
    ) {
        std::fs::create_dir_all(model_dir).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": model_id,
                "family": "imported",
                "model_type": "diffusion",
                "official_name": "test-bundle",
                "cleaned_name": "test-bundle",
                "source_path": entry_path.display().to_string(),
                "entry_path": entry_path.display().to_string(),
                "storage_kind": "external_reference",
                "bundle_format": "diffusers_directory",
                "pipeline_class": "StableDiffusionPipeline",
                "import_state": "ready",
                "validation_state": "valid",
                "pipeline_tag": "text-to-image",
                "task_type_primary": "text-to-image",
                "input_modalities": ["text"],
                "output_modalities": ["image"],
                "task_classification_source": "external-diffusers-import",
                "task_classification_confidence": 1.0,
                "model_type_resolution_source": "external-diffusers-import",
                "model_type_resolution_confidence": 1.0,
                "recommended_backend": "diffusers",
                "runtime_engine_hints": ["diffusers", "pytorch"]
            })
            .to_string(),
        )
        .unwrap();
    }

    #[tokio::test]
    async fn python_nodes_block_when_dependency_preflight_is_not_ready() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: HashMap::new(),
        });
        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::InvalidProfile),
            status: make_status(DependencyState::Invalid, Some("invalid_profile")),
            model_ref: None,
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model-not-ready"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        let err = executor
            .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
            .await
            .expect_err("preflight should block non-ready dependency state");

        match err {
            NodeEngineError::ExecutionFailed(message) => {
                assert!(message.contains("Dependency preflight blocked execution"));
                assert!(message.contains("invalid_profile"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert_eq!(requests.lock().expect("recording lock").len(), 0);
    }

    #[tokio::test]
    async fn python_nodes_receive_resolved_model_ref_and_env_ids_after_preflight() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("response".to_string(), serde_json::json!("ok"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "pytorch".to_string(),
            model_id: "model-a".to_string(),
            model_path: "/tmp/model-ready".to_string(),
            task_type_primary: "text-generation".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-a".to_string(),
                profile_id: "profile-a".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("venv:test".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            }],
            dependency_requirements_id: Some("requirements-test".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model-ready"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        let outputs = executor
            .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
            .await
            .expect("ready preflight should allow adapter execution");
        assert_eq!(outputs.get("response"), Some(&serde_json::json!("ok")));

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "pytorch-inference");
        assert_eq!(request.env_ids, vec!["venv:test".to_string()]);
        assert!(request.inputs.contains_key("model_ref"));
    }

    #[tokio::test]
    async fn diffusion_nodes_route_through_python_adapter_with_preflight() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
        adapter_response.insert("seed_used".to_string(), serde_json::json!(1234));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "pytorch".to_string(),
            model_id: "qwen-image".to_string(),
            model_path: "/tmp/qwen-image".to_string(),
            task_type_primary: "text-to-image".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-diffusion".to_string(),
                profile_id: "profile-diffusion".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                backend_key: Some("pytorch".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("venv:diffusion".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            }],
            dependency_requirements_id: Some("requirements-diffusion".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/qwen-image"),
        );
        inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
        inputs.insert(
            "prompt".to_string(),
            serde_json::json!("paper lantern in the rain"),
        );

        let outputs = executor
            .execute_task(
                "diffusion-inference-1",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("diffusion preflight should allow adapter execution");
        assert_eq!(
            outputs.get("image"),
            Some(&serde_json::json!("base64-image"))
        );
        assert_eq!(outputs.get("seed_used"), Some(&serde_json::json!(1234)));

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "diffusion-inference");
        assert_eq!(request.env_ids, vec!["venv:diffusion".to_string()]);
        assert_eq!(
            request
                .inputs
                .get("model_ref")
                .and_then(|value| value.get("taskTypePrimary"))
                .and_then(|value| value.as_str()),
            Some("text-to-image")
        );
        assert!(request.inputs.contains_key("model_ref"));
    }

    #[tokio::test]
    async fn onnx_nodes_route_through_python_adapter_with_preflight() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "onnx-runtime".to_string(),
            model_id: "kitten-tts".to_string(),
            model_path: "/tmp/model.onnx".to_string(),
            task_type_primary: "text-to-audio".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-onnx".to_string(),
                profile_id: "profile-onnx".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                backend_key: Some("onnx-runtime".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("venv:onnx".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            }],
            dependency_requirements_id: Some("requirements-onnx".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("onnx-runtime".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        let outputs = executor
            .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
            .await
            .expect("onnx preflight should allow adapter execution");
        assert_eq!(
            outputs.get("audio"),
            Some(&serde_json::json!("base64-audio"))
        );

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "onnx-inference");
        assert_eq!(request.env_ids, vec!["venv:onnx".to_string()]);
        assert!(request.inputs.contains_key("model_ref"));
    }

    #[tokio::test]
    async fn python_runtime_recorder_tracks_backend_and_environment_identity() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests,
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "onnx-runtime".to_string(),
            model_id: "kitten-tts".to_string(),
            model_path: "/tmp/model.onnx".to_string(),
            task_type_primary: "text-to-audio".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-onnx".to_string(),
                profile_id: "profile-onnx".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                backend_key: Some("onnx-runtime".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("venv:onnx".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            }],
            dependency_requirements_id: Some("requirements-onnx".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("onnx-runtime".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, mut extensions) = test_executor(adapter, resolver);
        let recorder = install_python_runtime_recorder(&mut extensions);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        );
        inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

        executor
            .execute_task("onnx-inference-1", inputs, &Context::new(), &extensions)
            .await
            .expect("onnx execution should succeed");

        let metadata = recorder.snapshot().expect("python runtime metadata");
        assert_eq!(
            metadata.snapshot.runtime_id.as_deref(),
            Some("onnx-runtime")
        );
        assert_eq!(
            metadata.snapshot.runtime_instance_id.as_deref(),
            Some("python-runtime:onnx-runtime:venv_onnx")
        );
        assert_eq!(metadata.snapshot.runtime_reused, Some(false));
        assert_eq!(
            metadata.snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
        assert!(!metadata.snapshot.active);
        assert_eq!(metadata.model_target.as_deref(), Some("/tmp/model.onnx"));
        assert_eq!(metadata.health_assessment, None);
    }

    #[tokio::test]
    async fn python_runtime_recorder_keeps_process_runtime_non_reused_across_runs() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests,
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "onnx-runtime".to_string(),
            model_id: "kitten-tts".to_string(),
            model_path: "/tmp/model.onnx".to_string(),
            task_type_primary: "text-to-audio".to_string(),
            dependency_bindings: vec![ModelDependencyBinding {
                binding_id: "binding-onnx".to_string(),
                profile_id: "profile-onnx".to_string(),
                profile_version: 1,
                profile_hash: Some("hash".to_string()),
                backend_key: Some("onnx-runtime".to_string()),
                platform_selector: Some("linux-x86_64".to_string()),
                environment_kind: Some("python".to_string()),
                env_id: Some("venv:onnx".to_string()),
                python_executable_override: None,
                validation_state: DependencyValidationState::Resolved,
                validation_errors: Vec::new(),
                requirements: Vec::new(),
            }],
            dependency_requirements_id: Some("requirements-onnx".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("onnx-runtime".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, mut extensions) = test_executor(adapter, resolver);
        let recorder = install_python_runtime_recorder(&mut extensions);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        );
        inputs.insert("backend_key".to_string(), serde_json::json!("onnxruntime"));

        executor
            .execute_task(
                "onnx-inference-1",
                inputs.clone(),
                &Context::new(),
                &extensions,
            )
            .await
            .expect("first onnx execution should succeed");
        executor
            .execute_task("onnx-inference-2", inputs, &Context::new(), &extensions)
            .await
            .expect("second onnx execution should succeed");

        let metadata = recorder.snapshot().expect("python runtime metadata");
        assert_eq!(metadata.snapshot.runtime_reused, Some(false));
        assert_eq!(
            metadata.snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
        assert!(!metadata.snapshot.active);
        assert_eq!(metadata.health_assessment, None);
    }

    struct FailingPythonAdapter;

    #[async_trait]
    impl PythonRuntimeAdapter for FailingPythonAdapter {
        async fn execute_node(
            &self,
            _request: PythonNodeExecutionRequest,
        ) -> std::result::Result<HashMap<String, serde_json::Value>, String> {
            Err("python sidecar crashed".to_string())
        }
    }

    #[tokio::test]
    async fn python_runtime_recorder_progresses_failed_execution_health_state() {
        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("pytorch".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let executor = TauriTaskExecutor::with_python_runtime(None, Arc::new(FailingPythonAdapter));
        let mut extensions = ExecutorExtensions::new();
        extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);
        let recorder = install_python_runtime_recorder(&mut extensions);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.safetensors"),
        );
        inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));

        for _ in 0..3 {
            let error = executor
                .execute_task(
                    "pytorch-inference-1",
                    inputs.clone(),
                    &Context::new(),
                    &extensions,
                )
                .await
                .expect_err("python execution should fail");

            match error {
                NodeEngineError::ExecutionFailed(message) => {
                    assert!(message.contains("python sidecar crashed"));
                }
                other => panic!("unexpected error variant: {other:?}"),
            }
        }

        let snapshots = recorder.snapshots();
        assert_eq!(snapshots.len(), 3);

        let first_assessment = snapshots[0]
            .health_assessment
            .clone()
            .expect("first failed execution health assessment");
        assert!(first_assessment.healthy);
        assert_eq!(first_assessment.consecutive_failures, 1);
        assert_eq!(
            first_assessment.state,
            crate::runtime_health::RuntimeHealthState::Degraded {
                reason: "python sidecar crashed".to_string(),
            }
        );

        let second_assessment = snapshots[1]
            .health_assessment
            .clone()
            .expect("second failed execution health assessment");
        assert!(second_assessment.healthy);
        assert_eq!(second_assessment.consecutive_failures, 2);
        assert_eq!(
            second_assessment.state,
            crate::runtime_health::RuntimeHealthState::Degraded {
                reason: "python sidecar crashed".to_string(),
            }
        );

        let third = snapshots.last().expect("third runtime metadata");
        assert!(!third.snapshot.active);
        assert_eq!(
            third.snapshot.last_error.as_deref(),
            Some("python sidecar crashed")
        );
        let third_assessment = third
            .health_assessment
            .clone()
            .expect("third failed execution health assessment");
        assert!(!third_assessment.healthy);
        assert_eq!(
            third_assessment.error.as_deref(),
            Some("python sidecar crashed")
        );
        assert_eq!(third_assessment.consecutive_failures, 3);
        assert_eq!(
            third_assessment.state,
            crate::runtime_health::RuntimeHealthState::Unhealthy {
                reason: "python sidecar crashed".to_string(),
            }
        );
    }

    #[tokio::test]
    async fn onnx_nodes_apply_inference_setting_defaults_before_python_execution() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("base64-audio"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("onnx-runtime".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));
        inputs.insert(
            "inference_settings".to_string(),
            serde_json::json!([
                {"key": "voice", "default": "expr-voice-5-m"},
                {"key": "speed", "default": 0.9},
                {"key": "clean_text", "default": true},
                {"key": "sample_rate", "default": 24000}
            ]),
        );

        let _ = executor
            .execute_task(
                "onnx-inference-defaults",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("onnx execution with inference defaults should succeed");

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(
            request.inputs.get("voice"),
            Some(&serde_json::json!("expr-voice-5-m"))
        );
        assert_eq!(request.inputs.get("speed"), Some(&serde_json::json!(0.9)));
        assert_eq!(
            request.inputs.get("clean_text"),
            Some(&serde_json::json!(true))
        );
        assert_eq!(
            request.inputs.get("sample_rate"),
            Some(&serde_json::json!(24000))
        );
    }

    #[tokio::test]
    async fn python_nodes_emit_stream_events_when_event_sink_extension_exists() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
        adapter_response.insert(
            "stream".to_string(),
            serde_json::json!([
                {
                    "type": "audio_chunk",
                    "mode": "append",
                    "audio_base64": "chunk-1",
                    "mime_type": "audio/wav",
                    "sequence": 0,
                    "is_final": false
                },
                {
                    "type": "audio_chunk",
                    "mode": "append",
                    "audio_base64": "chunk-2",
                    "mime_type": "audio/wav",
                    "sequence": 1,
                    "is_final": true
                }
            ]),
        );
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("onnx-runtime".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let (executor, mut extensions) = test_executor(adapter, resolver);
        let sink = Arc::new(VecEventSink::new());
        extensions.set(
            runtime_extension_keys::EVENT_SINK,
            sink.clone() as Arc<dyn node_engine::EventSink>,
        );
        extensions.set(
            runtime_extension_keys::EXECUTION_ID,
            "exec-stream-test".to_string(),
        );

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model.onnx"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("stream this"));

        let _ = executor
            .execute_task(
                "onnx-inference-stream",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("onnx stream execution should succeed");

        let events = sink.events();
        let stream_events: Vec<_> = events
            .into_iter()
            .filter_map(|event| match event {
                WorkflowEvent::TaskStream {
                    task_id,
                    execution_id,
                    port,
                    data,
                    ..
                } => Some((task_id, execution_id, port, data)),
                _ => None,
            })
            .collect();

        assert_eq!(stream_events.len(), 2);
        assert_eq!(stream_events[0].0, "onnx-inference-stream");
        assert_eq!(stream_events[0].1, "exec-stream-test");
        assert_eq!(stream_events[0].2, "stream");
        assert_eq!(stream_events[0].3["audio_base64"], "chunk-1");
        assert_eq!(stream_events[0].3["sequence"], 0);
        assert_eq!(stream_events[0].3["is_final"], false);
        assert_eq!(stream_events[1].3["audio_base64"], "chunk-2");
        assert_eq!(stream_events[1].3["sequence"], 1);
        assert_eq!(stream_events[1].3["is_final"], true);
    }

    #[tokio::test]
    async fn audio_generation_nodes_do_not_emit_buffered_stream_events_after_completion() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("audio".to_string(), serde_json::json!("final-audio"));
        adapter_response.insert(
            "stream".to_string(),
            serde_json::json!([
                {
                    "type": "audio_chunk",
                    "mode": "append",
                    "audio_base64": "chunk-1",
                    "mime_type": "audio/wav",
                    "sequence": 0,
                    "is_final": false
                }
            ]),
        );
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests,
            response: adapter_response,
        });

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: ModelDependencyRequirements {
                backend_key: Some("stable_audio".to_string()),
                ..make_requirements(DependencyValidationState::Resolved)
            },
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let (executor, mut extensions) = test_executor(adapter, resolver);
        let sink = Arc::new(VecEventSink::new());
        extensions.set(
            runtime_extension_keys::EVENT_SINK,
            sink.clone() as Arc<dyn node_engine::EventSink>,
        );
        extensions.set(
            runtime_extension_keys::EXECUTION_ID,
            "exec-audio-batch-test".to_string(),
        );

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/stable-audio"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("pad ambience"));

        let outputs = executor
            .execute_task(
                "audio-generation-batch",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("audio-generation execution should succeed");

        assert_eq!(
            outputs.get("audio"),
            Some(&serde_json::json!("final-audio"))
        );
        let stream_events: Vec<_> = sink
            .events()
            .into_iter()
            .filter(|event| matches!(event, WorkflowEvent::TaskStream { .. }))
            .collect();
        assert!(stream_events.is_empty());
    }

    #[test]
    fn apply_inference_setting_defaults_preserves_explicit_values() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "inference_settings".to_string(),
            serde_json::json!([
                {"key": "voice", "default": "expr-voice-5-m"},
                {"key": "speed", "default": 1.0}
            ]),
        );
        inputs.insert("voice".to_string(), serde_json::json!("custom-voice"));
        inputs.insert("speed".to_string(), serde_json::Value::Null);

        TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

        assert_eq!(
            inputs.get("voice"),
            Some(&serde_json::json!("custom-voice"))
        );
        assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.0)));
    }

    #[test]
    fn apply_inference_setting_defaults_resolves_option_object_values() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "inference_settings".to_string(),
            serde_json::json!([
                {"key": "voice", "default": {"label": "Leo", "value": "expr-voice-5-m"}},
                {"key": "speed", "default": 1.0}
            ]),
        );
        inputs.insert(
            "speed".to_string(),
            serde_json::json!({"label": "Fast", "value": 1.2}),
        );

        TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

        assert_eq!(
            inputs.get("voice"),
            Some(&serde_json::json!("expr-voice-5-m"))
        );
        assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
    }

    #[test]
    fn apply_inference_setting_defaults_resolves_allowed_value_labels() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "inference_settings".to_string(),
            serde_json::json!([
                {
                    "key": "voice",
                    "default": "Leo",
                    "constraints": {
                        "allowed_values": [
                            {"label": "Leo", "value": "expr-voice-5-m"}
                        ]
                    }
                },
                {"key": "speed", "default": 1.0}
            ]),
        );
        inputs.insert("speed".to_string(), serde_json::json!(1.2));

        TauriTaskExecutor::apply_inference_setting_defaults(&mut inputs);

        assert_eq!(
            inputs.get("voice"),
            Some(&serde_json::json!("expr-voice-5-m"))
        );
        assert_eq!(inputs.get("speed"), Some(&serde_json::json!(1.2)));
    }

    #[test]
    fn collect_runtime_env_ids_includes_environment_ref() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "environment_ref".to_string(),
            serde_json::json!({
                "state": "ready",
                "env_id": "env:primary",
                "env_ids": ["env:extra"]
            }),
        );
        inputs.insert(
            "model_ref".to_string(),
            serde_json::json!({
                "dependencyBindings": [
                    {"envId": "env:primary"},
                    {"envId": "env:secondary"}
                ]
            }),
        );

        let env_ids = TauriTaskExecutor::collect_runtime_env_ids(&inputs);
        assert_eq!(
            env_ids,
            vec![
                "env:extra".to_string(),
                "env:primary".to_string(),
                "env:secondary".to_string(),
            ]
        );
    }

    #[test]
    fn stable_hash_hex_is_deterministic() {
        let one = TauriTaskExecutor::stable_hash_hex("abc|123");
        let two = TauriTaskExecutor::stable_hash_hex("abc|123");
        let three = TauriTaskExecutor::stable_hash_hex("abc|124");
        assert_eq!(one, two);
        assert_ne!(one, three);
        assert_eq!(one.len(), 16);
    }

    #[test]
    fn build_model_dependency_request_normalizes_backend_aliases() {
        let mut inputs = HashMap::new();
        inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

        let request = TauriTaskExecutor::build_model_dependency_request(
            "pytorch-inference",
            "/tmp/model",
            &inputs,
        );
        assert_eq!(request.backend_key.as_deref(), Some("onnx-runtime"));
    }

    #[test]
    fn build_model_dependency_request_prefers_requirements_backend_when_input_missing() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "dependency_requirements".to_string(),
            serde_json::json!({
                "model_id": "model-a",
                "platform_key": "linux-x86_64",
                "backend_key": "torch",
                "dependency_contract_version": 1,
                "validation_state": "resolved",
                "validation_errors": [],
                "bindings": [],
                "selected_binding_ids": []
            }),
        );

        let request = TauriTaskExecutor::build_model_dependency_request(
            "pytorch-inference",
            "/tmp/model",
            &inputs,
        );
        assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
    }

    #[test]
    fn build_model_dependency_request_prefers_recommended_backend_for_diffusion() {
        let mut inputs = HashMap::new();
        inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
        inputs.insert(
            "recommended_backend".to_string(),
            serde_json::json!("diffusers"),
        );

        let request = TauriTaskExecutor::build_model_dependency_request(
            "diffusion-inference",
            "/tmp/model",
            &inputs,
        );
        assert_eq!(request.backend_key.as_deref(), Some("diffusers"));
    }

    #[test]
    fn build_model_dependency_request_leaves_diffusion_backend_unspecified_by_default() {
        let mut inputs = HashMap::new();
        inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));

        let request = TauriTaskExecutor::build_model_dependency_request(
            "diffusion-inference",
            "/tmp/model",
            &inputs,
        );
        assert_eq!(request.backend_key, None);
    }

    #[tokio::test]
    async fn python_nodes_fail_fast_when_environment_ref_is_not_ready() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: HashMap::new(),
        });
        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/model-ready"),
        );
        inputs.insert("prompt".to_string(), serde_json::json!("hello"));
        inputs.insert(
            "environment_ref".to_string(),
            serde_json::json!({
                "state": "missing",
                "env_id": "env:test"
            }),
        );

        let err = executor
            .execute_task("pytorch-inference-1", inputs, &Context::new(), &extensions)
            .await
            .expect_err("preflight should block when environment_ref state is not ready");

        match err {
            NodeEngineError::ExecutionFailed(message) => {
                assert!(message.contains("environment_ref_gate"));
                assert!(message.contains("missing"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert_eq!(requests.lock().expect("recording lock").len(), 0);
    }

    #[tokio::test]
    async fn python_nodes_allow_execution_when_no_dependency_bindings_are_available() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "pytorch".to_string(),
            model_id: "diffusion/imported/tiny-sd-turbo".to_string(),
            model_path: "/tmp/external/tiny-sd-turbo".to_string(),
            task_type_primary: "text-to-image".to_string(),
            dependency_bindings: Vec::new(),
            dependency_requirements_id: Some("requirements-diffusion".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_status(DependencyState::Unresolved, Some("no_dependency_bindings")),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/external/tiny-sd-turbo"),
        );
        inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
        inputs.insert(
            "prompt".to_string(),
            serde_json::json!("paper lantern in the rain"),
        );

        let outputs = executor
            .execute_task(
                "diffusion-inference-2",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("python nodes should execute without dependency bindings");
        assert_eq!(
            outputs.get("image"),
            Some(&serde_json::json!("base64-image"))
        );

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "diffusion-inference");
        assert!(request.env_ids.is_empty());
        assert_eq!(
            request
                .inputs
                .get("model_ref")
                .and_then(|value| value.get("modelPath"))
                .and_then(|value| value.as_str()),
            Some("/tmp/external/tiny-sd-turbo")
        );
    }

    #[tokio::test]
    async fn python_nodes_allow_execution_when_bindings_are_missing_only_runtime_packages() {
        let requests = Arc::new(Mutex::new(Vec::<PythonNodeExecutionRequest>::new()));
        let mut adapter_response = HashMap::new();
        adapter_response.insert("image".to_string(), serde_json::json!("base64-image"));
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: requests.clone(),
            response: adapter_response,
        });

        let resolved_model_ref = ModelRefV2 {
            contract_version: 2,
            engine: "diffusers".to_string(),
            model_id: "diffusion/cc-nms/tiny-sd-turbo".to_string(),
            model_path: "/tmp/external/tiny-sd-turbo".to_string(),
            task_type_primary: "text-to-image".to_string(),
            dependency_bindings: Vec::new(),
            dependency_requirements_id: Some("requirements-diffusion".to_string()),
        };

        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_missing_binding_status("requirements_missing"),
            model_ref: Some(resolved_model_ref),
        });
        let (executor, extensions) = test_executor(adapter, resolver);

        let mut inputs = HashMap::new();
        inputs.insert(
            "model_path".to_string(),
            serde_json::json!("/tmp/external/tiny-sd-turbo"),
        );
        inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));
        inputs.insert(
            "prompt".to_string(),
            serde_json::json!("paper lantern in the rain"),
        );

        let outputs = executor
            .execute_task(
                "diffusion-inference-3",
                inputs,
                &Context::new(),
                &extensions,
            )
            .await
            .expect("python nodes should execute when only runtime packages are missing");
        assert_eq!(
            outputs.get("image"),
            Some(&serde_json::json!("base64-image"))
        );

        let recorded = requests.lock().expect("recording lock");
        assert_eq!(recorded.len(), 1);
        let request = &recorded[0];
        assert_eq!(request.node_type, "diffusion-inference");
        assert!(request.env_ids.is_empty());
        assert_eq!(
            request
                .inputs
                .get("model_ref")
                .and_then(|value| value.get("engine"))
                .and_then(|value| value.as_str()),
            Some("diffusers")
        );
    }

    #[tokio::test]
    async fn puma_lib_execution_rebinds_stale_model_path_from_model_id() {
        let adapter: Arc<dyn PythonRuntimeAdapter> = Arc::new(RecordingPythonAdapter {
            requests: Arc::new(Mutex::new(Vec::new())),
            response: HashMap::new(),
        });
        let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(StubDependencyResolver {
            requirements: make_requirements(DependencyValidationState::Resolved),
            status: make_status(DependencyState::Ready, None),
            model_ref: None,
        });
        let (executor, mut extensions) = test_executor(adapter, resolver);

        let temp_dir = create_test_env();
        let bundle_root = temp_dir.path().join("external/tiny-sd-turbo");
        write_test_diffusers_bundle(&bundle_root);
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models/diffusion/imported/test-bundle");
        write_imported_diffusion_metadata(
            &model_dir,
            "diffusion/imported/test-bundle",
            &bundle_root,
        );

        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .build()
                .await
                .expect("pumas api should initialize"),
        );
        extensions.set(extension_keys::PUMAS_API, api);

        let mut inputs = HashMap::new();
        inputs.insert(
            "_data".to_string(),
            serde_json::json!({
                "modelPath": "/stale/location/tiny-sd-turbo",
                "model_id": "diffusion/imported/test-bundle",
                "model_type": "diffusion",
                "task_type_primary": "text-to-image",
                "recommended_backend": "diffusers",
                "inference_settings": []
            }),
        );

        let outputs = executor
            .execute_task("puma-lib-1", inputs, &Context::new(), &extensions)
            .await
            .expect("puma-lib should resolve runtime path");

        assert_eq!(
            outputs.get("model_path"),
            Some(&serde_json::json!(bundle_root.display().to_string()))
        );
        assert_eq!(
            outputs.get("model_id"),
            Some(&serde_json::json!("diffusion/imported/test-bundle"))
        );
        assert_eq!(
            outputs.get("recommended_backend"),
            Some(&serde_json::json!("diffusers"))
        );
    }
}
