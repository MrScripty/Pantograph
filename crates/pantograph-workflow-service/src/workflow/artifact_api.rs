use super::{
    diagnostic_errors::{
        WorkflowDiagnosticArtifactScope, WorkflowDiagnosticErrorRecordRequest,
        WorkflowDiagnosticRunContext,
    },
    ArtifactAttribution, ArtifactBodyRead, ArtifactConsumeAcknowledgementRequest,
    ArtifactConsumeAcknowledgementResponse, ArtifactDescriptor, ArtifactDescriptorQueryRequest,
    ArtifactDescriptorQueryResponse, ArtifactPolicy, ArtifactReadRequest, ArtifactStoreError,
    ArtifactStoreStats, ArtifactStreamBodyRead, ArtifactStreamChunkRecord,
    ArtifactStreamChunkWriteRequest, ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest,
    ArtifactStreamReadRequest, ArtifactWriteRequest, WorkflowService, WorkflowServiceError,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId, WorkflowVersionId};

impl WorkflowService {
    pub fn write_artifact(
        &self,
        request: ArtifactWriteRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        let attribution = request.attribution.clone();
        let payload_ref = request.artifact_id.as_ref().map(artifact_payload_ref);
        let mut store = match self.artifact_store_guard() {
            Ok(store) => store,
            Err(error) => {
                return Err(self.artifact_error_with_diagnostics(&attribution, payload_ref, error));
            }
        };
        store.write_artifact(request).map_err(|error| {
            self.artifact_error_with_diagnostics(
                &attribution,
                payload_ref,
                artifact_store_error(error),
            )
        })
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
        let store = self.artifact_store_guard()?;
        let attribution = store
            .descriptor(&request.artifact_id)
            .ok()
            .map(|descriptor| descriptor.attribution);
        let payload_ref = Some(artifact_payload_ref(&request.artifact_id));
        match store.read_body(request) {
            Ok(response) => Ok(response),
            Err(error) => {
                let error = artifact_store_error(error);
                if let Some(attribution) = attribution.as_ref() {
                    Err(self.artifact_error_with_diagnostics(attribution, payload_ref, error))
                } else {
                    Err(error)
                }
            }
        }
    }

    pub fn open_artifact_stream(
        &self,
        request: ArtifactStreamOpenRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        let attribution = request.attribution.clone();
        let payload_ref = request.artifact_id.as_ref().map(artifact_payload_ref);
        let mut store = match self.artifact_store_guard() {
            Ok(store) => store,
            Err(error) => {
                return Err(self.artifact_error_with_diagnostics(&attribution, payload_ref, error));
            }
        };
        store.open_stream(request).map_err(|error| {
            self.artifact_error_with_diagnostics(
                &attribution,
                payload_ref,
                artifact_store_error(error),
            )
        })
    }

    pub fn append_artifact_stream_chunk(
        &self,
        request: ArtifactStreamChunkWriteRequest,
    ) -> Result<ArtifactStreamChunkRecord, WorkflowServiceError> {
        let mut store = self.artifact_store_guard()?;
        let attribution = store
            .descriptor(&request.artifact_id)
            .ok()
            .map(|descriptor| descriptor.attribution);
        let payload_ref = Some(artifact_payload_ref(&request.artifact_id));
        match store.append_stream_chunk(request) {
            Ok(record) => Ok(record),
            Err(error) => {
                let error = artifact_store_error(error);
                if let Some(attribution) = attribution.as_ref() {
                    Err(self.artifact_error_with_diagnostics(attribution, payload_ref, error))
                } else {
                    Err(error)
                }
            }
        }
    }

    pub fn read_artifact_stream_body(
        &self,
        request: ArtifactStreamReadRequest,
    ) -> Result<ArtifactStreamBodyRead, WorkflowServiceError> {
        let store = self.artifact_store_guard()?;
        let attribution = store
            .descriptor(&request.artifact_id)
            .ok()
            .map(|descriptor| descriptor.attribution);
        let payload_ref = Some(artifact_payload_ref(&request.artifact_id));
        match store.read_stream_body(request) {
            Ok(response) => Ok(response),
            Err(error) => {
                let error = artifact_store_error(error);
                if let Some(attribution) = attribution.as_ref() {
                    Err(self.artifact_error_with_diagnostics(attribution, payload_ref, error))
                } else {
                    Err(error)
                }
            }
        }
    }

    pub fn finalize_artifact_stream(
        &self,
        request: ArtifactStreamFinalizeRequest,
    ) -> Result<ArtifactDescriptor, WorkflowServiceError> {
        let mut store = self.artifact_store_guard()?;
        let attribution = store
            .descriptor(&request.artifact_id)
            .ok()
            .map(|descriptor| descriptor.attribution);
        let payload_ref = Some(artifact_payload_ref(&request.artifact_id));
        match store.finalize_stream(request) {
            Ok(descriptor) => Ok(descriptor),
            Err(error) => {
                let error = artifact_store_error(error);
                if let Some(attribution) = attribution.as_ref() {
                    Err(self.artifact_error_with_diagnostics(attribution, payload_ref, error))
                } else {
                    Err(error)
                }
            }
        }
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
        self.artifact_store_guard()?
            .stats()
            .map_err(artifact_store_error)
    }
}

impl WorkflowService {
    fn artifact_error_with_diagnostics(
        &self,
        attribution: &ArtifactAttribution,
        payload_ref: Option<String>,
        error: WorkflowServiceError,
    ) -> WorkflowServiceError {
        let Some(scope) = artifact_diagnostic_scope(attribution, payload_ref) else {
            return error;
        };
        let workflow_run_id = scope.run.workflow_run_id.clone();
        match self.record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::artifact_failed(scope, &error)
                .with_source_instance_id("artifact-store"),
        ) {
            Ok(outcome) => error.with_diagnostics(outcome.into_error_link(Some(&workflow_run_id))),
            Err(record_error) => record_error,
        }
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
        | ArtifactStoreError::ArtifactAccountingOverflow { .. }
        | ArtifactStoreError::InvalidByteRange => {
            WorkflowServiceError::InvalidRequest(error.to_string())
        }
        ArtifactStoreError::Io(_) | ArtifactStoreError::Manifest(_) => {
            WorkflowServiceError::Internal(error.to_string())
        }
    }
}

fn artifact_diagnostic_scope(
    attribution: &ArtifactAttribution,
    payload_ref: Option<String>,
) -> Option<WorkflowDiagnosticArtifactScope> {
    let workflow_id = attribution.workflow_id.as_ref()?;
    let workflow_version_id = attribution
        .workflow_version_id
        .as_ref()
        .map(|value| WorkflowVersionId::try_from(value.to_string()))
        .transpose()
        .ok()?;
    Some(WorkflowDiagnosticArtifactScope {
        run: WorkflowDiagnosticRunContext {
            workflow_run_id: WorkflowRunId::try_from(attribution.workflow_run_id.to_string())
                .ok()?,
            workflow_id: WorkflowId::try_from(workflow_id.to_string()).ok()?,
            workflow_version_id,
            workflow_semantic_version: None,
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            scheduler_policy_id: None,
            retention_policy_id: None,
        },
        node_id: attribution.node_id.clone(),
        payload_ref,
    })
}

fn artifact_payload_ref(artifact_id: impl AsRef<str>) -> String {
    format!("artifact://{}", artifact_id.as_ref())
}
