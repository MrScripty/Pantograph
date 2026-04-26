use super::store::WorkflowTraceRunState;
use super::types::{
    WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStatus,
};

pub(super) fn snapshot_for_request<'a>(
    traces: impl Iterator<Item = &'a WorkflowTraceRunState>,
    retained_trace_limit: usize,
    request: &WorkflowTraceSnapshotRequest,
) -> WorkflowTraceSnapshotResponse {
    let traces = matching_traces(traces, request)
        .into_iter()
        .map(WorkflowTraceRunState::snapshot)
        .collect();

    WorkflowTraceSnapshotResponse {
        traces,
        retained_trace_limit,
    }
}

pub(super) fn runtime_metrics_selection<'a>(
    traces: impl Iterator<Item = &'a WorkflowTraceRunState>,
    request: &WorkflowTraceSnapshotRequest,
) -> WorkflowTraceRuntimeSelection {
    let matching_traces = matching_traces(traces, request);
    let matched_workflow_run_ids = matching_traces
        .iter()
        .map(|trace| trace.workflow_run_id.clone())
        .collect::<Vec<_>>();

    match matching_traces.as_slice() {
        [trace] => WorkflowTraceRuntimeSelection {
            workflow_run_id: Some(trace.workflow_run_id.clone()),
            runtime: Some(trace.runtime.clone()),
            matched_workflow_run_ids,
        },
        _ => WorkflowTraceRuntimeSelection {
            workflow_run_id: None,
            runtime: None,
            matched_workflow_run_ids,
        },
    }
}

fn matching_traces<'a>(
    traces: impl Iterator<Item = &'a WorkflowTraceRunState>,
    request: &WorkflowTraceSnapshotRequest,
) -> Vec<&'a WorkflowTraceRunState> {
    traces
        .filter(|trace| trace_matches_request(trace, request))
        .collect()
}

fn trace_matches_request(
    trace: &WorkflowTraceRunState,
    request: &WorkflowTraceSnapshotRequest,
) -> bool {
    if let Some(workflow_run_id) = request.workflow_run_id.as_deref() {
        if trace.workflow_run_id != workflow_run_id {
            return false;
        }
    }
    if let Some(session_id) = request.session_id.as_deref() {
        if trace.session_id.as_deref() != Some(session_id) {
            return false;
        }
    }
    if let Some(workflow_id) = request.workflow_id.as_deref() {
        if trace.workflow_id.as_deref() != Some(workflow_id) {
            return false;
        }
    }
    if request.include_completed == Some(false)
        && matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
        )
    {
        return false;
    }

    true
}
