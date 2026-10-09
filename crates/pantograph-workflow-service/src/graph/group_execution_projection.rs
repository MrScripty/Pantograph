//! Pure execution projection for top-level CPU processing groups.
//!
//! Saved authoring state stays grouped. Wrappers are composition facts, never
//! executable tasks; primitive IDs, configuration and edge IDs are retained.
use std::collections::{BTreeMap, BTreeSet};

use pantograph_inference_interface_contracts::InferenceDiagnosticCode as Code;
use serde::{Deserialize, Serialize};

use pantograph_node_contracts::{
    ComposedInternalEdge, ComposedInternalGraph, ComposedInternalNode, ComposedNodeContract,
    ComposedPortMapping, ComposedPortMappings, ComposedTracePolicy, EffectiveNodeContract,
    NodeAuthoringMetadata, NodeBehaviorVersion, NodeCategory, NodeExecutionSemantics,
    NodeTypeContract, PortContract, PortKind,
};

use super::effective_definition::{effective_node_contract, EffectiveDefinitionError};
use super::{
    validate_workflow_graph_contract_diagnostics, GraphNode, NodeGroup, NodeRegistry, PortMapping,
    WorkflowGraph,
};
use crate::workflow::WorkflowServiceError;

const GROUP_NODE_TYPE: &str = "node-group";
// Identifies the composition interface, not a registered wrapper task behavior.
const COMPOSITION_CONTRACT_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq)]
pub struct GroupExecutionProjection {
    pub executable_graph: WorkflowGraph,
    pub compositions: BTreeMap<String, ComposedNodeContract>,
    pub parent_by_node: BTreeMap<String, String>,
}

/// Run the same typed preflight used by current validation and submission.
/// The compatibility lowerer retains its existing service errors.
pub fn lower_groups(
    authored: &WorkflowGraph,
    registry: &NodeRegistry,
) -> Result<GroupExecutionProjection, WorkflowServiceError> {
    preflight_groups(authored, registry).map_err(GroupPreflightError::into_service_error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupRejectionKind {
    Malformed,
    Unsupported,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowGroupFailureFact {
    pub code: Code,
    pub rejection_kind: GroupRejectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    /// Raw malformed IDs over 128 UTF-8 bytes are omitted, never truncated into
    /// a misleading identity. Group/field and the bounded message remain available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_id: Option<String>,
    pub field: String,
    pub message: String,
    pub repair_hint: String,
    pub blocking_submission: bool,
}

#[derive(Debug)]
pub struct GroupPreflightError {
    pub failure: Box<WorkflowGroupFailureFact>,
    service_error: Box<WorkflowServiceError>,
}
impl GroupPreflightError {
    pub fn into_service_error(self) -> WorkflowServiceError {
        *self.service_error
    }
}

/// Lower the published saved group representation using canonical contracts.
/// Flat graphs pass through unchanged, including their existing validation policy.
/// Groups currently admit only JSON Filter and merge processing primitives.
pub fn preflight_groups(
    authored: &WorkflowGraph,
    registry: &NodeRegistry,
) -> Result<GroupExecutionProjection, GroupPreflightError> {
    let mut projection = GroupExecutionProjection {
        executable_graph: authored.clone(),
        compositions: BTreeMap::new(),
        parent_by_node: BTreeMap::new(),
    };
    if !authored
        .nodes
        .iter()
        .any(|node| node.node_type == GROUP_NODE_TYPE)
    {
        return Ok(projection);
    }

    let mut node_ids = BTreeSet::new();
    for node in &authored.nodes {
        let canonical = node
            .id
            .parse::<pantograph_node_contracts::NodeInstanceId>()
            .map_err(|error| {
                invalid(
                    Code::GroupIdentityInvalid,
                    None,
                    Some(&node.id),
                    "nodes.id",
                    error.to_string(),
                )
            })?;
        if canonical.as_str() != node.id {
            return Err(invalid(
                Code::GroupIdentityInvalid,
                None,
                Some(&node.id),
                "nodes.id",
                format!("authored node '{}' has a noncanonical identity", node.id),
            ));
        }
        if !node_ids.insert(node.id.clone()) {
            return Err(invalid(
                Code::GroupIdentityInvalid,
                None,
                Some(&node.id),
                "nodes.id",
                format!("duplicate authored node id '{}'", node.id),
            ));
        }
    }
    let root_ids = node_ids.clone();
    let mut groups = BTreeMap::new();
    for wrapper in authored
        .nodes
        .iter()
        .filter(|node| node.node_type == GROUP_NODE_TYPE)
    {
        let group = read_group(wrapper)?;
        let mut contracts = BTreeMap::new();
        for node in &group.nodes {
            if !matches!(node.node_type.as_str(), "json-filter" | "merge") {
                return Err(invalid(
                    Code::GroupChildUnsupported,
                    Some(&group.id),
                    Some(&node.id),
                    "group.nodes.node_type",
                    format!(
                        "group '{}' contains unsupported processing type '{}'",
                        group.id, node.node_type
                    ),
                ));
            }
            if !node_ids.insert(node.id.clone()) {
                return Err(invalid(
                    Code::GroupIdentityInvalid,
                    Some(&group.id),
                    Some(&node.id),
                    "group.nodes.id",
                    format!(
                        "group '{}' has colliding primitive id '{}'",
                        group.id, node.id
                    ),
                ));
            }
            let contract = effective_node_contract(node, registry).map_err(|error| {
                let (code, field) = match &error {
                    EffectiveDefinitionError::InvalidNodeId { .. } => {
                        (Code::GroupIdentityInvalid, "group.nodes.id")
                    }
                    EffectiveDefinitionError::UnknownNodeType(_)
                    | EffectiveDefinitionError::InvalidNodeType { .. } => {
                        (Code::GroupChildUnsupported, "group.nodes.node_type")
                    }
                    EffectiveDefinitionError::InvalidDynamicDefinition { .. } => {
                        (Code::GroupCompositionInvalid, "group.nodes.data.definition")
                    }
                };
                invalid(
                    code,
                    Some(&group.id),
                    Some(&node.id),
                    field,
                    format!("group '{}' primitive '{}': {error:?}", group.id, node.id),
                )
            })?;
            if contract.context.node_instance_id.as_str() != node.id {
                return Err(invalid(
                    Code::GroupIdentityInvalid,
                    Some(&group.id),
                    Some(&node.id),
                    "group.nodes.id",
                    format!(
                        "group '{}' primitive '{}' has a noncanonical identity",
                        group.id, node.id
                    ),
                ));
            }
            // These CPU adapters materialize the existing built-in ports only.
            // Resolve effective contracts first, then refuse execution-shape
            // refinements that these fixed CPU adapters cannot materialize.
            if contract.inputs.iter().any(|port| {
                !cpu_port_matches(&port.base, contract.static_contract.input(&port.base.id))
            }) || contract.outputs.iter().any(|port| {
                !cpu_port_matches(&port.base, contract.static_contract.output(&port.base.id))
            }) {
                return Err(invalid(
                    Code::GroupEffectivePortsUnsupported,
                    Some(&group.id),
                    Some(&node.id),
                    "group.nodes.data.definition",
                    format!(
                        "group '{}' primitive '{}' has unsupported effective CPU ports",
                        group.id, node.id
                    ),
                ));
            }
            contracts.insert(node.id.clone(), contract);
            projection
                .parent_by_node
                .insert(node.id.clone(), group.id.clone());
        }
        let composition = compose(&group, &contracts)?;
        projection
            .compositions
            .insert(group.id.clone(), composition);
        groups.insert(group.id.clone(), group);
    }

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for node in &authored.nodes {
        if let Some(group) = groups.get(&node.id) {
            let inner_ids = group
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<BTreeSet<_>>();
            for edge in &group.edges {
                if !inner_ids.contains(edge.source.as_str())
                    || !inner_ids.contains(edge.target.as_str())
                {
                    return Err(invalid(
                        Code::GroupConnectionInvalid,
                        Some(&group.id),
                        None,
                        "group.edges",
                        format!(
                            "group '{}' edge '{}' escapes its internal graph",
                            group.id, edge.id
                        ),
                    ));
                }
            }
            nodes.extend(group.nodes.clone());
            edges.extend(group.edges.clone());
        } else {
            nodes.push(node.clone());
        }
    }
    for edge in &authored.edges {
        if !root_ids.contains(&edge.source) || !root_ids.contains(&edge.target) {
            return Err(invalid(
                Code::GroupConnectionInvalid,
                None,
                None,
                "edges",
                format!("authored edge '{}' has a missing root endpoint", edge.id),
            ));
        }
        let mut rewritten = edge.clone();
        if let Some(composition) = projection.compositions.get(&edge.source) {
            let mapping = boundary_mapping(
                &composition.port_mappings.outputs,
                &edge.source_handle,
                &edge.source,
            )?;
            rewritten.source = mapping.internal_node_id.as_str().to_string();
            rewritten.source_handle = mapping.internal_port_id.as_str().to_string();
        }
        if let Some(composition) = projection.compositions.get(&edge.target) {
            let mapping = boundary_mapping(
                &composition.port_mappings.inputs,
                &edge.target_handle,
                &edge.target,
            )?;
            rewritten.target = mapping.internal_node_id.as_str().to_string();
            rewritten.target_handle = mapping.internal_port_id.as_str().to_string();
        }
        edges.push(rewritten);
    }
    let mut connections = BTreeSet::new();
    for edge in &edges {
        if !connections.insert((
            &edge.source,
            &edge.source_handle,
            &edge.target,
            &edge.target_handle,
        )) {
            return Err(invalid(
                Code::GroupConnectionInvalid,
                projection
                    .parent_by_node
                    .get(&edge.target)
                    .or_else(|| projection.parent_by_node.get(&edge.source))
                    .map(String::as_str),
                Some(&edge.target),
                "edges",
                format!(
                    "lowered graph has duplicate connection on edge '{}'",
                    edge.id
                ),
            ));
        }
    }
    // Preserve authored derived metadata; execution consumes only these primitives.
    projection.executable_graph.nodes = nodes;
    projection.executable_graph.edges = edges;
    let diagnostics =
        validate_workflow_graph_contract_diagnostics(&projection.executable_graph, registry)
            .into_iter()
            .filter(|diagnostic| diagnostic.blocking_submission)
            .collect::<Vec<_>>();
    if !diagnostics.is_empty() {
        let message = format!(
            "lowered group graph has {} blocking contract diagnostic(s)",
            diagnostics.len()
        );
        let child_id = diagnostics.first().and_then(|d| d.node_id.as_deref());
        let group_id = child_id
            .and_then(|id| projection.parent_by_node.get(id))
            .map(String::as_str);
        let mut error = invalid(
            Code::GroupContractBlocked,
            group_id,
            child_id,
            "lowered_graph",
            message.clone(),
        );
        error.service_error = Box::new(WorkflowServiceError::StaleWorkflowGraph {
            message,
            diagnostics,
        });
        return Err(error);
    }
    Ok(projection)
}

fn invalid(
    code: Code,
    group_id: Option<&str>,
    child_id: Option<&str>,
    field: &str,
    message: impl Into<String>,
) -> GroupPreflightError {
    let message = message.into();
    let (rejection_kind, repair_hint) = match code {
        Code::GroupSchemaInvalid => (GroupRejectionKind::Malformed, "Repair the saved group object and its required arrays and object fields."),
        Code::GroupChildUnsupported => (GroupRejectionKind::Unsupported, "Use supported top-level json-filter or merge children; native and nested groups require a supported execution adapter."),
        Code::GroupEffectivePortsUnsupported => (GroupRejectionKind::Unsupported, "Restore the built-in CPU port shape; display labels and editor hints may remain customized."),
        Code::GroupIdentityInvalid => (GroupRejectionKind::Invalid, "Use canonical, globally unique group and child identifiers matching the saved wrapper."),
        Code::GroupMappingInvalid => (GroupRejectionKind::Invalid, "Repair the exposed port mapping to an existing child port in the correct direction; identical output fan-out mappings may remain."),
        Code::GroupConnectionInvalid => (GroupRejectionKind::Invalid, "Repair the edge endpoints and duplicate connections while retaining the authored group."),
        _ => (GroupRejectionKind::Invalid, "Repair the indicated child contract or lowered connection; retain the authored group and validate again."),
    };
    GroupPreflightError {
        failure: Box::new(WorkflowGroupFailureFact {
            code,
            rejection_kind,
            group_id: group_id.filter(|id| id.len() <= 128).map(str::to_owned),
            child_id: child_id.filter(|id| id.len() <= 128).map(str::to_owned),
            field: field.to_owned(),
            message: message.chars().take(2048).collect(),
            repair_hint: repair_hint.to_owned(),
            blocking_submission: true,
        }),
        service_error: Box::new(WorkflowServiceError::InvalidRequest(message)),
    }
}

fn cpu_port_matches(effective: &PortContract, builtin: Option<&PortContract>) -> bool {
    let Some(builtin) = builtin else {
        return false;
    };
    let mut shape = effective.clone();
    // Display-only changes do not alter the materialized adapter contract.
    shape.label.clone_from(&builtin.label);
    shape.editor_hints.clone_from(&builtin.editor_hints);
    &shape == builtin
}

fn read_group(wrapper: &GraphNode) -> Result<NodeGroup, GroupPreflightError> {
    let raw = wrapper
        .data
        .get("group")
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            invalid(
                Code::GroupSchemaInvalid,
                Some(&wrapper.id),
                None,
                "group",
                format!("group '{}' must be a JSON object", wrapper.id),
            )
        })?;
    // Serde accepts arrays for structs too; require the published object shape.
    for field in ["nodes", "edges", "exposed_inputs", "exposed_outputs"] {
        let values = raw
            .get(field)
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| {
                invalid(
                    Code::GroupSchemaInvalid,
                    Some(&wrapper.id),
                    None,
                    &format!("group.{field}"),
                    format!("group '{}' field '{field}' must be an array", wrapper.id),
                )
            })?;
        if values.iter().any(|value| !value.is_object()) {
            return Err(invalid(
                Code::GroupSchemaInvalid,
                Some(&wrapper.id),
                None,
                &format!("group.{field}"),
                format!(
                    "group '{}' field '{field}' must contain objects",
                    wrapper.id
                ),
            ));
        }
    }
    let group: NodeGroup = serde_json::from_value(raw.clone()).map_err(|error| {
        invalid(
            Code::GroupSchemaInvalid,
            Some(&wrapper.id),
            None,
            "group",
            format!("group '{}' is malformed: {error}", wrapper.id),
        )
    })?;
    if group.id != wrapper.id {
        return Err(invalid(
            Code::GroupIdentityInvalid,
            Some(&wrapper.id),
            None,
            "group.id",
            format!(
                "group '{}' has mismatched identity or no primitives",
                wrapper.id
            ),
        ));
    }
    if group.nodes.is_empty() {
        return Err(invalid(
            Code::GroupSchemaInvalid,
            Some(&wrapper.id),
            None,
            "group.nodes",
            format!(
                "group '{}' has mismatched identity or no primitives",
                wrapper.id
            ),
        ));
    }
    // Validate wrapper identity even though it will not become a primitive task.
    wrapper
        .id
        .parse::<pantograph_node_contracts::NodeInstanceId>()
        .map_err(|error| {
            invalid(
                Code::GroupSchemaInvalid,
                Some(&wrapper.id),
                None,
                "group",
                error.to_string(),
            )
        })?;
    Ok(group)
}

fn compose(
    group: &NodeGroup,
    contracts: &BTreeMap<String, EffectiveNodeContract>,
) -> Result<ComposedNodeContract, GroupPreflightError> {
    let (inputs, input_mappings) =
        exposed_ports(&group.id, &group.exposed_inputs, PortKind::Input, contracts)?;
    let (outputs, output_mappings) = exposed_ports(
        &group.id,
        &group.exposed_outputs,
        PortKind::Output,
        contracts,
    )?;
    let mut internal_nodes = Vec::new();
    for node in &group.nodes {
        let effective = &contracts[&node.id];
        let version =
            NodeBehaviorVersion::from_contract(&effective.static_contract).map_err(|error| {
                invalid(
                    Code::GroupCompositionInvalid,
                    Some(&group.id),
                    None,
                    "group",
                    error.to_string(),
                )
            })?;
        internal_nodes.push(ComposedInternalNode {
            node_id: effective.context.node_instance_id.clone(),
            node_type: effective.context.node_type.clone(),
            label: effective.static_contract.label.clone(),
            contract_version: Some(version.contract_version),
            contract_digest: Some(version.behavior_digest),
        });
    }
    let internal_edges = group
        .edges
        .iter()
        .map(|edge| {
            Ok(ComposedInternalEdge {
                source_node_id: edge.source.parse().map_err(
                    |error: pantograph_node_contracts::NodeContractError| {
                        invalid(
                            Code::GroupCompositionInvalid,
                            Some(&group.id),
                            None,
                            "group",
                            error.to_string(),
                        )
                    },
                )?,
                source_port_id: edge.source_handle.parse().map_err(
                    |error: pantograph_node_contracts::NodeContractError| {
                        invalid(
                            Code::GroupCompositionInvalid,
                            Some(&group.id),
                            None,
                            "group",
                            error.to_string(),
                        )
                    },
                )?,
                target_node_id: edge.target.parse().map_err(
                    |error: pantograph_node_contracts::NodeContractError| {
                        invalid(
                            Code::GroupCompositionInvalid,
                            Some(&group.id),
                            None,
                            "group",
                            error.to_string(),
                        )
                    },
                )?,
                target_port_id: edge.target_handle.parse().map_err(
                    |error: pantograph_node_contracts::NodeContractError| {
                        invalid(
                            Code::GroupCompositionInvalid,
                            Some(&group.id),
                            None,
                            "group",
                            error.to_string(),
                        )
                    },
                )?,
            })
        })
        .collect::<Result<Vec<_>, GroupPreflightError>>()?;
    let composition = ComposedNodeContract {
        external_contract: NodeTypeContract {
            node_type: GROUP_NODE_TYPE.parse().map_err(
                |error: pantograph_node_contracts::NodeContractError| {
                    invalid(
                        Code::GroupCompositionInvalid,
                        Some(&group.id),
                        None,
                        "group",
                        error.to_string(),
                    )
                },
            )?,
            category: NodeCategory::Processing,
            label: group.name.clone(),
            description: group.description.clone().unwrap_or_default(),
            inputs,
            outputs,
            execution_semantics: NodeExecutionSemantics::Reactive,
            capability_requirements: Vec::new(),
            inference_tasks: Vec::new(),
            authoring: NodeAuthoringMetadata::default(),
            contract_version: Some(COMPOSITION_CONTRACT_VERSION.to_string()),
            contract_digest: None,
        },
        internal_graph: ComposedInternalGraph {
            graph_id: group.id.clone(),
            nodes: internal_nodes,
            edges: internal_edges,
        },
        port_mappings: ComposedPortMappings {
            inputs: input_mappings,
            outputs: output_mappings,
        },
        trace_policy: ComposedTracePolicy::PreservePrimitiveFacts,
        upgrade_metadata: None,
    };
    composition.validate().map_err(|error| {
        let child = match &error {
            pantograph_node_contracts::NodeContractError::UnknownCompositionInternalNode {
                node_id,
            } => Some(node_id.as_str()),
            _ => None,
        };
        invalid(
            Code::GroupCompositionInvalid,
            Some(&group.id),
            child,
            if child.is_some() {
                "group.edges"
            } else {
                "group"
            },
            format!("group '{}' composition: {error}", group.id),
        )
    })?;
    Ok(composition)
}

fn exposed_ports(
    group_id: &str,
    mappings: &[PortMapping],
    kind: PortKind,
    contracts: &BTreeMap<String, EffectiveNodeContract>,
) -> Result<(Vec<PortContract>, Vec<ComposedPortMapping>), GroupPreflightError> {
    let mut resolved = BTreeMap::<String, (PortContract, ComposedPortMapping)>::new();
    for mapping in mappings {
        let contract = contracts.get(&mapping.internal_node_id).ok_or_else(|| {
            invalid(
                Code::GroupMappingInvalid,
                Some(group_id),
                Some(&mapping.internal_node_id),
                match kind {
                    PortKind::Input => "group.exposed_inputs",
                    PortKind::Output => "group.exposed_outputs",
                },
                format!(
                    "group '{group_id}' mapping references missing primitive '{}'",
                    mapping.internal_node_id
                ),
            )
        })?;
        let ports = match kind {
            PortKind::Input => &contract.inputs,
            PortKind::Output => &contract.outputs,
        };
        let port = ports
            .iter()
            .find(|port| port.base.id.as_str() == mapping.internal_port_id)
            .ok_or_else(|| {
                invalid(
                    Code::GroupMappingInvalid,
                    Some(group_id),
                    Some(&mapping.internal_node_id),
                    match kind {
                        PortKind::Input => "group.exposed_inputs",
                        PortKind::Output => "group.exposed_outputs",
                    },
                    format!(
                        "group '{group_id}' mapping references unknown {kind:?} '{}.{}'",
                        mapping.internal_node_id, mapping.internal_port_id
                    ),
                )
            })?;
        let external_id = mapping.group_port_id.parse().map_err(
            |error: pantograph_node_contracts::NodeContractError| {
                invalid(
                    Code::GroupMappingInvalid,
                    Some(group_id),
                    Some(&mapping.internal_node_id),
                    match kind {
                        PortKind::Input => "group.exposed_inputs",
                        PortKind::Output => "group.exposed_outputs",
                    },
                    error.to_string(),
                )
            },
        )?;
        if mapping.group_port_id != pantograph_node_contracts::PortId::as_str(&external_id) {
            return Err(invalid(
                Code::GroupMappingInvalid,
                Some(group_id),
                Some(&mapping.internal_node_id),
                match kind {
                    PortKind::Input => "group.exposed_inputs",
                    PortKind::Output => "group.exposed_outputs",
                },
                format!(
                    "group '{group_id}' has a noncanonical external port '{}'",
                    mapping.group_port_id
                ),
            ));
        }
        let canonical_mapping = ComposedPortMapping {
            external_port_id: external_id,
            internal_node_id: contract.context.node_instance_id.clone(),
            internal_port_id: port.base.id.clone(),
        };
        if let Some((_, previous)) = resolved.get(&mapping.group_port_id) {
            if previous != &canonical_mapping {
                return Err(invalid(
                    Code::GroupMappingInvalid,
                    Some(group_id),
                    Some(&mapping.internal_node_id),
                    match kind {
                        PortKind::Input => "group.exposed_inputs",
                        PortKind::Output => "group.exposed_outputs",
                    },
                    format!(
                        "group '{group_id}' has conflicting mapping '{}'",
                        mapping.group_port_id
                    ),
                ));
            }
            continue; // Public CreateGroup duplicates identical mappings for fan-out.
        }
        let mut external_port = port.base.clone();
        external_port.id = canonical_mapping.external_port_id.clone();
        resolved.insert(
            mapping.group_port_id.clone(),
            (external_port, canonical_mapping),
        );
    }
    Ok(resolved.into_values().unzip())
}

fn boundary_mapping<'a>(
    mappings: &'a [ComposedPortMapping],
    external_port: &str,
    group_id: &str,
) -> Result<&'a ComposedPortMapping, GroupPreflightError> {
    mappings
        .iter()
        .find(|mapping| mapping.external_port_id.as_str() == external_port)
        .ok_or_else(|| {
            invalid(
                Code::GroupMappingInvalid,
                Some(group_id),
                None,
                "edges.boundary_port",
                format!("group '{group_id}' has no boundary mapping '{external_port}'"),
            )
        })
}
