use std::collections::BTreeMap;

use super::runtime::apply_runtime_snapshot;
use super::scheduler::apply_scheduler_snapshot;
use super::store::{WorkflowTraceExecutionContext, WorkflowTraceRunState};
use super::types::{
    WorkflowTraceEvent, WorkflowTraceNodeRecord, WorkflowTraceNodeStatus,
    WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics, WorkflowTraceStatus,
};
use crate::workflow::{
    checked_timing_duration_ms, WorkflowTimingAttemptId, WorkflowTimingAttemptKind,
    WorkflowTimingDiagnostic,
};

pub(super) fn create_trace_run_state(
    workflow_run_id: &str,
    workflow_id: Option<String>,
    context: &WorkflowTraceExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) -> WorkflowTraceRunState {
    WorkflowTraceRunState {
        workflow_run_id: workflow_run_id.to_string(),
        session_id: None,
        workflow_id,
        graph_fingerprint: context.graph_fingerprint.clone(),
        status: WorkflowTraceStatus::Running,
        started_at_ms: timestamp_ms,
        ended_at_ms: None,
        duration_ms: None,
        timing_attempt_id: WorkflowTimingAttemptId::generate(),
        timing_diagnostics: Vec::new(),
        queue: WorkflowTraceQueueMetrics::default(),
        runtime: WorkflowTraceRuntimeMetrics::default(),
        node_count_at_start,
        event_count: 0,
        stream_event_count: 0,
        last_dirty_tasks: Vec::new(),
        last_incremental_task_ids: Vec::new(),
        last_graph_memory_impact: None,
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
                trace.last_dirty_tasks.clear();
                trace.last_incremental_task_ids.clear();
                trace.last_graph_memory_impact = None;
            }
        }
        WorkflowTraceEvent::NodeStarted { .. } if trace.status == WorkflowTraceStatus::Waiting => {
            trace.status = WorkflowTraceStatus::Running;
            trace.waiting_for_input = false;
        }
        WorkflowTraceEvent::NodeStarted { .. } => {}
        WorkflowTraceEvent::IncrementalExecutionStarted { task_ids, .. }
            if trace.status == WorkflowTraceStatus::Waiting =>
        {
            trace.status = WorkflowTraceStatus::Running;
            trace.waiting_for_input = false;
            trace.last_incremental_task_ids = task_ids.clone();
        }
        WorkflowTraceEvent::IncrementalExecutionStarted { task_ids, .. } => {
            trace.last_incremental_task_ids = task_ids.clone();
        }
        WorkflowTraceEvent::NodeStream { .. } => {
            trace.stream_event_count += 1;
        }
        WorkflowTraceEvent::GraphModified {
            dirty_tasks,
            memory_impact,
            ..
        } => {
            trace.last_dirty_tasks = dirty_tasks.clone();
            trace.last_graph_memory_impact = memory_impact.clone();
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            trace.status = WorkflowTraceStatus::Waiting;
            trace.waiting_for_input = true;
        }
        WorkflowTraceEvent::RunCompleted { .. } => {
            trace.status = WorkflowTraceStatus::Completed;
            trace.waiting_for_input = false;
            trace.ended_at_ms = Some(timestamp_ms);
            let diagnostics_before = trace.timing_diagnostics.len();
            trace.duration_ms = trace_duration_ms(trace, timestamp_ms);
            if trace.timing_diagnostics.len() > diagnostics_before {
                trace.status = WorkflowTraceStatus::Failed;
            }
        }
        WorkflowTraceEvent::RunFailed { error, .. } => {
            trace.status = WorkflowTraceStatus::Failed;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = trace_duration_ms(trace, timestamp_ms);
        }
        WorkflowTraceEvent::RunCancelled { error, .. } => {
            trace.status = WorkflowTraceStatus::Cancelled;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = trace_duration_ms(trace, timestamp_ms);
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
            workflow_run_id,
            session_id,
            session,
            items,
            diagnostics,
            error,
            ..
        } => apply_scheduler_snapshot(
            trace,
            workflow_run_id,
            session_id,
            session.as_ref(),
            items,
            diagnostics.as_ref(),
            error.as_deref(),
        ),
        WorkflowTraceEvent::NodeProgress { .. }
        | WorkflowTraceEvent::NodeCompleted { .. }
        | WorkflowTraceEvent::NodeFailed { .. } => {}
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
        WorkflowTraceEvent::NodeProgress { detail, .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.last_progress_detail = detail.clone();
        }
        WorkflowTraceEvent::NodeStream { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.stream_event_count += 1;
        }
        WorkflowTraceEvent::NodeCompleted { .. } => {
            node.status = WorkflowTraceNodeStatus::Completed;
            node.ended_at_ms = Some(timestamp_ms);
            let diagnostics_before = node.timing_diagnostics.len();
            node.duration_ms = node_duration_ms(node, timestamp_ms);
            if node.timing_diagnostics.len() > diagnostics_before {
                node.status = WorkflowTraceNodeStatus::Failed;
            } else {
                node.last_error = None;
            }
        }
        WorkflowTraceEvent::NodeFailed { error, .. } => {
            node.status = WorkflowTraceNodeStatus::Failed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node_duration_ms(node, timestamp_ms);
            node.last_error = Some(error.clone());
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            node.status = WorkflowTraceNodeStatus::Waiting;
            node.ended_at_ms = Some(timestamp_ms);
            let diagnostics_before = node.timing_diagnostics.len();
            node.duration_ms = node_duration_ms(node, timestamp_ms);
            if node.timing_diagnostics.len() > diagnostics_before {
                node.status = WorkflowTraceNodeStatus::Failed;
            }
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
            node.duration_ms = node_duration_ms(node, timestamp_ms);
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
    trace.graph_fingerprint = context.graph_fingerprint.clone();
    trace.status = WorkflowTraceStatus::Running;
    trace.started_at_ms = timestamp_ms;
    trace.ended_at_ms = None;
    trace.duration_ms = None;
    trace.timing_attempt_id = WorkflowTimingAttemptId::generate();
    trace.timing_diagnostics.clear();
    trace.queue = WorkflowTraceQueueMetrics::default();
    trace.runtime = WorkflowTraceRuntimeMetrics::default();
    trace.node_count_at_start = node_count_at_start;
    trace.event_count = 1;
    trace.stream_event_count = 0;
    trace.last_dirty_tasks.clear();
    trace.last_incremental_task_ids.clear();
    trace.last_graph_memory_impact = None;
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
        timing_attempt_id: Some(WorkflowTimingAttemptId::generate()),
        timing_diagnostics: Vec::new(),
        event_count: 0,
        stream_event_count: 0,
        last_error: None,
        last_progress_detail: None,
        timing_expectation: None,
    }
}

fn trace_duration_ms(trace: &mut WorkflowTraceRunState, completed_at_ms: u64) -> Option<u64> {
    let duration = checked_timing_duration_ms(
        &trace.timing_attempt_id,
        trace.started_at_ms,
        completed_at_ms,
    );
    checked_trace_span_duration(
        duration,
        &mut trace.timing_diagnostics,
        Some(&mut trace.last_error),
    )
}

fn node_duration_ms(node: &mut WorkflowTraceNodeRecord, completed_at_ms: u64) -> Option<u64> {
    let started_at_ms = node.started_at_ms?;
    let attempt_id = node
        .timing_attempt_id
        .get_or_insert_with(WorkflowTimingAttemptId::generate);
    let duration = checked_timing_duration_ms(attempt_id, started_at_ms, completed_at_ms);
    checked_trace_span_duration(
        duration,
        &mut node.timing_diagnostics,
        Some(&mut node.last_error),
    )
}

fn checked_trace_span_duration(
    duration: Result<u64, crate::workflow::WorkflowTimingContractError>,
    diagnostics: &mut Vec<WorkflowTimingDiagnostic>,
    last_error: Option<&mut Option<String>>,
) -> Option<u64> {
    match duration {
        Ok(duration_ms) => Some(duration_ms),
        Err(error) => {
            let diagnostic = WorkflowTimingDiagnostic::from_contract_error(
                &error,
                WorkflowTimingAttemptKind::SchedulerTraceSpan,
            )
            .expect("duration underflow must map to a timing diagnostic");
            if let Some(last_error) = last_error {
                last_error.get_or_insert_with(|| diagnostic.message.clone());
            }
            diagnostics.push(diagnostic);
            None
        }
    }
}
