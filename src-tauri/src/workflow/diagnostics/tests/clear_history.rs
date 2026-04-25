use super::*;

#[test]
fn clear_history_preserves_runtime_and_scheduler_snapshots() {
    let store = WorkflowDiagnosticsStore::default();
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-1".to_string()),
        captured_at_ms: 2_000,
        ..Default::default()
    });
    store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
        workflow_id: Some("wf-1".to_string()),
        session_id: Some("exec-1".to_string()),
        captured_at_ms: 2_100,
        ..Default::default()
    });

    let snapshot = store.clear_history();

    assert!(snapshot.runs_by_id.is_empty());
    assert!(snapshot.run_order.is_empty());
    assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("exec-1"));
}
