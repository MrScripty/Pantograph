use std::collections::HashSet;

use serde_json::json;

use super::super::registry::NodeRegistry;
use super::super::types::{GraphEdge, GraphNode, Position, WorkflowGraph};
use super::{canonicalize_workflow_graph, canonicalize_workflow_graph_with_migrations};

fn graph_node(id: &str, node_type: &str, data: serde_json::Value) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        position: Position { x: 0.0, y: 0.0 },
        data,
    }
}

#[test]
fn canonicalize_workflow_graph_preserves_retired_inference_nodes_for_stale_diagnostics() {
    let registry = NodeRegistry::new();
    let retired_node_types = [
        "diffusion-inference",
        "llamacpp-inference",
        "pytorch-inference",
        "embedding",
        "reranker",
        "ollama-inference",
    ];
    let graph = WorkflowGraph {
        nodes: retired_node_types
            .iter()
            .map(|node_type| {
                graph_node(
                    node_type,
                    node_type,
                    json!({
                        "label": node_type,
                        "model_path": "/models/legacy",
                        "task_kind": "legacy"
                    }),
                )
            })
            .collect(),
        edges: Vec::new(),
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical_node_types = result
        .graph
        .nodes
        .iter()
        .map(|node| node.node_type.as_str())
        .collect::<HashSet<_>>();

    for retired_node_type in retired_node_types {
        assert!(
            canonical_node_types.contains(retired_node_type),
            "{retired_node_type} should remain available for stale diagnostics"
        );
    }
    assert!(
        result.migration_records.is_empty(),
        "current canonicalization must not create compatibility upgrade records"
    );
}

#[test]
fn canonicalize_workflow_graph_repairs_llm_stream_edge_to_text_output() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            graph_node(
                "llm",
                "llm-inference",
                json!({
                    "task_kind": "text_generation",
                    "backend_key": "llama_cpp",
                    "pumas_model_ref": {
                        "source": "puma-lib",
                        "status": "resolved",
                        "model_id": "family/model",
                        "model_path": "/models/model.gguf"
                    }
                }),
            ),
            graph_node("output", "text-output", json!({})),
        ],
        edges: vec![GraphEdge {
            id: "llm-stream-output-stream".to_string(),
            source: "llm".to_string(),
            source_handle: "stream".to_string(),
            target: "output".to_string(),
            target_handle: "stream".to_string(),
        }],
        derived_graph: None,
    };

    let canonical = canonicalize_workflow_graph(graph, &registry);

    assert_eq!(canonical.edges.len(), 1);
    assert_eq!(canonical.edges[0].source, "llm");
    assert_eq!(canonical.edges[0].source_handle, "response");
    assert_eq!(canonical.edges[0].target, "output");
    assert_eq!(canonical.edges[0].target_handle, "text");
    assert_eq!(canonical.edges[0].id, "llm-response-output-text");
}

#[test]
fn canonicalize_workflow_graph_does_not_hydrate_retired_inference_settings_edges() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            graph_node(
                "source",
                "puma-lib",
                json!({
                    "inference_settings": [
                        {
                            "key": "steps",
                            "label": "Steps",
                            "param_type": "Number",
                            "default": 30
                        }
                    ]
                }),
            ),
            graph_node("expand", "expand-settings", json!({})),
            graph_node(
                "image-generation",
                "llm-inference",
                json!({
                    "task_kind": "image_generation",
                    "backend_key": "pytorch"
                }),
            ),
        ],
        edges: vec![
            GraphEdge {
                id: "source-settings-expand-settings".to_string(),
                source: "source".to_string(),
                source_handle: "inference_settings".to_string(),
                target: "expand".to_string(),
                target_handle: "inference_settings".to_string(),
            },
            GraphEdge {
                id: "expand-settings-image-generation-settings".to_string(),
                source: "expand".to_string(),
                source_handle: "inference_settings".to_string(),
                target: "image-generation".to_string(),
                target_handle: "inference_settings".to_string(),
            },
        ],
        derived_graph: None,
    };

    let canonical = canonicalize_workflow_graph(graph, &registry);
    let image_generation_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "image-generation")
        .expect("image-generation node");

    assert!(
        image_generation_node.data.get("definition").is_none(),
        "retired inference_settings edges must not synthesize descriptor ports"
    );
    assert!(
        canonical
            .edges
            .iter()
            .all(|edge| edge.source_handle != "steps" && edge.target_handle != "steps"),
        "retired inference_settings edges must not synthesize dynamic parameter edges"
    );
}
