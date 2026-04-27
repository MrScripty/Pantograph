use crate::{
    AttributionError, BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientCredential,
    ClientRegistrationRequest, ClientRegistrationResponse, ClientSessionDisconnectRequest,
    ClientSessionExpireRequest, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, CredentialProofRequest, WorkflowRunRecord,
    WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest, WorkflowRunStartRequest,
    WorkflowVersionRecord, WorkflowVersionResolveRequest,
};

pub trait AttributionRepository {
    fn register_client(
        &mut self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, AttributionError>;

    fn verify_credential(
        &self,
        request: &CredentialProofRequest,
    ) -> Result<ClientCredential, AttributionError>;

    fn open_session(
        &mut self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, AttributionError>;

    fn resume_session(
        &mut self,
        request: ClientSessionResumeRequest,
    ) -> Result<ClientSessionRecord, AttributionError>;

    fn disconnect_session(
        &mut self,
        request: ClientSessionDisconnectRequest,
    ) -> Result<ClientSessionRecord, AttributionError>;

    fn expire_session(
        &mut self,
        request: ClientSessionExpireRequest,
    ) -> Result<ClientSessionRecord, AttributionError>;

    fn create_bucket(
        &mut self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, AttributionError>;

    fn delete_bucket(
        &mut self,
        request: BucketDeleteRequest,
    ) -> Result<BucketRecord, AttributionError>;

    fn start_workflow_run(
        &mut self,
        request: WorkflowRunStartRequest,
    ) -> Result<WorkflowRunRecord, AttributionError>;

    fn resolve_workflow_version(
        &mut self,
        request: WorkflowVersionResolveRequest,
    ) -> Result<WorkflowVersionRecord, AttributionError>;

    fn create_workflow_run_snapshot(
        &mut self,
        request: WorkflowRunSnapshotRequest,
    ) -> Result<WorkflowRunSnapshotRecord, AttributionError>;
}
