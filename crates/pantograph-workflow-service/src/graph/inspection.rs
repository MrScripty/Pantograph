use serde::{Deserialize, Serialize};

use super::contract_validation::validate_workflow_graph_contract_diagnostics;
use super::diagnostics::WorkflowGraphDiagnostic;
use super::registry::NodeRegistry;
use super::types::{GraphNode, WorkflowGraph};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInspectionRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInspectionRunContext {
    pub workflow_run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInspectionSelectedNode {
    pub node: GraphNode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowGraphDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowGraphInspectionProjection {
    pub graph: WorkflowGraph,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_node: Option<WorkflowGraphInspectionSelectedNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowGraphDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_context: Option<WorkflowGraphInspectionRunContext>,
}

pub fn inspect_workflow_graph(
    graph: WorkflowGraph,
    registry: &NodeRegistry,
    selected_node_id: Option<&str>,
) -> WorkflowGraphInspectionProjection {
    inspect_workflow_graph_with_run_context(graph, registry, selected_node_id, None)
}

pub fn inspect_workflow_graph_with_run_context(
    graph: WorkflowGraph,
    registry: &NodeRegistry,
    selected_node_id: Option<&str>,
    run_context: Option<WorkflowGraphInspectionRunContext>,
) -> WorkflowGraphInspectionProjection {
    let diagnostics = validate_workflow_graph_contract_diagnostics(&graph, registry);
    let selected_node = selected_node_id.and_then(|node_id| {
        graph
            .find_node(node_id)
            .cloned()
            .map(|node| WorkflowGraphInspectionSelectedNode {
                diagnostics: diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.node_id.as_deref() == Some(node_id))
                    .cloned()
                    .collect(),
                node,
            })
    });

    WorkflowGraphInspectionProjection {
        graph,
        selected_node,
        diagnostics,
        run_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, Position, WorkflowGraphDiagnosticCode};

    fn graph_with_retired_node() -> WorkflowGraph {
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
                id: "prompt-diffusion".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "diffusion".to_string(),
                target_handle: "prompt".to_string(),
            }],
            derived_graph: None,
        }
    }

    #[test]
    fn inspection_projection_returns_stable_stale_graph_diagnostics() {
        let registry = NodeRegistry::new();
        let graph = graph_with_retired_node();

        let first = inspect_workflow_graph(graph.clone(), &registry, Some("diffusion"));
        let second = inspect_workflow_graph(graph, &registry, Some("diffusion"));

        assert_eq!(first, second);
        assert_eq!(first.graph.nodes.len(), 2);
        assert_eq!(
            first
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code)
                .collect::<Vec<_>>(),
            vec![
                WorkflowGraphDiagnosticCode::RetiredNodeType,
                WorkflowGraphDiagnosticCode::MissingTargetContract,
            ]
        );
        let selected = first.selected_node.expect("selected node projection");
        assert_eq!(selected.node.id, "diffusion");
        assert_eq!(selected.diagnostics.len(), 1);
        assert_eq!(
            selected.diagnostics[0].code,
            WorkflowGraphDiagnosticCode::RetiredNodeType
        );
    }

    #[test]
    fn inspection_projection_serializes_without_frontend_inference_fields() {
        let registry = NodeRegistry::new();
        let projection = inspect_workflow_graph_with_run_context(
            graph_with_retired_node(),
            &registry,
            Some("diffusion"),
            Some(WorkflowGraphInspectionRunContext {
                workflow_run_id: "run-1".to_string(),
                workflow_id: Some("workflow-1".to_string()),
            }),
        );

        let encoded = serde_json::to_value(&projection).expect("encode projection");

        assert_eq!(encoded["run_context"]["workflow_run_id"], "run-1");
        assert_eq!(
            encoded["selected_node"]["diagnostics"][0]["scope"],
            serde_json::json!("node")
        );
        assert!(encoded["diagnostics"]
            .as_array()
            .is_some_and(|diagnostics| {
                diagnostics.iter().any(|diagnostic| {
                    diagnostic["code"] == serde_json::json!("retired_node_type")
                        && diagnostic["scope"] == serde_json::json!("node")
                })
            }));
    }
}
