use std::collections::HashMap;

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use pantograph_runtime_identity::canonical_engine_backend_key;

#[cfg(feature = "pytorch-nodes")]
use inference::ModelArtifactKind;
#[cfg(feature = "inference-nodes")]
use inference::{
    resolve_task_registry_entry, InferenceExecutionInputKind, InferenceTaskId,
    ResolvedModelPackageFacts, TaskRegistryEntry,
};

use crate::model_dependencies::ModelDependencyBinding;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::model_dependencies::ModelDependencyRequest;

use super::super::read_optional_input_value;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use super::super::{read_optional_input_string_aliases, read_optional_input_value_aliases};

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
pub(crate) fn read_resolved_model_package_facts_for_preflight(
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
pub(crate) fn read_resolved_model_package_facts_for_preflight(
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
