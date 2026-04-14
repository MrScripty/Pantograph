//! Backend-owned helpers for embedding workflow runtime preparation.
//!
//! These helpers inspect workflow graphs to determine whether embedding runtime
//! preparation is required and which Puma-Lib model id must back an embedding
//! workflow execution.

use std::collections::{BTreeSet, HashMap};

fn node_data_string(data: &serde_json::Value, keys: &[&str]) -> Option<String> {
    let obj = data.as_object()?;
    keys.iter().find_map(|key| {
        obj.get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    })
}

pub fn workflow_graph_has_embedding_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph.nodes.iter().any(|node| node.node_type == "embedding")
}

pub fn workflow_graph_has_llamacpp_inference_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph
        .nodes
        .iter()
        .any(|node| node.node_type == "llamacpp-inference")
}

pub fn resolve_embedding_model_id_from_workflow_graph(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> Result<Option<String>, String> {
    let node_by_id = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();

    let embedding_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.node_type == "embedding")
        .collect::<Vec<_>>();
    if embedding_nodes.is_empty() {
        return Ok(None);
    }

    let mut selected_model_ids = BTreeSet::new();
    for embedding_node in embedding_nodes {
        let mut model_ids_for_node = BTreeSet::new();
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.target == embedding_node.id && edge.target_handle == "model")
        {
            let source_node = node_by_id.get(edge.source.as_str()).ok_or_else(|| {
                format!(
                    "Embedding node '{}' references unknown source node '{}'",
                    embedding_node.id, edge.source
                )
            })?;
            if source_node.node_type != "puma-lib" {
                return Err(format!(
                    "Embedding node '{}' must receive `model` from a Puma-Lib node",
                    embedding_node.id
                ));
            }
            let model_id = node_data_string(&source_node.data, &["model_id", "modelId"])
                .ok_or_else(|| {
                    format!(
                        "Puma-Lib node '{}' is missing `model_id`. Re-select a model in Puma-Lib.",
                        source_node.id
                    )
                })?;
            model_ids_for_node.insert(model_id);
        }

        if model_ids_for_node.is_empty() {
            return Err(format!(
                "Embedding node '{}' must connect Puma-Lib `model_path` output to `model` input",
                embedding_node.id
            ));
        }
        if model_ids_for_node.len() > 1 {
            return Err(format!(
                "Embedding node '{}' has multiple Puma-Lib model IDs connected to `model`; use exactly one",
                embedding_node.id
            ));
        }
        selected_model_ids.extend(model_ids_for_node);
    }

    if selected_model_ids.len() > 1 {
        return Err(
            "All embedding nodes in one workflow run must use the same Puma-Lib model".to_string(),
        );
    }

    Ok(selected_model_ids.into_iter().next())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_workflow_service::{GraphEdge, GraphNode, Position, WorkflowGraph};

    fn graph(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> WorkflowGraph {
        WorkflowGraph {
            nodes,
            edges,
            ..WorkflowGraph::default()
        }
    }

    fn node(id: &str, node_type: &str, data: serde_json::Value) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            position: Position { x: 0.0, y: 0.0 },
            data,
        }
    }

    fn edge(id: &str, source: &str, target: &str, target_handle: &str) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: "model_path".to_string(),
            target_handle: target_handle.to_string(),
        }
    }

    #[test]
    fn workflow_graph_embedding_helpers_detect_embedding_and_llamacpp_nodes() {
        let graph = graph(
            vec![
                node("embed", "embedding", serde_json::json!({})),
                node("infer", "llamacpp-inference", serde_json::json!({})),
            ],
            Vec::new(),
        );

        assert!(workflow_graph_has_embedding_node(&graph));
        assert!(workflow_graph_has_llamacpp_inference_node(&graph));
    }

    #[test]
    fn resolve_embedding_model_id_returns_connected_puma_lib_model() {
        let graph = graph(
            vec![
                node(
                    "puma",
                    "puma-lib",
                    serde_json::json!({ "model_id": "embed-model" }),
                ),
                node("embed", "embedding", serde_json::json!({})),
            ],
            vec![edge("edge-1", "puma", "embed", "model")],
        );

        assert_eq!(
            resolve_embedding_model_id_from_workflow_graph(&graph).expect("model id"),
            Some("embed-model".to_string())
        );
    }

    #[test]
    fn resolve_embedding_model_id_rejects_non_puma_lib_sources() {
        let graph = graph(
            vec![
                node(
                    "input",
                    "text-input",
                    serde_json::json!({ "value": "not a model" }),
                ),
                node("embed", "embedding", serde_json::json!({})),
            ],
            vec![edge("edge-1", "input", "embed", "model")],
        );

        let error =
            resolve_embedding_model_id_from_workflow_graph(&graph).expect_err("should reject");
        assert!(error.contains("must receive `model` from a Puma-Lib node"));
    }
}
