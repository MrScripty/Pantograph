use std::collections::{HashSet, VecDeque};

use pantograph_inference_interface_contracts::{
    InferenceArtifactType, InferenceConnectionSurface, InferenceConnectionSurfaceStatus,
    InferenceConstraintType, InferencePortDescriptor, InferencePortRequirement,
    InferenceReferenceType, InferenceScalarType, InferenceValueType,
};

use super::effective_definition::{effective_node_definition, EffectiveDefinitionError};
use super::registry::NodeRegistry;
use super::types::{
    ConnectionAnchor, ConnectionCandidatesResponse, ConnectionCommitResponse, ConnectionRejection,
    ConnectionRejectionReason, ConnectionTargetAnchorCandidate, ConnectionTargetNodeCandidate,
    GraphEdge, GraphNode, InsertableNodeTypeCandidate, NodeDefinition, PortDefinition,
    WorkflowGraph,
};
use super::validation::{check_connection_ports, validate_connection};

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

#[derive(Debug, Clone, Copy)]
pub struct InferenceConnectionSurfaceView<'a> {
    surfaces: &'a [InferenceConnectionSurface],
}

impl<'a> InferenceConnectionSurfaceView<'a> {
    pub fn new(surfaces: &'a [InferenceConnectionSurface]) -> Self {
        Self { surfaces }
    }

    fn current_surface_for(&self, node_id: &str) -> Option<&'a InferenceConnectionSurface> {
        self.surfaces.iter().find(|surface| {
            surface.status == InferenceConnectionSurfaceStatus::Current
                && surface.node_id.as_str() == node_id
        })
    }
}

impl Default for InferenceConnectionSurfaceView<'_> {
    fn default() -> Self {
        Self { surfaces: &[] }
    }
}

fn is_static_llm_connection_input(port_id: &str) -> bool {
    matches!(
        port_id,
        "pumas_model_ref" | "dependency_environment_sidecar"
    )
}

fn is_connectable_input_port(node_type: &str, port: &PortDefinition) -> bool {
    node_type != "llm-inference" || is_static_llm_connection_input(&port.id)
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

fn effective_definition_error_message(role: &str, error: EffectiveDefinitionError) -> String {
    match error {
        EffectiveDefinitionError::UnknownNodeType(node_type) => {
            format!("{role} node type '{node_type}' is unknown")
        }
        EffectiveDefinitionError::InvalidNodeId { node_id, message } => {
            format!("{role} node id '{node_id}' is invalid: {message}")
        }
        EffectiveDefinitionError::InvalidNodeType { node_type, message } => {
            format!("{role} node type '{node_type}' is invalid: {message}")
        }
        EffectiveDefinitionError::InvalidDynamicDefinition { message } => {
            format!("{role} node dynamic definition is invalid: {message}")
        }
    }
}

fn connection_definition_for_node(
    node: &GraphNode,
    registry: &NodeRegistry,
    surfaces: InferenceConnectionSurfaceView<'_>,
) -> Result<NodeDefinition, EffectiveDefinitionError> {
    if node.node_type != "llm-inference" {
        return effective_node_definition(node, registry);
    }

    let mut definition = registry
        .get_definition(&node.node_type)
        .cloned()
        .ok_or_else(|| EffectiveDefinitionError::UnknownNodeType(node.node_type.clone()))?;
    definition
        .inputs
        .retain(|port| is_static_llm_connection_input(&port.id));

    let Some(surface) = surfaces.current_surface_for(&node.id) else {
        return Ok(definition);
    };

    definition.inputs.extend(
        surface
            .inputs
            .iter()
            .map(inference_port_descriptor_to_definition)
            .collect::<Result<Vec<_>, _>>()?,
    );
    definition.outputs.extend(
        surface
            .outputs
            .iter()
            .map(inference_port_descriptor_to_definition)
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(definition)
}

fn inference_port_descriptor_to_definition(
    port: &InferencePortDescriptor,
) -> Result<PortDefinition, EffectiveDefinitionError> {
    Ok(PortDefinition {
        id: port.port_id.as_str().to_string(),
        label: port.label.clone(),
        data_type: inference_value_type_to_port_data_type(&port.value_type)?,
        required: match port.requirement {
            InferencePortRequirement::Required => true,
            InferencePortRequirement::Optional => false,
            _ => {
                return Err(EffectiveDefinitionError::InvalidDynamicDefinition {
                    message: format!(
                        "inference connection surface port '{}' has unsupported requirement",
                        port.port_id.as_str()
                    ),
                });
            }
        },
        multiple: false,
        options_provider: None,
        inference_payloads: Vec::new(),
    })
}

fn inference_value_type_to_port_data_type(
    value_type: &InferenceValueType,
) -> Result<super::types::PortDataType, EffectiveDefinitionError> {
    let data_type = match value_type {
        InferenceValueType::Scalar(InferenceScalarType::String) => {
            super::types::PortDataType::String
        }
        InferenceValueType::Scalar(InferenceScalarType::Bool) => {
            super::types::PortDataType::Boolean
        }
        InferenceValueType::Scalar(
            InferenceScalarType::I64 | InferenceScalarType::U64 | InferenceScalarType::F64,
        ) => super::types::PortDataType::Number,
        InferenceValueType::Artifact(InferenceArtifactType::Image) => {
            super::types::PortDataType::Image
        }
        InferenceValueType::Artifact(InferenceArtifactType::Audio) => {
            super::types::PortDataType::Audio
        }
        InferenceValueType::Artifact(InferenceArtifactType::Tensor) => {
            super::types::PortDataType::Tensor
        }
        InferenceValueType::Artifact(InferenceArtifactType::Document) => {
            super::types::PortDataType::Document
        }
        InferenceValueType::Artifact(
            InferenceArtifactType::Video | InferenceArtifactType::Media,
        ) => super::types::PortDataType::Json,
        InferenceValueType::Reference(
            InferenceReferenceType::PumasModel
            | InferenceReferenceType::MediaArtifact
            | InferenceReferenceType::RuntimeArtifact
            | InferenceReferenceType::SchedulerTaskResult,
        ) => super::types::PortDataType::Json,
        InferenceValueType::Constraint(
            InferenceConstraintType::Runtime
            | InferenceConstraintType::Device
            | InferenceConstraintType::DenoisingScheduler
            | InferenceConstraintType::SamplingMethod,
        ) => super::types::PortDataType::String,
        _ => {
            return Err(EffectiveDefinitionError::InvalidDynamicDefinition {
                message: "inference connection surface uses an unsupported port value type"
                    .to_string(),
            });
        }
    };
    Ok(data_type)
}

fn resolve_output_anchor<'a>(
    graph: &'a WorkflowGraph,
    registry: &'a NodeRegistry,
    surfaces: InferenceConnectionSurfaceView<'_>,
    anchor: &ConnectionAnchor,
) -> Result<ResolvedOutputAnchor<'a>, ConnectionRejection> {
    let node = graph
        .find_node(&anchor.node_id)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownSourceAnchor,
            message: format!("source node '{}' was not found", anchor.node_id),
            contract_diagnostic: None,
        })?;
    let definition = connection_definition_for_node(node, registry, surfaces).map_err(|error| {
        ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownSourceAnchor,
            message: effective_definition_error_message("source", error),
            contract_diagnostic: None,
        }
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
            contract_diagnostic: None,
        })?;

    Ok(ResolvedOutputAnchor { node, port })
}

fn resolve_input_anchor<'a>(
    graph: &'a WorkflowGraph,
    registry: &'a NodeRegistry,
    surfaces: InferenceConnectionSurfaceView<'_>,
    anchor: &ConnectionAnchor,
) -> Result<ResolvedInputAnchor<'a>, ConnectionRejection> {
    let node = graph
        .find_node(&anchor.node_id)
        .ok_or_else(|| ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownTargetAnchor,
            message: format!("target node '{}' was not found", anchor.node_id),
            contract_diagnostic: None,
        })?;
    let definition = connection_definition_for_node(node, registry, surfaces).map_err(|error| {
        ConnectionRejection {
            reason: ConnectionRejectionReason::UnknownTargetAnchor,
            message: effective_definition_error_message("target", error),
            contract_diagnostic: None,
        }
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
            contract_diagnostic: None,
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
            contract_diagnostic: None,
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
            contract_diagnostic: None,
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
    surfaces: InferenceConnectionSurfaceView<'_>,
    source_anchor: &ConnectionAnchor,
    target_anchor: &ConnectionAnchor,
) -> Result<(), ConnectionRejection> {
    let source = resolve_output_anchor(graph, registry, surfaces, source_anchor)?;
    let target = resolve_input_anchor(graph, registry, surfaces, target_anchor)?;

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
            contract_diagnostic: None,
        });
    }

    if source.node.id == target.node.id {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::SelfConnection,
            message: format!("node '{}' cannot connect to itself", source.node.id),
            contract_diagnostic: None,
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
            contract_diagnostic: None,
        });
    }

    let compatibility =
        check_connection_ports(&source.node.id, &source.port, &target.node.id, &target.port)
            .map_err(|error| ConnectionRejection {
                reason: ConnectionRejectionReason::IncompatibleTypes,
                message: format!(
                    "connection '{}.{}' -> '{}.{}' could not be checked: {}",
                    source.node.id, source.port.id, target.node.id, target.port.id, error
                ),
                contract_diagnostic: None,
            })?;
    if !compatibility.is_compatible() {
        let diagnostic = compatibility.rejection;
        let message = diagnostic.as_ref().map_or_else(
            || {
                format!(
                    "source type '{:?}' is not compatible with target type '{:?}'",
                    source.port.data_type, target.port.data_type
                )
            },
            |diagnostic| diagnostic.message.clone(),
        );
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::IncompatibleTypes,
            message,
            contract_diagnostic: diagnostic.map(Box::new),
        });
    }

    if would_create_cycle(graph, &source.node.id, &target.node.id) {
        return Err(ConnectionRejection {
            reason: ConnectionRejectionReason::CycleDetected,
            message: format!(
                "connection '{}.{}' -> '{}.{}' would create a cycle",
                source.node.id, source.port.id, target.node.id, target.port.id
            ),
            contract_diagnostic: None,
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
    connection_candidates_with_surfaces(
        graph,
        registry,
        InferenceConnectionSurfaceView::default(),
        source_anchor,
        requested_revision,
    )
}

pub fn connection_candidates_with_surfaces(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    surfaces: InferenceConnectionSurfaceView<'_>,
    source_anchor: ConnectionAnchor,
    requested_revision: Option<&str>,
) -> Result<ConnectionCandidatesResponse, ConnectionRejection> {
    let source = resolve_output_anchor(graph, registry, surfaces, &source_anchor)?;
    let graph_revision = graph.compute_fingerprint();

    let mut compatible_nodes = Vec::new();
    for node in &graph.nodes {
        if node.id == source.node.id {
            continue;
        }

        let Ok(definition) = connection_definition_for_node(node, registry, surfaces) else {
            continue;
        };

        let mut anchors = Vec::new();
        for port in &definition.inputs {
            let target_anchor = ConnectionAnchor {
                node_id: node.id.clone(),
                port_id: port.id.clone(),
            };
            if evaluate_connection(graph, registry, surfaces, &source_anchor, &target_anchor)
                .is_ok()
            {
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
                .filter(|port| is_connectable_input_port(&definition.node_type, port))
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
    commit_connection_with_surfaces(
        graph,
        registry,
        InferenceConnectionSurfaceView::default(),
        graph_revision,
        source_anchor,
        target_anchor,
    )
}

pub fn commit_connection_with_surfaces(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    surfaces: InferenceConnectionSurfaceView<'_>,
    graph_revision: &str,
    source_anchor: &ConnectionAnchor,
    target_anchor: &ConnectionAnchor,
) -> Result<(), ConnectionRejection> {
    ensure_graph_revision(graph, graph_revision)?;
    evaluate_connection(graph, registry, surfaces, source_anchor, target_anchor)
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
        workflow_execution_session_state: None,
        rejection: Some(rejection),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{NodeCategory, Position};

    fn current_connection_surface() -> InferenceConnectionSurface {
        serde_json::from_str(include_str!(
            "../../../pantograph-inference-interface-contracts/tests/fixtures/connection_surface_image_generation_current.json"
        ))
        .expect("current connection surface fixture")
    }

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

    fn descriptor_backed_inference_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "source".into(),
                    node_type: "text-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({"label": "Source"}),
                },
                GraphNode {
                    id: "inference-node-1".into(),
                    node_type: "llm-inference".into(),
                    position: Position { x: 100.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "image-output".into(),
                    node_type: "image-output".into(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: serde_json::json!({}),
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
            node.node_type == "merge"
                && node.category == NodeCategory::Control
                && node
                    .matching_input_port_ids
                    .iter()
                    .any(|port_id| port_id == "inputs")
        }));
        assert!(
            !response
                .insertable_node_types
                .iter()
                .any(|node| node.node_type == "llm-inference"),
            "static llm-inference task ports are descriptor-backed and must not appear as insertable static ports"
        );
    }

    #[test]
    fn connection_candidates_use_current_inference_connection_surface_ports() {
        let registry = NodeRegistry::new();
        let surface = current_connection_surface();
        let graph = descriptor_backed_inference_graph();
        let response = connection_candidates_with_surfaces(
            &graph,
            &registry,
            InferenceConnectionSurfaceView::new(std::slice::from_ref(&surface)),
            ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            None,
        )
        .expect("candidate query should succeed");

        let inference_node = response
            .compatible_nodes
            .iter()
            .find(|node| node.node_id == "inference-node-1")
            .expect("descriptor-backed inference node candidate");
        assert!(inference_node
            .anchors
            .iter()
            .any(|anchor| anchor.port_id == "prompt"));
    }

    #[test]
    fn commit_connection_accepts_current_descriptor_backed_inference_ports() {
        let registry = NodeRegistry::new();
        let surface = current_connection_surface();
        let mut graph = descriptor_backed_inference_graph();
        let revision = graph.compute_fingerprint();

        commit_connection_with_surfaces(
            &graph,
            &registry,
            InferenceConnectionSurfaceView::new(std::slice::from_ref(&surface)),
            &revision,
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            &ConnectionAnchor {
                node_id: "inference-node-1".into(),
                port_id: "prompt".into(),
            },
        )
        .expect("current descriptor-backed prompt should accept text");

        graph.edges.push(GraphEdge {
            id: "source-text-inference-prompt".into(),
            source: "source".into(),
            source_handle: "text".into(),
            target: "inference-node-1".into(),
            target_handle: "prompt".into(),
        });
        let revision = graph.compute_fingerprint();

        commit_connection_with_surfaces(
            &graph,
            &registry,
            InferenceConnectionSurfaceView::new(std::slice::from_ref(&surface)),
            &revision,
            &ConnectionAnchor {
                node_id: "inference-node-1".into(),
                port_id: "image".into(),
            },
            &ConnectionAnchor {
                node_id: "image-output".into(),
                port_id: "image".into(),
            },
        )
        .expect("current descriptor-backed image output should connect to image output node");
    }

    #[test]
    fn commit_connection_rejects_incompatible_types_with_contract_diagnostic() {
        let registry = NodeRegistry::new();
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "image".into(),
                    node_type: "image-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "text".into(),
                    node_type: "text-output".into(),
                    position: Position { x: 100.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: Vec::new(),
            derived_graph: None,
        };
        let revision = graph.compute_fingerprint();

        let rejection = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "image".into(),
                port_id: "image".into(),
            },
            &ConnectionAnchor {
                node_id: "text".into(),
                port_id: "text".into(),
            },
        )
        .expect_err("image output should not connect to text input");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::IncompatibleTypes
        );
        let diagnostic = rejection
            .contract_diagnostic
            .expect("canonical rejection diagnostic");
        assert_eq!(
            diagnostic.reason,
            pantograph_node_contracts::ConnectionRejectionReason::IncompatibleTypes
        );
        assert_eq!(diagnostic.source_node_id.as_str(), "image");
        assert_eq!(diagnostic.source_port_id.as_str(), "image");
        assert_eq!(diagnostic.target_node_id.as_str(), "text");
        assert_eq!(diagnostic.target_port_id.as_str(), "text");
    }

    #[test]
    fn commit_connection_rejects_static_llm_device_as_connection_port() {
        let registry = NodeRegistry::new();
        let graph = text_graph();
        let revision = graph.compute_fingerprint();

        let rejection = commit_connection(
            &graph,
            &registry,
            &revision,
            &ConnectionAnchor {
                node_id: "source".into(),
                port_id: "text".into(),
            },
            &ConnectionAnchor {
                node_id: "llm".into(),
                port_id: "device".into(),
            },
        )
        .expect_err("static llm device constraint must not be graph-connectable");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::UnknownTargetAnchor
        );
        assert!(
            rejection.message.contains("llm.device"),
            "unexpected rejection: {rejection:?}"
        );
    }

    #[test]
    fn preview_node_insert_on_edge_rejects_llm_without_descriptor_ports() {
        let registry = NodeRegistry::new();
        let graph = text_graph_with_edge();
        let revision = graph.compute_fingerprint();

        let rejection = preview_node_insert_on_edge(
            &graph,
            &registry,
            &revision,
            "source-text-target-text",
            "llm-inference",
        )
        .expect_err("static llm-inference no longer exposes task bridge ports");

        assert_eq!(
            rejection.reason,
            ConnectionRejectionReason::NoCompatibleInsertInput
        );
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
    fn insert_node_on_edge_returns_two_replacement_edges_for_merge() {
        let registry = NodeRegistry::new();
        let graph = text_graph_with_edge();
        let revision = graph.compute_fingerprint();

        let (inserted_node, incoming_edge, outgoing_edge, bridge) = insert_node_on_edge(
            &graph,
            &registry,
            &revision,
            "source-text-target-text",
            "merge",
            &super::super::types::InsertNodePositionHint {
                position: Position { x: 50.0, y: 24.0 },
            },
        )
        .expect("edge insert should succeed");

        assert_eq!(inserted_node.node_type, "merge");
        assert_eq!(bridge.input_port_id, "inputs");
        assert_eq!(bridge.output_port_id, "merged");
        assert_eq!(incoming_edge.source, "source");
        assert_eq!(incoming_edge.target, inserted_node.id);
        assert_eq!(incoming_edge.target_handle, "inputs");
        assert_eq!(outgoing_edge.source, inserted_node.id);
        assert_eq!(outgoing_edge.target, "target");
        assert_eq!(outgoing_edge.source_handle, "merged");
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
