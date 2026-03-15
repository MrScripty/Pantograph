use serde_json::Value;

use super::registry::NodeRegistry;
use super::types::{GraphNode, NodeDefinition, PortDefinition};

pub enum EffectiveDefinitionError {
    UnknownNodeType(String),
}

pub fn effective_node_definition(
    node: &GraphNode,
    registry: &NodeRegistry,
) -> Result<NodeDefinition, EffectiveDefinitionError> {
    let mut definition = registry
        .get_definition(&node.node_type)
        .cloned()
        .ok_or_else(|| EffectiveDefinitionError::UnknownNodeType(node.node_type.clone()))?;

    let Some(dynamic_definition) = node.data.get("definition") else {
        return Ok(definition);
    };

    if let Some(dynamic_node_type) = dynamic_definition.get("node_type").and_then(|v| v.as_str()) {
        if dynamic_node_type != node.node_type {
            return Ok(definition);
        }
    }

    if let Some(inputs) = parse_ports(dynamic_definition.get("inputs")) {
        definition.inputs = inputs;
    }

    if let Some(outputs) = parse_ports(dynamic_definition.get("outputs")) {
        definition.outputs = outputs;
    }

    Ok(definition)
}

fn parse_ports(value: Option<&Value>) -> Option<Vec<PortDefinition>> {
    serde_json::from_value(value?.clone()).ok()
}
