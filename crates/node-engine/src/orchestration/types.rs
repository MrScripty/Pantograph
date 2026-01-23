//! Orchestration graph types for control flow execution.
//!
//! This module defines the data structures for orchestration graphs,
//! which represent high-level control flow between data graphs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an orchestration node.
pub type OrchestrationNodeId = String;

/// Unique identifier for an orchestration edge.
pub type OrchestrationEdgeId = String;

/// Unique identifier for an orchestration graph.
pub type OrchestrationGraphId = String;

/// An orchestration graph containing control flow nodes and edges.
///
/// Orchestration graphs define the high-level execution flow between
/// data graphs. Each node in an orchestration graph can reference
/// a data graph that performs the actual computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationGraph {
    /// Unique identifier for this orchestration graph.
    pub id: OrchestrationGraphId,
    /// Human-readable name for this orchestration.
    pub name: String,
    /// Description of what this orchestration does.
    #[serde(default)]
    pub description: String,
    /// Control flow nodes in this orchestration.
    pub nodes: Vec<OrchestrationNode>,
    /// Edges connecting orchestration nodes.
    pub edges: Vec<OrchestrationEdge>,
    /// Mapping from DataGraph node IDs to their data graph IDs.
    /// This allows looking up which data graph a DataGraph node references.
    #[serde(default)]
    pub data_graphs: HashMap<OrchestrationNodeId, String>,
}

impl OrchestrationGraph {
    /// Create a new empty orchestration graph.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            data_graphs: HashMap::new(),
        }
    }

    /// Find a node by its ID.
    pub fn find_node(&self, node_id: &str) -> Option<&OrchestrationNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// Find the Start node in this graph.
    pub fn find_start_node(&self) -> Option<&OrchestrationNode> {
        self.nodes
            .iter()
            .find(|n| matches!(n.node_type, OrchestrationNodeType::Start))
    }

    /// Find all End nodes in this graph.
    pub fn find_end_nodes(&self) -> Vec<&OrchestrationNode> {
        self.nodes
            .iter()
            .filter(|n| matches!(n.node_type, OrchestrationNodeType::End))
            .collect()
    }

    /// Get all edges leaving a given node.
    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&OrchestrationEdge> {
        self.edges.iter().filter(|e| e.source == node_id).collect()
    }

    /// Get all edges entering a given node.
    pub fn incoming_edges(&self, node_id: &str) -> Vec<&OrchestrationEdge> {
        self.edges.iter().filter(|e| e.target == node_id).collect()
    }

    /// Get the data graph ID associated with a DataGraph node.
    pub fn get_data_graph_id(&self, node_id: &str) -> Option<&String> {
        self.data_graphs.get(node_id)
    }
}

/// A node in an orchestration graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationNode {
    /// Unique identifier for this node.
    pub id: OrchestrationNodeId,
    /// The type of orchestration node.
    pub node_type: OrchestrationNodeType,
    /// Position in the visual editor (x, y).
    pub position: (f64, f64),
    /// Node-specific configuration.
    #[serde(default)]
    pub config: serde_json::Value,
}

impl OrchestrationNode {
    /// Create a new orchestration node.
    pub fn new(
        id: impl Into<String>,
        node_type: OrchestrationNodeType,
        position: (f64, f64),
    ) -> Self {
        Self {
            id: id.into(),
            node_type,
            position,
            config: serde_json::Value::Null,
        }
    }

    /// Create a new orchestration node with configuration.
    pub fn with_config(
        id: impl Into<String>,
        node_type: OrchestrationNodeType,
        position: (f64, f64),
        config: serde_json::Value,
    ) -> Self {
        Self {
            id: id.into(),
            node_type,
            position,
            config,
        }
    }
}

/// The type of an orchestration node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationNodeType {
    /// Entry point of the orchestration. Only one per graph.
    Start,
    /// Exit point of the orchestration. Can have multiple.
    End,
    /// Conditional branching based on a boolean condition.
    Condition,
    /// Loop execution with iteration control.
    Loop,
    /// References and executes a data graph.
    DataGraph,
    /// Merges multiple execution paths into one.
    Merge,
}

impl OrchestrationNodeType {
    /// Get the available output handles for this node type.
    pub fn output_handles(&self) -> Vec<&'static str> {
        match self {
            OrchestrationNodeType::Start => vec!["next"],
            OrchestrationNodeType::End => vec![],
            OrchestrationNodeType::Condition => vec!["true", "false"],
            OrchestrationNodeType::Loop => vec!["iteration", "complete"],
            OrchestrationNodeType::DataGraph => vec!["next", "error"],
            OrchestrationNodeType::Merge => vec!["next"],
        }
    }

    /// Get the available input handles for this node type.
    pub fn input_handles(&self) -> Vec<&'static str> {
        match self {
            OrchestrationNodeType::Start => vec![],
            OrchestrationNodeType::End => vec!["input"],
            OrchestrationNodeType::Condition => vec!["input"],
            OrchestrationNodeType::Loop => vec!["input", "loop_back"],
            OrchestrationNodeType::DataGraph => vec!["input"],
            OrchestrationNodeType::Merge => vec!["a", "b", "c", "d"], // Up to 4 merge inputs
        }
    }

    /// Get a human-readable label for this node type.
    pub fn label(&self) -> &'static str {
        match self {
            OrchestrationNodeType::Start => "Start",
            OrchestrationNodeType::End => "End",
            OrchestrationNodeType::Condition => "Condition",
            OrchestrationNodeType::Loop => "Loop",
            OrchestrationNodeType::DataGraph => "Data Graph",
            OrchestrationNodeType::Merge => "Merge",
        }
    }
}

/// An edge connecting two orchestration nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationEdge {
    /// Unique identifier for this edge.
    pub id: OrchestrationEdgeId,
    /// Source node ID.
    pub source: OrchestrationNodeId,
    /// Source handle name (e.g., "next", "true", "false", "iteration", "complete").
    pub source_handle: String,
    /// Target node ID.
    pub target: OrchestrationNodeId,
    /// Target handle name (e.g., "input", "loop_back").
    pub target_handle: String,
}

impl OrchestrationEdge {
    /// Create a new orchestration edge.
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        source_handle: impl Into<String>,
        target: impl Into<String>,
        target_handle: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            source_handle: source_handle.into(),
            target: target.into(),
            target_handle: target_handle.into(),
        }
    }
}

/// Configuration for a Condition node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConditionConfig {
    /// The key in the execution context to check.
    pub condition_key: String,
    /// Optional: A specific value to compare against.
    pub expected_value: Option<serde_json::Value>,
}

/// Configuration for a Loop node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopConfig {
    /// Maximum number of iterations (0 = unlimited).
    #[serde(default)]
    pub max_iterations: u32,
    /// Key in context to check for loop exit condition.
    pub exit_condition_key: Option<String>,
    /// Key in context to store current iteration number.
    #[serde(default = "default_iteration_key")]
    pub iteration_key: String,
}

fn default_iteration_key() -> String {
    "loop_iteration".to_string()
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            exit_condition_key: None,
            iteration_key: default_iteration_key(),
        }
    }
}

/// Configuration for a DataGraph node.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataGraphConfig {
    /// The ID of the data graph to execute.
    pub data_graph_id: String,
    /// Mapping of orchestration context keys to data graph input ports.
    #[serde(default)]
    pub input_mappings: HashMap<String, String>,
    /// Mapping of data graph output ports to orchestration context keys.
    #[serde(default)]
    pub output_mappings: HashMap<String, String>,
}

/// Result of executing an orchestration graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestrationResult {
    /// Whether execution completed successfully.
    pub success: bool,
    /// Final output data from the orchestration.
    pub outputs: HashMap<String, serde_json::Value>,
    /// Error message if execution failed.
    pub error: Option<String>,
    /// Number of nodes executed.
    pub nodes_executed: u32,
    /// Total execution time in milliseconds.
    pub execution_time_ms: u64,
}

impl OrchestrationResult {
    /// Create a successful result.
    pub fn success(outputs: HashMap<String, serde_json::Value>, nodes_executed: u32, execution_time_ms: u64) -> Self {
        Self {
            success: true,
            outputs,
            error: None,
            nodes_executed,
            execution_time_ms,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>, nodes_executed: u32, execution_time_ms: u64) -> Self {
        Self {
            success: false,
            outputs: HashMap::new(),
            error: Some(error.into()),
            nodes_executed,
            execution_time_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestration_graph_creation() {
        let graph = OrchestrationGraph::new("test-graph", "Test Graph");
        assert_eq!(graph.id, "test-graph");
        assert_eq!(graph.name, "Test Graph");
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_find_start_node() {
        let mut graph = OrchestrationGraph::new("test", "Test");
        graph.nodes.push(OrchestrationNode::new(
            "start",
            OrchestrationNodeType::Start,
            (0.0, 0.0),
        ));
        graph.nodes.push(OrchestrationNode::new(
            "end",
            OrchestrationNodeType::End,
            (100.0, 0.0),
        ));

        let start = graph.find_start_node();
        assert!(start.is_some());
        assert_eq!(start.unwrap().id, "start");
    }

    #[test]
    fn test_node_type_handles() {
        assert_eq!(
            OrchestrationNodeType::Condition.output_handles(),
            vec!["true", "false"]
        );
        assert_eq!(
            OrchestrationNodeType::Loop.output_handles(),
            vec!["iteration", "complete"]
        );
    }

    #[test]
    fn test_orchestration_result() {
        let success = OrchestrationResult::success(HashMap::new(), 5, 1000);
        assert!(success.success);
        assert!(success.error.is_none());

        let failure = OrchestrationResult::failure("Test error", 3, 500);
        assert!(!failure.success);
        assert_eq!(failure.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_serialization() {
        let node = OrchestrationNode::new("test", OrchestrationNodeType::DataGraph, (10.0, 20.0));
        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("\"nodeType\":\"data_graph\""));
        assert!(json.contains("\"position\":[10.0,20.0]"));
    }
}
