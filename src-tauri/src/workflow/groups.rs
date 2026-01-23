//! Tauri commands for node group operations
//!
//! These commands provide CRUD operations for node groups:
//! - Create a group from selected nodes
//! - Expand/collapse groups
//! - Update group port mappings
//! - Delete groups

use serde::{Deserialize, Serialize};
use tauri::command;

use super::types::{GraphEdge, GraphNode, PortDataType, WorkflowGraph};

/// Port mapping for group boundaries
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

/// A node group containing multiple nodes
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
    pub position: Position,
    /// Whether the group is currently collapsed
    pub collapsed: bool,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional color/theme
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Position on the canvas
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Result of creating a group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupResult {
    /// The created group
    pub group: NodeGroup,
    /// IDs of edges that were internalized (moved into the group)
    pub internalized_edge_ids: Vec<String>,
    /// IDs of edges that cross the group boundary
    pub boundary_edge_ids: Vec<String>,
    /// Suggested input port mappings
    pub suggested_inputs: Vec<PortMapping>,
    /// Suggested output port mappings
    pub suggested_outputs: Vec<PortMapping>,
}

/// Result of expanding a group
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandGroupResult {
    /// Nodes that were inside the group
    pub nodes: Vec<GraphNode>,
    /// Edges that were inside the group
    pub edges: Vec<GraphEdge>,
    /// The group ID that was expanded
    pub group_id: String,
}

/// Create a node group from selected nodes
///
/// This command:
/// 1. Extracts selected nodes from the graph
/// 2. Identifies internal edges (both ends in selection)
/// 3. Identifies boundary edges (one end in, one end out)
/// 4. Creates port mappings for boundary edges
/// 5. Returns the group with suggested port mappings
#[command]
pub fn create_node_group(
    name: String,
    selected_node_ids: Vec<String>,
    graph: WorkflowGraph,
) -> Result<CreateGroupResult, String> {
    if selected_node_ids.is_empty() {
        return Err("Cannot create empty group".to_string());
    }

    if selected_node_ids.len() < 2 {
        return Err("Group must contain at least 2 nodes".to_string());
    }

    let group_id = format!("group-{}", uuid::Uuid::new_v4());
    let selected_set: std::collections::HashSet<&str> =
        selected_node_ids.iter().map(|s| s.as_str()).collect();

    // Extract selected nodes
    let group_nodes: Vec<GraphNode> = graph
        .nodes
        .iter()
        .filter(|n| selected_set.contains(n.id.as_str()))
        .cloned()
        .collect();

    if group_nodes.len() != selected_node_ids.len() {
        return Err("Some selected nodes were not found in the graph".to_string());
    }

    // Categorize edges
    let mut internal_edges = Vec::new();
    let mut internalized_edge_ids = Vec::new();
    let mut boundary_edge_ids = Vec::new();
    let mut suggested_inputs = Vec::new();
    let mut suggested_outputs = Vec::new();

    for edge in &graph.edges {
        let source_inside = selected_set.contains(edge.source.as_str());
        let target_inside = selected_set.contains(edge.target.as_str());

        if source_inside && target_inside {
            // Internal edge - move into group
            internal_edges.push(edge.clone());
            internalized_edge_ids.push(edge.id.clone());
        } else if source_inside && !target_inside {
            // Output boundary - source is inside, target is outside
            boundary_edge_ids.push(edge.id.clone());

            // Get the source node to determine data type
            let source_node = group_nodes.iter().find(|n| n.id == edge.source);
            let data_type = if let Some(_node) = source_node {
                // Default to Any - frontend will resolve actual type from definitions
                PortDataType::Any
            } else {
                PortDataType::Any
            };

            suggested_outputs.push(PortMapping {
                internal_node_id: edge.source.clone(),
                internal_port_id: edge.source_handle.clone(),
                group_port_id: format!("out-{}-{}", edge.source, edge.source_handle),
                group_port_label: edge.source_handle.clone(),
                data_type,
            });
        } else if !source_inside && target_inside {
            // Input boundary - source is outside, target is inside
            boundary_edge_ids.push(edge.id.clone());

            suggested_inputs.push(PortMapping {
                internal_node_id: edge.target.clone(),
                internal_port_id: edge.target_handle.clone(),
                group_port_id: format!("in-{}-{}", edge.target, edge.target_handle),
                group_port_label: edge.target_handle.clone(),
                data_type: PortDataType::Any,
            });
        }
    }

    // Calculate center position for the group
    let position = if !group_nodes.is_empty() {
        let sum_x: f64 = group_nodes.iter().map(|n| n.position.x).sum();
        let sum_y: f64 = group_nodes.iter().map(|n| n.position.y).sum();
        let count = group_nodes.len() as f64;
        Position {
            x: sum_x / count,
            y: sum_y / count,
        }
    } else {
        Position::default()
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

    Ok(CreateGroupResult {
        group,
        internalized_edge_ids,
        boundary_edge_ids,
        suggested_inputs,
        suggested_outputs,
    })
}

/// Expand a node group to access its internal graph
///
/// This returns the internal nodes and edges for editing.
/// The group remains in the graph but is marked as expanded.
#[command]
pub fn expand_node_group(group: NodeGroup) -> Result<ExpandGroupResult, String> {
    Ok(ExpandGroupResult {
        nodes: group.nodes.clone(),
        edges: group.edges.clone(),
        group_id: group.id,
    })
}

/// Collapse a node group after editing
///
/// Updates the group with new nodes/edges from the editing session.
#[command]
pub fn collapse_node_group(
    group_id: String,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    exposed_inputs: Vec<PortMapping>,
    exposed_outputs: Vec<PortMapping>,
) -> Result<NodeGroup, String> {
    // Calculate new center position
    let position = if !nodes.is_empty() {
        let sum_x: f64 = nodes.iter().map(|n| n.position.x).sum();
        let sum_y: f64 = nodes.iter().map(|n| n.position.y).sum();
        let count = nodes.len() as f64;
        Position {
            x: sum_x / count,
            y: sum_y / count,
        }
    } else {
        Position::default()
    };

    // Preserve the group name from ID (or allow it to be passed)
    let name = group_id.replace("group-", "Group ");

    Ok(NodeGroup {
        id: group_id,
        name,
        nodes,
        edges,
        exposed_inputs,
        exposed_outputs,
        position,
        collapsed: true,
        description: None,
        color: None,
    })
}

/// Update the port mappings for a group
///
/// Allows users to customize which internal ports are exposed at the group level.
#[command]
pub fn update_group_ports(
    mut group: NodeGroup,
    exposed_inputs: Vec<PortMapping>,
    exposed_outputs: Vec<PortMapping>,
) -> Result<NodeGroup, String> {
    // Validate that all referenced nodes exist in the group
    let node_ids: std::collections::HashSet<&str> =
        group.nodes.iter().map(|n| n.id.as_str()).collect();

    for mapping in &exposed_inputs {
        if !node_ids.contains(mapping.internal_node_id.as_str()) {
            return Err(format!(
                "Internal node '{}' not found in group",
                mapping.internal_node_id
            ));
        }
    }

    for mapping in &exposed_outputs {
        if !node_ids.contains(mapping.internal_node_id.as_str()) {
            return Err(format!(
                "Internal node '{}' not found in group",
                mapping.internal_node_id
            ));
        }
    }

    group.exposed_inputs = exposed_inputs;
    group.exposed_outputs = exposed_outputs;

    Ok(group)
}

/// Rename a node group
#[command]
pub fn rename_node_group(mut group: NodeGroup, new_name: String) -> Result<NodeGroup, String> {
    if new_name.trim().is_empty() {
        return Err("Group name cannot be empty".to_string());
    }
    group.name = new_name;
    Ok(group)
}

/// Delete a node group and restore its nodes to the main graph
///
/// Returns the nodes and edges that should be added back to the main graph.
#[command]
pub fn ungroup_nodes(group: NodeGroup) -> Result<ExpandGroupResult, String> {
    Ok(ExpandGroupResult {
        nodes: group.nodes,
        edges: group.edges,
        group_id: group.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, x: f64, y: f64) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "test".to_string(),
            position: super::super::types::Position { x, y },
            data: serde_json::Value::Null,
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
        let graph = WorkflowGraph {
            nodes: vec![
                make_node("a", 0.0, 0.0),
                make_node("b", 100.0, 0.0),
                make_node("c", 200.0, 0.0),
            ],
            edges: vec![
                make_edge("e1", "a", "b"),
                make_edge("e2", "b", "c"),
            ],
        };

        let result = create_node_group(
            "Test Group".to_string(),
            vec!["a".to_string(), "b".to_string()],
            graph,
        )
        .unwrap();

        assert_eq!(result.group.nodes.len(), 2);
        assert_eq!(result.group.edges.len(), 1); // e1 is internal
        assert_eq!(result.internalized_edge_ids, vec!["e1"]);
        assert_eq!(result.boundary_edge_ids, vec!["e2"]); // e2 crosses boundary
    }

    #[test]
    fn test_create_group_empty() {
        let graph = WorkflowGraph {
            nodes: vec![],
            edges: vec![],
        };

        let result = create_node_group("Empty".to_string(), vec![], graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_group_single_node() {
        let graph = WorkflowGraph {
            nodes: vec![make_node("a", 0.0, 0.0)],
            edges: vec![],
        };

        let result = create_node_group("Single".to_string(), vec!["a".to_string()], graph);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_group_ports_invalid_node() {
        let group = NodeGroup {
            id: "g1".to_string(),
            name: "Test".to_string(),
            nodes: vec![make_node("a", 0.0, 0.0)],
            edges: vec![],
            exposed_inputs: vec![],
            exposed_outputs: vec![],
            position: Position::default(),
            collapsed: true,
            description: None,
            color: None,
        };

        let result = update_group_ports(
            group,
            vec![PortMapping {
                internal_node_id: "nonexistent".to_string(),
                internal_port_id: "input".to_string(),
                group_port_id: "in".to_string(),
                group_port_label: "Input".to_string(),
                data_type: PortDataType::String,
            }],
            vec![],
        );

        assert!(result.is_err());
    }
}
