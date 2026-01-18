//! Graph validation for workflows
//!
//! Validates workflow graphs for:
//! - Cycle detection (using Kahn's algorithm)
//! - Type compatibility between connected ports
//! - Required input connections
//! - Valid node types

use std::collections::{HashMap, HashSet, VecDeque};

use super::registry::NodeRegistry;
use super::types::{PortDataType, WorkflowGraph};

/// Errors that can occur during graph validation
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Cycle detected in workflow graph")]
    CycleDetected,

    #[error("Node '{node_id}' has unconnected required input '{port}'")]
    UnconnectedInput { node_id: String, port: String },

    #[error("Type mismatch on edge '{edge_id}': {source_type:?} cannot connect to {target_type:?}")]
    TypeMismatch {
        edge_id: String,
        source_type: PortDataType,
        target_type: PortDataType,
    },

    #[error("Unknown node type: {0}")]
    UnknownNodeType(String),

    #[error("Unknown port '{port}' on node '{node_id}'")]
    UnknownPort { node_id: String, port: String },

    #[error("Node not found: {0}")]
    NodeNotFound(String),
}

/// Validates workflow graphs
pub struct WorkflowValidator<'a> {
    registry: &'a NodeRegistry,
}

impl<'a> WorkflowValidator<'a> {
    /// Create a new validator with access to the node registry
    pub fn new(registry: &'a NodeRegistry) -> Self {
        Self { registry }
    }

    /// Validate an entire workflow graph
    ///
    /// Performs the following checks in order:
    /// 1. All node types are known
    /// 2. No cycles in the graph
    /// 3. All required inputs are connected
    /// 4. All edge types are compatible
    pub fn validate(&self, graph: &WorkflowGraph) -> Result<(), ValidationError> {
        self.validate_node_types(graph)?;
        self.detect_cycles(graph)?;
        self.validate_required_inputs(graph)?;
        self.validate_edge_types(graph)?;
        Ok(())
    }

    /// Check that all nodes have known types
    fn validate_node_types(&self, graph: &WorkflowGraph) -> Result<(), ValidationError> {
        for node in &graph.nodes {
            if self.registry.get_definition(&node.node_type).is_none() {
                return Err(ValidationError::UnknownNodeType(node.node_type.clone()));
            }
        }
        Ok(())
    }

    /// Detect cycles using Kahn's algorithm (topological sort)
    ///
    /// If we can't visit all nodes through topological sort,
    /// there must be a cycle.
    fn detect_cycles(&self, graph: &WorkflowGraph) -> Result<(), ValidationError> {
        // Build adjacency list and in-degree map
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

        // Initialize all nodes with 0 in-degree
        for node in &graph.nodes {
            in_degree.insert(&node.id, 0);
            adjacency.insert(&node.id, Vec::new());
        }

        // Build the graph from edges
        for edge in &graph.edges {
            if let Some(adj) = adjacency.get_mut(edge.source.as_str()) {
                adj.push(&edge.target);
            }
            if let Some(degree) = in_degree.get_mut(edge.target.as_str()) {
                *degree += 1;
            }
        }

        // Start with nodes that have no incoming edges
        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut visited = 0;

        // Process nodes in topological order
        while let Some(node) = queue.pop_front() {
            visited += 1;

            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        // If we didn't visit all nodes, there's a cycle
        if visited != graph.nodes.len() {
            return Err(ValidationError::CycleDetected);
        }

        Ok(())
    }

    /// Check that all required inputs have connections
    fn validate_required_inputs(&self, graph: &WorkflowGraph) -> Result<(), ValidationError> {
        // Build set of connected inputs: (node_id, port_id)
        let connected_inputs: HashSet<(&str, &str)> = graph
            .edges
            .iter()
            .map(|e| (e.target.as_str(), e.target_handle.as_str()))
            .collect();

        for node in &graph.nodes {
            let definition = self
                .registry
                .get_definition(&node.node_type)
                .ok_or_else(|| ValidationError::UnknownNodeType(node.node_type.clone()))?;

            for input in &definition.inputs {
                if input.required
                    && !connected_inputs.contains(&(node.id.as_str(), input.id.as_str()))
                {
                    // Check if the input is provided via node data
                    let has_data_value = node
                        .data
                        .as_object()
                        .map(|obj| obj.contains_key(&input.id))
                        .unwrap_or(false);

                    if !has_data_value {
                        return Err(ValidationError::UnconnectedInput {
                            node_id: node.id.clone(),
                            port: input.id.clone(),
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate that all edges connect compatible port types
    fn validate_edge_types(&self, graph: &WorkflowGraph) -> Result<(), ValidationError> {
        for edge in &graph.edges {
            let source_node = graph
                .find_node(&edge.source)
                .ok_or_else(|| ValidationError::NodeNotFound(edge.source.clone()))?;

            let target_node = graph
                .find_node(&edge.target)
                .ok_or_else(|| ValidationError::NodeNotFound(edge.target.clone()))?;

            let source_def = self
                .registry
                .get_definition(&source_node.node_type)
                .ok_or_else(|| ValidationError::UnknownNodeType(source_node.node_type.clone()))?;

            let target_def = self
                .registry
                .get_definition(&target_node.node_type)
                .ok_or_else(|| ValidationError::UnknownNodeType(target_node.node_type.clone()))?;

            let source_port = source_def
                .outputs
                .iter()
                .find(|p| p.id == edge.source_handle)
                .ok_or_else(|| ValidationError::UnknownPort {
                    node_id: source_node.id.clone(),
                    port: edge.source_handle.clone(),
                })?;

            let target_port = target_def
                .inputs
                .iter()
                .find(|p| p.id == edge.target_handle)
                .ok_or_else(|| ValidationError::UnknownPort {
                    node_id: target_node.id.clone(),
                    port: edge.target_handle.clone(),
                })?;

            if !source_port.data_type.is_compatible_with(&target_port.data_type) {
                return Err(ValidationError::TypeMismatch {
                    edge_id: edge.id.clone(),
                    source_type: source_port.data_type.clone(),
                    target_type: target_port.data_type.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Check if a single connection between two port types is valid
///
/// This is used by the frontend to validate connections as they're made.
pub fn validate_connection(source_type: &PortDataType, target_type: &PortDataType) -> bool {
    source_type.is_compatible_with(target_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::types::{GraphEdge, GraphNode, Position};

    fn create_test_registry() -> NodeRegistry {
        NodeRegistry::new()
    }

    #[test]
    fn test_cycle_detection_no_cycle() {
        let registry = create_test_registry();
        let validator = WorkflowValidator::new(&registry);

        // A -> B -> C (no cycle)
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "a".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::json!({"text": "test"}),
                },
                GraphNode {
                    id: "b".into(),
                    node_type: "text-output".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![GraphEdge {
                id: "e1".into(),
                source: "a".into(),
                source_handle: "text".into(),
                target: "b".into(),
                target_handle: "text".into(),
            }],
        };

        assert!(validator.detect_cycles(&graph).is_ok());
    }

    #[test]
    fn test_cycle_detection_with_cycle() {
        let registry = create_test_registry();
        let validator = WorkflowValidator::new(&registry);

        // A -> B -> A (cycle!)
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "a".into(),
                    node_type: "text-input".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
                GraphNode {
                    id: "b".into(),
                    node_type: "text-output".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    source_handle: "text".into(),
                    target: "b".into(),
                    target_handle: "text".into(),
                },
                GraphEdge {
                    id: "e2".into(),
                    source: "b".into(),
                    source_handle: "text".into(),
                    target: "a".into(),
                    target_handle: "text".into(),
                },
            ],
        };

        assert!(matches!(
            validator.detect_cycles(&graph),
            Err(ValidationError::CycleDetected)
        ));
    }

    #[test]
    fn test_validate_connection_compatible() {
        assert!(validate_connection(
            &PortDataType::String,
            &PortDataType::String
        ));
        assert!(validate_connection(
            &PortDataType::String,
            &PortDataType::Prompt
        ));
        assert!(validate_connection(
            &PortDataType::Any,
            &PortDataType::String
        ));
    }

    #[test]
    fn test_validate_connection_incompatible() {
        assert!(!validate_connection(
            &PortDataType::Image,
            &PortDataType::String
        ));
        assert!(!validate_connection(
            &PortDataType::Number,
            &PortDataType::Image
        ));
    }
}
