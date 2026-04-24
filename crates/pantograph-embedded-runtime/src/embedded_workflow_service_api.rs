use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, WorkflowAttributedRunRequest,
    WorkflowAttributedRunResponse, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse,
    WorkflowIoRequest, WorkflowIoResponse, WorkflowPreflightRequest, WorkflowPreflightResponse,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCloseResponse, WorkflowSessionCreateRequest, WorkflowSessionCreateResponse,
    WorkflowSessionInspectionRequest, WorkflowSessionInspectionResponse,
    WorkflowSessionKeepAliveRequest, WorkflowSessionKeepAliveResponse,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueCancelResponse,
    WorkflowSessionQueueListRequest, WorkflowSessionQueueListResponse,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionQueueReprioritizeResponse,
    WorkflowSessionRunRequest, WorkflowSessionStaleCleanupRequest,
    WorkflowSessionStaleCleanupResponse, WorkflowSessionStatusRequest,
    WorkflowSessionStatusResponse,
};

use crate::EmbeddedRuntime;

impl EmbeddedRuntime {
    pub fn register_attribution_client(
        &self,
        request: ClientRegistrationRequest,
    ) -> Result<ClientRegistrationResponse, WorkflowServiceError> {
        self.workflow_service.register_attribution_client(request)
    }

    pub fn open_client_session(
        &self,
        request: ClientSessionOpenRequest,
    ) -> Result<ClientSessionOpenResponse, WorkflowServiceError> {
        self.workflow_service.open_client_session(request)
    }

    pub fn resume_client_session(
        &self,
        request: ClientSessionResumeRequest,
    ) -> Result<ClientSessionRecord, WorkflowServiceError> {
        self.workflow_service.resume_client_session(request)
    }

    pub fn create_client_bucket(
        &self,
        request: BucketCreateRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        self.workflow_service.create_client_bucket(request)
    }

    pub fn delete_client_bucket(
        &self,
        request: BucketDeleteRequest,
    ) -> Result<BucketRecord, WorkflowServiceError> {
        self.workflow_service.delete_client_bucket(request)
    }

    pub async fn workflow_run_attributed(
        &self,
        request: WorkflowAttributedRunRequest,
    ) -> Result<WorkflowAttributedRunResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_run_attributed(&self.host(), request)
            .await
    }

    pub async fn workflow_run(
        &self,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_run(&self.host(), request)
            .await
    }

    pub async fn workflow_get_capabilities(
        &self,
        request: WorkflowCapabilitiesRequest,
    ) -> Result<WorkflowCapabilitiesResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_capabilities(&self.host(), request)
            .await
    }

    pub async fn workflow_get_io(
        &self,
        request: WorkflowIoRequest,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_io(&self.host(), request)
            .await
    }

    pub async fn workflow_preflight(
        &self,
        request: WorkflowPreflightRequest,
    ) -> Result<WorkflowPreflightResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_preflight(&self.host(), request)
            .await
    }

    pub async fn create_workflow_session(
        &self,
        request: WorkflowSessionCreateRequest,
    ) -> Result<WorkflowSessionCreateResponse, WorkflowServiceError> {
        self.workflow_service
            .create_workflow_session(&self.host(), request)
            .await
    }

    pub async fn run_workflow_session(
        &self,
        request: WorkflowSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_service
            .run_workflow_session(&self.host(), request)
            .await
    }

    pub async fn close_workflow_session(
        &self,
        request: WorkflowSessionCloseRequest,
    ) -> Result<WorkflowSessionCloseResponse, WorkflowServiceError> {
        self.workflow_service
            .close_workflow_session(&self.host(), request)
            .await
    }

    pub async fn workflow_get_session_status(
        &self,
        request: WorkflowSessionStatusRequest,
    ) -> Result<WorkflowSessionStatusResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_session_status(request)
            .await
    }

    pub async fn workflow_get_session_inspection(
        &self,
        request: WorkflowSessionInspectionRequest,
    ) -> Result<WorkflowSessionInspectionResponse, WorkflowServiceError> {
        let host = self.host();
        self.workflow_service
            .workflow_get_session_inspection(&host, request)
            .await
    }

    pub async fn workflow_list_session_queue(
        &self,
        request: WorkflowSessionQueueListRequest,
    ) -> Result<WorkflowSessionQueueListResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_list_session_queue(request)
            .await
    }

    pub async fn workflow_cleanup_stale_sessions(
        &self,
        request: WorkflowSessionStaleCleanupRequest,
    ) -> Result<WorkflowSessionStaleCleanupResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cleanup_stale_sessions(request)
            .await
    }

    pub async fn workflow_cancel_session_queue_item(
        &self,
        request: WorkflowSessionQueueCancelRequest,
    ) -> Result<WorkflowSessionQueueCancelResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cancel_session_queue_item(request)
            .await
    }

    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request: WorkflowSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowSessionQueueReprioritizeResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_reprioritize_session_queue_item(request)
            .await
    }

    pub async fn workflow_set_session_keep_alive(
        &self,
        request: WorkflowSessionKeepAliveRequest,
    ) -> Result<WorkflowSessionKeepAliveResponse, WorkflowServiceError> {
        let host = self.host();
        let response = self
            .workflow_service
            .workflow_set_session_keep_alive(&host, request)
            .await?;
        host.sync_loaded_session_runtime_retention_hint(
            &response.session_id,
            response.keep_alive,
            response.state,
        )?;
        Ok(response)
    }
}
