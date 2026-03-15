//! Interactive connection intent evaluation for workflow editing sessions.
//!
//! This module centralizes connection eligibility so GUI and headless editing
//! clients can discover candidates and commit edges using the same rules.

use std::collections::{HashSet, VecDeque};

use serde_json::{Map, Value};
use uuid::Uuid;

use super::effective_definition::{effective_node_definition, EffectiveDefinitionError};
use super::registry::NodeRegistry;
use super::types::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse, ConnectionRejection,
    ConnectionRejectionReason, ConnectionTargetAnchorCandidate, ConnectionTargetNodeCandidate,
    GraphEdge, GraphNode, InsertNodeConnectionResponse, InsertNodePositionHint,
    InsertableNodeTypeCandidate, NodeDefinition, PortDefinition, WorkflowGraph,
};
use super::validation::validate_connection;

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
                node_type: definition.node_type.clone(),
                category: definition.category.clone(),
                label: definition.label.clone(),
                description: definition.description.clone(),
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
    let current_revision = graph.compute_fingerprint();
    if graph_revision != current_revision {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::StaleRevision,
            message: format!(
                "graph revision '{}' is stale; current revision is '{}'",
                graph_revision, current_revision
            ),
        });
    }

    evaluate_connection(graph, registry, source_anchor, target_anchor)
}

pub fn rejected_commit_response(
    graph: &WorkflowGraph,
    rejection: ConnectionRejection,
) -> ConnectionCommitResponse {
    ConnectionCommitResponse {
        accepted: false,
        graph_revision: graph.compute_fingerprint(),
        graph: None,
        rejection: Some(rejection),
    }
}

fn build_insert_node_data(definition: &NodeDefinition) -> Value {
    let mut data = Map::new();
    data.insert("label".to_string(), Value::String(definition.label.clone()));
    for input in &definition.inputs {
        data.insert(input.id.clone(), Value::Null);
    }
    Value::Object(data)
}

fn resolve_insert_definition<'a>(
    registry: &'a NodeRegistry,
    node_type: &str,
) -> Result<&'a NodeDefinition, ConnectionRejection> {
    registry
        .get_definition(node_type)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownInsertNodeType,
            message: format!("insertable node type '{}' is unknown", node_type),
        })
}

fn resolve_insert_target_anchor(
    source_port: &PortDefinition,
    definition: &NodeDefinition,
    preferred_input_port_id: Option<&str>,
) -> Result<String, ConnectionRejection> {
    if let Some(preferred) = preferred_input_port_id {
        if definition.inputs.iter().any(|port| {
            port.id == preferred && validate_connection(&source_port.data_type, &port.data_type)
        }) {
            return Ok(preferred.to_string());
        }
    }

    definition
        .inputs
        .iter()
        .filter(|port| validate_connection(&source_port.data_type, &port.data_type))
        .min_by(|left, right| {
            left.label
                .cmp(&right.label)
                .then_with(|| left.id.cmp(&right.id))
        })
        .map(|port| port.id.clone())
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::NoCompatibleInsertInput,
            message: format!(
                "node type '{}' has no compatible input for source type '{:?}'",
                definition.node_type, source_port.data_type
            ),
        })
}

pub fn insert_node_and_connect(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    graph_revision: &str,
    source_anchor: &ConnectionAnchor,
    node_type: &str,
    position_hint: &InsertNodePositionHint,
    preferred_input_port_id: Option<&str>,
) -> Result<(GraphNode, GraphEdge), ConnectionRejection> {
    let current_revision = graph.compute_fingerprint();
    if graph_revision != current_revision {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::StaleRevision,
            message: format!(
                "graph revision '{}' is stale; current revision is '{}'",
                graph_revision, current_revision
            ),
        });
    }

    let source = resolve_output_anchor(graph, registry, source_anchor)?;
    let definition = resolve_insert_definition(registry, node_type)?;
    let target_port_id =
        resolve_insert_target_anchor(&source.port, definition, preferred_input_port_id)?;

    let inserted_node = GraphNode {
        id: format!("{}-{}", definition.node_type, Uuid::new_v4()),
        node_type: definition.node_type.clone(),
        position: position_hint.position.clone(),
        data: build_insert_node_data(definition),
    };
    let inserted_edge = GraphEdge {
        id: format!(
            "{}-{}-{}-{}",
            source_anchor.node_id, source_anchor.port_id, inserted_node.id, target_port_id
        ),
        source: source_anchor.node_id.clone(),
        source_handle: source_anchor.port_id.clone(),
        target: inserted_node.id.clone(),
        target_handle: target_port_id,
    };

    Ok((inserted_node, inserted_edge))
}

pub fn rejected_insert_response(
    graph: &WorkflowGraph,
    rejection: ConnectionRejection,
) -> InsertNodeConnectionResponse {
    InsertNodeConnectionResponse {
        accepted: false,
        graph_revision: graph.compute_fingerprint(),
        inserted_node_id: None,
        graph: None,
        rejection: Some(rejection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::types::{GraphEdge, InsertNodePositionHint, NodeCategory, Position};

    fn graph() -> WorkflowGraph {
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
            &graph(),
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
    fn commit_connection_rejects_stale_revision() {
        let registry = NodeRegistry::new();
        let graph = graph();
        let rejection = commit_connection(
            &graph,
            &registry,
            "stale",
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            &ConnectionAnchor {
                node_id: "target".into(),
                port_id: "text".into(),
            },
        )
        .expect_err("stale revision should be rejected");

        assert_eq!(rejection.reason, ConnectionRejectionReason::StaleRevision);
    }

    #[test]
    fn commit_connection_rejects_occupied_single_input() {
        let registry = NodeRegistry::new();
        let mut graph = graph();
        graph.edges.push(GraphEdge {
            id: "existing".into(),
            source: "source".into(),
            source_handle: "text".into(),
            target: "target".into(),
            target_handle: "text".into(),
        });
        let revision = graph.compute_fingerprint();

        let rejection = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "llm".into(),
                port_id: "response".into(),
            },
            &ConnectionAnchor {
                node_id: "target".into(),
                port_id: "text".into(),
            },
        )
        .expect_err("occupied input should be rejected");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::TargetCapacityReached
        );
    }

    #[test]
    fn commit_connection_rejects_cycles() {
        let registry = NodeRegistry::new();
        let mut graph = graph();
        graph.edges.push(GraphEdge {
            id: "e1".into(),
            source: "source".into(),
            source_handle: "text".into(),
            target: "llm".into(),
            target_handle: "prompt".into(),
        });
        graph.edges.push(GraphEdge {
            id: "e2".into(),
            source: "llm".into(),
            source_handle: "response".into(),
            target: "target".into(),
            target_handle: "text".into(),
        });
        let revision = graph.compute_fingerprint();

        let rejection = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "target".into(),
                port_id: "text".into(),
            },
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
        )
        .expect_err("cycle should be rejected");

        assert_eq!(rejection.reason, ConnectionRejectionReason::CycleDetected);
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

        assert!(result.is_ok(), "dynamic expand input should accept number output");
    }

    #[test]
    fn diffusion_image_can_connect_to_image_output() {
        let registry = NodeRegistry::new();
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "diffusion".into(),
                    node_type: "diffusion-inference".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "image-output".into(),
                    node_type: "image-output".into(),
                    position: Position { x: 240.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: Vec::new(),
            derived_graph: None,
        };

        let response = connection_candidates(
            &graph,
            &registry,
            ConnectionAnchor {
                node_id: "diffusion".into(),
                port_id: "image".into(),
            },
            None,
        )
        .expect("candidate query should succeed");

        assert!(response.compatible_nodes.iter().any(|node| {
            node.node_id == "image-output"
                && node.anchors.iter().any(|port| port.port_id == "image")
        }));

        let revision = graph.compute_fingerprint();
        let result = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "diffusion".into(),
                port_id: "image".into(),
            },
            &ConnectionAnchor {
                node_id: "image-output".into(),
                port_id: "image".into(),
            },
        );

        assert!(result.is_ok(), "image output should accept diffusion image");
    }

    #[test]
    fn insert_node_and_connect_returns_inserted_node_and_edge() {
        let registry = NodeRegistry::new();
        let graph = graph();
        let revision = graph.compute_fingerprint();

        let (node, edge) = insert_node_and_connect(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            "llm-inference",
            &InsertNodePositionHint {
                position: Position { x: 140.0, y: 24.0 },
            },
            Some("prompt"),
        )
        .expect("compatible insert should succeed");

        assert_eq!(node.node_type, "llm-inference");
        assert_eq!(node.position, Position { x: 140.0, y: 24.0 });
        assert_eq!(edge.source, "source");
        assert_eq!(edge.source_handle, "text");
        assert_eq!(edge.target, node.id);
        assert_eq!(edge.target_handle, "prompt");
    }

    #[test]
    fn insert_node_and_connect_rejects_stale_revision() {
        let registry = NodeRegistry::new();
        let graph = graph();
        let rejection = insert_node_and_connect(
            &graph,
            &registry,
            "stale",
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            "llm-inference",
            &InsertNodePositionHint {
                position: Position { x: 0.0, y: 0.0 },
            },
            None,
        )
        .expect_err("stale revision should reject insert");

        assert_eq!(rejection.reason, ConnectionRejectionReason::StaleRevision);
    }

    #[test]
    fn insert_node_and_connect_rejects_unknown_node_type() {
        let registry = NodeRegistry::new();
        let graph = graph();
        let revision = graph.compute_fingerprint();

        let rejection = insert_node_and_connect(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            "missing-node",
            &InsertNodePositionHint {
                position: Position { x: 0.0, y: 0.0 },
            },
            None,
        )
        .expect_err("unknown insert type should be rejected");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::UnknownInsertNodeType
        );
    }

    #[test]
    fn insert_node_and_connect_rejects_without_compatible_input() {
        let registry = NodeRegistry::new();
        let graph = graph();
        let revision = graph.compute_fingerprint();

        let rejection = insert_node_and_connect(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            "number-input",
            &InsertNodePositionHint {
                position: Position { x: 0.0, y: 0.0 },
            },
            None,
        )
        .expect_err("insert without compatible input should reject");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::NoCompatibleInsertInput
        );
    }
}
