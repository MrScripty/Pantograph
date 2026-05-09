use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::ArtifactStoreError;
use crate::workflow::{
    ArtifactDescriptor, ArtifactLifecycleState, ArtifactPolicy, IoArtifactRetentionState,
};

pub(super) const MANIFEST_FILE: &str = "manifest.json";
pub(super) const BODIES_DIR: &str = "bodies";
const READ_HANDLE_SCHEME: &str = "artifact-read://";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ArtifactStoreManifest {
    pub(super) policy: ArtifactPolicy,
    pub(super) artifacts: Vec<ArtifactStoreEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ArtifactStoreEntry {
    pub(super) descriptor: ArtifactDescriptor,
    pub(super) body_file: Option<String>,
    pub(super) created_at_ms: u64,
    #[serde(default)]
    pub(super) consumed_by: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) pending_stream: Option<ArtifactPendingStream>,
}

impl ArtifactStoreEntry {
    pub(super) fn retained(
        descriptor: ArtifactDescriptor,
        body_file: String,
        created_at_ms: u64,
    ) -> Self {
        Self {
            descriptor,
            body_file: Some(body_file),
            created_at_ms,
            consumed_by: BTreeSet::new(),
            pending_stream: None,
        }
    }

    pub(super) fn streaming(
        descriptor: ArtifactDescriptor,
        stream_file: String,
        created_at_ms: u64,
    ) -> Self {
        Self {
            descriptor,
            body_file: None,
            created_at_ms,
            consumed_by: BTreeSet::new(),
            pending_stream: Some(ArtifactPendingStream {
                body_file: stream_file,
                next_sequence: 0,
                byte_length: 0,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct ArtifactPendingStream {
    pub(super) body_file: String,
    pub(super) next_sequence: u64,
    pub(super) byte_length: u64,
}

pub(super) fn validate_artifact_id(artifact_id: &str) -> Result<(), ArtifactStoreError> {
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

pub(super) fn body_file_name(artifact_id: &str) -> String {
    format!("{artifact_id}.bin")
}

pub(super) fn read_handle(artifact_id: &str) -> String {
    format!("{READ_HANDLE_SCHEME}{artifact_id}")
}

pub(super) fn reconcile_manifest(root_dir: &Path, manifest: &mut ArtifactStoreManifest) {
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
        let stream_exists = entry
            .pending_stream
            .as_ref()
            .map(|stream| root_dir.join(BODIES_DIR).join(&stream.body_file).is_file())
            .unwrap_or(false);
        if entry.pending_stream.is_some() && !stream_exists {
            entry.pending_stream = None;
            entry.descriptor.lifecycle_state = ArtifactLifecycleState::Failed;
            entry.descriptor.retention_state = IoArtifactRetentionState::MetadataOnly;
            entry.descriptor.access_modes.clear();
            entry.descriptor.read_handle = None;
            entry.descriptor.stream_handle = None;
            entry.descriptor.retention_reason = Some("stream_body_missing_on_recovery".to_string());
        }
    }
}

pub(super) fn delete_body(
    root_dir: &Path,
    entry: &mut ArtifactStoreEntry,
) -> Result<(), ArtifactStoreError> {
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

pub(super) fn apply_byte_range(
    body: Vec<u8>,
    start: Option<u64>,
    end_exclusive: Option<u64>,
) -> Result<Vec<u8>, ArtifactStoreError> {
    let len = body.len() as u64;
    let start = start.unwrap_or(0);
    let end = end_exclusive.unwrap_or(len);
    if start > end || start > len {
        return Err(ArtifactStoreError::InvalidByteRange);
    }
    let end = end.min(len);
    let start = usize::try_from(start).map_err(|_| ArtifactStoreError::InvalidByteRange)?;
    let end = usize::try_from(end).map_err(|_| ArtifactStoreError::InvalidByteRange)?;
    Ok(body[start..end].to_vec())
}

pub(super) fn enforce_single_artifact_limit(
    policy: &ArtifactPolicy,
    byte_length: u64,
) -> Result<(), ArtifactStoreError> {
    if let Some(max_bytes) = policy.max_single_artifact_bytes {
        if byte_length > max_bytes {
            return Err(ArtifactStoreError::ArtifactTooLarge {
                actual_bytes: byte_length,
                max_bytes,
            });
        }
    }
    Ok(())
}

pub(super) fn cacheable_body_length(policy: &ArtifactPolicy, byte_length: u64) -> Option<u64> {
    let spill_threshold_bytes = policy.spill_threshold_bytes?;
    policy.max_memory_bytes?;
    (byte_length <= spill_threshold_bytes).then_some(byte_length)
}

pub(super) fn save_manifest(
    path: &Path,
    manifest: &ArtifactStoreManifest,
) -> Result<(), ArtifactStoreError> {
    let contents = serde_json::to_string_pretty(manifest)?;
    fs::write(path, contents)?;
    Ok(())
}

pub(super) fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
