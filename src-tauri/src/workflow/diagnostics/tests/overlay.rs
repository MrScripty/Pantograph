use super::*;

#[test]
fn node_progress_detail_is_exposed_in_diagnostics_snapshot() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            workflow_run_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeProgress {
            node_id: "llm-1".to_string(),
            progress: 0.0,
            message: Some("kv cache restored".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::RestoreInput,
                    outcome: node_engine::KvCacheEventOutcome::Hit,
                    cache_id: Some("cache-1".to_string()),
                    backend_key: Some("llamacpp".to_string()),
                    reuse_source: Some("llamacpp_slot".to_string()),
                    token_count: Some(48),
                    reason: Some("restored_input_handle".to_string()),
                },
            )),
            workflow_run_id: "exec-1".to_string(),
        },
        1_020,
    );

    let snapshot = store.snapshot();
    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    let node = run.nodes.get("llm-1").expect("node trace");
    match node.last_progress_detail.as_ref() {
        Some(node_engine::TaskProgressDetail::KvCache(detail)) => {
            assert_eq!(detail.outcome, node_engine::KvCacheEventOutcome::Hit);
            assert_eq!(detail.cache_id.as_deref(), Some("cache-1"));
        }
        other => panic!("unexpected progress detail: {other:?}"),
    }
    assert_eq!(node.last_progress, None);
}
