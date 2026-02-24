//! Storage backends for KV cache data
//!
//! Provides in-memory and on-disk storage implementations behind the
//! `StorageBackend` async trait.

use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::error::KvCacheError;
use super::types::{KvCacheEntry, KvCacheMetadata};

/// Async storage backend for KV cache entries.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Save a complete cache entry (metadata + data).
    async fn save(&self, entry: &KvCacheEntry) -> Result<(), KvCacheError>;

    /// Load the raw cache data for a given cache ID.
    async fn load_data(&self, cache_id: &str) -> Result<Vec<u8>, KvCacheError>;

    /// Load only the metadata for a given cache ID.
    async fn load_metadata(&self, cache_id: &str) -> Result<KvCacheMetadata, KvCacheError>;

    /// Save updated metadata without touching the data blob.
    async fn save_metadata(&self, metadata: &KvCacheMetadata) -> Result<(), KvCacheError>;

    /// Delete a cache entry entirely.
    async fn delete(&self, cache_id: &str) -> Result<(), KvCacheError>;

    /// List metadata for all stored cache entries.
    async fn list(&self) -> Result<Vec<KvCacheMetadata>, KvCacheError>;

    /// Check whether a cache entry exists.
    async fn exists(&self, cache_id: &str) -> Result<bool, KvCacheError>;
}

// ---------------------------------------------------------------------------
// In-memory storage
// ---------------------------------------------------------------------------

/// In-memory KV cache storage backed by a `HashMap` behind a `RwLock`.
pub struct MemoryStorage {
    entries: RwLock<HashMap<String, KvCacheEntry>>,
}

impl MemoryStorage {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StorageBackend for MemoryStorage {
    async fn save(&self, entry: &KvCacheEntry) -> Result<(), KvCacheError> {
        let mut map = self.entries.write().await;
        map.insert(entry.metadata.cache_id.clone(), entry.clone());
        Ok(())
    }

    async fn load_data(&self, cache_id: &str) -> Result<Vec<u8>, KvCacheError> {
        let map = self.entries.read().await;
        map.get(cache_id)
            .map(|e| e.data.clone())
            .ok_or_else(|| KvCacheError::NotFound {
                cache_id: cache_id.to_string(),
            })
    }

    async fn load_metadata(&self, cache_id: &str) -> Result<KvCacheMetadata, KvCacheError> {
        let map = self.entries.read().await;
        map.get(cache_id)
            .map(|e| e.metadata.clone())
            .ok_or_else(|| KvCacheError::NotFound {
                cache_id: cache_id.to_string(),
            })
    }

    async fn save_metadata(&self, metadata: &KvCacheMetadata) -> Result<(), KvCacheError> {
        let mut map = self.entries.write().await;
        match map.get_mut(&metadata.cache_id) {
            Some(entry) => {
                entry.metadata = metadata.clone();
                Ok(())
            }
            None => Err(KvCacheError::NotFound {
                cache_id: metadata.cache_id.clone(),
            }),
        }
    }

    async fn delete(&self, cache_id: &str) -> Result<(), KvCacheError> {
        let mut map = self.entries.write().await;
        map.remove(cache_id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<KvCacheMetadata>, KvCacheError> {
        let map = self.entries.read().await;
        Ok(map.values().map(|e| e.metadata.clone()).collect())
    }

    async fn exists(&self, cache_id: &str) -> Result<bool, KvCacheError> {
        let map = self.entries.read().await;
        Ok(map.contains_key(cache_id))
    }
}

// ---------------------------------------------------------------------------
// Disk storage
// ---------------------------------------------------------------------------

/// On-disk KV cache storage.
///
/// Layout:
/// ```text
/// {base_dir}/{cache_id}/metadata.json
/// {base_dir}/{cache_id}/data.bin
/// ```
///
/// All I/O is performed through `tokio::fs` for async compatibility.
///
/// NOTE: The `compressed` flag in metadata is reserved for future zstd
/// support. Currently data is always stored uncompressed.
pub struct DiskStorage {
    base_dir: PathBuf,
}

impl DiskStorage {
    /// Create a new disk storage rooted at `base_dir`.
    ///
    /// The directory will be created on first write if it does not exist.
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    fn cache_dir(&self, cache_id: &str) -> PathBuf {
        self.base_dir.join(cache_id)
    }

    fn metadata_path(&self, cache_id: &str) -> PathBuf {
        self.cache_dir(cache_id).join("metadata.json")
    }

    fn data_path(&self, cache_id: &str) -> PathBuf {
        self.cache_dir(cache_id).join("data.bin")
    }
}

#[async_trait]
impl StorageBackend for DiskStorage {
    async fn save(&self, entry: &KvCacheEntry) -> Result<(), KvCacheError> {
        let dir = self.cache_dir(&entry.metadata.cache_id);
        tokio::fs::create_dir_all(&dir).await?;

        // Write metadata
        let meta_json = serde_json::to_string_pretty(&entry.metadata).map_err(|e| {
            KvCacheError::Codec {
                message: format!("failed to serialize metadata: {e}"),
            }
        })?;
        tokio::fs::write(self.metadata_path(&entry.metadata.cache_id), meta_json).await?;

        // Write data
        // TODO: When compressed == true, apply zstd compression before writing.
        tokio::fs::write(self.data_path(&entry.metadata.cache_id), &entry.data).await?;

        Ok(())
    }

    async fn load_data(&self, cache_id: &str) -> Result<Vec<u8>, KvCacheError> {
        let path = self.data_path(cache_id);
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(KvCacheError::NotFound {
                cache_id: cache_id.to_string(),
            });
        }
        // TODO: When compressed == true, decompress data after reading.
        let data = tokio::fs::read(&path).await?;
        Ok(data)
    }

    async fn load_metadata(&self, cache_id: &str) -> Result<KvCacheMetadata, KvCacheError> {
        let path = self.metadata_path(cache_id);
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(KvCacheError::NotFound {
                cache_id: cache_id.to_string(),
            });
        }
        let bytes = tokio::fs::read(&path).await?;
        let metadata: KvCacheMetadata =
            serde_json::from_slice(&bytes).map_err(|e| KvCacheError::Codec {
                message: format!("failed to deserialize metadata: {e}"),
            })?;
        Ok(metadata)
    }

    async fn save_metadata(&self, metadata: &KvCacheMetadata) -> Result<(), KvCacheError> {
        let dir = self.cache_dir(&metadata.cache_id);
        if !tokio::fs::try_exists(&dir).await.unwrap_or(false) {
            return Err(KvCacheError::NotFound {
                cache_id: metadata.cache_id.clone(),
            });
        }
        let meta_json =
            serde_json::to_string_pretty(metadata).map_err(|e| KvCacheError::Codec {
                message: format!("failed to serialize metadata: {e}"),
            })?;
        tokio::fs::write(self.metadata_path(&metadata.cache_id), meta_json).await?;
        Ok(())
    }

    async fn delete(&self, cache_id: &str) -> Result<(), KvCacheError> {
        let dir = self.cache_dir(cache_id);
        if tokio::fs::try_exists(&dir).await.unwrap_or(false) {
            tokio::fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    async fn list(&self) -> Result<Vec<KvCacheMetadata>, KvCacheError> {
        if !tokio::fs::try_exists(&self.base_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();
        let mut read_dir = tokio::fs::read_dir(&self.base_dir).await?;

        while let Some(dir_entry) = read_dir.next_entry().await? {
            if !dir_entry.file_type().await?.is_dir() {
                continue;
            }
            let cache_id = dir_entry.file_name().to_string_lossy().to_string();
            let meta_path = self.metadata_path(&cache_id);
            if tokio::fs::try_exists(&meta_path).await.unwrap_or(false) {
                match self.load_metadata(&cache_id).await {
                    Ok(meta) => results.push(meta),
                    Err(e) => {
                        log::warn!("skipping cache entry {cache_id}: {e}");
                    }
                }
            }
        }

        Ok(results)
    }

    async fn exists(&self, cache_id: &str) -> Result<bool, KvCacheError> {
        let meta_path = self.metadata_path(cache_id);
        Ok(tokio::fs::try_exists(&meta_path).await.unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kv_cache::types::ModelFingerprint;

    fn make_test_entry(cache_id: &str) -> KvCacheEntry {
        KvCacheEntry {
            metadata: KvCacheMetadata {
                cache_id: cache_id.to_string(),
                label: Some("test".to_string()),
                model_fingerprint: ModelFingerprint {
                    model_id: "test-model".to_string(),
                    config_hash: "hash123".to_string(),
                },
                backend_hint: "test".to_string(),
                token_count: 256,
                markers: vec![],
                created_at: 1700000000,
                updated_at: 1700000000,
                compressed: false,
                extra: serde_json::json!({}),
            },
            data: vec![1, 2, 3, 4, 5],
        }
    }

    #[tokio::test]
    async fn test_memory_save_and_load_roundtrip() {
        let storage = MemoryStorage::new();
        let entry = make_test_entry("cache-1");

        storage.save(&entry).await.expect("save should succeed");

        let loaded_data = storage
            .load_data("cache-1")
            .await
            .expect("load_data should succeed");
        assert_eq!(loaded_data, vec![1, 2, 3, 4, 5]);

        let loaded_meta = storage
            .load_metadata("cache-1")
            .await
            .expect("load_metadata should succeed");
        assert_eq!(loaded_meta.cache_id, "cache-1");
        assert_eq!(loaded_meta.token_count, 256);
    }

    #[tokio::test]
    async fn test_memory_delete_removes_entry() {
        let storage = MemoryStorage::new();
        let entry = make_test_entry("cache-del");

        storage.save(&entry).await.expect("save should succeed");
        assert!(storage.exists("cache-del").await.unwrap());

        storage
            .delete("cache-del")
            .await
            .expect("delete should succeed");
        assert!(!storage.exists("cache-del").await.unwrap());

        let result = storage.load_data("cache-del").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_memory_list_returns_all() {
        let storage = MemoryStorage::new();

        storage
            .save(&make_test_entry("a"))
            .await
            .expect("save a should succeed");
        storage
            .save(&make_test_entry("b"))
            .await
            .expect("save b should succeed");
        storage
            .save(&make_test_entry("c"))
            .await
            .expect("save c should succeed");

        let list = storage.list().await.expect("list should succeed");
        assert_eq!(list.len(), 3);

        let mut ids: Vec<String> = list.into_iter().map(|m| m.cache_id).collect();
        ids.sort();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn test_disk_save_and_load_roundtrip() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let storage = DiskStorage::new(tmp.path().to_path_buf());
        let entry = make_test_entry("disk-1");

        storage.save(&entry).await.expect("save should succeed");

        let loaded_data = storage
            .load_data("disk-1")
            .await
            .expect("load_data should succeed");
        assert_eq!(loaded_data, vec![1, 2, 3, 4, 5]);

        let loaded_meta = storage
            .load_metadata("disk-1")
            .await
            .expect("load_metadata should succeed");
        assert_eq!(loaded_meta.cache_id, "disk-1");
        assert_eq!(loaded_meta.token_count, 256);

        // Verify files exist on disk
        assert!(tmp.path().join("disk-1/metadata.json").exists());
        assert!(tmp.path().join("disk-1/data.bin").exists());
    }

    #[tokio::test]
    async fn test_disk_delete_removes_files() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let storage = DiskStorage::new(tmp.path().to_path_buf());
        let entry = make_test_entry("disk-del");

        storage.save(&entry).await.expect("save should succeed");
        assert!(storage.exists("disk-del").await.unwrap());
        assert!(tmp.path().join("disk-del").exists());

        storage
            .delete("disk-del")
            .await
            .expect("delete should succeed");
        assert!(!storage.exists("disk-del").await.unwrap());
        assert!(!tmp.path().join("disk-del").exists());
    }
}
