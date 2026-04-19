use std::collections::HashMap;

use crate::core_executor::{
    human_input_auto_accept, human_input_default_value, human_input_prompt,
    human_input_response_value,
};
use crate::types::{NodeId, WorkflowGraph};

pub(super) fn prepare_node_inputs(
    graph: &WorkflowGraph,
    node_id: &NodeId,
    inputs: &mut HashMap<String, serde_json::Value>,
) -> Option<Option<String>> {
    let node = graph.find_node(node_id)?;

    if !node.data.is_null() {
        inputs.insert("_data".to_string(), node.data.clone());
    }

    inject_kv_cache_input_from_node_memory(inputs);

    unresolved_human_input_prompt(&node.node_type, inputs)
}

fn inject_kv_cache_input_from_node_memory(inputs: &mut HashMap<String, serde_json::Value>) {
    if inputs.contains_key("kv_cache_in") {
        return;
    }

    let Some(node_memory) = inputs.get("_node_memory") else {
        return;
    };
    let Some(status) = node_memory.get("status").and_then(|value| value.as_str()) else {
        return;
    };
    if status != "ready" {
        return;
    }
    let Some(indirect_reference) = node_memory.get("indirect_state_reference") else {
        return;
    };
    let Some(reference_kind) = indirect_reference
        .get("reference_kind")
        .and_then(|value| value.as_str())
    else {
        return;
    };
    if reference_kind != "kv_cache_handle" {
        return;
    }

    let Some(reference_id) = indirect_reference
        .get("reference_id")
        .and_then(|value| value.as_str())
    else {
        return;
    };
    let Some(inspection_metadata) = indirect_reference.get("inspection_metadata") else {
        return;
    };
    let Some(model_fingerprint) = inspection_metadata.get("model_fingerprint") else {
        return;
    };
    let Some(runtime_fingerprint) = inspection_metadata.get("runtime_fingerprint") else {
        return;
    };

    inputs.insert(
        "kv_cache_in".to_string(),
        serde_json::json!({
            "cache_id": reference_id,
            "compatibility": {
                "model_fingerprint": model_fingerprint.clone(),
                "runtime_fingerprint": runtime_fingerprint.clone(),
            }
        }),
    );
}

fn unresolved_human_input_prompt(
    node_type: &str,
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<Option<String>> {
    if node_type != "human-input" {
        return None;
    }

    if human_input_response_value(inputs).is_some() {
        return None;
    }

    if human_input_auto_accept(inputs) && human_input_default_value(inputs).is_some() {
        return None;
    }

    Some(human_input_prompt(inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{GraphNode, WorkflowGraph};

    #[test]
    fn prepare_node_inputs_injects_static_node_data() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "text".to_string(),
                node_type: "text-input".to_string(),
                data: serde_json::json!({"label": "Prompt"}),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let mut inputs = HashMap::new();

        let wait_prompt = prepare_node_inputs(&graph, &"text".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
        assert_eq!(
            inputs.get("_data"),
            Some(&serde_json::json!({"label": "Prompt"}))
        );
    }

    #[test]
    fn prepare_node_inputs_returns_wait_prompt_for_unanswered_human_input() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "approval".to_string(),
                node_type: "human-input".to_string(),
                data: serde_json::json!({
                    "prompt": "Approve deployment?"
                }),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let mut inputs = HashMap::new();

        let wait_prompt = prepare_node_inputs(&graph, &"approval".to_string(), &mut inputs);

        assert_eq!(wait_prompt, Some(Some("Approve deployment?".to_string())));
        assert!(inputs.contains_key("_data"));
    }

    #[test]
    fn prepare_node_inputs_skips_wait_prompt_when_response_present() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "approval".to_string(),
                node_type: "human-input".to_string(),
                data: serde_json::json!({
                    "prompt": "Approve deployment?"
                }),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let mut inputs =
            HashMap::from([("user_response".to_string(), serde_json::json!("approved"))]);

        let wait_prompt = prepare_node_inputs(&graph, &"approval".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
    }

    #[test]
    fn prepare_node_inputs_projects_kv_cache_handle_from_node_memory_when_missing() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "llm".to_string(),
                node_type: "llamacpp-inference".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let mut inputs = HashMap::from([(
            "_node_memory".to_string(),
            serde_json::json!({
                "status": "ready",
                "indirect_state_reference": {
                    "reference_kind": "kv_cache_handle",
                    "reference_id": "cache-1",
                    "restore_strategy": "rehydrate_before_resume",
                    "inspection_metadata": {
                        "source_port": "kv_cache_out",
                        "backend_key": "llamacpp",
                        "model_fingerprint": {
                            "model_id": "model-1",
                            "config_hash": "cfg-1",
                        },
                        "runtime_fingerprint": {
                            "runtime_id": "runtime-1",
                            "backend_key": "llamacpp",
                            "tokenizer_fingerprint": "tok-1",
                            "prompt_format_fingerprint": "prompt-1",
                            "runtime_build_fingerprint": "build-1",
                        }
                    }
                }
            }),
        )]);

        let wait_prompt = prepare_node_inputs(&graph, &"llm".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
        assert_eq!(
            inputs.get("kv_cache_in"),
            Some(&serde_json::json!({
                "cache_id": "cache-1",
                "compatibility": {
                    "model_fingerprint": {
                        "model_id": "model-1",
                        "config_hash": "cfg-1",
                    },
                    "runtime_fingerprint": {
                        "runtime_id": "runtime-1",
                        "backend_key": "llamacpp",
                        "tokenizer_fingerprint": "tok-1",
                        "prompt_format_fingerprint": "prompt-1",
                        "runtime_build_fingerprint": "build-1",
                    }
                }
            }))
        );
    }

    #[test]
    fn prepare_node_inputs_preserves_explicit_kv_cache_input() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "llm".to_string(),
                node_type: "llamacpp-inference".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let explicit_handle = serde_json::json!({
            "cache_id": "explicit-cache",
            "compatibility": {
                "model_fingerprint": {
                    "model_id": "model-explicit",
                    "config_hash": "cfg-explicit",
                },
                "runtime_fingerprint": {
                    "runtime_id": "runtime-explicit",
                    "backend_key": "llamacpp",
                    "tokenizer_fingerprint": "tok-explicit",
                    "prompt_format_fingerprint": "prompt-explicit",
                    "runtime_build_fingerprint": "build-explicit",
                }
            }
        });
        let mut inputs = HashMap::from([
            ("kv_cache_in".to_string(), explicit_handle.clone()),
            (
                "_node_memory".to_string(),
                serde_json::json!({
                    "indirect_state_reference": {
                        "reference_kind": "kv_cache_handle",
                        "reference_id": "cache-1",
                        "restore_strategy": "rehydrate_before_resume",
                        "inspection_metadata": {
                            "model_fingerprint": {
                                "model_id": "model-1",
                                "config_hash": "cfg-1",
                            },
                            "runtime_fingerprint": {
                                "runtime_id": "runtime-1",
                                "backend_key": "llamacpp",
                                "tokenizer_fingerprint": "tok-1",
                                "prompt_format_fingerprint": "prompt-1",
                                "runtime_build_fingerprint": "build-1",
                            }
                        }
                    }
                }),
            ),
        ]);

        let wait_prompt = prepare_node_inputs(&graph, &"llm".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
        assert_eq!(inputs.get("kv_cache_in"), Some(&explicit_handle));
    }

    #[test]
    fn prepare_node_inputs_skips_invalidated_node_memory_kv_reference() {
        let graph = WorkflowGraph {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            nodes: vec![GraphNode {
                id: "llm".to_string(),
                node_type: "llamacpp-inference".to_string(),
                data: serde_json::json!({}),
                position: (0.0, 0.0),
            }],
            edges: Vec::new(),
            groups: Vec::new(),
        };
        let mut inputs = HashMap::from([(
            "_node_memory".to_string(),
            serde_json::json!({
                "status": "invalidated",
                "indirect_state_reference": {
                    "reference_kind": "kv_cache_handle",
                    "reference_id": "cache-1",
                    "restore_strategy": "rehydrate_before_resume",
                    "inspection_metadata": {
                        "model_fingerprint": {
                            "model_id": "model-1",
                            "config_hash": "cfg-1",
                        },
                        "runtime_fingerprint": {
                            "runtime_id": "runtime-1",
                            "backend_key": "llamacpp",
                            "tokenizer_fingerprint": "tok-1",
                            "prompt_format_fingerprint": "prompt-1",
                            "runtime_build_fingerprint": "build-1",
                        }
                    }
                }
            }),
        )]);

        let wait_prompt = prepare_node_inputs(&graph, &"llm".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
        assert!(!inputs.contains_key("kv_cache_in"));
    }
}
