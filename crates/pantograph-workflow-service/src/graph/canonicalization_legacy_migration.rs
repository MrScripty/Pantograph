use std::collections::HashMap;

use pantograph_node_contracts::{
    ContractUpgradeChange, ContractUpgradeDiagnostic, ContractUpgradeOutcome,
    ContractUpgradeRecord, ContractUpgradeRejectionReason, DiagnosticsLineagePolicy,
    NodeInstanceId, NodeTypeId, PortId, PortKind,
};
use serde_json::json;

use super::super::types::WorkflowGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LegacyNodeMigrationKind {
    SystemPrompt,
    OllamaInference,
}

pub(super) fn canonicalize_legacy_node_types(
    graph: WorkflowGraph,
) -> (WorkflowGraph, HashMap<String, LegacyNodeMigrationKind>) {
    let mut migrated_nodes = HashMap::new();
    let nodes = graph
        .nodes
        .into_iter()
        .map(|mut node| {
            match node.node_type.as_str() {
                "system-prompt" => {
                    migrated_nodes.insert(node.id.clone(), LegacyNodeMigrationKind::SystemPrompt);
                    node.node_type = "text-input".to_string();
                    if let Some(data) = node.data.as_object_mut() {
                        if let Some(prompt) = data.remove("prompt") {
                            data.entry("text".to_string()).or_insert(prompt);
                        }
                    }
                }
                "ollama-inference" => {
                    migrated_nodes
                        .insert(node.id.clone(), LegacyNodeMigrationKind::OllamaInference);
                    node.node_type = "llm-inference".to_string();
                    migrate_ollama_node_data(&mut node.data);
                }
                _ => {}
            }

            node
        })
        .collect::<Vec<_>>();
    let edges = graph
        .edges
        .into_iter()
        .filter_map(|edge| {
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::OllamaInference)
                && matches!(edge.source_handle.as_str(), "model_used" | "model_ref")
            {
                return None;
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::OllamaInference)
                && matches!(
                    edge.target_handle.as_str(),
                    "model" | "temperature" | "max_tokens"
                )
            {
                return None;
            }
            Some(edge)
        })
        .map(|mut edge| {
            if migrated_nodes.get(&edge.source) == Some(&LegacyNodeMigrationKind::SystemPrompt)
                && edge.source_handle == "prompt"
            {
                edge.source_handle = "text".to_string();
            }
            if migrated_nodes.get(&edge.target) == Some(&LegacyNodeMigrationKind::SystemPrompt)
                && edge.target_handle == "prompt"
            {
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
        migrated_nodes,
    )
}

pub(super) fn legacy_node_type_migration_records(
    migrated_nodes: &HashMap<String, LegacyNodeMigrationKind>,
) -> Vec<ContractUpgradeRecord> {
    let mut records = migrated_nodes
        .iter()
        .filter_map(|(node_id, migration)| match migration {
            LegacyNodeMigrationKind::SystemPrompt => legacy_system_prompt_migration_record(node_id),
            LegacyNodeMigrationKind::OllamaInference => legacy_ollama_migration_record(node_id),
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| {
        let left_node = upgrade_record_node_id(left);
        let right_node = upgrade_record_node_id(right);
        left_node.cmp(&right_node)
    });
    records
}

fn migrate_ollama_node_data(data: &mut serde_json::Value) {
    if !data.is_object() {
        *data = json!({});
    }

    let Some(object) = data.as_object_mut() else {
        return;
    };
    let legacy_model = object.get("model").cloned();
    let legacy_temperature = object.get("temperature").cloned();
    let legacy_max_tokens = object.get("max_tokens").cloned();

    object
        .entry("task_kind".to_string())
        .or_insert_with(|| json!("text_generation"));
    object
        .entry("runtime_hint".to_string())
        .or_insert_with(|| json!("retired_ollama"));
    object
        .entry("pumas_model_ref".to_string())
        .or_insert_with(|| {
            json!({
                "status": "unresolved",
                "source": "legacy_ollama",
                "legacy_model": legacy_model,
                "message": "Ollama is retired as a first-party Pantograph backend; select a Pumas model reference before running this node."
            })
        });
    object
        .entry("migration_diagnostics".to_string())
        .or_insert_with(|| {
            json!([{
                "code": "legacy_ollama_backend_retired",
                "severity": "error",
                "message": "Migrated from ollama-inference to llm-inference without preserving Ollama execution support. Select a Pumas model reference and supported runtime before execution.",
                "legacy_model": legacy_model,
                "legacy_temperature": legacy_temperature,
                "legacy_max_tokens": legacy_max_tokens
            }])
        });
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

fn legacy_ollama_migration_record(node_id: &str) -> Option<ContractUpgradeRecord> {
    let node_id = NodeInstanceId::try_from(node_id.to_string()).ok()?;
    let record = ContractUpgradeRecord {
        node_type: NodeTypeId::try_from("ollama-inference".to_string()).ok()?,
        outcome: ContractUpgradeOutcome::Upgraded,
        source_contract_version: Some("0.0.0".to_string()),
        source_contract_digest: None,
        target_contract_version: Some("1.0.0".to_string()),
        target_contract_digest: None,
        diagnostics_lineage: DiagnosticsLineagePolicy::RejectToAvoidSilentChange,
        changes: vec![
            ContractUpgradeChange::NodeTypeChanged {
                node_id: node_id.clone(),
                from: NodeTypeId::try_from("ollama-inference".to_string()).ok()?,
                to: NodeTypeId::try_from("llm-inference".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("model".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("temperature".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Input,
                port_id: PortId::try_from("max_tokens".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_used".to_string()).ok()?,
            },
            ContractUpgradeChange::PortRemoved {
                node_id: node_id.clone(),
                kind: PortKind::Output,
                port_id: PortId::try_from("model_ref".to_string()).ok()?,
            },
        ],
        diagnostics: vec![ContractUpgradeDiagnostic {
            reason: ContractUpgradeRejectionReason::UnsupportedLegacyContract,
            message: "Ollama execution is retired; this node was migrated to llm-inference with an unresolved Pumas model reference diagnostic.".to_string(),
            node_id: Some(node_id),
            node_type: Some(NodeTypeId::try_from("ollama-inference".to_string()).ok()?),
            port_id: None,
        }],
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
