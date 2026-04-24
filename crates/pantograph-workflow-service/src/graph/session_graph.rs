use super::types::{GraphEdge, GraphNode, Position, WorkflowGraph};
use uuid::Uuid;

pub(super) fn hydrate_embedding_emit_metadata_flags(mut graph: WorkflowGraph) -> WorkflowGraph {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        match node.data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                node.data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
    }
    graph
}

pub(super) fn sync_embedding_emit_metadata_flags(graph: &mut WorkflowGraph) {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if node.node_type != "embedding" {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        match node.data {
            serde_json::Value::Object(ref mut map) => {
                map.insert(
                    "emit_metadata".to_string(),
                    serde_json::json!(emit_metadata),
                );
            }
            _ => {
                node.data = serde_json::json!({ "emit_metadata": emit_metadata });
            }
        }
    }
}

pub fn convert_graph_to_node_engine(graph: &WorkflowGraph) -> node_engine::WorkflowGraph {
    let mut ne_graph =
        node_engine::WorkflowGraph::new(Uuid::new_v4().to_string(), "Workflow".to_string());

    for node in &graph.nodes {
        let mut data = node.data.clone();
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert("node_type".to_string(), serde_json::json!(node.node_type));
        }
        ne_graph.nodes.push(node_engine::GraphNode {
            id: node.id.clone(),
            node_type: node.node_type.clone(),
            data,
            position: (node.position.x, node.position.y),
        });
    }

    for edge in &graph.edges {
        ne_graph.edges.push(node_engine::GraphEdge {
            id: edge.id.clone(),
            source: edge.source.clone(),
            source_handle: edge.source_handle.clone(),
            target: edge.target.clone(),
            target_handle: edge.target_handle.clone(),
        });
    }

    ne_graph
}

pub fn convert_graph_from_node_engine(graph: &node_engine::WorkflowGraph) -> WorkflowGraph {
    WorkflowGraph {
        nodes: graph
            .nodes
            .iter()
            .map(|node| GraphNode {
                id: node.id.clone(),
                node_type: node.node_type.clone(),
                position: Position {
                    x: node.position.0,
                    y: node.position.1,
                },
                data: node.data.clone(),
            })
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| GraphEdge {
                id: edge.id.clone(),
                source: edge.source.clone(),
                source_handle: edge.source_handle.clone(),
                target: edge.target.clone(),
                target_handle: edge.target_handle.clone(),
            })
            .collect(),
        derived_graph: None,
    }
}

pub(super) fn merge_node_data(existing: &mut serde_json::Value, patch: serde_json::Value) {
    match (existing, patch) {
        (serde_json::Value::Object(existing_map), serde_json::Value::Object(patch_map)) => {
            for (key, value) in patch_map {
                existing_map.insert(key, value);
            }
        }
        (existing_value, replacement) => {
            *existing_value = replacement;
        }
    }
}
