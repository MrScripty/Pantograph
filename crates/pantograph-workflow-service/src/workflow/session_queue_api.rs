use crate::scheduler::scheduler_snapshot_workflow_run_id;

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    SchedulerQueueControlAction, SchedulerQueueControlActorScope, SchedulerQueueControlOutcome,
    SchedulerQueueControlPayload,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId, WorkflowRunSnapshotRecord};

use super::{
    WorkflowAdminQueueCancelRequest, WorkflowAdminQueueCancelResponse,
    WorkflowAdminQueuePushFrontRequest, WorkflowAdminQueuePushFrontResponse,
    WorkflowAdminQueueReprioritizeRequest, WorkflowAdminQueueReprioritizeResponse,
    WorkflowExecutionSessionInspectionRequest, WorkflowExecutionSessionInspectionResponse,
    WorkflowExecutionSessionQueueCancelRequest, WorkflowExecutionSessionQueueCancelResponse,
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionQueueListRequest,
    WorkflowExecutionSessionQueueListResponse, WorkflowExecutionSessionQueuePushFrontRequest,
    WorkflowExecutionSessionQueuePushFrontResponse,
    WorkflowExecutionSessionQueueReprioritizeRequest,
    WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowExecutionSessionStatusRequest,
    WorkflowExecutionSessionStatusResponse, WorkflowExecutionSessionSummary, WorkflowHost,
    WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse, WorkflowService,
    WorkflowServiceError,
};

impl WorkflowService {
    pub async fn workflow_get_execution_session_status(
        &self,
        request: WorkflowExecutionSessionStatusRequest,
    ) -> Result<WorkflowExecutionSessionStatusResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store_guard()?;
        store.touch_session(session_id)?;
        let session = store.session_summary(session_id)?;
        Ok(WorkflowExecutionSessionStatusResponse { session })
    }

    pub async fn workflow_get_execution_session_inspection<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionInspectionRequest,
    ) -> Result<WorkflowExecutionSessionInspectionResponse, WorkflowServiceError> {
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
        let workflow_execution_session_state = host
            .workflow_execution_session_inspection_state(session_id, &session.workflow_id)
            .await?;
        Ok(WorkflowExecutionSessionInspectionResponse {
            session,
            workflow_execution_session_state,
        })
    }

    pub async fn workflow_list_execution_session_queue(
        &self,
        request: WorkflowExecutionSessionQueueListRequest,
    ) -> Result<WorkflowExecutionSessionQueueListResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let mut store = self.session_store_guard()?;
        store.touch_session(session_id)?;
        let items = store.list_queue(session_id)?;
        Ok(WorkflowExecutionSessionQueueListResponse {
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
                workflow_run_id: scheduler_snapshot_workflow_run_id(&items),
                session,
                items,
                diagnostics: Some(diagnostics),
            });
        }

        self.graph_session_store
            .get_scheduler_snapshot(session_id)
            .await
    }

    pub async fn workflow_cancel_execution_session_queue_item(
        &self,
        request: WorkflowExecutionSessionQueueCancelRequest,
    ) -> Result<WorkflowExecutionSessionQueueCancelResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let (session, cancelled_item, cancel_result) = {
            let mut store = self.session_store_guard()?;
            let session = store.session_summary(session_id)?;
            let cancelled_item = store
                .list_queue(session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let cancel_result = store.cancel_queue_item(session_id, workflow_run_id);
            (session, cancelled_item, cancel_result)
        };
        match cancel_result {
            Ok(()) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    cancelled_item.as_ref(),
                    SchedulerQueueControlAction::Cancel,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::ClientSession,
                    None,
                    Some("queue item cancelled".to_string()),
                )?;
                Ok(WorkflowExecutionSessionQueueCancelResponse { ok: true })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    cancelled_item.as_ref(),
                    SchedulerQueueControlAction::Cancel,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::ClientSession,
                    None,
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    pub async fn workflow_admin_cancel_queue_item(
        &self,
        request: WorkflowAdminQueueCancelRequest,
    ) -> Result<WorkflowAdminQueueCancelResponse, WorkflowServiceError> {
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let (session, previous_item, cancel_result) = {
            let mut store = self.session_store_guard()?;
            let session_id = store.session_id_for_queue_item(workflow_run_id)?;
            let session = store.session_summary(&session_id)?;
            let previous_item = store
                .list_queue(&session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let cancel_result = store.cancel_queue_item(&session_id, workflow_run_id);
            (session, previous_item, cancel_result)
        };
        match cancel_result {
            Ok(()) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Cancel,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    None,
                    Some("admin cancelled queue item".to_string()),
                )?;
                Ok(WorkflowAdminQueueCancelResponse {
                    ok: true,
                    session_id: session.session_id,
                })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Cancel,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    None,
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    pub async fn workflow_reprioritize_execution_session_queue_item(
        &self,
        request: WorkflowExecutionSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowExecutionSessionQueueReprioritizeResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }
        let (session, previous_item, reprioritize_result) = {
            let mut store = self.session_store_guard()?;
            let session = store.session_summary(session_id)?;
            let previous_item = store
                .list_queue(session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let reprioritize_result =
                store.reprioritize_queue_item(session_id, workflow_run_id, request.priority);
            (session, previous_item, reprioritize_result)
        };
        match reprioritize_result {
            Ok(()) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Reprioritize,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::ClientSession,
                    Some(request.priority),
                    Some("queue item reprioritized".to_string()),
                )?;
                Ok(WorkflowExecutionSessionQueueReprioritizeResponse { ok: true })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Reprioritize,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::ClientSession,
                    Some(request.priority),
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    pub async fn workflow_admin_reprioritize_queue_item(
        &self,
        request: WorkflowAdminQueueReprioritizeRequest,
    ) -> Result<WorkflowAdminQueueReprioritizeResponse, WorkflowServiceError> {
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let (session, previous_item, reprioritize_result) = {
            let mut store = self.session_store_guard()?;
            let session_id = store.session_id_for_queue_item(workflow_run_id)?;
            let session = store.session_summary(&session_id)?;
            let previous_item = store
                .list_queue(&session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let reprioritize_result =
                store.reprioritize_queue_item(&session_id, workflow_run_id, request.priority);
            (session, previous_item, reprioritize_result)
        };
        match reprioritize_result {
            Ok(()) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Reprioritize,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    Some(request.priority),
                    Some("admin reprioritized queue item".to_string()),
                )?;
                Ok(WorkflowAdminQueueReprioritizeResponse {
                    ok: true,
                    session_id: session.session_id,
                })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::Reprioritize,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    Some(request.priority),
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    pub async fn workflow_push_execution_session_queue_item_to_front(
        &self,
        request: WorkflowExecutionSessionQueuePushFrontRequest,
    ) -> Result<WorkflowExecutionSessionQueuePushFrontResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }
        let (session, previous_item, push_result) = {
            let mut store = self.session_store_guard()?;
            let session = store.session_summary(session_id)?;
            let previous_item = store
                .list_queue(session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let push_result = store.push_queue_item_to_front(session_id, workflow_run_id);
            (session, previous_item, push_result)
        };
        match push_result {
            Ok(priority) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::PushToFront,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::ClientSession,
                    Some(priority),
                    Some("queue item pushed to front".to_string()),
                )?;
                Ok(WorkflowExecutionSessionQueuePushFrontResponse { ok: true, priority })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::PushToFront,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::ClientSession,
                    None,
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    pub async fn workflow_admin_push_queue_item_to_front(
        &self,
        request: WorkflowAdminQueuePushFrontRequest,
    ) -> Result<WorkflowAdminQueuePushFrontResponse, WorkflowServiceError> {
        let workflow_run_id = request.workflow_run_id.trim();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let (session, previous_item, push_result) = {
            let mut store = self.session_store_guard()?;
            let session_id = store.session_id_for_queue_item(workflow_run_id)?;
            let session = store.session_summary(&session_id)?;
            let previous_item = store
                .list_queue(&session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id);
            let push_result = store.push_queue_item_to_front(&session_id, workflow_run_id);
            (session, previous_item, push_result)
        };
        match push_result {
            Ok(priority) => {
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::PushToFront,
                    SchedulerQueueControlOutcome::Accepted,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    Some(priority),
                    Some("admin pushed queue item to front".to_string()),
                )?;
                Ok(WorkflowAdminQueuePushFrontResponse {
                    ok: true,
                    session_id: session.session_id,
                    priority,
                })
            }
            Err(error) => {
                let reason = queue_control_denial_reason(&error);
                self.record_scheduler_queue_control_event_if_configured(
                    &session,
                    workflow_run_id,
                    previous_item.as_ref(),
                    SchedulerQueueControlAction::PushToFront,
                    SchedulerQueueControlOutcome::Denied,
                    SchedulerQueueControlActorScope::GuiAdmin,
                    None,
                    Some(reason),
                )?;
                Err(error)
            }
        }
    }

    fn record_scheduler_queue_control_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        workflow_run_id: &str,
        previous_item: Option<&WorkflowExecutionSessionQueueItem>,
        action: SchedulerQueueControlAction,
        outcome: SchedulerQueueControlOutcome,
        actor_scope: SchedulerQueueControlActorScope,
        new_priority: Option<i32>,
        reason: Option<String>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = WorkflowId::try_from(session.workflow_id.clone())?;
        let snapshot = self.workflow_run_snapshot_if_configured(&workflow_run_id)?;
        let previous_queue_position = previous_item
            .and_then(|item| item.queue_position)
            .map(|position| {
                u32::try_from(position).map_err(|_| {
                    WorkflowServiceError::Internal(format!(
                        "queue position '{}' exceeds scheduler event limit",
                        position
                    ))
                })
            })
            .transpose()?;

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        DiagnosticsLedgerRepository::append_diagnostic_event(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms: crate::scheduler::unix_timestamp_ms() as i64,
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.workflow_semantic_version.clone()),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.client_id.clone()),
                client_session_id: snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.client_session_id.clone()),
                bucket_id: snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot.bucket_id.clone()),
                scheduler_policy_id: snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.scheduler_policy.clone()),
                retention_policy_id: snapshot
                    .as_ref()
                    .map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::SchedulerQueueControl(
                    SchedulerQueueControlPayload {
                        action,
                        outcome,
                        actor_scope,
                        previous_queue_position,
                        previous_priority: previous_item.map(|item| item.priority),
                        new_priority,
                        reason,
                    },
                ),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn workflow_run_snapshot_if_configured(
        &self,
        workflow_run_id: &WorkflowRunId,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, WorkflowServiceError> {
        if self.attribution_store.is_none() {
            return Ok(None);
        }
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_snapshot(workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }
}

fn queue_control_denial_reason(error: &WorkflowServiceError) -> String {
    match error {
        WorkflowServiceError::QueueItemNotFound(_) => "queue item not found".to_string(),
        WorkflowServiceError::InvalidRequest(message) if message.contains("currently running") => {
            "queue item currently running".to_string()
        }
        WorkflowServiceError::InvalidRequest(message)
            if message.contains("priority ceiling reached") =>
        {
            "queue priority ceiling reached".to_string()
        }
        _ => "queue control denied".to_string(),
    }
}
