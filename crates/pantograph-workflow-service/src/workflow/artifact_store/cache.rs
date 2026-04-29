use std::fs;
use std::path::Path;

use super::manifest::cacheable_body_length;
use super::{ArtifactStore, ArtifactStoreError};

impl ArtifactStore {
    pub(super) fn rebuild_memory_cache(&mut self) {
        self.memory_cache.clear();
        self.memory_cache_bytes = 0;
        let artifact_ids = self
            .manifest
            .artifacts
            .iter()
            .filter(|entry| entry.body_file.is_some())
            .map(|entry| entry.descriptor.artifact_id.clone())
            .collect::<Vec<_>>();
        for artifact_id in artifact_ids {
            let Ok(path) = self.body_path(&artifact_id) else {
                continue;
            };
            let _ = self.cache_body_from_disk_if_allowed(&artifact_id, &path);
        }
    }

    pub(super) fn cache_body_from_disk_if_allowed(
        &mut self,
        artifact_id: &str,
        path: &Path,
    ) -> Result<(), ArtifactStoreError> {
        let Some(byte_length) =
            cacheable_body_length(&self.manifest.policy, path.metadata()?.len())
        else {
            return Ok(());
        };
        if !self.has_memory_cache_capacity(byte_length) {
            return Ok(());
        }
        let body = fs::read(path)?;
        self.cache_body_if_allowed(artifact_id, body);
        Ok(())
    }

    pub(super) fn cache_body_if_allowed(&mut self, artifact_id: &str, body: Vec<u8>) {
        let body_len = body.len() as u64;
        if cacheable_body_length(&self.manifest.policy, body_len).is_none()
            || !self.has_memory_cache_capacity(body_len)
        {
            return;
        }
        self.memory_cache_insert(artifact_id, body);
    }

    pub(super) fn memory_cache_remove(&mut self, artifact_id: &str) {
        if let Some(body) = self.memory_cache.remove(artifact_id) {
            self.memory_cache_bytes = self.memory_cache_bytes.saturating_sub(body.len() as u64);
        }
    }

    fn has_memory_cache_capacity(&self, byte_length: u64) -> bool {
        let Some(max_memory_bytes) = self.manifest.policy.max_memory_bytes else {
            return false;
        };
        self.memory_cache_bytes.saturating_add(byte_length) <= max_memory_bytes
    }

    fn memory_cache_insert(&mut self, artifact_id: &str, body: Vec<u8>) {
        self.memory_cache_remove(artifact_id);
        self.memory_cache_bytes += body.len() as u64;
        self.memory_cache.insert(artifact_id.to_string(), body);
    }
}
