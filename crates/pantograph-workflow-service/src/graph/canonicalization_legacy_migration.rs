use std::collections::HashSet;

use pantograph_node_contracts::{
    ContractUpgradeChange, ContractUpgradeOutcome, ContractUpgradeRecord, DiagnosticsLineagePolicy,
    NodeInstanceId, NodeTypeId, PortId, PortKind,
};

use super::super::types::WorkflowGraph;

pub(super) fn canonicalize_legacy_node_types(
    graph: WorkflowGraph,
) -> (WorkflowGraph, HashSet<String>) {
    let mut migrated_node_ids = HashSet::new();
    let nodes = graph
        .nodes
        .into_iter()
        .map(|mut node| {
            if node.node_type != "system-prompt" {
                return node;
            }
            migrated_node_ids.insert(node.id.clone());
            node.node_type = "text-input".to_string();
            if let Some(data) = node.data.as_object_mut() {
                if let Some(prompt) = data.remove("prompt") {
                    data.entry("text".to_string()).or_insert(prompt);
                }
            }
            node
        })
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .into_iter()
        .map(|mut edge| {
            if migrated_node_ids.contains(&edge.source) && edge.source_handle == "prompt" {
                edge.source_handle = "text".to_string();
            }
            if migrated_node_ids.contains(&edge.target) && edge.target_handle == "prompt" {
                edge.target_handle = "text".to_string();
            }
            edge
        })
        .collect::<Vec<_>>();
    (
        WorkflowGraph {
            nodes,
            edges,
            derived_graph: None,
        },
        migrated_node_ids,
    )
}

pub(super) fn legacy_node_type_migration_records(
    migrated_node_ids: &HashSet<String>,
) -> Vec<ContractUpgradeRecord> {
    let mut records = migrated_node_ids
        .iter()
        .filter_map(|node_id| legacy_system_prompt_migration_record(node_id))
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        let left_node = upgrade_record_node_id(left);
        let right_node = upgrade_record_node_id(right);
        left_node.cmp(&right_node)
    });
    records
}

fn legacy_system_prompt_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("system-prompt".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::PreservePrimitiveLineage,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("system-prompt".to_string()).ok()?,
                to: NodeTypeId::try_from("text-input".to_string()).ok()?,
            },
            ContractUpgradeChange::PortIdChanged {
                node_id,
                kind: PortKind::Output,
                from: PortId::try_from("prompt".to_string()).ok()?,
                to: PortId::try_from("text".to_string()).ok()?,
            },
        ],
        diagnostics: Vec::new(),
    };
    record.validate().ok()?;
    Some(record)
}

fn upgrade_record_node_id(record: &ContractUpgradeRecord) -> String {
    record
        .changes
        .iter()
        .find_map(|change| match change {
            ContractUpgradeChange::NodeTypeChanged { node_id, .. }
            | ContractUpgradeChange::PortIdChanged { node_id, .. }
            | ContractUpgradeChange::PortAdded { node_id, .. }
            | ContractUpgradeChange::PortRemoved { node_id, .. } => {
                Some(node_id.as_str().to_string())
            }
            ContractUpgradeChange::VolatileProjectionRegenerated { .. } => None,
        })
        .unwrap_or_default()
}
