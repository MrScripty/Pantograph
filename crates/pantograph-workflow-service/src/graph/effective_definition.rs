use serde_json::Value;

use pantograph_node_contracts::{
    ContractResolutionWarning, EffectiveNodeContract, NodeInstanceContext, NodeInstanceId,
    NodeTypeId, PortContract, PortKind,
};

use super::registry::{convert_port, NodeRegistry};
use super::types::{GraphNode, NodeDefinition, PortDefinition};

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
    let Some(dynamic_definition) = node.data.get("definition") else {
        return Ok(overlay);
    };

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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::graph::{PortDataType, Position};
    use pantograph_node_contracts::ContractExpansionReason;

    #[test]
    fn effective_node_definition_merges_dynamic_ports_without_dropping_static_ports() {
        let registry = NodeRegistry::new();
        let node = GraphNode {
            id: "llm".to_string(),
            node_type: "llm-inference".to_string(),
            position: Position::default(),
            data: json!({
                "definition": {
                    "node_type": "llm-inference",
                    "inputs": [
                        {
                            "id": "temperature",
                            "label": "Temperature",
                            "data_type": "number",
                            "required": false,
                            "multiple": false
                        }
                    ]
                }
            }),
        };

        let definition = effective_node_definition(&node, &registry).expect("definition");

        assert!(
            definition.inputs.iter().any(|port| port.id == "prompt"),
            "static prompt input must remain available"
        );
        assert_eq!(
            definition
                .inputs
                .iter()
                .find(|port| port.id == "temperature")
                .map(|port| &port.data_type),
            Some(&PortDataType::Number)
        );
    }

    #[test]
    fn effective_node_contract_reports_mismatched_dynamic_definition() {
        let registry = NodeRegistry::new();
        let node = GraphNode {
            id: "llm".to_string(),
            node_type: "llm-inference".to_string(),
            position: Position::default(),
            data: json!({
                "definition": {
                    "node_type": "text-input",
                    "inputs": [
                        {
                            "id": "temperature",
                            "label": "Temperature",
                            "data_type": "number"
                        }
                    ]
                }
            }),
        };

        let effective = effective_node_contract(&node, &registry).expect("contract");

        assert!(
            effective
                .inputs
                .iter()
                .all(|port| port.base.id.as_str() != "temperature"),
            "mismatched dynamic definition must not add ports"
        );
        assert_eq!(
            effective.diagnostics.warnings[0].code,
            "dynamic_node_type_mismatch"
        );
    }

    #[test]
    fn effective_node_contract_records_dynamic_expansion_reason() {
        let registry = NodeRegistry::new();
        let node = GraphNode {
            id: "llm".to_string(),
            node_type: "llm-inference".to_string(),
            position: Position::default(),
            data: json!({
                "definition": {
                    "node_type": "llm-inference",
                    "inputs": [
                        {
                            "id": "temperature",
                            "label": "Temperature",
                            "data_type": "number"
                        }
                    ]
                }
            }),
        };

        let effective = effective_node_contract(&node, &registry).expect("contract");

        assert_eq!(
            effective.diagnostics.expansion_reasons,
            vec![ContractExpansionReason::DynamicConfiguration]
        );
        assert_eq!(
            effective
                .inputs
                .last()
                .expect("dynamic port")
                .expansion_reasons,
            vec![ContractExpansionReason::DynamicConfiguration]
        );
    }
}
