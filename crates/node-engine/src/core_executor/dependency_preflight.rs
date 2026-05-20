use std::collections::HashMap;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use std::sync::Arc;
#[cfg(feature = "inference-nodes")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use pantograph_runtime_identity::canonical_engine_backend_key;

#[cfg(feature = "pytorch-nodes")]
use inference::ModelArtifactKind;
#[cfg(feature = "inference-nodes")]
use inference::{
    resolve_task_registry_entry, InferenceCompatibilityIssueSummary,
    InferenceCompatibilityReportSummary, InferenceExecutionInputKind, InferenceLifecyclePhase,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceTaskId, ResolvedModelPackageFacts,
    TaskRegistryEntry,
};
#[cfg(feature = "pytorch-nodes")]
use inference::{BackendCompatibilityOptions, BackendCompatibilityRequest, PyTorchBackend};

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::error::{NodeEngineError, Result};
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::extensions::extension_keys;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::extensions::ExecutorExtensions;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::model_dependencies::{DependencyState, ModelDependencyRequest, ModelDependencyResolver};
use crate::model_dependencies::{ModelDependencyBinding, ModelRefV2};

use super::{read_optional_input_string, read_optional_input_value};
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use super::{read_optional_input_string_aliases, read_optional_input_value_aliases};

#[cfg(feature = "pytorch-nodes")]
const MAX_DEPENDENCY_PREFLIGHT_COMPATIBILITY_ISSUES: usize = 32;

#[cfg(feature = "inference-nodes")]
#[derive(Debug, Clone)]
pub(crate) struct DependencyPreflightLifecycleContext {
    pub(crate) task_id: String,
    pub(crate) execution_id: String,
    pub(crate) task_label: String,
    pub(crate) backend_key: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) resolved_artifact_kind: Option<String>,
}

#[cfg(feature = "inference-nodes")]
#[derive(Debug, Default)]
struct DependencyPreflightCompatibilityDiagnostics {
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
}

#[cfg(all(feature = "audio-nodes", not(feature = "inference-nodes")))]
pub(crate) struct DependencyPreflightLifecycleContext;

pub(crate) fn read_input_dependency_bindings(
    inputs: &HashMap<String, serde_json::Value>,
) -> Vec<ModelDependencyBinding> {
    let Some(raw) = read_optional_input_value(inputs, "dependency_bindings") else {
        return Vec::new();
    };
    if raw.is_null() {
        return Vec::new();
    }
    serde_json::from_value(raw).unwrap_or_default()
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn read_input_selected_binding_ids(
    inputs: &HashMap<String, serde_json::Value>,
) -> Vec<String> {
    let Some(raw) =
        read_optional_input_value_aliases(inputs, &["selected_binding_ids", "selectedBindingIds"])
    else {
        return Vec::new();
    };

    raw.as_array()
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        .collect()
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn infer_task_type_primary(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> String {
    #[cfg(feature = "inference-nodes")]
    if let Some(task_entry) = canonical_inference_task_entry(inputs) {
        return resolver_task_type_primary(&task_entry);
    }

    if let Some(task) =
        read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
    {
        if !task.trim().is_empty() {
            return task;
        }
    }

    let model_type = read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
        .or_else(|| {
            inputs
                .get("_data")
                .and_then(|d| d.get("model_type"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
        .to_lowercase();

    if node_type == "audio-generation" || model_type == "audio" {
        return "text-to-audio".to_string();
    }
    if model_type == "reranker" {
        return "reranking".to_string();
    }

    match model_type.as_str() {
        "diffusion" => "text-to-image".to_string(),
        "vision" => "image-to-text".to_string(),
        "embedding" => "feature-extraction".to_string(),
        "reranker" => "reranking".to_string(),
        _ => "text-generation".to_string(),
    }
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn canonical_inference_task_id(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<InferenceTaskId> {
    canonical_inference_task_entry(inputs).map(|entry| entry.task_id)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn canonical_inference_input_kind(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<InferenceExecutionInputKind> {
    canonical_inference_task_entry(inputs)
        .and_then(|entry| entry.request_contract())
        .map(|contract| contract.input_kind)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn canonical_inference_task_entry(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<TaskRegistryEntry> {
    read_inference_task_label(inputs).and_then(|label| resolve_task_registry_entry(&label))
}

#[cfg(feature = "inference-nodes")]
fn read_inference_task_label(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
    read_optional_input_string_aliases(
        inputs,
        &[
            "task_kind",
            "taskKind",
            "task_type_primary",
            "taskTypePrimary",
            "pipeline_tag",
            "pipelineTag",
        ],
    )
    .or_else(|| {
        inputs.get("pumas_model_ref").and_then(|model_ref| {
            read_optional_string_aliases_from_value(
                model_ref,
                &[
                    "task_kind",
                    "taskKind",
                    "task_type_primary",
                    "taskTypePrimary",
                    "pipeline_tag",
                    "pipelineTag",
                ],
            )
        })
    })
}

#[cfg(feature = "inference-nodes")]
fn resolver_task_type_primary(task: &TaskRegistryEntry) -> String {
    match task.task_id {
        InferenceTaskId::Embedding => "feature-extraction".to_string(),
        InferenceTaskId::Rerank => "reranking".to_string(),
        InferenceTaskId::AudioTranscription => "automatic-speech-recognition".to_string(),
        InferenceTaskId::TextGeneration | InferenceTaskId::ChatCompletion => {
            "text-generation".to_string()
        }
        _ => task.canonical_label().replace('_', "-"),
    }
}

pub(crate) fn build_model_ref_v2(
    resolved: Option<ModelRefV2>,
    engine: &str,
    model_id: &str,
    model_path: &str,
    task_type_primary: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> ModelRefV2 {
    let fallback_dependency_bindings = read_input_dependency_bindings(inputs);
    let fallback_dependency_requirements_id =
        read_optional_input_string(inputs, "dependency_requirements_id");

    let mut model_ref = resolved.unwrap_or(ModelRefV2 {
        contract_version: 2,
        engine: engine.to_string(),
        model_id: model_id.to_string(),
        model_path: model_path.to_string(),
        task_type_primary: task_type_primary.to_string(),
        dependency_bindings: fallback_dependency_bindings.clone(),
        dependency_requirements_id: fallback_dependency_requirements_id.clone(),
    });

    if model_ref.contract_version != 2 {
        model_ref.contract_version = 2;
    }
    if model_ref.engine.trim().is_empty() {
        model_ref.engine = engine.to_string();
    }
    if model_ref.model_id.trim().is_empty() {
        model_ref.model_id = model_id.to_string();
    }
    if model_ref.model_path.trim().is_empty() {
        model_ref.model_path = model_path.to_string();
    }
    if model_ref.task_type_primary.trim().is_empty() {
        model_ref.task_type_primary = task_type_primary.to_string();
    }
    if model_ref.dependency_bindings.is_empty() {
        model_ref.dependency_bindings = fallback_dependency_bindings;
    }
    if model_ref.dependency_requirements_id.is_none() {
        model_ref.dependency_requirements_id = fallback_dependency_requirements_id;
    }

    model_ref
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn canonical_backend_key(value: Option<&str>) -> Option<String> {
    canonical_engine_backend_key(value)
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn infer_backend_key(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    match node_type {
        "audio-generation" => Some("stable_audio".to_string()),
        "llm-inference" => {
            #[cfg(feature = "inference-nodes")]
            if matches!(
                canonical_inference_task_id(inputs),
                Some(InferenceTaskId::Embedding | InferenceTaskId::Rerank)
            ) {
                return Some("llamacpp".to_string());
            }
            let model_type =
                read_optional_input_string_aliases(inputs, &["model_type", "modelType"])
                    .or_else(|| {
                        inputs.get("pumas_model_ref").and_then(|model_ref| {
                            read_optional_string_aliases_from_value(
                                model_ref,
                                &["model_type", "modelType"],
                            )
                        })
                    })
                    .unwrap_or_default()
                    .to_ascii_lowercase();
            if model_type == "embedding" || model_type == "reranker" {
                Some("llamacpp".to_string())
            } else {
                Some("pytorch".to_string())
            }
        }
        "onnx-inference" => Some("onnx-runtime".to_string()),
        "embedding" | "reranker" | "llamacpp-inference" | "pytorch-inference" => None,
        _ => Some("pytorch".to_string()),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn preferred_backend_key(
    _node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    if let Some(backend) =
        read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
            .and_then(|value| canonical_backend_key(Some(value.as_str())))
    {
        return Some(backend);
    }

    None
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn build_model_dependency_request(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> ModelDependencyRequest {
    let package_facts = read_resolved_model_package_facts_for_preflight(inputs);
    let backend_key =
        preferred_backend_key(node_type, inputs).or_else(|| infer_backend_key(node_type, inputs));

    let task_type_primary =
        read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
            .filter(|s| !s.trim().is_empty())
            .or_else(|| task_type_primary_from_package_facts(package_facts.as_ref()))
            .unwrap_or_else(|| infer_task_type_primary(node_type, inputs));

    ModelDependencyRequest {
        node_type: node_type.to_string(),
        model_path: String::new(),
        model_id: read_optional_input_string_aliases(inputs, &["model_id", "modelId"])
            .or_else(|| model_id_from_pumas_model_ref_input(inputs)),
        model_type: read_optional_input_string_aliases(inputs, &["model_type", "modelType"]),
        task_type_primary: Some(task_type_primary),
        backend_key,
        platform_context: read_optional_input_value_aliases(
            inputs,
            &["platform_context", "platformContext"],
        ),
        selected_binding_ids: read_input_selected_binding_ids(inputs),
        dependency_override_patches: Vec::new(),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn model_id_from_pumas_model_ref_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    read_optional_input_value_aliases(inputs, &["pumas_model_ref", "pumasModelRef"]).and_then(
        |model_ref| read_optional_string_aliases_from_value(&model_ref, &["model_id", "modelId"]),
    )
}

#[cfg(feature = "inference-nodes")]
fn read_resolved_model_package_facts_for_preflight(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<ResolvedModelPackageFacts> {
    read_optional_input_value_aliases(
        inputs,
        &[
            "resolved_model_package_facts",
            "resolvedModelPackageFacts",
            "model_package_facts",
            "modelPackageFacts",
        ],
    )
    .filter(|raw| !raw.is_null())
    .and_then(|raw| serde_json::from_value(raw).ok())
}

#[cfg(not(feature = "inference-nodes"))]
fn read_resolved_model_package_facts_for_preflight(
    _inputs: &HashMap<String, serde_json::Value>,
) -> Option<()> {
    None
}

#[cfg(feature = "pytorch-nodes")]
pub(crate) fn read_resolved_artifact_kind_from_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    read_resolved_model_package_facts_for_preflight(inputs)
        .map(|facts| model_artifact_kind_label(&facts.artifact.artifact_kind).to_string())
}

#[cfg(feature = "pytorch-nodes")]
fn model_artifact_kind_label(kind: &ModelArtifactKind) -> &'static str {
    match kind {
        ModelArtifactKind::Gguf => "gguf",
        ModelArtifactKind::HfCompatibleDirectory => "hf_compatible_directory",
        ModelArtifactKind::Safetensors => "safetensors",
        ModelArtifactKind::DiffusersBundle => "diffusers_bundle",
        ModelArtifactKind::Onnx => "onnx",
        ModelArtifactKind::Adapter => "adapter",
        ModelArtifactKind::Shard => "shard",
        ModelArtifactKind::Unknown => "unknown",
    }
}

#[cfg(feature = "inference-nodes")]
fn task_type_primary_from_package_facts(
    facts: Option<&ResolvedModelPackageFacts>,
) -> Option<String> {
    facts
        .and_then(|facts| facts.task.task_type_primary.clone())
        .filter(|task| !task.trim().is_empty())
}

#[cfg(not(feature = "inference-nodes"))]
fn task_type_primary_from_package_facts(_facts: Option<&()>) -> Option<String> {
    None
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn inputs_with_model_path_from_ref(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    reject_retired_resolved_model_source_inputs(inputs)?;
    reject_unresolved_model_reference_inputs(inputs)?;

    let mut canonical_inputs = inputs.clone();
    if canonical_inputs
        .get("model_path")
        .and_then(|value| value.as_str())
        .is_none_or(|value| value.trim().is_empty())
    {
        if let Some(model_path) = read_model_path_from_inputs(inputs) {
            canonical_inputs.insert("model_path".to_string(), serde_json::json!(model_path));
        }
    }
    if canonical_inputs
        .get("mmproj_path")
        .and_then(|value| value.as_str())
        .is_none_or(|value| value.trim().is_empty())
    {
        if let Some(mmproj_path) = read_mmproj_path_from_inputs(inputs) {
            canonical_inputs.insert("mmproj_path".to_string(), serde_json::json!(mmproj_path));
        }
    }
    Ok(canonical_inputs)
}

#[cfg(feature = "inference-nodes")]
fn reject_retired_resolved_model_source_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    if read_optional_input_value_aliases(inputs, &["resolved_model_source", "resolvedModelSource"])
        .is_some()
    {
        return Err(NodeEngineError::ExecutionFailed(
            "Retired resolved_model_source input cannot provide executable model paths. Use canonical pumas_model_ref and host-provided planning facts instead."
                .to_string(),
        ));
    }

    Ok(())
}

#[cfg(feature = "inference-nodes")]
fn reject_unresolved_model_reference_inputs(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<()> {
    for (field_name, aliases) in [("pumas_model_ref", ["pumas_model_ref", "pumasModelRef"])] {
        let Some(raw) = read_optional_input_value_aliases(inputs, &aliases) else {
            continue;
        };
        if !model_reference_status_is_unresolved(&raw) {
            continue;
        }
        let source = raw
            .get("source")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Canonical inference model reference is unresolved in {field_name} from {source}. Resolve this model through Pumas before execution."
        )));
    }

    Ok(())
}

#[cfg(feature = "inference-nodes")]
fn model_reference_status_is_unresolved(value: &serde_json::Value) -> bool {
    value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("unresolved"))
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn read_model_path_from_inputs(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
    read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
}

#[cfg(feature = "inference-nodes")]
fn read_mmproj_path_from_inputs(inputs: &HashMap<String, serde_json::Value>) -> Option<String> {
    read_optional_input_string_aliases(
        inputs,
        &[
            "mmproj_path",
            "mmprojPath",
            "selected_mmproj_path",
            "selectedMmprojPath",
        ],
    )
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn read_optional_string_aliases_from_value(
    value: &serde_json::Value,
    aliases: &[&str],
) -> Option<String> {
    aliases.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) async fn enforce_dependency_preflight(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<Option<ModelRefV2>> {
    enforce_dependency_preflight_inner(node_type, inputs, extensions, None).await
}

#[cfg(any(feature = "pytorch-nodes", test))]
pub(crate) async fn enforce_dependency_preflight_with_lifecycle(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
) -> Result<Option<ModelRefV2>> {
    enforce_dependency_preflight_inner(node_type, inputs, extensions, lifecycle_context).await
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
async fn enforce_dependency_preflight_inner(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    #[cfg_attr(not(feature = "inference-nodes"), allow(unused_variables))]
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
) -> Result<Option<ModelRefV2>> {
    let llm_backend_key = preferred_backend_key(node_type, inputs);
    let should_preflight = node_type == "audio-generation"
        || (node_type == "llm-inference" && llm_backend_key.as_deref() == Some("pytorch"));
    if !should_preflight {
        return Ok(None);
    }

    let Some(resolver) = extensions
        .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
    else {
        let message =
            "Dependency preflight blocked execution: dependency resolver is not configured"
                .to_string();
        record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
        return Err(NodeEngineError::ExecutionFailed(message));
    };

    let request = build_model_dependency_request(node_type, inputs);
    let request_model_id = match request.model_id.as_deref() {
        Some(model_id) if !model_id.trim().is_empty() => model_id.to_string(),
        _ => {
            let message =
                "Missing pumas_model_ref/model_id input. Connect Puma-Lib pumas_model_ref output."
                    .to_string();
            record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
            return Err(NodeEngineError::ExecutionFailed(message));
        }
    };
    let requirements = match resolver
        .resolve_model_dependency_requirements(request.clone())
        .await
    {
        Ok(requirements) => requirements,
        Err(error) => {
            let message = format!(
                "Dependency preflight requirements resolution failed for '{}': {}",
                node_type, error
            );
            record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
            return Err(NodeEngineError::ExecutionFailed(message));
        }
    };

    let status = match resolver.check_dependencies(request.clone()).await {
        Ok(status) => status,
        Err(error) => {
            let message = format!(
                "Dependency preflight check failed for '{}': {}",
                node_type, error
            );
            record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
            return Err(NodeEngineError::ExecutionFailed(message));
        }
    };

    if status.state != DependencyState::Ready {
        let payload = serde_json::json!({
            "kind": "dependency_preflight",
            "node_type": node_type,
            "model_id": request_model_id,
            "validation_state": requirements.validation_state,
            "validation_errors": requirements.validation_errors,
            "selected_binding_ids": requirements.selected_binding_ids,
            "state": status.state,
            "code": status.code,
            "bindings": status.bindings,
            "message": status.message,
        });
        let message = format!("Dependency preflight blocked execution: {}", payload);
        record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
        return Err(NodeEngineError::ExecutionFailed(message));
    }

    let resolved = match resolver
        .resolve_model_ref(request, Some(requirements))
        .await
    {
        Ok(resolved) => resolved,
        Err(error) => {
            let message = format!(
                "Dependency preflight failed to resolve model_ref: {}",
                error
            );
            record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &message);
            return Err(NodeEngineError::ExecutionFailed(message));
        }
    };
    if let Some(ref model_ref) = resolved {
        if let Err(error) = model_ref.validate() {
            record_dependency_preflight_failure_lifecycle(extensions, lifecycle_context, &error);
            return Err(NodeEngineError::ExecutionFailed(error));
        }
    }

    #[cfg(feature = "inference-nodes")]
    let compatibility_diagnostics =
        dependency_preflight_compatibility_diagnostics(inputs, lifecycle_context);
    #[cfg(not(feature = "inference-nodes"))]
    let compatibility_diagnostics = ();
    record_dependency_preflight_success_lifecycle(
        extensions,
        lifecycle_context,
        &compatibility_diagnostics,
    );
    Ok(resolved)
}

#[cfg(feature = "inference-nodes")]
fn record_dependency_preflight_failure_lifecycle(
    extensions: &ExecutorExtensions,
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
    detail: &str,
) {
    record_dependency_preflight_lifecycle(
        extensions,
        lifecycle_context,
        InferenceRequestLifecycleEventKind::Failed,
        Some(sanitize_dependency_preflight_lifecycle_detail(detail)),
        &DependencyPreflightCompatibilityDiagnostics::default(),
    );
}

#[cfg(feature = "inference-nodes")]
fn sanitize_dependency_preflight_lifecycle_detail(detail: &str) -> String {
    let mut sanitized = detail.to_string();
    for candidate in detail
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '"' | '\'' | ',' | ':' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>'
                )
        })
        .filter(|candidate| !candidate.is_empty())
    {
        if inference::looks_like_local_artifact_ref(candidate) {
            sanitized = sanitized.replace(candidate, "[local-path]");
        }
    }
    sanitized
}

#[cfg(not(feature = "inference-nodes"))]
fn record_dependency_preflight_failure_lifecycle(
    _extensions: &ExecutorExtensions,
    _lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
    _detail: &str,
) {
}

#[cfg(feature = "inference-nodes")]
fn record_dependency_preflight_success_lifecycle(
    extensions: &ExecutorExtensions,
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
    compatibility_diagnostics: &DependencyPreflightCompatibilityDiagnostics,
) {
    record_dependency_preflight_lifecycle(
        extensions,
        lifecycle_context,
        InferenceRequestLifecycleEventKind::Completed,
        None,
        compatibility_diagnostics,
    );
}

#[cfg(not(feature = "inference-nodes"))]
fn record_dependency_preflight_success_lifecycle(
    _extensions: &ExecutorExtensions,
    _lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
    _compatibility_diagnostics: &(),
) {
}

#[cfg(feature = "inference-nodes")]
fn record_dependency_preflight_lifecycle(
    extensions: &ExecutorExtensions,
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
    terminal_kind: InferenceRequestLifecycleEventKind,
    terminal_detail: Option<String>,
    compatibility_diagnostics: &DependencyPreflightCompatibilityDiagnostics,
) {
    let Some(context) = lifecycle_context else {
        return;
    };
    let Some(sink) = extensions
        .get::<Arc<dyn InferenceRequestLifecycleEventSink>>(
            extension_keys::INFERENCE_LIFECYCLE_SINK,
        )
        .cloned()
    else {
        return;
    };

    let request_id = Some(format!(
        "{}:{}:{}",
        context.execution_id, context.task_id, context.task_label
    ));
    let runtime_id = context.backend_key.clone();
    for (kind, detail) in [
        (InferenceRequestLifecycleEventKind::Started, None),
        (terminal_kind, terminal_detail),
        (InferenceRequestLifecycleEventKind::CleanupCompleted, None),
    ] {
        let emit_compatibility = kind == InferenceRequestLifecycleEventKind::Completed;
        let event = InferenceRequestLifecycleEvent::builder(
            InferenceLifecyclePhase::ModelPackageResolution,
            kind,
            dependency_preflight_unix_timestamp_ms(),
        )
        .with_request_id(request_id.clone())
        .with_task_id(Some(context.task_label.clone()))
        .with_backend_key(context.backend_key.clone())
        .with_runtime_id(runtime_id.clone())
        .with_model_id(context.model_id.clone())
        .with_resolved_artifact_kind(context.resolved_artifact_kind.clone())
        .with_detail(detail)
        .with_compatibility_report(
            emit_compatibility
                .then(|| compatibility_diagnostics.compatibility_report.clone())
                .flatten(),
        )
        .with_compatibility_issues(if emit_compatibility {
            compatibility_diagnostics.compatibility_issues.clone()
        } else {
            Vec::new()
        })
        .build();
        if let Err(error) = sink.record(event) {
            log::warn!("failed to record inference dependency preflight lifecycle event: {error}");
        }
    }
}

#[cfg(feature = "pytorch-nodes")]
fn dependency_preflight_compatibility_diagnostics(
    inputs: &HashMap<String, serde_json::Value>,
    lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
) -> DependencyPreflightCompatibilityDiagnostics {
    let Some(context) = lifecycle_context else {
        return DependencyPreflightCompatibilityDiagnostics::default();
    };
    if context.backend_key.as_deref() != Some("pytorch") {
        return DependencyPreflightCompatibilityDiagnostics::default();
    }
    let Some(package_facts) = read_resolved_model_package_facts_for_preflight(inputs) else {
        return DependencyPreflightCompatibilityDiagnostics::default();
    };
    let Some(task) = resolve_task_registry_entry(&context.task_label)
        .or_else(|| canonical_inference_task_entry(inputs))
    else {
        return DependencyPreflightCompatibilityDiagnostics::default();
    };

    let report = PyTorchBackend::static_capabilities().check_model_compatibility(
        context.backend_key.as_deref(),
        BackendCompatibilityRequest::new(&task, &package_facts)
            .with_options(BackendCompatibilityOptions::default()),
    );

    DependencyPreflightCompatibilityDiagnostics {
        compatibility_report: Some(report.to_inference_compatibility_report_summary()),
        compatibility_issues: report.to_inference_compatibility_issue_summaries(
            MAX_DEPENDENCY_PREFLIGHT_COMPATIBILITY_ISSUES,
        ),
    }
}

#[cfg(all(feature = "inference-nodes", not(feature = "pytorch-nodes")))]
fn dependency_preflight_compatibility_diagnostics(
    _inputs: &HashMap<String, serde_json::Value>,
    _lifecycle_context: Option<&DependencyPreflightLifecycleContext>,
) -> DependencyPreflightCompatibilityDiagnostics {
    DependencyPreflightCompatibilityDiagnostics::default()
}

#[cfg(feature = "inference-nodes")]
fn dependency_preflight_unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Retired direct-backend inference nodes
// ---------------------------------------------------------------------------
