use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, ClientRegistrationRequest,
    ClientRegistrationResponse, ClientSessionOpenRequest, ClientSessionOpenResponse,
    ClientSessionRecord, ClientSessionResumeRequest, WorkflowAttributedRunRequest,
    WorkflowAttributedRunResponse, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse,
    WorkflowExecutionSessionCloseRequest, WorkflowExecutionSessionCloseResponse,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionCreateResponse,
    WorkflowExecutionSessionInspectionRequest, WorkflowExecutionSessionInspectionResponse,
    WorkflowExecutionSessionKeepAliveRequest, WorkflowExecutionSessionKeepAliveResponse,
    WorkflowExecutionSessionQueueCancelRequest, WorkflowExecutionSessionQueueCancelResponse,
    WorkflowExecutionSessionQueueListRequest, WorkflowExecutionSessionQueueListResponse,
    WorkflowExecutionSessionQueueReprioritizeRequest,
    WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowExecutionSessionRunRequest,
    WorkflowExecutionSessionStaleCleanupRequest, WorkflowExecutionSessionStaleCleanupResponse,
    WorkflowExecutionSessionStatusRequest, WorkflowExecutionSessionStatusResponse,
    WorkflowIoRequest, WorkflowIoResponse, WorkflowPreflightRequest, WorkflowPreflightResponse,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowServiceError,
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

    pub async fn create_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        self.workflow_service
            .create_workflow_execution_session(&self.host(), request)
            .await
    }

    pub async fn run_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_service
            .run_workflow_execution_session(&self.host(), request)
            .await
    }

    pub async fn close_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionCloseRequest,
    ) -> Result<WorkflowExecutionSessionCloseResponse, WorkflowServiceError> {
        self.workflow_service
            .close_workflow_execution_session(&self.host(), request)
            .await
    }

    pub async fn workflow_get_execution_session_status(
        &self,
        request: WorkflowExecutionSessionStatusRequest,
    ) -> Result<WorkflowExecutionSessionStatusResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_execution_session_status(request)
            .await
    }

    pub async fn workflow_get_execution_session_inspection(
        &self,
        request: WorkflowExecutionSessionInspectionRequest,
    ) -> Result<WorkflowExecutionSessionInspectionResponse, WorkflowServiceError> {
        let host = self.host();
        self.workflow_service
            .workflow_get_execution_session_inspection(&host, request)
            .await
    }

    pub async fn workflow_list_execution_session_queue(
        &self,
        request: WorkflowExecutionSessionQueueListRequest,
    ) -> Result<WorkflowExecutionSessionQueueListResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_list_execution_session_queue(request)
            .await
    }

    pub async fn workflow_cleanup_stale_execution_sessions(
        &self,
        request: WorkflowExecutionSessionStaleCleanupRequest,
    ) -> Result<WorkflowExecutionSessionStaleCleanupResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cleanup_stale_execution_sessions(request)
            .await
    }

    pub async fn workflow_cancel_execution_session_queue_item(
        &self,
        request: WorkflowExecutionSessionQueueCancelRequest,
    ) -> Result<WorkflowExecutionSessionQueueCancelResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cancel_execution_session_queue_item(request)
            .await
    }

    pub async fn workflow_reprioritize_execution_session_queue_item(
        &self,
        request: WorkflowExecutionSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_reprioritize_execution_session_queue_item(request)
            .await
    }

    pub async fn workflow_set_execution_session_keep_alive(
        &self,
        request: WorkflowExecutionSessionKeepAliveRequest,
    ) -> Result<WorkflowExecutionSessionKeepAliveResponse, WorkflowServiceError> {
        let host = self.host();
        let response = self
            .workflow_service
            .workflow_set_execution_session_keep_alive(&host, request)
            .await?;
        host.sync_loaded_session_runtime_retention_hint(
            &response.session_id,
            response.keep_alive,
            response.state,
        )?;
        Ok(response)
    }
}
