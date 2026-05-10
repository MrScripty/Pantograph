use crate::{
    FileSystemWorkflowGraphStore, GraphEdge, GraphNode, Position, WorkflowGraph,
    WorkflowGraphDiagnosticCode, WorkflowGraphInspectionRequest, WorkflowGraphSaveRequest,
    WorkflowService,
};

fn stale_diffusion_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position::default(),
                data: serde_json::json!({ "text": "hello" }),
            },
            GraphNode {
                id: "diffusion".to_string(),
                node_type: "diffusion-inference".to_string(),
                position: Position { x: 100.0, y: 0.0 },
                data: serde_json::json!({ "prompt": "hello" }),
            },
        ],
        edges: vec![GraphEdge {
            id: "prompt-diffusion-prompt".to_string(),
            source: "prompt".to_string(),
            source_handle: "text".to_string(),
            target: "diffusion".to_string(),
            target_handle: "prompt".to_string(),
        }],
        derived_graph: None,
    }
}

#[test]
fn workflow_graph_inspect_loads_saved_stale_graph_with_backend_diagnostics() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = FileSystemWorkflowGraphStore::new(temp.path());
    let service = WorkflowService::new();
    let saved = service
        .workflow_graph_save(
            &store,
            WorkflowGraphSaveRequest {
                name: "stale-diffusion".to_string(),
                graph: stale_diffusion_graph(),
            },
        )
        .expect("save workflow");

    let first = service
        .workflow_graph_inspect(
            &store,
            WorkflowGraphInspectionRequest {
                path: saved.path.clone(),
                selected_node_id: Some("diffusion".to_string()),
            },
        )
        .expect("inspect saved workflow");
    let second = service
        .workflow_graph_inspect(
            &store,
            WorkflowGraphInspectionRequest {
                path: saved.path,
                selected_node_id: Some("diffusion".to_string()),
            },
        )
        .expect("repeat inspect saved workflow");

    assert_eq!(first, second);
    assert!(first
        .graph
        .nodes
        .iter()
        .any(|node| node.node_type == "diffusion-inference"));
    assert!(first.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowGraphDiagnosticCode::RetiredNodeType
            && diagnostic.node_id.as_deref() == Some("diffusion")
            && diagnostic.blocking_submission
    }));
    assert!(first.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowGraphDiagnosticCode::MissingTargetContract
            && diagnostic
                .details
                .get("target_node_type")
                .map(String::as_str)
                == Some("diffusion-inference")
    }));

    let selected = first.selected_node.expect("selected stale node");
    assert_eq!(selected.node.id, "diffusion");
    assert_eq!(selected.diagnostics.len(), 1);
    assert_eq!(
        selected.diagnostics[0].code,
        WorkflowGraphDiagnosticCode::RetiredNodeType
    );
}
