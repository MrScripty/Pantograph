use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use uuid::Uuid;

use super::manifest::{
    body_file_name, enforce_single_artifact_limit, read_handle, unix_now_ms, validate_artifact_id,
    ArtifactStoreEntry, BODIES_DIR,
};
use super::{
    ArtifactStore, ArtifactStoreError, ArtifactStreamChunkWriteRequest,
    ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest,
};
use crate::workflow::{
    ArtifactAccessMode, ArtifactDescriptor, ArtifactLifecycleState, ArtifactStreamChunkRecord,
    IoArtifactRetentionState,
};

const STREAM_HANDLE_SCHEME: &str = "artifact-stream://";

impl ArtifactStore {
    pub fn open_stream(
        &mut self,
        request: ArtifactStreamOpenRequest,
    ) -> Result<ArtifactDescriptor, ArtifactStoreError> {
        let artifact_id = request
            .artifact_id
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        validate_artifact_id(&artifact_id)?;
        self.remove_existing(&artifact_id)?;

        let stream_file = stream_file_name(&artifact_id);
        fs::File::create(self.root_dir.join(BODIES_DIR).join(&stream_file))?;
        let descriptor = ArtifactDescriptor {
            artifact_id: artifact_id.clone(),
            payload_kind: request.payload_kind,
            lifecycle_state: ArtifactLifecycleState::Streaming,
            retention_state: IoArtifactRetentionState::Retained,
            byte_length: Some(0),
            content_hash: None,
            format: request.format,
            attribution: request.attribution,
            access_modes: vec![ArtifactAccessMode::Stream],
            read_handle: None,
            stream_handle: Some(stream_handle(&artifact_id)),
            retention_reason: None,
        };
        self.manifest.artifacts.push(ArtifactStoreEntry::streaming(
            descriptor.clone(),
            stream_file,
            unix_now_ms(),
        ));
        self.save()?;
        Ok(descriptor)
    }

    pub fn append_stream_chunk(
        &mut self,
        request: ArtifactStreamChunkWriteRequest,
    ) -> Result<ArtifactStreamChunkRecord, ArtifactStoreError> {
        validate_artifact_id(&request.artifact_id)?;
        let root_dir = self.root_dir.clone();
        let (stream_file, expected_sequence, current_length) = {
            let entry = self.entry(&request.artifact_id)?;
            let stream = entry.pending_stream.as_ref().ok_or_else(|| {
                ArtifactStoreError::StreamNotWritable {
                    artifact_id: request.artifact_id.clone(),
                }
            })?;
            (
                stream.body_file.clone(),
                stream.next_sequence,
                stream.byte_length,
            )
        };
        if request.sequence != expected_sequence {
            return Err(ArtifactStoreError::InvalidStreamSequence {
                artifact_id: request.artifact_id,
                expected: expected_sequence,
                actual: request.sequence,
            });
        }
        let chunk_len = request.body.len() as u64;
        let total_len = current_length.saturating_add(chunk_len);
        enforce_single_artifact_limit(&self.manifest.policy, total_len)?;
        self.enforce_disk_limit_for(&request.artifact_id, total_len)?;

        let stream_path = root_dir.join(BODIES_DIR).join(stream_file);
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(false)
            .open(stream_path)?;
        file.write_all(&request.body)?;

        let entry = self.entry_mut(&request.artifact_id)?;
        let stream =
            entry
                .pending_stream
                .as_mut()
                .ok_or_else(|| ArtifactStoreError::StreamNotWritable {
                    artifact_id: request.artifact_id.clone(),
                })?;
        stream.byte_length = total_len;
        stream.next_sequence += 1;
        entry.descriptor.byte_length = Some(total_len);
        entry.descriptor.lifecycle_state = ArtifactLifecycleState::Streaming;
        let content_hash = format!("blake3:{}", blake3::hash(&request.body).to_hex());
        let record = ArtifactStreamChunkRecord {
            artifact_id: entry.descriptor.artifact_id.clone(),
            stream_handle: entry
                .descriptor
                .stream_handle
                .clone()
                .unwrap_or_else(|| stream_handle(&entry.descriptor.artifact_id)),
            sequence: request.sequence,
            byte_length: chunk_len,
            lifecycle_state: ArtifactLifecycleState::Streaming,
            content_hash: Some(content_hash),
        };
        self.save()?;
        Ok(record)
    }

    pub fn finalize_stream(
        &mut self,
        request: ArtifactStreamFinalizeRequest,
    ) -> Result<ArtifactDescriptor, ArtifactStoreError> {
        validate_artifact_id(&request.artifact_id)?;
        let root_dir = self.root_dir.clone();
        let final_file = body_file_name(&request.artifact_id);
        let final_path = root_dir.join(BODIES_DIR).join(&final_file);
        let stream_file = {
            let entry = self.entry_mut(&request.artifact_id)?;
            let stream = entry.pending_stream.as_ref().ok_or_else(|| {
                ArtifactStoreError::StreamNotWritable {
                    artifact_id: request.artifact_id.clone(),
                }
            })?;
            entry.descriptor.lifecycle_state = ArtifactLifecycleState::Finalizing;
            stream.body_file.clone()
        };

        let stream_path = root_dir.join(BODIES_DIR).join(&stream_file);
        let byte_length = fs::metadata(&stream_path)?.len();
        enforce_single_artifact_limit(&self.manifest.policy, byte_length)?;
        self.enforce_disk_limit_for(&request.artifact_id, byte_length)?;
        match fs::remove_file(&final_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        fs::rename(&stream_path, &final_path)?;
        let content_hash = hash_file(&final_path)?;

        let descriptor = {
            let entry = self.entry_mut(&request.artifact_id)?;
            entry.pending_stream = None;
            entry.body_file = Some(final_file);
            entry.descriptor.lifecycle_state = ArtifactLifecycleState::Retained;
            entry.descriptor.retention_state = IoArtifactRetentionState::Retained;
            entry.descriptor.byte_length = Some(byte_length);
            entry.descriptor.content_hash = Some(content_hash);
            entry.descriptor.access_modes =
                vec![ArtifactAccessMode::Read, ArtifactAccessMode::Download];
            entry.descriptor.read_handle = Some(read_handle(&request.artifact_id));
            entry.descriptor.stream_handle = None;
            entry.descriptor.retention_reason = None;
            entry.descriptor.clone()
        };
        self.cache_body_from_disk_if_allowed(&request.artifact_id, &final_path)?;
        self.save()?;
        Ok(descriptor)
    }
}

fn stream_file_name(artifact_id: &str) -> String {
    format!("{artifact_id}.stream")
}

fn stream_handle(artifact_id: &str) -> String {
    format!("{STREAM_HANDLE_SCHEME}{artifact_id}")
}

fn hash_file(path: &Path) -> Result<String, ArtifactStoreError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}
