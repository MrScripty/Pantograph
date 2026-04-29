use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

mod cache;
mod manifest;
mod stream;

use self::manifest::{
    apply_byte_range, body_file_name, delete_body, enforce_single_artifact_limit, read_handle,
    reconcile_manifest, save_manifest, unix_now_ms, validate_artifact_id, ArtifactStoreEntry,
    ArtifactStoreManifest, BODIES_DIR,
};
use super::{
    ArtifactAccessMode, ArtifactAttribution, ArtifactBodyTransport,
    ArtifactConsumeAcknowledgementRequest, ArtifactConsumeAcknowledgementResponse,
    ArtifactDescriptor, ArtifactFormatMetadata, ArtifactLifecycleState, ArtifactPayloadKind,
    ArtifactPolicy, ArtifactReadRequest, ArtifactReadResponse, IoArtifactRetentionState,
};

#[derive(Debug, Error)]
pub enum ArtifactStoreError {
    #[error("invalid artifact id")]
    InvalidArtifactId,
    #[error("artifact not found: {artifact_id}")]
    NotFound { artifact_id: String },
    #[error("artifact body is unavailable: {artifact_id}")]
    BodyUnavailable { artifact_id: String },
    #[error("artifact size {actual_bytes} exceeds max_single_artifact_bytes {max_bytes}")]
    ArtifactTooLarge { actual_bytes: u64, max_bytes: u64 },
    #[error("artifact disk usage {actual_bytes} exceeds max_disk_bytes {max_bytes}")]
    DiskLimitExceeded { actual_bytes: u64, max_bytes: u64 },
    #[error("artifact stream is not writable: {artifact_id}")]
    StreamNotWritable { artifact_id: String },
    #[error(
        "invalid artifact stream sequence for {artifact_id}: expected {expected}, actual {actual}"
    )]
    InvalidStreamSequence {
        artifact_id: String,
        expected: u64,
        actual: u64,
    },
    #[error("invalid byte range")]
    InvalidByteRange,
    #[error("artifact store io error: {0}")]
    Io(#[from] io::Error),
    #[error("artifact store manifest error: {0}")]
    Manifest(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactWriteRequest {
    pub artifact_id: Option<String>,
    pub payload_kind: ArtifactPayloadKind,
    pub media_type: String,
    pub format: Option<ArtifactFormatMetadata>,
    pub attribution: ArtifactAttribution,
    pub artifact_role: Option<String>,
    pub parent_artifact_id: Option<String>,
    pub revision_index: Option<u64>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStreamOpenRequest {
    pub artifact_id: Option<String>,
    pub payload_kind: ArtifactPayloadKind,
    pub media_type: String,
    pub format: Option<ArtifactFormatMetadata>,
    pub attribution: ArtifactAttribution,
    pub artifact_role: Option<String>,
    pub parent_artifact_id: Option<String>,
    pub revision_index: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStreamChunkWriteRequest {
    pub artifact_id: String,
    pub sequence: u64,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactStreamFinalizeRequest {
    pub artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactBodyRead {
    pub response: ArtifactReadResponse,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactStoreStats {
    pub artifact_count: usize,
    pub retained_body_count: usize,
    pub retained_body_bytes: u64,
    pub memory_cache_body_count: usize,
    pub memory_cache_body_bytes: u64,
    pub streaming_body_count: usize,
    pub streaming_body_bytes: u64,
    pub metadata_only_count: usize,
}

#[derive(Debug)]
pub struct ArtifactStore {
    root_dir: PathBuf,
    manifest_path: PathBuf,
    manifest: ArtifactStoreManifest,
    memory_cache: BTreeMap<String, Vec<u8>>,
    memory_cache_bytes: u64,
}

impl ArtifactStore {
    pub fn open(
        root_dir: impl AsRef<Path>,
        policy: ArtifactPolicy,
    ) -> Result<Self, ArtifactStoreError> {
        let root_dir = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(root_dir.join(BODIES_DIR))?;
        let manifest_path = root_dir.join(manifest::MANIFEST_FILE);
        let mut manifest = if manifest_path.exists() {
            let contents = fs::read_to_string(&manifest_path)?;
            serde_json::from_str::<ArtifactStoreManifest>(&contents)?
        } else {
            ArtifactStoreManifest {
                policy: policy.clone(),
                artifacts: Vec::new(),
            }
        };
        manifest.policy = policy;
        reconcile_manifest(&root_dir, &mut manifest);
        save_manifest(&manifest_path, &manifest)?;

        let mut store = Self {
            root_dir,
            manifest_path,
            manifest,
            memory_cache: BTreeMap::new(),
            memory_cache_bytes: 0,
        };
        store.rebuild_memory_cache();

        Ok(store)
    }

    pub fn policy(&self) -> &ArtifactPolicy {
        &self.manifest.policy
    }

    pub fn update_policy(&mut self, policy: ArtifactPolicy) -> Result<(), ArtifactStoreError> {
        self.manifest.policy = policy;
        self.rebuild_memory_cache();
        self.save()
    }

    pub fn write_artifact(
        &mut self,
        request: ArtifactWriteRequest,
    ) -> Result<ArtifactDescriptor, ArtifactStoreError> {
        let artifact_id = request
            .artifact_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        validate_artifact_id(&artifact_id)?;
        if let Some(parent_artifact_id) = &request.parent_artifact_id {
            validate_artifact_id(parent_artifact_id)?;
        }
        let byte_length = request.body.len() as u64;
        enforce_single_artifact_limit(&self.manifest.policy, byte_length)?;
        self.enforce_disk_limit_for(&artifact_id, byte_length)?;

        self.remove_existing(&artifact_id)?;
        fs::write(self.body_path(&artifact_id)?, &request.body)?;
        let content_hash = format!("blake3:{}", blake3::hash(&request.body).to_hex());
        let descriptor = ArtifactDescriptor {
            artifact_id: artifact_id.clone(),
            payload_kind: request.payload_kind,
            lifecycle_state: ArtifactLifecycleState::Retained,
            retention_state: IoArtifactRetentionState::Retained,
            artifact_role: request.artifact_role,
            parent_artifact_id: request.parent_artifact_id,
            revision_index: request.revision_index,
            byte_length: Some(byte_length),
            content_hash: Some(content_hash),
            format: request.format,
            attribution: request.attribution,
            access_modes: vec![ArtifactAccessMode::Read, ArtifactAccessMode::Download],
            read_handle: Some(read_handle(&artifact_id)),
            stream_handle: None,
            retention_reason: None,
        };
        self.manifest.artifacts.push(ArtifactStoreEntry::retained(
            descriptor.clone(),
            body_file_name(&artifact_id),
            unix_now_ms(),
        ));
        self.cache_body_if_allowed(&artifact_id, request.body);
        self.save()?;
        Ok(descriptor)
    }

    pub fn descriptor(&self, artifact_id: &str) -> Result<ArtifactDescriptor, ArtifactStoreError> {
        validate_artifact_id(artifact_id)?;
        Ok(self.entry(artifact_id)?.descriptor.clone())
    }

    pub fn read_body(
        &self,
        request: ArtifactReadRequest,
    ) -> Result<ArtifactBodyRead, ArtifactStoreError> {
        validate_artifact_id(&request.artifact_id)?;
        let entry = self.entry(&request.artifact_id)?;
        if entry.descriptor.lifecycle_state != ArtifactLifecycleState::Retained {
            return Err(ArtifactStoreError::BodyUnavailable {
                artifact_id: request.artifact_id,
            });
        }
        let body_file =
            entry
                .body_file
                .as_deref()
                .ok_or_else(|| ArtifactStoreError::BodyUnavailable {
                    artifact_id: request.artifact_id.clone(),
                })?;
        let body = self
            .memory_cache
            .get(&request.artifact_id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| fs::read(self.root_dir.join(BODIES_DIR).join(body_file)))?;
        let body = apply_byte_range(
            body,
            request.byte_range_start,
            request.byte_range_end_exclusive,
        )?;
        let response = ArtifactReadResponse {
            artifact_id: request.artifact_id,
            media_type: entry
                .descriptor
                .format
                .as_ref()
                .map(|format| format.media_type.clone())
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            body_transport: ArtifactBodyTransport::BinaryBody,
            read_handle: entry
                .descriptor
                .read_handle
                .clone()
                .unwrap_or_else(|| read_handle(&entry.descriptor.artifact_id)),
            byte_length: body.len() as u64,
            content_hash: entry.descriptor.content_hash.clone(),
            complete: request.byte_range_start.is_none()
                && request.byte_range_end_exclusive.is_none(),
        };
        Ok(ArtifactBodyRead { response, body })
    }

    pub fn acknowledge_consume(
        &mut self,
        request: ArtifactConsumeAcknowledgementRequest,
    ) -> Result<ArtifactConsumeAcknowledgementResponse, ArtifactStoreError> {
        validate_artifact_id(&request.artifact_id)?;
        let delete_on_consume = self.manifest.policy.delete_on_consume;
        let root_dir = self.root_dir.clone();
        if delete_on_consume {
            self.memory_cache_remove(&request.artifact_id);
        }
        let entry = self.entry_mut(&request.artifact_id)?;
        entry.consumed_by.insert(request.consumer_id);
        if delete_on_consume {
            delete_body(&root_dir, entry)?;
        }
        let retained_after_consume = entry.body_file.is_some();
        self.save()?;
        Ok(ArtifactConsumeAcknowledgementResponse {
            artifact_id: request.artifact_id,
            retained_after_consume,
        })
    }

    pub fn apply_retention_cleanup(&mut self, now_ms: u64) -> Result<u64, ArtifactStoreError> {
        let Some(ttl_seconds) = self.manifest.policy.ttl_seconds else {
            return Ok(0);
        };
        let cutoff_ms = now_ms.saturating_sub(ttl_seconds.saturating_mul(1000));
        let root_dir = self.root_dir.clone();
        let mut evict_cache_ids = Vec::new();
        let mut expired_count = 0;
        for entry in &mut self.manifest.artifacts {
            if entry.body_file.is_some() && entry.created_at_ms <= cutoff_ms {
                evict_cache_ids.push(entry.descriptor.artifact_id.clone());
                delete_body(&root_dir, entry)?;
                expired_count += 1;
            }
        }
        for artifact_id in evict_cache_ids {
            self.memory_cache_remove(&artifact_id);
        }
        self.save()?;
        Ok(expired_count)
    }

    pub fn stats(&self) -> ArtifactStoreStats {
        self.manifest.artifacts.iter().fold(
            ArtifactStoreStats {
                artifact_count: self.manifest.artifacts.len(),
                retained_body_count: 0,
                retained_body_bytes: 0,
                memory_cache_body_count: self.memory_cache.len(),
                memory_cache_body_bytes: self.memory_cache_bytes,
                streaming_body_count: 0,
                streaming_body_bytes: 0,
                metadata_only_count: 0,
            },
            |mut stats, entry| {
                if let Some(stream) = &entry.pending_stream {
                    stats.streaming_body_count += 1;
                    stats.streaming_body_bytes += stream.byte_length;
                } else if entry.body_file.is_some() {
                    stats.retained_body_count += 1;
                    stats.retained_body_bytes += entry.descriptor.byte_length.unwrap_or_default();
                } else {
                    stats.metadata_only_count += 1;
                }
                stats
            },
        )
    }

    fn entry(&self, artifact_id: &str) -> Result<&ArtifactStoreEntry, ArtifactStoreError> {
        self.manifest
            .artifacts
            .iter()
            .find(|entry| entry.descriptor.artifact_id == artifact_id)
            .ok_or_else(|| ArtifactStoreError::NotFound {
                artifact_id: artifact_id.to_string(),
            })
    }

    fn entry_mut(
        &mut self,
        artifact_id: &str,
    ) -> Result<&mut ArtifactStoreEntry, ArtifactStoreError> {
        self.manifest
            .artifacts
            .iter_mut()
            .find(|entry| entry.descriptor.artifact_id == artifact_id)
            .ok_or_else(|| ArtifactStoreError::NotFound {
                artifact_id: artifact_id.to_string(),
            })
    }

    fn body_path(&self, artifact_id: &str) -> Result<PathBuf, ArtifactStoreError> {
        validate_artifact_id(artifact_id)?;
        Ok(self
            .root_dir
            .join(BODIES_DIR)
            .join(body_file_name(artifact_id)))
    }

    fn remove_existing(&mut self, artifact_id: &str) -> Result<(), ArtifactStoreError> {
        self.memory_cache_remove(artifact_id);
        let mut removed_files = Vec::new();
        self.manifest.artifacts.retain(|entry| {
            if entry.descriptor.artifact_id == artifact_id {
                if let Some(file) = &entry.body_file {
                    removed_files.push(file.clone());
                }
                if let Some(stream) = &entry.pending_stream {
                    removed_files.push(stream.body_file.clone());
                }
                false
            } else {
                true
            }
        });
        for file in removed_files {
            let path = self.root_dir.join(BODIES_DIR).join(file);
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn save(&self) -> Result<(), ArtifactStoreError> {
        save_manifest(&self.manifest_path, &self.manifest)
    }

    fn enforce_disk_limit_for(
        &self,
        artifact_id: &str,
        replacement_body_bytes: u64,
    ) -> Result<(), ArtifactStoreError> {
        let Some(max_bytes) = self.manifest.policy.max_disk_bytes else {
            return Ok(());
        };
        let projected_bytes = self
            .manifest
            .artifacts
            .iter()
            .filter(|entry| entry.descriptor.artifact_id != artifact_id)
            .map(|entry| {
                if let Some(stream) = &entry.pending_stream {
                    stream.byte_length
                } else if entry.body_file.is_some() {
                    entry.descriptor.byte_length.unwrap_or_default()
                } else {
                    0
                }
            })
            .sum::<u64>()
            .saturating_add(replacement_body_bytes);
        if projected_bytes > max_bytes {
            return Err(ArtifactStoreError::DiskLimitExceeded {
                actual_bytes: projected_bytes,
                max_bytes,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::{
        ArtifactConversionDependency, ArtifactConversionStatus, ArtifactPayloadKind,
    };

    #[test]
    fn descriptor_keeps_conversion_metadata_after_delete_on_consume_removes_body() {
        let temp = tempfile::tempdir().expect("temp artifact store");
        let mut store =
            ArtifactStore::open(temp.path(), policy_with_delete_on_consume()).expect("open store");
        let descriptor = store
            .write_artifact(ArtifactWriteRequest {
                artifact_id: Some("artifact-converted".to_string()),
                payload_kind: ArtifactPayloadKind::Image,
                media_type: "image/jpeg".to_string(),
                format: Some(converted_format_metadata()),
                attribution: ArtifactAttribution {
                    workflow_run_id: "run-conversion".to_string(),
                    workflow_id: Some("workflow-image".to_string()),
                    workflow_version_id: Some("workflow-image@1".to_string()),
                    node_id: Some("image-output".to_string()),
                    port_id: Some("image".to_string()),
                    model_id: None,
                    runtime_id: Some("embedded".to_string()),
                },
                artifact_role: Some("workflow_output".to_string()),
                parent_artifact_id: None,
                revision_index: None,
                body: b"converted bytes".to_vec(),
            })
            .expect("write artifact");
        assert_eq!(
            descriptor
                .format
                .as_ref()
                .and_then(|format| format.conversion_status.clone()),
            Some(ArtifactConversionStatus::Converted)
        );

        let consume = store
            .acknowledge_consume(ArtifactConsumeAcknowledgementRequest {
                artifact_id: "artifact-converted".to_string(),
                consumer_id: "client-a".to_string(),
            })
            .expect("consume artifact");
        assert!(!consume.retained_after_consume);

        let descriptor = store
            .descriptor("artifact-converted")
            .expect("descriptor remains queryable");
        assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Deleted);
        assert_eq!(
            descriptor.retention_state,
            IoArtifactRetentionState::Deleted
        );
        assert!(descriptor.read_handle.is_none());
        let format = descriptor.format.expect("format metadata is retained");
        assert_eq!(format.conversion_id.as_deref(), Some("conversion_test"));
        assert_eq!(
            format.conversion_status,
            Some(ArtifactConversionStatus::Converted)
        );
        assert_eq!(
            format.conversion_command_id.as_deref(),
            Some("image_oiiotool")
        );
        assert_eq!(format.conversion_dependencies.len(), 1);
        assert_eq!(format.conversion_dependencies[0].dependency_id, "oiiotool");
        assert_eq!(
            format.conversion_dependencies[0].lease_holder,
            "workflow_run:run-conversion/node:image-output/port:image/conversion:conversion_test"
        );
    }

    fn policy_with_delete_on_consume() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "test-policy".to_string(),
            policy_version: 1,
            ttl_seconds: None,
            max_disk_bytes: None,
            max_memory_bytes: Some(1024 * 1024),
            max_single_artifact_bytes: Some(1024 * 1024),
            spill_threshold_bytes: Some(1024),
            delete_on_consume: true,
        }
    }

    fn converted_format_metadata() -> ArtifactFormatMetadata {
        ArtifactFormatMetadata {
            format_id: "jpg".to_string(),
            media_type: "image/jpeg".to_string(),
            codec_id: None,
            quality_percent: Some(75),
            bitrate_kbps: None,
            crf: None,
            bit_depth: Some("8bit".to_string()),
            color_profile_id: Some("srgb".to_string()),
            converter_id: None,
            converter_version: None,
            library_version: None,
            conversion_id: Some("conversion_test".to_string()),
            conversion_status: Some(ArtifactConversionStatus::Converted),
            conversion_command_id: Some("image_oiiotool".to_string()),
            conversion_dependencies: vec![ArtifactConversionDependency {
                dependency_id: "oiiotool".to_string(),
                active_version: "v3.0.3.1".to_string(),
                lease_id: "lease_test".to_string(),
                lease_holder:
                    "workflow_run:run-conversion/node:image-output/port:image/conversion:conversion_test"
                        .to_string(),
            }],
        }
    }
}
