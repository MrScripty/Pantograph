use crate::{GraphEdge, GraphNode, Position, WorkflowGraph};

use super::*;

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "input".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({"value": "first"}),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({"name": "Output"}),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge".to_string(),
            source: "input".to_string(),
            source_handle: "text".to_string(),
            target: "output".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    }
}

#[test]
fn resolve_workflow_graph_version_reuses_same_executable_fingerprint() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let first = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("first version");
    let second = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("reused version");

    assert_eq!(first.workflow_version_id, second.workflow_version_id);
    assert_eq!(first.semantic_version, "1.0.0");
    assert!(first
        .execution_fingerprint
        .starts_with("workflow-exec-blake3:"));
}

#[test]
fn resolve_workflow_graph_version_rejects_semantic_version_conflict() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("first version");

    let mut changed_graph = graph();
    changed_graph.edges[0].target_handle = "other-port".to_string();
    let err = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &changed_graph)
        .expect_err("semantic version conflict");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("semantic version"))
    );
}
