use pantograph_runtime_attribution::{WorkflowRunId, WorkflowRunSnapshotRecord};
use pantograph_scheduler::SchedulerTaskStateKind;

use super::{
    workflow_scheduler_task_run_summary, WorkflowErrorDiagnosticsLink,
    WorkflowExecutionSessionState, WorkflowExecutionSessionSummary, WorkflowPortBinding,
    WorkflowRunResponse, WorkflowService, WorkflowServiceError,
};

pub(super) struct WorkflowRunFinalizationRequest<'a> {
    pub(super) session: &'a WorkflowExecutionSessionSummary,
    pub(super) run_snapshot: Option<&'a WorkflowRunSnapshotRecord>,
    pub(super) session_id: &'a str,
    pub(super) workflow_run_id: &'a str,
    pub(super) workflow_semantic_version: &'a str,
    pub(super) io_artifact_inputs: Option<&'a [WorkflowPortBinding]>,
    pub(super) run_result: Result<WorkflowRunResponse, WorkflowServiceError>,
}

pub(super) struct WorkflowRunFinalizationOutcome {
    pub(super) unload_runtime: bool,
    pub(super) run_result: Result<WorkflowRunResponse, WorkflowServiceError>,
}

pub(super) fn finalize_admitted_workflow_run(
    service: &WorkflowService,
    request: WorkflowRunFinalizationRequest<'_>,
) -> Result<WorkflowRunFinalizationOutcome, WorkflowServiceError> {
    let finish_state = {
        let mut store = service.session_store_guard()?;
        store.finish_run(request.session_id, request.workflow_run_id)?
    };
    record_finished_workflow_run(service, request, finish_state.unload_runtime)
}

/// Finish only a refused, unassigned runtime branch with no active task custody.
/// The worker must retain its registered responder and settle its event first.
pub(super) fn finalize_unstarted_runtime_branch_failure(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
    workflow_id: &str,
    error: WorkflowServiceError,
) -> Result<(), WorkflowServiceError> {
    let run_id = WorkflowRunId::try_from(workflow_run_id.to_owned())?;
    let snapshot = service.workflow_run_snapshot_for_execution_resume_if_configured(&run_id)?;
    let (session, active_run, finish_state) = {
        let mut store = service.session_store_guard()?;
        let session = match store.session_summary(session_id) {
            Ok(session) => session,
            // Unadmitted/closed commands still return their original worker error.
            Err(WorkflowServiceError::SessionNotFound(_)) => return Ok(()),
            Err(error) => return Err(error),
        };
        if session.state != WorkflowExecutionSessionState::Running {
            // A completed continuation must not publish another terminal event.
            return Ok(());
        }
        let active_run = store.active_run_context(session_id, workflow_run_id)?;
        if session.workflow_id != workflow_id {
            return Err(WorkflowServiceError::InvalidRequest(
                "preparation refusal workflow does not match the active session".into(),
            ));
        }
        let (graph, records) = store
            .active_run_scheduler_task_state(session_id, workflow_run_id)?
            .ok_or_else(|| {
                WorkflowServiceError::Internal(
                    "preparation refusal has no active scheduler task graph".into(),
                )
            })?;
        if graph.workflow_id.as_str() != workflow_id
            || graph.workflow_run_id.as_str() != workflow_run_id
        {
            return Err(WorkflowServiceError::InvalidRequest(
                "preparation refusal does not match the active scheduler graph".into(),
            ));
        }
        workflow_scheduler_task_run_summary(&graph, &records)
            .map(|_| ())
            .map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "preparation refusal scheduler state is incoherent: {error}"
                ))
            })?;
        let attempts =
            store.active_run_scheduler_task_attempt_read_facts(session_id, workflow_run_id)?;
        if !attempts.is_empty()
            || records
                .iter()
                .any(|record| record.state.kind() == SchedulerTaskStateKind::Running)
            || graph.tasks.iter().any(|task| {
                store.completion_cleanup_pending(session_id, workflow_run_id, task.task_id.as_str())
            })
        {
            return Err(WorkflowServiceError::scheduler_busy(
                "preparation refusal retains started task or pending cleanup ownership",
            ));
        }
        // Validate and finish under the same lock: a concurrent Start cannot
        // install task/reservation custody between the guard and finish_run.
        let finish_state = store.finish_run(session_id, workflow_run_id)?;
        (session, active_run, finish_state)
    };
    record_finished_workflow_run(
        service,
        WorkflowRunFinalizationRequest {
            session: &session,
            run_snapshot: snapshot.as_ref(),
            session_id,
            workflow_run_id,
            workflow_semantic_version: &active_run.workflow_semantic_version,
            io_artifact_inputs: None,
            run_result: Err(error),
        },
        finish_state.unload_runtime,
    )
    .map(|_| ())
}

fn record_finished_workflow_run(
    service: &WorkflowService,
    request: WorkflowRunFinalizationRequest<'_>,
    unload_runtime: bool,
) -> Result<WorkflowRunFinalizationOutcome, WorkflowServiceError> {
    if let Err(record_error) = service.record_run_terminal_event_if_configured(
        request.session,
        request.run_snapshot,
        request.workflow_run_id,
        Some(request.workflow_semantic_version),
        &request.run_result,
    ) {
        if let Err(error) = request.run_result {
            return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                workflow_run_id: Some(request.workflow_run_id.to_string()),
                diagnostic_event_id: None,
                diagnostics_unavailable: Some(record_error.message().to_string()),
            }));
        }
        return Err(record_error);
    }
    if let (Some(inputs), Ok(response)) = (request.io_artifact_inputs, request.run_result.as_ref())
    {
        service.record_workflow_io_artifact_events_if_configured(
            request.session,
            request.run_snapshot,
            request.workflow_run_id,
            request.workflow_semantic_version,
            inputs,
            &response.outputs,
        )?;
    }
    Ok(WorkflowRunFinalizationOutcome {
        unload_runtime,
        run_result: request.run_result,
    })
}
