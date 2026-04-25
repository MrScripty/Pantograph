use pantograph_diagnostics_ledger::{SqliteDiagnosticsLedger, WorkflowTimingExpectationComparison};

use super::*;

#[test]
fn workflow_trace_store_projects_timing_expectation_from_prior_history() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-1", 1_000, 100);
    record_completed_timing_run(&store, "exec-2", 2_000, 200);
    record_completed_timing_run(&store, "exec-3", 3_000, 300);
    let snapshot = record_completed_timing_run(&store, "exec-4", 4_000, 450);

    let trace = snapshot.traces.first().expect("current trace");
    let node = trace.nodes.first().expect("current node");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("node timing expectation");

    assert_eq!(expectation.sample_count, 4);
    assert_eq!(
        expectation.comparison,
        WorkflowTimingExpectationComparison::SlowerThanExpected
    );
    assert_eq!(expectation.median_duration_ms, Some(300));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

#[test]
fn workflow_trace_store_includes_completed_run_in_returned_timing_expectation() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-1", 1_000, 100);
    record_completed_timing_run(&store, "exec-2", 2_000, 200);
    let snapshot = record_completed_timing_run(&store, "exec-3", 3_000, 300);

    let trace = snapshot.traces.first().expect("current trace");
    let node = trace.nodes.first().expect("current node");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("node timing expectation");

    assert_eq!(expectation.sample_count, 3);
    assert_eq!(expectation.median_duration_ms, Some(200));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

fn record_completed_timing_run(
    store: &WorkflowTraceStore,
    execution_id: &str,
    started_at_ms: u64,
    node_duration_ms: u64,
) -> WorkflowTraceSnapshotResponse {
    store.set_execution_metadata(
        execution_id,
        Some("wf-timing".to_string()),
        Some("Timing Workflow".to_string()),
    );
    store.set_execution_graph_context(
        execution_id,
        &WorkflowTraceGraphContext {
            graph_fingerprint: Some("graph-timing".to_string()),
            node_count_at_start: 1,
            node_types_by_id: HashMap::from([("node-1".to_string(), "llm-inference".to_string())]),
        },
    );
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            execution_id: execution_id.to_string(),
            workflow_id: Some("wf-timing".to_string()),
            node_count: 1,
        },
        started_at_ms,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            execution_id: execution_id.to_string(),
            node_id: "node-1".to_string(),
            node_type: None,
        },
        started_at_ms + 10,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeCompleted {
            execution_id: execution_id.to_string(),
            node_id: "node-1".to_string(),
        },
        started_at_ms + 10 + node_duration_ms,
    );
    store.record_event(
        &WorkflowTraceEvent::RunCompleted {
            execution_id: execution_id.to_string(),
            workflow_id: Some("wf-timing".to_string()),
        },
        started_at_ms + 20 + node_duration_ms,
    )
}
