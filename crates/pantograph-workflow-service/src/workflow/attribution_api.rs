use pantograph_runtime_attribution::{
    BucketCreateRequest, BucketDeleteRequest, BucketRecord, BucketSelection,
    ClientRegistrationRequest, ClientRegistrationResponse, ClientSessionOpenRequest,
    ClientSessionOpenResponse, ClientSessionRecord, ClientSessionResumeRequest, WorkflowId,
    WorkflowRunAttribution, WorkflowRunRecord, WorkflowRunStartRequest,
};

use super::{
    AttributionRepository, WorkflowHost, WorkflowRunRequest, WorkflowRunResponse, WorkflowService,
    WorkflowServiceError,
};

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowAttributedRunRequest {
    pub credential: pantograph_runtime_attribution::CredentialProofRequest,
    pub client_session_id: pantograph_runtime_attribution::ClientSessionId,
    pub bucket_selection: BucketSelection,
    pub run: WorkflowRunRequest,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowAttributedRunResponse {
    pub run: WorkflowRunResponse,
    pub workflow_run: WorkflowRunRecord,
    pub attribution: WorkflowRunAttribution,
}

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

    pub async fn workflow_run_attributed<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowAttributedRunRequest,
    ) -> Result<WorkflowAttributedRunResponse, WorkflowServiceError> {
        if request
            .run
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "run_id is backend-owned for attributed workflow runs".to_string(),
            ));
        }

        let workflow_id = WorkflowId::try_from(request.run.workflow_id.clone())
            .map_err(WorkflowServiceError::from)?;
        let workflow_run = {
            let mut store = self.attribution_store_guard()?;
            store
                .start_workflow_run(WorkflowRunStartRequest {
                    credential: request.credential,
                    client_session_id: request.client_session_id,
                    workflow_id,
                    bucket_selection: request.bucket_selection,
                })
                .map_err(WorkflowServiceError::from)?
        };

        let mut run_request = request.run;
        run_request.run_id = Some(workflow_run.workflow_run_id.to_string());
        let run = self
            .workflow_run_internal(host, run_request, None, None)
            .await?;

        Ok(WorkflowAttributedRunResponse {
            attribution: WorkflowRunAttribution {
                client_id: workflow_run.client_id.clone(),
                client_session_id: workflow_run.client_session_id.clone(),
                bucket_id: workflow_run.bucket_id.clone(),
                workflow_run_id: workflow_run.workflow_run_id.clone(),
            },
            run,
            workflow_run,
        })
    }
}
