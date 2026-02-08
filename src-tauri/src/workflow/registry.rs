//! Node registry - manages available node type definitions
//!
//! The registry stores node definitions for the UI palette.
//! Definitions are sourced from node-engine's inventory-based registry,
//! creating a single source of truth for node metadata.

use std::collections::HashMap;

use super::types::{ExecutionMode, NodeCategory, NodeDefinition, PortDataType, PortDefinition};

/// Convert node-engine's TaskMetadata to src-tauri's NodeDefinition
fn convert_metadata(meta: node_engine::TaskMetadata) -> NodeDefinition {
    NodeDefinition {
        node_type: meta.node_type,
        category: convert_category(meta.category),
        label: meta.label,
        description: meta.description,
        inputs: meta.inputs.into_iter().map(convert_port).collect(),
        outputs: meta.outputs.into_iter().map(convert_port).collect(),
        execution_mode: convert_execution_mode(meta.execution_mode),
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
        // Map additional node-engine types to closest match
        node_engine::PortDataType::ModelHandle => PortDataType::String,
        node_engine::PortDataType::EmbeddingHandle => PortDataType::String,
        node_engine::PortDataType::DatabaseHandle => PortDataType::String,
        node_engine::PortDataType::Vector => PortDataType::Embedding,
        node_engine::PortDataType::Tensor => PortDataType::Json,
        node_engine::PortDataType::AudioSamples => PortDataType::Audio,
    }
}

/// Registry of available node types
///
/// Stores node definitions and provides them to the frontend for
/// the node palette. Definitions are sourced from node-engine's
/// inventory-based registry.
pub struct NodeRegistry {
    definitions: HashMap<String, NodeDefinition>,
}

impl NodeRegistry {
    /// Create a new registry with all built-in node definitions
    pub fn new() -> Self {
        let mut definitions = HashMap::new();

        // Single source of truth: discover all nodes via inventory
        let engine_registry = node_engine::NodeRegistry::with_builtins();
        for meta in engine_registry.all_metadata() {
            let def = convert_metadata(meta.clone());
            definitions.insert(def.node_type.clone(), def);
        }

        Self { definitions }
    }

    /// Get a node definition by type
    pub fn get_definition(&self, node_type: &str) -> Option<&NodeDefinition> {
        self.definitions.get(node_type)
    }

    /// Get all registered node definitions
    pub fn all_definitions(&self) -> Vec<NodeDefinition> {
        self.definitions.values().cloned().collect()
    }

    /// Get definitions grouped by category
    pub fn definitions_by_category(&self) -> HashMap<String, Vec<NodeDefinition>> {
        let mut grouped: HashMap<String, Vec<NodeDefinition>> = HashMap::new();

        for def in self.definitions.values() {
            let category = format!("{:?}", def.category).to_lowercase();
            grouped.entry(category).or_default().push(def.clone());
        }

        grouped
    }

    /// Check if a node type is registered
    pub fn has_node_type(&self, node_type: &str) -> bool {
        self.definitions.contains_key(node_type)
    }

    /// Get the number of registered node types
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_builtin_nodes() {
        let registry = NodeRegistry::new();

        // Nodes from TaskDescriptor (workflow-nodes crate)
        assert!(registry.has_node_type("text-input"));
        assert!(registry.has_node_type("image-input"));
        assert!(registry.has_node_type("human-input"));
        assert!(registry.has_node_type("vector-db"));
        assert!(registry.has_node_type("linked-input"));
        assert!(registry.has_node_type("model-provider"));
        assert!(registry.has_node_type("llm-inference"));
        assert!(registry.has_node_type("ollama-inference"));
        assert!(registry.has_node_type("vision-analysis"));
        assert!(registry.has_node_type("embedding"));
        assert!(registry.has_node_type("lancedb"));
        assert!(registry.has_node_type("text-output"));
        assert!(registry.has_node_type("component-preview"));
        assert!(registry.has_node_type("read-file"));
        assert!(registry.has_node_type("write-file"));
        assert!(registry.has_node_type("tool-loop"));

        // New control nodes
        assert!(registry.has_node_type("tool-executor"));
        assert!(registry.has_node_type("conditional"));
        assert!(registry.has_node_type("merge"));

        // New processing nodes
        assert!(registry.has_node_type("validator"));
        assert!(registry.has_node_type("json-filter"));

        // Tauri-only nodes
        assert!(registry.has_node_type("puma-lib"));
        assert!(registry.has_node_type("agent-tools"));
        assert!(registry.has_node_type("llamacpp-inference"));
    }

    #[test]
    fn test_registry_get_definition() {
        let registry = NodeRegistry::new();

        let def = registry.get_definition("text-input").unwrap();
        assert_eq!(def.node_type, "text-input");
        assert_eq!(def.category, NodeCategory::Input);
    }

    #[test]
    fn test_registry_all_definitions() {
        let registry = NodeRegistry::new();

        let all = registry.all_definitions();
        assert!(!all.is_empty());
        assert!(all.len() >= 10); // At least 10 built-in nodes
    }

    #[test]
    fn test_registry_definitions_by_category() {
        let registry = NodeRegistry::new();

        let grouped = registry.definitions_by_category();

        assert!(grouped.contains_key("input"));
        assert!(grouped.contains_key("processing"));
        assert!(grouped.contains_key("output"));
        assert!(grouped.contains_key("tool"));
        assert!(grouped.contains_key("control"));
    }

    #[test]
    fn test_descriptor_conversion() {
        // Verify that inventory-based conversion produces correct output
        let registry = NodeRegistry::new();
        let def = registry.get_definition("text-input").unwrap();

        assert_eq!(def.node_type, "text-input");
        assert_eq!(def.category, NodeCategory::Input);
        assert_eq!(def.inputs.len(), 1);
        assert_eq!(def.inputs[0].id, "text");
        assert_eq!(def.outputs.len(), 1);
        assert_eq!(def.outputs[0].id, "text");
    }
}
