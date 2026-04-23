use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use node_engine::ModelDependencyRequest;
use pantograph_runtime_identity::canonical_engine_backend_key;

#[derive(Debug, Clone)]
pub(super) struct ResolvedModelDescriptor {
    pub(super) model_id: String,
    pub(super) model_path: String,
    pub(super) model_type: Option<String>,
    pub(super) task_type_primary: String,
    pub(super) platform_key: String,
    pub(super) backend_key: Option<String>,
    pub(super) selected_binding_ids: Option<Vec<String>>,
    pub(super) model_id_resolved: bool,
}

#[derive(Debug, Clone)]
struct ResolvedPumasModel {
    record: pumas_library::ModelRecord,
    execution_descriptor: Option<pumas_library::models::ModelExecutionDescriptor>,
}

fn stable_platform_key(platform_context: &Option<serde_json::Value>) -> String {
    let mut parts = Vec::new();
    if let Some(context) = platform_context {
        if let Some(os) = context.get("os").and_then(|v| v.as_str()) {
            if !os.trim().is_empty() {
                parts.push(os.to_lowercase());
            }
        }
        if let Some(arch) = context.get("arch").and_then(|v| v.as_str()) {
            if !arch.trim().is_empty() {
                parts.push(arch.to_lowercase());
            }
        }
        if parts.is_empty() {
            parts.push(stable_json(context));
        }
    }

    if parts.is_empty() {
        std::env::consts::OS.to_string() + "-" + std::env::consts::ARCH
    } else {
        parts.join("-")
    }
}

fn stable_json(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(key);
                out.push(':');
                if let Some(v) = map.get(key) {
                    out.push_str(&stable_json(v));
                }
            }
            out.push('}');
            out
        }
        serde_json::Value::Array(items) => {
            let mut out = String::from("[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&stable_json(item));
            }
            out.push(']');
            out
        }
        _ => value.to_string(),
    }
}

fn normalize_path(path: &str) -> String {
    let p = Path::new(path);
    if let Ok(canon) = p.canonicalize() {
        canon.to_string_lossy().to_string()
    } else {
        path.to_string()
    }
}

pub(super) fn cache_key(request: &ModelDependencyRequest) -> String {
    let model_key = request
        .model_id
        .clone()
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| request.model_path.clone());
    let backend_key = request
        .backend_key
        .clone()
        .filter(|b| !b.trim().is_empty())
        .unwrap_or_else(|| "unspecified".to_string());
    let platform_key = stable_platform_key(&request.platform_context);

    let mut selected = request
        .selected_binding_ids
        .iter()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect::<Vec<_>>();
    selected.sort();

    format!(
        "{}|{}|{}|{}",
        model_key,
        backend_key,
        platform_key,
        selected.join(",")
    )
}

fn canonical_backend_key(value: Option<&str>) -> Option<String> {
    canonical_engine_backend_key(value)
}

pub(super) fn infer_engine(
    backend_key: Option<&str>,
    node_type: &str,
    model_type: Option<&str>,
) -> String {
    if let Some(backend) = canonical_backend_key(backend_key) {
        return backend;
    }
    match node_type {
        "audio-generation" => "stable_audio".to_string(),
        "pytorch-inference" => "pytorch".to_string(),
        "diffusion-inference" => "pytorch".to_string(),
        "llamacpp-inference" => "llamacpp".to_string(),
        "reranker" => "llamacpp".to_string(),
        "ollama-inference" => "ollama".to_string(),
        _ => {
            if model_type.unwrap_or_default().eq_ignore_ascii_case("audio") {
                "stable_audio".to_string()
            } else {
                "pytorch".to_string()
            }
        }
    }
}

pub(super) fn map_pipeline_tag_to_task(pipeline_tag: &str) -> String {
    match pipeline_tag.to_lowercase().as_str() {
        "text-to-audio" | "text-to-speech" => "text-to-audio".to_string(),
        "automatic-speech-recognition" => "audio-to-text".to_string(),
        "text-to-image" | "image-to-image" => "text-to-image".to_string(),
        "feature-extraction" | "sentence-similarity" => "feature-extraction".to_string(),
        "reranking" | "reranker" => "reranking".to_string(),
        "image-classification" | "object-detection" | "image-to-text" => {
            "image-to-text".to_string()
        }
        _ => "text-generation".to_string(),
    }
}

fn metadata_string(
    metadata: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<String> {
    keys.iter().find_map(|key| {
        metadata
            .get(*key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    })
}

fn record_metadata_object(
    record: &pumas_library::ModelRecord,
) -> Option<&serde_json::Map<String, serde_json::Value>> {
    record.metadata.as_object()
}

fn record_metadata_string(record: &pumas_library::ModelRecord, keys: &[&str]) -> Option<String> {
    let metadata = record_metadata_object(record)?;
    metadata_string(metadata, keys)
}

fn record_entry_path(record: &pumas_library::ModelRecord) -> Option<String> {
    record_metadata_string(record, &["entry_path", "entryPath"])
        .filter(|path| !path.trim().is_empty())
}

pub(super) fn descriptor_lookup_fallback_allowed(error: &pumas_library::PumasError) -> bool {
    matches!(
        error,
        pumas_library::PumasError::ModelNotFound { .. }
            | pumas_library::PumasError::NotFound { .. }
    )
}

async fn resolve_execution_descriptor_with_api(
    api: &Arc<pumas_library::PumasApi>,
    record: &pumas_library::ModelRecord,
) -> Result<Option<pumas_library::models::ModelExecutionDescriptor>, String> {
    if record.id.trim().is_empty() {
        return Ok(None);
    }

    match api.resolve_model_execution_descriptor(&record.id).await {
        Ok(descriptor) => Ok(Some(descriptor)),
        Err(error) if descriptor_lookup_fallback_allowed(&error) => Ok(None),
        Err(error) => Err(format!(
            "Failed to resolve execution descriptor for '{}': {}",
            record.id, error
        )),
    }
}

pub(super) fn normalized_backend_key(value: &Option<String>) -> Option<String> {
    canonical_backend_key(value.as_deref())
}

fn normalized_selected_binding_ids(value: &[String]) -> Option<Vec<String>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for id in value {
        let trimmed = id.trim();
        if trimmed.is_empty() {
            continue;
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            out.push(owned);
        }
    }
    if out.is_empty() { None } else { Some(out) }
}

pub(super) fn make_requirements_id(
    model_id: &str,
    backend_key: Option<&str>,
    platform_key: &str,
    selected_binding_ids: &[String],
) -> String {
    format!(
        "{}:{}:{}:{}",
        model_id,
        backend_key.unwrap_or("any"),
        platform_key,
        selected_binding_ids.join(",")
    )
}

async fn resolve_model_record_with_api(
    api: &Arc<pumas_library::PumasApi>,
    request: &ModelDependencyRequest,
) -> Result<Option<pumas_library::ModelRecord>, String> {
    if let Some(model_id) = request.model_id.as_deref() {
        if !model_id.trim().is_empty() {
            return api
                .get_model(model_id)
                .await
                .map_err(|e| format!("Failed to query model '{model_id}': {e}"));
        }
    }

    let all = api
        .list_models()
        .await
        .map_err(|e| format!("Failed to list models: {e}"))?;
    let target = normalize_path(&request.model_path);
    Ok(all.into_iter().find(|record| {
        let rp = normalize_path(&record.path);
        if rp == target || target == record.path || record.path == request.model_path {
            return true;
        }

        let Some(entry_path) = record_entry_path(record) else {
            return false;
        };
        let ep = normalize_path(&entry_path);
        ep == target || target == entry_path || entry_path == request.model_path
    }))
}

async fn resolve_model_with_api(
    api: &Arc<pumas_library::PumasApi>,
    request: &ModelDependencyRequest,
) -> Result<Option<ResolvedPumasModel>, String> {
    let Some(record) = resolve_model_record_with_api(api, request).await? else {
        return Ok(None);
    };

    let execution_descriptor = resolve_execution_descriptor_with_api(api, &record).await?;

    Ok(Some(ResolvedPumasModel {
        record,
        execution_descriptor,
    }))
}

pub(super) async fn resolve_descriptor(
    request: &ModelDependencyRequest,
    api: Option<&Arc<pumas_library::PumasApi>>,
) -> Result<ResolvedModelDescriptor, String> {
    let resolved_record = if let Some(api) = api {
        resolve_model_with_api(api, request).await?
    } else {
        None
    };

    let mut model_id = request
        .model_id
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| request.model_path.clone());
    let mut model_path = request.model_path.clone();
    let mut model_type = request.model_type.clone();
    let mut task_type_primary = request
        .task_type_primary
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "text-generation".to_string());

    if let Some(resolved) = resolved_record {
        let record = resolved.record;
        model_id = record.id.clone();
        model_path = resolved
            .execution_descriptor
            .as_ref()
            .map(|descriptor| descriptor.entry_path.clone())
            .unwrap_or_else(|| record.path.clone());
        model_type = resolved
            .execution_descriptor
            .as_ref()
            .map(|descriptor| descriptor.model_type.clone())
            .or_else(|| Some(record.model_type.clone()));
        if let Some(descriptor) = resolved.execution_descriptor.as_ref() {
            let task = descriptor.task_type_primary.trim();
            if !task.is_empty() && task != "unknown" {
                task_type_primary = task.to_string();
            }
        }
        if let Some(meta) = record.metadata.as_object() {
            if let Some(task) = metadata_string(
                meta,
                &[
                    "task_type_primary",
                    "taskTypePrimary",
                    "task_type",
                    "taskType",
                ],
            ) {
                task_type_primary = task;
            } else if let Some(tag) = metadata_string(meta, &["pipeline_tag", "pipelineTag"]) {
                task_type_primary = map_pipeline_tag_to_task(&tag);
            }
        }
    }

    let model_id_resolved = request
        .model_id
        .as_ref()
        .map(|id| !id.trim().is_empty())
        .unwrap_or(false)
        || model_id != request.model_path;

    Ok(ResolvedModelDescriptor {
        model_id,
        model_path,
        model_type,
        task_type_primary,
        platform_key: stable_platform_key(&request.platform_context),
        backend_key: normalized_backend_key(&request.backend_key),
        selected_binding_ids: normalized_selected_binding_ids(&request.selected_binding_ids),
        model_id_resolved,
    })
}
