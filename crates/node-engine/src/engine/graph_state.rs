use crate::error::{NodeEngineError, Result};
use crate::types::{GraphEdge, GraphNode, NodeId, WorkflowGraph};

use super::{WorkflowExecutor, graph_events};

pub(super) async fn update_node_data(
    executor: &WorkflowExecutor,
    node_id: &NodeId,
    data: serde_json::Value,
) -> Result<()> {
    {
        let mut graph = executor.graph.write().await;
        if let Some(node) = graph.find_node_mut(node_id) {
            node.data = data;
        } else {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Node '{}' not found",
                node_id
            )));
        }
    }

    executor.mark_modified(node_id).await;
    Ok(())
}

pub(super) async fn add_node(executor: &WorkflowExecutor, node: GraphNode) {
    let node_id = node.id.clone();
    let mut graph = executor.graph.write().await;
    graph.nodes.push(node);
    let workflow_id = graph.id.clone();
    drop(graph);
    executor.emit_graph_modified(workflow_id, vec![node_id]);
}

pub(super) async fn add_edge(executor: &WorkflowExecutor, edge: GraphEdge) {
    let target = edge.target.clone();
    {
        let mut graph = executor.graph.write().await;
        graph.edges.push(edge);
    }
    executor.mark_modified(&target).await;
}

pub(super) async fn remove_edge(executor: &WorkflowExecutor, edge_id: &str) {
    let target = {
        let mut graph = executor.graph.write().await;
        if let Some(idx) = graph.edges.iter().position(|edge| edge.id == edge_id) {
            let edge = graph.edges.remove(idx);
            Some(edge.target)
        } else {
            None
        }
    };

    if let Some(target) = target {
        executor.mark_modified(&target).await;
    }
}

pub(super) async fn get_graph_snapshot(executor: &WorkflowExecutor) -> WorkflowGraph {
    executor.graph.read().await.clone()
}

pub(super) async fn restore_graph_snapshot(executor: &WorkflowExecutor, graph: WorkflowGraph) {
    let workflow_id = graph.id.clone();
    let dirty_tasks = graph_events::snapshot_dirty_tasks(&graph);
    {
        let mut current_graph = executor.graph.write().await;
        *current_graph = graph;
    }

    let mut engine = executor.demand_engine.write().await;
    engine.clear_cache();
    drop(engine);
    executor.emit_graph_modified(workflow_id, dirty_tasks);
}
