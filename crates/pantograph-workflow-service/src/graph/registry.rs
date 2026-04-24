use std::collections::HashMap;

use super::types::{
    ExecutionMode, IoBindingOrigin, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

fn convert_contract(contract: pantograph_node_contracts::NodeTypeContract) -> NodeDefinition {
    let category = convert_category(contract.category);
    NodeDefinition {
        node_type: contract.node_type.as_str().to_string(),
        category: category.clone(),
        label: contract.label,
        description: contract.description,
        io_binding_origin: determine_io_binding_origin(contract.node_type.as_str(), &category),
        inputs: contract.inputs.into_iter().map(convert_port).collect(),
        outputs: contract.outputs.into_iter().map(convert_port).collect(),
        execution_mode: convert_execution_mode(contract.execution_semantics),
    }
}

fn determine_io_binding_origin(node_type: &str, category: &NodeCategory) -> IoBindingOrigin {
    if !matches!(category, NodeCategory::Input | NodeCategory::Output) {
        return IoBindingOrigin::Integrated;
    }

    match node_type {
        "puma-lib" | "linked-input" | "model-provider" | "component-preview"
        | "point-cloud-output" => IoBindingOrigin::Integrated,
        "audio-input" | "boolean-input" | "human-input" | "image-input" | "masked-text-input"
        | "number-input" | "selection-input" | "text-input" | "vector-input" | "audio-output"
        | "image-output" | "text-output" | "vector-output" => IoBindingOrigin::ClientSession,
        _ => panic!(
            "input/output node type '{}' is missing explicit io_binding_origin mapping",
            node_type
        ),
    }
}

fn convert_category(cat: pantograph_node_contracts::NodeCategory) -> NodeCategory {
    match cat {
        pantograph_node_contracts::NodeCategory::Input => NodeCategory::Input,
        pantograph_node_contracts::NodeCategory::Output => NodeCategory::Output,
        pantograph_node_contracts::NodeCategory::Processing => NodeCategory::Processing,
        pantograph_node_contracts::NodeCategory::Control => NodeCategory::Control,
        pantograph_node_contracts::NodeCategory::Tool => NodeCategory::Tool,
    }
}

fn convert_execution_mode(
    mode: pantograph_node_contracts::NodeExecutionSemantics,
) -> ExecutionMode {
    match mode {
        pantograph_node_contracts::NodeExecutionSemantics::Batch => ExecutionMode::Reactive,
        pantograph_node_contracts::NodeExecutionSemantics::Stream => ExecutionMode::Stream,
        pantograph_node_contracts::NodeExecutionSemantics::Reactive => ExecutionMode::Reactive,
        pantograph_node_contracts::NodeExecutionSemantics::Manual => ExecutionMode::Manual,
    }
}

fn convert_port(port: pantograph_node_contracts::PortContract) -> PortDefinition {
    PortDefinition {
        id: port.id.as_str().to_string(),
        label: port.label,
        data_type: PortDataType::from_contract_value_type(port.value_type),
        required: matches!(
            port.requirement,
            pantograph_node_contracts::PortRequirement::Required
        ),
        multiple: matches!(
            port.cardinality,
            pantograph_node_contracts::PortCardinality::Multiple
        ),
    }
}

pub fn validate_workflow_connection(
    source_type: &PortDataType,
    target_type: &PortDataType,
) -> bool {
    source_type.is_compatible_with(target_type)
}

pub struct NodeRegistry {
    definitions: HashMap<String, NodeDefinition>,
}

impl NodeRegistry {
    pub fn new() -> Self {
        let mut definitions = HashMap::new();
        let contracts = workflow_nodes::builtin_node_contracts()
            .expect("built-in workflow node descriptors must project to canonical contracts");
        for contract in contracts {
            let def = convert_contract(contract);
            definitions.insert(def.node_type.clone(), def);
        }
        Self { definitions }
    }

    pub fn get_definition(&self, node_type: &str) -> Option<&NodeDefinition> {
        self.definitions.get(node_type)
    }

    pub fn all_definitions(&self) -> Vec<NodeDefinition> {
        self.definitions.values().cloned().collect()
    }

    pub fn definitions_by_category(&self) -> HashMap<String, Vec<NodeDefinition>> {
        let mut grouped = HashMap::new();
        for def in self.definitions.values() {
            let category = format!("{:?}", def.category).to_lowercase();
            grouped
                .entry(category)
                .or_insert_with(Vec::new)
                .push(def.clone());
        }
        grouped
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
