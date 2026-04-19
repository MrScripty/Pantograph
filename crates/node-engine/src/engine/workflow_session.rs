use super::{WorkflowExecutor, WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState};

pub(super) async fn workflow_session_residency(
    executor: &WorkflowExecutor,
) -> WorkflowSessionResidencyState {
    executor.session_state.residency().await
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::events::NullEventSink;

    use super::{
        WorkflowExecutor, WorkflowSessionResidencyState, set_workflow_session_residency,
        workflow_session_checkpoint_summary, workflow_session_residency,
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
}
