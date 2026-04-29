use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use super::{
    ArtifactAccessMode, ArtifactAttribution, ArtifactBodyTransport,
    ArtifactConsumeAcknowledgementRequest, ArtifactConsumeAcknowledgementResponse,
    ArtifactDescriptor, ArtifactFormatMetadata, ArtifactLifecycleState, ArtifactPayloadKind,
    ArtifactPolicy, ArtifactReadRequest, ArtifactReadResponse, IoArtifactRetentionState,
};

const MANIFEST_FILE: &str = "manifest.json";
const BODIES_DIR: &str = "bodies";
const READ_HANDLE_SCHEME: &str = "artifact-read://";

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
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub metadata_only_count: usize,
}

#[derive(Debug)]
pub struct ArtifactStore {
    root_dir: PathBuf,
    manifest_path: PathBuf,
    manifest: ArtifactStoreManifest,
}

impl ArtifactStore {
    pub fn open(
        root_dir: impl AsRef<Path>,
        policy: ArtifactPolicy,
    ) -> Result<Self, ArtifactStoreError> {
        let root_dir = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(root_dir.join(BODIES_DIR))?;
        let manifest_path = root_dir.join(MANIFEST_FILE);
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

        Ok(Self {
            root_dir,
            manifest_path,
            manifest,
        })
    }

    pub fn policy(&self) -> &ArtifactPolicy {
        &self.manifest.policy
    }

    pub fn update_policy(&mut self, policy: ArtifactPolicy) -> Result<(), ArtifactStoreError> {
        self.manifest.policy = policy;
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
        let byte_length = request.body.len() as u64;
        if let Some(max_bytes) = self.manifest.policy.max_single_artifact_bytes {
            if byte_length > max_bytes {
                return Err(ArtifactStoreError::ArtifactTooLarge {
                    actual_bytes: byte_length,
                    max_bytes,
                });
            }
        }

        fs::write(self.body_path(&artifact_id)?, &request.body)?;
        let content_hash = format!("blake3:{}", blake3::hash(&request.body).to_hex());
        let descriptor = ArtifactDescriptor {
            artifact_id: artifact_id.clone(),
            payload_kind: request.payload_kind,
            lifecycle_state: ArtifactLifecycleState::Retained,
            retention_state: IoArtifactRetentionState::Retained,
            byte_length: Some(byte_length),
            content_hash: Some(content_hash),
            format: request.format,
            attribution: request.attribution,
            access_modes: vec![ArtifactAccessMode::Read, ArtifactAccessMode::Download],
            read_handle: Some(read_handle(&artifact_id)),
            stream_handle: None,
            retention_reason: None,
        };
        self.remove_existing(&artifact_id);
        self.manifest.artifacts.push(ArtifactStoreEntry {
            descriptor: descriptor.clone(),
            body_file: Some(body_file_name(&artifact_id)),
            created_at_ms: unix_now_ms(),
            consumed_by: BTreeSet::new(),
        });
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
        let body_file =
            entry
                .body_file
                .as_deref()
                .ok_or_else(|| ArtifactStoreError::BodyUnavailable {
                    artifact_id: request.artifact_id.clone(),
                })?;
        let body = fs::read(self.root_dir.join(BODIES_DIR).join(body_file))?;
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
        let mut expired_count = 0;
        for entry in &mut self.manifest.artifacts {
            if entry.body_file.is_some() && entry.created_at_ms <= cutoff_ms {
                delete_body(&root_dir, entry)?;
                expired_count += 1;
            }
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
                metadata_only_count: 0,
            },
            |mut stats, entry| {
                if entry.body_file.is_some() {
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

    fn remove_existing(&mut self, artifact_id: &str) {
        self.manifest
            .artifacts
            .retain(|entry| entry.descriptor.artifact_id != artifact_id);
    }

    fn save(&self) -> Result<(), ArtifactStoreError> {
        save_manifest(&self.manifest_path, &self.manifest)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ArtifactStoreManifest {
    policy: ArtifactPolicy,
    artifacts: Vec<ArtifactStoreEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ArtifactStoreEntry {
    descriptor: ArtifactDescriptor,
    body_file: Option<String>,
    created_at_ms: u64,
    #[serde(default)]
    consumed_by: BTreeSet<String>,
}

fn validate_artifact_id(artifact_id: &str) -> Result<(), ArtifactStoreError> {
    let valid = !artifact_id.is_empty()
        && artifact_id.len() <= 128
        && artifact_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(ArtifactStoreError::InvalidArtifactId)
    }
}

fn body_file_name(artifact_id: &str) -> String {
    format!("{artifact_id}.bin")
}

fn read_handle(artifact_id: &str) -> String {
    format!("{READ_HANDLE_SCHEME}{artifact_id}")
}

fn reconcile_manifest(root_dir: &Path, manifest: &mut ArtifactStoreManifest) {
    for entry in &mut manifest.artifacts {
        let body_exists = entry
            .body_file
            .as_ref()
            .map(|file| root_dir.join(BODIES_DIR).join(file).is_file())
            .unwrap_or(false);
        if !body_exists && entry.body_file.is_some() {
            entry.body_file = None;
            entry.descriptor.lifecycle_state = ArtifactLifecycleState::Failed;
            entry.descriptor.retention_state = IoArtifactRetentionState::MetadataOnly;
            entry.descriptor.access_modes.clear();
            entry.descriptor.read_handle = None;
            entry.descriptor.retention_reason = Some("body_missing_on_recovery".to_string());
        }
    }
}

fn delete_body(root_dir: &Path, entry: &mut ArtifactStoreEntry) -> Result<(), ArtifactStoreError> {
    if let Some(file) = entry.body_file.take() {
        let path = root_dir.join(BODIES_DIR).join(file);
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    entry.descriptor.lifecycle_state = ArtifactLifecycleState::Deleted;
    entry.descriptor.retention_state = IoArtifactRetentionState::Deleted;
    entry.descriptor.access_modes.clear();
    entry.descriptor.read_handle = None;
    entry.descriptor.retention_reason = Some("body_deleted_by_policy".to_string());
    Ok(())
}

fn apply_byte_range(
    body: Vec<u8>,
    start: Option<u64>,
    end_exclusive: Option<u64>,
) -> Result<Vec<u8>, ArtifactStoreError> {
    let len = body.len() as u64;
    let start = start.unwrap_or(0);
    let end = end_exclusive.unwrap_or(len);
    if start > end || end > len {
        return Err(ArtifactStoreError::InvalidByteRange);
    }
    let start = usize::try_from(start).map_err(|_| ArtifactStoreError::InvalidByteRange)?;
    let end = usize::try_from(end).map_err(|_| ArtifactStoreError::InvalidByteRange)?;
    Ok(body[start..end].to_vec())
}

fn save_manifest(path: &Path, manifest: &ArtifactStoreManifest) -> Result<(), ArtifactStoreError> {
    let contents = serde_json::to_string_pretty(manifest)?;
    fs::write(path, contents)?;
    Ok(())
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
