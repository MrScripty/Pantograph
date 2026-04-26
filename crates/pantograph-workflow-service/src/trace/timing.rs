use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, SqliteDiagnosticsLedger, WorkflowRunSummaryRecord,
    WorkflowRunSummaryStatus, WorkflowTimingExpectationQuery, WorkflowTimingObservation,
    WorkflowTimingObservationScope, WorkflowTimingObservationStatus,
};

use super::store::WorkflowTraceState;
use super::types::{
    WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceGraphTimingExpectations,
    WorkflowTraceNodeRecord, WorkflowTraceNodeStatus, WorkflowTraceNodeTimingExpectation,
    WorkflowTraceSnapshotResponse, WorkflowTraceStatus, WorkflowTraceSummary,
};

pub(super) fn terminal_timing_observations(
    state: &WorkflowTraceState,
    event: &WorkflowTraceEvent,
    recorded_at_ms: u64,
) -> Vec<WorkflowTimingObservation> {
    let Some(trace) = state.traces_by_id.get(event.workflow_run_id()) else {
        return Vec::new();
    };
    match event {
        WorkflowTraceEvent::RunCompleted { .. }
        | WorkflowTraceEvent::RunFailed { .. }
        | WorkflowTraceEvent::RunCancelled { .. } => {
            let mut observations: Vec<_> = run_timing_observation(trace, recorded_at_ms)
                .into_iter()
                .collect();
            observations.extend(trace.nodes_by_id.iter().filter_map(|(node_id, node)| {
                node_timing_observation(
                    trace,
                    node_id,
                    event_status_for_node(node.status),
                    recorded_at_ms,
                )
            }));
            observations
        }
        _ => Vec::new(),
    }
}

pub(super) fn run_summary_record(
    trace: &super::store::WorkflowTraceRunState,
    recorded_at_ms: u64,
) -> Option<WorkflowRunSummaryRecord> {
    Some(WorkflowRunSummaryRecord {
        workflow_run_id: trace.workflow_run_id.clone(),
        workflow_id: trace.workflow_id.clone()?,
        session_id: trace.session_id.clone(),
        graph_fingerprint: trace.graph_fingerprint.clone(),
        status: run_summary_status(trace.status),
        started_at_ms: trace.started_at_ms as i64,
        ended_at_ms: trace.ended_at_ms.map(|ended_at_ms| ended_at_ms as i64),
        duration_ms: trace.duration_ms,
        node_count_at_start: trace.node_count_at_start,
        event_count: trace.event_count,
        last_error: trace.last_error.clone(),
        recorded_at_ms: recorded_at_ms as i64,
    })
}

pub(super) fn enrich_snapshot_timing(
    mut snapshot: WorkflowTraceSnapshotResponse,
    ledger: &SqliteDiagnosticsLedger,
    now_ms: u64,
) -> WorkflowTraceSnapshotResponse {
    for trace in &mut snapshot.traces {
        trace.timing_expectation = trace_timing_expectation(trace, ledger, now_ms);
        let run_runtime_id = trace.runtime.runtime_id.clone();
        let workflow_id = trace.workflow_id.clone();
        let graph_fingerprint = trace.graph_fingerprint.clone();
        for node in &mut trace.nodes {
            node.timing_expectation = node_timing_expectation_projection(
                workflow_id.as_deref(),
                graph_fingerprint.as_deref(),
                node,
                run_runtime_id.as_deref(),
                ledger,
                now_ms,
            );
        }
    }
    snapshot
}

pub(super) fn graph_timing_expectations(
    workflow_id: String,
    graph_context: &WorkflowTraceGraphContext,
    ledger: Option<&SqliteDiagnosticsLedger>,
) -> WorkflowTraceGraphTimingExpectations {
    let query_workflow_id = resolve_timing_query_workflow_id(&workflow_id, graph_context, ledger);
    let timing_expectation =
        graph_context
            .graph_fingerprint
            .as_ref()
            .and_then(|graph_fingerprint| {
                ledger?
                    .timing_expectation(WorkflowTimingExpectationQuery {
                        scope: WorkflowTimingObservationScope::Run,
                        workflow_id: query_workflow_id.clone(),
                        graph_fingerprint: graph_fingerprint.clone(),
                        node_id: None,
                        node_type: None,
                        runtime_id: None,
                        current_duration_ms: None,
                        current_duration_is_complete: false,
                    })
                    .ok()
            });
    let mut nodes: Vec<_> = graph_context
        .node_types_by_id
        .iter()
        .map(|(node_id, node_type)| WorkflowTraceNodeTimingExpectation {
            node_id: node_id.clone(),
            node_type: Some(node_type.clone()),
            timing_expectation: graph_context.graph_fingerprint.as_ref().and_then(
                |graph_fingerprint| {
                    ledger?
                        .timing_expectation(WorkflowTimingExpectationQuery {
                            scope: WorkflowTimingObservationScope::Node,
                            workflow_id: query_workflow_id.clone(),
                            graph_fingerprint: graph_fingerprint.clone(),
                            node_id: Some(node_id.clone()),
                            node_type: Some(node_type.clone()),
                            runtime_id: None,
                            current_duration_ms: None,
                            current_duration_is_complete: false,
                        })
                        .ok()
                },
            ),
        })
        .collect();
    nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    WorkflowTraceGraphTimingExpectations {
        workflow_id,
        graph_fingerprint: graph_context.graph_fingerprint.clone(),
        timing_expectation,
        nodes,
    }
}

fn resolve_timing_query_workflow_id(
    workflow_id: &str,
    graph_context: &WorkflowTraceGraphContext,
    ledger: Option<&SqliteDiagnosticsLedger>,
) -> String {
    let Some(ledger) = ledger else {
        return workflow_id.to_string();
    };
    let Some(graph_fingerprint) = graph_context.graph_fingerprint.as_deref() else {
        return workflow_id.to_string();
    };
    let Ok(workflow_ids) = ledger.workflow_ids_for_timing_graph_fingerprint(graph_fingerprint)
    else {
        return workflow_id.to_string();
    };
    if workflow_ids.iter().any(|stored| stored == workflow_id) {
        return workflow_id.to_string();
    }
    if let [legacy_workflow_id] = workflow_ids.as_slice() {
        return legacy_workflow_id.clone();
    }
    workflow_id.to_string()
}

fn run_timing_observation(
    trace: &super::store::WorkflowTraceRunState,
    recorded_at_ms: u64,
) -> Option<WorkflowTimingObservation> {
    let workflow_id = trace.workflow_id.clone()?;
    let graph_fingerprint = trace.graph_fingerprint.clone()?;
    let ended_at_ms = trace.ended_at_ms?;
    let duration_ms = trace.duration_ms?;
    Some(WorkflowTimingObservation {
        observation_key: format!("run:{}", trace.workflow_run_id),
        scope: WorkflowTimingObservationScope::Run,
        workflow_run_id: trace.workflow_run_id.clone(),
        workflow_id,
        graph_fingerprint,
        node_id: None,
        node_type: None,
        runtime_id: trace.runtime.runtime_id.clone(),
        status: event_status_for_run(trace.status),
        started_at_ms: trace.started_at_ms as i64,
        ended_at_ms: ended_at_ms as i64,
        duration_ms,
        recorded_at_ms: recorded_at_ms as i64,
    })
}

fn node_timing_observation(
    trace: &super::store::WorkflowTraceRunState,
    node_id: &str,
    status: WorkflowTimingObservationStatus,
    recorded_at_ms: u64,
) -> Option<WorkflowTimingObservation> {
    let workflow_id = trace.workflow_id.clone()?;
    let graph_fingerprint = trace.graph_fingerprint.clone()?;
    let node = trace.nodes_by_id.get(node_id)?;
    let started_at_ms = node.started_at_ms?;
    let ended_at_ms = node.ended_at_ms?;
    let duration_ms = node.duration_ms?;
    Some(WorkflowTimingObservation {
        observation_key: format!("node:{}:{node_id}", trace.workflow_run_id),
        scope: WorkflowTimingObservationScope::Node,
        workflow_run_id: trace.workflow_run_id.clone(),
        workflow_id,
        graph_fingerprint,
        node_id: Some(node_id.to_string()),
        node_type: node.node_type.clone(),
        runtime_id: trace.runtime.runtime_id.clone(),
        status,
        started_at_ms: started_at_ms as i64,
        ended_at_ms: ended_at_ms as i64,
        duration_ms,
        recorded_at_ms: recorded_at_ms as i64,
    })
}

fn trace_timing_expectation(
    trace: &WorkflowTraceSummary,
    ledger: &SqliteDiagnosticsLedger,
    now_ms: u64,
) -> Option<pantograph_diagnostics_ledger::WorkflowTimingExpectation> {
    let workflow_id = trace.workflow_id.clone()?;
    let graph_fingerprint = trace.graph_fingerprint.clone()?;
    ledger
        .timing_expectation(WorkflowTimingExpectationQuery {
            scope: WorkflowTimingObservationScope::Run,
            workflow_id,
            graph_fingerprint,
            node_id: None,
            node_type: None,
            runtime_id: trace.runtime.runtime_id.clone(),
            current_duration_ms: trace_current_duration(trace, now_ms),
            current_duration_is_complete: trace.ended_at_ms.is_some(),
        })
        .ok()
}

fn node_timing_expectation_projection(
    workflow_id: Option<&str>,
    graph_fingerprint: Option<&str>,
    node: &WorkflowTraceNodeRecord,
    runtime_id: Option<&str>,
    ledger: &SqliteDiagnosticsLedger,
    now_ms: u64,
) -> Option<pantograph_diagnostics_ledger::WorkflowTimingExpectation> {
    let workflow_id = workflow_id?.to_string();
    let graph_fingerprint = graph_fingerprint?.to_string();
    ledger
        .timing_expectation(WorkflowTimingExpectationQuery {
            scope: WorkflowTimingObservationScope::Node,
            workflow_id,
            graph_fingerprint,
            node_id: Some(node.node_id.clone()),
            node_type: node.node_type.clone(),
            runtime_id: runtime_id.map(str::to_string),
            current_duration_ms: node_current_duration(node, now_ms),
            current_duration_is_complete: node.ended_at_ms.is_some(),
        })
        .ok()
}

fn trace_current_duration(trace: &WorkflowTraceSummary, now_ms: u64) -> Option<u64> {
    trace
        .duration_ms
        .or_else(|| now_ms.checked_sub(trace.started_at_ms))
}

fn node_current_duration(node: &WorkflowTraceNodeRecord, now_ms: u64) -> Option<u64> {
    node.duration_ms.or_else(|| {
        node.started_at_ms
            .and_then(|started_at_ms| now_ms.checked_sub(started_at_ms))
    })
}

fn event_status_for_run(status: WorkflowTraceStatus) -> WorkflowTimingObservationStatus {
    match status {
        WorkflowTraceStatus::Completed => WorkflowTimingObservationStatus::Completed,
        WorkflowTraceStatus::Cancelled => WorkflowTimingObservationStatus::Cancelled,
        WorkflowTraceStatus::Queued
        | WorkflowTraceStatus::Running
        | WorkflowTraceStatus::Waiting => WorkflowTimingObservationStatus::Failed,
        WorkflowTraceStatus::Failed => WorkflowTimingObservationStatus::Failed,
    }
}

fn run_summary_status(status: WorkflowTraceStatus) -> WorkflowRunSummaryStatus {
    match status {
        WorkflowTraceStatus::Queued => WorkflowRunSummaryStatus::Queued,
        WorkflowTraceStatus::Running => WorkflowRunSummaryStatus::Running,
        WorkflowTraceStatus::Waiting => WorkflowRunSummaryStatus::Waiting,
        WorkflowTraceStatus::Completed => WorkflowRunSummaryStatus::Completed,
        WorkflowTraceStatus::Failed => WorkflowRunSummaryStatus::Failed,
        WorkflowTraceStatus::Cancelled => WorkflowRunSummaryStatus::Cancelled,
    }
}

fn event_status_for_node(status: WorkflowTraceNodeStatus) -> WorkflowTimingObservationStatus {
    match status {
        WorkflowTraceNodeStatus::Completed => WorkflowTimingObservationStatus::Completed,
        WorkflowTraceNodeStatus::Cancelled => WorkflowTimingObservationStatus::Cancelled,
        WorkflowTraceNodeStatus::Pending
        | WorkflowTraceNodeStatus::Running
        | WorkflowTraceNodeStatus::Waiting
        | WorkflowTraceNodeStatus::Failed => WorkflowTimingObservationStatus::Failed,
    }
}
