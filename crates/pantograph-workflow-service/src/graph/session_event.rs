use std::collections::{HashMap, HashSet, VecDeque};

use node_engine::WorkflowEvent;

use super::types::WorkflowGraph;

pub(crate) fn graph_modified_event(
    workflow_id: &str,
    execution_id: &str,
    dirty_tasks: Vec<String>,
) -> WorkflowEvent {
    WorkflowEvent::GraphModified {
        workflow_id: workflow_id.to_string(),
        execution_id: execution_id.to_string(),
        dirty_tasks,
        occurred_at_ms: None,
    }
    .now()
}

pub(crate) fn dirty_tasks_from_seed_nodes(
    graph: &WorkflowGraph,
    seed_node_ids: &[String],
) -> Vec<String> {
    if seed_node_ids.is_empty() {
        return Vec::new();
    }

    let downstream = downstream_targets_by_source(graph);
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    let mut dirty_tasks = Vec::new();

    for seed_node_id in seed_node_ids {
        if seen.insert(seed_node_id.clone()) {
            dirty_tasks.push(seed_node_id.clone());
            queue.push_back(seed_node_id.clone());
        }
    }

    while let Some(node_id) = queue.pop_front() {
        for target in downstream.get(&node_id).into_iter().flatten() {
            if seen.insert(target.clone()) {
                dirty_tasks.push(target.clone());
                queue.push_back(target.clone());
            }
        }
    }

    dirty_tasks
}

pub(crate) fn dirty_tasks_for_full_snapshot(graph: &WorkflowGraph) -> Vec<String> {
    graph.nodes.iter().map(|node| node.id.clone()).collect()
}

fn downstream_targets_by_source(graph: &WorkflowGraph) -> HashMap<String, Vec<String>> {
    let mut downstream = HashMap::new();

    for edge in &graph.edges {
        downstream
            .entry(edge.source.clone())
            .or_insert_with(Vec::new)
            .push(edge.target.clone());
    }

    downstream
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::{GraphEdge, GraphNode, Position};

    fn sample_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "input".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "middle".to_string(),
                    node_type: "llm-inference".to_string(),
                    position: Position { x: 100.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "output".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "input-middle".to_string(),
                    source: "input".to_string(),
                    source_handle: "text".to_string(),
                    target: "middle".to_string(),
                    target_handle: "prompt".to_string(),
                },
                GraphEdge {
                    id: "middle-output".to_string(),
                    source: "middle".to_string(),
                    source_handle: "response".to_string(),
                    target: "output".to_string(),
                    target_handle: "text".to_string(),
                },
            ],
            derived_graph: None,
        }
    }

    #[test]
    fn dirty_tasks_follow_graph_order_from_seed_nodes() {
        let graph = sample_graph();

        let dirty_tasks = dirty_tasks_from_seed_nodes(&graph, &["input".to_string()]);

        assert_eq!(dirty_tasks, vec!["input", "middle", "output"]);
    }

    #[test]
    fn dirty_tasks_for_full_snapshot_keeps_node_order() {
        let graph = sample_graph();

        let dirty_tasks = dirty_tasks_for_full_snapshot(&graph);

        assert_eq!(dirty_tasks, vec!["input", "middle", "output"]);
    }
}
