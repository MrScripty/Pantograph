use std::collections::HashMap;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use std::sync::Arc;

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use pantograph_runtime_identity::canonical_engine_backend_key;

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::error::{NodeEngineError, Result};
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::extensions::ExecutorExtensions;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::extensions::extension_keys;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::model_dependencies::{DependencyState, ModelDependencyRequest, ModelDependencyResolver};
use crate::model_dependencies::{ModelDependencyBinding, ModelRefV2};

use super::{read_optional_input_string, read_optional_input_value};
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use super::{read_optional_input_string_aliases, read_optional_input_value_aliases};

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
    if node_type == "diffusion-inference" {
        return "text-to-image".to_string();
    }
    if node_type == "reranker" || model_type == "reranker" {
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
pub(crate) fn infer_backend_key(node_type: &str) -> Option<String> {
    match node_type {
        "audio-generation" => Some("stable_audio".to_string()),
        "pytorch-inference" => Some("pytorch".to_string()),
        // Leave diffusion unspecified when the graph does not provide a
        // concrete backend so Pumas can apply the model's recommended
        // execution profile.
        "diffusion-inference" => None,
        "llamacpp-inference" => Some("llamacpp".to_string()),
        "reranker" => Some("llamacpp".to_string()),
        "ollama-inference" => Some("ollama".to_string()),
        "onnx-inference" => Some("onnx-runtime".to_string()),
        _ => Some("pytorch".to_string()),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn preferred_backend_key(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<String> {
    if node_type == "diffusion-inference" {
        if let Some(backend) = read_optional_input_string_aliases(
            inputs,
            &["recommended_backend", "recommendedBackend"],
        )
        .and_then(|value| canonical_backend_key(Some(value.as_str())))
        {
            return Some(backend);
        }
    }

    read_optional_input_string_aliases(inputs, &["backend_key", "backendKey"])
        .and_then(|value| canonical_backend_key(Some(value.as_str())))
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
pub(crate) fn build_model_dependency_request(
    node_type: &str,
    model_path: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> ModelDependencyRequest {
    let backend_key =
        preferred_backend_key(node_type, inputs).or_else(|| infer_backend_key(node_type));

    let task_type_primary =
        read_optional_input_string_aliases(inputs, &["task_type_primary", "taskTypePrimary"])
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| infer_task_type_primary(node_type, inputs));

    ModelDependencyRequest {
        node_type: node_type.to_string(),
        model_path: model_path.to_string(),
        model_id: read_optional_input_string_aliases(inputs, &["model_id", "modelId"]),
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
pub(crate) async fn enforce_dependency_preflight(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<Option<ModelRefV2>> {
    if node_type != "pytorch-inference"
        && node_type != "diffusion-inference"
        && node_type != "audio-generation"
    {
        return Ok(None);
    }

    let Some(resolver) = extensions
        .get::<Arc<dyn ModelDependencyResolver>>(extension_keys::MODEL_DEPENDENCY_RESOLVER)
    else {
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

    let request = build_model_dependency_request(node_type, model_path, inputs);
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

// ---------------------------------------------------------------------------
// Ollama (pure HTTP, no gateway needed)
// ---------------------------------------------------------------------------
