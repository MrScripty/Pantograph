//! Core types for the workflow system
//!
//! Defines port data types, node definitions, and graph structures.

use std::collections::HashMap;

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
    /// Audio data for audio models
    Audio,
    /// Streaming audio chunk data
    AudioStream,
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
    /// Vector database reference
    VectorDb,
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
            (PortDataType::String, PortDataType::Prompt)
                | (PortDataType::Prompt, PortDataType::String)
        ) {
            return true;
        }
        // AudioStream remains compatible with legacy Stream ports
        if matches!(
            (self, target),
            (PortDataType::AudioStream, PortDataType::Stream)
                | (PortDataType::Stream, PortDataType::AudioStream)
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
    pub fn required(
        id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            data_type,
            required: true,
            multiple: false,
        }
    }

    /// Create a new optional port
    pub fn optional(
        id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
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

/// Declares whether an input/output node is externally bindable by clients/sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IoBindingOrigin {
    /// Bindable through headless/client session APIs.
    ClientSession,
    /// Provided/consumed internally by integrated systems.
    Integrated,
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
    /// Whether this node is bindable by client/session APIs or integrated only.
    pub io_binding_origin: IoBindingOrigin,
    /// Input port definitions
    pub inputs: Vec<PortDefinition>,
    /// Output port definitions
    pub outputs: Vec<PortDefinition>,
    /// How this node executes
    #[serde(default)]
    pub execution_mode: ExecutionMode,
}

/// A node instance in a workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// An edge connecting two nodes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// A specific node port anchor used during interactive connection flows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionAnchor {
    /// Node instance that owns the anchor.
    pub node_id: String,
    /// Port/handle identifier on the node.
    pub port_id: String,
}

/// A compatible input anchor on an existing node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTargetAnchorCandidate {
    /// Input port identifier.
    pub port_id: String,
    /// Human-readable label for the input port.
    pub port_label: String,
    /// Data type accepted by the input port.
    pub data_type: PortDataType,
    /// Whether the input accepts multiple incoming edges.
    pub multiple: bool,
}

/// An existing node that can accept a connection from the active source anchor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTargetNodeCandidate {
    /// Node instance identifier.
    pub node_id: String,
    /// Registered node type.
    pub node_type: String,
    /// Display label for the node.
    pub node_label: String,
    /// Canvas position of the node.
    pub position: Position,
    /// Compatible input anchors on this node.
    pub anchors: Vec<ConnectionTargetAnchorCandidate>,
}

/// A node type that could be inserted to continue the current connection intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InsertableNodeTypeCandidate {
    /// Registered node type identifier.
    pub node_type: String,
    /// Palette category for client grouping.
    pub category: NodeCategory,
    /// Human-readable label.
    pub label: String,
    /// Description presented in search/insert UI.
    pub description: String,
    /// Compatible input port ids on the node type.
    pub matching_input_port_ids: Vec<String>,
}

/// Candidate discovery response for an interactive connection intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCandidatesResponse {
    /// Structural revision for the graph used to compute these candidates.
    pub graph_revision: String,
    /// Whether the caller-provided revision matched the current graph.
    pub revision_matches: bool,
    /// Echo of the active source anchor.
    pub source_anchor: ConnectionAnchor,
    /// Existing nodes in the graph that can accept this source anchor.
    pub compatible_nodes: Vec<ConnectionTargetNodeCandidate>,
    /// Node types that expose at least one compatible input.
    pub insertable_node_types: Vec<InsertableNodeTypeCandidate>,
}

/// Canonical structured rejection reasons for connection commits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionRejectionReason {
    /// The client's graph revision does not match the current session graph.
    StaleRevision,
    /// The source anchor does not exist or is not an output.
    UnknownSourceAnchor,
    /// The target anchor does not exist or is not an input.
    UnknownTargetAnchor,
    /// The same edge already exists.
    DuplicateConnection,
    /// The target input is already occupied and does not accept multiple edges.
    TargetCapacityReached,
    /// The connection would create a self-loop.
    SelfConnection,
    /// The connection would introduce a cycle.
    CycleDetected,
    /// Source and target port types are not compatible.
    IncompatibleTypes,
}

/// Structured rejection payload returned when a connection commit is denied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionRejection {
    /// Stable machine-consumable reason code.
    pub reason: ConnectionRejectionReason,
    /// Human-readable explanation for logs or UI.
    pub message: String,
}

/// Result of attempting to commit an interactive connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCommitResponse {
    /// True when the edge was added to the graph.
    pub accepted: bool,
    /// Current graph revision after evaluating the request.
    pub graph_revision: String,
    /// Updated graph when the connection succeeds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<WorkflowGraph>,
    /// Structured rejection when the connection is denied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ConnectionRejection>,
}

/// Derived graph metadata persisted alongside the workflow graph.
///
/// This data is advisory and can be recomputed from the graph. It is trusted
/// only when the fingerprint matches the current graph structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkflowDerivedGraph {
    /// Version for derived graph schema evolution.
    pub schema_version: u32,
    /// Deterministic fingerprint of node/edge structure.
    pub graph_fingerprint: String,
    /// Outgoing consumer counts by "node_id:port_id".
    pub consumer_count_map: HashMap<String, u32>,
}

/// Complete workflow graph
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct WorkflowGraph {
    /// All nodes in the graph
    pub nodes: Vec<GraphNode>,
    /// All edges connecting nodes
    pub edges: Vec<GraphEdge>,
    /// Optional persisted derived metadata for execution hints.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_graph: Option<WorkflowDerivedGraph>,
}

impl WorkflowGraph {
    /// Current version for the derived graph metadata schema.
    pub const DERIVED_GRAPH_SCHEMA_VERSION: u32 = 1;

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
    pub fn incoming_edges<'a>(
        &'a self,
        node_id: &'a str,
    ) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.target == node_id)
    }

    /// Get all edges that come out of a specific node
    pub fn outgoing_edges<'a>(
        &'a self,
        node_id: &'a str,
    ) -> impl Iterator<Item = &'a GraphEdge> + 'a {
        self.edges.iter().filter(move |e| e.source == node_id)
    }

    /// Compute outgoing consumer counts by "node_id:port_id".
    pub fn compute_consumer_count_map(&self) -> HashMap<String, u32> {
        let mut out: HashMap<String, u32> = HashMap::new();
        for edge in &self.edges {
            let key = format!("{}:{}", edge.source, edge.source_handle);
            out.entry(key).and_modify(|count| *count += 1).or_insert(1);
        }
        out
    }

    /// Compute a deterministic graph fingerprint from node/edge structure.
    ///
    /// This intentionally excludes mutable node data and focuses only on
    /// structural shape relevant to port consumer analysis.
    pub fn compute_fingerprint(&self) -> String {
        let mut node_rows = self
            .nodes
            .iter()
            .map(|n| format!("{}|{}", n.id, n.node_type))
            .collect::<Vec<_>>();
        node_rows.sort();

        let mut edge_rows = self
            .edges
            .iter()
            .map(|e| {
                format!(
                    "{}|{}|{}|{}",
                    e.source, e.source_handle, e.target, e.target_handle
                )
            })
            .collect::<Vec<_>>();
        edge_rows.sort();

        let mut digest = FNV64_OFFSET_BASIS;
        digest = fnv1a64_update(digest, b"v1");
        for row in node_rows {
            digest = fnv1a64_update(digest, row.as_bytes());
            digest = fnv1a64_update(digest, b"\n");
        }
        digest = fnv1a64_update(digest, b"--");
        for row in edge_rows {
            digest = fnv1a64_update(digest, row.as_bytes());
            digest = fnv1a64_update(digest, b"\n");
        }

        format!("{:016x}", digest)
    }

    /// Build derived graph metadata from current graph structure.
    pub fn build_derived_graph(&self) -> WorkflowDerivedGraph {
        WorkflowDerivedGraph {
            schema_version: Self::DERIVED_GRAPH_SCHEMA_VERSION,
            graph_fingerprint: self.compute_fingerprint(),
            consumer_count_map: self.compute_consumer_count_map(),
        }
    }

    /// Refresh persisted derived metadata for this graph.
    pub fn refresh_derived_graph(&mut self) {
        self.derived_graph = Some(self.build_derived_graph());
    }

    /// Return true if persisted derived metadata is present and valid.
    pub fn has_valid_derived_graph(&self) -> bool {
        self.derived_graph.as_ref().is_some_and(|derived| {
            derived.schema_version == Self::DERIVED_GRAPH_SCHEMA_VERSION
                && derived.graph_fingerprint == self.compute_fingerprint()
        })
    }

    /// Resolve effective consumer counts, using persisted data when valid.
    pub fn effective_consumer_count_map(&self) -> HashMap<String, u32> {
        if let Some(derived) = &self.derived_graph {
            if derived.schema_version == Self::DERIVED_GRAPH_SCHEMA_VERSION
                && derived.graph_fingerprint == self.compute_fingerprint()
            {
                return derived.consumer_count_map.clone();
            }
        }
        self.compute_consumer_count_map()
    }
}

const FNV64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV64_PRIME: u64 = 0x100000001b3;

fn fnv1a64_update(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV64_PRIME);
    }
    hash
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
#[serde(rename_all = "camelCase")]
pub struct WorkflowMetadata {
    /// Filename stem (e.g., "coding-agent") used for loading
    /// Populated by list_workflows, not stored in the JSON file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Display name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// ISO 8601 timestamp of creation
    pub created: String,
    /// ISO 8601 timestamp of last modification
    pub modified: String,
    /// Optional link to parent orchestration for zoom-out navigation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orchestration_id: Option<String>,
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
                id: None, // Will be populated by list_workflows from filename
                name: name.into(),
                description: None,
                created: now.clone(),
                modified: now,
                orchestration_id: None,
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
    fn test_type_compatibility_audio_stream_legacy_stream() {
        assert!(PortDataType::AudioStream.is_compatible_with(&PortDataType::Stream));
        assert!(PortDataType::Stream.is_compatible_with(&PortDataType::AudioStream));
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
            nodes: vec![GraphNode {
                id: "node1".into(),
                node_type: "test".into(),
                position: Position::default(),
                data: serde_json::Value::Null,
            }],
            edges: vec![],
            derived_graph: None,
        };

        assert!(graph.find_node("node1").is_some());
        assert!(graph.find_node("nonexistent").is_none());
    }

    #[test]
    fn test_graph_has_edge_to() {
        let graph = WorkflowGraph {
            nodes: vec![],
            edges: vec![GraphEdge {
                id: "e1".into(),
                source: "a".into(),
                source_handle: "out".into(),
                target: "b".into(),
                target_handle: "in".into(),
            }],
            derived_graph: None,
        };

        assert!(graph.has_edge_to("b", "in"));
        assert!(!graph.has_edge_to("b", "other"));
        assert!(!graph.has_edge_to("a", "in"));
    }

    #[test]
    fn test_compute_consumer_count_map_counts_by_source_port() {
        let graph = WorkflowGraph {
            nodes: vec![],
            edges: vec![
                GraphEdge {
                    id: "e1".into(),
                    source: "a".into(),
                    source_handle: "metadata".into(),
                    target: "b".into(),
                    target_handle: "x".into(),
                },
                GraphEdge {
                    id: "e2".into(),
                    source: "a".into(),
                    source_handle: "metadata".into(),
                    target: "c".into(),
                    target_handle: "x".into(),
                },
                GraphEdge {
                    id: "e3".into(),
                    source: "a".into(),
                    source_handle: "embedding".into(),
                    target: "d".into(),
                    target_handle: "x".into(),
                },
            ],
            derived_graph: None,
        };

        let counts = graph.compute_consumer_count_map();
        assert_eq!(counts.get("a:metadata"), Some(&2));
        assert_eq!(counts.get("a:embedding"), Some(&1));
    }

    #[test]
    fn test_fingerprint_changes_when_graph_edges_change() {
        let mut graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "n1".into(),
                node_type: "embedding".into(),
                position: Position::default(),
                data: serde_json::Value::Null,
            }],
            edges: vec![],
            derived_graph: None,
        };
        let first = graph.compute_fingerprint();
        graph.edges.push(GraphEdge {
            id: "e1".into(),
            source: "n1".into(),
            source_handle: "embedding".into(),
            target: "n2".into(),
            target_handle: "vector".into(),
        });
        let second = graph.compute_fingerprint();
        assert_ne!(first, second);
    }

    #[test]
    fn test_effective_consumer_count_map_uses_valid_derived_graph() {
        let mut graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "a".into(),
                node_type: "embedding".into(),
                position: Position::default(),
                data: serde_json::Value::Null,
            }],
            edges: vec![GraphEdge {
                id: "e1".into(),
                source: "a".into(),
                source_handle: "metadata".into(),
                target: "b".into(),
                target_handle: "x".into(),
            }],
            derived_graph: None,
        };

        graph.refresh_derived_graph();
        let counts = graph.effective_consumer_count_map();
        assert_eq!(counts.get("a:metadata"), Some(&1));
    }
}
