use serde_json::Value;

use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, AuthoredInferencePortSnapshot, InferenceArtifactType,
    InferenceConstraintType, InferencePortDirection, InferencePortRequirement,
    InferenceReferenceType, InferenceScalarType, InferenceValueType,
    ValidatedAuthoredInferenceInterfaceSnapshot,
};
use pantograph_node_contracts::{
    ContractResolutionWarning, EffectiveNodeContract, NodeInstanceContext, NodeInstanceId,
    NodeTypeId, PortCardinality, PortContract, PortId, PortKind, PortRequirement, PortValueType,
    PortVisibility,
};

use super::registry::{convert_port, NodeRegistry};
use super::types::{GraphNode, NodeDefinition, PortDefinition};

const GENERIC_INFERENCE_NODE_TYPE: &str = "llm-inference";
const INFERENCE_DYNAMIC_DEFINITION_REJECTION: &str = concat!(
    "llm-inference node.data.definition is not an executable interface source; ",
    "inference ports must come from the authored inference interface snapshot"
);
const INFERENCE_INTERFACE_SNAPSHOT_FIELD: &str = "inference_interface_snapshot";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectiveDefinitionError {
    UnknownNodeType(String),
    InvalidNodeId { node_id: String, message: String },
    InvalidNodeType { node_type: String, message: String },
    InvalidDynamicDefinition { message: String },
}

pub fn effective_node_definition(
    node: &GraphNode,
    registry: &NodeRegistry,
) -> Result<NodeDefinition, EffectiveDefinitionError> {
    let effective_contract = effective_node_contract(node, registry)?;
    let mut definition = registry
        .get_definition(&node.node_type)
        .cloned()
        .ok_or_else(|| EffectiveDefinitionError::UnknownNodeType(node.node_type.clone()))?;
    definition.inputs = effective_contract
        .inputs
        .iter()
        .map(|port| convert_port(&port.base))
        .collect();
    definition.outputs = effective_contract
        .outputs
        .iter()
        .map(|port| convert_port(&port.base))
        .collect();
    Ok(definition)
}

pub fn effective_node_contract(
    node: &GraphNode,
    registry: &NodeRegistry,
) -> Result<EffectiveNodeContract, EffectiveDefinitionError> {
    let static_contract = registry
        .get_contract(&node.node_type)
        .cloned()
        .ok_or_else(|| EffectiveDefinitionError::UnknownNodeType(node.node_type.clone()))?;
    let context = NodeInstanceContext {
        node_instance_id: parse_node_instance_id(&node.id)?,
        node_type: parse_node_type_id(&node.node_type)?,
        graph_revision: None,
        configuration: Some(node.data.clone()),
    };
    let overlay = dynamic_contract_ports(node)?;
    let mut effective = EffectiveNodeContract::from_static_with_dynamic_ports(
        context,
        static_contract,
        overlay.inputs,
        overlay.outputs,
    )
    .map_err(|error| EffectiveDefinitionError::InvalidDynamicDefinition {
        message: error.to_string(),
    })?;
    effective.diagnostics.warnings.extend(overlay.warnings);
    Ok(effective)
}

fn dynamic_contract_ports(
    node: &GraphNode,
) -> Result<DynamicContractPorts, EffectiveDefinitionError> {
    let mut overlay = DynamicContractPorts::default();
    if node.node_type == GENERIC_INFERENCE_NODE_TYPE {
        if let Some(snapshot_value) = node.data.get(INFERENCE_INTERFACE_SNAPSHOT_FIELD) {
            return inference_snapshot_contract_ports(snapshot_value);
        }
    }

    let Some(dynamic_definition) = node.data.get("definition") else {
        return Ok(overlay);
    };

    if node.node_type == GENERIC_INFERENCE_NODE_TYPE {
        return Err(EffectiveDefinitionError::InvalidDynamicDefinition {
            message: INFERENCE_DYNAMIC_DEFINITION_REJECTION.to_string(),
        });
    }

    if let Some(dynamic_node_type) = dynamic_definition.get("node_type").and_then(|v| v.as_str()) {
        if dynamic_node_type != node.node_type {
            overlay.warnings.push(ContractResolutionWarning {
                code: "dynamic_node_type_mismatch".to_string(),
                message: format!(
                    "dynamic definition node_type '{}' does not match node type '{}'",
                    dynamic_node_type, node.node_type
                ),
            });
            return Ok(overlay);
        }
    }

    overlay.inputs = parse_ports(dynamic_definition.get("inputs"), PortKind::Input, "inputs")?;
    overlay.outputs = parse_ports(
        dynamic_definition.get("outputs"),
        PortKind::Output,
        "outputs",
    )?;
    Ok(overlay)
}

#[derive(Default)]
struct DynamicContractPorts {
    inputs: Option<Vec<PortContract>>,
    outputs: Option<Vec<PortContract>>,
    warnings: Vec<ContractResolutionWarning>,
}

fn inference_snapshot_contract_ports(
    value: &Value,
) -> Result<DynamicContractPorts, EffectiveDefinitionError> {
    let snapshot = serde_json::from_value::<AuthoredInferenceInterfaceSnapshot>(value.clone())
        .map_err(|error| EffectiveDefinitionError::InvalidDynamicDefinition {
            message: format!("node.data.{INFERENCE_INTERFACE_SNAPSHOT_FIELD} is invalid: {error}"),
        })?;
    let snapshot =
        ValidatedAuthoredInferenceInterfaceSnapshot::try_from(snapshot).map_err(|error| {
            EffectiveDefinitionError::InvalidDynamicDefinition {
                message: format!(
                    "node.data.{INFERENCE_INTERFACE_SNAPSHOT_FIELD} is invalid: {error}"
                ),
            }
        })?;
    let snapshot = snapshot.as_snapshot();
    Ok(DynamicContractPorts {
        inputs: Some(snapshot_ports_to_contracts(
            &snapshot.inputs,
            InferencePortDirection::Input,
            PortKind::Input,
        )?),
        outputs: Some(snapshot_ports_to_contracts(
            &snapshot.outputs,
            InferencePortDirection::Output,
            PortKind::Output,
        )?),
        warnings: Vec::new(),
    })
}

fn snapshot_ports_to_contracts(
    ports: &[AuthoredInferencePortSnapshot],
    expected_direction: InferencePortDirection,
    kind: PortKind,
) -> Result<Vec<PortContract>, EffectiveDefinitionError> {
    ports
        .iter()
        .filter(|port| port.direction == expected_direction)
        .map(|port| snapshot_port_to_contract(port, kind))
        .collect()
}

fn snapshot_port_to_contract(
    port: &AuthoredInferencePortSnapshot,
    kind: PortKind,
) -> Result<PortContract, EffectiveDefinitionError> {
    let port_id = PortId::try_from(port.port_id.as_str().to_string()).map_err(|error| {
        EffectiveDefinitionError::InvalidDynamicDefinition {
            message: format!(
                "node.data.{INFERENCE_INTERFACE_SNAPSHOT_FIELD} port '{}' is invalid: {error}",
                port.port_id.as_str()
            ),
        }
    })?;
    let contract = PortContract {
        id: port_id,
        kind,
        label: port.label.clone(),
        value_type: inference_value_type_to_port_value_type(&port.value_type)?,
        requirement: inference_requirement_to_port_requirement(port.requirement)?,
        cardinality: PortCardinality::Single,
        visibility: PortVisibility::Public,
        constraints: Vec::new(),
        editor_hints: Vec::new(),
        inference_payloads: Vec::new(),
        options_provider: None,
    };
    contract
        .validate()
        .map_err(|error| EffectiveDefinitionError::InvalidDynamicDefinition {
            message: format!(
                "node.data.{INFERENCE_INTERFACE_SNAPSHOT_FIELD} port '{}' is invalid: {error}",
                port.port_id.as_str()
            ),
        })?;
    Ok(contract)
}

fn inference_requirement_to_port_requirement(
    requirement: InferencePortRequirement,
) -> Result<PortRequirement, EffectiveDefinitionError> {
    match requirement {
        InferencePortRequirement::Required => Ok(PortRequirement::Required),
        InferencePortRequirement::Optional => Ok(PortRequirement::Optional),
        _ => Err(EffectiveDefinitionError::InvalidDynamicDefinition {
            message: "authored inference snapshot uses an unsupported port requirement".to_string(),
        }),
    }
}

fn inference_value_type_to_port_value_type(
    value_type: &InferenceValueType,
) -> Result<PortValueType, EffectiveDefinitionError> {
    let value_type = match value_type {
        InferenceValueType::Scalar(InferenceScalarType::String) => PortValueType::String,
        InferenceValueType::Scalar(InferenceScalarType::Bool) => PortValueType::Boolean,
        InferenceValueType::Scalar(
            InferenceScalarType::I64 | InferenceScalarType::U64 | InferenceScalarType::F64,
        ) => PortValueType::Number,
        InferenceValueType::Artifact(InferenceArtifactType::Image) => PortValueType::Image,
        InferenceValueType::Artifact(InferenceArtifactType::Audio) => PortValueType::Audio,
        InferenceValueType::Artifact(InferenceArtifactType::Tensor) => PortValueType::Tensor,
        InferenceValueType::Artifact(InferenceArtifactType::Document) => PortValueType::Document,
        InferenceValueType::Artifact(
            InferenceArtifactType::Video | InferenceArtifactType::Media,
        ) => PortValueType::Json,
        InferenceValueType::Reference(
            InferenceReferenceType::PumasModel
            | InferenceReferenceType::MediaArtifact
            | InferenceReferenceType::RuntimeArtifact
            | InferenceReferenceType::SchedulerTaskResult,
        ) => PortValueType::Json,
        InferenceValueType::Constraint(
            InferenceConstraintType::Runtime
            | InferenceConstraintType::Device
            | InferenceConstraintType::DenoisingScheduler
            | InferenceConstraintType::SamplingMethod,
        ) => PortValueType::String,
        _ => {
            return Err(EffectiveDefinitionError::InvalidDynamicDefinition {
                message: "authored inference snapshot uses an unsupported port value type"
                    .to_string(),
            });
        }
    };
    Ok(value_type)
}

fn parse_node_instance_id(node_id: &str) -> Result<NodeInstanceId, EffectiveDefinitionError> {
    node_id
        .parse::<NodeInstanceId>()
        .map_err(|error| EffectiveDefinitionError::InvalidNodeId {
            node_id: node_id.to_string(),
            message: error.to_string(),
        })
}

fn parse_node_type_id(node_type: &str) -> Result<NodeTypeId, EffectiveDefinitionError> {
    node_type
        .parse::<NodeTypeId>()
        .map_err(|error| EffectiveDefinitionError::InvalidNodeType {
            node_type: node_type.to_string(),
            message: error.to_string(),
        })
}

fn parse_ports(
    value: Option<&Value>,
    kind: PortKind,
    field: &'static str,
) -> Result<Option<Vec<PortContract>>, EffectiveDefinitionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let ports = serde_json::from_value::<Vec<PortDefinition>>(value.clone()).map_err(|error| {
        EffectiveDefinitionError::InvalidDynamicDefinition {
            message: format!("node.data.definition.{field} is invalid: {error}"),
        }
    })?;
    ports
        .into_iter()
        .map(|port| workflow_port_to_contract(port, kind))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn workflow_port_to_contract(
    port: PortDefinition,
    kind: PortKind,
) -> Result<PortContract, EffectiveDefinitionError> {
    let port_id = port.id.clone();
    port.to_contract_port(kind).map_err(|error| {
        EffectiveDefinitionError::InvalidDynamicDefinition {
            message: format!("dynamic port '{port_id}' is invalid: {error}"),
        }
    })
}
