//! Core types for the workflow system
//!
//! Defines port data types, node definitions, and graph structures.

use serde::{Deserialize, Serialize};

/// Data types that can flow through ports
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortDataType {
    /// Accepts any type
    Any,
    /// Plain text string
    String,
    /// Base64-encoded image data
    Image,
    /// Svelte component path or content
    Component,
    /// Streaming data (emits chunks over time)
    Stream,
    /// LLM prompt (compatible with String)
    Prompt,
    /// Collection of tool definitions
    Tools,
    /// Vector embedding
    Embedding,
    /// Document chunk from RAG
    Document,
    /// JSON object
    Json,
    /// Boolean value
    Boolean,
    /// Numeric value
    Number,
}

impl PortDataType {
    /// Check if this type can connect to a target type
    ///
    /// Rules:
    /// - Any accepts/provides everything
    /// - String is compatible with Prompt
    /// - Json is compatible with String
    /// - Number is compatible with String
    /// - Same types are always compatible
    pub fn is_compatible_with(&self, target: &PortDataType) -> bool {
        // Any is a wildcard - accepts everything
        if *self == PortDataType::Any || *target == PortDataType::Any {
            return true;
        }

        // Same types are always compatible
        if self == target {
            return true;
        }

        // String/Prompt are interchangeable
        if matches!(
            (self, target),
            (PortDataType::String, PortDataType::Prompt) |
            (PortDataType::Prompt, PortDataType::String)
        ) {
            return true;
        }

        // Types coercible to String
        if *target == PortDataType::String {
            return matches!(
                self,
                PortDataType::Json | PortDataType::Number | PortDataType::Boolean
            );
        }

        false
    }
}

/// Definition of a single port (input or output)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortDefinition {
    /// Unique identifier within the node
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Data type this port accepts/produces
    pub data_type: PortDataType,
    /// Whether this input is required for execution
    #[serde(default)]
    pub required: bool,
    /// Whether this port can accept multiple connections
    #[serde(default)]
    pub multiple: bool,
}

impl PortDefinition {
    /// Create a new required port
    pub fn required(id: impl Into<String>, label: impl Into<String>, data_type: PortDataType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required: true,
            multiple: false,
        }
    }

    /// Create a new optional port
    pub fn optional(id: impl Into<String>, label: impl Into<String>, data_type: PortDataType) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required: false,
            multiple: false,
        }
    }

    /// Mark this port as accepting multiple connections
    pub fn multiple(mut self) -> Self {
        self.multiple = true;
        self
    }
}

/// Category for organizing nodes in the palette
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    /// Input nodes (user input, file input, etc.)
    Input,
    /// Processing nodes (LLM, vision, RAG, etc.)
    Processing,
    /// Tool nodes (file operations, validation, etc.)
    Tool,
    /// Output nodes (display, preview, etc.)
    Output,
    /// Control flow nodes (loops, conditionals, etc.)
    Control,
}

/// How a node executes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    /// Auto-execute when all inputs are ready
    Reactive,
    /// Requires explicit trigger to execute
    Manual,
    /// Emits values over time (streaming)
    Stream,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Reactive
    }
}

/// Complete definition of a node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    /// Unique type identifier (e.g., "llm-inference")
    pub node_type: String,
    /// Category for palette organization
    pub category: NodeCategory,
    /// Human-readable name
    pub label: String,
    /// Description for tooltips
    pub description: String,
    /// Input port definitions
    pub inputs: Vec<PortDefinition>,
    /// Output port definitions
    pub outputs: Vec<PortDefinition>,
    /// How this node executes
    #[serde(default)]
    pub execution_mode: ExecutionMode,
}

/// A node instance in a workflow graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    /// Unique instance ID
    pub id: String,
    /// Node type (references NodeDefinition.node_type)
    pub node_type: String,
    /// Position on canvas
    pub position: Position,
    /// Node-specific configuration data
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Position on the canvas
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// An edge connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Unique edge ID
    pub id: String,
    /// Source node ID
    pub source: String,
    /// Source port ID (output)
    pub source_handle: String,
    /// Target node ID
    pub target: String,
    /// Target port ID (input)
    pub target_handle: String,
}

/// Complete workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowGraph {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// All edges connecting nodes
    pub edges: Vec<GraphEdge>,
}

impl WorkflowGraph {
    /// Create an empty graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Find a node by ID
    pub fn find_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Check if there's an edge connecting to a specific input port
    pub fn has_edge_to(&self, node_id: &str, port_id: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e.target == node_id && e.target_handle == port_id)
    }

    /// Get all edges that feed into a specific node
    pub fn incoming_edges<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.target == node_id)
    }

    /// Get all edges that come out of a specific node
    pub fn outgoing_edges<'a>(&'a self, node_id: &'a str) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.source == node_id)
    }
}

/// Viewport state for workflow editor
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

/// Metadata for a saved workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// Display name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// ISO 8601 timestamp of creation
    pub created: String,
    /// ISO 8601 timestamp of last modification
    pub modified: String,
}

/// Complete workflow file structure for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowFile {
    /// File format version for forward compatibility
    pub version: String,
    /// Workflow metadata
    pub metadata: WorkflowMetadata,
    /// The workflow graph
    pub graph: WorkflowGraph,
    /// Optional viewport state for restoring editor position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
}

impl WorkflowFile {
    /// Current file format version
    pub const CURRENT_VERSION: &'static str = "1.0";

    /// Create a new workflow file
    pub fn new(name: impl Into<String>, graph: WorkflowGraph) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            version: Self::CURRENT_VERSION.to_string(),
            metadata: WorkflowMetadata {
                name: name.into(),
                description: None,
                created: now.clone(),
                modified: now,
            },
            graph,
            viewport: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatibility_same_types() {
        assert!(PortDataType::String.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::Image.is_compatible_with(&PortDataType::Image));
        assert!(PortDataType::Json.is_compatible_with(&PortDataType::Json));
    }

    #[test]
    fn test_type_compatibility_any() {
        assert!(PortDataType::Any.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::String.is_compatible_with(&PortDataType::Any));
        assert!(PortDataType::Image.is_compatible_with(&PortDataType::Any));
    }

    #[test]
    fn test_type_compatibility_string_prompt() {
        assert!(PortDataType::String.is_compatible_with(&PortDataType::Prompt));
        assert!(PortDataType::Prompt.is_compatible_with(&PortDataType::String));
    }

    #[test]
    fn test_type_compatibility_coercion_to_string() {
        assert!(PortDataType::Json.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::Number.is_compatible_with(&PortDataType::String));
        assert!(PortDataType::Boolean.is_compatible_with(&PortDataType::String));
    }

    #[test]
    fn test_type_incompatibility() {
        assert!(!PortDataType::Image.is_compatible_with(&PortDataType::String));
        assert!(!PortDataType::String.is_compatible_with(&PortDataType::Image));
        assert!(!PortDataType::Number.is_compatible_with(&PortDataType::Boolean));
    }

    #[test]
    fn test_graph_find_node() {
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "node1".into(),
                    node_type: "test".into(),
                    position: Position::default(),
                    data: serde_json::Value::Null,
                },
            ],
            edges: vec![],
        };

        assert!(graph.find_node("node1").is_some());
        assert!(graph.find_node("nonexistent").is_none());
    }

    #[test]
    fn test_graph_has_edge_to() {
        let graph = WorkflowGraph {
            nodes: vec![],
            edges: vec![
                GraphEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    source_handle: "out".into(),
                    target: "b".into(),
                    target_handle: "in".into(),
                },
            ],
        };

        assert!(graph.has_edge_to("b", "in"));
        assert!(!graph.has_edge_to("b", "other"));
        assert!(!graph.has_edge_to("a", "in"));
    }
}
