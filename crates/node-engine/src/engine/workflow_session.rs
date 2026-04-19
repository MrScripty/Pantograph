use super::{
    NodeMemorySnapshot, WorkflowExecutor, WorkflowSessionCheckpointSummary,
    WorkflowSessionResidencyState,
};

pub(super) async fn workflow_session_residency(
    executor: &WorkflowExecutor,
) -> WorkflowSessionResidencyState {
    executor.session_state.residency().await
}

pub(super) async fn bind_workflow_session(
    executor: &WorkflowExecutor,
    workflow_session_id: impl Into<String>,
) {
    executor
        .session_state
        .bind_workflow_session(workflow_session_id.into())
        .await;
}

pub(super) async fn bound_workflow_session_id(executor: &WorkflowExecutor) -> Option<String> {
    executor.session_state.bound_workflow_session_id().await
}

pub(super) async fn clear_bound_workflow_session(executor: &WorkflowExecutor) {
    executor.session_state.clear_bound_workflow_session().await;
}

pub(super) async fn set_workflow_session_residency(
    executor: &WorkflowExecutor,
    state: WorkflowSessionResidencyState,
) {
    executor.session_state.set_residency(state).await;
}

pub(super) async fn workflow_session_checkpoint_summary(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
) -> WorkflowSessionCheckpointSummary {
    let graph_revision = executor.graph.read().await.id.clone();
    executor
        .session_state
        .checkpoint_summary(workflow_session_id, &graph_revision)
        .await
}

pub(super) async fn workflow_session_node_memory_snapshots(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
) -> Vec<NodeMemorySnapshot> {
    executor
        .session_state
        .node_memory_snapshots(workflow_session_id)
        .await
}

pub(super) async fn record_workflow_session_node_memory(
    executor: &WorkflowExecutor,
    snapshot: NodeMemorySnapshot,
) {
    executor.session_state.record_node_memory(snapshot).await;
}

pub(super) async fn clear_workflow_session_node_memory(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
) {
    executor
        .session_state
        .clear_node_memory(workflow_session_id)
        .await;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::events::NullEventSink;

    use super::{
        NodeMemorySnapshot, WorkflowExecutor, WorkflowSessionResidencyState, bind_workflow_session,
        bound_workflow_session_id, clear_bound_workflow_session,
        clear_workflow_session_node_memory, record_workflow_session_node_memory,
        set_workflow_session_residency, workflow_session_checkpoint_summary,
        workflow_session_node_memory_snapshots, workflow_session_residency,
    };

    #[tokio::test]
    async fn executor_workflow_session_helpers_preserve_graph_revision_and_residency() {
        let executor = WorkflowExecutor::new(
            "exec-1",
            crate::types::WorkflowGraph::new("graph-1", "Graph"),
            Arc::new(NullEventSink),
        );

        assert_eq!(
            workflow_session_residency(&executor).await,
            WorkflowSessionResidencyState::Active
        );

        set_workflow_session_residency(
            &executor,
            WorkflowSessionResidencyState::CheckpointedButUnloaded,
        )
        .await;

        let summary = workflow_session_checkpoint_summary(&executor, "session-1").await;
        assert_eq!(summary.session_id, "session-1");
        assert_eq!(summary.graph_revision, "graph-1");
        assert_eq!(
            summary.residency,
            WorkflowSessionResidencyState::CheckpointedButUnloaded
        );
        assert!(!summary.checkpoint_available);
    }

    #[tokio::test]
    async fn executor_workflow_session_helpers_round_trip_node_memory_snapshots() {
        let executor = WorkflowExecutor::new(
            "exec-1",
            crate::types::WorkflowGraph::new("graph-1", "Graph"),
            Arc::new(NullEventSink),
        );

        record_workflow_session_node_memory(
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
                inspection_metadata: Some(serde_json::json!({ "label": "Alpha" })),
            },
        )
        .await;

        let snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].identity.node_id, "node-a");

        clear_workflow_session_node_memory(&executor, "session-1").await;
        assert!(
            workflow_session_node_memory_snapshots(&executor, "session-1")
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn executor_workflow_session_helpers_bind_and_clear_workflow_session_identity() {
        let executor = WorkflowExecutor::new(
            "exec-1",
            crate::types::WorkflowGraph::new("graph-1", "Graph"),
            Arc::new(NullEventSink),
        );

        assert_eq!(bound_workflow_session_id(&executor).await, None);
        bind_workflow_session(&executor, "session-1").await;
        assert_eq!(
            bound_workflow_session_id(&executor).await,
            Some("session-1".to_string())
        );
        clear_bound_workflow_session(&executor).await;
        assert_eq!(bound_workflow_session_id(&executor).await, None);
    }
}
