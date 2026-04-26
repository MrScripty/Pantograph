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

pub(super) fn trace_attempt_state_for_workflow_run(
    trace_store: &WorkflowTraceStore,
    workflow_run_id: &str,
) -> Option<TraceAttemptState> {
    trace_store
        .snapshot(&WorkflowTraceSnapshotRequest {
            workflow_run_id: Some(workflow_run_id.to_string()),
            session_id: None,
            workflow_id: None,
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
    workflow_run_id: &str,
) -> Option<TraceAttemptState> {
    traces
        .traces
        .iter()
        .find(|trace| trace.workflow_run_id == workflow_run_id)
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

pub(super) fn trace_event_workflow_run_id(event: &WorkflowTraceEvent) -> &str {
    match event {
        WorkflowTraceEvent::RunStarted {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::RunCompleted {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::RunFailed {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::RunCancelled {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::NodeStarted {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::NodeProgress {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::NodeStream {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::NodeCompleted {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::NodeFailed {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::WaitingForInput {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::GraphModified {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::IncrementalExecutionStarted {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id, ..
        }
        | WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id, ..
        } => workflow_run_id,
    }
}
