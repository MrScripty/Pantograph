use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::State;
use workflow_nodes::setup::{PumasSelectedModelDetail, PumasSelectorAccess, PUMAS_SELECTOR_ACCESS};

use super::commands::{SharedExtensions, SharedWorkflowService};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PumaHfDownloadStartAuditResponse {
    pub download_id: String,
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
    extensions: State<'_, SharedExtensions>,
    model_id: Option<String>,
) -> Result<PumaLibNodeHydrationResponse, String> {
    let requested_model_id = clean_optional(model_id);
    let requested_model_id =
        requested_model_id.ok_or_else(|| "model_id is required".to_string())?;

    let selector_access = {
        let ext = extensions.read().await;
        pumas_update_feed_access_from_extensions(&ext)
    };
    let selector_access = selector_access.ok_or_else(|| {
        "Pumas selector access not available in executor extensions for model_id hydration"
            .to_string()
    })?;
    let option =
        find_matching_model_option_from_selector_access(&selector_access, &requested_model_id)
            .await?;

    let node_data = build_hydrated_node_data(&option)?;

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

pub async fn start_hf_download_with_audit(
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    request: pumas_library::model_library::DownloadRequest,
) -> Result<PumaHfDownloadStartAuditResponse, String> {
    validate_hf_repo_id_for_audit(&request.repo_id)?;
    let api = require_pumas_api(&extensions).await?;
    let download_id = api
        .start_hf_download(&request)
        .await
        .map_err(|error| error.to_string())?;
    let audit_event_seq = record_hf_model_download_audit(&workflow_service, &request.repo_id);

    Ok(PumaHfDownloadStartAuditResponse {
        download_id,
        audit_event_seq,
    })
}

pub async fn model_package_facts_summary_snapshot(
    extensions: State<'_, SharedExtensions>,
    limit: Option<usize>,
    offset: Option<usize>,
) -> Result<pumas_library::models::ModelPackageFactsSummarySnapshot, String> {
    let limit = validate_pumas_model_library_page_limit(limit.unwrap_or(100))?;
    let offset = offset.unwrap_or(0);
    let selector_access = {
        let ext = extensions.read().await;
        pumas_update_feed_access_from_extensions(&ext)
    };
    model_package_facts_summary_snapshot_from_access(selector_access, limit, offset).await
}

pub async fn resolve_model_package_facts_summary(
    extensions: State<'_, SharedExtensions>,
    model_id: String,
) -> Result<pumas_library::models::ModelPackageFactsSummaryResult, String> {
    let model_id = validate_pumas_model_id_for_lookup(&model_id)?;
    let selector_access = {
        let ext = extensions.read().await;
        pumas_update_feed_access_from_extensions(&ext)
    };
    resolve_model_package_facts_summary_from_access(selector_access, &model_id).await
}

pub async fn list_model_library_updates_since(
    extensions: State<'_, SharedExtensions>,
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<pumas_library::models::ModelLibraryUpdateFeed, String> {
    let cursor = validate_optional_pumas_update_cursor(cursor)?;
    let limit = validate_pumas_model_library_page_limit(limit.unwrap_or(100))?;
    let selector_access = {
        let ext = extensions.read().await;
        pumas_update_feed_access_from_extensions(&ext)
    };
    list_model_library_updates_since_from_access(selector_access, cursor.as_deref(), limit).await
}

async fn list_model_library_updates_since_from_extensions(
    extensions: &node_engine::ExecutorExtensions,
    cursor: Option<&str>,
    limit: usize,
) -> Result<pumas_library::models::ModelLibraryUpdateFeed, String> {
    let selector_access = pumas_update_feed_access_from_extensions(extensions);
    list_model_library_updates_since_from_access(selector_access, cursor, limit).await
}

async fn model_package_facts_summary_snapshot_from_extensions(
    extensions: &node_engine::ExecutorExtensions,
    limit: usize,
    offset: usize,
) -> Result<pumas_library::models::ModelPackageFactsSummarySnapshot, String> {
    let selector_access = pumas_update_feed_access_from_extensions(extensions);
    model_package_facts_summary_snapshot_from_access(selector_access, limit, offset).await
}

async fn model_package_facts_summary_snapshot_from_access(
    selector_access: Option<Arc<PumasSelectorAccess>>,
    limit: usize,
    offset: usize,
) -> Result<pumas_library::models::ModelPackageFactsSummarySnapshot, String> {
    if let Some(selector_access) = selector_access {
        return selector_access
            .model_package_facts_summary_snapshot(limit, offset)
            .await
            .map_err(|error| error.to_string());
    }

    Err("Pumas selector access not available in executor extensions".to_string())
}

async fn resolve_model_package_facts_summary_from_extensions(
    extensions: &node_engine::ExecutorExtensions,
    model_id: &str,
) -> Result<pumas_library::models::ModelPackageFactsSummaryResult, String> {
    let selector_access = pumas_update_feed_access_from_extensions(extensions);
    resolve_model_package_facts_summary_from_access(selector_access, model_id).await
}

async fn resolve_model_package_facts_summary_from_access(
    selector_access: Option<Arc<PumasSelectorAccess>>,
    model_id: &str,
) -> Result<pumas_library::models::ModelPackageFactsSummaryResult, String> {
    if let Some(selector_access) = selector_access {
        return selector_access
            .resolve_model_package_facts_summary(model_id)
            .await
            .map_err(|error| error.to_string());
    }

    Err("Pumas selector access not available in executor extensions".to_string())
}

fn pumas_update_feed_access_from_extensions(
    extensions: &node_engine::ExecutorExtensions,
) -> Option<Arc<PumasSelectorAccess>> {
    extensions
        .get::<Arc<PumasSelectorAccess>>(PUMAS_SELECTOR_ACCESS)
        .cloned()
}

async fn list_model_library_updates_since_from_access(
    selector_access: Option<Arc<PumasSelectorAccess>>,
    cursor: Option<&str>,
    limit: usize,
) -> Result<pumas_library::models::ModelLibraryUpdateFeed, String> {
    if let Some(selector_access) = selector_access {
        return selector_access
            .list_model_library_updates_since(cursor, limit)
            .await
            .map_err(|error| error.to_string());
    }

    Err("Pumas selector access not available in executor extensions".to_string())
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

fn record_hf_model_download_audit(
    workflow_service: &SharedWorkflowService,
    repo_id: &str,
) -> Option<i64> {
    match workflow_service.workflow_library_asset_access_record(
        pantograph_workflow_service::WorkflowLibraryAssetAccessRecordRequest {
            asset_id: format!("hf://models/{repo_id}"),
            operation: pantograph_workflow_service::LibraryAssetOperation::Download,
            cache_status: Some(pantograph_workflow_service::LibraryAssetCacheStatus::Unknown),
            network_bytes: None,
            source_instance_id: Some("pumas-hf-download".to_string()),
        },
    ) {
        Ok(response) => response.event_seq,
        Err(error) => {
            log::warn!("Failed to record Pumas HuggingFace download audit event: {error}");
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

async fn find_matching_model_option_from_selector_access(
    selector_access: &Arc<PumasSelectorAccess>,
    model_id: &str,
) -> Result<node_engine::PortOption, String> {
    let detail = selector_access
        .selected_model_detail(model_id)
        .await
        .map_err(|error| error.to_string())?;
    build_selected_model_option_from_detail(model_id, detail)
}

fn build_selected_model_option_from_detail(
    model_id: &str,
    detail: PumasSelectedModelDetail,
) -> Result<node_engine::PortOption, String> {
    let row = detail.selector_row.as_ref();
    let descriptor = detail.descriptor.as_ref();
    let summary_result = detail.package_summary_result.as_ref();
    let model_ref = row.map(|row| row.model_ref.clone()).unwrap_or_else(|| {
        pumas_library::models::PumasModelRef {
            model_id: model_id.to_string(),
            ..Default::default()
        }
    });
    let display_name = row
        .map(|row| row.display_name.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| model_id.to_string());
    let model_type = descriptor
        .map(|descriptor| descriptor.model_type.clone())
        .or_else(|| row.and_then(|row| row.model_type.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    let tags = row
        .map(|row| row.tags.clone())
        .unwrap_or_default()
        .join(", ");
    let model_ref_value = serde_json::to_value(&model_ref).map_err(|error| error.to_string())?;

    Ok(node_engine::PortOption {
        value: model_ref_value.clone(),
        label: display_name.clone(),
        description: Some(if tags.is_empty() {
            model_type.clone()
        } else {
            format!("{model_type} | {tags}")
        }),
        metadata: Some(json!({
            "id": model_ref.model_id,
            "model_ref": model_ref_value.clone(),
            "pumas_model_ref": model_ref_value,
            "model_type": model_type,
            "cleaned_name": display_name,
            "pipeline_tag": summary_result
                .and_then(|result| {
                    result.summary.as_ref().and_then(|summary| summary.task.pipeline_tag.clone())
                })
                .or_else(|| row.and_then(|row| row.pipeline_tag.clone())),
            "task_type_primary": descriptor
                .map(|descriptor| descriptor.task_type_primary.clone())
                .or_else(|| {
                    summary_result.and_then(|result| {
                        result.summary.as_ref().and_then(|summary| {
                            summary.task.task_type_primary.clone()
                        })
                    })
                })
                .or_else(|| row.and_then(|row| row.task_type_primary.clone())),
            "recommended_backend": descriptor
                .and_then(|descriptor| descriptor.recommended_backend.clone())
                .or_else(|| row.and_then(|row| row.recommended_backend.clone())),
        })),
        disabled: false,
        unavailable_state: None,
        unavailable_reason_code: None,
        unavailable_reason: None,
    })
}

fn build_hydrated_node_data(option: &node_engine::PortOption) -> Result<Value, String> {
    let metadata = option
        .metadata
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| "Puma-Lib option metadata is missing".to_string())?;
    let pumas_model_ref = metadata
        .get("pumas_model_ref")
        .or_else(|| metadata.get("model_ref"))
        .cloned()
        .filter(|value| !value.is_null())
        .ok_or_else(|| "Puma-Lib option is missing pumas_model_ref".to_string())?;
    let model_id = pumas_model_ref
        .get("model_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| metadata.get("id").and_then(Value::as_str).map(str::trim))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Puma-Lib option is missing pumas_model_ref.model_id".to_string())?
        .to_string();

    let node_data = json!({
        "modelName": option.label,
        "model_id": model_id,
        "pumas_model_ref": pumas_model_ref,
    });

    Ok(node_data)
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

fn validate_pumas_model_id_for_lookup(model_id: &str) -> Result<&str, String> {
    let trimmed = model_id.trim();
    if trimmed.is_empty() {
        return Err("model_id is required".to_string());
    }
    if trimmed != model_id || trimmed.chars().any(char::is_control) {
        return Err("model_id is not a valid Pumas model identifier".to_string());
    }
    Ok(trimmed)
}

fn validate_pumas_model_library_page_limit(limit: usize) -> Result<usize, String> {
    if limit == 0 || limit > 1000 {
        return Err("limit must be between 1 and 1000".to_string());
    }
    Ok(limit)
}

fn validate_optional_pumas_update_cursor(cursor: Option<String>) -> Result<Option<String>, String> {
    cursor
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            if trimmed != value || trimmed.chars().any(char::is_control) || trimmed.len() > 128 {
                return Err("cursor is not a valid Pumas model-library update cursor".to_string());
            }
            Ok(Some(value))
        })
        .transpose()
        .map(Option::flatten)
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

fn validate_hf_repo_id_for_audit(repo_id: &str) -> Result<&str, String> {
    let trimmed = repo_id.trim();
    if trimmed.is_empty() {
        return Err("repo_id is required".to_string());
    }
    if trimmed != repo_id || trimmed.chars().any(char::is_whitespace) {
        return Err(
            "repo_id must not contain leading, trailing, or embedded whitespace".to_string(),
        );
    }
    if trimmed.len() + "hf://models/".len() > 128 {
        return Err("repo_id is too long for HuggingFace audit identifiers".to_string());
    }
    if trimmed.starts_with('/') || trimmed.contains('\\') {
        return Err("repo_id must be a relative HuggingFace repository identifier".to_string());
    }
    if trimmed
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err("repo_id contains an invalid path segment".to_string());
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_env() -> TempDir {
        let temp_dir = TempDir::new().expect("temporary Pumas root should be created");
        std::fs::create_dir_all(temp_dir.path().join("launcher-data")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
        temp_dir
    }

    fn write_library_owned_file_model(
        model_dir: &std::path::Path,
        model_id: &str,
    ) -> std::path::PathBuf {
        std::fs::create_dir_all(model_dir).unwrap();
        let model_file = model_dir.join("model.gguf");
        std::fs::write(&model_file, vec![0_u8; 256]).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": model_id,
                "family": "imported",
                "model_type": "llm",
                "official_name": "test-gguf",
                "cleaned_name": "test-gguf",
                "source_path": model_dir.display().to_string(),
                "entry_path": model_file.display().to_string(),
                "storage_kind": "library_owned",
                "import_state": "ready",
                "validation_state": "valid",
                "task_type_primary": "text-generation",
                "recommended_backend": "llamacpp",
                "runtime_engine_hints": ["llamacpp"]
            })
            .to_string(),
        )
        .unwrap();
        model_file
    }

    fn sample_option() -> node_engine::PortOption {
        node_engine::PortOption {
            value: json!("/models/tiny-sd-turbo"),
            label: "Tiny SD Turbo".to_string(),
            description: None,
            metadata: Some(json!({
                "id": "diffusion/cc-nms/tiny-sd-turbo",
                "pumas_model_ref": {
                    "model_id": "diffusion/cc-nms/tiny-sd-turbo"
                },
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
            disabled: false,
            unavailable_state: None,
            unavailable_reason_code: None,
            unavailable_reason: None,
        }
    }

    #[test]
    fn build_hydrated_node_data_uses_backend_owned_defaults() {
        let node_data = build_hydrated_node_data(&sample_option()).expect("node data");

        assert_eq!(node_data["modelName"], json!("Tiny SD Turbo"));
        assert_eq!(
            node_data["model_id"],
            json!("diffusion/cc-nms/tiny-sd-turbo")
        );
        assert_eq!(
            node_data["pumas_model_ref"],
            json!({ "model_id": "diffusion/cc-nms/tiny-sd-turbo" })
        );
        assert!(node_data.get("modelPath").is_none());
        assert!(node_data.get("selected_binding_ids").is_none());
        assert!(node_data.get("entry_path").is_none());
        assert!(node_data.get("inference_settings").is_none());
        assert!(node_data.get("dependency_requirements").is_none());
    }

    #[tokio::test]
    async fn find_matching_model_option_hydrates_only_selected_model_detail() {
        let temp_dir = create_test_env();
        let model_id = "llm/imported/test-gguf";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_library_owned_file_model(&model_dir, model_id);
        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .build()
                .await
                .unwrap(),
        );
        api.rebuild_model_index().await.unwrap();
        let selector_access = Arc::new(PumasSelectorAccess::Owner(api));

        let option = find_matching_model_option_from_selector_access(&selector_access, model_id)
            .await
            .expect("selected model option should hydrate");
        assert_eq!(option.value["model_id"], json!(model_id));
        let metadata = option
            .metadata
            .as_ref()
            .and_then(Value::as_object)
            .expect("selected option metadata should be an object");
        assert_eq!(metadata["id"], json!(model_id));
        assert_eq!(
            metadata["pumas_model_ref"]["model_id"],
            serde_json::json!(model_id)
        );
        assert!(metadata.get("entry_path").is_none());
        assert!(metadata.get("execution_contract_version").is_none());
        assert!(metadata.get("inference_settings").is_none());
    }

    #[tokio::test]
    async fn find_matching_model_option_hydrates_read_only_selector_row_without_owner_api() {
        let temp_dir = create_test_env();
        let model_id = "llm/imported/test-gguf";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_library_owned_file_model(&model_dir, model_id);
        let api = pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .unwrap();
        api.rebuild_model_index().await.unwrap();
        let read_only = pumas_library::PumasReadOnlyLibrary::open(
            temp_dir.path().join("shared-resources/models"),
        )
        .unwrap();
        let selector_access = Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only)));

        let option = find_matching_model_option_from_selector_access(&selector_access, model_id)
            .await
            .expect("read-only selector row should hydrate enough selected model data");

        assert_eq!(option.value["model_id"], json!(model_id));
        let metadata = option
            .metadata
            .as_ref()
            .and_then(Value::as_object)
            .expect("selected option metadata should be an object");
        assert_eq!(metadata["id"], json!(model_id));
        assert_eq!(
            metadata["pumas_model_ref"]["model_id"],
            serde_json::json!(model_id)
        );
        assert_eq!(metadata["recommended_backend"], json!("llamacpp"));
        assert!(metadata.get("entry_path").is_none());
        assert!(metadata.get("inference_settings").is_none());
    }

    #[tokio::test]
    async fn list_model_library_updates_since_uses_owner_selector_access() {
        let temp_dir = create_test_env();
        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .build()
                .await
                .unwrap(),
        );
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set(
            PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::Owner(api)),
        );

        let feed = list_model_library_updates_since_from_extensions(&extensions, None, 100)
            .await
            .expect("owner selector access update feed should load");

        assert!(feed.cursor.starts_with("model-library-updates:"));
        assert!(!feed.stale_cursor);
        assert!(!feed.snapshot_required);
    }

    #[tokio::test]
    async fn list_model_library_updates_since_prefers_selector_access_role() {
        let temp_dir = TempDir::new().unwrap();
        let _index = pumas_library::ModelIndex::new(temp_dir.path().join("models.db")).unwrap();
        let read_only = pumas_library::PumasReadOnlyLibrary::open(temp_dir.path()).unwrap();
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set(
            PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
        );

        let error = list_model_library_updates_since_from_extensions(
            &extensions,
            Some("model-library-updates:1"),
            100,
        )
        .await
        .unwrap_err();

        assert!(
            error.contains("read-only Pumas selector access does not provide update feeds"),
            "unexpected error: {error}"
        );
        assert!(
            extensions
                .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .is_none(),
            "read-only selector access must not require raw PUMAS_API"
        );
    }

    #[tokio::test]
    async fn package_facts_summary_snapshot_uses_read_only_selector_access_without_pumas_api() {
        let temp_dir = create_test_env();
        let model_id = "llm/imported/test-gguf";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_library_owned_file_model(&model_dir, model_id);
        let api = pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .unwrap();
        api.rebuild_model_index().await.unwrap();
        let read_only = pumas_library::PumasReadOnlyLibrary::open(
            temp_dir.path().join("shared-resources/models"),
        )
        .unwrap();
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set(
            PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
        );

        let snapshot = model_package_facts_summary_snapshot_from_extensions(&extensions, 100, 0)
            .await
            .expect("read-only selector summary snapshot should load");

        assert!(snapshot.cursor.starts_with("model-library-updates:"));
        assert_eq!(snapshot.items.len(), 1);
        assert_eq!(snapshot.items[0].model_id, model_id);
    }

    #[tokio::test]
    async fn resolve_package_facts_summary_uses_read_only_selector_access_without_pumas_api() {
        let temp_dir = create_test_env();
        let model_id = "llm/imported/test-gguf";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_library_owned_file_model(&model_dir, model_id);
        let api = pumas_library::PumasApi::builder(temp_dir.path())
            .build()
            .await
            .unwrap();
        api.rebuild_model_index().await.unwrap();
        let read_only = pumas_library::PumasReadOnlyLibrary::open(
            temp_dir.path().join("shared-resources/models"),
        )
        .unwrap();
        let mut extensions = node_engine::ExecutorExtensions::new();
        extensions.set(
            PUMAS_SELECTOR_ACCESS,
            Arc::new(PumasSelectorAccess::ReadOnly(Arc::new(read_only))),
        );

        let summary = resolve_model_package_facts_summary_from_extensions(&extensions, model_id)
            .await
            .expect("read-only selector summary should resolve");

        assert_eq!(summary.model_id, model_id);
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
    fn validate_pumas_model_library_page_limit_bounds_queries() {
        assert_eq!(
            validate_pumas_model_library_page_limit(1).expect("minimum"),
            1
        );
        assert_eq!(
            validate_pumas_model_library_page_limit(1000).expect("maximum"),
            1000
        );
        assert!(validate_pumas_model_library_page_limit(0).is_err());
        assert!(validate_pumas_model_library_page_limit(1001).is_err());
    }

    #[test]
    fn validate_optional_pumas_update_cursor_rejects_ambiguous_cursors() {
        assert_eq!(
            validate_optional_pumas_update_cursor(Some("model-library-updates:42".to_string()))
                .expect("valid cursor"),
            Some("model-library-updates:42".to_string())
        );
        assert_eq!(
            validate_optional_pumas_update_cursor(Some("  ".to_string()))
                .expect("blank cursor clears"),
            None
        );

        for value in [" padded", "bad\ncursor"] {
            assert!(
                validate_optional_pumas_update_cursor(Some(value.to_string())).is_err(),
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

    #[test]
    fn validate_hf_repo_id_for_audit_accepts_hf_style_ids() {
        assert_eq!(
            validate_hf_repo_id_for_audit("org/model-name").expect("valid repo id"),
            "org/model-name"
        );
    }

    #[test]
    fn validate_hf_repo_id_for_audit_rejects_unsafe_ids() {
        for value in [
            "",
            " repo",
            "repo id",
            "/absolute",
            "org//model",
            "org/../model",
        ] {
            assert!(
                validate_hf_repo_id_for_audit(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }
}
