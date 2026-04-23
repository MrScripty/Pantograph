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
    CacheMarker, KvCacheEntry, KvCacheHandle, KvCacheMetadata, KvCacheRuntimeFingerprint,
    ModelFingerprint, StoragePolicy,
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

        self.validate_model_fingerprint(&entry.metadata, fingerprint)?;

        Ok(entry)
    }

    /// Load a cache entry for executable reuse.
    ///
    /// This path enforces the same model/runtime compatibility checks used when
    /// restoring a live runtime slot or returning a typed executable handle.
    pub async fn load_for_execution(
        &self,
        cache_id: &str,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> Result<KvCacheEntry, KvCacheError> {
        let entry = self.load(cache_id, model_fingerprint).await?;
        self.validate_execution_compatibility(
            &entry.metadata,
            model_fingerprint,
            runtime_fingerprint,
        )?;
        Ok(entry)
    }

    /// Load a typed executable handle after enforcing full compatibility.
    pub async fn load_handle(
        &self,
        cache_id: &str,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> Result<KvCacheHandle, KvCacheError> {
        let entry = self
            .load_for_execution(cache_id, model_fingerprint, runtime_fingerprint)
            .await?;
        entry
            .metadata
            .executable_handle()
            .ok_or_else(|| KvCacheError::MissingRuntimeFingerprint {
                cache_id: cache_id.to_string(),
            })
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

    /// Remove the oldest cache entries until the store is bounded by
    /// `max_entries`.
    ///
    /// Entries are evicted in ascending `updated_at`, then `created_at`, then
    /// `cache_id` order so the behavior is stable and easy to reason about in
    /// tests and operational tooling.
    pub async fn prune_to_max_entries(
        &self,
        max_entries: usize,
    ) -> Result<Vec<String>, KvCacheError> {
        let mut metadata = self.list().await?;
        if metadata.len() <= max_entries {
            return Ok(Vec::new());
        }

        metadata.sort_by(|left, right| {
            left.updated_at
                .cmp(&right.updated_at)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.cache_id.cmp(&right.cache_id))
        });

        let remove_count = metadata.len() - max_entries;
        let evicted_ids: Vec<String> = metadata
            .into_iter()
            .take(remove_count)
            .map(|entry| entry.cache_id)
            .collect();

        for cache_id in &evicted_ids {
            self.delete(cache_id).await?;
        }

        Ok(evicted_ids)
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
        let truncated_data = codec.truncate(&data, token_pos).await?;

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

    fn validate_model_fingerprint(
        &self,
        metadata: &KvCacheMetadata,
        fingerprint: &ModelFingerprint,
    ) -> Result<(), KvCacheError> {
        if metadata.matches_model_fingerprint(fingerprint) {
            return Ok(());
        }

        Err(KvCacheError::ModelMismatch {
            cache_model: metadata.model_fingerprint.model_id.clone(),
            requested_model: fingerprint.model_id.clone(),
        })
    }

    fn validate_execution_compatibility(
        &self,
        metadata: &KvCacheMetadata,
        model_fingerprint: &ModelFingerprint,
        runtime_fingerprint: &KvCacheRuntimeFingerprint,
    ) -> Result<(), KvCacheError> {
        self.validate_model_fingerprint(metadata, model_fingerprint)?;

        let Some(cache_runtime_fingerprint) = metadata.runtime_fingerprint.as_ref() else {
            return Err(KvCacheError::MissingRuntimeFingerprint {
                cache_id: metadata.cache_id.clone(),
            });
        };

        if metadata.is_executable_compatible_with(model_fingerprint, runtime_fingerprint) {
            return Ok(());
        }

        Err(KvCacheError::RuntimeMismatch {
            cache_runtime: cache_runtime_fingerprint.runtime_id.clone(),
            requested_runtime: runtime_fingerprint.runtime_id.clone(),
        })
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
