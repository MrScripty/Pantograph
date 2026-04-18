use std::collections::HashMap;

use crate::types::{NodeId, WorkflowGraph};

const MODEL_PATH_CONTEXT_KEYS: [&str; 9] = [
    "model_id",
    "model_type",
    "task_type_primary",
    "backend_key",
    "recommended_backend",
    "selected_binding_ids",
    "platform_context",
    "dependency_bindings",
    "dependency_requirements_id",
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

        if let Some(value) = dep_outputs.get(&edge.source_handle) {
            inputs.insert(edge.target_handle.clone(), value.clone());
        }

        if edge.target_handle == "model_path" {
            merge_model_path_context(&mut inputs, dep_outputs);
        }
    }

    inputs
}

fn merge_model_path_context(
    inputs: &mut HashMap<String, serde_json::Value>,
    dep_outputs: &HashMap<String, serde_json::Value>,
) {
    for context_key in MODEL_PATH_CONTEXT_KEYS {
        if inputs.contains_key(context_key) {
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

        let inputs =
            resolve_dependency_inputs(&graph, &"target".to_string(), &dependency_outputs);

        assert_eq!(inputs.get("input"), Some(&serde_json::json!("hello")));
    }

    #[test]
    fn resolve_dependency_inputs_merges_model_path_context() {
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
                ("model_path".to_string(), serde_json::json!("/tmp/model.gguf")),
                ("model_id".to_string(), serde_json::json!("family/model")),
                ("backend_key".to_string(), serde_json::json!("llamacpp")),
            ]),
        )]);

        let inputs =
            resolve_dependency_inputs(&graph, &"runtime".to_string(), &dependency_outputs);

        assert_eq!(
            inputs.get("model_path"),
            Some(&serde_json::json!("/tmp/model.gguf"))
        );
        assert_eq!(
            inputs.get("model_id"),
            Some(&serde_json::json!("family/model"))
        );
        assert_eq!(
            inputs.get("backend_key"),
            Some(&serde_json::json!("llamacpp"))
        );
    }
}
