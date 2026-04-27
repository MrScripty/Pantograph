use pantograph_diagnostics_ledger::{
    DiagnosticEventKind, DiagnosticsLedgerRepository, NodeExecutionProjectionStatus,
    NodeStatusProjectionQuery, SqliteDiagnosticsLedger, WorkflowRunSummaryQuery,
    WorkflowRunSummaryStatus, WorkflowTimingExpectationComparison,
};
use pantograph_runtime_attribution::WorkflowRunId;

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

#[test]
fn graph_timing_expectations_reads_prior_workflow_id_history() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run_with_workflow(&store, "exec-name-1", "saved-workflow", 1_000, 100);
    record_completed_timing_run_with_workflow(&store, "exec-name-2", "saved-workflow", 2_000, 200);
    record_completed_timing_run_with_workflow(&store, "exec-name-3", "saved-workflow", 3_000, 300);

    let history =
        store.graph_timing_expectations("saved-workflow".to_string(), &timing_graph_context());
    let node = history.nodes.first().expect("node timing history");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("workflow id timing expectation");

    assert_eq!(history.workflow_id, "saved-workflow");
    assert_eq!(expectation.sample_count, 3);
    assert_eq!(expectation.median_duration_ms, Some(200));
}

#[test]
fn graph_timing_expectations_recovers_unique_legacy_id_for_same_graph() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run_with_workflow(&store, "exec-name-1", "Display Name", 1_000, 100);
    record_completed_timing_run_with_workflow(&store, "exec-name-2", "Display Name", 2_000, 200);
    record_completed_timing_run_with_workflow(&store, "exec-name-3", "Display Name", 3_000, 300);

    let history =
        store.graph_timing_expectations("saved-workflow-id".to_string(), &timing_graph_context());
    let node = history.nodes.first().expect("node timing history");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("legacy id timing expectation");

    assert_eq!(history.workflow_id, "saved-workflow-id");
    assert_eq!(expectation.sample_count, 3);
    assert_eq!(expectation.median_duration_ms, Some(200));
}

#[test]
fn graph_timing_expectations_ignores_ambiguous_legacy_ids_for_same_graph() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run_with_workflow(&store, "exec-name-1", "Display Name", 1_000, 100);
    record_completed_timing_run_with_workflow(&store, "exec-alias-1", "Other Name", 2_000, 200);

    let history =
        store.graph_timing_expectations("saved-workflow-id".to_string(), &timing_graph_context());
    let node = history.nodes.first().expect("node timing history");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("empty timing expectation");

    assert_eq!(history.workflow_id, "saved-workflow-id");
    assert_eq!(expectation.sample_count, 0);
}

#[test]
fn workflow_trace_store_persists_run_summary_history() {
    let store = WorkflowTraceStore::with_timing_ledger(
        10,
        SqliteDiagnosticsLedger::open_in_memory().expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-summary", 1_000, 100);

    let history = store
        .workflow_run_summaries(WorkflowRunSummaryQuery {
            workflow_id: Some("wf-timing".to_string()),
            workflow_run_id: None,
            limit: 10,
        })
        .expect("run summary history loads");
    let run = history.runs.first().expect("run summary");

    assert_eq!(run.workflow_run_id, "exec-summary");
    assert_eq!(run.workflow_id, "wf-timing");
    assert_eq!(run.status, WorkflowRunSummaryStatus::Completed);
    assert_eq!(run.duration_ms, Some(120));
    assert_eq!(run.node_count_at_start, 1);
    assert!(run.event_count >= 3);
}

#[test]
fn workflow_trace_store_records_bounded_node_status_events() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let ledger_path = temp_dir.path().join("diagnostics.sqlite");

    {
        let store = WorkflowTraceStore::with_timing_ledger(
            10,
            SqliteDiagnosticsLedger::open(&ledger_path).expect("ledger opens"),
        );
        store.set_execution_metadata("run-node-status", Some("workflow-node-status".to_string()));
        store.set_execution_graph_context(
            "run-node-status",
            &WorkflowTraceGraphContext {
                graph_fingerprint: Some("graph-node-status".to_string()),
                node_count_at_start: 2,
                node_types_by_id: HashMap::from([
                    ("node-1".to_string(), "image-generator".to_string()),
                    ("node-2".to_string(), "approval-gate".to_string()),
                ]),
            },
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                workflow_run_id: "run-node-status".to_string(),
                workflow_id: Some("workflow-node-status".to_string()),
                node_count: 2,
            },
            1_000,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            1_010,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeProgress {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-1".to_string(),
                detail: None,
            },
            1_020,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStream {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-1".to_string(),
            },
            1_030,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeCompleted {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-1".to_string(),
            },
            1_110,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeCompleted {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-1".to_string(),
            },
            1_115,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                workflow_run_id: "run-node-status".to_string(),
                node_id: "node-2".to_string(),
                node_type: None,
            },
            1_120,
        );
        store.record_event(
            &WorkflowTraceEvent::WaitingForInput {
                workflow_run_id: "run-node-status".to_string(),
                workflow_id: Some("workflow-node-status".to_string()),
                node_id: "node-2".to_string(),
            },
            1_130,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCancelled {
                workflow_run_id: "run-node-status".to_string(),
                workflow_id: Some("workflow-node-status".to_string()),
                error: "cancelled by operator".to_string(),
            },
            1_140,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCancelled {
                workflow_run_id: "run-node-status".to_string(),
                workflow_id: Some("workflow-node-status".to_string()),
                error: "cancelled by operator".to_string(),
            },
            1_150,
        );
    }

    let mut ledger = SqliteDiagnosticsLedger::open(&ledger_path).expect("ledger reopens");
    let events = ledger
        .diagnostic_events_after(0, 20)
        .expect("diagnostic events load");
    let node_status_events = events
        .iter()
        .filter(|event| event.event_kind == DiagnosticEventKind::NodeExecutionStatus)
        .collect::<Vec<_>>();
    assert_eq!(node_status_events.len(), 5);

    ledger
        .drain_node_status_projection(20)
        .expect("node status projection drains");
    let records = ledger
        .query_node_status_projection(NodeStatusProjectionQuery {
            workflow_run_id: Some(WorkflowRunId::try_from("run-node-status".to_string()).unwrap()),
            node_id: None,
            status: None,
            after_event_seq: None,
            limit: 10,
        })
        .expect("node status projection loads");
    assert_eq!(records.len(), 2);

    let completed_node = records
        .iter()
        .find(|record| record.node_id == "node-1")
        .expect("completed node status");
    assert_eq!(
        completed_node.status,
        NodeExecutionProjectionStatus::Completed
    );
    assert_eq!(completed_node.node_type.as_deref(), Some("image-generator"));
    assert_eq!(completed_node.started_at_ms, Some(1_010));
    assert_eq!(completed_node.completed_at_ms, Some(1_110));
    assert_eq!(completed_node.duration_ms, Some(100));

    let cancelled_node = records
        .iter()
        .find(|record| record.node_id == "node-2")
        .expect("cancelled node status");
    assert_eq!(
        cancelled_node.status,
        NodeExecutionProjectionStatus::Cancelled
    );
    assert_eq!(cancelled_node.node_type.as_deref(), Some("approval-gate"));
    assert_eq!(cancelled_node.started_at_ms, Some(1_120));
    assert_eq!(cancelled_node.completed_at_ms, Some(1_140));
    assert_eq!(cancelled_node.duration_ms, Some(20));
    assert_eq!(
        cancelled_node.error.as_deref(),
        Some("cancelled by operator")
    );
}

fn record_completed_timing_run(
    store: &WorkflowTraceStore,
    workflow_run_id: &str,
    started_at_ms: u64,
    node_duration_ms: u64,
) -> WorkflowTraceSnapshotResponse {
    record_completed_timing_run_with_workflow(
        store,
        workflow_run_id,
        "wf-timing",
        started_at_ms,
        node_duration_ms,
    )
}

fn record_completed_timing_run_with_workflow(
    store: &WorkflowTraceStore,
    workflow_run_id: &str,
    workflow_id: &str,
    started_at_ms: u64,
    node_duration_ms: u64,
) -> WorkflowTraceSnapshotResponse {
    store.set_execution_metadata(workflow_run_id, Some(workflow_id.to_string()));
    store.set_execution_graph_context(workflow_run_id, &timing_graph_context());
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: workflow_run_id.to_string(),
            workflow_id: Some(workflow_id.to_string()),
            node_count: 1,
        },
        started_at_ms,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: workflow_run_id.to_string(),
            node_id: "node-1".to_string(),
            node_type: None,
        },
        started_at_ms + 10,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeCompleted {
            workflow_run_id: workflow_run_id.to_string(),
            node_id: "node-1".to_string(),
        },
        started_at_ms + 10 + node_duration_ms,
    );
    store.record_event(
        &WorkflowTraceEvent::RunCompleted {
            workflow_run_id: workflow_run_id.to_string(),
            workflow_id: Some(workflow_id.to_string()),
        },
        started_at_ms + 20 + node_duration_ms,
    )
}

fn timing_graph_context() -> WorkflowTraceGraphContext {
    WorkflowTraceGraphContext {
        graph_fingerprint: Some("graph-timing".to_string()),
        node_count_at_start: 1,
        node_types_by_id: HashMap::from([("node-1".to_string(), "llm-inference".to_string())]),
    }
}
