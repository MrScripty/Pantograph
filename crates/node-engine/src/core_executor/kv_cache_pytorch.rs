use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use inference::kv_cache::{
    KvCacheEntry, KvCacheHandle, KvCacheMetadata, KvCacheStore, StoragePolicy,
};

use crate::error::{NodeEngineError, Result};
use crate::events::{
    EventSink, KvCacheEventAction, KvCacheEventOutcome, KvCacheExecutionDiagnostics,
};
use crate::extensions::ExecutorExtensions;

use super::{emit_kv_cache_detail, kv_reuse_source, kv_slot_temp_path, require_store};

pub(crate) async fn restore_pytorch_input_handle(
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

pub(crate) async fn capture_pytorch_output_handle(
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
