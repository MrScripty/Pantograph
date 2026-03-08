use std::collections::HashMap;

use super::types::{
    ExecutionMode, IoBindingOrigin, NodeCategory, NodeDefinition, PortDataType, PortDefinition,
};

fn convert_metadata(meta: node_engine::TaskMetadata) -> NodeDefinition {
    let category = convert_category(meta.category);
    NodeDefinition {
        node_type: meta.node_type.clone(),
        category: category.clone(),
        label: meta.label,
        description: meta.description,
        io_binding_origin: determine_io_binding_origin(&meta.node_type, &category),
        inputs: meta.inputs.into_iter().map(convert_port).collect(),
        outputs: meta.outputs.into_iter().map(convert_port).collect(),
        execution_mode: convert_execution_mode(meta.execution_mode),
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

fn convert_category(cat: node_engine::NodeCategory) -> NodeCategory {
    match cat {
        node_engine::NodeCategory::Input => NodeCategory::Input,
        node_engine::NodeCategory::Output => NodeCategory::Output,
        node_engine::NodeCategory::Processing => NodeCategory::Processing,
        node_engine::NodeCategory::Control => NodeCategory::Control,
        node_engine::NodeCategory::Tool => NodeCategory::Tool,
    }
}

fn convert_execution_mode(mode: node_engine::ExecutionMode) -> ExecutionMode {
    match mode {
        node_engine::ExecutionMode::Batch => ExecutionMode::Reactive,
        node_engine::ExecutionMode::Stream => ExecutionMode::Stream,
        node_engine::ExecutionMode::Reactive => ExecutionMode::Reactive,
        node_engine::ExecutionMode::Manual => ExecutionMode::Manual,
    }
}

fn convert_port(port: node_engine::PortMetadata) -> PortDefinition {
    PortDefinition {
        id: port.id,
        label: port.label,
        data_type: convert_data_type(port.data_type),
        required: port.required,
        multiple: port.multiple,
    }
}

fn convert_data_type(dt: node_engine::PortDataType) -> PortDataType {
    match dt {
        node_engine::PortDataType::Any => PortDataType::Any,
        node_engine::PortDataType::String => PortDataType::String,
        node_engine::PortDataType::Image => PortDataType::Image,
        node_engine::PortDataType::Audio => PortDataType::Audio,
        node_engine::PortDataType::AudioStream => PortDataType::AudioStream,
        node_engine::PortDataType::Component => PortDataType::Component,
        node_engine::PortDataType::Stream => PortDataType::Stream,
        node_engine::PortDataType::Prompt => PortDataType::Prompt,
        node_engine::PortDataType::Tools => PortDataType::Tools,
        node_engine::PortDataType::Embedding => PortDataType::Embedding,
        node_engine::PortDataType::Document => PortDataType::Document,
        node_engine::PortDataType::Json => PortDataType::Json,
        node_engine::PortDataType::Boolean => PortDataType::Boolean,
        node_engine::PortDataType::Number => PortDataType::Number,
        node_engine::PortDataType::VectorDb => PortDataType::VectorDb,
        node_engine::PortDataType::ModelHandle => PortDataType::String,
        node_engine::PortDataType::EmbeddingHandle => PortDataType::String,
        node_engine::PortDataType::DatabaseHandle => PortDataType::String,
        node_engine::PortDataType::Vector => PortDataType::Embedding,
        node_engine::PortDataType::Tensor => PortDataType::Json,
        node_engine::PortDataType::AudioSamples => PortDataType::Audio,
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
        let engine_registry = node_engine::NodeRegistry::with_builtins();
        for meta in engine_registry.all_metadata() {
            let def = convert_metadata(meta.clone());
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
