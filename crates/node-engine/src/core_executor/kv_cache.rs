use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use inference::kv_cache::{
    CacheMarker, KvCacheCodec, KvCacheEntry, KvCacheHandle, KvCacheMetadata, KvCacheStore,
    ModelFingerprint, StoragePolicy,
};
use inference::InferenceGateway;
use uuid::Uuid;

use crate::core_executor::require_gateway;
use crate::error::{NodeEngineError, Result};
use crate::events::{
    EventSink, KvCacheEventAction, KvCacheEventOutcome, KvCacheExecutionDiagnostics,
    TaskProgressDetail, WorkflowEvent,
};
use crate::extensions::ExecutorExtensions;

const LLAMACPP_SLOT_ID: u32 = 0;

fn kv_reuse_source(metadata: &KvCacheMetadata) -> Option<String> {
    metadata
        .extra
        .get("source")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn emit_kv_cache_detail(
    event_sink: Option<&Arc<dyn EventSink>>,
    task_id: &str,
    execution_id: &str,
    progress: f32,
    message: impl Into<String>,
    detail: KvCacheExecutionDiagnostics,
) {
    let Some(event_sink) = event_sink else {
        return;
    };

    let _ = event_sink.send(WorkflowEvent::task_progress_with_detail(
        task_id,
        execution_id,
        progress,
        Some(message.into()),
        TaskProgressDetail::KvCache(detail),
    ));
}

pub(super) async fn execute_save(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    let store = require_store(extensions)?;

    let cache_handle_val = inputs
        .get("cache_data")
        .ok_or_else(|| NodeEngineError::MissingInput("cache_data".to_string()))?;
    let cache_handle: KvCacheHandle = serde_json::from_value(cache_handle_val.clone())?;
    let model_fingerprint = read_legacy_model_fingerprint(inputs)?
        .unwrap_or_else(|| cache_handle.compatibility.model_fingerprint.clone());
    if model_fingerprint != cache_handle.compatibility.model_fingerprint {
        return Err(NodeEngineError::ExecutionFailed(
            "Legacy model_fingerprint input does not match the KV cache handle".to_string(),
        ));
    }

    let source_entry = store
        .load(&cache_handle.cache_id, &model_fingerprint)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("KV cache load failed: {error}"))
        })?;

    let label = inputs
        .get("label")
        .and_then(|value| value.as_str())
        .map(String::from)
        .or_else(|| source_entry.metadata.label.clone());
    let compressed = inputs
        .get("compressed")
        .and_then(|value| value.as_bool())
        .unwrap_or(source_entry.metadata.compressed);
    let backend_hint = inputs
        .get("_data")
        .and_then(|data| data.get("backend_hint"))
        .and_then(|backend| backend.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| source_entry.metadata.backend_hint.clone());
    let storage_policy = parse_storage_policy(inputs);
    let cache_dir = inputs
        .get("cache_dir")
        .and_then(|value| value.as_str())
        .map(PathBuf::from);
    let markers = match inputs.get("markers") {
        Some(_) => parse_markers(inputs)?,
        None => source_entry.metadata.markers.clone(),
    };

    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label,
            model_fingerprint,
            runtime_fingerprint: source_entry
                .metadata
                .runtime_fingerprint
                .clone()
                .or_else(|| Some(cache_handle.compatibility.runtime_fingerprint.clone())),
            backend_hint,
            token_count: source_entry.metadata.token_count,
            markers,
            created_at: 0,
            updated_at: 0,
            compressed,
            extra: source_entry.metadata.extra.clone(),
        },
        data: source_entry.data,
    };

    let cache_id = match cache_dir {
        Some(path) => store.save_to(entry, path, Some(storage_policy)).await,
        None => store.save(entry, Some(storage_policy)).await,
    }
    .map_err(|error| NodeEngineError::ExecutionFailed(format!("KV cache save failed: {error}")))?;

    let metadata = store.get_metadata(&cache_id).await.map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Failed to read metadata: {error}"))
    })?;

    let mut outputs = HashMap::new();
    outputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    outputs.insert("metadata".to_string(), serde_json::to_value(&metadata)?);
    Ok(outputs)
}

pub(super) async fn restore_llamacpp_input_handle(
    inputs: &HashMap<String, serde_json::Value>,
    gateway: &Arc<InferenceGateway>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
) -> Result<bool> {
    let Some(handle_value) = inputs.get("kv_cache_in").filter(|value| !value.is_null()) else {
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "KV cache input not provided",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Miss,
                cache_id: None,
                backend_key: None,
                reuse_source: None,
                token_count: None,
                reason: Some("no_input_handle".to_string()),
            },
        );
        return Ok(false);
    };

    let handle: KvCacheHandle = serde_json::from_value(handle_value.clone())?;
    let store = require_store(extensions)?;
    let runtime_fingerprint = gateway
        .kv_cache_runtime_fingerprint()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "KV cache runtime fingerprint lookup failed: {}",
                error
            ))
        })?;
    let model_fingerprint = gateway
        .kv_cache_model_fingerprint()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "KV cache model fingerprint lookup failed: {}",
                error
            ))
        })?;

    if !handle.is_compatible_with(&model_fingerprint, &runtime_fingerprint) {
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "KV cache input invalidated for active llama.cpp runtime",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Invalidated,
                cache_id: Some(handle.cache_id.clone()),
                backend_key: Some(handle.compatibility.runtime_fingerprint.backend_key.clone()),
                reuse_source: None,
                token_count: None,
                reason: Some("incompatible_runtime_or_model".to_string()),
            },
        );
        return Ok(false);
    }

    let entry = match store
        .load_for_execution(&handle.cache_id, &model_fingerprint, &runtime_fingerprint)
        .await
    {
        Ok(entry) => entry,
        Err(error) => {
            log::warn!("KV cache load failed for '{}': {}", handle.cache_id, error);
            emit_kv_cache_detail(
                event_sink,
                task_id,
                execution_id,
                0.0,
                "KV cache input invalidated after load failure",
                KvCacheExecutionDiagnostics {
                    action: KvCacheEventAction::RestoreInput,
                    outcome: KvCacheEventOutcome::Invalidated,
                    cache_id: Some(handle.cache_id.clone()),
                    backend_key: Some(handle.compatibility.runtime_fingerprint.backend_key.clone()),
                    reuse_source: None,
                    token_count: None,
                    reason: Some("load_failed".to_string()),
                },
            );
            return Ok(false);
        }
    };
    let slot_path = kv_slot_temp_path("restore", handle.cache_id.as_str());
    fs::write(&slot_path, &entry.data).map_err(|error| {
        NodeEngineError::ExecutionFailed(format!(
            "Failed to write temporary KV cache slot file '{}': {}",
            slot_path.display(),
            error
        ))
    })?;

    let restore_result = gateway
        .restore_kv_cache_slot(LLAMACPP_SLOT_ID, &slot_path)
        .await;
    let _ = fs::remove_file(&slot_path);
    match restore_result {
        Ok(()) => {
            emit_kv_cache_detail(
                event_sink,
                task_id,
                execution_id,
                0.0,
                "KV cache input restored",
                KvCacheExecutionDiagnostics {
                    action: KvCacheEventAction::RestoreInput,
                    outcome: KvCacheEventOutcome::Hit,
                    cache_id: Some(handle.cache_id),
                    backend_key: Some(entry.metadata.backend_hint.clone()),
                    reuse_source: kv_reuse_source(&entry.metadata),
                    token_count: Some(entry.metadata.token_count),
                    reason: Some("restored_input_handle".to_string()),
                },
            );
            Ok(true)
        }
        Err(error) => {
            log::warn!(
                "KV cache slot restore failed for '{}': {}",
                handle.cache_id,
                error
            );
            emit_kv_cache_detail(
                event_sink,
                task_id,
                execution_id,
                0.0,
                "KV cache input invalidated after restore failure",
                KvCacheExecutionDiagnostics {
                    action: KvCacheEventAction::RestoreInput,
                    outcome: KvCacheEventOutcome::Invalidated,
                    cache_id: Some(handle.cache_id),
                    backend_key: Some(runtime_fingerprint.backend_key),
                    reuse_source: kv_reuse_source(&entry.metadata),
                    token_count: Some(entry.metadata.token_count),
                    reason: Some("restore_failed".to_string()),
                },
            );
            Ok(false)
        }
    }
}

pub(super) async fn capture_llamacpp_output_handle(
    task_id: &str,
    execution_id: &str,
    gateway: &Arc<InferenceGateway>,
    extensions: &ExecutorExtensions,
    event_sink: Option<&Arc<dyn EventSink>>,
) -> Result<serde_json::Value> {
    let Some(store) = extensions.get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
    else {
        return Ok(serde_json::Value::Null);
    };

    let runtime_fingerprint = gateway
        .kv_cache_runtime_fingerprint()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "KV cache runtime fingerprint lookup failed: {}",
                error
            ))
        })?;
    let model_fingerprint = gateway
        .kv_cache_model_fingerprint()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "KV cache model fingerprint lookup failed: {}",
                error
            ))
        })?;

    let slot_path = kv_slot_temp_path("capture", task_id);
    let save_result = gateway
        .save_kv_cache_slot(LLAMACPP_SLOT_ID, &slot_path)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("KV cache slot save failed: {}", error))
        });
    if let Err(error) = save_result {
        let _ = fs::remove_file(&slot_path);
        return Err(error);
    }

    let data = fs::read(&slot_path).map_err(|error| {
        NodeEngineError::ExecutionFailed(format!(
            "Failed to read temporary KV cache slot file '{}': {}",
            slot_path.display(),
            error
        ))
    })?;
    let _ = fs::remove_file(&slot_path);

    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some(format!("{} KV Cache", task_id)),
            model_fingerprint,
            runtime_fingerprint: Some(runtime_fingerprint.clone()),
            backend_hint: runtime_fingerprint.backend_key.clone(),
            token_count: 0,
            markers: Vec::new(),
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({
                "source": "llamacpp_slot",
                "slotId": LLAMACPP_SLOT_ID,
            }),
        },
        data,
    };
    let cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("KV cache save failed: {error}"))
        })?;
    let metadata = store.get_metadata(&cache_id).await.map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Failed to read KV metadata: {error}"))
    })?;

    let handle = metadata.executable_handle().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Saved KV metadata did not produce an executable handle".to_string(),
        )
    })?;
    emit_kv_cache_detail(
        event_sink,
        task_id,
        execution_id,
        1.0,
        "KV cache output captured",
        KvCacheExecutionDiagnostics {
            action: KvCacheEventAction::CaptureOutput,
            outcome: KvCacheEventOutcome::Saved,
            cache_id: Some(metadata.cache_id.clone()),
            backend_key: Some(metadata.backend_hint.clone()),
            reuse_source: kv_reuse_source(&metadata),
            token_count: Some(metadata.token_count),
            reason: Some("captured_output_handle".to_string()),
        },
    );
    serde_json::to_value(&handle).map_err(Into::into)
}

#[cfg(feature = "pytorch-nodes")]
pub(super) async fn restore_pytorch_input_handle(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
) -> Result<bool> {
    let active_model = inference::backend::pytorch::active_loaded_model_info()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "PyTorch loaded-model lookup failed: {}",
                error
            ))
        })?;

    let Some(handle_value) = inputs.get("kv_cache_in").filter(|value| !value.is_null()) else {
        inference::backend::pytorch::clear_live_kv_snapshot()
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch live KV clear failed: {}", error))
            })?;
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "KV cache input not provided",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Miss,
                cache_id: None,
                backend_key: None,
                reuse_source: None,
                token_count: None,
                reason: Some("no_input_handle".to_string()),
            },
        );
        return Ok(false);
    };

    if !inference::backend::pytorch::supports_live_kv_reuse(&active_model.model_type) {
        inference::backend::pytorch::clear_live_kv_snapshot()
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch live KV clear failed: {}", error))
            })?;
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "PyTorch runtime does not support live KV reuse for this model",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Unsupported,
                cache_id: None,
                backend_key: Some("pytorch".to_string()),
                reuse_source: None,
                token_count: None,
                reason: Some("live_reuse_unsupported_for_model_type".to_string()),
            },
        );
        return Ok(false);
    }

    let handle: KvCacheHandle = serde_json::from_value(handle_value.clone())?;
    let store = require_store(extensions)?;
    let runtime_fingerprint =
        inference::backend::pytorch::kv_cache_runtime_fingerprint_for_loaded_model(&active_model);
    let model_fingerprint =
        inference::backend::pytorch::kv_cache_model_fingerprint_for_loaded_model(&active_model);

    if !handle.is_compatible_with(&model_fingerprint, &runtime_fingerprint) {
        inference::backend::pytorch::clear_live_kv_snapshot()
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch live KV clear failed: {}", error))
            })?;
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "KV cache input invalidated for active PyTorch runtime",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Invalidated,
                cache_id: Some(handle.cache_id.clone()),
                backend_key: Some(handle.compatibility.runtime_fingerprint.backend_key.clone()),
                reuse_source: None,
                token_count: None,
                reason: Some("incompatible_runtime_or_model".to_string()),
            },
        );
        return Ok(false);
    }

    let entry = match store
        .load_for_execution(&handle.cache_id, &model_fingerprint, &runtime_fingerprint)
        .await
    {
        Ok(entry) => entry,
        Err(error) => {
            log::warn!("KV cache load failed for '{}': {}", handle.cache_id, error);
            inference::backend::pytorch::clear_live_kv_snapshot()
                .await
                .map_err(|clear_error| {
                    NodeEngineError::ExecutionFailed(format!(
                        "PyTorch live KV clear failed: {}",
                        clear_error
                    ))
                })?;
            emit_kv_cache_detail(
                event_sink,
                task_id,
                execution_id,
                0.0,
                "KV cache input invalidated after load failure",
                KvCacheExecutionDiagnostics {
                    action: KvCacheEventAction::RestoreInput,
                    outcome: KvCacheEventOutcome::Invalidated,
                    cache_id: Some(handle.cache_id.clone()),
                    backend_key: Some(handle.compatibility.runtime_fingerprint.backend_key.clone()),
                    reuse_source: None,
                    token_count: None,
                    reason: Some("load_failed".to_string()),
                },
            );
            return Ok(false);
        }
    };
    let snapshot_path = kv_slot_temp_path("pytorch-restore", handle.cache_id.as_str());
    fs::write(&snapshot_path, &entry.data).map_err(|error| {
        NodeEngineError::ExecutionFailed(format!(
            "Failed to write temporary PyTorch KV snapshot file '{}': {}",
            snapshot_path.display(),
            error
        ))
    })?;

    let restore_result =
        inference::backend::pytorch::restore_live_kv_snapshot(&snapshot_path).await;
    let _ = fs::remove_file(&snapshot_path);
    let restored_info = match restore_result {
        Ok(restored_info) => restored_info,
        Err(error) => {
            log::warn!(
                "PyTorch KV snapshot restore failed for '{}': {}",
                handle.cache_id,
                error
            );
            inference::backend::pytorch::clear_live_kv_snapshot()
                .await
                .map_err(|clear_error| {
                    NodeEngineError::ExecutionFailed(format!(
                        "PyTorch live KV clear failed: {}",
                        clear_error
                    ))
                })?;
            emit_kv_cache_detail(
                event_sink,
                task_id,
                execution_id,
                0.0,
                "KV cache input invalidated after restore failure",
                KvCacheExecutionDiagnostics {
                    action: KvCacheEventAction::RestoreInput,
                    outcome: KvCacheEventOutcome::Invalidated,
                    cache_id: Some(handle.cache_id.clone()),
                    backend_key: Some(runtime_fingerprint.backend_key.clone()),
                    reuse_source: kv_reuse_source(&entry.metadata),
                    token_count: Some(entry.metadata.token_count),
                    reason: Some("restore_failed".to_string()),
                },
            );
            return Ok(false);
        }
    };

    let restored_runtime =
        inference::backend::pytorch::kv_cache_runtime_fingerprint_for_live_kv(&restored_info);
    let restored_model =
        inference::backend::pytorch::kv_cache_model_fingerprint_for_live_kv(&restored_info);
    if restored_runtime != runtime_fingerprint || restored_model != model_fingerprint {
        inference::backend::pytorch::clear_live_kv_snapshot()
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch live KV clear failed: {}", error))
            })?;
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            0.0,
            "KV cache input invalidated after restored snapshot mismatch",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::RestoreInput,
                outcome: KvCacheEventOutcome::Invalidated,
                cache_id: Some(handle.cache_id),
                backend_key: Some(runtime_fingerprint.backend_key),
                reuse_source: kv_reuse_source(&entry.metadata),
                token_count: Some(entry.metadata.token_count),
                reason: Some("restored_snapshot_mismatch".to_string()),
            },
        );
        return Ok(false);
    }

    emit_kv_cache_detail(
        event_sink,
        task_id,
        execution_id,
        0.0,
        "KV cache input restored",
        KvCacheExecutionDiagnostics {
            action: KvCacheEventAction::RestoreInput,
            outcome: KvCacheEventOutcome::Hit,
            cache_id: Some(handle.cache_id),
            backend_key: Some(entry.metadata.backend_hint.clone()),
            reuse_source: kv_reuse_source(&entry.metadata),
            token_count: Some(entry.metadata.token_count),
            reason: Some("restored_input_handle".to_string()),
        },
    );
    Ok(true)
}

#[cfg(feature = "pytorch-nodes")]
pub(super) async fn capture_pytorch_output_handle(
    task_id: &str,
    execution_id: &str,
    extensions: &ExecutorExtensions,
    event_sink: Option<&Arc<dyn EventSink>>,
) -> Result<serde_json::Value> {
    let Some(store) = extensions.get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
    else {
        return Ok(serde_json::Value::Null);
    };

    let active_model = inference::backend::pytorch::active_loaded_model_info()
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!(
                "PyTorch loaded-model lookup failed: {}",
                error
            ))
        })?;
    if !inference::backend::pytorch::supports_live_kv_reuse(&active_model.model_type) {
        emit_kv_cache_detail(
            event_sink,
            task_id,
            execution_id,
            1.0,
            "PyTorch runtime does not support live KV reuse for this model",
            KvCacheExecutionDiagnostics {
                action: KvCacheEventAction::CaptureOutput,
                outcome: KvCacheEventOutcome::Unsupported,
                cache_id: None,
                backend_key: Some("pytorch".to_string()),
                reuse_source: None,
                token_count: None,
                reason: Some("live_reuse_unsupported_for_model_type".to_string()),
            },
        );
        return Ok(serde_json::Value::Null);
    }

    let snapshot_path = kv_slot_temp_path("pytorch-capture", task_id);
    let save_info = inference::backend::pytorch::save_live_kv_snapshot(&snapshot_path)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("PyTorch KV snapshot save failed: {}", error))
        })?;
    let data = fs::read(&snapshot_path).map_err(|error| {
        NodeEngineError::ExecutionFailed(format!(
            "Failed to read temporary PyTorch KV snapshot file '{}': {}",
            snapshot_path.display(),
            error
        ))
    })?;
    let _ = fs::remove_file(&snapshot_path);

    let runtime_fingerprint =
        inference::backend::pytorch::kv_cache_runtime_fingerprint_for_live_kv(&save_info);
    let model_fingerprint =
        inference::backend::pytorch::kv_cache_model_fingerprint_for_live_kv(&save_info);
    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some(format!("{} KV Cache", task_id)),
            model_fingerprint,
            runtime_fingerprint: Some(runtime_fingerprint.clone()),
            backend_hint: runtime_fingerprint.backend_key.clone(),
            token_count: save_info.token_count,
            markers: Vec::new(),
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({
                "source": "pytorch_live_kv_snapshot",
                "modelPath": save_info.model_path,
                "modelType": save_info.model_type,
                "device": save_info.device,
            }),
        },
        data,
    };
    let cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("KV cache save failed: {error}"))
        })?;
    let metadata = store.get_metadata(&cache_id).await.map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Failed to read KV metadata: {error}"))
    })?;

    let handle = metadata.executable_handle().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Saved PyTorch KV metadata did not produce an executable handle".to_string(),
        )
    })?;
    emit_kv_cache_detail(
        event_sink,
        task_id,
        execution_id,
        1.0,
        "KV cache output captured",
        KvCacheExecutionDiagnostics {
            action: KvCacheEventAction::CaptureOutput,
            outcome: KvCacheEventOutcome::Saved,
            cache_id: Some(metadata.cache_id.clone()),
            backend_key: Some(metadata.backend_hint.clone()),
            reuse_source: kv_reuse_source(&metadata),
            token_count: Some(metadata.token_count),
            reason: Some("captured_output_handle".to_string()),
        },
    );
    serde_json::to_value(&handle).map_err(Into::into)
}

pub(super) async fn execute_load(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    gateway: Option<&Arc<InferenceGateway>>,
) -> Result<HashMap<String, serde_json::Value>> {
    let store = require_store(extensions)?;

    let cache_id = inputs
        .get("cache_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::MissingInput("cache_id".to_string()))?;

    let fingerprint_val = inputs
        .get("model_fingerprint")
        .ok_or_else(|| NodeEngineError::MissingInput("model_fingerprint".to_string()))?;
    let fingerprint: ModelFingerprint = serde_json::from_value(fingerprint_val.clone())?;

    let mut outputs = HashMap::new();
    match store.load(cache_id, &fingerprint).await {
        Ok(entry) => {
            let valid = if entry.metadata.executable_handle().is_some() {
                if let Some(gateway) = gateway {
                    match gateway.kv_cache_runtime_fingerprint().await {
                        Ok(runtime_fingerprint) => match store
                            .load_handle(cache_id, &fingerprint, &runtime_fingerprint)
                            .await
                        {
                            Ok(handle) => {
                                outputs.insert(
                                    "cache_data".to_string(),
                                    serde_json::to_value(&handle)?,
                                );
                                true
                            }
                            Err(error) => {
                                log::warn!(
                                    "KV cache '{}' is not reusable in the active runtime: {}",
                                    cache_id,
                                    error
                                );
                                outputs.insert("cache_data".to_string(), serde_json::Value::Null);
                                false
                            }
                        },
                        Err(error) => {
                            log::warn!(
                                "KV cache load runtime fingerprint lookup failed for '{}': {}",
                                cache_id,
                                error
                            );
                            outputs.insert("cache_data".to_string(), serde_json::Value::Null);
                            false
                        }
                    }
                } else {
                    outputs.insert("cache_data".to_string(), serde_json::Value::Null);
                    false
                }
            } else {
                outputs.insert("cache_data".to_string(), serde_json::Value::Null);
                false
            };
            outputs.insert(
                "metadata".to_string(),
                serde_json::to_value(&entry.metadata)?,
            );
            outputs.insert("valid".to_string(), serde_json::json!(valid));
        }
        Err(error) => {
            log::warn!("KV cache load failed for '{}': {}", cache_id, error);
            outputs.insert("cache_data".to_string(), serde_json::Value::Null);
            outputs.insert(
                "metadata".to_string(),
                serde_json::json!({"cache_id": cache_id}),
            );
            outputs.insert("valid".to_string(), serde_json::json!(false));
        }
    }

    Ok(outputs)
}

pub(super) async fn execute_truncate(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    gateway: Option<&Arc<InferenceGateway>>,
    task_id: &str,
    execution_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
) -> Result<HashMap<String, serde_json::Value>> {
    let store = require_store(extensions)?;

    let cache_id = inputs
        .get("cache_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::MissingInput("cache_id".to_string()))?;

    let marker_name = inputs.get("marker_name").and_then(|value| value.as_str());
    let token_position = inputs
        .get("token_position")
        .map(parse_token_position)
        .transpose()?;

    if marker_name.is_some() || token_position.is_some() {
        let gateway = require_gateway(gateway)?;
        let runtime_fingerprint =
            gateway
                .kv_cache_runtime_fingerprint()
                .await
                .map_err(|error| {
                    NodeEngineError::ExecutionFailed(format!(
                        "KV cache runtime fingerprint lookup failed: {}",
                        error
                    ))
                })?;
        let model_fingerprint = gateway
            .kv_cache_model_fingerprint()
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!(
                    "KV cache model fingerprint lookup failed: {}",
                    error
                ))
            })?;
        store
            .load_for_execution(cache_id, &model_fingerprint, &runtime_fingerprint)
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!(
                    "KV cache truncation compatibility check failed: {}",
                    error
                ))
            })?;

        let codec = GatewayKvCacheCodec {
            gateway,
            model_fingerprint,
        };

        if let Some(marker_name) = marker_name {
            store
                .truncate_to_marker(cache_id, marker_name, &codec)
                .await
        } else if let Some(token_position) = token_position {
            store
                .truncate_to_token(cache_id, token_position, &codec)
                .await
        } else {
            Ok(())
        }
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("KV cache truncation failed: {error}"))
        })?;
    }

    let metadata = store.get_metadata(cache_id).await.map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Failed to load metadata: {error}"))
    })?;
    emit_kv_cache_detail(
        event_sink,
        task_id,
        execution_id,
        1.0,
        "KV cache truncated",
        KvCacheExecutionDiagnostics {
            action: KvCacheEventAction::Truncate,
            outcome: KvCacheEventOutcome::Truncated,
            cache_id: Some(metadata.cache_id.clone()),
            backend_key: Some(metadata.backend_hint.clone()),
            reuse_source: kv_reuse_source(&metadata),
            token_count: Some(metadata.token_count),
            reason: Some("truncated_cache".to_string()),
        },
    );

    let mut outputs = HashMap::new();
    outputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    outputs.insert("metadata".to_string(), serde_json::to_value(&metadata)?);
    Ok(outputs)
}

struct GatewayKvCacheCodec<'a> {
    gateway: &'a Arc<InferenceGateway>,
    model_fingerprint: ModelFingerprint,
}

#[async_trait]
impl KvCacheCodec for GatewayKvCacheCodec<'_> {
    async fn truncate(
        &self,
        data: &[u8],
        token_position: usize,
    ) -> std::result::Result<Vec<u8>, inference::kv_cache::KvCacheError> {
        self.gateway
            .truncate_kv_cache_data(data, token_position)
            .await
            .map_err(|error| inference::kv_cache::KvCacheError::Codec {
                message: error.to_string(),
            })
    }

    fn model_fingerprint(
        &self,
    ) -> std::result::Result<ModelFingerprint, inference::kv_cache::KvCacheError> {
        Ok(self.model_fingerprint.clone())
    }

    fn backend_name(&self) -> &'static str {
        "gateway"
    }
}

fn require_store(extensions: &ExecutorExtensions) -> Result<&Arc<KvCacheStore>> {
    extensions
        .get::<Arc<KvCacheStore>>(crate::extension_keys::KV_CACHE_STORE)
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "KvCacheStore not configured in executor extensions".to_string(),
            )
        })
}

fn parse_storage_policy(inputs: &HashMap<String, serde_json::Value>) -> StoragePolicy {
    match inputs
        .get("storage_policy")
        .and_then(|value| value.as_str())
        .unwrap_or("memory")
    {
        "disk" => StoragePolicy::DiskOnly,
        "both" => StoragePolicy::MemoryAndDisk,
        _ => StoragePolicy::MemoryOnly,
    }
}

fn parse_markers(inputs: &HashMap<String, serde_json::Value>) -> Result<Vec<CacheMarker>> {
    match inputs.get("markers") {
        Some(value) => Ok(serde_json::from_value(value.clone())?),
        None => Ok(Vec::new()),
    }
}

fn read_legacy_model_fingerprint(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Option<ModelFingerprint>> {
    match inputs.get("model_fingerprint") {
        Some(value) => Ok(Some(serde_json::from_value(value.clone())?)),
        None => Ok(None),
    }
}

fn parse_token_position(value: &serde_json::Value) -> Result<usize> {
    let Some(position) = value.as_f64() else {
        return Err(NodeEngineError::ExecutionFailed(
            "token_position must be numeric".to_string(),
        ));
    };
    if !position.is_finite() || position < 0.0 || position.fract() != 0.0 {
        return Err(NodeEngineError::ExecutionFailed(
            "token_position must be a non-negative integer".to_string(),
        ));
    }
    Ok(position as usize)
}

fn kv_slot_temp_path(stage: &str, discriminator: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "pantograph-llamacpp-kv-{}-{}-{}.bin",
        stage,
        discriminator,
        Uuid::new_v4()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use futures_util::{stream, Stream};
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{InferenceGateway, RerankRequest, RerankResponse};

    #[test]
    fn parse_storage_policy_defaults_to_memory() {
        let inputs = HashMap::new();
        assert!(matches!(
            parse_storage_policy(&inputs),
            StoragePolicy::MemoryOnly
        ));
    }

    #[test]
    fn parse_storage_policy_supports_disk_and_both() {
        let mut disk_inputs = HashMap::new();
        disk_inputs.insert("storage_policy".to_string(), serde_json::json!("disk"));
        assert!(matches!(
            parse_storage_policy(&disk_inputs),
            StoragePolicy::DiskOnly
        ));

        let mut both_inputs = HashMap::new();
        both_inputs.insert("storage_policy".to_string(), serde_json::json!("both"));
        assert!(matches!(
            parse_storage_policy(&both_inputs),
            StoragePolicy::MemoryAndDisk
        ));
    }

    #[test]
    fn parse_markers_returns_empty_when_missing() {
        let inputs = HashMap::new();
        let markers = parse_markers(&inputs).expect("missing markers should default to empty");
        assert!(markers.is_empty());
    }

    #[test]
    fn parse_markers_parses_marker_payloads() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "markers".to_string(),
            serde_json::json!([{
                "name": "system",
                "tokenPosition": 12,
                "description": "prefix boundary"
            }]),
        );

        let markers = parse_markers(&inputs).expect("marker payload should parse");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].name, "system");
        assert_eq!(markers[0].token_position, 12);
    }

    struct MockKvProcessHandle;

    impl ProcessHandle for MockKvProcessHandle {
        fn pid(&self) -> u32 {
            1
        }

        fn kill(&self) -> std::result::Result<(), String> {
            Ok(())
        }
    }

    struct MockKvProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockKvProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> std::result::Result<
            (
                tokio::sync::mpsc::Receiver<ProcessEvent>,
                Box<dyn ProcessHandle>,
            ),
            String,
        > {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok((rx, Box::new(MockKvProcessHandle)))
        }

        fn app_data_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }

        fn binaries_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }
    }

    struct MockKvBackend {
        bytes: Vec<u8>,
        restored: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    #[async_trait]
    impl InferenceBackend for MockKvBackend {
        fn name(&self) -> &'static str {
            "MockKv"
        }

        fn description(&self) -> &'static str {
            "Mock backend with KV slot support"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> std::result::Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome::default())
        }

        fn stop(&mut self) {}

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            Some("http://127.0.0.1:11434".to_string())
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> std::result::Result<
            Pin<Box<dyn Stream<Item = std::result::Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(
            &self,
            _request: RerankRequest,
        ) -> std::result::Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }

        async fn kv_cache_runtime_fingerprint(
            &self,
            _active_config: Option<&BackendConfig>,
        ) -> std::result::Result<KvCacheRuntimeFingerprint, BackendError> {
            Ok(KvCacheRuntimeFingerprint {
                runtime_id: "mock".to_string(),
                backend_key: "mock".to_string(),
                tokenizer_fingerprint: "tok".to_string(),
                prompt_format_fingerprint: Some("prompt".to_string()),
                runtime_build_fingerprint: Some("build".to_string()),
            })
        }

        async fn kv_cache_model_fingerprint(
            &self,
            _active_config: Option<&BackendConfig>,
        ) -> std::result::Result<ModelFingerprint, BackendError> {
            Ok(ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            })
        }

        async fn save_kv_cache_slot(
            &self,
            _slot_id: u32,
            path: &Path,
        ) -> std::result::Result<(), BackendError> {
            fs::write(path, &self.bytes)
                .map_err(|error| BackendError::Inference(format!("mock save failed: {}", error)))
        }

        async fn restore_kv_cache_slot(
            &self,
            _slot_id: u32,
            path: &Path,
        ) -> std::result::Result<(), BackendError> {
            let bytes = fs::read(path).map_err(|error| {
                BackendError::Inference(format!("mock restore failed: {}", error))
            })?;
            self.restored
                .lock()
                .expect("lock should succeed")
                .push(bytes);
            Ok(())
        }

        async fn truncate_kv_cache_data(
            &self,
            data: &[u8],
            token_position: usize,
            _active_config: Option<&BackendConfig>,
        ) -> std::result::Result<Vec<u8>, BackendError> {
            Ok(data[..token_position.min(data.len())].to_vec())
        }
    }

    #[tokio::test]
    async fn capture_llamacpp_output_handle_saves_slot_into_store() {
        let restored = Arc::new(Mutex::new(Vec::new()));
        let gateway = Arc::new(InferenceGateway::with_backend(
            Box::new(MockKvBackend {
                bytes: vec![1, 2, 3, 4],
                restored: restored.clone(),
            }),
            "mock-kv",
        ));
        gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

        let mut extensions = ExecutorExtensions::new();
        let store = Arc::new(KvCacheStore::memory_only());
        extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

        let handle_value =
            capture_llamacpp_output_handle("task-a", "exec-a", &gateway, &extensions, None)
                .await
                .expect("capture should succeed");
        let handle: KvCacheHandle =
            serde_json::from_value(handle_value).expect("capture should return a typed handle");
        let entry = store
            .load(
                &handle.cache_id,
                &ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
            )
            .await
            .expect("captured handle should resolve through the store");

        assert_eq!(entry.data, vec![1, 2, 3, 4]);
        assert_eq!(
            entry
                .metadata
                .runtime_fingerprint
                .as_ref()
                .map(|fp| fp.runtime_id.as_str()),
            Some("mock")
        );
    }

    #[tokio::test]
    async fn execute_save_clones_handle_backed_entry() {
        let mut extensions = ExecutorExtensions::new();
        let store = Arc::new(KvCacheStore::memory_only());
        let entry = KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: String::new(),
                label: Some("source".to_string()),
                model_fingerprint: ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
                runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                    runtime_id: "mock".to_string(),
                    backend_key: "mock".to_string(),
                    tokenizer_fingerprint: "tok".to_string(),
                    prompt_format_fingerprint: Some("prompt".to_string()),
                    runtime_build_fingerprint: Some("build".to_string()),
                }),
                backend_hint: "mock".to_string(),
                token_count: 0,
                markers: Vec::new(),
                created_at: 0,
                updated_at: 0,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![4, 5, 6],
        };
        let source_cache_id = store
            .save(entry, Some(StoragePolicy::MemoryOnly))
            .await
            .expect("fixture save should succeed");
        let source_handle = store
            .get_metadata(&source_cache_id)
            .await
            .expect("metadata should exist")
            .executable_handle()
            .expect("metadata should produce a handle");
        extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

        let mut inputs = HashMap::new();
        inputs.insert(
            "cache_data".to_string(),
            serde_json::to_value(source_handle).expect("handle should serialize"),
        );
        inputs.insert("label".to_string(), serde_json::json!("saved-copy"));

        let outputs = execute_save(&inputs, &extensions)
            .await
            .expect("save should clone the handle-backed entry");
        let saved_cache_id = outputs
            .get("cache_id")
            .and_then(|value| value.as_str())
            .expect("save should return a cache id");
        assert_ne!(saved_cache_id, source_cache_id);

        let saved_entry = store
            .load(
                saved_cache_id,
                &ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
            )
            .await
            .expect("saved entry should be readable");
        assert_eq!(saved_entry.data, vec![4, 5, 6]);
        assert_eq!(saved_entry.metadata.label.as_deref(), Some("saved-copy"));
    }

    #[tokio::test]
    async fn execute_load_returns_typed_handle_when_runtime_matches() {
        let mut extensions = ExecutorExtensions::new();
        let store = Arc::new(KvCacheStore::memory_only());
        let entry = KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: String::new(),
                label: Some("saved".to_string()),
                model_fingerprint: ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
                runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                    runtime_id: "mock".to_string(),
                    backend_key: "mock".to_string(),
                    tokenizer_fingerprint: "tok".to_string(),
                    prompt_format_fingerprint: Some("prompt".to_string()),
                    runtime_build_fingerprint: Some("build".to_string()),
                }),
                backend_hint: "mock".to_string(),
                token_count: 0,
                markers: Vec::new(),
                created_at: 0,
                updated_at: 0,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![1, 2, 3],
        };
        let cache_id = store
            .save(entry, Some(StoragePolicy::MemoryOnly))
            .await
            .expect("fixture save should succeed");
        extensions.set(crate::extension_keys::KV_CACHE_STORE, store);

        let gateway = Arc::new(InferenceGateway::with_backend(
            Box::new(MockKvBackend {
                bytes: vec![9, 9, 9],
                restored: Arc::new(Mutex::new(Vec::new())),
            }),
            "mock-kv",
        ));
        gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

        let mut inputs = HashMap::new();
        inputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
        inputs.insert(
            "model_fingerprint".to_string(),
            serde_json::json!({
                "modelId": "model",
                "configHash": "cfg",
            }),
        );

        let outputs = execute_load(&inputs, &extensions, Some(&gateway))
            .await
            .expect("load should succeed");
        assert_eq!(outputs.get("valid"), Some(&serde_json::json!(true)));
        let handle: KvCacheHandle = serde_json::from_value(
            outputs
                .get("cache_data")
                .cloned()
                .expect("load should return cache_data"),
        )
        .expect("load should return a typed handle");
        assert_eq!(handle.compatibility.runtime_fingerprint.runtime_id, "mock");
    }

    #[tokio::test]
    async fn restore_llamacpp_input_handle_restores_saved_slot_bytes() {
        let restored = Arc::new(Mutex::new(Vec::new()));
        let gateway = Arc::new(InferenceGateway::with_backend(
            Box::new(MockKvBackend {
                bytes: vec![9, 9, 9],
                restored: restored.clone(),
            }),
            "mock-kv",
        ));
        gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

        let mut extensions = ExecutorExtensions::new();
        let store = Arc::new(KvCacheStore::memory_only());
        let entry = KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: String::new(),
                label: Some("saved".to_string()),
                model_fingerprint: ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
                runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                    runtime_id: "mock".to_string(),
                    backend_key: "mock".to_string(),
                    tokenizer_fingerprint: "tok".to_string(),
                    prompt_format_fingerprint: Some("prompt".to_string()),
                    runtime_build_fingerprint: Some("build".to_string()),
                }),
                backend_hint: "mock".to_string(),
                token_count: 0,
                markers: Vec::new(),
                created_at: 0,
                updated_at: 0,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![7, 8, 9],
        };
        let cache_id = store
            .save(entry, Some(StoragePolicy::MemoryOnly))
            .await
            .expect("fixture save should succeed");
        let metadata = store
            .get_metadata(&cache_id)
            .await
            .expect("metadata should be available");
        let handle = metadata
            .executable_handle()
            .expect("metadata should produce an executable handle");
        extensions.set(crate::extension_keys::KV_CACHE_STORE, store);

        let mut inputs = HashMap::new();
        inputs.insert(
            "kv_cache_in".to_string(),
            serde_json::to_value(handle).expect("handle should serialize"),
        );

        let restored_slot =
            restore_llamacpp_input_handle(&inputs, &gateway, &extensions, "task-a", "exec-a", None)
                .await
                .expect("restore should succeed");
        assert!(restored_slot);
        assert_eq!(
            restored.lock().expect("lock should succeed").as_slice(),
            [vec![7, 8, 9]]
        );
    }

    #[tokio::test]
    async fn execute_truncate_delegates_to_backend_owned_codec() {
        let mut extensions = ExecutorExtensions::new();
        let store = Arc::new(KvCacheStore::memory_only());
        let entry = KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: String::new(),
                label: Some("saved".to_string()),
                model_fingerprint: ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
                runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                    runtime_id: "mock".to_string(),
                    backend_key: "mock".to_string(),
                    tokenizer_fingerprint: "tok".to_string(),
                    prompt_format_fingerprint: Some("prompt".to_string()),
                    runtime_build_fingerprint: Some("build".to_string()),
                }),
                backend_hint: "mock".to_string(),
                token_count: 4,
                markers: vec![CacheMarker {
                    name: "prefix".to_string(),
                    token_position: 2,
                    description: None,
                }],
                created_at: 0,
                updated_at: 0,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![1, 2, 3, 4],
        };
        let cache_id = store
            .save(entry, Some(StoragePolicy::MemoryOnly))
            .await
            .expect("fixture save should succeed");
        extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

        let gateway = Arc::new(InferenceGateway::with_backend(
            Box::new(MockKvBackend {
                bytes: vec![9, 9, 9],
                restored: Arc::new(Mutex::new(Vec::new())),
            }),
            "mock-kv",
        ));
        gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

        let mut inputs = HashMap::new();
        inputs.insert("cache_id".to_string(), serde_json::json!(cache_id.clone()));
        inputs.insert("token_position".to_string(), serde_json::json!(2));

        let outputs = execute_truncate(
            &inputs,
            &extensions,
            Some(&gateway),
            "task-a",
            "exec-a",
            None,
        )
        .await
        .expect("truncate should succeed");
        let metadata: KvCacheMetadata = serde_json::from_value(
            outputs
                .get("metadata")
                .cloned()
                .expect("metadata output should exist"),
        )
        .expect("metadata output should deserialize");
        assert_eq!(metadata.token_count, 2);
        assert_eq!(metadata.markers.len(), 1);

        let truncated = store
            .load(
                &cache_id,
                &ModelFingerprint {
                    model_id: "model".to_string(),
                    config_hash: "cfg".to_string(),
                },
            )
            .await
            .expect("truncated entry should load");
        assert_eq!(truncated.data, vec![1, 2]);
    }
}
