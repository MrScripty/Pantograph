use super::*;

#[tokio::test]
async fn executor_workflow_execution_session_helpers_preserve_graph_revision_and_residency() {
    let executor = WorkflowExecutor::new(
        "exec-1",
        crate::types::WorkflowGraph::new("graph-1", "Graph"),
        Arc::new(NullEventSink),
    );

    assert_eq!(
        workflow_execution_session_residency(&executor).await,
        WorkflowExecutionSessionResidencyState::Active
    );

    set_workflow_execution_session_residency(
        &executor,
        WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded,
    )
    .await;

    let summary = workflow_execution_session_checkpoint_summary(&executor, "session-1").await;
    assert_eq!(summary.session_id, "session-1");
    assert_eq!(summary.graph_revision, "graph-1");
    assert_eq!(
        summary.residency,
        WorkflowExecutionSessionResidencyState::CheckpointedButUnloaded
    );
    assert!(!summary.checkpoint_available);

    mark_workflow_execution_session_checkpoint_available(&executor, "session-1").await;
    let checkpointed_summary =
        workflow_execution_session_checkpoint_summary(&executor, "session-1").await;
    assert!(checkpointed_summary.checkpoint_available);
    assert!(checkpointed_summary.checkpointed_at_ms.is_some());

    clear_workflow_execution_session_checkpoint(&executor, "session-1").await;
    let cleared_summary =
        workflow_execution_session_checkpoint_summary(&executor, "session-1").await;
    assert!(!cleared_summary.checkpoint_available);
}

#[tokio::test]
async fn executor_workflow_execution_session_helpers_round_trip_node_memory_snapshots() {
    let executor = WorkflowExecutor::new(
        "exec-1",
        crate::types::WorkflowGraph::new("graph-1", "Graph"),
        Arc::new(NullEventSink),
    );

    record_workflow_execution_session_node_memory(
        &executor,
        NodeMemorySnapshot {
            identity: crate::engine::NodeMemoryIdentity {
                session_id: "session-1".to_string(),
                node_id: "node-a".to_string(),
                node_type: "text-input".to_string(),
                schema_version: Some("v1".to_string()),
            },
            status: crate::engine::NodeMemoryStatus::Ready,
            input_fingerprint: Some("fp-a".to_string()),
            output_snapshot: Some(serde_json::json!({ "text": "alpha" })),
            private_state: None,
            indirect_state_reference: None,
            inspection_metadata: Some(serde_json::json!({ "label": "Alpha" })),
        },
    )
    .await;

    let snapshots = workflow_execution_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].identity.node_id, "node-a");

    clear_workflow_execution_session_node_memory(&executor, "session-1").await;
    assert!(
        workflow_execution_session_node_memory_snapshots(&executor, "session-1")
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn executor_workflow_execution_session_helpers_bind_and_clear_workflow_execution_session_identity(
) {
    let executor = WorkflowExecutor::new(
        "exec-1",
        crate::types::WorkflowGraph::new("graph-1", "Graph"),
        Arc::new(NullEventSink),
    );

    assert_eq!(bound_workflow_execution_session_id(&executor).await, None);
    bind_workflow_execution_session(&executor, "session-1").await;
    assert_eq!(
        bound_workflow_execution_session_id(&executor).await,
        Some("session-1".to_string())
    );
    clear_bound_workflow_execution_session(&executor).await;
    assert_eq!(bound_workflow_execution_session_id(&executor).await, None);
}

#[tokio::test]
async fn sync_bound_session_node_memory_from_cache_projects_all_cached_nodes() {
    let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
    bind_workflow_execution_session(&executor, "session-1").await;

    executor
        .demand(&"c".to_string(), &SnapshotTaskExecutor)
        .await
        .expect("demand graph");
    sync_bound_session_node_memory_from_cache(&executor).await;

    let snapshots = workflow_execution_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(snapshots.len(), 3);
    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.identity.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(snapshots[0].identity.schema_version.as_deref(), Some("v1"));
    assert_eq!(
        snapshots[2].output_snapshot,
        Some(serde_json::json!({
            "out": "c",
            "value": "c"
        }))
    );
    assert_eq!(
        snapshots[1].input_fingerprint.as_deref(),
        Some("{\"_data\":{},\"in\":\"a\"}")
    );
    assert_eq!(
        snapshots[1].inspection_metadata,
        Some(serde_json::json!({
            "projection_source": "demand_engine_cache",
            "cache_version": 1,
            "input_snapshot": {
                "_data": {},
                "in": "a"
            }
        }))
    );
}

#[tokio::test]
async fn repeated_runs_replace_node_memory_for_the_same_workflow_execution_session() {
    let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
    let task_executor = SequencedSnapshotTaskExecutor::new();
    bind_workflow_execution_session(&executor, "session-1").await;

    executor
        .demand(&"c".to_string(), &task_executor)
        .await
        .expect("run first demand");
    let first_snapshots =
        workflow_execution_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(first_snapshots.len(), 3);
    assert_eq!(
        first_snapshots[2].output_snapshot,
        Some(serde_json::json!({
            "out": {
                "task": "c",
                "sequence": 3,
            },
            "value": {
                "task": "c",
                "sequence": 3,
            }
        }))
    );

    executor.mark_modified(&"a".to_string()).await;
    executor
        .demand(&"c".to_string(), &task_executor)
        .await
        .expect("run second demand");

    let second_snapshots =
        workflow_execution_session_node_memory_snapshots(&executor, "session-1").await;
    assert_eq!(second_snapshots.len(), 3);
    assert_eq!(
        second_snapshots
            .iter()
            .map(|snapshot| snapshot.identity.node_id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
    assert_eq!(
        second_snapshots[2].output_snapshot,
        Some(serde_json::json!({
            "out": {
                "task": "c",
                "sequence": 6,
            },
            "value": {
                "task": "c",
                "sequence": 6,
            }
        }))
    );
    assert_ne!(
        first_snapshots[2].input_fingerprint,
        second_snapshots[2].input_fingerprint
    );
}
