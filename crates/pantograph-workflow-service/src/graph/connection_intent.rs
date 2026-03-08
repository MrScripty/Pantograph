use std::collections::{HashSet, VecDeque};

use serde_json::{Map, Value};
use uuid::Uuid;

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
    port: &'a PortDefinition,
}

struct ResolvedInputAnchor<'a> {
    node: &'a GraphNode,
    port: &'a PortDefinition,
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
    let definition =
        registry
            .get_definition(&node.node_type)
            .ok_or_else(|| ConnectionRejection {
                reason: ConnectionRejectionReason::UnknownSourceAnchor,
                message: format!("source node type '{}' is unknown", node.node_type),
            })?;
    let port = definition
        .outputs
        .iter()
        .find(|port| port.id == anchor.port_id)
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
    let definition =
        registry
            .get_definition(&node.node_type)
            .ok_or_else(|| ConnectionRejection {
                reason: ConnectionRejectionReason::UnknownTargetAnchor,
                message: format!("target node type '{}' is unknown", node.node_type),
            })?;
    let port = definition
        .inputs
        .iter()
        .find(|port| port.id == anchor.port_id)
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

        let Some(definition) = registry.get_definition(&node.node_type) else {
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
                node_label: node_label(node, definition),
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
        revision_matches: requested_revision.map_or(true, |value| value == graph_revision),
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
    if current_revision != graph_revision {
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
        graph: Some(graph.clone()),
        rejection: Some(rejection),
    }
}

fn next_inserted_node_label(graph: &WorkflowGraph, definition: &NodeDefinition) -> String {
    let prefix = definition.label.clone();
    let existing = graph
        .nodes
        .iter()
        .filter(|node| node.node_type == definition.node_type)
        .count();
    if existing == 0 {
        prefix
    } else {
        format!("{} {}", prefix, existing + 1)
    }
}

fn default_node_data(definition: &NodeDefinition, label: String) -> Value {
    let mut map = Map::new();
    map.insert("label".to_string(), Value::String(label));
    for input in &definition.inputs {
        if input.required && matches!(input.data_type, super::types::PortDataType::Boolean) {
            map.insert(input.id.clone(), Value::Bool(false));
        }
    }
    Value::Object(map)
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
    if current_revision != graph_revision {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::StaleRevision,
            message: format!(
                "graph revision '{}' is stale; current revision is '{}'",
                graph_revision, current_revision
            ),
        });
    }

    let source = resolve_output_anchor(graph, registry, source_anchor)?;
    let definition = registry
        .get_definition(node_type)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownInsertNodeType,
            message: format!("node type '{}' is not registered", node_type),
        })?;

    let compatible_inputs = definition
        .inputs
        .iter()
        .filter(|port| validate_connection(&source.port.data_type, &port.data_type))
        .collect::<Vec<_>>();
    if compatible_inputs.is_empty() {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::NoCompatibleInsertInput,
            message: format!(
                "node type '{}' has no input compatible with '{:?}'",
                node_type, source.port.data_type
            ),
        });
    }

    let target_port = preferred_input_port_id
        .and_then(|preferred| compatible_inputs.iter().find(|port| port.id == preferred).copied())
        .or_else(|| compatible_inputs.first().copied())
        .expect("compatible inputs must be non-empty");

    let inserted_node_id = format!("node-{}", Uuid::new_v4());
    let inserted_node = GraphNode {
        id: inserted_node_id.clone(),
        node_type: definition.node_type.clone(),
        position: position_hint.position.clone(),
        data: default_node_data(definition, next_inserted_node_label(graph, definition)),
    };
    let inserted_edge = GraphEdge {
        id: format!(
            "{}-{}-{}-{}",
            source_anchor.node_id, source_anchor.port_id, inserted_node_id, target_port.id
        ),
        source: source_anchor.node_id.clone(),
        source_handle: source_anchor.port_id.clone(),
        target: inserted_node_id,
        target_handle: target_port.id.clone(),
    };

    let mut preview_graph = graph.clone();
    preview_graph.nodes.push(inserted_node.clone());
    evaluate_connection(
        &preview_graph,
        registry,
        source_anchor,
        &ConnectionAnchor {
            node_id: inserted_edge.target.clone(),
            port_id: inserted_edge.target_handle.clone(),
        },
    )?;

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
        graph: Some(graph.clone()),
        rejection: Some(rejection),
    }
}
