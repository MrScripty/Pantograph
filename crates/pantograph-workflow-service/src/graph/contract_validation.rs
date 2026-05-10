use std::collections::{HashMap, HashSet, VecDeque};

use super::diagnostics::{
    WorkflowGraphDiagnostic, WorkflowGraphDiagnosticCode, WorkflowGraphDiagnosticSeverity,
};
use super::effective_definition::effective_node_definition;
use super::effective_definition::EffectiveDefinitionError;
use super::registry::NodeRegistry;
use super::types::{GraphEdge, GraphNode, WorkflowGraph};
use super::validation::check_connection_ports;

const RETIRED_NODE_TYPES: &[&str] = &[
    "diffusion-inference",
    "llamacpp-inference",
    "pytorch-inference",
    "ollama-inference",
    "embedding",
    "reranker",
    "vision-analysis",
];

pub fn validate_workflow_graph_contract(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
) -> Vec<String> {
    validate_workflow_graph_contract_diagnostics(graph, registry)
        .into_iter()
        .map(|diagnostic| diagnostic.message)
        .collect()
}

pub fn validate_workflow_graph_contract_diagnostics(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
) -> Vec<WorkflowGraphDiagnostic> {
    let mut diagnostics = Vec::new();
    validate_unique_ids(graph, &mut diagnostics);

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
            diagnostics.push(diagnostic_from_effective_definition_error(node, error));
        }
    }

    for edge in &graph.edges {
        validate_edge_contract(graph, registry, edge, &target_counts, &mut diagnostics);
    }

    diagnostics
}

fn validate_unique_ids(graph: &WorkflowGraph, diagnostics: &mut Vec<WorkflowGraphDiagnostic>) {
    let mut node_ids = HashSet::new();
    for node in &graph.nodes {
        if !node_ids.insert(node.id.as_str()) {
            diagnostics.push(
                WorkflowGraphDiagnostic::node(
                    WorkflowGraphDiagnosticCode::DuplicateNodeId,
                    WorkflowGraphDiagnosticSeverity::Error,
                    &node.id,
                    &node.node_type,
                    format!("duplicate node id '{}'", node.id),
                    true,
                )
                .with_detail("node_id", &node.id),
            );
        }
    }

    let mut edge_ids = HashSet::new();
    for edge in &graph.edges {
        if !edge_ids.insert(edge.id.as_str()) {
            diagnostics.push(
                WorkflowGraphDiagnostic::edge(
                    WorkflowGraphDiagnosticCode::DuplicateEdgeId,
                    WorkflowGraphDiagnosticSeverity::Error,
                    &edge.id,
                    format!("duplicate edge id '{}'", edge.id),
                    true,
                )
                .with_detail("source_node_id", &edge.source)
                .with_detail("source_port_id", &edge.source_handle)
                .with_detail("target_node_id", &edge.target)
                .with_detail("target_port_id", &edge.target_handle),
            );
        }
    }
}

fn validate_edge_contract(
    graph: &WorkflowGraph,
    registry: &NodeRegistry,
    edge: &GraphEdge,
    target_counts: &HashMap<(&str, &str), usize>,
    diagnostics: &mut Vec<WorkflowGraphDiagnostic>,
) {
    let Some(source_node) = graph.find_node(&edge.source) else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingEdgeSourceNode,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' references unknown source node '{}'",
                    edge.id, edge.source
                ),
                true,
            )
            .with_detail("source_node_id", &edge.source)
            .with_detail("source_port_id", &edge.source_handle),
        );
        return;
    };
    let Some(target_node) = graph.find_node(&edge.target) else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingEdgeTargetNode,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' references unknown target node '{}'",
                    edge.id, edge.target
                ),
                true,
            )
            .with_detail("target_node_id", &edge.target)
            .with_detail("target_port_id", &edge.target_handle),
        );
        return;
    };
    if source_node.id == target_node.id {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::SelfConnection,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!("edge '{}' connects node to itself", edge.id),
                true,
            )
            .with_detail("node_id", &source_node.id),
        );
        return;
    }

    let Ok(source_definition) = effective_node_definition(source_node, registry) else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingSourceContract,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' source node '{}' has no resolvable contract",
                    edge.id, source_node.id
                ),
                true,
            )
            .with_detail("source_node_id", &source_node.id)
            .with_detail("source_node_type", &source_node.node_type),
        );
        return;
    };
    let Ok(target_definition) = effective_node_definition(target_node, registry) else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingTargetContract,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' target node '{}' has no resolvable contract",
                    edge.id, target_node.id
                ),
                true,
            )
            .with_detail("target_node_id", &target_node.id)
            .with_detail("target_node_type", &target_node.node_type),
        );
        return;
    };

    let Some(source_port) = source_definition
        .outputs
        .iter()
        .find(|port| port.id == edge.source_handle)
    else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingSourceOutput,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' references unknown source output '{}.{}'",
                    edge.id, edge.source, edge.source_handle
                ),
                true,
            )
            .with_detail("source_node_id", &edge.source)
            .with_detail("source_node_type", &source_node.node_type)
            .with_detail("source_port_id", &edge.source_handle),
        );
        return;
    };
    let Some(target_port) = target_definition
        .inputs
        .iter()
        .find(|port| port.id == edge.target_handle)
    else {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::MissingTargetInput,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "edge '{}' references unknown target input '{}.{}'",
                    edge.id, edge.target, edge.target_handle
                ),
                true,
            )
            .with_detail("target_node_id", &edge.target)
            .with_detail("target_node_type", &target_node.node_type)
            .with_detail("target_port_id", &edge.target_handle),
        );
        return;
    };

    if !target_port.multiple
        && target_counts
            .get(&(edge.target.as_str(), edge.target_handle.as_str()))
            .is_some_and(|count| *count > 1)
    {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::TargetInputCapacityReached,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!(
                    "target input '{}.{}' has multiple incoming edges",
                    edge.target, edge.target_handle
                ),
                true,
            )
            .with_detail("target_node_id", &edge.target)
            .with_detail("target_port_id", &edge.target_handle),
        );
    }

    match check_connection_ports(&source_node.id, source_port, &target_node.id, target_port) {
        Ok(result) if result.is_compatible() => {}
        Ok(result) => {
            if let Some(diagnostic) = result.rejection {
                diagnostics.push(
                    WorkflowGraphDiagnostic::edge(
                        WorkflowGraphDiagnosticCode::IncompatiblePortTypes,
                        WorkflowGraphDiagnosticSeverity::Error,
                        &edge.id,
                        format!("edge '{}' is incompatible: {}", edge.id, diagnostic.message),
                        true,
                    )
                    .with_detail("source_node_id", &source_node.id)
                    .with_detail("source_port_id", &source_port.id)
                    .with_detail("target_node_id", &target_node.id)
                    .with_detail("target_port_id", &target_port.id)
                    .with_detail("rejection_reason", format!("{:?}", diagnostic.reason)),
                );
            } else {
                diagnostics.push(WorkflowGraphDiagnostic::edge(
                    WorkflowGraphDiagnosticCode::IncompatiblePortTypes,
                    WorkflowGraphDiagnosticSeverity::Error,
                    &edge.id,
                    format!("edge '{}' is incompatible", edge.id),
                    true,
                ));
            }
        }
        Err(error) => diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::CompatibilityCheckFailed,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!("edge '{}' compatibility check failed: {}", edge.id, error),
                true,
            )
            .with_detail("source_node_id", &source_node.id)
            .with_detail("source_port_id", &source_port.id)
            .with_detail("target_node_id", &target_node.id)
            .with_detail("target_port_id", &target_port.id),
        ),
    }

    if would_create_cycle(graph, &source_node.id, &target_node.id) {
        diagnostics.push(
            WorkflowGraphDiagnostic::edge(
                WorkflowGraphDiagnosticCode::CycleDetected,
                WorkflowGraphDiagnosticSeverity::Error,
                &edge.id,
                format!("edge '{}' would create a cycle", edge.id),
                true,
            )
            .with_detail("source_node_id", &source_node.id)
            .with_detail("target_node_id", &target_node.id),
        );
    }
}

fn diagnostic_from_effective_definition_error(
    node: &GraphNode,
    error: EffectiveDefinitionError,
) -> WorkflowGraphDiagnostic {
    match error {
        EffectiveDefinitionError::UnknownNodeType(node_type)
            if RETIRED_NODE_TYPES.contains(&node_type.as_str()) =>
        {
            WorkflowGraphDiagnostic::node(
                WorkflowGraphDiagnosticCode::RetiredNodeType,
                WorkflowGraphDiagnosticSeverity::Error,
                &node.id,
                &node_type,
                format!(
                    "retired node type '{}' is no longer executable; use canonical llm-inference",
                    node_type
                ),
                true,
            )
            .with_detail("replacement_node_type", "llm-inference")
        }
        EffectiveDefinitionError::UnknownNodeType(node_type) => WorkflowGraphDiagnostic::node(
            WorkflowGraphDiagnosticCode::UnknownNodeType,
            WorkflowGraphDiagnosticSeverity::Error,
            &node.id,
            &node_type,
            format!("node '{}' has unknown node type '{}'", node.id, node_type),
            true,
        ),
        EffectiveDefinitionError::InvalidNodeId { node_id, message } => {
            WorkflowGraphDiagnostic::node(
                WorkflowGraphDiagnosticCode::InvalidNodeId,
                WorkflowGraphDiagnosticSeverity::Error,
                &node.id,
                &node.node_type,
                format!("node '{}' has invalid node id: {}", node.id, message),
                true,
            )
            .with_detail("invalid_node_id", node_id)
        }
        EffectiveDefinitionError::InvalidNodeType { node_type, message } => {
            WorkflowGraphDiagnostic::node(
                WorkflowGraphDiagnosticCode::InvalidNodeType,
                WorkflowGraphDiagnosticSeverity::Error,
                &node.id,
                &node.node_type,
                format!(
                    "node '{}' has invalid node type '{}': {}",
                    node.id, node_type, message
                ),
                true,
            )
            .with_detail("invalid_node_type", node_type)
        }
        EffectiveDefinitionError::InvalidDynamicDefinition { message } => {
            WorkflowGraphDiagnostic::node(
                WorkflowGraphDiagnosticCode::InvalidDynamicDefinition,
                WorkflowGraphDiagnosticSeverity::Error,
                &node.id,
                &node.node_type,
                format!(
                    "node '{}' dynamic contract definition is invalid: {}",
                    node.id, message
                ),
                true,
            )
        }
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
#[path = "contract_validation_tests.rs"]
mod tests;
