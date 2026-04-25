use super::*;

#[test]
fn diagnostics_projection_exposes_backend_timing_expectation() {
    let store = WorkflowDiagnosticsStore::with_default_timing_ledger(
        pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory()
            .expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-1", 1_000, 100);
    record_completed_timing_run(&store, "exec-2", 2_000, 200);
    record_completed_timing_run(&store, "exec-3", 3_000, 300);
    let projection = record_completed_timing_run(&store, "exec-4", 4_000, 450);

    let run = projection.runs_by_id.get("exec-4").expect("run trace");
    let node = run.nodes.get("llm-1").expect("node trace");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("timing expectation");

    assert_eq!(expectation.sample_count, 4);
    assert_eq!(
        expectation.comparison,
        pantograph_workflow_service::WorkflowTimingExpectationComparison::SlowerThanExpected
    );
    assert_eq!(expectation.median_duration_ms, Some(300));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

#[test]
fn diagnostics_projection_serializes_timing_expectation_as_camel_case() {
    let store = WorkflowDiagnosticsStore::with_default_timing_ledger(
        pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory()
            .expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-1", 1_000, 100);
    record_completed_timing_run(&store, "exec-2", 2_000, 200);
    record_completed_timing_run(&store, "exec-3", 3_000, 300);
    let projection = record_completed_timing_run(&store, "exec-4", 4_000, 450);
    let run = projection.runs_by_id.get("exec-4").expect("run trace");
    let node = run.nodes.get("llm-1").expect("node trace");
    let value = serde_json::to_value(node).expect("node trace serializes");
    let expectation = value
        .get("timingExpectation")
        .expect("timing expectation field");

    assert_eq!(
        expectation
            .get("comparison")
            .and_then(|value| value.as_str()),
        Some("slower_than_expected")
    );
    assert_eq!(
        expectation
            .get("sampleCount")
            .and_then(|value| value.as_u64()),
        Some(4)
    );
    assert_eq!(
        expectation
            .get("medianDurationMs")
            .and_then(|value| value.as_u64()),
        Some(300)
    );
    assert!(expectation.get("sample_count").is_none());
    assert!(expectation.get("median_duration_ms").is_none());
}

#[test]
fn workflow_timing_history_reads_prior_runs_without_active_trace() {
    let store = WorkflowDiagnosticsStore::with_default_timing_ledger(
        pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory()
            .expect("ledger opens"),
    );

    record_completed_timing_run(&store, "exec-1", 1_000, 100);
    record_completed_timing_run(&store, "exec-2", 2_000, 200);
    record_completed_timing_run(&store, "exec-3", 3_000, 300);

    let history = store.workflow_timing_history(
        "wf-timing".to_string(),
        Some("Timing Workflow".to_string()),
        &sample_graph(),
    );
    let node = history.nodes.get("llm-1").expect("node history");
    let expectation = node
        .timing_expectation
        .as_ref()
        .expect("node timing expectation");

    assert_eq!(history.workflow_id, "wf-timing");
    assert_eq!(history.workflow_name.as_deref(), Some("Timing Workflow"));
    assert_eq!(history.graph_fingerprint.as_deref(), Some("graph-123"));
    assert_eq!(expectation.sample_count, 3);
    assert_eq!(
        expectation.comparison,
        pantograph_workflow_service::WorkflowTimingExpectationComparison::NoCurrentDuration
    );
    assert_eq!(expectation.current_duration_ms, None);
    assert_eq!(expectation.median_duration_ms, Some(200));
    assert_eq!(expectation.typical_min_duration_ms, Some(200));
    assert_eq!(expectation.typical_max_duration_ms, Some(300));
}

fn record_completed_timing_run(
    store: &WorkflowDiagnosticsStore,
    execution_id: &str,
    started_at_ms: u64,
    node_duration_ms: u64,
) -> WorkflowDiagnosticsProjection {
    store.set_execution_metadata(
        execution_id,
        Some("wf-timing".to_string()),
        Some("Timing Workflow".to_string()),
    );
    store.set_execution_graph(execution_id, &sample_graph());
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-timing".to_string(),
            node_count: 1,
            execution_id: execution_id.to_string(),
        },
        started_at_ms,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeStarted {
            node_id: "llm-1".to_string(),
            node_type: "llm-inference".to_string(),
            execution_id: execution_id.to_string(),
        },
        started_at_ms + 10,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeCompleted {
            node_id: "llm-1".to_string(),
            outputs: std::collections::HashMap::new(),
            execution_id: execution_id.to_string(),
        },
        started_at_ms + 10 + node_duration_ms,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-timing".to_string(),
            outputs: std::collections::HashMap::new(),
            execution_id: execution_id.to_string(),
        },
        started_at_ms + 20 + node_duration_ms,
    )
}
