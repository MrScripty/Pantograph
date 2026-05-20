use super::*;
use crate::graph::{
    GraphEdge, GraphNode, Position, WorkflowGraphDiagnosticCode, WorkflowGraphDiagnosticScope,
    WorkflowGraphDiagnosticSeverity,
};

fn node(id: &str, node_type: &str) -> GraphNode {
    GraphNode {
        id: id.to_string(),
        node_type: node_type.to_string(),
        position: Position::default(),
        data: serde_json::json!({}),
    }
}

#[test]
fn contract_validation_reports_canonical_incompatible_edges() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "image".to_string(),
                node_type: "image-input".to_string(),
                position: Position::default(),
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "text".to_string(),
                node_type: "text-output".to_string(),
                position: Position::default(),
                data: serde_json::json!({}),
            },
        ],
        edges: vec![GraphEdge {
            id: "image-to-text".to_string(),
            source: "image".to_string(),
            source_handle: "image".to_string(),
            target: "text".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    };

    let errors = validate_workflow_graph_contract(&graph, &registry);

    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("source type 'Image' is not compatible"));
}

#[test]
fn contract_diagnostics_classify_unknown_and_retired_node_types() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            node("retired", "diffusion-inference"),
            node("unknown", "custom-missing-node"),
        ],
        edges: Vec::new(),
        derived_graph: None,
    };

    let diagnostics = validate_workflow_graph_contract_diagnostics(&graph, &registry);

    let retired = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.node_id.as_deref() == Some("retired"))
        .expect("retired diagnostic");
    assert_eq!(retired.code, WorkflowGraphDiagnosticCode::RetiredNodeType);
    assert_eq!(retired.severity, WorkflowGraphDiagnosticSeverity::Error);
    assert_eq!(retired.scope, WorkflowGraphDiagnosticScope::Node);
    assert_eq!(retired.node_type.as_deref(), Some("diffusion-inference"));
    assert!(retired.blocking_submission);
    assert_eq!(
        retired
            .details
            .get("replacement_node_type")
            .map(String::as_str),
        Some("llm-inference")
    );

    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.node_id.as_deref() == Some("unknown"))
        .expect("unknown diagnostic");
    assert_eq!(unknown.code, WorkflowGraphDiagnosticCode::UnknownNodeType);
    assert_eq!(unknown.node_type.as_deref(), Some("custom-missing-node"));
    assert!(unknown.blocking_submission);
}

#[test]
fn contract_diagnostics_classify_invalid_pumas_model_refs_without_live_lookup() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "puma-local-path".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position::default(),
                data: serde_json::json!({
                    "pumas_model_ref": {
                        "model_ref_contract_version": 1,
                        "model_id": "/models/tiny-sd"
                    }
                }),
            },
            GraphNode {
                id: "llm-invalid-shape".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position::default(),
                data: serde_json::json!({
                    "model_ref": {
                        "model_ref_contract_version": 1
                    }
                }),
            },
            GraphNode {
                id: "puma-valid".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position::default(),
                data: serde_json::json!({
                    "pumas_model_ref": {
                        "model_ref_contract_version": 1,
                        "model_id": "pumas://models/image/stable-diffusion/tiny-sd"
                    }
                }),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    };

    let diagnostics = validate_workflow_graph_contract_diagnostics(&graph, &registry);

    let local_path = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.node_id.as_deref() == Some("puma-local-path"))
        .expect("local path diagnostic");
    assert_eq!(
        local_path.code,
        WorkflowGraphDiagnosticCode::InvalidPumasModelReference
    );
    assert_eq!(local_path.scope, WorkflowGraphDiagnosticScope::Node);
    assert!(local_path.blocking_submission);
    assert_eq!(
        local_path.details.get("field_path").map(String::as_str),
        Some("data.pumas_model_ref")
    );

    let invalid_shape = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.node_id.as_deref() == Some("llm-invalid-shape"))
        .expect("invalid shape diagnostic");
    assert_eq!(
        invalid_shape.code,
        WorkflowGraphDiagnosticCode::InvalidPumasModelReference
    );
    assert_eq!(
        invalid_shape.details.get("field_path").map(String::as_str),
        Some("data.model_ref")
    );

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.node_id.as_deref() != Some("puma-valid")),
        "valid Pumas model refs should not become stale graph diagnostics"
    );
}

#[test]
fn contract_diagnostics_classify_missing_edge_endpoints_and_handles() {
    let registry = NodeRegistry::new();
    let graph = WorkflowGraph {
        nodes: vec![
            node("source", "text-input"),
            node("target", "text-output"),
            node("valid-llm", "llm-inference"),
        ],
        edges: vec![
            GraphEdge {
                id: "missing-source-node".to_string(),
                source: "missing".to_string(),
                source_handle: "text".to_string(),
                target: "target".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "missing-target-node".to_string(),
                source: "source".to_string(),
                source_handle: "text".to_string(),
                target: "missing".to_string(),
                target_handle: "text".to_string(),
            },
            GraphEdge {
                id: "missing-source-output".to_string(),
                source: "source".to_string(),
                source_handle: "missing_output".to_string(),
                target: "valid-llm".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "missing-target-input".to_string(),
                source: "source".to_string(),
                source_handle: "text".to_string(),
                target: "valid-llm".to_string(),
                target_handle: "missing_input".to_string(),
            },
        ],
        derived_graph: None,
    };

    let diagnostics = validate_workflow_graph_contract_diagnostics(&graph, &registry);
    let code_for_edge = |edge_id: &str| {
        diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.details.get("edge_id").map(String::as_str) == Some(edge_id)
            })
            .map(|diagnostic| diagnostic.code)
    };

    assert_eq!(
        code_for_edge("missing-source-node"),
        Some(WorkflowGraphDiagnosticCode::MissingEdgeSourceNode)
    );
    assert_eq!(
        code_for_edge("missing-target-node"),
        Some(WorkflowGraphDiagnosticCode::MissingEdgeTargetNode)
    );
    assert_eq!(
        code_for_edge("missing-source-output"),
        Some(WorkflowGraphDiagnosticCode::MissingSourceOutput)
    );
    assert_eq!(
        code_for_edge("missing-target-input"),
        Some(WorkflowGraphDiagnosticCode::MissingTargetInput)
    );
}
