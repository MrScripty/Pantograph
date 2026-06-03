use std::collections::HashMap;

use pantograph_node_contracts::ContractUpgradeRecord;

use super::registry::NodeRegistry;
use super::types::{GraphEdge, GraphNode, WorkflowGraph};

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowGraphCanonicalizationResult {
    pub graph: WorkflowGraph,
    pub migration_records: Vec<ContractUpgradeRecord>,
}

pub fn canonicalize_workflow_graph(graph: WorkflowGraph, registry: &NodeRegistry) -> WorkflowGraph {
    canonicalize_workflow_graph_with_migrations(graph, registry).graph
}

pub fn canonicalize_workflow_graph_with_migrations(
    graph: WorkflowGraph,
    _registry: &NodeRegistry,
) -> WorkflowGraphCanonicalizationResult {
    let mut migration_records: Vec<ContractUpgradeRecord> = Vec::new();
    let nodes = graph.nodes;
    let mut edges = graph.edges;
    canonicalize_inference_text_output_edges(&nodes, &mut edges);

    let graph = WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    };
    migration_records.sort_by(|left, right| left.node_type.as_str().cmp(right.node_type.as_str()));

    WorkflowGraphCanonicalizationResult {
        graph,
        migration_records,
    }
}

fn canonicalize_inference_text_output_edges(nodes: &[GraphNode], edges: &mut [GraphEdge]) {
    let node_types = nodes
        .iter()
        .map(|node| (node.id.as_str(), node.node_type.as_str()))
        .collect::<HashMap<_, _>>();

    for edge in edges {
        let source_is_llm = node_types.get(edge.source.as_str()).copied() == Some("llm-inference");
        let target_is_text_output =
            node_types.get(edge.target.as_str()).copied() == Some("text-output");
        if source_is_llm
            && target_is_text_output
            && edge.source_handle == "stream"
            && edge.target_handle == "stream"
        {
            edge.source_handle = "response".to_string();
            edge.target_handle = "text".to_string();
            edge.id = format!("{}-response-{}-text", edge.source, edge.target);
        }
    }
}

#[cfg(test)]
#[path = "canonicalization_tests.rs"]
mod tests;
