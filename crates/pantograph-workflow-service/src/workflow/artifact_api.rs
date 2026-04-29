use super::{
    ArtifactBodyRead, ArtifactConsumeAcknowledgementRequest,
    ArtifactConsumeAcknowledgementResponse, ArtifactDescriptor, ArtifactDescriptorQueryRequest,
    ArtifactDescriptorQueryResponse, ArtifactPolicy, ArtifactReadRequest, ArtifactStoreError,
    ArtifactStoreStats, ArtifactStreamChunkRecord, ArtifactStreamChunkWriteRequest,
    ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest, ArtifactWriteRequest,
    WorkflowService, WorkflowServiceError,
};

impl WorkflowService {
    pub fn write_artifact(
        &self,
        request: ArtifactWriteRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .write_artifact(request)
            .map_err(artifact_store_error)
    }

    pub fn artifact_descriptor(
        &self,
        request: ArtifactDescriptorQueryRequest,
    ) -> Result<ArtifactDescriptorQueryResponse, WorkflowServiceError> {
        let artifact = self
            .artifact_store_guard()?
            .descriptor(&request.artifact_id)
            .map(Some)
            .map_err(artifact_store_error)?;
        Ok(ArtifactDescriptorQueryResponse { artifact })
    }

    pub fn read_artifact_body(
        &self,
        request: ArtifactReadRequest,
    ) -> Result<ArtifactBodyRead, WorkflowServiceError> {
        self.artifact_store_guard()?
            .read_body(request)
            .map_err(artifact_store_error)
    }

    pub fn open_artifact_stream(
        &self,
        request: ArtifactStreamOpenRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .open_stream(request)
            .map_err(artifact_store_error)
    }

    pub fn append_artifact_stream_chunk(
        &self,
        request: ArtifactStreamChunkWriteRequest,
    ) -> Result<ArtifactStreamChunkRecord, WorkflowServiceError> {
        self.artifact_store_guard()?
            .append_stream_chunk(request)
            .map_err(artifact_store_error)
    }

    pub fn finalize_artifact_stream(
        &self,
        request: ArtifactStreamFinalizeRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        self.artifact_store_guard()?
            .finalize_stream(request)
            .map_err(artifact_store_error)
    }

    pub fn acknowledge_artifact_consumed(
        &self,
        request: ArtifactConsumeAcknowledgementRequest,
    ) -> Result<ArtifactConsumeAcknowledgementResponse, WorkflowServiceError> {
        self.artifact_store_guard()?
            .acknowledge_consume(request)
            .map_err(artifact_store_error)
    }

    pub fn artifact_policy(&self) -> Result<ArtifactPolicy, WorkflowServiceError> {
        Ok(self.artifact_store_guard()?.policy().clone())
    }

    pub fn update_artifact_policy(
        &self,
        policy: ArtifactPolicy,
    ) -> Result<ArtifactPolicy, WorkflowServiceError> {
        let mut store = self.artifact_store_guard()?;
        store.update_policy(policy).map_err(artifact_store_error)?;
        Ok(store.policy().clone())
    }

    pub fn apply_artifact_store_retention_cleanup(
        &self,
        now_ms: u64,
    ) -> Result<u64, WorkflowServiceError> {
        self.artifact_store_guard()?
            .apply_retention_cleanup(now_ms)
            .map_err(artifact_store_error)
    }

    pub fn artifact_store_stats(&self) -> Result<ArtifactStoreStats, WorkflowServiceError> {
        Ok(self.artifact_store_guard()?.stats())
    }
}

fn artifact_store_error(error: ArtifactStoreError) -> WorkflowServiceError {
    match error {
        ArtifactStoreError::InvalidArtifactId
        | ArtifactStoreError::NotFound { .. }
        | ArtifactStoreError::BodyUnavailable { .. }
        | ArtifactStoreError::ArtifactTooLarge { .. }
        | ArtifactStoreError::DiskLimitExceeded { .. }
        | ArtifactStoreError::StreamNotWritable { .. }
        | ArtifactStoreError::InvalidStreamSequence { .. }
        | ArtifactStoreError::InvalidByteRange => {
            WorkflowServiceError::InvalidRequest(error.to_string())
        }
        ArtifactStoreError::Io(_) | ArtifactStoreError::Manifest(_) => {
            WorkflowServiceError::Internal(error.to_string())
        }
    }
}
