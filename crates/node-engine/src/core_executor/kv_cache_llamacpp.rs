use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use inference::kv_cache::{
    KvCacheEntry, KvCacheHandle, KvCacheMetadata, KvCacheStore, StoragePolicy,
};
use inference::InferenceGateway;

use crate::error::{NodeEngineError, Result};
use crate::events::{
    EventSink, KvCacheEventAction, KvCacheEventOutcome, KvCacheExecutionDiagnostics,
};
use crate::extensions::ExecutorExtensions;

use super::{emit_kv_cache_detail, kv_reuse_source, kv_slot_temp_path, require_store};

const LLAMACPP_SLOT_ID: u32 = 0;

pub(crate) async fn restore_llamacpp_input_handle(
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

pub(crate) async fn capture_llamacpp_output_handle(
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
