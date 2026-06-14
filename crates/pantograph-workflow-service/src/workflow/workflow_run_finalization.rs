use pantograph_runtime_attribution::WorkflowRunSnapshotRecord;

use super::{
    WorkflowErrorDiagnosticsLink, WorkflowExecutionSessionSummary, WorkflowPortBinding,
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
        unload_runtime: finish_state.unload_runtime,
        run_result: request.run_result,
    })
}
