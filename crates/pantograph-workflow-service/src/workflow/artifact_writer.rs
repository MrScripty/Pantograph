use std::sync::{Arc, Mutex};

use super::{
    ArtifactBodyRead, ArtifactConsumeAcknowledgementRequest,
    ArtifactConsumeAcknowledgementResponse, ArtifactDescriptor, ArtifactPolicy,
    ArtifactReadRequest, ArtifactStore, ArtifactStoreError, ArtifactStoreStats,
    ArtifactStreamBodyRead, ArtifactStreamChunkRecord, ArtifactStreamChunkWriteRequest,
    ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest, ArtifactStreamReadRequest,
    ArtifactWriteRequest, WorkflowServiceError,
};

#[derive(Clone)]
pub struct WorkflowArtifactWriter {
    artifact_store: Arc<Mutex<ArtifactStore>>,
}

impl WorkflowArtifactWriter {
    #[must_use]
    pub fn new(store: ArtifactStore) -> Self {
        Self {
            artifact_store: Arc::new(Mutex::new(store)),
        }
    }

    pub fn write_artifact(
        &self,
        request: ArtifactWriteRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .write_artifact(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn descriptor(
        &self,
        artifact_id: &str,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .descriptor(artifact_id)
            .map_err(artifact_store_error)
    }

    pub(crate) fn read_body(
        &self,
        request: ArtifactReadRequest,
    ) -> Result<ArtifactBodyRead, WorkflowServiceError> {
        self.artifact_store_guard()?
            .read_body(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn open_stream(
        &self,
        request: ArtifactStreamOpenRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .open_stream(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn append_stream_chunk(
        &self,
        request: ArtifactStreamChunkWriteRequest,
    ) -> Result<ArtifactStreamChunkRecord, WorkflowServiceError> {
        self.artifact_store_guard()?
            .append_stream_chunk(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn read_stream_body(
        &self,
        request: ArtifactStreamReadRequest,
    ) -> Result<ArtifactStreamBodyRead, WorkflowServiceError> {
        self.artifact_store_guard()?
            .read_stream_body(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn finalize_stream(
        &self,
        request: ArtifactStreamFinalizeRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .finalize_stream(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn acknowledge_consume(
        &self,
        request: ArtifactConsumeAcknowledgementRequest,
    ) -> Result<ArtifactConsumeAcknowledgementResponse, WorkflowServiceError> {
        self.artifact_store_guard()?
            .acknowledge_consume(request)
            .map_err(artifact_store_error)
    }

    pub(crate) fn policy(&self) -> Result<ArtifactPolicy, WorkflowServiceError> {
        Ok(self.artifact_store_guard()?.policy().clone())
    }

    pub(crate) fn update_policy(
        &self,
        policy: ArtifactPolicy,
    ) -> Result<ArtifactPolicy, WorkflowServiceError> {
        let mut store = self.artifact_store_guard()?;
        store.update_policy(policy).map_err(artifact_store_error)?;
        Ok(store.policy().clone())
    }

    pub(crate) fn apply_retention_cleanup(&self, now_ms: u64) -> Result<u64, WorkflowServiceError> {
        self.artifact_store_guard()?
            .apply_retention_cleanup(now_ms)
            .map_err(artifact_store_error)
    }

    pub(crate) fn stats(&self) -> Result<ArtifactStoreStats, WorkflowServiceError> {
        self.artifact_store_guard()?
            .stats()
            .map_err(artifact_store_error)
    }

    fn artifact_store_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ArtifactStore>, WorkflowServiceError> {
        self.artifact_store
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("artifact store lock poisoned".to_string()))
    }
}

pub(crate) fn artifact_store_error(error: ArtifactStoreError) -> WorkflowServiceError {
    match error {
        ArtifactStoreError::InvalidArtifactId
        | ArtifactStoreError::NotFound { .. }
        | ArtifactStoreError::BodyUnavailable { .. }
        | ArtifactStoreError::ArtifactTooLarge { .. }
        | ArtifactStoreError::DiskLimitExceeded { .. }
        | ArtifactStoreError::StreamNotWritable { .. }
        | ArtifactStoreError::InvalidStreamSequence { .. }
        | ArtifactStoreError::ArtifactAccountingOverflow { .. }
        | ArtifactStoreError::InvalidByteRange => {
            WorkflowServiceError::InvalidRequest(error.to_string())
        }
        ArtifactStoreError::Io(_) | ArtifactStoreError::Manifest(_) => {
            WorkflowServiceError::Internal(error.to_string())
        }
    }
}
