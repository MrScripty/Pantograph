use std::collections::HashMap;
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

#[path = "kv_cache_llamacpp.rs"]
mod llamacpp;
pub(super) use llamacpp::{capture_llamacpp_output_handle, restore_llamacpp_input_handle};

#[cfg(feature = "pytorch-nodes")]
#[path = "kv_cache_pytorch.rs"]
mod pytorch;
#[cfg(feature = "pytorch-nodes")]
pub(super) use pytorch::{capture_pytorch_output_handle, restore_pytorch_input_handle};

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
#[path = "kv_cache_parsing_tests.rs"]
mod parsing_tests;

#[cfg(test)]
#[path = "kv_cache_tests.rs"]
mod tests;
