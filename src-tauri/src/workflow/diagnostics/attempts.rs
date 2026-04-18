use pantograph_workflow_service::{
    WorkflowTraceEvent, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TraceAttemptState {
    pub(super) started_at_ms: u64,
    pub(super) event_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OverlayRecordDecision {
    pub(super) reset_overlay: bool,
    pub(super) record_overlay: bool,
}

pub(super) fn trace_attempt_state_for_execution(
    trace_store: &WorkflowTraceStore,
    execution_id: &str,
) -> Option<TraceAttemptState> {
    trace_store
        .snapshot(&WorkflowTraceSnapshotRequest {
            execution_id: Some(execution_id.to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(true),
        })
        .expect("trace snapshot request should be valid")
        .traces
        .into_iter()
        .next()
        .map(|trace| TraceAttemptState {
            started_at_ms: trace.started_at_ms,
            event_count: trace.event_count,
        })
}

pub(super) fn trace_attempt_state_in_snapshot(
    traces: &WorkflowTraceSnapshotResponse,
    execution_id: &str,
) -> Option<TraceAttemptState> {
    traces
        .traces
        .iter()
        .find(|trace| trace.execution_id == execution_id)
        .map(|trace| TraceAttemptState {
            started_at_ms: trace.started_at_ms,
            event_count: trace.event_count,
        })
}

pub(super) fn overlay_record_decision(
    previous_state: Option<TraceAttemptState>,
    current_state: Option<TraceAttemptState>,
) -> OverlayRecordDecision {
    match (previous_state, current_state) {
        (None, Some(_)) => OverlayRecordDecision {
            reset_overlay: false,
            record_overlay: true,
        },
        (Some(previous), Some(current))
            if current.started_at_ms != previous.started_at_ms
                || current.event_count < previous.event_count =>
        {
            OverlayRecordDecision {
                reset_overlay: true,
                record_overlay: true,
            }
        }
        (Some(previous), Some(current)) if current.event_count > previous.event_count => {
            OverlayRecordDecision {
                reset_overlay: false,
                record_overlay: true,
            }
        }
        _ => OverlayRecordDecision {
            reset_overlay: false,
            record_overlay: false,
        },
    }
}

pub(super) fn trace_event_execution_id(event: &WorkflowTraceEvent) -> &str {
    match event {
        WorkflowTraceEvent::RunStarted { execution_id, .. }
        | WorkflowTraceEvent::RunCompleted { execution_id, .. }
        | WorkflowTraceEvent::RunFailed { execution_id, .. }
        | WorkflowTraceEvent::RunCancelled { execution_id, .. }
        | WorkflowTraceEvent::NodeStarted { execution_id, .. }
        | WorkflowTraceEvent::NodeProgress { execution_id, .. }
        | WorkflowTraceEvent::NodeStream { execution_id, .. }
        | WorkflowTraceEvent::NodeCompleted { execution_id, .. }
        | WorkflowTraceEvent::NodeFailed { execution_id, .. }
        | WorkflowTraceEvent::WaitingForInput { execution_id, .. }
        | WorkflowTraceEvent::GraphModified { execution_id, .. }
        | WorkflowTraceEvent::IncrementalExecutionStarted { execution_id, .. }
        | WorkflowTraceEvent::RuntimeSnapshotCaptured { execution_id, .. }
        | WorkflowTraceEvent::SchedulerSnapshotCaptured { execution_id, .. } => execution_id,
    }
}
