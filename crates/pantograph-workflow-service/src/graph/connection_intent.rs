use std::collections::{HashSet, VecDeque};

use super::effective_definition::{effective_node_definition, EffectiveDefinitionError};
use super::registry::NodeRegistry;
use super::types::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse, ConnectionRejection,
    ConnectionRejectionReason, ConnectionTargetAnchorCandidate, ConnectionTargetNodeCandidate,
    GraphEdge, GraphNode, InsertableNodeTypeCandidate, NodeDefinition, PortDefinition,
    WorkflowGraph,
};
use super::validation::validate_connection;

#[path = "connection_insert.rs"]
mod connection_insert;

pub use connection_insert::{
    insert_node_and_connect, insert_node_on_edge, preview_node_insert_on_edge,
    rejected_edge_insert_preview_response, rejected_insert_on_edge_response,
    rejected_insert_response,
};

struct ResolvedOutputAnchor<'a> {
    node: &'a GraphNode,
    port: PortDefinition,
}

struct ResolvedInputAnchor<'a> {
    node: &'a GraphNode,
    port: PortDefinition,
}

fn node_label(node: &GraphNode, definition: &NodeDefinition) -> String {
    node.data
        .get("label")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| definition.label.clone())
}

fn resolve_output_anchor<'a>(
    graph: &'a WorkflowGraph,
    registry: &'a NodeRegistry,
    anchor: &ConnectionAnchor,
) -> Result<ResolvedOutputAnchor<'a>, ConnectionRejection> {
    let node = graph
        .find_node(&anchor.node_id)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownSourceAnchor,
            message: format!("source node '{}' was not found", anchor.node_id),
        })?;
    let definition = effective_node_definition(node, registry).map_err(|error| match error {
        EffectiveDefinitionError::UnknownNodeType(node_type) => ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownSourceAnchor,
            message: format!("source node type '{}' is unknown", node_type),
        },
    })?;
    let port = definition
        .outputs
        .iter()
        .find(|port| port.id == anchor.port_id)
        .cloned()
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownSourceAnchor,
            message: format!(
                "source anchor '{}.{}' was not found",
                anchor.node_id, anchor.port_id
            ),
        })?;

    Ok(ResolvedOutputAnchor { node, port })
}

fn resolve_input_anchor<'a>(
    graph: &'a WorkflowGraph,
    registry: &'a NodeRegistry,
    anchor: &ConnectionAnchor,
) -> Result<ResolvedInputAnchor<'a>, ConnectionRejection> {
    let node = graph
        .find_node(&anchor.node_id)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownTargetAnchor,
            message: format!("target node '{}' was not found", anchor.node_id),
        })?;
    let definition = effective_node_definition(node, registry).map_err(|error| match error {
        EffectiveDefinitionError::UnknownNodeType(node_type) => ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownTargetAnchor,
            message: format!("target node type '{}' is unknown", node_type),
        },
    })?;
    let port = definition
        .inputs
        .iter()
        .find(|port| port.id == anchor.port_id)
        .cloned()
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownTargetAnchor,
            message: format!(
                "target anchor '{}.{}' was not found",
                anchor.node_id, anchor.port_id
            ),
        })?;

    Ok(ResolvedInputAnchor { node, port })
}

fn ensure_graph_revision(
    graph: &WorkflowGraph,
    graph_revision: &str,
) -> Result<(), ConnectionRejection> {
    let current_revision = graph.compute_fingerprint();
    if current_revision != graph_revision {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::StaleRevision,
            message: format!(
                "graph revision '{}' is stale; current revision is '{}'",
                graph_revision, current_revision
            ),
        });
    }

    Ok(())
}

fn resolve_edge<'a>(
    graph: &'a WorkflowGraph,
    edge_id: &str,
) -> Result<&'a GraphEdge, ConnectionRejection> {
    graph
        .edges
        .iter()
        .find(|edge| edge.id == edge_id)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownEdge,
            message: format!("edge '{}' was not found", edge_id),
        })
}

fn would_create_cycle(graph: &WorkflowGraph, source_node_id: &str, target_node_id: &str) -> bool {
    let mut queue = VecDeque::from([target_node_id.to_string()]);
    let mut visited = HashSet::new();

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if node_id == source_node_id {
            return true;
        }
        for edge in graph.outgoing_edges(&node_id) {
            queue.push_back(edge.target.clone());
        }
    }

    false
}

fn evaluate_connection(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    source_anchor: &ConnectionAnchor,
    target_anchor: &ConnectionAnchor,
) -> Result<(), ConnectionRejection> {
    let source = resolve_output_anchor(graph, registry, source_anchor)?;
    let target = resolve_input_anchor(graph, registry, target_anchor)?;

    if graph.edges.iter().any(|edge| {
        edge.source == source_anchor.node_id
            && edge.source_handle == source_anchor.port_id
            && edge.target == target_anchor.node_id
            && edge.target_handle == target_anchor.port_id
    }) {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::DuplicateConnection,
            message: format!(
                "connection '{}.{}' -> '{}.{}' already exists",
                source_anchor.node_id,
                source_anchor.port_id,
                target_anchor.node_id,
                target_anchor.port_id
            ),
        });
    }

    if source.node.id == target.node.id {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::SelfConnection,
            message: format!("node '{}' cannot connect to itself", source.node.id),
        });
    }

    if !target.port.multiple
        && graph
            .edges
            .iter()
            .any(|edge| edge.target == target.node.id && edge.target_handle == target.port.id)
    {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::TargetCapacityReached,
            message: format!(
                "target input '{}.{}' is already occupied",
                target.node.id, target.port.id
            ),
        });
    }

    if !validate_connection(&source.port.data_type, &target.port.data_type) {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::IncompatibleTypes,
            message: format!(
                "source type '{:?}' is not compatible with target type '{:?}'",
                source.port.data_type, target.port.data_type
            ),
        });
    }

    if would_create_cycle(graph, &source.node.id, &target.node.id) {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::CycleDetected,
            message: format!(
                "connection '{}.{}' -> '{}.{}' would create a cycle",
                source.node.id, source.port.id, target.node.id, target.port.id
            ),
        });
    }

    Ok(())
}

pub fn connection_candidates(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    source_anchor: ConnectionAnchor,
    requested_revision: Option<&str>,
) -> Result<ConnectionCandidatesResponse, ConnectionRejection> {
    let source = resolve_output_anchor(graph, registry, &source_anchor)?;
    let graph_revision = graph.compute_fingerprint();

    let mut compatible_nodes = Vec::new();
    for node in &graph.nodes {
        if node.id == source.node.id {
            continue;
        }

        let Ok(definition) = effective_node_definition(node, registry) else {
            continue;
        };

        let mut anchors = Vec::new();
        for port in &definition.inputs {
            let target_anchor = ConnectionAnchor {
                node_id: node.id.clone(),
                port_id: port.id.clone(),
            };
            if evaluate_connection(graph, registry, &source_anchor, &target_anchor).is_ok() {
                anchors.push(ConnectionTargetAnchorCandidate {
                    port_id: port.id.clone(),
                    port_label: port.label.clone(),
                    data_type: port.data_type.clone(),
                    multiple: port.multiple,
                });
            }
        }

        if !anchors.is_empty() {
            anchors.sort_by(|left, right| left.port_label.cmp(&right.port_label));
            compatible_nodes.push(ConnectionTargetNodeCandidate {
                node_id: node.id.clone(),
                node_type: node.node_type.clone(),
                node_label: node_label(node, &definition),
                position: node.position.clone(),
                anchors,
            });
        }
    }

    compatible_nodes.sort_by(|left, right| {
        left.node_label
            .cmp(&right.node_label)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });

    let mut insertable_node_types = registry
        .all_definitions()
        .into_iter()
        .filter_map(|definition| {
            let mut matching_input_port_ids = definition
                .inputs
                .iter()
                .filter(|port| validate_connection(&source.port.data_type, &port.data_type))
                .map(|port| port.id.clone())
                .collect::<Vec<_>>();
            if matching_input_port_ids.is_empty() {
                return None;
            }
            matching_input_port_ids.sort();
            Some(InsertableNodeTypeCandidate {
                node_type: definition.node_type,
                category: definition.category,
                label: definition.label,
                description: definition.description,
                matching_input_port_ids,
            })
        })
        .collect::<Vec<_>>();

    insertable_node_types.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.node_type.cmp(&right.node_type))
    });

    Ok(ConnectionCandidatesResponse {
        graph_revision: graph_revision.clone(),
        revision_matches: requested_revision.is_none_or(|value| value == graph_revision),
        source_anchor,
        compatible_nodes,
        insertable_node_types,
    })
}

pub fn commit_connection(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    graph_revision: &str,
    source_anchor: &ConnectionAnchor,
    target_anchor: &ConnectionAnchor,
) -> Result<(), ConnectionRejection> {
    ensure_graph_revision(graph, graph_revision)?;
    evaluate_connection(graph, registry, source_anchor, target_anchor)
}

pub fn rejected_commit_response(
    graph: &WorkflowGraph,
    rejection: ConnectionRejection,
) -> ConnectionCommitResponse {
    ConnectionCommitResponse {
        accepted: false,
        graph_revision: graph.compute_fingerprint(),
        graph: Some(graph.clone()),
        workflow_event: None,
        workflow_session_state: None,
        rejection: Some(rejection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeCategory, Position};

    fn text_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "source".into(),
                    node_type: "text-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({"label": "Source"}),
                },
                GraphNode {
                    id: "target".into(),
                    node_type: "text-output".into(),
                    position: Position { x: 100.0, y: 0.0 },
                    data: serde_json::json!({"label": "Target"}),
                },
                GraphNode {
                    id: "llm".into(),
                    node_type: "llm-inference".into(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: Vec::new(),
            derived_graph: None,
        }
    }

    fn expand_target_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "number".into(),
                    node_type: "number-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({"label": "Number Input"}),
                },
                GraphNode {
                    id: "expand".into(),
                    node_type: "expand-settings".into(),
                    position: Position { x: 160.0, y: 0.0 },
                    data: serde_json::json!({
                        "label": "Expand Settings",
                        "definition": {
                            "node_type": "expand-settings",
                            "inputs": [
                                {"id": "inference_settings", "label": "Inference Settings", "data_type": "json", "required": true, "multiple": false},
                                {"id": "temperature", "label": "Temperature", "data_type": "number", "required": false, "multiple": false}
                            ],
                            "outputs": [
                                {"id": "inference_settings", "label": "Inference Settings", "data_type": "json", "required": true, "multiple": false},
                                {"id": "temperature", "label": "Temperature", "data_type": "number", "required": false, "multiple": false}
                            ]
                        }
                    }),
                },
            ],
            edges: Vec::new(),
            derived_graph: None,
        }
    }

    #[test]
    fn connection_candidates_return_existing_nodes_and_insertable_types() {
        let registry = NodeRegistry::new();
        let response = connection_candidates(
            &text_graph(),
            &registry,
            ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            None,
        )
        .expect("candidate query should succeed");

        assert!(!response.graph_revision.is_empty());
        assert!(response.revision_matches);
        assert!(response
            .compatible_nodes
            .iter()
            .any(|node| node.node_id == "target"
                && node.anchors.iter().any(|port| port.port_id == "text")));
        assert!(response.insertable_node_types.iter().any(|node| {
            node.node_type == "llm-inference"
                && node.category == NodeCategory::Processing
                && node
                    .matching_input_port_ids
                    .iter()
                    .any(|port_id| port_id == "prompt")
        }));
    }

    #[test]
    fn connection_candidates_include_dynamic_expand_setting_inputs() {
        let registry = NodeRegistry::new();
        let response = connection_candidates(
            &expand_target_graph(),
            &registry,
            ConnectionAnchor {
                node_id: "number".into(),
                port_id: "value".into(),
            },
            None,
        )
        .expect("candidate query should succeed");

        assert!(response.compatible_nodes.iter().any(|node| {
            node.node_id == "expand"
                && node
                    .anchors
                    .iter()
                    .any(|port| port.port_id == "temperature")
        }));
    }

    #[test]
    fn commit_connection_accepts_dynamic_expand_setting_inputs() {
        let registry = NodeRegistry::new();
        let graph = expand_target_graph();
        let revision = graph.compute_fingerprint();

        let result = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "number".into(),
                port_id: "value".into(),
            },
            &ConnectionAnchor {
                node_id: "expand".into(),
                port_id: "temperature".into(),
            },
        );

        assert!(
            result.is_ok(),
            "dynamic expand input should accept number output"
        );
    }

    #[test]
    fn preview_node_insert_on_edge_returns_valid_bridge_for_llm() {
        let registry = NodeRegistry::new();
        let graph = text_graph_with_edge();
        let revision = graph.compute_fingerprint();

        let bridge = preview_node_insert_on_edge(
            &graph,
            &registry,
            &revision,
            "source-text-target-text",
            "llm-inference",
        )
        .expect("preview should find a valid bridge");

        assert_eq!(bridge.input_port_id, "prompt");
        assert_eq!(bridge.output_port_id, "response");
    }

    #[test]
    fn preview_node_insert_on_edge_rejects_node_without_valid_path() {
        let registry = NodeRegistry::new();
        let graph = boolean_graph_with_edge();
        let revision = graph.compute_fingerprint();

        let rejection = preview_node_insert_on_edge(
            &graph,
            &registry,
            &revision,
            "boolean-source-value-human-target-auto_accept",
            "human-input",
        )
        .expect_err("preview should reject nodes without a valid bridge");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::NoCompatibleInsertPath
        );
    }

    #[test]
    fn insert_node_on_edge_returns_two_replacement_edges() {
        let registry = NodeRegistry::new();
        let graph = text_graph_with_edge();
        let revision = graph.compute_fingerprint();

        let (inserted_node, incoming_edge, outgoing_edge, bridge) = insert_node_on_edge(
            &graph,
            &registry,
            &revision,
            "source-text-target-text",
            "llm-inference",
            &super::super::types::InsertNodePositionHint {
                position: Position { x: 50.0, y: 24.0 },
            },
        )
        .expect("edge insert should succeed");

        assert_eq!(inserted_node.node_type, "llm-inference");
        assert_eq!(bridge.input_port_id, "prompt");
        assert_eq!(bridge.output_port_id, "response");
        assert_eq!(incoming_edge.source, "source");
        assert_eq!(incoming_edge.target, inserted_node.id);
        assert_eq!(incoming_edge.target_handle, "prompt");
        assert_eq!(outgoing_edge.source, inserted_node.id);
        assert_eq!(outgoing_edge.target, "target");
        assert_eq!(outgoing_edge.source_handle, "response");
        assert_eq!(outgoing_edge.target_handle, "text");
    }

    #[test]
    fn insert_node_on_edge_rejects_stale_revision() {
        let registry = NodeRegistry::new();
        let graph = text_graph_with_edge();

        let rejection = insert_node_on_edge(
            &graph,
            &registry,
            "stale",
            "source-text-target-text",
            "llm-inference",
            &super::super::types::InsertNodePositionHint {
                position: Position { x: 50.0, y: 24.0 },
            },
        )
        .expect_err("stale revision should be rejected");

        assert_eq!(rejection.reason, ConnectionRejectionReason::StaleRevision);
    }

    fn text_graph_with_edge() -> WorkflowGraph {
        let mut graph = text_graph();
        graph.edges.push(GraphEdge {
            id: "source-text-target-text".into(),
            source: "source".into(),
            source_handle: "text".into(),
            target: "target".into(),
            target_handle: "text".into(),
        });
        graph
    }

    fn boolean_graph_with_edge() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "boolean-source".into(),
                    node_type: "boolean-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({"label": "Boolean Source"}),
                },
                GraphNode {
                    id: "human-target".into(),
                    node_type: "human-input".into(),
                    position: Position { x: 120.0, y: 0.0 },
                    data: serde_json::json!({"label": "Human Target"}),
                },
            ],
            edges: vec![GraphEdge {
                id: "boolean-source-value-human-target-auto_accept".into(),
                source: "boolean-source".into(),
                source_handle: "value".into(),
                target: "human-target".into(),
                target_handle: "auto_accept".into(),
            }],
            derived_graph: None,
        }
    }
}
