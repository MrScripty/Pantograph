use super::{
    NodeMemoryIdentity, NodeMemorySnapshot, NodeMemoryStatus, WorkflowExecutor,
    WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState,
};
use crate::types::NodeId;

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

pub(super) async fn sync_bound_session_node_memory_from_cache(executor: &WorkflowExecutor) {
    let Some(workflow_session_id) = executor.session_state.bound_workflow_session_id().await else {
        return;
    };

    let snapshots = {
        let graph = executor.graph.read().await;
        let demand_engine = executor.demand_engine.read().await;
        demand_engine
            .cache
            .iter()
            .filter_map(|(node_id, cached)| {
                node_memory_snapshot_from_cache_entry(&workflow_session_id, node_id, cached, &graph)
            })
            .collect::<Vec<_>>()
    };

    for snapshot in snapshots {
        executor.session_state.record_node_memory(snapshot).await;
    }
}

fn node_memory_snapshot_from_cache_entry(
    workflow_session_id: &str,
    node_id: &NodeId,
    cached: &super::CachedOutput,
    graph: &crate::types::WorkflowGraph,
) -> Option<NodeMemorySnapshot> {
    let node = graph.find_node(node_id)?;
    Some(NodeMemorySnapshot {
        identity: NodeMemoryIdentity {
            session_id: workflow_session_id.to_string(),
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            schema_version: node_schema_version(&node.data),
        },
        status: NodeMemoryStatus::Ready,
        input_fingerprint: None,
        output_snapshot: Some(cached.value.clone()),
        private_state: None,
        inspection_metadata: None,
    })
}

fn node_schema_version(node_data: &serde_json::Value) -> Option<String> {
    node_data
        .get("definition")
        .and_then(|definition| {
            definition
                .get("schema_version")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    definition
                        .get("schemaVersion")
                        .and_then(|value| value.as_str())
                })
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use crate::TaskExecutor;
    use crate::events::NullEventSink;

    use super::{
        NodeMemorySnapshot, WorkflowExecutor, WorkflowSessionResidencyState, bind_workflow_session,
        bound_workflow_session_id, clear_bound_workflow_session,
        clear_workflow_session_node_memory, record_workflow_session_node_memory,
        set_workflow_session_residency, sync_bound_session_node_memory_from_cache,
        workflow_session_checkpoint_summary, workflow_session_node_memory_snapshots,
        workflow_session_residency,
    };

    struct SnapshotTaskExecutor;

    #[async_trait]
    impl TaskExecutor for SnapshotTaskExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            _inputs: std::collections::HashMap<String, serde_json::Value>,
            _context: &graph_flow::Context,
            _extensions: &crate::extensions::ExecutorExtensions,
        ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
            Ok(std::collections::HashMap::from([(
                "value".to_string(),
                serde_json::json!(task_id),
            )]))
        }
    }

    struct SequencedSnapshotTaskExecutor {
        execution_counter: AtomicUsize,
    }

    impl SequencedSnapshotTaskExecutor {
        fn new() -> Self {
            Self {
                execution_counter: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl TaskExecutor for SequencedSnapshotTaskExecutor {
        async fn execute_task(
            &self,
            task_id: &str,
            _inputs: std::collections::HashMap<String, serde_json::Value>,
            _context: &graph_flow::Context,
            _extensions: &crate::extensions::ExecutorExtensions,
        ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
            let sequence = self.execution_counter.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(std::collections::HashMap::from([(
                "value".to_string(),
                serde_json::json!({
                    "task": task_id,
                    "sequence": sequence,
                }),
            )]))
        }
    }

    fn linear_graph() -> crate::types::WorkflowGraph {
        let mut graph = crate::types::WorkflowGraph::new("graph-1", "Graph");
        graph.nodes.push(crate::types::GraphNode {
            id: "a".to_string(),
            node_type: "input".to_string(),
            data: serde_json::json!({
                "definition": { "schema_version": "v1" }
            }),
            position: (0.0, 0.0),
        });
        graph.nodes.push(crate::types::GraphNode {
            id: "b".to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({}),
            position: (100.0, 0.0),
        });
        graph.nodes.push(crate::types::GraphNode {
            id: "c".to_string(),
            node_type: "output".to_string(),
            data: serde_json::json!({}),
            position: (200.0, 0.0),
        });
        graph.edges.push(crate::types::GraphEdge {
            id: "e1".to_string(),
            source: "a".to_string(),
            source_handle: "out".to_string(),
            target: "b".to_string(),
            target_handle: "in".to_string(),
        });
        graph.edges.push(crate::types::GraphEdge {
            id: "e2".to_string(),
            source: "b".to_string(),
            source_handle: "out".to_string(),
            target: "c".to_string(),
            target_handle: "in".to_string(),
        });
        graph
    }

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

    #[tokio::test]
    async fn sync_bound_session_node_memory_from_cache_projects_all_cached_nodes() {
        let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
        bind_workflow_session(&executor, "session-1").await;

        executor
            .demand(&"c".to_string(), &SnapshotTaskExecutor)
            .await
            .expect("demand graph");
        sync_bound_session_node_memory_from_cache(&executor).await;

        let snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
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
            Some(serde_json::json!({ "value": "c" }))
        );
    }

    #[tokio::test]
    async fn repeated_runs_replace_node_memory_for_the_same_workflow_session() {
        let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
        let task_executor = SequencedSnapshotTaskExecutor::new();
        bind_workflow_session(&executor, "session-1").await;

        executor
            .demand(&"c".to_string(), &task_executor)
            .await
            .expect("run first demand");
        let first_snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
        assert_eq!(first_snapshots.len(), 3);
        assert_eq!(
            first_snapshots[2].output_snapshot,
            Some(serde_json::json!({
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

        let second_snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
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
                "value": {
                    "task": "c",
                    "sequence": 6,
                }
            }))
        );
    }
}
