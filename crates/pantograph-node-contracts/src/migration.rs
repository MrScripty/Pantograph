use serde::{Deserialize, Serialize};

use crate::{NodeContractError, NodeInstanceId, NodeTypeId, PortId, PortKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ContractUpgradeRecord {
    pub node_type: NodeTypeId,
    pub outcome: ContractUpgradeOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_contract_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_contract_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_contract_digest: Option<String>,
    pub diagnostics_lineage: DiagnosticsLineagePolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changes: Vec<ContractUpgradeChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ContractUpgradeDiagnostic>,
}

impl ContractUpgradeRecord {
    pub fn validate(&self) -> Result<(), NodeContractError> {
        if self.changes.is_empty() {
            return Err(NodeContractError::MissingContractUpgradeChange);
        }
        if self.outcome == ContractUpgradeOutcome::TypedRejection && self.diagnostics.is_empty() {
            return Err(NodeContractError::MissingContractUpgradeDiagnostic);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContractUpgradeOutcome {
    Upgraded,
    Regenerated,
    TypedRejection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsLineagePolicy {
    PreservePrimitiveLineage,
    RegenerateVolatileProjection,
    RejectToAvoidSilentChange,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ContractUpgradeChange {
    NodeTypeChanged {
        node_id: NodeInstanceId,
        from: NodeTypeId,
        to: NodeTypeId,
    },
    PortIdChanged {
        node_id: NodeInstanceId,
        kind: PortKind,
        from: PortId,
        to: PortId,
    },
    PortAdded {
        node_id: NodeInstanceId,
        kind: PortKind,
        port_id: PortId,
    },
    PortRemoved {
        node_id: NodeInstanceId,
        kind: PortKind,
        port_id: PortId,
    },
    VolatileProjectionRegenerated {
        projection: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ContractUpgradeDiagnostic {
    pub reason: ContractUpgradeRejectionReason,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeInstanceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_type: Option<NodeTypeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<PortId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ContractUpgradeRejectionReason {
    UnknownLegacyNodeType,
    UnknownLegacyPort,
    AmbiguousPortMapping,
    BehaviorChangeWouldBeSilent,
    PrimitiveLineageUnavailable,
    UnsupportedLegacyContract,
}
