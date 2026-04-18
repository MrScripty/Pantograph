use std::collections::BTreeMap;

use super::runtime::apply_runtime_snapshot;
use super::scheduler::apply_scheduler_snapshot;
use super::store::{WorkflowTraceExecutionContext, WorkflowTraceRunState};
use super::types::{
    WorkflowTraceEvent, WorkflowTraceNodeRecord, WorkflowTraceNodeStatus,
    WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics, WorkflowTraceStatus,
};

pub(super) fn create_trace_run_state(
    execution_id: &str,
    workflow_id: Option<String>,
    context: &WorkflowTraceExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) -> WorkflowTraceRunState {
    WorkflowTraceRunState {
        execution_id: execution_id.to_string(),
        session_id: None,
        workflow_id,
        workflow_name: context.workflow_name.clone(),
        graph_fingerprint: context.graph_fingerprint.clone(),
        status: WorkflowTraceStatus::Running,
        started_at_ms: timestamp_ms,
        ended_at_ms: None,
        duration_ms: None,
        queue: WorkflowTraceQueueMetrics::default(),
        runtime: WorkflowTraceRuntimeMetrics::default(),
        node_count_at_start,
        event_count: 0,
        stream_event_count: 0,
        waiting_for_input: false,
        last_error: None,
        nodes_by_id: BTreeMap::new(),
    }
}

pub(super) fn apply_trace_event(
    trace: &mut WorkflowTraceRunState,
    context: &WorkflowTraceExecutionContext,
    event: &WorkflowTraceEvent,
    timestamp_ms: u64,
) {
    if is_idempotent_terminal_trace_event(trace, event) {
        return;
    }

    trace.event_count += 1;

    match event {
        WorkflowTraceEvent::RunStarted { node_count, .. } => {
            if trace_can_restart_attempt(trace) {
                reset_trace_for_restart(trace, context, timestamp_ms, *node_count);
            } else {
                trace.status = WorkflowTraceStatus::Running;
                trace.waiting_for_input = false;
                trace.last_error = None;
                trace.ended_at_ms = None;
                trace.duration_ms = None;
                trace.node_count_at_start = *node_count;
            }
        }
        WorkflowTraceEvent::NodeStarted { .. } if trace.status == WorkflowTraceStatus::Waiting => {
            trace.status = WorkflowTraceStatus::Running;
            trace.waiting_for_input = false;
        }
        WorkflowTraceEvent::NodeStarted { .. } => {}
        WorkflowTraceEvent::IncrementalExecutionStarted { .. }
            if trace.status == WorkflowTraceStatus::Waiting =>
        {
            trace.status = WorkflowTraceStatus::Running;
            trace.waiting_for_input = false;
        }
        WorkflowTraceEvent::NodeStream { .. } => {
            trace.stream_event_count += 1;
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            trace.status = WorkflowTraceStatus::Waiting;
            trace.waiting_for_input = true;
        }
        WorkflowTraceEvent::RunCompleted { .. } => {
            trace.status = WorkflowTraceStatus::Completed;
            trace.waiting_for_input = false;
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
        }
        WorkflowTraceEvent::RunFailed { error, .. } => {
            trace.status = WorkflowTraceStatus::Failed;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
        }
        WorkflowTraceEvent::RunCancelled { error, .. } => {
            trace.status = WorkflowTraceStatus::Cancelled;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
            cancel_active_trace_nodes(trace, error, timestamp_ms);
        }
        WorkflowTraceEvent::RuntimeSnapshotCaptured {
            captured_at_ms,
            runtime,
            capabilities,
            error,
            ..
        } => apply_runtime_snapshot(
            trace,
            runtime,
            capabilities.as_ref(),
            error.as_deref(),
            *captured_at_ms,
        ),
        WorkflowTraceEvent::SchedulerSnapshotCaptured {
            execution_id,
            session_id,
            captured_at_ms,
            session,
            items,
            diagnostics,
            error,
            ..
        } => apply_scheduler_snapshot(
            trace,
            execution_id,
            session_id,
            session.as_ref(),
            items,
            diagnostics.as_ref(),
            error.as_deref(),
            *captured_at_ms,
        ),
        WorkflowTraceEvent::NodeProgress { .. }
        | WorkflowTraceEvent::NodeCompleted { .. }
        | WorkflowTraceEvent::NodeFailed { .. }
        | WorkflowTraceEvent::GraphModified { .. }
        | WorkflowTraceEvent::IncrementalExecutionStarted { .. } => {}
    }

    let Some(node_id) = event.node_id() else {
        return;
    };
    let explicit_node_type = event.node_type().map(ToOwned::to_owned);
    let node = trace
        .nodes_by_id
        .entry(node_id.to_string())
        .or_insert_with(|| {
            create_trace_node_record(
                node_id,
                explicit_node_type
                    .clone()
                    .or_else(|| context.node_types_by_id.get(node_id).cloned()),
            )
        });
    if node.node_type.is_none() {
        node.node_type =
            explicit_node_type.or_else(|| context.node_types_by_id.get(node_id).cloned());
    }
    node.event_count += 1;

    match event {
        WorkflowTraceEvent::NodeStarted { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.started_at_ms.get_or_insert(timestamp_ms);
            node.ended_at_ms = None;
            node.duration_ms = None;
            node.last_error = None;
        }
        WorkflowTraceEvent::NodeProgress { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
        }
        WorkflowTraceEvent::NodeStream { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.stream_event_count += 1;
        }
        WorkflowTraceEvent::NodeCompleted { .. } => {
            node.status = WorkflowTraceNodeStatus::Completed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            node.last_error = None;
        }
        WorkflowTraceEvent::NodeFailed { error, .. } => {
            node.status = WorkflowTraceNodeStatus::Failed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            node.last_error = Some(error.clone());
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            node.status = WorkflowTraceNodeStatus::Waiting;
        }
        WorkflowTraceEvent::RunStarted { .. }
        | WorkflowTraceEvent::RunCompleted { .. }
        | WorkflowTraceEvent::RunFailed { .. }
        | WorkflowTraceEvent::RunCancelled { .. }
        | WorkflowTraceEvent::GraphModified { .. }
        | WorkflowTraceEvent::IncrementalExecutionStarted { .. }
        | WorkflowTraceEvent::RuntimeSnapshotCaptured { .. }
        | WorkflowTraceEvent::SchedulerSnapshotCaptured { .. } => {}
    }
}

fn is_idempotent_terminal_trace_event(
    trace: &WorkflowTraceRunState,
    event: &WorkflowTraceEvent,
) -> bool {
    match event {
        WorkflowTraceEvent::RunCompleted { .. } => {
            trace.status == WorkflowTraceStatus::Completed && trace.ended_at_ms.is_some()
        }
        WorkflowTraceEvent::RunFailed { error, .. } => {
            trace.status == WorkflowTraceStatus::Failed
                && trace.ended_at_ms.is_some()
                && trace.last_error.as_deref() == Some(error.as_str())
        }
        WorkflowTraceEvent::RunCancelled { error, .. } => {
            trace.status == WorkflowTraceStatus::Cancelled
                && trace.ended_at_ms.is_some()
                && trace.last_error.as_deref() == Some(error.as_str())
        }
        WorkflowTraceEvent::NodeCompleted { node_id, .. } => {
            trace.nodes_by_id.get(node_id).is_some_and(|node| {
                node.status == WorkflowTraceNodeStatus::Completed && node.ended_at_ms.is_some()
            })
        }
        WorkflowTraceEvent::NodeFailed { node_id, error, .. } => {
            trace.nodes_by_id.get(node_id).is_some_and(|node| {
                node.status == WorkflowTraceNodeStatus::Failed
                    && node.ended_at_ms.is_some()
                    && node.last_error.as_deref() == Some(error.as_str())
            })
        }
        _ => false,
    }
}

fn cancel_active_trace_nodes(trace: &mut WorkflowTraceRunState, error: &str, timestamp_ms: u64) {
    for node in trace.nodes_by_id.values_mut() {
        if matches!(
            node.status,
            WorkflowTraceNodeStatus::Running | WorkflowTraceNodeStatus::Waiting
        ) {
            node.status = WorkflowTraceNodeStatus::Cancelled;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            if node.last_error.is_none() {
                node.last_error = Some(error.to_string());
            }
        }
    }
}

fn trace_can_restart_attempt(trace: &WorkflowTraceRunState) -> bool {
    trace.ended_at_ms.is_some()
        || matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
        )
}

fn reset_trace_for_restart(
    trace: &mut WorkflowTraceRunState,
    context: &WorkflowTraceExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) {
    trace.workflow_name = context.workflow_name.clone();
    trace.graph_fingerprint = context.graph_fingerprint.clone();
    trace.status = WorkflowTraceStatus::Running;
    trace.started_at_ms = timestamp_ms;
    trace.ended_at_ms = None;
    trace.duration_ms = None;
    trace.queue = WorkflowTraceQueueMetrics::default();
    trace.runtime = WorkflowTraceRuntimeMetrics::default();
    trace.node_count_at_start = node_count_at_start;
    trace.event_count = 1;
    trace.stream_event_count = 0;
    trace.waiting_for_input = false;
    trace.last_error = None;
    trace.nodes_by_id.clear();
}

fn create_trace_node_record(node_id: &str, node_type: Option<String>) -> WorkflowTraceNodeRecord {
    WorkflowTraceNodeRecord {
        node_id: node_id.to_string(),
        node_type,
        status: WorkflowTraceNodeStatus::Running,
        started_at_ms: None,
        ended_at_ms: None,
        duration_ms: None,
        event_count: 0,
        stream_event_count: 0,
        last_error: None,
    }
}
