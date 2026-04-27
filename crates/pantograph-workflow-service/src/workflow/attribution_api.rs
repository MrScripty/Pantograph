use pantograph_runtime_attribution::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, WorkflowId,
    WorkflowPresentationRevisionRecord, WorkflowPresentationRevisionResolveRequest, WorkflowRunId,
    WorkflowRunSnapshotRecord, WorkflowRunVersionProjection, WorkflowVersionId,
    WorkflowVersionRecord, WorkflowVersionResolveRequest,
};

use crate::graph::{
    workflow_executable_topology, workflow_execution_fingerprint_for_topology,
    workflow_presentation_fingerprint_for_metadata, workflow_presentation_metadata,
    workflow_presentation_metadata_json, WorkflowGraph,
};

use super::{validate_workflow_id, AttributionRepository, WorkflowService, WorkflowServiceError};

impl WorkflowService {
    pub fn register_attribution_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .register_client(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn open_client_session(
        &self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .open_session(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resume_client_session(
        &self,
        request: ClientSessionResumeRequest,
    ) -> Result<ClientSessionRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .resume_session(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn create_client_bucket(
        &self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .create_bucket(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn delete_client_bucket(
        &self,
        request: BucketDeleteRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        let mut store = self.attribution_store_guard()?;
        store
            .delete_bucket(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resolve_workflow_graph_version(
        &self,
        workflow_id: &str,
        semantic_version: &str,
        graph: &WorkflowGraph,
    ) -> Result<WorkflowVersionRecord, WorkflowServiceError> {
        validate_workflow_id(workflow_id)?;
        let topology = workflow_executable_topology(graph)?;
        let execution_fingerprint = workflow_execution_fingerprint_for_topology(&topology)?;
        let executable_topology_json = serde_json::to_string(&topology).map_err(|error| {
            WorkflowServiceError::CapabilityViolation(format!(
                "failed to encode workflow executable topology: {error}"
            ))
        })?;
        let request = WorkflowVersionResolveRequest {
            workflow_id: WorkflowId::try_from(workflow_id.to_string())?,
            semantic_version: semantic_version.to_string(),
            execution_fingerprint,
            executable_topology_json,
        };
        let mut store = self.attribution_store_guard()?;
        store
            .resolve_workflow_version(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn resolve_workflow_graph_presentation_revision(
        &self,
        workflow_id: &str,
        workflow_version_id: &str,
        graph: &WorkflowGraph,
    ) -> Result<WorkflowPresentationRevisionRecord, WorkflowServiceError> {
        validate_workflow_id(workflow_id)?;
        let metadata = workflow_presentation_metadata(graph);
        let presentation_fingerprint = workflow_presentation_fingerprint_for_metadata(&metadata)?;
        let presentation_metadata_json = workflow_presentation_metadata_json(&metadata)?;
        let request = WorkflowPresentationRevisionResolveRequest {
            workflow_id: WorkflowId::try_from(workflow_id.to_string())?,
            workflow_version_id: WorkflowVersionId::try_from(workflow_version_id.to_string())?,
            presentation_fingerprint,
            presentation_metadata_json,
        };
        let mut store = self.attribution_store_guard()?;
        store
            .resolve_workflow_presentation_revision(request)
            .map_err(WorkflowServiceError::from)
    }

    pub fn workflow_run_snapshot(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_snapshot(&workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }

    pub fn workflow_run_version_projection(
        &self,
        workflow_run_id: &str,
    ) -> Result<Option<WorkflowRunVersionProjection>, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_version_projection(&workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }
}
