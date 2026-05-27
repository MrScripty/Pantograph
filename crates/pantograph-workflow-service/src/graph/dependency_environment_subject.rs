use pantograph_inference_interface_contracts::{InferenceDiagnosticCode, WorkflowNodeId};

use super::types::WorkflowGraph;

pub(crate) const DEPENDENCY_ENVIRONMENT_NODE_TYPE: &str = "dependency-environment";
pub(crate) const INFERENCE_NODE_TYPE: &str = "llm-inference";
pub(crate) const DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID: &str = "dependency_environment_sidecar";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DependencyEnvironmentActionSubjectResolution {
    Resolved {
        inference_node_id: WorkflowNodeId,
    },
    Blocked {
        code: InferenceDiagnosticCode,
        message: &'static str,
        hint: Option<&'static str>,
    },
}

impl DependencyEnvironmentActionSubjectResolution {
    pub(crate) fn resolved(inference_node_id: WorkflowNodeId) -> Self {
        Self::Resolved { inference_node_id }
    }
}

pub(crate) fn resolve_dependency_environment_action_subject(
    graph: &WorkflowGraph,
    target_node_id: &WorkflowNodeId,
) -> DependencyEnvironmentActionSubjectResolution {
    let Some(target_node) = graph
        .nodes
        .iter()
        .find(|node| node.id == target_node_id.as_str())
    else {
        return blocked(
            InferenceDiagnosticCode::TargetNodeMissing,
            "Dependency environment action target node does not exist in the current graph.",
            None,
        );
    };

    if target_node.node_type != DEPENDENCY_ENVIRONMENT_NODE_TYPE {
        return blocked(
            InferenceDiagnosticCode::DependencySidecarTargetWrongType,
            "Dependency environment actions must target a dependency-environment node.",
            Some("Send the action intent for the dependency-environment sidecar node."),
        );
    }

    let sidecar_edges = graph
        .edges
        .iter()
        .filter(|edge| {
            edge.source == target_node_id.as_str()
                && edge.source_handle == DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID
                && edge.target_handle == DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID
        })
        .collect::<Vec<_>>();

    if sidecar_edges.is_empty() {
        let has_invalid_sidecar_edge = graph.edges.iter().any(|edge| {
            edge.source == target_node_id.as_str()
                && (edge.source_handle == DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID
                    || edge.target_handle == DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID)
        });
        if has_invalid_sidecar_edge {
            return blocked(
                InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
                "Dependency environment sidecar association uses the wrong handles.",
                Some("Connect dependency_environment_sidecar to dependency_environment_sidecar."),
            );
        }
        return blocked(
            InferenceDiagnosticCode::DependencySidecarAssociationMissing,
            "Dependency environment node is not associated with an inference node.",
            Some("Connect the dependency environment sidecar output to one inference node."),
        );
    }

    if sidecar_edges.len() > 1 {
        return blocked(
            InferenceDiagnosticCode::DependencySidecarAssociationDuplicate,
            "Dependency environment node is associated with more than one inference node.",
            Some("Keep exactly one dependency environment sidecar association."),
        );
    }

    let edge = sidecar_edges[0];
    let Some(inference_node) = graph.nodes.iter().find(|node| node.id == edge.target) else {
        return blocked(
            InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
            "Dependency environment sidecar association points to a missing inference node.",
            None,
        );
    };

    if inference_node.node_type != INFERENCE_NODE_TYPE {
        return blocked(
            InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
            "Dependency environment sidecar association must target an inference node.",
            Some("Connect the sidecar association to a canonical llm-inference node."),
        );
    }

    match edge.target.parse::<WorkflowNodeId>() {
        Ok(inference_node_id) => {
            DependencyEnvironmentActionSubjectResolution::resolved(inference_node_id)
        }
        Err(_) => blocked(
            InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
            "Dependency environment sidecar association has an invalid inference node id.",
            None,
        ),
    }
}

fn blocked(
    code: InferenceDiagnosticCode,
    message: &'static str,
    hint: Option<&'static str>,
) -> DependencyEnvironmentActionSubjectResolution {
    DependencyEnvironmentActionSubjectResolution::Blocked {
        code,
        message,
        hint,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};

    #[test]
    fn resolves_exact_sidecar_association_to_inference_node() {
        let graph = graph_with_edges(vec![sidecar_edge("edge-1", "dep", "infer")]);

        let result = resolve_dependency_environment_action_subject(&graph, &node_id("dep"));

        assert_eq!(
            result,
            DependencyEnvironmentActionSubjectResolution::resolved(node_id("infer"))
        );
    }

    #[test]
    fn rejects_wrong_action_target_type() {
        let graph = graph_with_edges(Vec::new());

        let result = resolve_dependency_environment_action_subject(&graph, &node_id("infer"));

        assert!(matches!(
            result,
            DependencyEnvironmentActionSubjectResolution::Blocked {
                code: InferenceDiagnosticCode::DependencySidecarTargetWrongType,
                ..
            }
        ));
    }

    #[test]
    fn rejects_missing_sidecar_association() {
        let graph = graph_with_edges(Vec::new());

        let result = resolve_dependency_environment_action_subject(&graph, &node_id("dep"));

        assert!(matches!(
            result,
            DependencyEnvironmentActionSubjectResolution::Blocked {
                code: InferenceDiagnosticCode::DependencySidecarAssociationMissing,
                ..
            }
        ));
    }

    #[test]
    fn rejects_duplicate_sidecar_association() {
        let graph = graph_with_edges(vec![
            sidecar_edge("edge-1", "dep", "infer"),
            sidecar_edge("edge-2", "dep", "infer-2"),
        ]);

        let result = resolve_dependency_environment_action_subject(&graph, &node_id("dep"));

        assert!(matches!(
            result,
            DependencyEnvironmentActionSubjectResolution::Blocked {
                code: InferenceDiagnosticCode::DependencySidecarAssociationDuplicate,
                ..
            }
        ));
    }

    #[test]
    fn rejects_wrong_sidecar_handle() {
        let graph = graph_with_edges(vec![GraphEdge {
            id: "edge-1".to_string(),
            source: "dep".to_string(),
            source_handle: DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID.to_string(),
            target: "infer".to_string(),
            target_handle: "pumas_model_ref".to_string(),
        }]);

        let result = resolve_dependency_environment_action_subject(&graph, &node_id("dep"));

        assert!(matches!(
            result,
            DependencyEnvironmentActionSubjectResolution::Blocked {
                code: InferenceDiagnosticCode::DependencySidecarAssociationInvalid,
                ..
            }
        ));
    }

    fn graph_with_edges(edges: Vec<GraphEdge>) -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                graph_node("dep", DEPENDENCY_ENVIRONMENT_NODE_TYPE),
                graph_node("infer", INFERENCE_NODE_TYPE),
                graph_node("infer-2", INFERENCE_NODE_TYPE),
            ],
            edges,
            derived_graph: None,
        }
    }

    fn graph_node(id: &str, node_type: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: node_type.to_string(),
            position: Position::default(),
            data: serde_json::Value::Null,
        }
    }

    fn sidecar_edge(id: &str, source: &str, target: &str) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            source_handle: DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID.to_string(),
            target: target.to_string(),
            target_handle: DEPENDENCY_ENVIRONMENT_SIDECAR_PORT_ID.to_string(),
        }
    }

    fn node_id(value: &str) -> WorkflowNodeId {
        value.parse().expect("valid workflow node id")
    }
}
