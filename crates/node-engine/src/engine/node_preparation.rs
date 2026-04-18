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

    unresolved_human_input_prompt(&node.node_type, inputs)
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
        assert_eq!(inputs.get("_data"), Some(&serde_json::json!({"label": "Prompt"})));
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
        let mut inputs = HashMap::from([(
            "user_response".to_string(),
            serde_json::json!("approved"),
        )]);

        let wait_prompt = prepare_node_inputs(&graph, &"approval".to_string(), &mut inputs);

        assert_eq!(wait_prompt, None);
    }
}
