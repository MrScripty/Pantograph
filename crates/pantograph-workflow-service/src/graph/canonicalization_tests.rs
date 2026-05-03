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
fn canonicalize_workflow_graph_migrates_legacy_ollama_nodes() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model".to_string(),
                node_type: "model-provider".to_string(),
                position: super::super::types::Position { x: 0.0, y: 0.0 },
                data: json!({ "model_name": "llama3:8b" }),
            },
            GraphNode {
                id: "ollama".to_string(),
                node_type: "ollama-inference".to_string(),
                position: super::super::types::Position { x: 100.0, y: 0.0 },
                data: json!({
                    "model": "llama3:8b",
                    "temperature": 0.2,
                    "max_tokens": 128,
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "model-ref-output".to_string(),
                node_type: "text-output".to_string(),
                position: super::super::types::Position { x: 200.0, y: 100.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "model-name-ollama-model".to_string(),
                source: "model".to_string(),
                source_handle: "model_name".to_string(),
                target: "ollama".to_string(),
                target_handle: "model".to_string(),
            },
            GraphEdge {
                id: "ollama-response-output-text".to_string(),
                source: "ollama".to_string(),
                source_handle: "response".to_string(),
                target: "output".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "ollama-model-ref-output-text".to_string(),
                source: "ollama".to_string(),
                source_handle: "model_ref".to_string(),
                target: "model-ref-output".to_string(),
                target_handle: "text".to_string(),
            },
        ],
        derived_graph: None,
    };

    let result = canonicalize_workflow_graph_with_migrations(graph, &registry);
    let canonical = result.graph;
    let migrated = canonical
        .nodes
        .iter()
        .find(|node| node.id == "ollama")
        .expect("migrated ollama node");

    assert_eq!(migrated.node_type, "llm-inference");
    assert_eq!(migrated.data["task_kind"], json!("text_generation"));
    assert_eq!(migrated.data["runtime_hint"], json!("retired_ollama"));
    assert_eq!(
        migrated.data["pumas_model_ref"]["status"],
        json!("unresolved")
    );
    assert_eq!(
        migrated.data["pumas_model_ref"]["legacy_model"],
        json!("llama3:8b")
    );
    assert_eq!(
        migrated.data["migration_diagnostics"][0]["code"],
        json!("legacy_ollama_backend_retired")
    );
    assert!(canonical.edges.iter().any(|edge| {
        edge.id == "ollama-response-output-text"
            && edge.source_handle == "response"
            && edge.target_handle == "text"
    }));
    assert!(!canonical
        .edges
        .iter()
        .any(|edge| edge.target == "ollama" && edge.target_handle == "model"));
    assert!(!canonical
        .edges
        .iter()
        .any(|edge| edge.source == "ollama" && edge.source_handle == "model_ref"));

    assert_eq!(result.migration_records.len(), 1);
    let record = &result.migration_records[0];
    assert_eq!(record.node_type.as_str(), "ollama-inference");
    assert_eq!(record.outcome, ContractUpgradeOutcome::Upgraded);
    assert_eq!(
        record.diagnostics_lineage,
        DiagnosticsLineagePolicy::RejectToAvoidSilentChange
    );
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::NodeTypeChanged { from, to, .. }
            if from.as_str() == "ollama-inference" && to.as_str() == "llm-inference"
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model" && *kind == PortKind::Input
    )));
    assert!(record.changes.iter().any(|change| matches!(
        change,
        ContractUpgradeChange::PortRemoved { port_id, kind, .. }
            if port_id.as_str() == "model_ref" && *kind == PortKind::Output
    )));
    assert!(record
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.message.contains("Ollama execution is retired")));
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
