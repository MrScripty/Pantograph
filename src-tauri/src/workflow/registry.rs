//! Node registry - manages available node types
//!
//! The registry stores node definitions and creates node instances
//! for workflow execution.

use std::collections::HashMap;

use super::node::{Node, NodeError};
use super::nodes::{
    ComponentPreviewNode, ImageInputNode, LLMInferenceNode, RAGSearchNode, ReadFileNode,
    TextInputNode, TextOutputNode, ToolLoopNode, VisionAnalysisNode, WriteFileNode,
};
use super::types::NodeDefinition;

/// Registry of available node types
///
/// Stores node definitions and provides factory methods for creating
/// node instances during workflow execution.
pub struct NodeRegistry {
    definitions: HashMap<String, NodeDefinition>,
}

impl NodeRegistry {
    /// Create a new registry with all built-in nodes registered
    pub fn new() -> Self {
        let mut definitions = HashMap::new();

        // Input nodes
        Self::register(&mut definitions, TextInputNode::definition());
        Self::register(&mut definitions, ImageInputNode::definition());

        // Processing nodes
        Self::register(&mut definitions, LLMInferenceNode::definition());
        Self::register(&mut definitions, VisionAnalysisNode::definition());
        Self::register(&mut definitions, RAGSearchNode::definition());

        // Output nodes
        Self::register(&mut definitions, TextOutputNode::definition());
        Self::register(&mut definitions, ComponentPreviewNode::definition());

        // Tool nodes
        Self::register(&mut definitions, ReadFileNode::definition());
        Self::register(&mut definitions, WriteFileNode::definition());

        // Control nodes
        Self::register(&mut definitions, ToolLoopNode::definition());

        Self { definitions }
    }

    /// Register a node definition
    fn register(map: &mut HashMap<String, NodeDefinition>, def: NodeDefinition) {
        map.insert(def.node_type.clone(), def);
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
            grouped
                .entry(category)
                .or_default()
                .push(def.clone());
        }

        grouped
    }

    /// Create a node instance by type
    ///
    /// # Arguments
    /// * `node_type` - The type of node to create (e.g., "text-input")
    /// * `id` - The instance ID for the node
    ///
    /// # Returns
    /// A boxed node instance, or an error if the type is unknown
    pub fn create_node(&self, node_type: &str, id: &str) -> Result<Box<dyn Node>, NodeError> {
        match node_type {
            // Input nodes
            "text-input" => Ok(Box::new(TextInputNode::new(id))),
            "image-input" => Ok(Box::new(ImageInputNode::new(id))),

            // Processing nodes
            "llm-inference" => Ok(Box::new(LLMInferenceNode::new(id))),
            "vision-analysis" => Ok(Box::new(VisionAnalysisNode::new(id))),
            "rag-search" => Ok(Box::new(RAGSearchNode::new(id))),

            // Output nodes
            "text-output" => Ok(Box::new(TextOutputNode::new(id))),
            "component-preview" => Ok(Box::new(ComponentPreviewNode::new(id))),

            // Tool nodes
            "read-file" => Ok(Box::new(ReadFileNode::new(id))),
            "write-file" => Ok(Box::new(WriteFileNode::new(id))),

            // Control nodes
            "tool-loop" => Ok(Box::new(ToolLoopNode::new(id))),

            // Unknown type
            _ => Err(NodeError::ExecutionFailed(format!(
                "Unknown node type: {}",
                node_type
            ))),
        }
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
    use crate::workflow::types::NodeCategory;

    #[test]
    fn test_registry_has_builtin_nodes() {
        let registry = NodeRegistry::new();

        assert!(registry.has_node_type("text-input"));
        assert!(registry.has_node_type("image-input"));
        assert!(registry.has_node_type("llm-inference"));
        assert!(registry.has_node_type("vision-analysis"));
        assert!(registry.has_node_type("rag-search"));
        assert!(registry.has_node_type("text-output"));
        assert!(registry.has_node_type("component-preview"));
        assert!(registry.has_node_type("read-file"));
        assert!(registry.has_node_type("write-file"));
        assert!(registry.has_node_type("tool-loop"));
    }

    #[test]
    fn test_registry_get_definition() {
        let registry = NodeRegistry::new();

        let def = registry.get_definition("text-input").unwrap();
        assert_eq!(def.node_type, "text-input");
        assert_eq!(def.category, NodeCategory::Input);
    }

    #[test]
    fn test_registry_create_node() {
        let registry = NodeRegistry::new();

        let node = registry.create_node("text-input", "test-1").unwrap();
        assert_eq!(node.id(), "test-1");
        assert_eq!(node.definition().node_type, "text-input");
    }

    #[test]
    fn test_registry_unknown_type() {
        let registry = NodeRegistry::new();

        let result = registry.create_node("unknown-type", "test-1");
        assert!(result.is_err());
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
}
