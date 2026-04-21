use crate::scheduler::scheduler_snapshot_trace_execution_id;

use super::{
    WorkflowHost, WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowService, WorkflowServiceError, WorkflowSessionInspectionRequest,
    WorkflowSessionInspectionResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionStatusRequest,
    WorkflowSessionStatusResponse,
};

impl WorkflowService {
    pub async fn workflow_get_session_status(
        &self,
        request: WorkflowSessionStatusRequest,
    ) -> Result<WorkflowSessionStatusResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store_guard()?;
        store.touch_session(session_id)?;
        let session = store.session_summary(session_id)?;
        Ok(WorkflowSessionStatusResponse { session })
    }

    pub async fn workflow_get_session_inspection<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowSessionInspectionRequest,
    ) -> Result<WorkflowSessionInspectionResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let session = {
            let mut store = self.session_store_guard()?;
            store.touch_session(session_id)?;
            store.session_summary(session_id)?
        };
        let workflow_session_state = host
            .workflow_session_inspection_state(session_id, &session.workflow_id)
            .await?;
        Ok(WorkflowSessionInspectionResponse {
            session,
            workflow_session_state,
        })
    }

    pub async fn workflow_list_session_queue(
        &self,
        request: WorkflowSessionQueueListRequest,
    ) -> Result<WorkflowSessionQueueListResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store_guard()?;
        store.touch_session(session_id)?;
        let items = store.list_queue(session_id)?;
        Ok(WorkflowSessionQueueListResponse {
            session_id: session_id.to_string(),
            items,
        })
    }

    pub async fn workflow_get_scheduler_snapshot(
        &self,
        request: WorkflowSchedulerSnapshotRequest,
    ) -> Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }

        let scheduler_diagnostics_provider = self
            .scheduler_diagnostics_provider
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "scheduler diagnostics provider lock poisoned".to_string(),
                )
            })?
            .clone();

        let workflow_snapshot = {
            let mut store = self.session_store_guard()?;
            match store
                .touch_session(session_id)
                .and_then(|_| store.session_summary(session_id))
            {
                Ok(session) => {
                    let items = store.list_queue(session_id)?;
                    let runtime_diagnostics_request =
                        store.scheduler_runtime_diagnostics_request(session_id)?;
                    let diagnostics = store.scheduler_snapshot_diagnostics(session_id)?;
                    Some((session, items, diagnostics, runtime_diagnostics_request))
                }
                Err(WorkflowServiceError::SessionNotFound(_)) => None,
                Err(error) => return Err(error),
            }
        };

        if let Some((session, items, mut diagnostics, runtime_diagnostics_request)) =
            workflow_snapshot
        {
            if let Some(provider) = scheduler_diagnostics_provider.as_ref() {
                diagnostics.runtime_registry = provider
                    .scheduler_runtime_registry_diagnostics(&runtime_diagnostics_request)
                    .await?;
            }
            return Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some(session.workflow_id.clone()),
                session_id: session_id.to_string(),
                trace_execution_id: scheduler_snapshot_trace_execution_id(&items),
                session,
                items,
                diagnostics: Some(diagnostics),
            });
        }

        self.graph_session_store
            .get_scheduler_snapshot(session_id)
            .await
    }

    pub async fn workflow_cancel_session_queue_item(
        &self,
        request: WorkflowSessionQueueCancelRequest,
    ) -> Result<WorkflowSessionQueueCancelResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let queue_id = request.queue_id.trim();
        if queue_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "queue_id must be non-empty".to_string(),
            ));
        }

        let mut store = self.session_store_guard()?;
        store.cancel_queue_item(session_id, queue_id)?;
        Ok(WorkflowSessionQueueCancelResponse { ok: true })
    }

    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request: WorkflowSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowSessionQueueReprioritizeResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let queue_id = request.queue_id.trim();
        if queue_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "queue_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store_guard()?;
        store.reprioritize_queue_item(session_id, queue_id, request.priority)?;
        Ok(WorkflowSessionQueueReprioritizeResponse { ok: true })
    }
}
