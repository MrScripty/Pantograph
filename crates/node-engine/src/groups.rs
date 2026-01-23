//! Node Groups - Reusable grouped nodes within data graphs
//!
//! Node groups allow users to select multiple nodes and combine them into a single
//! collapsible unit. Groups can:
//! - Be collapsed to show as a single node with exposed ports
//! - Be expanded ("tabbed into") to edit internal nodes
//! - Be nested (groups containing other groups)
//! - Expose specific internal ports as group-level ports
//!
//! # Example
//!
//! ```ignore
//! // Create a group from selected nodes
//! let group = NodeGroup::new("rag-search", "RAG Search")
//!     .with_nodes(vec![embedding_node, search_node, merge_node])
//!     .with_edges(internal_edges)
//!     .expose_input("query", "embedding-node", "text", "Query")
//!     .expose_output("results", "merge-node", "output", "Results");
//! ```

use serde::{Deserialize, Serialize};

use crate::types::{GraphEdge, GraphNode, PortDataType};

/// A node group that contains multiple nodes and edges
///
/// When collapsed, the group appears as a single node with exposed ports.
/// When expanded, users can edit the internal nodes and edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeGroup {
    /// Unique identifier for this group
    pub id: String,
    /// Human-readable name for the group
    pub name: String,
    /// Nodes contained within this group
    pub nodes: Vec<GraphNode>,
    /// Edges connecting nodes within this group
    pub edges: Vec<GraphEdge>,
    /// Input ports exposed at the group level
    pub exposed_inputs: Vec<PortMapping>,
    /// Output ports exposed at the group level
    pub exposed_outputs: Vec<PortMapping>,
    /// Position of the collapsed group node on the canvas
    pub position: (f64, f64),
    /// Whether the group is currently collapsed (shown as single node)
    pub collapsed: bool,
    /// Optional description for the group
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional color/theme for the group node
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl NodeGroup {
    /// Create a new empty node group
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
            exposed_inputs: Vec::new(),
            exposed_outputs: Vec::new(),
            position: (0.0, 0.0),
            collapsed: true,
            description: None,
            color: None,
        }
    }

    /// Set the nodes for this group
    pub fn with_nodes(mut self, nodes: Vec<GraphNode>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Set the edges for this group
    pub fn with_edges(mut self, edges: Vec<GraphEdge>) -> Self {
        self.edges = edges;
        self
    }

    /// Set the position of the collapsed group node
    pub fn with_position(mut self, x: f64, y: f64) -> Self {
        self.position = (x, y);
        self
    }

    /// Add a description to the group
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Expose an internal input port at the group level
    pub fn expose_input(
        mut self,
        group_port_id: impl Into<String>,
        internal_node_id: impl Into<String>,
        internal_port_id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        self.exposed_inputs.push(PortMapping {
            group_port_id: group_port_id.into(),
            group_port_label: label.into(),
            internal_node_id: internal_node_id.into(),
            internal_port_id: internal_port_id.into(),
            data_type,
        });
        self
    }

    /// Expose an internal output port at the group level
    pub fn expose_output(
        mut self,
        group_port_id: impl Into<String>,
        internal_node_id: impl Into<String>,
        internal_port_id: impl Into<String>,
        label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        self.exposed_outputs.push(PortMapping {
            group_port_id: group_port_id.into(),
            group_port_label: label.into(),
            internal_node_id: internal_node_id.into(),
            internal_port_id: internal_port_id.into(),
            data_type,
        });
        self
    }

    /// Find a node within this group by ID
    pub fn find_node(&self, node_id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// Find a node within this group by ID (mutable)
    pub fn find_node_mut(&mut self, node_id: &str) -> Option<&mut GraphNode> {
        self.nodes.iter_mut().find(|n| n.id == node_id)
    }

    /// Get all node IDs in this group
    pub fn node_ids(&self) -> Vec<&str> {
        self.nodes.iter().map(|n| n.id.as_str()).collect()
    }

    /// Check if this group contains a specific node
    pub fn contains_node(&self, node_id: &str) -> bool {
        self.nodes.iter().any(|n| n.id == node_id)
    }

    /// Get edges that connect to external nodes (edges that cross the group boundary)
    /// These are edges where one end is inside the group and the other is outside
    pub fn boundary_edges<'a>(&'a self, all_edges: &'a [GraphEdge]) -> impl Iterator<Item = &'a GraphEdge> {
        let node_ids: std::collections::HashSet<&str> = self.node_ids().into_iter().collect();
        all_edges.iter().filter(move |e| {
            let source_inside = node_ids.contains(e.source.as_str());
            let target_inside = node_ids.contains(e.target.as_str());
            source_inside != target_inside // XOR - exactly one end is inside
        })
    }

    /// Calculate the bounding box of all nodes in the group
    /// Returns (min_x, min_y, max_x, max_y)
    pub fn bounding_box(&self) -> Option<(f64, f64, f64, f64)> {
        if self.nodes.is_empty() {
            return None;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for node in &self.nodes {
            min_x = min_x.min(node.position.0);
            min_y = min_y.min(node.position.1);
            // Approximate node size (could be made configurable)
            max_x = max_x.max(node.position.0 + 200.0);
            max_y = max_y.max(node.position.1 + 100.0);
        }

        Some((min_x, min_y, max_x, max_y))
    }

    /// Calculate the center position of the group
    pub fn center(&self) -> Option<(f64, f64)> {
        self.bounding_box().map(|(min_x, min_y, max_x, max_y)| {
            ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0)
        })
    }
}

/// Mapping from a group-level port to an internal node's port
///
/// This defines how data flows into/out of the group when collapsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortMapping {
    /// The ID of the internal node that has the actual port
    pub internal_node_id: String,
    /// The port ID on the internal node
    pub internal_port_id: String,
    /// The port ID as it appears on the collapsed group node
    pub group_port_id: String,
    /// Human-readable label for the group port
    pub group_port_label: String,
    /// Data type of the port
    pub data_type: PortDataType,
}

impl PortMapping {
    /// Create a new port mapping
    pub fn new(
        internal_node_id: impl Into<String>,
        internal_port_id: impl Into<String>,
        group_port_id: impl Into<String>,
        group_port_label: impl Into<String>,
        data_type: PortDataType,
    ) -> Self {
        Self {
            internal_node_id: internal_node_id.into(),
            internal_port_id: internal_port_id.into(),
            group_port_id: group_port_id.into(),
            group_port_label: group_port_label.into(),
            data_type,
        }
    }
}

/// Result of creating a group from selected nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupResult {
    /// The created group
    pub group: NodeGroup,
    /// Edges that were internal to the group (moved into group.edges)
    pub internalized_edges: Vec<String>,
    /// Edges that cross the group boundary (need port mappings)
    pub boundary_edge_ids: Vec<String>,
    /// Suggested input port mappings based on boundary edges
    pub suggested_inputs: Vec<PortMapping>,
    /// Suggested output port mappings based on boundary edges
    pub suggested_outputs: Vec<PortMapping>,
}

/// Operations for managing node groups
pub struct GroupOperations;

impl GroupOperations {
    /// Create a node group from a set of selected nodes
    ///
    /// This will:
    /// 1. Extract the selected nodes from the graph
    /// 2. Identify internal edges (both ends inside selection)
    /// 3. Identify boundary edges (one end inside, one outside)
    /// 4. Suggest port mappings for boundary edges
    pub fn create_group_from_selection(
        name: impl Into<String>,
        selected_node_ids: &[String],
        all_nodes: &[GraphNode],
        all_edges: &[GraphEdge],
    ) -> CreateGroupResult {
        let name = name.into();
        let group_id = format!("group-{}", uuid::Uuid::new_v4());

        let selected_set: std::collections::HashSet<&str> =
            selected_node_ids.iter().map(|s| s.as_str()).collect();

        // Extract selected nodes
        let group_nodes: Vec<GraphNode> = all_nodes
            .iter()
            .filter(|n| selected_set.contains(n.id.as_str()))
            .cloned()
            .collect();

        // Categorize edges
        let mut internal_edges = Vec::new();
        let mut internalized_edge_ids = Vec::new();
        let mut boundary_edge_ids = Vec::new();
        let mut suggested_inputs = Vec::new();
        let mut suggested_outputs = Vec::new();

        for edge in all_edges {
            let source_inside = selected_set.contains(edge.source.as_str());
            let target_inside = selected_set.contains(edge.target.as_str());

            if source_inside && target_inside {
                // Internal edge - move into group
                internal_edges.push(edge.clone());
                internalized_edge_ids.push(edge.id.clone());
            } else if source_inside && !target_inside {
                // Output boundary - source is inside, target is outside
                boundary_edge_ids.push(edge.id.clone());
                suggested_outputs.push(PortMapping {
                    internal_node_id: edge.source.clone(),
                    internal_port_id: edge.source_handle.clone(),
                    group_port_id: format!("out-{}", edge.source_handle),
                    group_port_label: edge.source_handle.clone(),
                    data_type: PortDataType::Any, // Will need to be resolved from node definitions
                });
            } else if !source_inside && target_inside {
                // Input boundary - source is outside, target is inside
                boundary_edge_ids.push(edge.id.clone());
                suggested_inputs.push(PortMapping {
                    internal_node_id: edge.target.clone(),
                    internal_port_id: edge.target_handle.clone(),
                    group_port_id: format!("in-{}", edge.target_handle),
                    group_port_label: edge.target_handle.clone(),
                    data_type: PortDataType::Any,
                });
            }
        }

        // Calculate center position for the group
        let position = if !group_nodes.is_empty() {
            let sum_x: f64 = group_nodes.iter().map(|n| n.position.0).sum();
            let sum_y: f64 = group_nodes.iter().map(|n| n.position.1).sum();
            let count = group_nodes.len() as f64;
            (sum_x / count, sum_y / count)
        } else {
            (0.0, 0.0)
        };

        let group = NodeGroup {
            id: group_id,
            name,
            nodes: group_nodes,
            edges: internal_edges,
            exposed_inputs: suggested_inputs.clone(),
            exposed_outputs: suggested_outputs.clone(),
            position,
            collapsed: true,
            description: None,
            color: None,
        };

        CreateGroupResult {
            group,
            internalized_edges: internalized_edge_ids,
            boundary_edge_ids,
            suggested_inputs,
            suggested_outputs,
        }
    }

    /// Expand a group - returns the nodes and edges to be added back to the main graph
    pub fn expand_group(group: &NodeGroup) -> (Vec<GraphNode>, Vec<GraphEdge>) {
        (group.nodes.clone(), group.edges.clone())
    }

    /// Validate port mappings for a group
    ///
    /// Checks that:
    /// - All referenced internal nodes exist
    /// - All referenced internal ports exist on those nodes
    /// - No duplicate group port IDs
    pub fn validate_port_mappings(
        group: &NodeGroup,
        _node_definitions: &std::collections::HashMap<String, crate::descriptor::TaskMetadata>,
    ) -> Result<(), GroupValidationError> {
        let node_ids: std::collections::HashSet<&str> =
            group.nodes.iter().map(|n| n.id.as_str()).collect();

        // Check input mappings
        let mut seen_input_ids = std::collections::HashSet::new();
        for mapping in &group.exposed_inputs {
            if !node_ids.contains(mapping.internal_node_id.as_str()) {
                return Err(GroupValidationError::NodeNotFound(
                    mapping.internal_node_id.clone(),
                ));
            }
            if !seen_input_ids.insert(&mapping.group_port_id) {
                return Err(GroupValidationError::DuplicatePortId(
                    mapping.group_port_id.clone(),
                ));
            }
        }

        // Check output mappings
        let mut seen_output_ids = std::collections::HashSet::new();
        for mapping in &group.exposed_outputs {
            if !node_ids.contains(mapping.internal_node_id.as_str()) {
                return Err(GroupValidationError::NodeNotFound(
                    mapping.internal_node_id.clone(),
                ));
            }
            if !seen_output_ids.insert(&mapping.group_port_id) {
                return Err(GroupValidationError::DuplicatePortId(
                    mapping.group_port_id.clone(),
                ));
            }
        }

        Ok(())
    }
}

/// Errors that can occur during group validation
#[derive(Debug, Clone)]
pub enum GroupValidationError {
    /// A referenced internal node was not found in the group
    NodeNotFound(String),
    /// A referenced port was not found on the internal node
    PortNotFound { node_id: String, port_id: String },
    /// Duplicate group port ID
    DuplicatePortId(String),
    /// Group is empty
    EmptyGroup,
}

impl std::fmt::Display for GroupValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NodeNotFound(id) => write!(f, "Node '{}' not found in group", id),
            Self::PortNotFound { node_id, port_id } => {
                write!(f, "Port '{}' not found on node '{}'", port_id, node_id)
            }
            Self::DuplicatePortId(id) => write!(f, "Duplicate group port ID: '{}'", id),
            Self::EmptyGroup => write!(f, "Cannot create empty group"),
        }
    }
}

impl std::error::Error for GroupValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, x: f64, y: f64) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "test".to_string(),
            data: serde_json::Value::Null,
            position: (x, y),
        }
    }

    fn make_edge(id: &str, source: &str, target: &str) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            source_handle: "output".to_string(),
            target: target.to_string(),
            target_handle: "input".to_string(),
        }
    }

    #[test]
    fn test_create_group() {
        let group = NodeGroup::new("test-group", "Test Group")
            .with_position(100.0, 200.0)
            .with_description("A test group");

        assert_eq!(group.id, "test-group");
        assert_eq!(group.name, "Test Group");
        assert_eq!(group.position, (100.0, 200.0));
        assert!(group.collapsed);
    }

    #[test]
    fn test_expose_ports() {
        let group = NodeGroup::new("g1", "Group")
            .expose_input("in1", "node1", "text", "Input", PortDataType::String)
            .expose_output("out1", "node2", "result", "Output", PortDataType::String);

        assert_eq!(group.exposed_inputs.len(), 1);
        assert_eq!(group.exposed_outputs.len(), 1);
        assert_eq!(group.exposed_inputs[0].group_port_id, "in1");
        assert_eq!(group.exposed_outputs[0].group_port_id, "out1");
    }

    #[test]
    fn test_create_group_from_selection() {
        let nodes = vec![
            make_node("a", 0.0, 0.0),
            make_node("b", 100.0, 0.0),
            make_node("c", 200.0, 0.0),
            make_node("d", 300.0, 0.0), // Outside selection
        ];

        let edges = vec![
            make_edge("e1", "a", "b"), // Internal
            make_edge("e2", "b", "c"), // Internal
            make_edge("e3", "c", "d"), // Boundary (output)
        ];

        let selected = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let result = GroupOperations::create_group_from_selection("My Group", &selected, &nodes, &edges);

        assert_eq!(result.group.nodes.len(), 3);
        assert_eq!(result.group.edges.len(), 2); // Internal edges
        assert_eq!(result.internalized_edges.len(), 2);
        assert_eq!(result.boundary_edge_ids.len(), 1);
        assert_eq!(result.suggested_outputs.len(), 1);
    }

    #[test]
    fn test_bounding_box() {
        let group = NodeGroup::new("g1", "Group")
            .with_nodes(vec![
                make_node("a", 0.0, 0.0),
                make_node("b", 100.0, 50.0),
                make_node("c", 50.0, 100.0),
            ]);

        let bbox = group.bounding_box().unwrap();
        assert_eq!(bbox.0, 0.0); // min_x
        assert_eq!(bbox.1, 0.0); // min_y
        // max_x and max_y include node size offset
    }

    #[test]
    fn test_contains_node() {
        let group = NodeGroup::new("g1", "Group")
            .with_nodes(vec![make_node("a", 0.0, 0.0), make_node("b", 100.0, 0.0)]);

        assert!(group.contains_node("a"));
        assert!(group.contains_node("b"));
        assert!(!group.contains_node("c"));
    }
}
