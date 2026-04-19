use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use inference::kv_cache::{
    CacheMarker, KvCacheEntry, KvCacheMetadata, KvCacheStore, ModelFingerprint, StoragePolicy,
};

use crate::error::{NodeEngineError, Result};
use crate::extensions::ExecutorExtensions;

pub(super) async fn execute_save(
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    let store = require_store(extensions)?;

    let cache_data_val = inputs
        .get("cache_data")
        .ok_or_else(|| NodeEngineError::MissingInput("cache_data".to_string()))?;
    let data_bytes: Vec<u8> = serde_json::from_value(cache_data_val.clone())?;

    let fingerprint_val = inputs
        .get("model_fingerprint")
        .ok_or_else(|| NodeEngineError::MissingInput("model_fingerprint".to_string()))?;
    let model_fingerprint: ModelFingerprint = serde_json::from_value(fingerprint_val.clone())?;

    let label = inputs
        .get("label")
        .and_then(|value| value.as_str())
        .map(String::from);
    let compressed = inputs
        .get("compressed")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let backend_hint = inputs
        .get("_data")
        .and_then(|data| data.get("backend_hint"))
        .and_then(|backend| backend.as_str())
        .unwrap_or("unknown")
        .to_string();
    let storage_policy = parse_storage_policy(inputs);
    let cache_dir = inputs
        .get("cache_dir")
        .and_then(|value| value.as_str())
        .map(PathBuf::from);
    let markers = parse_markers(inputs)?;
    let token_count = data_bytes.len();

    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label,
            model_fingerprint,
            runtime_fingerprint: None,
            backend_hint,
            token_count,
            markers,
            created_at: 0,
            updated_at: 0,
            compressed,
            extra: serde_json::json!({}),
        },
        data: data_bytes,
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
            outputs.insert("cache_data".to_string(), serde_json::to_value(&entry.data)?);
            outputs.insert(
                "metadata".to_string(),
                serde_json::to_value(&entry.metadata)?,
            );
            outputs.insert("valid".to_string(), serde_json::json!(true));
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
) -> Result<HashMap<String, serde_json::Value>> {
    let store = require_store(extensions)?;

    let cache_id = inputs
        .get("cache_id")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::MissingInput("cache_id".to_string()))?;

    let marker_name = inputs.get("marker_name").and_then(|value| value.as_str());
    let token_position = inputs
        .get("token_position")
        .and_then(|value| value.as_f64());

    if marker_name.is_some() || token_position.is_some() {
        return Err(NodeEngineError::ExecutionFailed(
            "KV cache truncation requires a backend-specific KvCacheCodec. \
             No codec is currently available. Connect an inference backend first."
                .to_string(),
        ));
    }

    let metadata = store.get_metadata(cache_id).await.map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Failed to load metadata: {error}"))
    })?;

    let mut outputs = HashMap::new();
    outputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    outputs.insert("metadata".to_string(), serde_json::to_value(&metadata)?);
    Ok(outputs)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
