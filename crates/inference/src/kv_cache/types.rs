//! KV cache type definitions

use serde::{Deserialize, Serialize};

/// Fingerprint identifying the runtime and tokenizer semantics that produced a
/// KV artifact.
///
/// KV reuse is valid only when both the model fingerprint and this runtime
/// fingerprint remain compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheRuntimeFingerprint {
    pub runtime_id: String,
    pub backend_key: String,
    pub tokenizer_fingerprint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_format_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_build_fingerprint: Option<String>,
}

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheMarker {
    pub name: String,
    pub token_position: usize,
    pub description: Option<String>,
}

/// Workflow-level intent for how one inference node should interact with KV
/// artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum KvCacheUsageMode {
    #[default]
    Disabled,
    ProduceOnly,
    ConsumeOnly,
    ConsumeAndProduce,
}

/// Read-only compatibility key for one reusable KV artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheCompatibility {
    pub model_fingerprint: ModelFingerprint,
    pub runtime_fingerprint: KvCacheRuntimeFingerprint,
}

impl KvCacheCompatibility {
    pub fn matches(
        &self,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> bool {
        self.model_fingerprint == *model_fingerprint
            && self.runtime_fingerprint == *runtime_fingerprint
    }
}

/// Executable boundary contract passed through workflow graphs and session
/// state when a node wants to consume or retain a reusable KV artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheHandle {
    pub cache_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub compatibility: KvCacheCompatibility,
    pub backend_hint: String,
    pub token_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<CacheMarker>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl KvCacheHandle {
    pub fn is_compatible_with(
        &self,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> bool {
        self.compatibility
            .matches(model_fingerprint, runtime_fingerprint)
    }
}

/// Metadata describing a stored KV cache entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvCacheMetadata {
    pub cache_id: String,
    pub label: Option<String>,
    pub model_fingerprint: ModelFingerprint,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_fingerprint: Option<KvCacheRuntimeFingerprint>,
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

impl KvCacheMetadata {
    pub fn matches_model_fingerprint(&self, model_fingerprint: &ModelFingerprint) -> bool {
        self.model_fingerprint == *model_fingerprint
    }

    pub fn is_executable_compatible_with(
        &self,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> bool {
        self.matches_model_fingerprint(model_fingerprint)
            && self
                .runtime_fingerprint
                .as_ref()
                .is_some_and(|fingerprint| fingerprint == runtime_fingerprint)
    }

    pub fn executable_handle(&self) -> Option<KvCacheHandle> {
        let runtime_fingerprint = self.runtime_fingerprint.clone()?;
        Some(KvCacheHandle {
            cache_id: self.cache_id.clone(),
            label: self.label.clone(),
            compatibility: KvCacheCompatibility {
                model_fingerprint: self.model_fingerprint.clone(),
                runtime_fingerprint,
            },
            backend_hint: self.backend_hint.clone(),
            token_count: self.token_count,
            markers: self.markers.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
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
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "runtime-a".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-123".to_string(),
                prompt_format_fingerprint: Some("chatml-v1".to_string()),
                runtime_build_fingerprint: Some("build-1".to_string()),
            }),
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
            json.contains("runtimeFingerprint"),
            "expected runtimeFingerprint, got: {json}"
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

    #[test]
    fn test_executable_handle_requires_runtime_fingerprint() {
        let metadata = KvCacheMetadata {
            cache_id: "cache-1".to_string(),
            label: None,
            model_fingerprint: ModelFingerprint {
                model_id: "llama-7b".to_string(),
                config_hash: "abc123".to_string(),
            },
            runtime_fingerprint: None,
            backend_hint: "llamacpp".to_string(),
            token_count: 42,
            markers: Vec::new(),
            created_at: 1,
            updated_at: 2,
            compressed: false,
            extra: serde_json::json!({}),
        };

        assert!(
            metadata.executable_handle().is_none(),
            "legacy metadata without runtime fingerprint should not become a reusable handle"
        );
    }

    #[test]
    fn test_executable_handle_preserves_compatibility_contract() {
        let runtime_fingerprint = KvCacheRuntimeFingerprint {
            runtime_id: "runtime-a".to_string(),
            backend_key: "llamacpp".to_string(),
            tokenizer_fingerprint: "tok-123".to_string(),
            prompt_format_fingerprint: Some("chatml-v1".to_string()),
            runtime_build_fingerprint: Some("build-1".to_string()),
        };
        let model_fingerprint = ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "abc123".to_string(),
        };
        let metadata = KvCacheMetadata {
            cache_id: "cache-1".to_string(),
            label: Some("Warm Prefix".to_string()),
            model_fingerprint: model_fingerprint.clone(),
            runtime_fingerprint: Some(runtime_fingerprint.clone()),
            backend_hint: "llamacpp".to_string(),
            token_count: 42,
            markers: vec![CacheMarker {
                name: "system".to_string(),
                token_position: 16,
                description: None,
            }],
            created_at: 1,
            updated_at: 2,
            compressed: false,
            extra: serde_json::json!({}),
        };

        let handle = metadata
            .executable_handle()
            .expect("runtime fingerprint should produce executable handle");
        assert_eq!(handle.cache_id, "cache-1");
        assert!(handle.is_compatible_with(&model_fingerprint, &runtime_fingerprint));
        assert!(
            !handle.is_compatible_with(
                &model_fingerprint,
                &KvCacheRuntimeFingerprint {
                    runtime_id: "runtime-a".to_string(),
                    backend_key: "llamacpp".to_string(),
                    tokenizer_fingerprint: "tok-999".to_string(),
                    prompt_format_fingerprint: Some("chatml-v1".to_string()),
                    runtime_build_fingerprint: Some("build-1".to_string()),
                }
            ),
            "tokenizer or runtime drift must invalidate reuse"
        );
    }

    #[test]
    fn metadata_runtime_compatibility_requires_matching_runtime_and_model() {
        let model_fingerprint = ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "abc123".to_string(),
        };
        let runtime_fingerprint = KvCacheRuntimeFingerprint {
            runtime_id: "runtime-a".to_string(),
            backend_key: "llamacpp".to_string(),
            tokenizer_fingerprint: "tok-123".to_string(),
            prompt_format_fingerprint: Some("chatml-v1".to_string()),
            runtime_build_fingerprint: Some("build-1".to_string()),
        };
        let metadata = KvCacheMetadata {
            cache_id: "test-id".to_string(),
            label: Some("Test Label".to_string()),
            model_fingerprint: model_fingerprint.clone(),
            runtime_fingerprint: Some(runtime_fingerprint.clone()),
            backend_hint: "llamacpp".to_string(),
            token_count: 512,
            markers: vec![],
            created_at: 1700000000,
            updated_at: 1700000001,
            compressed: false,
            extra: serde_json::json!({}),
        };

        assert!(metadata.is_executable_compatible_with(&model_fingerprint, &runtime_fingerprint));
        assert!(!metadata.is_executable_compatible_with(
            &ModelFingerprint {
                model_id: "mistral-7b".to_string(),
                config_hash: "abc123".to_string(),
            },
            &runtime_fingerprint,
        ));
        assert!(!metadata.is_executable_compatible_with(
            &model_fingerprint,
            &KvCacheRuntimeFingerprint {
                runtime_id: "runtime-b".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-123".to_string(),
                prompt_format_fingerprint: Some("chatml-v1".to_string()),
                runtime_build_fingerprint: Some("build-1".to_string()),
            },
        ));
    }
}
