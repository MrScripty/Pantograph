//! Backend-owned helpers for embedding workflow runtime preparation.
//!
//! These helpers inspect workflow graphs to determine whether embedding runtime
//! preparation is required and which Puma-Lib model id must back an embedding
//! workflow execution.

use std::collections::{BTreeSet, HashMap};

use pantograph_runtime_identity::canonical_runtime_backend_key;

const RETIRED_EMBEDDING_RUNTIME_PREPARE_ERROR: &str =
    "embedding_runtime_prepare_retired: direct embedding runtime preparation is scheduler-owned \
and must run through canonical scheduler task state/results plus runtime-host execution. The old \
embedded edit-session path must not resolve Puma-Lib model paths or start a dedicated embedding \
runtime from graph data.";

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

fn node_pumas_model_ref_model_id(data: &serde_json::Value) -> Option<String> {
    data.get("pumas_model_ref")
        .and_then(|value| value.get("model_id"))
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn node_task_kind(data: &serde_json::Value) -> Option<String> {
    node_data_string(data, &["task_kind", "taskKind"])
}

fn node_backend_key(data: &serde_json::Value) -> Option<String> {
    node_data_string(
        data,
        &[
            "backend_key",
            "backendKey",
            "recommended_backend",
            "recommendedBackend",
        ],
    )
    .or_else(|| {
        data.get("pumas_model_ref").and_then(|model_ref| {
            node_data_string(
                model_ref,
                &[
                    "backend_key",
                    "backendKey",
                    "recommended_backend",
                    "recommendedBackend",
                ],
            )
        })
    })
}

fn is_canonical_embedding_inference_node(node: &pantograph_workflow_service::GraphNode) -> bool {
    node.node_type == "llm-inference"
        && node_task_kind(&node.data)
            .is_some_and(|task_kind| task_kind.eq_ignore_ascii_case("embedding"))
}

pub fn workflow_graph_has_embedding_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph
        .nodes
        .iter()
        .any(is_canonical_embedding_inference_node)
}

pub fn workflow_graph_has_llamacpp_inference_node(
    graph: &pantograph_workflow_service::WorkflowGraph,
) -> bool {
    graph.nodes.iter().any(|node| {
        node.node_type == "llm-inference"
            && !is_canonical_embedding_inference_node(node)
            && node_backend_key(&node.data).is_some_and(|backend_key| {
                canonical_runtime_backend_key(&backend_key) == "llama_cpp"
            })
    })
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
        .filter(|node| is_canonical_embedding_inference_node(node))
        .collect::<Vec<_>>();
    if embedding_nodes.is_empty() {
        return Ok(None);
    }

    let mut selected_model_ids = BTreeSet::new();
    for embedding_node in embedding_nodes {
        let mut model_ids_for_node = BTreeSet::new();
        if let Some(model_id) = node_pumas_model_ref_model_id(&embedding_node.data) {
            model_ids_for_node.insert(model_id);
        }
        for edge in graph.edges.iter().filter(|edge| {
            edge.target == embedding_node.id && edge.target_handle == "pumas_model_ref"
        }) {
            let source_node = node_by_id.get(edge.source.as_str()).ok_or_else(|| {
                format!(
                    "Embedding node '{}' references unknown source node '{}'",
                    embedding_node.id, edge.source
                )
            })?;
            if source_node.node_type != "puma-lib" {
                return Err(format!(
                    "Embedding inference node '{}' must receive `pumas_model_ref` from a Puma-Lib node",
                    embedding_node.id
                ));
            }
            let model_id = node_pumas_model_ref_model_id(&source_node.data)
                .or_else(|| node_data_string(&source_node.data, &["model_id", "modelId"]))
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
                "Embedding inference node '{}' must connect Puma-Lib `pumas_model_ref` output to `pumas_model_ref` input or store a Pumas model ref with `model_id`",
                embedding_node.id
            ));
        }
        if model_ids_for_node.len() > 1 {
            return Err(format!(
                "Embedding inference node '{}' has multiple Puma-Lib model IDs; use exactly one",
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

pub async fn prepare_embedding_runtime_for_workflow(
    _gateway: &inference::InferenceGateway,
    _pumas_api: Option<&pumas_library::PumasApi>,
    _request: inference::EmbeddingStartRequest,
    _embedding_model_id_from_graph: Option<String>,
    needs_embedding_node: bool,
    _needs_llamacpp_inference_node: bool,
) -> Result<Option<inference::BackendConfig>, String> {
    if !needs_embedding_node {
        return Ok(None);
    }

    Err(RETIRED_EMBEDDING_RUNTIME_PREPARE_ERROR.to_string())
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

    fn edge(
        id: &str,
        source: &str,
        source_handle: &str,
        target: &str,
        target_handle: &str,
    ) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle: source_handle.to_string(),
            target_handle: target_handle.to_string(),
        }
    }

    #[test]
    fn workflow_graph_embedding_helpers_detect_embedding_and_llamacpp_nodes() {
        let graph = graph(
            vec![
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding", "backend_key": "llama_cpp"}),
                ),
                node(
                    "infer",
                    "llm-inference",
                    serde_json::json!({"task_kind": "text_generation", "backend_key": "llama_cpp"}),
                ),
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
                    serde_json::json!({
                        "pumas_model_ref": {
                            "model_id": "embed-model"
                        }
                    }),
                ),
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
            ],
            vec![edge(
                "edge-1",
                "puma",
                "pumas_model_ref",
                "embed",
                "pumas_model_ref",
            )],
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
                node(
                    "embed",
                    "llm-inference",
                    serde_json::json!({"task_kind": "embedding"}),
                ),
            ],
            vec![edge("edge-1", "input", "text", "embed", "pumas_model_ref")],
        );

        let error =
            resolve_embedding_model_id_from_workflow_graph(&graph).expect_err("should reject");
        assert!(error.contains("must receive `pumas_model_ref` from a Puma-Lib node"));
    }

    #[tokio::test]
    async fn prepare_embedding_runtime_fails_closed_before_path_resolution() {
        let gateway = inference::InferenceGateway::new();
        let error = prepare_embedding_runtime_for_workflow(
            &gateway,
            None,
            inference::EmbeddingStartRequest {
                gguf_model_path: Some("/tmp/legacy-embedding.gguf".into()),
                ..inference::EmbeddingStartRequest::default()
            },
            Some("embedding/example".to_string()),
            true,
            false,
        )
        .await
        .expect_err("direct embedding runtime preparation must fail closed");

        assert!(error.contains("embedding_runtime_prepare_retired"));
        assert!(error.contains("scheduler task state/results"));
        assert!(!error.contains("/tmp/legacy-embedding.gguf"));
    }
}
