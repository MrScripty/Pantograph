use super::{
    GraphMemoryImpactSummary, NodeMemoryIdentity, NodeMemorySnapshot, NodeMemoryStatus,
    WorkflowExecutor, WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState,
};
use crate::error::Result;
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

pub(super) async fn mark_workflow_session_checkpoint_available(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
) {
    executor
        .session_state
        .mark_checkpoint_available(workflow_session_id)
        .await;
}

pub(super) async fn clear_workflow_session_checkpoint(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
) {
    executor
        .session_state
        .clear_checkpoint(workflow_session_id)
        .await;
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

pub(super) async fn reconcile_workflow_session_node_memory(
    executor: &WorkflowExecutor,
    workflow_session_id: &str,
    memory_impact: &GraphMemoryImpactSummary,
) {
    executor
        .session_state
        .reconcile_node_memory(workflow_session_id, memory_impact)
        .await;
}

pub(super) async fn bound_workflow_session_node_memory_view(
    executor: &WorkflowExecutor,
) -> Option<std::collections::HashMap<NodeId, NodeMemorySnapshot>> {
    let workflow_session_id = executor.session_state.bound_workflow_session_id().await?;
    let snapshots = executor
        .session_state
        .node_memory_snapshots(&workflow_session_id)
        .await;
    if snapshots.is_empty() {
        return None;
    }

    Some(
        snapshots
            .into_iter()
            .map(|snapshot| (snapshot.identity.node_id.clone(), snapshot))
            .collect(),
    )
}

pub(super) fn inject_node_memory_input(
    inputs: &mut std::collections::HashMap<String, serde_json::Value>,
    snapshot: &NodeMemorySnapshot,
) -> Result<()> {
    inputs.insert("_node_memory".to_string(), serde_json::to_value(snapshot)?);
    Ok(())
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
                node_memory_snapshot_from_cache_entry(
                    &workflow_session_id,
                    node_id,
                    cached,
                    demand_engine.last_inputs.get(node_id),
                    &graph,
                )
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
    input_snapshot: Option<&serde_json::Value>,
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
        input_fingerprint: input_snapshot.map(canonical_json_fingerprint),
        output_snapshot: Some(cached.value.clone()),
        private_state: None,
        indirect_state_reference: None,
        inspection_metadata: Some(cache_projection_metadata(input_snapshot, cached.version)),
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

fn cache_projection_metadata(
    input_snapshot: Option<&serde_json::Value>,
    cache_version: u64,
) -> serde_json::Value {
    let mut metadata = serde_json::Map::from_iter([
        (
            "projection_source".to_string(),
            serde_json::json!("demand_engine_cache"),
        ),
        (
            "cache_version".to_string(),
            serde_json::json!(cache_version),
        ),
    ]);
    if let Some(input_snapshot) = input_snapshot {
        metadata.insert("input_snapshot".to_string(), input_snapshot.clone());
    }
    serde_json::Value::Object(metadata)
}

fn canonical_json_fingerprint(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        serde_json::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json_fingerprint)
                .collect::<Vec<_>>()
                .join(",")
        ),
        serde_json::Value::Object(values) => {
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            format!(
                "{{{}}}",
                entries
                    .into_iter()
                    .map(|(key, value)| format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_default(),
                        canonical_json_fingerprint(value)
                    ))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;

    use crate::TaskExecutor;
    use crate::events::NullEventSink;

    use super::{
        GraphMemoryImpactSummary, NodeMemorySnapshot, WorkflowExecutor,
        WorkflowSessionResidencyState, bind_workflow_session, bound_workflow_session_id,
        clear_bound_workflow_session, clear_workflow_session_checkpoint,
        clear_workflow_session_node_memory,
        mark_workflow_session_checkpoint_available,
        reconcile_workflow_session_node_memory, record_workflow_session_node_memory,
        set_workflow_session_residency, sync_bound_session_node_memory_from_cache,
        workflow_session_checkpoint_summary, workflow_session_node_memory_snapshots,
        workflow_session_residency,
    };
    use crate::{NodeMemoryCompatibility, NodeMemoryCompatibilitySnapshot};

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
            Ok(std::collections::HashMap::from([
                ("out".to_string(), serde_json::json!(task_id)),
                ("value".to_string(), serde_json::json!(task_id)),
            ]))
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
            let payload = serde_json::json!({
                "task": task_id,
                "sequence": sequence,
            });
            Ok(std::collections::HashMap::from([
                ("out".to_string(), payload.clone()),
                ("value".to_string(), payload),
            ]))
        }
    }

    struct MemoryConsumingTaskExecutor {
        execution_counter: AtomicUsize,
    }

    impl MemoryConsumingTaskExecutor {
        fn new() -> Self {
            Self {
                execution_counter: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl TaskExecutor for MemoryConsumingTaskExecutor {
        async fn execute_task(
            &self,
            _task_id: &str,
            inputs: std::collections::HashMap<String, serde_json::Value>,
            _context: &graph_flow::Context,
            _extensions: &crate::extensions::ExecutorExtensions,
        ) -> crate::error::Result<std::collections::HashMap<String, serde_json::Value>> {
            let sequence = self.execution_counter.fetch_add(1, Ordering::SeqCst) + 1;
            let previous_output = inputs
                .get("_node_memory")
                .and_then(|memory| memory.get("output_snapshot"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let memory_status = inputs
                .get("_node_memory")
                .and_then(|memory| memory.get("status"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let payload = serde_json::json!({
                "sequence": sequence,
                "previous_output": previous_output,
                "memory_status": memory_status,
            });

            Ok(std::collections::HashMap::from([
                ("out".to_string(), payload.clone()),
                ("value".to_string(), payload),
            ]))
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

    fn single_node_graph() -> crate::types::WorkflowGraph {
        let mut graph = crate::types::WorkflowGraph::new("graph-1", "Graph");
        graph.nodes.push(crate::types::GraphNode {
            id: "memory".to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({}),
            position: (0.0, 0.0),
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

        mark_workflow_session_checkpoint_available(&executor, "session-1").await;
        let checkpointed_summary = workflow_session_checkpoint_summary(&executor, "session-1").await;
        assert!(checkpointed_summary.checkpoint_available);
        assert!(checkpointed_summary.checkpointed_at_ms.is_some());

        clear_workflow_session_checkpoint(&executor, "session-1").await;
        let cleared_summary = workflow_session_checkpoint_summary(&executor, "session-1").await;
        assert!(!cleared_summary.checkpoint_available);
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
                indirect_state_reference: None,
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

    #[tokio::test]
    async fn rerun_can_consume_prior_node_memory_from_the_bound_workflow_session() {
        let executor =
            WorkflowExecutor::new("exec-1", single_node_graph(), Arc::new(NullEventSink));
        let task_executor = MemoryConsumingTaskExecutor::new();
        bind_workflow_session(&executor, "session-1").await;

        let first_run = executor
            .demand(&"memory".to_string(), &task_executor)
            .await
            .expect("run first memory demand");
        assert_eq!(
            first_run.get("value"),
            Some(&serde_json::json!({
                "sequence": 1,
                "previous_output": null,
                "memory_status": null,
            }))
        );

        executor.mark_modified(&"memory".to_string()).await;
        let second_run = executor
            .demand(&"memory".to_string(), &task_executor)
            .await
            .expect("run second memory demand");
        assert_eq!(
            second_run.get("value"),
            Some(&serde_json::json!({
                "sequence": 2,
                "previous_output": {
                    "out": {
                        "sequence": 1,
                        "previous_output": null,
                        "memory_status": null,
                    },
                    "value": {
                        "sequence": 1,
                        "previous_output": null,
                        "memory_status": null,
                    }
                },
                "memory_status": "ready",
            }))
        );
    }

    #[tokio::test]
    async fn workflow_session_helpers_reconcile_recorded_node_memory() {
        let executor = WorkflowExecutor::new("exec-1", linear_graph(), Arc::new(NullEventSink));
        bind_workflow_session(&executor, "session-1").await;
        record_workflow_session_node_memory(
            &executor,
            NodeMemorySnapshot {
                identity: super::NodeMemoryIdentity {
                    session_id: "session-1".to_string(),
                    node_id: "b".to_string(),
                    node_type: "process".to_string(),
                    schema_version: Some("v1".to_string()),
                },
                status: super::NodeMemoryStatus::Ready,
                input_fingerprint: Some("fp-b".to_string()),
                output_snapshot: Some(serde_json::json!({ "out": "b" })),
                private_state: None,
                indirect_state_reference: None,
                inspection_metadata: None,
            },
        )
        .await;

        reconcile_workflow_session_node_memory(
            &executor,
            "session-1",
            &GraphMemoryImpactSummary {
                node_decisions: vec![NodeMemoryCompatibilitySnapshot {
                    node_id: "b".to_string(),
                    compatibility: NodeMemoryCompatibility::PreserveWithInputRefresh,
                    reason: Some("upstream_dependency_changed".to_string()),
                }],
                fallback_to_full_invalidation: false,
            },
        )
        .await;

        let snapshots = workflow_session_node_memory_snapshots(&executor, "session-1").await;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].identity.node_id, "b");
        assert_eq!(snapshots[0].status, super::NodeMemoryStatus::Invalidated);
    }
}
