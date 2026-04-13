use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tauri::State;

use super::commands::{SharedExtensions, SharedNodeRegistry};
use super::model_dependencies::SharedModelDependencyResolver;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PumaLibNodeHydrationResponse {
    pub node_data: Value,
}

pub async fn hydrate_puma_lib_node(
    registry: State<'_, SharedNodeRegistry>,
    extensions: State<'_, SharedExtensions>,
    resolver: State<'_, SharedModelDependencyResolver>,
    model_path: Option<String>,
    model_id: Option<String>,
    selected_binding_ids: Option<Vec<String>>,
    resolve_requirements: Option<bool>,
) -> Result<PumaLibNodeHydrationResponse, String> {
    let requested_model_path = clean_optional(model_path);
    let requested_model_id = clean_optional(model_id);
    if requested_model_path.is_none() && requested_model_id.is_none() {
        return Err("model_path or model_id is required".to_string());
    }

    let option = find_matching_model_option(
        &registry,
        &extensions,
        requested_model_path.as_deref(),
        requested_model_id.as_deref(),
    )
    .await?;

    let mut node_data =
        build_hydrated_node_data(&option, selected_binding_ids.unwrap_or_default())?;

    if resolve_requirements.unwrap_or(false) {
        hydrate_dependency_requirements(&resolver, &mut node_data).await?;
    }

    Ok(PumaLibNodeHydrationResponse { node_data })
}

async fn find_matching_model_option(
    registry: &SharedNodeRegistry,
    extensions: &SharedExtensions,
    requested_model_path: Option<&str>,
    requested_model_id: Option<&str>,
) -> Result<node_engine::PortOption, String> {
    let ext = extensions.read().await;
    let result = registry
        .query_port_options(
            "puma-lib",
            "model_path",
            &node_engine::PortOptionsQuery::default(),
            &ext,
        )
        .await
        .map_err(|error| error.to_string())?;

    result
        .options
        .into_iter()
        .find(|option| {
            requested_model_path
                .is_some_and(|path| option_value_string(option).is_some_and(|value| value == path))
                || requested_model_id.is_some_and(|model_id| {
                    option_metadata_string(option, &["id"]).is_some_and(|value| value == model_id)
                })
        })
        .ok_or_else(|| {
            let id = requested_model_id.unwrap_or("<none>");
            let path = requested_model_path.unwrap_or("<none>");
            format!("Unable to resolve Puma-Lib model for model_id '{id}' and model_path '{path}'")
        })
}

fn build_hydrated_node_data(
    option: &node_engine::PortOption,
    selected_binding_ids: Vec<String>,
) -> Result<Value, String> {
    let model_path = option_value_string(option)
        .ok_or_else(|| "Puma-Lib option is missing a string model path".to_string())?;
    let metadata = option
        .metadata
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| "Puma-Lib option metadata is missing".to_string())?;

    let task_type_primary = metadata_string(
        metadata,
        &[
            "task_type_primary",
            "taskTypePrimary",
            "task_type",
            "taskType",
        ],
    );
    let recommended_backend = normalize_backend_key(
        metadata_string(metadata, &["recommended_backend", "recommendedBackend"]).as_deref(),
    );
    let dependency_bindings = metadata
        .get("dependency_bindings")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let backend_key = unique_binding_backend(&dependency_bindings)
        .or_else(|| recommended_backend.clone())
        .or_else(|| infer_backend_key_from_task(task_type_primary.as_deref()));

    let node_data = json!({
        "modelPath": model_path,
        "modelName": option.label,
        "model_id": metadata_string(metadata, &["id"]),
        "model_type": metadata_string(metadata, &["model_type", "modelType"]),
        "task_type_primary": task_type_primary,
        "backend_key": backend_key,
        "recommended_backend": recommended_backend,
        "runtime_engine_hints": metadata.get("runtime_engine_hints").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "requires_custom_code": metadata.get("requires_custom_code").cloned().unwrap_or(Value::Bool(false)),
        "custom_code_sources": metadata.get("custom_code_sources").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "platform_context": current_platform_context(),
        "dependency_bindings": dependency_bindings,
        "review_reasons": metadata.get("review_reasons").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "selected_binding_ids": sanitize_selected_binding_ids(selected_binding_ids),
        "inference_settings": metadata.get("inference_settings").cloned().unwrap_or_else(|| Value::Array(Vec::new())),
        "dependency_requirements_id": Value::Null,
        "dependency_requirements": Value::Null,
    });

    Ok(node_data)
}

async fn hydrate_dependency_requirements(
    resolver: &State<'_, SharedModelDependencyResolver>,
    node_data: &mut Value,
) -> Result<(), String> {
    let model_path = node_data
        .get("modelPath")
        .and_then(Value::as_str)
        .ok_or_else(|| "Hydrated Puma-Lib node is missing modelPath".to_string())?;
    let model_id = node_data
        .get("model_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let model_type = node_data
        .get("model_type")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let task_type_primary = node_data
        .get("task_type_primary")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let backend_key = node_data
        .get("backend_key")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let platform_context = node_data.get("platform_context").cloned();
    let mut selected_binding_ids = node_data
        .get("selected_binding_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let node_type = infer_runtime_node_type(task_type_primary.as_deref(), backend_key.as_deref());

    let mut requirements = resolver
        .resolve_requirements_request(node_engine::ModelDependencyRequest {
            node_type,
            model_path: model_path.to_string(),
            model_id,
            model_type,
            task_type_primary,
            backend_key,
            platform_context,
            selected_binding_ids: selected_binding_ids.clone(),
            dependency_override_patches: Vec::new(),
        })
        .await?;

    if selected_binding_ids.is_empty() && requirements.selected_binding_ids.is_empty() {
        selected_binding_ids = requirements
            .bindings
            .iter()
            .map(|binding| binding.binding_id.clone())
            .collect();
        requirements.selected_binding_ids = selected_binding_ids.clone();
    } else if !requirements.selected_binding_ids.is_empty() {
        selected_binding_ids = requirements.selected_binding_ids.clone();
    }

    let object = node_data
        .as_object_mut()
        .ok_or_else(|| "Hydrated Puma-Lib node data must be an object".to_string())?;
    object.insert(
        "dependency_requirements_id".to_string(),
        Value::String(requirements.model_id.clone()),
    );
    object.insert(
        "dependency_requirements".to_string(),
        serde_json::to_value(&requirements).map_err(|error| error.to_string())?,
    );
    object.insert(
        "selected_binding_ids".to_string(),
        json!(sanitize_selected_binding_ids(selected_binding_ids)),
    );
    Ok(())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn option_value_string(option: &node_engine::PortOption) -> Option<&str> {
    option.value.as_str()
}

fn option_metadata_string(option: &node_engine::PortOption, keys: &[&str]) -> Option<String> {
    option
        .metadata
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|metadata| metadata_string(metadata, keys))
}

fn metadata_string(metadata: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        metadata
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn normalize_backend_key(value: Option<&str>) -> Option<String> {
    let token = value?.trim().to_ascii_lowercase();
    if token.is_empty() {
        return None;
    }

    match token.as_str() {
        "llama.cpp" | "llama-cpp" | "llama_cpp" | "llamacpp" => Some("llamacpp".to_string()),
        "onnx-runtime" | "onnxruntime" | "onnx_runtime" => Some("onnx-runtime".to_string()),
        "torch" | "pytorch" => Some("pytorch".to_string()),
        "stable-audio" | "stable_audio" => Some("stable_audio".to_string()),
        other => Some(other.to_string()),
    }
}

fn unique_binding_backend(bindings: &Value) -> Option<String> {
    let unique = bindings
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|binding| {
            binding
                .as_object()
                .and_then(|value| value.get("backend_key"))
                .and_then(Value::as_str)
                .and_then(|value| normalize_backend_key(Some(value)))
        })
        .collect::<std::collections::BTreeSet<_>>();

    if unique.len() == 1 {
        unique.into_iter().next()
    } else {
        None
    }
}

fn infer_backend_key_from_task(task_type_primary: Option<&str>) -> Option<String> {
    let task = task_type_primary?.trim().to_ascii_lowercase();
    if task.is_empty() {
        return None;
    }

    match task.as_str() {
        "text-to-audio" => Some("stable_audio".to_string()),
        "audio-to-text" | "text-to-image" | "image-to-image" => Some("pytorch".to_string()),
        _ => Some("pytorch".to_string()),
    }
}

fn infer_runtime_node_type(task_type_primary: Option<&str>, backend_key: Option<&str>) -> String {
    let task = task_type_primary
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(task.as_str(), "text-to-image" | "image-to-image") {
        return "diffusion-inference".to_string();
    }

    if normalize_backend_key(backend_key).as_deref() == Some("onnx-runtime") {
        return "onnx-inference".to_string();
    }

    if task == "text-to-audio" {
        return "audio-generation".to_string();
    }

    "pytorch-inference".to_string()
}

fn current_platform_context() -> Value {
    json!({
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    })
}

fn sanitize_selected_binding_ids(selected_binding_ids: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for binding_id in selected_binding_ids {
        let trimmed = binding_id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            out.push(owned);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_option() -> node_engine::PortOption {
        node_engine::PortOption {
            value: json!("/models/tiny-sd-turbo"),
            label: "Tiny SD Turbo".to_string(),
            description: None,
            metadata: Some(json!({
                "id": "diffusion/cc-nms/tiny-sd-turbo",
                "model_type": "diffusion",
                "task_type_primary": "text-to-image",
                "recommended_backend": "diffusers",
                "runtime_engine_hints": ["diffusers", "pytorch"],
                "requires_custom_code": false,
                "custom_code_sources": [],
                "dependency_bindings": [
                    {
                        "binding_id": "binding-a",
                        "backend_key": "onnxruntime"
                    }
                ],
                "review_reasons": ["imported"],
                "inference_settings": [{ "key": "steps" }]
            })),
        }
    }

    #[test]
    fn build_hydrated_node_data_uses_backend_owned_defaults() {
        let node_data = build_hydrated_node_data(&sample_option(), vec![" binding-a ".to_string()])
            .expect("node data");

        assert_eq!(node_data["modelPath"], json!("/models/tiny-sd-turbo"));
        assert_eq!(node_data["modelName"], json!("Tiny SD Turbo"));
        assert_eq!(
            node_data["model_id"],
            json!("diffusion/cc-nms/tiny-sd-turbo")
        );
        assert_eq!(node_data["backend_key"], json!("onnx-runtime"));
        assert_eq!(node_data["recommended_backend"], json!("diffusers"));
        assert_eq!(node_data["selected_binding_ids"], json!(["binding-a"]));
        assert_eq!(node_data["inference_settings"], json!([{ "key": "steps" }]));
        assert!(node_data["dependency_requirements"].is_null());
    }

    #[test]
    fn infer_runtime_node_type_matches_puma_lib_task_shape() {
        assert_eq!(
            infer_runtime_node_type(Some("text-to-image"), Some("pytorch")),
            "diffusion-inference"
        );
        assert_eq!(
            infer_runtime_node_type(Some("text-generation"), Some("onnxruntime")),
            "onnx-inference"
        );
        assert_eq!(
            infer_runtime_node_type(Some("text-to-audio"), Some("stable_audio")),
            "audio-generation"
        );
        assert_eq!(
            infer_runtime_node_type(Some("text-generation"), Some("pytorch")),
            "pytorch-inference"
        );
    }

    #[test]
    fn normalize_backend_key_accepts_llama_cpp_alias() {
        assert_eq!(
            normalize_backend_key(Some("llama_cpp")),
            Some("llamacpp".to_string())
        );
    }

    #[test]
    fn sanitize_selected_binding_ids_deduplicates_and_trims() {
        let bindings = sanitize_selected_binding_ids(vec![
            " binding-a ".to_string(),
            "".to_string(),
            "binding-a".to_string(),
            "binding-b".to_string(),
        ]);

        assert_eq!(
            bindings,
            vec!["binding-a".to_string(), "binding-b".to_string()]
        );
    }
}
