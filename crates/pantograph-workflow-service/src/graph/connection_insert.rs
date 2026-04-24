use serde_json::{Map, Value};
use uuid::Uuid;

use super::super::registry::NodeRegistry;
use super::super::types::{
    ConnectionAnchor, ConnectionRejection, ConnectionRejectionReason, EdgeInsertionBridge,
    EdgeInsertionPreviewResponse, GraphEdge, GraphNode, InsertNodeConnectionResponse,
    InsertNodeOnEdgeResponse, InsertNodePositionHint, NodeDefinition, PortDataType, Position,
    WorkflowGraph,
};

const EDGE_INSERT_PREVIEW_NODE_ID: &str = "__edge_insert_preview__";
const EDGE_INSERT_PREVIEW_LABEL: &str = "Edge Insert Preview";

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
        if input.required && matches!(input.data_type, PortDataType::Boolean) {
            map.insert(input.id.clone(), Value::Bool(false));
        }
    }
    Value::Object(map)
}

fn resolve_insert_definition<'a>(
    registry: &'a NodeRegistry,
    node_type: &str,
) -> Result<&'a NodeDefinition, ConnectionRejection> {
    registry
        .get_definition(node_type)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownInsertNodeType,
            message: format!("node type '{}' is not registered", node_type),
        })
}

fn build_inserted_node(
    graph: &WorkflowGraph,
    definition: &NodeDefinition,
    node_id: String,
    position: Position,
    label: String,
) -> GraphNode {
    let resolved_label = if label.is_empty() {
        next_inserted_node_label(graph, definition)
    } else {
        label
    };

    GraphNode {
        id: node_id,
        node_type: definition.node_type.clone(),
        position,
        data: default_node_data(definition, resolved_label),
    }
}

fn edge_anchors(edge: &GraphEdge) -> (ConnectionAnchor, ConnectionAnchor) {
    (
        ConnectionAnchor {
            node_id: edge.source.clone(),
            port_id: edge.source_handle.clone(),
        },
        ConnectionAnchor {
            node_id: edge.target.clone(),
            port_id: edge.target_handle.clone(),
        },
    )
}

fn preview_graph_without_edge(
    graph: &WorkflowGraph,
    inserted_node: &GraphNode,
    replaced_edge_id: &str,
) -> WorkflowGraph {
    let mut preview_graph = graph.clone();
    preview_graph
        .edges
        .retain(|edge| edge.id != replaced_edge_id);
    preview_graph.nodes.push(inserted_node.clone());
    preview_graph
}

fn resolve_edge_insertion_bridge(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    edge_id: &str,
    definition: &NodeDefinition,
) -> Result<EdgeInsertionBridge, ConnectionRejection> {
    let edge = super::resolve_edge(graph, edge_id)?;
    let (source_anchor, target_anchor) = edge_anchors(edge);
    let source = super::resolve_output_anchor(graph, registry, &source_anchor)?;
    let target = super::resolve_input_anchor(graph, registry, &target_anchor)?;

    let compatible_inputs = definition
        .inputs
        .iter()
        .filter(|port| super::validate_connection(&source.port.data_type, &port.data_type))
        .collect::<Vec<_>>();
    if compatible_inputs.is_empty() {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::NoCompatibleInsertInput,
            message: format!(
                "node type '{}' has no input compatible with '{:?}'",
                definition.node_type, source.port.data_type
            ),
        });
    }

    let preview_node = build_inserted_node(
        graph,
        definition,
        EDGE_INSERT_PREVIEW_NODE_ID.to_string(),
        Position::default(),
        EDGE_INSERT_PREVIEW_LABEL.to_string(),
    );
    let preview_graph = preview_graph_without_edge(graph, &preview_node, edge_id);

    for input_port in compatible_inputs {
        let inserted_input_anchor = ConnectionAnchor {
            node_id: preview_node.id.clone(),
            port_id: input_port.id.clone(),
        };

        if super::evaluate_connection(
            &preview_graph,
            registry,
            &source_anchor,
            &inserted_input_anchor,
        )
        .is_err()
        {
            continue;
        }

        let mut preview_graph_with_input = preview_graph.clone();
        preview_graph_with_input.edges.push(GraphEdge {
            id: format!(
                "{}-{}-{}-{}",
                source_anchor.node_id, source_anchor.port_id, preview_node.id, input_port.id
            ),
            source: source_anchor.node_id.clone(),
            source_handle: source_anchor.port_id.clone(),
            target: preview_node.id.clone(),
            target_handle: input_port.id.clone(),
        });

        for output_port in &definition.outputs {
            if !super::validate_connection(&output_port.data_type, &target.port.data_type) {
                continue;
            }

            let inserted_output_anchor = ConnectionAnchor {
                node_id: preview_node.id.clone(),
                port_id: output_port.id.clone(),
            };

            if super::evaluate_connection(
                &preview_graph_with_input,
                registry,
                &inserted_output_anchor,
                &target_anchor,
            )
            .is_ok()
            {
                return Ok(EdgeInsertionBridge {
                    input_port_id: input_port.id.clone(),
                    output_port_id: output_port.id.clone(),
                });
            }
        }
    }

    Err(ConnectionRejection {
        reason: ConnectionRejectionReason::NoCompatibleInsertPath,
        message: format!(
            "node type '{}' has no valid path between edge '{}'",
            definition.node_type, edge_id
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
    super::ensure_graph_revision(graph, graph_revision)?;

    let source = super::resolve_output_anchor(graph, registry, source_anchor)?;
    let definition = resolve_insert_definition(registry, node_type)?;

    let compatible_inputs = definition
        .inputs
        .iter()
        .filter(|port| super::validate_connection(&source.port.data_type, &port.data_type))
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
        .and_then(|preferred| {
            compatible_inputs
                .iter()
                .find(|port| port.id == preferred)
                .copied()
        })
        .or_else(|| compatible_inputs.first().copied())
        .expect("compatible inputs must be non-empty");

    let inserted_node_id = format!("node-{}", Uuid::new_v4());
    let inserted_node = build_inserted_node(
        graph,
        definition,
        inserted_node_id.clone(),
        position_hint.position.clone(),
        String::new(),
    );
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
    super::evaluate_connection(
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

pub fn preview_node_insert_on_edge(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    graph_revision: &str,
    edge_id: &str,
    node_type: &str,
) -> Result<EdgeInsertionBridge, ConnectionRejection> {
    super::ensure_graph_revision(graph, graph_revision)?;
    let definition = resolve_insert_definition(registry, node_type)?;
    resolve_edge_insertion_bridge(graph, registry, edge_id, definition)
}

pub fn insert_node_on_edge(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    graph_revision: &str,
    edge_id: &str,
    node_type: &str,
    position_hint: &InsertNodePositionHint,
) -> Result<(GraphNode, GraphEdge, GraphEdge, EdgeInsertionBridge), ConnectionRejection> {
    super::ensure_graph_revision(graph, graph_revision)?;
    let edge = super::resolve_edge(graph, edge_id)?;
    let (source_anchor, target_anchor) = edge_anchors(edge);
    let definition = resolve_insert_definition(registry, node_type)?;
    let bridge = resolve_edge_insertion_bridge(graph, registry, edge_id, definition)?;

    let inserted_node_id = format!("node-{}", Uuid::new_v4());
    let inserted_node = build_inserted_node(
        graph,
        definition,
        inserted_node_id.clone(),
        position_hint.position.clone(),
        String::new(),
    );
    let incoming_edge = GraphEdge {
        id: format!(
            "{}-{}-{}-{}",
            source_anchor.node_id, source_anchor.port_id, inserted_node_id, bridge.input_port_id
        ),
        source: source_anchor.node_id.clone(),
        source_handle: source_anchor.port_id.clone(),
        target: inserted_node_id.clone(),
        target_handle: bridge.input_port_id.clone(),
    };
    let outgoing_edge = GraphEdge {
        id: format!(
            "{}-{}-{}-{}",
            inserted_node_id, bridge.output_port_id, target_anchor.node_id, target_anchor.port_id
        ),
        source: inserted_node_id,
        source_handle: bridge.output_port_id.clone(),
        target: target_anchor.node_id,
        target_handle: target_anchor.port_id,
    };

    Ok((inserted_node, incoming_edge, outgoing_edge, bridge))
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
        workflow_event: None,
        workflow_execution_session_state: None,
        rejection: Some(rejection),
    }
}

pub fn rejected_edge_insert_preview_response(
    graph: &WorkflowGraph,
    rejection: ConnectionRejection,
) -> EdgeInsertionPreviewResponse {
    EdgeInsertionPreviewResponse {
        accepted: false,
        graph_revision: graph.compute_fingerprint(),
        bridge: None,
        rejection: Some(rejection),
    }
}

pub fn rejected_insert_on_edge_response(
    graph: &WorkflowGraph,
    rejection: ConnectionRejection,
) -> InsertNodeOnEdgeResponse {
    InsertNodeOnEdgeResponse {
        accepted: false,
        graph_revision: graph.compute_fingerprint(),
        inserted_node_id: None,
        bridge: None,
        graph: Some(graph.clone()),
        workflow_event: None,
        workflow_execution_session_state: None,
        rejection: Some(rejection),
    }
}
