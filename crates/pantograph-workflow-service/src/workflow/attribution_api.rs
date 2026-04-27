use pantograph_runtime_attribution::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, WorkflowId, WorkflowVersionRecord,
    WorkflowVersionResolveRequest,
};

use crate::graph::{
    workflow_executable_topology, workflow_execution_fingerprint_for_topology, WorkflowGraph,
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
}
