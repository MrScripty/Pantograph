use std::collections::HashMap;

use pantograph_node_contracts::ContractUpgradeRecord;

use self::inference::{
    build_dynamic_expand_definition_json, build_expand_settings_schema, find_connected_targets,
    has_edge, parse_inference_settings, reconcile_inference_node, resolved_definition_json,
    set_node_definition, set_node_inference_settings,
};
use self::legacy_migration::{canonicalize_legacy_node_types, legacy_node_type_migration_records};
use super::registry::NodeRegistry;
use super::types::{GraphEdge, WorkflowGraph};

#[path = "canonicalization_inference.rs"]
mod inference;
#[path = "canonicalization_legacy_migration.rs"]
mod legacy_migration;

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
    registry: &NodeRegistry,
) -> WorkflowGraphCanonicalizationResult {
    let (graph, migrated_legacy_nodes) = canonicalize_legacy_node_types(graph);
    let mut migration_records = legacy_node_type_migration_records(&migrated_legacy_nodes);
    let mut nodes = graph.nodes;
    let mut edges = graph.edges;
    let node_indices = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.clone(), index))
        .collect::<HashMap<_, _>>();
    let source_node_ids = nodes
        .iter()
        .filter(|node| node.node_type != "expand-settings")
        .filter_map(|node| {
            parse_inference_settings(node.data.get("inference_settings")).map(|_| node.id.clone())
        })
        .collect::<Vec<_>>();

    for source_node_id in source_node_ids {
        let Some(source_index) = node_indices.get(&source_node_id).copied() else {
            continue;
        };
        let Some(inference_settings) =
            parse_inference_settings(nodes[source_index].data.get("inference_settings"))
        else {
            continue;
        };

        for target_node_id in find_connected_targets(&edges, &source_node_id, "inference_settings")
        {
            let Some(target_index) = node_indices.get(&target_node_id).copied() else {
                continue;
            };
            if nodes[target_index].node_type != "expand-settings" {
                reconcile_inference_node(&mut nodes[target_index], registry, &inference_settings);
                continue;
            }

            let Some(base_expand_definition) = registry.get_definition("expand-settings") else {
                continue;
            };
            let current_expand_definition =
                resolved_definition_json(&nodes[target_index], base_expand_definition);
            let downstream_node_ids =
                find_connected_targets(&edges, &target_node_id, "inference_settings");
            let downstream_base_definitions = downstream_node_ids
                .iter()
                .filter_map(|node_id| {
                    let node_index = node_indices.get(node_id).copied()?;
                    registry
                        .get_definition(nodes[node_index].node_type.as_str())
                        .cloned()
                })
                .collect::<Vec<_>>();
            let merged_expand_settings =
                build_expand_settings_schema(&downstream_base_definitions, &inference_settings);
            let expand_definition = build_dynamic_expand_definition_json(
                current_expand_definition,
                base_expand_definition,
                &merged_expand_settings,
            );
            set_node_definition(&mut nodes[target_index], expand_definition);
            set_node_inference_settings(&mut nodes[target_index], &merged_expand_settings);

            for downstream_node_id in downstream_node_ids {
                let Some(downstream_index) = node_indices.get(&downstream_node_id).copied() else {
                    continue;
                };
                let target_settings = reconcile_inference_node(
                    &mut nodes[downstream_index],
                    registry,
                    &inference_settings,
                );

                for param in target_settings {
                    if has_edge(
                        &edges,
                        &target_node_id,
                        &param.key,
                        &downstream_node_id,
                        &param.key,
                    ) {
                        continue;
                    }
                    edges.push(GraphEdge {
                        id: format!(
                            "{}-{}-{}-{}",
                            target_node_id, param.key, downstream_node_id, param.key
                        ),
                        source: target_node_id.clone(),
                        source_handle: param.key.clone(),
                        target: downstream_node_id.clone(),
                        target_handle: param.key,
                    });
                }
            }
        }
    }

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

#[cfg(test)]
#[path = "canonicalization_tests.rs"]
mod tests;
