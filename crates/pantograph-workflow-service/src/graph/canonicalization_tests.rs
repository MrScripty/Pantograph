use pantograph_node_contracts::{
    ContractUpgradeChange, ContractUpgradeOutcome, DiagnosticsLineagePolicy, PortKind,
};
use serde_json::json;

use super::super::registry::NodeRegistry;
use super::super::types::{GraphEdge, GraphNode, WorkflowGraph};
use super::{canonicalize_workflow_graph, canonicalize_workflow_graph_with_migrations};

#[test]
fn canonicalize_workflow_graph_migrates_legacy_system_prompt_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "system-prompt".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "prompt": "hello" }),
            },
            GraphNode {
                id: "target".to_string(),
                node_type: "llm-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: vec![GraphEdge {
            id: "prompt-prompt-target-prompt".to_string(),
            source: "prompt".to_string(),
            source_handle: "prompt".to_string(),
            target: "target".to_string(),
            target_handle: "prompt".to_string(),
        }],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let prompt_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "prompt")
        .expect("prompt node");
    assert_eq!(prompt_node.node_type, "text-input");
    assert_eq!(prompt_node.data["text"], json!("hello"));
    assert_eq!(canonical.edges[0].source_handle, "text");
    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "system-prompt");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert_eq!(
        record.diagnostics_lineage,
        DiagnosticsLineagePolicy::PreservePrimitiveLineage
    );
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "system-prompt" && to.as_str() == "text-input"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortIdChanged { from, to, kind, .. }
            if from.as_str() == "prompt"
                && to.as_str() == "text"
                && *kind == PortKind::Output
    )));
}

#[test]
fn canonicalize_workflow_graph_hydrates_expand_settings_and_passthrough_edges() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "source".to_string(),
                node_type: "model-provider".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({
                    "inference_settings": [
                        {
                            "key": "steps",
                            "label": "Steps",
                            "param_type": "Number",
                            "default": 30,
                        }
                    ]
                }),
            },
            GraphNode {
                id: "expand".to_string(),
                node_type: "expand-settings".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "diffusion".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
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
                id: "expand-settings-diffusion-settings".to_string(),
                source: "expand".to_string(),
                source_handle: "inference_settings".to_string(),
                target: "diffusion".to_string(),
                target_handle: "inference_settings".to_string(),
            },
        ],
        derived_graph: None,
    };

    let canonical = canonicalize_workflow_graph(graph, &registry);
    let expand_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "expand")
        .expect("expand node");
    let diffusion_node = canonical
        .nodes
        .iter()
        .find(|node| node.id == "diffusion")
        .expect("diffusion node");
    let expand_outputs = expand_node.data["definition"]["outputs"]
        .as_array()
        .expect("expand outputs");
    let diffusion_inputs = diffusion_node.data["definition"]["inputs"]
        .as_array()
        .expect("diffusion inputs");

    assert!(expand_outputs
        .iter()
        .any(|port| port["id"] == json!("steps")));
    assert!(diffusion_inputs
        .iter()
        .any(|port| port["id"] == json!("steps")));
    assert!(canonical.edges.iter().any(|edge| {
        edge.source == "expand"
            && edge.source_handle == "steps"
            && edge.target == "diffusion"
            && edge.target_handle == "steps"
    }));
}
