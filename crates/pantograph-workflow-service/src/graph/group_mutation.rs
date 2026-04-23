use std::collections::HashSet;

use serde_json::json;
use uuid::Uuid;

use crate::workflow::WorkflowServiceError;

use super::types::{GraphEdge, GraphNode, NodeGroup, PortDataType, PortMapping, WorkflowGraph};

const GROUP_NODE_TYPE: &str = "node-group";

pub(crate) fn create_node_group_graph(
    graph: &WorkflowGraph,
    name: String,
    selected_node_ids: &[String],
) -> Result<WorkflowGraph, WorkflowServiceError> {
    if selected_node_ids.is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(
            "Cannot create empty group".to_string(),
        ));
    }
    if selected_node_ids.len() < 2 {
        return Err(WorkflowServiceError::InvalidRequest(
            "Group must contain at least 2 nodes".to_string(),
        ));
    }

    let selected_set = selected_node_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    if selected_set.len() < 2 {
        return Err(WorkflowServiceError::InvalidRequest(
            "Group must contain at least 2 distinct nodes".to_string(),
        ));
    }
    let group_nodes = graph
        .nodes
        .iter()
        .filter(|node| selected_set.contains(node.id.as_str()))
        .cloned()
        .collect::<Vec<_>>();

    if group_nodes.len() != selected_set.len() {
        return Err(WorkflowServiceError::InvalidRequest(
            "Some selected nodes were not found in the graph".to_string(),
        ));
    }

    let mut internal_edges = Vec::new();
    let mut exposed_inputs = Vec::new();
    let mut exposed_outputs = Vec::new();

    for edge in &graph.edges {
        let source_inside = selected_set.contains(edge.source.as_str());
        let target_inside = selected_set.contains(edge.target.as_str());
        if source_inside && target_inside {
            internal_edges.push(edge.clone());
        } else if !source_inside && target_inside {
            exposed_inputs.push(PortMapping {
                internal_node_id: edge.target.clone(),
                internal_port_id: edge.target_handle.clone(),
                group_port_id: format!("in-{}-{}", edge.target, edge.target_handle),
                group_port_label: edge.target_handle.clone(),
                data_type: PortDataType::Any,
            });
        } else if source_inside && !target_inside {
            exposed_outputs.push(PortMapping {
                internal_node_id: edge.source.clone(),
                internal_port_id: edge.source_handle.clone(),
                group_port_id: format!("out-{}-{}", edge.source, edge.source_handle),
                group_port_label: edge.source_handle.clone(),
                data_type: PortDataType::Any,
            });
        }
    }

    let position = average_node_position(&group_nodes);
    let group = NodeGroup {
        id: format!("group-{}", Uuid::new_v4()),
        name,
        nodes: group_nodes,
        edges: internal_edges,
        exposed_inputs,
        exposed_outputs,
        position,
        collapsed: true,
        description: None,
        color: None,
    };
    let group_id = group.id.clone();
    let group_node = GraphNode {
        id: group_id.clone(),
        node_type: GROUP_NODE_TYPE.to_string(),
        position: group.position.clone(),
        data: json!({
            "label": group.name,
            "group": group,
            "isGroup": true,
        }),
    };
    let group = node_group_from_node(&group_node)?;

    let nodes = graph
        .nodes
        .iter()
        .filter(|node| !selected_set.contains(node.id.as_str()))
        .cloned()
        .chain(std::iter::once(group_node))
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .iter()
        .filter_map(|edge| grouped_boundary_edge(edge, &group, &selected_set))
        .collect::<Vec<_>>();

    Ok(WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    })
}

pub(crate) fn ungroup_node_graph(
    graph: &WorkflowGraph,
    group_id: &str,
) -> Result<WorkflowGraph, WorkflowServiceError> {
    let group_node = graph
        .nodes
        .iter()
        .find(|node| node.id == group_id)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!("group '{}' was not found", group_id))
        })?;
    let group = node_group_from_node(group_node)?;

    let mut nodes = graph
        .nodes
        .iter()
        .filter(|node| node.id != group_id)
        .cloned()
        .collect::<Vec<_>>();
    nodes.extend(group.nodes.clone());

    let mut edges = graph
        .edges
        .iter()
        .filter_map(|edge| ungrouped_boundary_edge(edge, &group))
        .collect::<Vec<_>>();
    edges.extend(group.edges.clone());

    Ok(WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    })
}

pub(crate) fn update_group_ports_graph(
    graph: &WorkflowGraph,
    group_id: &str,
    exposed_inputs: Vec<PortMapping>,
    exposed_outputs: Vec<PortMapping>,
) -> Result<WorkflowGraph, WorkflowServiceError> {
    let mut graph = graph.clone();
    let node = graph.find_node_mut(group_id).ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!("group '{}' was not found", group_id))
    })?;
    let mut group = node_group_from_node(node)?;
    validate_port_mappings(&group, &exposed_inputs)?;
    validate_port_mappings(&group, &exposed_outputs)?;

    group.exposed_inputs = exposed_inputs;
    group.exposed_outputs = exposed_outputs;
    let label = group.name.clone();
    node.data = json!({
        "label": label,
        "group": group,
        "isGroup": true,
    });
    Ok(graph)
}

fn average_node_position(nodes: &[GraphNode]) -> super::types::Position {
    if nodes.is_empty() {
        return super::types::Position::default();
    }
    let sum_x = nodes.iter().map(|node| node.position.x).sum::<f64>();
    let sum_y = nodes.iter().map(|node| node.position.y).sum::<f64>();
    let count = nodes.len() as f64;
    super::types::Position {
        x: sum_x / count,
        y: sum_y / count,
    }
}

fn grouped_boundary_edge(
    edge: &GraphEdge,
    group: &NodeGroup,
    selected_set: &HashSet<&str>,
) -> Option<GraphEdge> {
    let source_inside = selected_set.contains(edge.source.as_str());
    let target_inside = selected_set.contains(edge.target.as_str());
    if source_inside && target_inside {
        return None;
    }
    if target_inside {
        let mapping = group.exposed_inputs.iter().find(|mapping| {
            mapping.internal_node_id == edge.target
                && mapping.internal_port_id == edge.target_handle
        })?;
        return Some(GraphEdge {
            target: group.id.clone(),
            target_handle: mapping.group_port_id.clone(),
            ..edge.clone()
        });
    }
    if source_inside {
        let mapping = group.exposed_outputs.iter().find(|mapping| {
            mapping.internal_node_id == edge.source
                && mapping.internal_port_id == edge.source_handle
        })?;
        return Some(GraphEdge {
            source: group.id.clone(),
            source_handle: mapping.group_port_id.clone(),
            ..edge.clone()
        });
    }
    Some(edge.clone())
}

fn ungrouped_boundary_edge(edge: &GraphEdge, group: &NodeGroup) -> Option<GraphEdge> {
    if edge.target == group.id {
        let mapping = group
            .exposed_inputs
            .iter()
            .find(|mapping| mapping.group_port_id == edge.target_handle)?;
        return Some(GraphEdge {
            target: mapping.internal_node_id.clone(),
            target_handle: mapping.internal_port_id.clone(),
            ..edge.clone()
        });
    }
    if edge.source == group.id {
        let mapping = group
            .exposed_outputs
            .iter()
            .find(|mapping| mapping.group_port_id == edge.source_handle)?;
        return Some(GraphEdge {
            source: mapping.internal_node_id.clone(),
            source_handle: mapping.internal_port_id.clone(),
            ..edge.clone()
        });
    }
    Some(edge.clone())
}

fn validate_port_mappings(
    group: &NodeGroup,
    mappings: &[PortMapping],
) -> Result<(), WorkflowServiceError> {
    let node_ids = group
        .nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    for mapping in mappings {
        if !node_ids.contains(mapping.internal_node_id.as_str()) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "Internal node '{}' not found in group",
                mapping.internal_node_id
            )));
        }
    }
    Ok(())
}

fn node_group_from_node(node: &GraphNode) -> Result<NodeGroup, WorkflowServiceError> {
    serde_json::from_value::<NodeGroup>(node.data.get("group").cloned().ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!(
            "node '{}' does not contain group data",
            node.id
        ))
    })?)
    .map_err(|error| {
        WorkflowServiceError::InvalidRequest(format!(
            "node '{}' contains invalid group data: {}",
            node.id, error
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::types::Position;

    fn node(id: &str, x: f64, y: f64) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type: "text-input".to_string(),
            position: Position { x, y },
            data: json!({ "label": id }),
        }
    }

    fn edge(
        id: &str,
        source: &str,
        source_handle: &str,
        target: &str,
        target_handle: &str,
    ) -> GraphEdge {
        GraphEdge {
            id: id.to_string(),
            source: source.to_string(),
            source_handle: source_handle.to_string(),
            target: target.to_string(),
            target_handle: target_handle.to_string(),
        }
    }

    fn graph_with_boundaries() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                node("source", 0.0, 0.0),
                node("a", 100.0, 0.0),
                node("b", 200.0, 0.0),
                node("sink", 300.0, 0.0),
            ],
            edges: vec![
                edge("in", "source", "text", "a", "text"),
                edge("internal", "a", "text", "b", "text"),
                edge("out", "b", "text", "sink", "text"),
            ],
            derived_graph: None,
        }
    }

    #[test]
    fn create_node_group_graph_collapses_selected_nodes_and_rewrites_boundaries() {
        let graph = graph_with_boundaries();
        let grouped = create_node_group_graph(
            &graph,
            "Group".to_string(),
            &["a".to_string(), "b".to_string()],
        )
        .expect("create group");
        let group_node = grouped
            .nodes
            .iter()
            .find(|node| node.node_type == GROUP_NODE_TYPE)
            .expect("group node");
        let group = node_group_from_node(group_node).expect("group data");

        assert_eq!(group.nodes.len(), 2);
        assert_eq!(group.edges.len(), 1);
        assert_eq!(group.edges[0].id, "internal");
        assert!(grouped
            .nodes
            .iter()
            .all(|node| node.id != "a" && node.id != "b"));
        assert!(grouped.edges.iter().any(|edge| edge.id == "in"
            && edge.target == group.id
            && edge.target_handle == "in-a-text"));
        assert!(grouped.edges.iter().any(|edge| edge.id == "out"
            && edge.source == group.id
            && edge.source_handle == "out-b-text"));
    }

    #[test]
    fn ungroup_node_graph_restores_internal_nodes_and_boundary_edges() {
        let graph = graph_with_boundaries();
        let grouped = create_node_group_graph(
            &graph,
            "Group".to_string(),
            &["a".to_string(), "b".to_string()],
        )
        .expect("create group");
        let group_id = grouped
            .nodes
            .iter()
            .find(|node| node.node_type == GROUP_NODE_TYPE)
            .expect("group node")
            .id
            .clone();

        let ungrouped = ungroup_node_graph(&grouped, &group_id).expect("ungroup");

        assert!(ungrouped.nodes.iter().any(|node| node.id == "a"));
        assert!(ungrouped.nodes.iter().any(|node| node.id == "b"));
        assert!(ungrouped.nodes.iter().all(|node| node.id != group_id));
        assert!(ungrouped
            .edges
            .iter()
            .any(|edge| edge.id == "in" && edge.target == "a" && edge.target_handle == "text"));
        assert!(ungrouped
            .edges
            .iter()
            .any(|edge| edge.id == "internal" && edge.source == "a" && edge.target == "b"));
        assert!(ungrouped
            .edges
            .iter()
            .any(|edge| edge.id == "out" && edge.source == "b" && edge.source_handle == "text"));
    }
}
