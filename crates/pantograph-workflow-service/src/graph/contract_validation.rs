use std::collections::{HashMap, HashSet, VecDeque};

use super::effective_definition::effective_node_definition;
use super::registry::NodeRegistry;
use super::types::{GraphEdge, WorkflowGraph};
use super::validation::check_connection_ports;

pub fn validate_workflow_graph_contract(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
) -> Vec<String> {
    let mut errors = Vec::new();
    validate_unique_ids(graph, &mut errors);

    let target_counts = graph
        .edges
        .iter()
        .map(|edge| ((edge.target.as_str(), edge.target_handle.as_str()), 1usize))
        .fold(HashMap::new(), |mut counts, (key, count)| {
            *counts.entry(key).or_insert(0) += count;
            counts
        });

    for node in &graph.nodes {
        if let Err(error) = effective_node_definition(node, registry) {
            errors.push(format!(
                "node '{}' contract resolution failed: {:?}",
                node.id, error
            ));
        }
    }

    for edge in &graph.edges {
        validate_edge_contract(graph, registry, edge, &target_counts, &mut errors);
    }

    errors
}

fn validate_unique_ids(graph: &WorkflowGraph, errors: &mut Vec<String>) {
    let mut node_ids = HashSet::new();
    for node in &graph.nodes {
        if !node_ids.insert(node.id.as_str()) {
            errors.push(format!("duplicate node id '{}'", node.id));
        }
    }

    let mut edge_ids = HashSet::new();
    for edge in &graph.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            errors.push(format!("duplicate edge id '{}'", edge.id));
        }
    }
}

fn validate_edge_contract(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    edge: &GraphEdge,
    target_counts: &HashMap<(&str, &str), usize>,
    errors: &mut Vec<String>,
) {
    let Some(source_node) = graph.find_node(&edge.source) else {
        errors.push(format!(
            "edge '{}' references unknown source node '{}'",
            edge.id, edge.source
        ));
        return;
    };
    let Some(target_node) = graph.find_node(&edge.target) else {
        errors.push(format!(
            "edge '{}' references unknown target node '{}'",
            edge.id, edge.target
        ));
        return;
    };
    if source_node.id == target_node.id {
        errors.push(format!("edge '{}' connects node to itself", edge.id));
        return;
    }

    let Ok(source_definition) = effective_node_definition(source_node, registry) else {
        errors.push(format!(
            "edge '{}' source node '{}' has no resolvable contract",
            edge.id, source_node.id
        ));
        return;
    };
    let Ok(target_definition) = effective_node_definition(target_node, registry) else {
        errors.push(format!(
            "edge '{}' target node '{}' has no resolvable contract",
            edge.id, target_node.id
        ));
        return;
    };

    let Some(source_port) = source_definition
        .outputs
        .iter()
        .find(|port| port.id == edge.source_handle)
    else {
        errors.push(format!(
            "edge '{}' references unknown source output '{}.{}'",
            edge.id, edge.source, edge.source_handle
        ));
        return;
    };
    let Some(target_port) = target_definition
        .inputs
        .iter()
        .find(|port| port.id == edge.target_handle)
    else {
        errors.push(format!(
            "edge '{}' references unknown target input '{}.{}'",
            edge.id, edge.target, edge.target_handle
        ));
        return;
    };

    if !target_port.multiple
        && target_counts
            .get(&(edge.target.as_str(), edge.target_handle.as_str()))
            .is_some_and(|count| *count > 1)
    {
        errors.push(format!(
            "target input '{}.{}' has multiple incoming edges",
            edge.target, edge.target_handle
        ));
    }

    match check_connection_ports(&source_node.id, source_port, &target_node.id, target_port) {
        Ok(result) if result.is_compatible() => {}
        Ok(result) => {
            if let Some(diagnostic) = result.rejection {
                errors.push(format!(
                    "edge '{}' is incompatible: {}",
                    edge.id, diagnostic.message
                ));
            } else {
                errors.push(format!("edge '{}' is incompatible", edge.id));
            }
        }
        Err(error) => errors.push(format!(
            "edge '{}' compatibility check failed: {}",
            edge.id, error
        )),
    }

    if would_create_cycle(graph, &source_node.id, &target_node.id) {
        errors.push(format!("edge '{}' would create a cycle", edge.id));
    }
}

fn would_create_cycle(graph: &WorkflowGraph, source_node_id: &str, target_node_id: &str) -> bool {
    let mut queue = VecDeque::from([target_node_id.to_string()]);
    let mut visited = HashSet::new();

    while let Some(node_id) = queue.pop_front() {
        if !visited.insert(node_id.clone()) {
            continue;
        }
        if node_id == source_node_id {
            return true;
        }
        for edge in graph.outgoing_edges(&node_id) {
            queue.push_back(edge.target.clone());
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphEdge, GraphNode, Position};

    #[test]
    fn contract_validation_reports_canonical_incompatible_edges() {
        let registry = NodeRegistry::new();
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "image".to_string(),
                    node_type: "image-input".to_string(),
                    position: Position::default(),
                    data: serde_json::json!({}),
                },
                GraphNode {
                    id: "text".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position::default(),
                    data: serde_json::json!({}),
                },
            ],
            edges: vec![GraphEdge {
                id: "image-to-text".to_string(),
                source: "image".to_string(),
                source_handle: "image".to_string(),
                target: "text".to_string(),
                target_handle: "text".to_string(),
            }],
            derived_graph: None,
        };

        let errors = validate_workflow_graph_contract(&graph, &registry);

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("source type 'Image' is not compatible"));
    }
}
