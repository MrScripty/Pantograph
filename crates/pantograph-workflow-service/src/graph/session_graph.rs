use super::types::{GraphEdge, GraphNode, Position, WorkflowGraph};
use uuid::Uuid;

fn is_embedding_inference_node(node: &GraphNode) -> bool {
    node.node_type == "llm-inference"
        && node
            .data
            .get("task_kind")
            .or_else(|| node.data.get("taskKind"))
            .and_then(|value| value.as_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("embedding"))
}

fn set_embedding_emit_metadata(node: &mut GraphNode, emit_metadata: bool) {
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

pub(super) fn hydrate_embedding_emit_metadata_flags(mut graph: WorkflowGraph) -> WorkflowGraph {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if !is_embedding_inference_node(node) {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        set_embedding_emit_metadata(node, emit_metadata);
    }
    graph
}

pub(super) fn sync_embedding_emit_metadata_flags(graph: &mut WorkflowGraph) {
    let counts = graph.effective_consumer_count_map();
    for node in &mut graph.nodes {
        if !is_embedding_inference_node(node) {
            continue;
        }
        let key = format!("{}:metadata", node.id);
        let emit_metadata = counts.get(&key).copied().unwrap_or(0) > 0;
        set_embedding_emit_metadata(node, emit_metadata);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, node_type: &str, data: serde_json::Value) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            position: Position::default(),
            data,
        }
    }

    fn metadata_edge(source: &str) -> GraphEdge {
        GraphEdge {
            id: format!("{source}-metadata-output"),
            source: source.to_string(),
            source_handle: "metadata".to_string(),
            target: "metadata-output".to_string(),
            target_handle: "text".to_string(),
        }
    }

    #[test]
    fn hydrate_embedding_emit_metadata_uses_canonical_task_kind() {
        let graph = WorkflowGraph {
            nodes: vec![
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
                node("metadata-output", "text-output", serde_json::json!({})),
            ],
            edges: vec![metadata_edge("embed")],
            derived_graph: None,
        };

        let hydrated = hydrate_embedding_emit_metadata_flags(graph);
        let embed = hydrated
            .nodes
            .iter()
            .find(|node| node.id == "embed")
            .expect("embedding node");
        assert_eq!(embed.data["emit_metadata"], serde_json::json!(true));
    }

    #[test]
    fn sync_embedding_emit_metadata_ignores_retired_embedding_node_type() {
        let mut graph = WorkflowGraph {
            nodes: vec![
                node(
                    "legacy",
                    "embedding",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
                node("metadata-output", "text-output", serde_json::json!({})),
            ],
            edges: vec![metadata_edge("legacy")],
            derived_graph: None,
        };

        sync_embedding_emit_metadata_flags(&mut graph);
        let legacy = graph
            .nodes
            .iter()
            .find(|node| node.id == "legacy")
            .expect("legacy node");
        assert!(legacy.data.get("emit_metadata").is_none());
    }
}
