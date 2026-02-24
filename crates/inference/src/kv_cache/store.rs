//! High-level KV cache store
//!
//! Orchestrates memory and disk storage layers, provides model validation,
//! marker management, and truncation operations.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::codec::KvCacheCodec;
use super::error::KvCacheError;
use super::storage::{DiskStorage, MemoryStorage, StorageBackend};
use super::types::{
    CacheMarker, KvCacheEntry, KvCacheMetadata, ModelFingerprint, StoragePolicy,
};

/// High-level KV cache manager combining memory and disk storage.
///
/// Routes operations to the appropriate storage layer(s) based on
/// `StoragePolicy` and handles cross-cutting concerns like model
/// fingerprint validation, markers, and codec-based truncation.
pub struct KvCacheStore {
    memory: MemoryStorage,
    disk: Option<DiskStorage>,
    default_policy: StoragePolicy,
}

impl KvCacheStore {
    /// Create a new store with both memory and disk layers.
    ///
    /// The `base_dir` is the root directory for on-disk cache storage.
    pub fn new(base_dir: PathBuf, default_policy: StoragePolicy) -> Self {
        Self {
            memory: MemoryStorage::new(),
            disk: Some(DiskStorage::new(base_dir)),
            default_policy,
        }
    }

    /// Create a memory-only store (no disk persistence).
    pub fn memory_only() -> Self {
        Self {
            memory: MemoryStorage::new(),
            disk: None,
            default_policy: StoragePolicy::MemoryOnly,
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Save a cache entry, assigning it a new UUID and timestamps.
    ///
    /// Returns the generated `cache_id`.
    pub async fn save(
        &self,
        mut entry: KvCacheEntry,
        policy: Option<StoragePolicy>,
    ) -> Result<String, KvCacheError> {
        let cache_id = uuid::Uuid::new_v4().to_string();
        let now = Self::now_secs();

        entry.metadata.cache_id = cache_id.clone();
        entry.metadata.created_at = now;
        entry.metadata.updated_at = now;

        let policy = policy.unwrap_or(self.default_policy);
        self.save_to_policy(&entry, policy).await?;

        Ok(cache_id)
    }

    /// Save a cache entry to a specific directory, using a temporary
    /// `DiskStorage` for the override path.
    ///
    /// The entry is also saved to memory if the policy includes it.
    pub async fn save_to(
        &self,
        mut entry: KvCacheEntry,
        cache_dir: PathBuf,
        policy: Option<StoragePolicy>,
    ) -> Result<String, KvCacheError> {
        let cache_id = uuid::Uuid::new_v4().to_string();
        let now = Self::now_secs();

        entry.metadata.cache_id = cache_id.clone();
        entry.metadata.created_at = now;
        entry.metadata.updated_at = now;

        let policy = policy.unwrap_or(self.default_policy);

        match policy {
            StoragePolicy::MemoryOnly => {
                self.memory.save(&entry).await?;
            }
            StoragePolicy::DiskOnly => {
                let disk = DiskStorage::new(cache_dir);
                disk.save(&entry).await?;
            }
            StoragePolicy::MemoryAndDisk => {
                self.memory.save(&entry).await?;
                let disk = DiskStorage::new(cache_dir);
                disk.save(&entry).await?;
            }
        }

        Ok(cache_id)
    }

    /// Route a save to the appropriate storage layer(s) based on policy.
    async fn save_to_policy(
        &self,
        entry: &KvCacheEntry,
        policy: StoragePolicy,
    ) -> Result<(), KvCacheError> {
        match policy {
            StoragePolicy::MemoryOnly => {
                self.memory.save(entry).await?;
            }
            StoragePolicy::DiskOnly => {
                if let Some(ref disk) = self.disk {
                    disk.save(entry).await?;
                } else {
                    return Err(KvCacheError::InvalidData {
                        message: "disk storage not configured but DiskOnly policy requested"
                            .to_string(),
                    });
                }
            }
            StoragePolicy::MemoryAndDisk => {
                self.memory.save(entry).await?;
                if let Some(ref disk) = self.disk {
                    disk.save(entry).await?;
                } else {
                    return Err(KvCacheError::InvalidData {
                        message: "disk storage not configured but MemoryAndDisk policy requested"
                            .to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Load a cache entry, validating the model fingerprint.
    ///
    /// Tries memory first, then disk. Returns `ModelMismatch` if the
    /// cached fingerprint does not match `fingerprint`.
    pub async fn load(
        &self,
        cache_id: &str,
        fingerprint: &ModelFingerprint,
    ) -> Result<KvCacheEntry, KvCacheError> {
        // Try memory first
        let entry = match self.memory.load_data(cache_id).await {
            Ok(data) => {
                let metadata = self.memory.load_metadata(cache_id).await?;
                KvCacheEntry { metadata, data }
            }
            Err(KvCacheError::NotFound { .. }) => {
                // Fall through to disk
                if let Some(ref disk) = self.disk {
                    let metadata = disk.load_metadata(cache_id).await?;
                    let data = disk.load_data(cache_id).await?;
                    KvCacheEntry { metadata, data }
                } else {
                    return Err(KvCacheError::NotFound {
                        cache_id: cache_id.to_string(),
                    });
                }
            }
            Err(e) => return Err(e),
        };

        // Validate model fingerprint
        if entry.metadata.model_fingerprint != *fingerprint {
            return Err(KvCacheError::ModelMismatch {
                cache_model: entry.metadata.model_fingerprint.model_id.clone(),
                requested_model: fingerprint.model_id.clone(),
            });
        }

        Ok(entry)
    }

    /// Delete a cache entry from all storage layers.
    pub async fn delete(&self, cache_id: &str) -> Result<(), KvCacheError> {
        self.memory.delete(cache_id).await?;
        if let Some(ref disk) = self.disk {
            disk.delete(cache_id).await?;
        }
        Ok(())
    }

    /// List metadata for all cache entries across both storage layers.
    ///
    /// Deduplicates by `cache_id` (memory takes precedence).
    pub async fn list(&self) -> Result<Vec<KvCacheMetadata>, KvCacheError> {
        let mut seen = std::collections::HashMap::new();

        // Memory entries first (take precedence)
        for meta in self.memory.list().await? {
            seen.insert(meta.cache_id.clone(), meta);
        }

        // Disk entries (only add if not already in memory)
        if let Some(ref disk) = self.disk {
            for meta in disk.list().await? {
                seen.entry(meta.cache_id.clone()).or_insert(meta);
            }
        }

        Ok(seen.into_values().collect())
    }

    /// Get metadata for a specific cache entry (no data loaded).
    pub async fn get_metadata(&self, cache_id: &str) -> Result<KvCacheMetadata, KvCacheError> {
        match self.memory.load_metadata(cache_id).await {
            Ok(meta) => Ok(meta),
            Err(KvCacheError::NotFound { .. }) => {
                if let Some(ref disk) = self.disk {
                    disk.load_metadata(cache_id).await
                } else {
                    Err(KvCacheError::NotFound {
                        cache_id: cache_id.to_string(),
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Add a marker to an existing cache entry.
    pub async fn add_marker(
        &self,
        cache_id: &str,
        marker: CacheMarker,
    ) -> Result<(), KvCacheError> {
        let mut metadata = self.get_metadata(cache_id).await?;
        metadata.markers.push(marker);
        metadata.updated_at = Self::now_secs();
        self.update_metadata_everywhere(&metadata).await
    }

    /// Remove a marker by name from a cache entry.
    pub async fn remove_marker(
        &self,
        cache_id: &str,
        marker_name: &str,
    ) -> Result<(), KvCacheError> {
        let mut metadata = self.get_metadata(cache_id).await?;
        let original_len = metadata.markers.len();
        metadata.markers.retain(|m| m.name != marker_name);

        if metadata.markers.len() == original_len {
            return Err(KvCacheError::MarkerNotFound {
                marker_name: marker_name.to_string(),
            });
        }

        metadata.updated_at = Self::now_secs();
        self.update_metadata_everywhere(&metadata).await
    }

    /// Truncate cache data to a named marker position.
    ///
    /// Finds the marker, uses the codec to truncate the data, removes
    /// any markers beyond the truncation point, and updates metadata.
    pub async fn truncate_to_marker(
        &self,
        cache_id: &str,
        marker_name: &str,
        codec: &dyn KvCacheCodec,
    ) -> Result<(), KvCacheError> {
        let metadata = self.get_metadata(cache_id).await?;

        let marker = metadata
            .markers
            .iter()
            .find(|m| m.name == marker_name)
            .ok_or_else(|| KvCacheError::MarkerNotFound {
                marker_name: marker_name.to_string(),
            })?;

        let token_position = marker.token_position;
        self.truncate_to_token(cache_id, token_position, codec)
            .await
    }

    /// Truncate cache data to a specific token position.
    ///
    /// Uses the codec to truncate the raw data, removes markers beyond
    /// the position, and updates metadata across all storage layers.
    pub async fn truncate_to_token(
        &self,
        cache_id: &str,
        token_pos: usize,
        codec: &dyn KvCacheCodec,
    ) -> Result<(), KvCacheError> {
        // Load the full entry
        let data = self.load_data_from_any(cache_id).await?;
        let mut metadata = self.get_metadata(cache_id).await?;

        // Truncate via codec
        let truncated_data = codec.truncate(&data, token_pos)?;

        // Remove markers beyond the truncation point
        metadata.markers.retain(|m| m.token_position <= token_pos);
        metadata.token_count = token_pos;
        metadata.updated_at = Self::now_secs();

        // Save the modified entry back
        let entry = KvCacheEntry {
            metadata,
            data: truncated_data,
        };

        // Update in whichever layers have it
        if self.memory.exists(cache_id).await? {
            self.memory.save(&entry).await?;
        }
        if let Some(ref disk) = self.disk {
            if disk.exists(cache_id).await? {
                disk.save(&entry).await?;
            }
        }

        Ok(())
    }

    /// Update the label on a cache entry.
    pub async fn update_label(
        &self,
        cache_id: &str,
        label: Option<String>,
    ) -> Result<(), KvCacheError> {
        let mut metadata = self.get_metadata(cache_id).await?;
        metadata.label = label;
        metadata.updated_at = Self::now_secs();
        self.update_metadata_everywhere(&metadata).await
    }

    /// Load raw data from whichever storage layer has it.
    async fn load_data_from_any(&self, cache_id: &str) -> Result<Vec<u8>, KvCacheError> {
        match self.memory.load_data(cache_id).await {
            Ok(data) => Ok(data),
            Err(KvCacheError::NotFound { .. }) => {
                if let Some(ref disk) = self.disk {
                    disk.load_data(cache_id).await
                } else {
                    Err(KvCacheError::NotFound {
                        cache_id: cache_id.to_string(),
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Update metadata in all storage layers that contain the entry.
    async fn update_metadata_everywhere(
        &self,
        metadata: &KvCacheMetadata,
    ) -> Result<(), KvCacheError> {
        if self.memory.exists(&metadata.cache_id).await? {
            self.memory.save_metadata(metadata).await?;
        }
        if let Some(ref disk) = self.disk {
            if disk.exists(&metadata.cache_id).await? {
                disk.save_metadata(metadata).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(model_id: &str, config_hash: &str) -> KvCacheEntry {
        KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: String::new(), // Will be assigned by save()
                label: None,
                model_fingerprint: ModelFingerprint {
                    model_id: model_id.to_string(),
                    config_hash: config_hash.to_string(),
                },
                backend_hint: "test".to_string(),
                token_count: 100,
                markers: vec![],
                created_at: 0,
                updated_at: 0,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
        }
    }

    fn matching_fingerprint() -> ModelFingerprint {
        ModelFingerprint {
            model_id: "llama-7b".to_string(),
            config_hash: "abc".to_string(),
        }
    }

    fn different_fingerprint() -> ModelFingerprint {
        ModelFingerprint {
            model_id: "mistral-7b".to_string(),
            config_hash: "xyz".to_string(),
        }
    }

    /// A mock codec that truncates a byte slice to the first `token_position` bytes.
    struct MockCodec;

    impl KvCacheCodec for MockCodec {
        fn truncate(&self, data: &[u8], token_position: usize) -> Result<Vec<u8>, KvCacheError> {
            let end = token_position.min(data.len());
            Ok(data[..end].to_vec())
        }

        fn model_fingerprint(&self) -> Result<ModelFingerprint, KvCacheError> {
            Ok(ModelFingerprint {
                model_id: "mock".to_string(),
                config_hash: "mock".to_string(),
            })
        }

        fn backend_name(&self) -> &'static str {
            "mock"
        }
    }

    #[tokio::test]
    async fn test_load_validates_model_fingerprint() {
        let store = KvCacheStore::memory_only();
        let entry = make_entry("llama-7b", "abc");

        let cache_id = store
            .save(entry, None)
            .await
            .expect("save should succeed");

        let loaded = store.load(&cache_id, &matching_fingerprint()).await;
        assert!(loaded.is_ok(), "load with matching fingerprint should succeed");

        let loaded_entry = loaded.unwrap();
        assert_eq!(loaded_entry.data, vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
    }

    #[tokio::test]
    async fn test_load_model_mismatch_returns_error() {
        let store = KvCacheStore::memory_only();
        let entry = make_entry("llama-7b", "abc");

        let cache_id = store
            .save(entry, None)
            .await
            .expect("save should succeed");

        let result = store.load(&cache_id, &different_fingerprint()).await;
        assert!(result.is_err());

        match result.unwrap_err() {
            KvCacheError::ModelMismatch {
                cache_model,
                requested_model,
            } => {
                assert_eq!(cache_model, "llama-7b");
                assert_eq!(requested_model, "mistral-7b");
            }
            other => panic!("expected ModelMismatch, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_add_and_remove_marker() {
        let store = KvCacheStore::memory_only();
        let entry = make_entry("llama-7b", "abc");

        let cache_id = store
            .save(entry, None)
            .await
            .expect("save should succeed");

        // Add markers
        store
            .add_marker(
                &cache_id,
                CacheMarker {
                    name: "system".to_string(),
                    token_position: 50,
                    description: Some("End of system prompt".to_string()),
                },
            )
            .await
            .expect("add_marker should succeed");

        store
            .add_marker(
                &cache_id,
                CacheMarker {
                    name: "examples".to_string(),
                    token_position: 80,
                    description: None,
                },
            )
            .await
            .expect("add_marker should succeed");

        let meta = store
            .get_metadata(&cache_id)
            .await
            .expect("get_metadata should succeed");
        assert_eq!(meta.markers.len(), 2);

        // Remove one marker
        store
            .remove_marker(&cache_id, "system")
            .await
            .expect("remove_marker should succeed");

        let meta = store
            .get_metadata(&cache_id)
            .await
            .expect("get_metadata should succeed");
        assert_eq!(meta.markers.len(), 1);
        assert_eq!(meta.markers[0].name, "examples");

        // Removing non-existent marker should error
        let result = store.remove_marker(&cache_id, "nonexistent").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            KvCacheError::MarkerNotFound { marker_name } => {
                assert_eq!(marker_name, "nonexistent");
            }
            other => panic!("expected MarkerNotFound, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_truncate_to_marker_with_mock_codec() {
        let store = KvCacheStore::memory_only();
        let entry = make_entry("llama-7b", "abc");

        let cache_id = store
            .save(entry, None)
            .await
            .expect("save should succeed");

        // Add markers at different positions
        store
            .add_marker(
                &cache_id,
                CacheMarker {
                    name: "system".to_string(),
                    token_position: 5,
                    description: None,
                },
            )
            .await
            .unwrap();

        store
            .add_marker(
                &cache_id,
                CacheMarker {
                    name: "examples".to_string(),
                    token_position: 8,
                    description: None,
                },
            )
            .await
            .unwrap();

        let codec = MockCodec;

        // Truncate to the "system" marker at position 5
        store
            .truncate_to_marker(&cache_id, "system", &codec)
            .await
            .expect("truncate should succeed");

        // Verify data is truncated
        let loaded = store
            .load(&cache_id, &matching_fingerprint())
            .await
            .expect("load should succeed after truncation");
        assert_eq!(loaded.data, vec![10, 20, 30, 40, 50]);
        assert_eq!(loaded.metadata.token_count, 5);

        // Markers beyond position 5 should be removed
        assert_eq!(loaded.metadata.markers.len(), 1);
        assert_eq!(loaded.metadata.markers[0].name, "system");
    }

    #[tokio::test]
    async fn test_update_label() {
        let store = KvCacheStore::memory_only();
        let entry = make_entry("llama-7b", "abc");

        let cache_id = store
            .save(entry, None)
            .await
            .expect("save should succeed");

        // Initially no label (from make_entry)
        let meta = store.get_metadata(&cache_id).await.unwrap();
        assert!(meta.label.is_none());

        // Set a label
        store
            .update_label(&cache_id, Some("My Cache".to_string()))
            .await
            .expect("update_label should succeed");

        let meta = store.get_metadata(&cache_id).await.unwrap();
        assert_eq!(meta.label, Some("My Cache".to_string()));

        // Clear the label
        store
            .update_label(&cache_id, None)
            .await
            .expect("update_label should succeed");

        let meta = store.get_metadata(&cache_id).await.unwrap();
        assert!(meta.label.is_none());
    }
}
