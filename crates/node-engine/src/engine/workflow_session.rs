use super::{
    GraphMemoryImpactSummary, NodeMemoryIdentity, NodeMemoryIndirectStateReference,
    NodeMemoryRestoreStrategy, NodeMemorySnapshot, NodeMemoryStatus, WorkflowExecutor,
    WorkflowSessionCheckpointSummary, WorkflowSessionResidencyState,
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
        indirect_state_reference: kv_cache_reference_from_output_snapshot(&cached.value),
        inspection_metadata: Some(cache_projection_metadata(input_snapshot, cached.version)),
    })
}

fn kv_cache_reference_from_output_snapshot(
    output_snapshot: &serde_json::Value,
) -> Option<NodeMemoryIndirectStateReference> {
    let handle = output_snapshot.get("kv_cache_out")?;
    if handle.is_null() {
        return None;
    }

    let cache_id = handle.get("cache_id")?.as_str()?;
    let compatibility = handle.get("compatibility")?;
    let model_fingerprint = compatibility.get("model_fingerprint")?.clone();
    let runtime_fingerprint = compatibility.get("runtime_fingerprint")?.clone();
    let backend_key = runtime_fingerprint.get("backend_key")?.as_str()?;

    Some(NodeMemoryIndirectStateReference {
        reference_kind: "kv_cache_handle".to_string(),
        reference_id: cache_id.to_string(),
        restore_strategy: NodeMemoryRestoreStrategy::RehydrateBeforeResume,
        inspection_metadata: Some(serde_json::json!({
            "source_port": "kv_cache_out",
            "backend_key": backend_key,
            "model_fingerprint": model_fingerprint,
            "runtime_fingerprint": runtime_fingerprint,
        })),
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
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;

    use crate::events::NullEventSink;
    use crate::TaskExecutor;

    use super::{
        bind_workflow_session, bound_workflow_session_id, clear_bound_workflow_session,
        clear_workflow_session_checkpoint, clear_workflow_session_node_memory,
        mark_workflow_session_checkpoint_available, reconcile_workflow_session_node_memory,
        record_workflow_session_node_memory, set_workflow_session_residency,
        sync_bound_session_node_memory_from_cache, workflow_session_checkpoint_summary,
        workflow_session_node_memory_snapshots, workflow_session_residency,
        GraphMemoryImpactSummary, NodeMemorySnapshot, WorkflowExecutor,
        WorkflowSessionResidencyState,
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

    fn kv_suffix_reuse_graph() -> crate::types::WorkflowGraph {
        crate::types::WorkflowGraph {
            id: "graph-1".to_string(),
            name: "Graph".to_string(),
            nodes: vec![
                crate::types::GraphNode {
                    id: "prefix-input".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({ "text": "prefix-alpha" }),
                    position: (0.0, 0.0),
                },
                crate::types::GraphNode {
                    id: "suffix-input".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({ "text": "suffix-alpha" }),
                    position: (0.0, 160.0),
                },
                crate::types::GraphNode {
                    id: "prefix-llm".to_string(),
                    node_type: "llamacpp-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (220.0, 0.0),
                },
                crate::types::GraphNode {
                    id: "suffix-llm".to_string(),
                    node_type: "llamacpp-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (460.0, 80.0),
                },
            ],
            edges: vec![
                crate::types::GraphEdge {
                    id: "edge-prefix".to_string(),
                    source: "prefix-input".to_string(),
                    source_handle: "text".to_string(),
                    target: "prefix-llm".to_string(),
                    target_handle: "prompt".to_string(),
                },
                crate::types::GraphEdge {
                    id: "edge-suffix".to_string(),
                    source: "suffix-input".to_string(),
                    source_handle: "text".to_string(),
                    target: "suffix-llm".to_string(),
                    target_handle: "prompt".to_string(),
                },
                crate::types::GraphEdge {
                    id: "edge-kv".to_string(),
                    source: "prefix-llm".to_string(),
                    source_handle: "kv_cache_out".to_string(),
                    target: "suffix-llm".to_string(),
                    target_handle: "kv_cache_in".to_string(),
                },
            ],
            groups: Vec::new(),
        }
    }

    #[path = "workflow_session_tests/kv_cache_memory.rs"]
    mod kv_cache_memory;
    #[path = "workflow_session_tests/memory_reconciliation.rs"]
    mod memory_reconciliation;
    #[path = "workflow_session_tests/session_helpers.rs"]
    mod session_helpers;
}
