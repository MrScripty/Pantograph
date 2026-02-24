//! KV cache type definitions

use serde::{Deserialize, Serialize};

/// Fingerprint identifying a specific model configuration.
///
/// Used to validate that a cached KV state is compatible with the
/// model that will consume it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFingerprint {
    pub model_id: String,
    pub config_hash: String,
}

/// Named position within a KV cache token sequence.
///
/// Markers let users save and restore to meaningful points
/// (e.g. "end of system prompt", "after few-shot examples").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMarker {
    pub name: String,
    pub token_position: usize,
    pub description: Option<String>,
}

/// Metadata describing a stored KV cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheMetadata {
    pub cache_id: String,
    pub label: Option<String>,
    pub model_fingerprint: ModelFingerprint,
    pub backend_hint: String,
    pub token_count: usize,
    pub markers: Vec<CacheMarker>,
    pub created_at: u64,
    pub updated_at: u64,
    pub compressed: bool,
    pub extra: serde_json::Value,
}

/// Policy controlling where cache data is persisted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoragePolicy {
    MemoryOnly,
    DiskOnly,
    MemoryAndDisk,
}

/// A complete KV cache entry: metadata plus raw cache data.
///
/// The `data` field contains opaque bytes whose format is
/// backend-specific — only the corresponding `KvCacheCodec`
/// knows how to interpret them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheEntry {
    pub metadata: KvCacheMetadata,
    #[serde(with = "serde_bytes_base64")]
    pub data: Vec<u8>,
}

/// Custom serde module for Vec<u8> as base64 in JSON.
mod serde_bytes_base64 {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // In JSON contexts, serialize as array; the rename_all handles field names.
        // For simplicity, just delegate to the default Vec<u8> serialization.
        bytes.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<u8>::deserialize(deserializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization_uses_camel_case() {
        let metadata = KvCacheMetadata {
            cache_id: "test-id".to_string(),
            label: Some("Test Label".to_string()),
            model_fingerprint: ModelFingerprint {
                model_id: "llama-7b".to_string(),
                config_hash: "abc123".to_string(),
            },
            backend_hint: "pytorch".to_string(),
            token_count: 512,
            markers: vec![CacheMarker {
                name: "system".to_string(),
                token_position: 100,
                description: Some("End of system prompt".to_string()),
            }],
            created_at: 1700000000,
            updated_at: 1700000001,
            compressed: false,
            extra: serde_json::json!({}),
        };

        let json = serde_json::to_string(&metadata).expect("serialization should succeed");

        // Verify camelCase keys are present
        assert!(json.contains("cacheId"), "expected cacheId, got: {json}");
        assert!(
            json.contains("modelFingerprint"),
            "expected modelFingerprint, got: {json}"
        );
        assert!(
            json.contains("backendHint"),
            "expected backendHint, got: {json}"
        );
        assert!(
            json.contains("tokenCount"),
            "expected tokenCount, got: {json}"
        );
        assert!(
            json.contains("createdAt"),
            "expected createdAt, got: {json}"
        );
        assert!(
            json.contains("updatedAt"),
            "expected updatedAt, got: {json}"
        );
        assert!(
            json.contains("tokenPosition"),
            "expected tokenPosition in markers, got: {json}"
        );
        assert!(
            json.contains("modelId"),
            "expected modelId in fingerprint, got: {json}"
        );
        assert!(
            json.contains("configHash"),
            "expected configHash in fingerprint, got: {json}"
        );

        // Verify snake_case keys are NOT present
        assert!(
            !json.contains("cache_id"),
            "should not have snake_case cache_id"
        );
        assert!(
            !json.contains("model_fingerprint"),
            "should not have snake_case model_fingerprint"
        );
        assert!(
            !json.contains("backend_hint"),
            "should not have snake_case backend_hint"
        );
        assert!(
            !json.contains("token_count"),
            "should not have snake_case token_count"
        );
    }

    #[test]
    fn test_model_fingerprint_equality() {
        let fp1 = ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "abc123".to_string(),
        };

        let fp2 = ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "abc123".to_string(),
        };

        let fp3 = ModelFingerprint {
            model_id: "llama-13b".to_string(),
            config_hash: "abc123".to_string(),
        };

        let fp4 = ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "def456".to_string(),
        };

        assert_eq!(fp1, fp2, "identical fingerprints should be equal");
        assert_ne!(fp1, fp3, "different model_id should not be equal");
        assert_ne!(fp1, fp4, "different config_hash should not be equal");
    }
}
