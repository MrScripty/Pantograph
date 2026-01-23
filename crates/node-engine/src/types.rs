//! Core types for workflow graphs
//!
//! These types define the structure of workflow graphs, including
//! nodes, edges, ports, and their metadata.

use serde::{Deserialize, Serialize};

use crate::groups::NodeGroup;

/// Unique identifier for a node
pub type NodeId = String;

/// Unique identifier for an edge
pub type EdgeId = String;

/// Unique identifier for a port
pub type PortId = String;

/// The data type of a port
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDataType {
    /// Accepts any type
    Any,
    /// Text string
    String,
    /// Image data (base64 encoded)
    Image,
    /// Audio data
    Audio,
    /// UI component reference
    Component,
    /// Streaming data
    Stream,
    /// Prompt text (special string for LLM input)
    Prompt,
    /// Tool definitions
    Tools,
    /// Embedding vector
    Embedding,
    /// Document/text chunk
    Document,
    /// JSON object
    Json,
    /// Boolean value
    Boolean,
    /// Numeric value
    Number,
    /// Vector database reference
    VectorDb,
    /// Reference to a loaded model
    ModelHandle,
    /// Reference to an embedding model
    EmbeddingHandle,
    /// Reference to a database connection
    DatabaseHandle,
    /// Raw embedding vector (Vec<f32>)
    Vector,
    /// Generic tensor
    Tensor,
    /// Raw audio samples
    AudioSamples,
}

impl PortDataType {
    /// Check if this type can connect to another type
    pub fn is_compatible_with(&self, other: &PortDataType) -> bool {
        // Any type is compatible with everything
        if matches!(self, PortDataType::Any) || matches!(other, PortDataType::Any) {
            return true;
        }

        // Prompt is compatible with String
        if matches!(self, PortDataType::Prompt) && matches!(other, PortDataType::String) {
            return true;
        }
        if matches!(self, PortDataType::String) && matches!(other, PortDataType::Prompt) {
            return true;
        }

        // Exact type match
        self == other
    }
}

/// Definition of a port (input or output)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortDefinition {
    /// Unique identifier for this port
    pub id: PortId,
    /// Human-readable label
    pub label: String,
    /// Data type of the port
    pub data_type: PortDataType,
    /// Whether this port is required (for inputs)
    pub required: bool,
    /// Whether this port accepts multiple connections
    pub multiple: bool,
    /// Default value (for optional inputs)
    pub default_value: Option<serde_json::Value>,
}

impl PortDefinition {
    /// Create a required port
    pub fn required(id: impl Into<String>, label: impl Into<String>, data_type: PortDataType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required: true,
            multiple: false,
            default_value: None,
        }
    }

    /// Create an optional port
    pub fn optional(id: impl Into<String>, label: impl Into<String>, data_type: PortDataType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required: false,
            multiple: false,
            default_value: None,
        }
    }

    /// Set this port to accept multiple connections
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }

    /// Set a default value for this port
    pub fn with_default(mut self, value: serde_json::Value) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// Category of a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    /// Input nodes (user input, file input, etc.)
    Input,
    /// Output nodes (display, export, etc.)
    Output,
    /// Processing nodes (LLM, vision, RAG, etc.)
    Processing,
    /// Control flow nodes (conditionals, loops, etc.)
    Control,
    /// Tool nodes (function calls, integrations)
    Tool,
}

/// Execution mode for a node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Execute once when inputs are available
    Batch,
    /// Execute with streaming output
    Stream,
    /// Execute reactively when inputs change
    Reactive,
    /// Requires explicit trigger to execute
    Manual,
}

/// Definition of a node type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDefinition {
    /// Unique type identifier (e.g., "llm-inference")
    pub node_type: String,
    /// Category for grouping in UI
    pub category: NodeCategory,
    /// Human-readable label
    pub label: String,
    /// Description of what the node does
    pub description: String,
    /// Input port definitions
    pub inputs: Vec<PortDefinition>,
    /// Output port definitions
    pub outputs: Vec<PortDefinition>,
    /// Execution mode
    pub execution_mode: ExecutionMode,
}

/// An edge connecting two ports
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// Unique identifier for this edge
    pub id: EdgeId,
    /// Source node ID
    pub source: NodeId,
    /// Source port ID
    pub source_handle: PortId,
    /// Target node ID
    pub target: NodeId,
    /// Target port ID
    pub target_handle: PortId,
}

/// A node instance in a graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Unique identifier for this node instance
    pub id: NodeId,
    /// Node type (references a NodeDefinition)
    pub node_type: String,
    /// Custom data/configuration for this instance
    pub data: serde_json::Value,
    /// Position in the UI (x, y)
    pub position: (f64, f64),
}

/// A complete workflow graph
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowGraph {
    /// Unique identifier for this graph
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// Edges connecting nodes
    pub edges: Vec<GraphEdge>,
    /// Node groups (collapsed node collections)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<NodeGroup>,
}

impl WorkflowGraph {
    /// Create a new empty graph
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Find a node by ID
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Find a node by ID (mutable)
    pub fn find_node_mut(&mut self, id: &str) -> Option<&mut GraphNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    /// Get edges coming into a node
    pub fn incoming_edges<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.target == node_id)
    }

    /// Get edges going out of a node
    pub fn outgoing_edges<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.source == node_id)
    }

    /// Get the IDs of nodes that this node depends on (upstream nodes)
    pub fn get_dependencies(&self, node_id: &str) -> Vec<NodeId> {
        self.incoming_edges(node_id)
            .map(|e| e.source.clone())
            .collect()
    }

    /// Get the IDs of nodes that depend on this node (downstream nodes)
    pub fn get_dependents(&self, node_id: &str) -> Vec<NodeId> {
        self.outgoing_edges(node_id)
            .map(|e| e.target.clone())
            .collect()
    }

    /// Find a group by ID
    pub fn find_group(&self, group_id: &str) -> Option<&NodeGroup> {
        self.groups.iter().find(|g| g.id == group_id)
    }

    /// Find a group by ID (mutable)
    pub fn find_group_mut(&mut self, group_id: &str) -> Option<&mut NodeGroup> {
        self.groups.iter_mut().find(|g| g.id == group_id)
    }

    /// Add a group to the graph
    pub fn add_group(&mut self, group: NodeGroup) {
        self.groups.push(group);
    }

    /// Remove a group by ID
    pub fn remove_group(&mut self, group_id: &str) -> Option<NodeGroup> {
        if let Some(pos) = self.groups.iter().position(|g| g.id == group_id) {
            Some(self.groups.remove(pos))
        } else {
            None
        }
    }

    /// Check if a node is inside any group
    pub fn node_in_group(&self, node_id: &str) -> Option<&NodeGroup> {
        self.groups.iter().find(|g| g.contains_node(node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_data_type_compatibility() {
        assert!(PortDataType::Any.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::String.is_compatible_with(&PortDataType::Any));
        assert!(PortDataType::Prompt.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::String.is_compatible_with(&PortDataType::Prompt));
        assert!(!PortDataType::Number.is_compatible_with(&PortDataType::String));
    }

    #[test]
    fn test_graph_edges() {
        let mut graph = WorkflowGraph::new("test", "Test Graph");
        graph.nodes.push(GraphNode {
            id: "node1".to_string(),
            node_type: "input".to_string(),
            data: serde_json::Value::Null,
            position: (0.0, 0.0),
        });
        graph.nodes.push(GraphNode {
            id: "node2".to_string(),
            node_type: "output".to_string(),
            data: serde_json::Value::Null,
            position: (100.0, 0.0),
        });
        graph.edges.push(GraphEdge {
            id: "edge1".to_string(),
            source: "node1".to_string(),
            source_handle: "output".to_string(),
            target: "node2".to_string(),
            target_handle: "input".to_string(),
        });

        let deps = graph.get_dependencies("node2");
        assert_eq!(deps, vec!["node1"]);

        let dependents = graph.get_dependents("node1");
        assert_eq!(dependents, vec!["node2"]);
    }
}
