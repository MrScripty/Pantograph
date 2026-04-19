//! Graph validation for workflow and orchestration graphs
//!
//! Validates graph structure, port types, required connections,
//! and detects cycles.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::orchestration::{OrchestrationGraph, OrchestrationNodeType};
use crate::registry::NodeRegistry;
use crate::types::WorkflowGraph;

/// Validation error with location context
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Cycle detected in the graph
    CycleDetected,
    /// A node has an unknown type (not in registry)
    UnknownNodeType { node_id: String, node_type: String },
    /// A required input port is not connected and has no default
    UnconnectedRequiredInput { node_id: String, port_id: String },
    /// An edge connects incompatible port types
    IncompatiblePortTypes {
        edge_id: String,
        source_type: String,
        target_type: String,
    },
    /// An edge references a non-existent node
    UnknownNode { edge_id: String, node_id: String },
    /// A node has no connections (orphaned)
    OrphanedNode { node_id: String },
    /// Orchestration graph is missing a Start node
    MissingStartNode,
    /// Orchestration graph is missing an End node
    MissingEndNode,
    /// Orchestration graph has multiple Start nodes
    MultipleStartNodes,
    /// A node has an unconnected required handle
    MissingRequiredHandle { node_id: String, handle: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CycleDetected => write!(f, "Cycle detected in graph"),
            Self::UnknownNodeType { node_id, node_type } => {
                write!(
                    f,
                    "Unknown node type '{}' for node '{}'",
                    node_type, node_id
                )
            }
            Self::UnconnectedRequiredInput { node_id, port_id } => {
                write!(
                    f,
                    "Required input '{}' on node '{}' is not connected",
                    port_id, node_id
                )
            }
            Self::IncompatiblePortTypes {
                edge_id,
                source_type,
                target_type,
            } => {
                write!(
                    f,
                    "Edge '{}' connects incompatible types: {} -> {}",
                    edge_id, source_type, target_type
                )
            }
            Self::UnknownNode { edge_id, node_id } => {
                write!(
                    f,
                    "Edge '{}' references unknown node '{}'",
                    edge_id, node_id
                )
            }
            Self::OrphanedNode { node_id } => {
                write!(f, "Node '{}' has no connections", node_id)
            }
            Self::MissingStartNode => write!(f, "Orchestration graph has no Start node"),
            Self::MissingEndNode => write!(f, "Orchestration graph has no End node"),
            Self::MultipleStartNodes => {
                write!(f, "Orchestration graph has multiple Start nodes")
            }
            Self::MissingRequiredHandle { node_id, handle } => {
                write!(
                    f,
                    "Node '{}' is missing required handle '{}'",
                    node_id, handle
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate a workflow (data) graph
///
/// Returns all validation errors found (not just the first).
/// Pass a registry to enable node type and port type validation.
pub fn validate_workflow(
    graph: &WorkflowGraph,
    registry: Option<&NodeRegistry>,
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    validate_edge_references(graph, &mut errors);
    detect_cycles(graph, &mut errors);

    if let Some(reg) = registry {
        validate_node_types(graph, reg, &mut errors);
        validate_port_types(graph, reg, &mut errors);
        validate_required_inputs(graph, reg, &mut errors);
    }

    errors
}

/// Check that each edge connects compatible output and input port types.
fn validate_port_types(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    errors: &mut Vec<ValidationError>,
) {
    for edge in &graph.edges {
        let Some(source_node) = graph.nodes.iter().find(|node| node.id == edge.source) else {
            continue;
        };
        let Some(target_node) = graph.nodes.iter().find(|node| node.id == edge.target) else {
            continue;
        };

        let Some(source_metadata) = registry.get_metadata(&source_node.node_type) else {
            continue;
        };
        let Some(target_metadata) = registry.get_metadata(&target_node.node_type) else {
            continue;
        };

        let Some(source_port) = source_metadata
            .outputs
            .iter()
            .find(|port| port.id == edge.source_handle)
        else {
            continue;
        };
        let Some(target_port) = target_metadata
            .inputs
            .iter()
            .find(|port| port.id == edge.target_handle)
        else {
            continue;
        };

        if !source_port
            .data_type
            .is_compatible_with(&target_port.data_type)
        {
            errors.push(ValidationError::IncompatiblePortTypes {
                edge_id: edge.id.clone(),
                source_type: format!("{:?}", source_port.data_type),
                target_type: format!("{:?}", target_port.data_type),
            });
        }
    }
}

/// Validate an orchestration graph
///
/// Checks orchestration-specific rules.
pub fn validate_orchestration(graph: &OrchestrationGraph) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    validate_start_end_presence(graph, &mut errors);
    detect_orchestration_cycles(graph, &mut errors);

    errors
}

/// Check that all edge source/target nodes exist
fn validate_edge_references(graph: &WorkflowGraph, errors: &mut Vec<ValidationError>) {
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    for edge in &graph.edges {
        if !node_ids.contains(edge.source.as_str()) {
            errors.push(ValidationError::UnknownNode {
                edge_id: edge.id.clone(),
                node_id: edge.source.clone(),
            });
        }
        if !node_ids.contains(edge.target.as_str()) {
            errors.push(ValidationError::UnknownNode {
                edge_id: edge.id.clone(),
                node_id: edge.target.clone(),
            });
        }
    }
}

/// Detect cycles using Kahn's algorithm (topological sort)
fn detect_cycles(graph: &WorkflowGraph, errors: &mut Vec<ValidationError>) {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for node in &graph.nodes {
        in_degree.insert(&node.id, 0);
    }
    for edge in &graph.edges {
        *in_degree.entry(&edge.target).or_insert(0) += 1;
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0;
    while let Some(node_id) = queue.pop_front() {
        visited += 1;
        for edge in &graph.edges {
            if edge.source == node_id {
                if let Some(deg) = in_degree.get_mut(edge.target.as_str()) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(&edge.target);
                    }
                }
            }
        }
    }

    if visited < graph.nodes.len() {
        errors.push(ValidationError::CycleDetected);
    }
}

/// Check that all nodes have known types in the registry
fn validate_node_types(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    errors: &mut Vec<ValidationError>,
) {
    for node in &graph.nodes {
        if !registry.has_node_type(&node.node_type) {
            errors.push(ValidationError::UnknownNodeType {
                node_id: node.id.clone(),
                node_type: node.node_type.clone(),
            });
        }
    }
}

/// Check that required inputs are connected or have defaults
fn validate_required_inputs(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    errors: &mut Vec<ValidationError>,
) {
    // Build set of connected input ports
    let mut connected_inputs: HashSet<(String, String)> = HashSet::new();
    for edge in &graph.edges {
        connected_inputs.insert((edge.target.clone(), edge.target_handle.clone()));
    }

    for node in &graph.nodes {
        if let Some(metadata) = registry.get_metadata(&node.node_type) {
            for port in &metadata.inputs {
                if port.required && !connected_inputs.contains(&(node.id.clone(), port.id.clone()))
                {
                    // Check if the node data has a value for this port
                    let has_data_value = !node.data.is_null() && node.data.get(&port.id).is_some();

                    if !has_data_value {
                        errors.push(ValidationError::UnconnectedRequiredInput {
                            node_id: node.id.clone(),
                            port_id: port.id.clone(),
                        });
                    }
                }
            }
        }
    }
}

/// Check Start/End node presence in orchestration graph
fn validate_start_end_presence(graph: &OrchestrationGraph, errors: &mut Vec<ValidationError>) {
    let start_count = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.node_type, OrchestrationNodeType::Start))
        .count();
    let end_count = graph
        .nodes
        .iter()
        .filter(|n| matches!(n.node_type, OrchestrationNodeType::End))
        .count();

    if start_count == 0 {
        errors.push(ValidationError::MissingStartNode);
    } else if start_count > 1 {
        errors.push(ValidationError::MultipleStartNodes);
    }

    if end_count == 0 {
        errors.push(ValidationError::MissingEndNode);
    }
}

/// Detect cycles in orchestration graph using Kahn's algorithm
fn detect_orchestration_cycles(graph: &OrchestrationGraph, errors: &mut Vec<ValidationError>) {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    for node in &graph.nodes {
        in_degree.insert(&node.id, 0);
    }
    for edge in &graph.edges {
        *in_degree.entry(&edge.target).or_insert(0) += 1;
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();

    let mut visited = 0;
    while let Some(node_id) = queue.pop_front() {
        visited += 1;
        for edge in &graph.edges {
            if edge.source == node_id {
                if let Some(deg) = in_degree.get_mut(edge.target.as_str()) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(&edge.target);
                    }
                }
            }
        }
    }

    if visited < graph.nodes.len() {
        errors.push(ValidationError::CycleDetected);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{OrchestrationBuilder, WorkflowBuilder};
    use crate::descriptor::{PortMetadata, TaskMetadata};
    use crate::registry::NodeRegistry;
    use crate::types::{ExecutionMode, NodeCategory, PortDataType};

    fn make_test_registry() -> NodeRegistry {
        let mut registry = NodeRegistry::new();
        registry.register_metadata(TaskMetadata {
            node_type: "text-input".to_string(),
            category: NodeCategory::Input,
            label: "Text Input".to_string(),
            description: "Text input node".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata::optional("text", "Text", PortDataType::String)],
            execution_mode: ExecutionMode::Reactive,
        });
        registry.register_metadata(TaskMetadata {
            node_type: "text-output".to_string(),
            category: NodeCategory::Output,
            label: "Text Output".to_string(),
            description: "Text output node".to_string(),
            inputs: vec![PortMetadata::required("text", "Text", PortDataType::String)],
            outputs: vec![],
            execution_mode: ExecutionMode::Reactive,
        });
        registry.register_metadata(TaskMetadata {
            node_type: "kv-source".to_string(),
            category: NodeCategory::Processing,
            label: "KV Source".to_string(),
            description: "Produces a KV-cache artifact".to_string(),
            inputs: vec![],
            outputs: vec![PortMetadata::optional(
                "kv_cache_out",
                "KV Cache Out",
                PortDataType::KvCache,
            )],
            execution_mode: ExecutionMode::Reactive,
        });
        registry.register_metadata(TaskMetadata {
            node_type: "kv-target".to_string(),
            category: NodeCategory::Processing,
            label: "KV Target".to_string(),
            description: "Consumes a KV-cache artifact".to_string(),
            inputs: vec![PortMetadata::required(
                "kv_cache_in",
                "KV Cache In",
                PortDataType::KvCache,
            )],
            outputs: vec![],
            execution_mode: ExecutionMode::Reactive,
        });
        registry.register_metadata(TaskMetadata {
            node_type: "json-target".to_string(),
            category: NodeCategory::Processing,
            label: "JSON Target".to_string(),
            description: "Consumes JSON".to_string(),
            inputs: vec![PortMetadata::required("json", "Json", PortDataType::Json)],
            outputs: vec![],
            execution_mode: ExecutionMode::Reactive,
        });
        registry
    }

    #[test]
    fn test_valid_graph() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("a", "text-input", (0.0, 0.0))
            .add_node("b", "text-output", (100.0, 0.0))
            .add_edge("a", "text", "b", "text")
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_kv_cache_ports_accept_matching_connections() {
        let graph = WorkflowBuilder::new("wf", "KV Cache")
            .add_node("a", "kv-source", (0.0, 0.0))
            .add_node("b", "kv-target", (100.0, 0.0))
            .add_edge("a", "kv_cache_out", "b", "kv_cache_in")
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_kv_cache_ports_reject_json_targets() {
        let graph = WorkflowBuilder::new("wf", "KV Cache Mismatch")
            .add_node("a", "kv-source", (0.0, 0.0))
            .add_node("b", "json-target", (100.0, 0.0))
            .add_edge("a", "kv_cache_out", "b", "json")
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        assert!(errors.iter().any(|error| {
            matches!(
                error,
                ValidationError::IncompatiblePortTypes {
                    source_type,
                    target_type,
                    ..
                } if source_type == "KvCache" && target_type == "Json"
            )
        }));
    }

    #[test]
    fn test_detect_cycle() {
        let graph = WorkflowBuilder::new("wf", "Cyclic")
            .add_node("a", "text-input", (0.0, 0.0))
            .add_node("b", "text-input", (100.0, 0.0))
            .add_edge("a", "out", "b", "in")
            .add_edge("b", "out", "a", "in")
            .build();

        let errors = validate_workflow(&graph, None);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::CycleDetected)));
    }

    #[test]
    fn test_no_cycle_linear() {
        let graph = WorkflowBuilder::new("wf", "Linear")
            .add_node("a", "text-input", (0.0, 0.0))
            .add_node("b", "text-input", (100.0, 0.0))
            .add_node("c", "text-input", (200.0, 0.0))
            .add_edge("a", "out", "b", "in")
            .add_edge("b", "out", "c", "in")
            .build();

        let errors = validate_workflow(&graph, None);
        assert!(!errors
            .iter()
            .any(|e| matches!(e, ValidationError::CycleDetected)));
    }

    #[test]
    fn test_unknown_node_type() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("a", "unknown-type", (0.0, 0.0))
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnknownNodeType { .. })));
    }

    #[test]
    fn test_unconnected_required_input() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("b", "text-output", (100.0, 0.0))
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnconnectedRequiredInput { .. })));
    }

    #[test]
    fn test_edge_references_missing_node() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("a", "text-input", (0.0, 0.0))
            .add_edge("a", "out", "missing", "in")
            .build();

        let errors = validate_workflow(&graph, None);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::UnknownNode { .. })));
    }

    #[test]
    fn test_orchestration_missing_start() {
        let graph = OrchestrationBuilder::new("orch", "Test")
            .add_end("end", (0.0, 0.0))
            .build();

        let errors = validate_orchestration(&graph);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingStartNode)));
    }

    #[test]
    fn test_orchestration_missing_end() {
        let graph = OrchestrationBuilder::new("orch", "Test")
            .add_start("start", (0.0, 0.0))
            .build();

        let errors = validate_orchestration(&graph);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MissingEndNode)));
    }

    #[test]
    fn test_orchestration_multiple_starts() {
        let graph = OrchestrationBuilder::new("orch", "Test")
            .add_start("start1", (0.0, 0.0))
            .add_start("start2", (0.0, 50.0))
            .add_end("end", (100.0, 0.0))
            .build();

        let errors = validate_orchestration(&graph);
        assert!(errors
            .iter()
            .any(|e| matches!(e, ValidationError::MultipleStartNodes)));
    }

    #[test]
    fn test_valid_orchestration() {
        let graph = OrchestrationBuilder::new("orch", "Valid")
            .add_start("start", (0.0, 0.0))
            .add_end("end", (100.0, 0.0))
            .connect("start", "next", "end", "input")
            .build();

        let errors = validate_orchestration(&graph);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn test_collects_multiple_errors() {
        let graph = WorkflowBuilder::new("wf", "Test")
            .add_node("a", "unknown-type-1", (0.0, 0.0))
            .add_node("b", "unknown-type-2", (100.0, 0.0))
            .add_edge("a", "out", "b", "in")
            .add_edge("b", "out", "a", "in")
            .build();

        let registry = make_test_registry();
        let errors = validate_workflow(&graph, Some(&registry));
        // Should have both cycle and unknown type errors
        assert!(errors.len() >= 2);
    }
}
