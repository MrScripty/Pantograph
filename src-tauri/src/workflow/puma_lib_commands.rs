use pantograph_runtime_identity::canonical_engine_backend_key;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::Arc;
use tauri::State;

use super::commands::{SharedExtensions, SharedNodeRegistry, SharedWorkflowService};
use super::model_dependencies::SharedModelDependencyResolver;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PumaLibNodeHydrationResponse {
    pub node_data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PumaModelDeleteAuditResponse {
    pub success: bool,
    pub error: Option<String>,
    pub audit_event_seq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PumaHfModelSearchAuditResponse {
    pub models: Vec<pumas_library::models::HuggingFaceModel>,
    pub audit_event_seq: Option<i64>,
}

async fn require_pumas_api(
    extensions: &State<'_, SharedExtensions>,
) -> Result<Arc<pumas_library::PumasApi>, String> {
    let ext = extensions.read().await;
    ext.get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
        .cloned()
        .ok_or_else(|| "Pumas API not available in executor extensions".to_string())
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

pub async fn delete_pumas_model_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    model_id: String,
) -> Result<PumaModelDeleteAuditResponse, String> {
    let model_id = validate_pumas_model_id_for_audit(&model_id)?;
    let api = require_pumas_api(&extensions).await?;
    let delete_result = api
        .delete_model_with_cascade(model_id)
        .await
        .map_err(|error| error.to_string())?;

    let audit_event_seq = if delete_result.success {
        record_pumas_model_delete_audit(&workflow_service, model_id)
    } else {
        None
    };

    Ok(PumaModelDeleteAuditResponse {
        success: delete_result.success,
        error: delete_result.error,
        audit_event_seq,
    })
}

pub async fn search_hf_models_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    query: String,
    kind: Option<String>,
    limit: Option<usize>,
    hydrate_limit: Option<usize>,
) -> Result<PumaHfModelSearchAuditResponse, String> {
    let query = validate_hf_search_query(&query)?;
    let kind = validate_optional_hf_search_kind(kind)?;
    let limit = validate_hf_search_limit(limit.unwrap_or(50))?;
    let hydrate_limit = validate_hf_search_limit(hydrate_limit.unwrap_or(limit))?.min(limit);
    let api = require_pumas_api(&extensions).await?;
    let models = api
        .search_hf_models_with_hydration(query, kind.as_deref(), limit, hydrate_limit)
        .await
        .map_err(|error| error.to_string())?;
    let audit_event_seq = record_hf_model_search_audit(&workflow_service);

    Ok(PumaHfModelSearchAuditResponse {
        models,
        audit_event_seq,
    })
}

fn record_pumas_model_delete_audit(
    workflow_service: &SharedWorkflowService,
    model_id: &str,
) -> Option<i64> {
    match workflow_service.workflow_library_asset_access_record(
        pantograph_workflow_service::WorkflowLibraryAssetAccessRecordRequest {
            asset_id: format!("pumas://models/{model_id}"),
            operation: pantograph_workflow_service::LibraryAssetOperation::Delete,
            cache_status: Some(pantograph_workflow_service::LibraryAssetCacheStatus::NotApplicable),
            network_bytes: None,
            source_instance_id: Some("pumas-model-delete".to_string()),
        },
    ) {
        Ok(response) => response.event_seq,
        Err(error) => {
            log::warn!("Failed to record Pumas model delete audit event: {error}");
            None
        }
    }
}

fn record_hf_model_search_audit(workflow_service: &SharedWorkflowService) -> Option<i64> {
    match workflow_service.workflow_library_asset_access_record(
        pantograph_workflow_service::WorkflowLibraryAssetAccessRecordRequest {
            asset_id: "hf://models".to_string(),
            operation: pantograph_workflow_service::LibraryAssetOperation::Search,
            cache_status: Some(pantograph_workflow_service::LibraryAssetCacheStatus::Unknown),
            network_bytes: None,
            source_instance_id: Some("pumas-hf-search".to_string()),
        },
    ) {
        Ok(response) => response.event_seq,
        Err(error) => {
            log::warn!("Failed to record Pumas HuggingFace search audit event: {error}");
            None
        }
    }
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

fn validate_pumas_model_id_for_audit(model_id: &str) -> Result<&str, String> {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return Err("model_id is required".to_string());
    }
    if trimmed != model_id || trimmed.chars().any(char::is_whitespace) {
        return Err(
            "model_id must not contain leading, trailing, or embedded whitespace".to_string(),
        );
    }
    if trimmed.len() + "pumas://models/".len() > 128 {
        return Err("model_id is too long for Pumas audit identifiers".to_string());
    }
    if trimmed.starts_with('/') || trimmed.contains('\\') {
        return Err("model_id must be a relative Pumas model identifier".to_string());
    }
    if trimmed
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err("model_id contains an invalid path segment".to_string());
    }
    Ok(trimmed)
}

fn validate_hf_search_query(query: &str) -> Result<&str, String> {
    let trimmed = query.trim();
    if trimmed != query {
        return Err("query must not contain leading or trailing whitespace".to_string());
    }
    if trimmed.len() > 256 || trimmed.chars().any(char::is_control) {
        return Err("query is not a valid HuggingFace search string".to_string());
    }
    Ok(trimmed)
}

fn validate_optional_hf_search_kind(kind: Option<String>) -> Result<Option<String>, String> {
    kind.map(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("kind must not be empty when provided".to_string());
        }
        if trimmed != value || trimmed.len() > 64 || trimmed.chars().any(char::is_control) {
            return Err("kind is not a valid HuggingFace search filter".to_string());
        }
        Ok(value)
    })
    .transpose()
}

fn validate_hf_search_limit(limit: usize) -> Result<usize, String> {
    if limit == 0 || limit > 100 {
        return Err("limit must be between 1 and 100".to_string());
    }
    Ok(limit)
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
    canonical_engine_backend_key(value)
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

    #[test]
    fn validate_pumas_model_id_for_audit_accepts_hf_style_ids() {
        assert_eq!(
            validate_pumas_model_id_for_audit("org/model-name").expect("valid model id"),
            "org/model-name"
        );
    }

    #[test]
    fn validate_pumas_model_id_for_audit_rejects_unsafe_ids() {
        for value in [
            "",
            " model",
            "model id",
            "/absolute",
            "org//model",
            "org/../model",
        ] {
            assert!(
                validate_pumas_model_id_for_audit(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn validate_hf_search_query_accepts_empty_and_text_queries() {
        assert_eq!(validate_hf_search_query("").expect("empty list query"), "");
        assert_eq!(
            validate_hf_search_query("text-to-image").expect("valid search query"),
            "text-to-image"
        );
    }

    #[test]
    fn validate_hf_search_query_rejects_unbounded_or_ambiguous_queries() {
        let oversized = "a".repeat(257);
        for value in [" padded", "padded ", "bad\nquery", oversized.as_str()] {
            assert!(
                validate_hf_search_query(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn validate_hf_search_limit_bounds_queries() {
        assert_eq!(validate_hf_search_limit(1).expect("minimum"), 1);
        assert_eq!(validate_hf_search_limit(100).expect("maximum"), 100);
        assert!(validate_hf_search_limit(0).is_err());
        assert!(validate_hf_search_limit(101).is_err());
    }
}
