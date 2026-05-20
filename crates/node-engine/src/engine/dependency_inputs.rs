use std::collections::HashMap;

use crate::types::{NodeId, WorkflowGraph};

const MODEL_INTENT_CONTEXT_KEYS: [&str; 6] = [
    "pumas_model_ref",
    "model_id",
    "model_type",
    "task_type_primary",
    "selected_binding_ids",
    "platform_context",
];

pub(super) fn resolve_dependency_inputs(
    graph: &WorkflowGraph,
    node_id: &NodeId,
    dependency_outputs: &HashMap<NodeId, HashMap<String, serde_json::Value>>,
) -> HashMap<String, serde_json::Value> {
    let mut inputs = HashMap::new();

    for edge in graph.incoming_edges(node_id) {
        let Some(dep_outputs) = dependency_outputs.get(&edge.source) else {
            continue;
        };

        let (source_handle, target_handle) = canonical_dependency_edge_handles(graph, edge);
        if matches!(
            target_handle,
            "resolved_model_source" | "resolved_model_package_facts" | "model_package_facts"
        ) {
            continue;
        }
        if let Some(value) = dep_outputs
            .get(source_handle)
            .filter(|value| direct_dependency_value_allowed(target_handle, value))
        {
            inputs.insert(target_handle.to_string(), value.clone());
        }

        if matches!(target_handle, "model_path" | "pumas_model_ref") {
            merge_model_context(&mut inputs, dep_outputs);
        }
    }

    inputs
}

fn direct_dependency_value_allowed(target_handle: &str, value: &serde_json::Value) -> bool {
    target_handle != "pumas_model_ref" || value.is_object()
}

fn canonical_dependency_edge_handles<'a>(
    graph: &'a WorkflowGraph,
    edge: &'a crate::types::GraphEdge,
) -> (&'a str, &'a str) {
    let source_is_llm = graph
        .find_node(&edge.source)
        .is_some_and(|node| node.node_type == "llm-inference");
    let target_is_text_output = graph
        .find_node(&edge.target)
        .is_some_and(|node| node.node_type == "text-output");

    if source_is_llm
        && target_is_text_output
        && edge.source_handle == "stream"
        && edge.target_handle == "stream"
    {
        return ("response", "text");
    }

    (&edge.source_handle, &edge.target_handle)
}

fn merge_model_context(
    inputs: &mut HashMap<String, serde_json::Value>,
    dep_outputs: &HashMap<String, serde_json::Value>,
) {
    for context_key in MODEL_INTENT_CONTEXT_KEYS {
        if let Some(existing) = inputs.get(context_key) {
            if context_key == "pumas_model_ref" && !existing.is_object() {
                if let Some(value) = dep_outputs
                    .get(context_key)
                    .filter(|value| value.is_object())
                {
                    inputs.insert(context_key.to_string(), value.clone());
                }
            }
            continue;
        }
        if let Some(value) = dep_outputs.get(context_key) {
            inputs.insert(context_key.to_string(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraphEdge, GraphNode, WorkflowGraph};

    #[test]
    fn resolve_dependency_inputs_maps_edges_by_port() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "source".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "target".to_string(),
                    node_type: "text-output".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "source".to_string(),
                source_handle: "text".to_string(),
                target: "target".to_string(),
                target_handle: "input".to_string(),
            }],
            groups: Vec::new(),
        };

        let dependency_outputs = HashMap::from([(
            "source".to_string(),
            HashMap::from([("text".to_string(), serde_json::json!("hello"))]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"target".to_string(), &dependency_outputs);

        assert_eq!(inputs.get("input"), Some(&serde_json::json!("hello")));
    }

    #[test]
    fn resolve_dependency_inputs_preserves_direct_model_path_edge_without_path_context_repair() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "puma-lib".to_string(),
                    node_type: "puma-lib".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "runtime".to_string(),
                    node_type: "llm".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "puma-lib".to_string(),
                source_handle: "model_path".to_string(),
                target: "runtime".to_string(),
                target_handle: "model_path".to_string(),
            }],
            groups: Vec::new(),
        };

        let dependency_outputs = HashMap::from([(
            "puma-lib".to_string(),
            HashMap::from([
                (
                    "model_path".to_string(),
                    serde_json::json!("/tmp/model.gguf"),
                ),
                ("model_id".to_string(), serde_json::json!("family/model")),
                ("backend_key".to_string(), serde_json::json!("llamacpp")),
                (
                    "recommended_backend".to_string(),
                    serde_json::json!("llamacpp"),
                ),
            ]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(
            inputs.get("model_path"),
            Some(&serde_json::json!("/tmp/model.gguf"))
        );
        assert_eq!(
            inputs.get("model_id"),
            Some(&serde_json::json!("family/model"))
        );
        assert_eq!(inputs.get("backend_key"), None);
        assert_eq!(inputs.get("recommended_backend"), None);
    }

    #[test]
    fn resolve_dependency_inputs_keeps_pumas_model_ref_intent_only() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "puma-lib".to_string(),
                    node_type: "puma-lib".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "runtime".to_string(),
                    node_type: "llm-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "puma-lib".to_string(),
                source_handle: "pumas_model_ref".to_string(),
                target: "runtime".to_string(),
                target_handle: "pumas_model_ref".to_string(),
            }],
            groups: Vec::new(),
        };

        let package_facts = serde_json::json!({
            "package_facts_contract_version": 1,
            "model_ref": {
                "model_id": "family/model"
            }
        });
        let load_target = serde_json::json!({
            "model_ref": {
                "model_id": "family/model"
            },
            "artifact_kind": "diffusers_bundle",
            "local_load_path": "/pumas/models/family/model",
            "load_path_kind": "directory",
            "storage_kind": "library_owned",
            "validation_state": "valid"
        });
        let dependency_outputs = HashMap::from([(
            "puma-lib".to_string(),
            HashMap::from([
                (
                    "pumas_model_ref".to_string(),
                    serde_json::json!({
                        "model_id": "family/model"
                    }),
                ),
                ("model_id".to_string(), serde_json::json!("family/model")),
                (
                    "resolved_model_package_facts".to_string(),
                    package_facts.clone(),
                ),
                (
                    "resolved_model_artifact_load_target".to_string(),
                    load_target.clone(),
                ),
            ]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(
            inputs.get("pumas_model_ref"),
            Some(&serde_json::json!({
                "model_id": "family/model"
            }))
        );
        assert_eq!(inputs.get("resolved_model_package_facts"), None);
        assert_eq!(inputs.get("resolved_model_artifact_load_target"), None);
        assert_eq!(
            inputs.get("model_id"),
            Some(&serde_json::json!("family/model"))
        );
    }

    #[test]
    fn resolve_dependency_inputs_uses_object_model_ref_when_path_edge_targets_model_ref() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "puma-lib".to_string(),
                    node_type: "puma-lib".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "runtime".to_string(),
                    node_type: "llm-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "puma-lib".to_string(),
                source_handle: "model_path".to_string(),
                target: "runtime".to_string(),
                target_handle: "pumas_model_ref".to_string(),
            }],
            groups: Vec::new(),
        };

        let model_ref = serde_json::json!({
            "source": "puma-lib",
            "status": "resolved",
            "model_id": "family/model"
        });
        let dependency_outputs = HashMap::from([(
            "puma-lib".to_string(),
            HashMap::from([
                (
                    "model_path".to_string(),
                    serde_json::json!("/tmp/model.gguf"),
                ),
                ("pumas_model_ref".to_string(), model_ref.clone()),
                ("model_id".to_string(), serde_json::json!("family/model")),
                ("backend_key".to_string(), serde_json::json!("llamacpp")),
                (
                    "recommended_backend".to_string(),
                    serde_json::json!("llamacpp"),
                ),
            ]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(inputs.get("model_path"), None);
        assert_eq!(inputs.get("pumas_model_ref"), Some(&model_ref));
        assert_eq!(
            inputs.get("model_id"),
            Some(&serde_json::json!("family/model"))
        );
        assert_eq!(inputs.get("backend_key"), None);
        assert_eq!(inputs.get("recommended_backend"), None);
    }

    #[test]
    fn resolve_dependency_inputs_rejects_path_shaped_model_ref_target() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "puma-lib".to_string(),
                    node_type: "puma-lib".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "runtime".to_string(),
                    node_type: "llm-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "puma-lib".to_string(),
                source_handle: "model_path".to_string(),
                target: "runtime".to_string(),
                target_handle: "pumas_model_ref".to_string(),
            }],
            groups: Vec::new(),
        };

        let dependency_outputs = HashMap::from([(
            "puma-lib".to_string(),
            HashMap::from([(
                "model_path".to_string(),
                serde_json::json!("/tmp/model.gguf"),
            )]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(inputs.get("pumas_model_ref"), None);
        assert_eq!(inputs.get("model_path"), None);
    }

    #[test]
    fn resolve_dependency_inputs_repairs_llm_stream_edge_to_text_output() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "llm".to_string(),
                    node_type: "llm-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "output".to_string(),
                    node_type: "text-output".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "llm".to_string(),
                source_handle: "stream".to_string(),
                target: "output".to_string(),
                target_handle: "stream".to_string(),
            }],
            groups: Vec::new(),
        };

        let dependency_outputs = HashMap::from([(
            "llm".to_string(),
            HashMap::from([("response".to_string(), serde_json::json!("generated text"))]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"output".to_string(), &dependency_outputs);

        assert_eq!(
            inputs.get("text"),
            Some(&serde_json::json!("generated text"))
        );
        assert!(inputs.get("stream").is_none());
    }

    #[test]
    fn resolve_dependency_inputs_rejects_package_facts_target_context() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![
                GraphNode {
                    id: "puma-lib".to_string(),
                    node_type: "puma-lib".to_string(),
                    data: serde_json::json!({}),
                    position: (0.0, 0.0),
                },
                GraphNode {
                    id: "runtime".to_string(),
                    node_type: "llm-inference".to_string(),
                    data: serde_json::json!({}),
                    position: (100.0, 0.0),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge".to_string(),
                source: "puma-lib".to_string(),
                source_handle: "resolved_model_package_facts".to_string(),
                target: "runtime".to_string(),
                target_handle: "resolved_model_package_facts".to_string(),
            }],
            groups: Vec::new(),
        };

        let package_facts = serde_json::json!({
            "package_facts_contract_version": 1,
            "model_ref": {
                "model_id": "family/model",
                "selected_artifact_path": "family/model/model.gguf"
            }
        });
        let dependency_outputs = HashMap::from([(
            "puma-lib".to_string(),
            HashMap::from([
                (
                    "pumas_model_ref".to_string(),
                    serde_json::json!({
                        "model_id": "family/model",
                        "selected_artifact_path": "family/model/model.gguf"
                    }),
                ),
                (
                    "resolved_model_package_facts".to_string(),
                    package_facts.clone(),
                ),
                ("model_id".to_string(), serde_json::json!("family/model")),
                (
                    "selected_binding_ids".to_string(),
                    serde_json::json!(["q4"]),
                ),
                (
                    "platform_context".to_string(),
                    serde_json::json!({"os": "linux", "arch": "x86_64"}),
                ),
                (
                    "dependency_bindings".to_string(),
                    serde_json::json!([{"id": "llamacpp"}]),
                ),
            ]),
        )]);

        let inputs = resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(inputs.get("resolved_model_package_facts"), None);
        assert_eq!(inputs.get("pumas_model_ref"), None);
        assert_eq!(inputs.get("model_id"), None);
        assert_eq!(inputs.get("selected_binding_ids"), None);
        assert_eq!(inputs.get("platform_context"), None);
        assert_eq!(inputs.get("dependency_bindings"), None);
    }
}
