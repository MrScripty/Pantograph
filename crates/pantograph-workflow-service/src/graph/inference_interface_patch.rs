use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, InferenceInterfaceContractError,
    InferenceInterfaceDiagnostic, InferenceInterfaceDriftReport, InferenceInterfaceFingerprint,
    InferencePortId, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
const MAX_OPERATIONS: usize = 256;
const MAX_AFFECTED_EDGES: usize = 256;
const MAX_DIAGNOSTICS: usize = 128;

macro_rules! validated_graph_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn parse(
                value: impl AsRef<str>,
            ) -> Result<Self, InferenceInterfaceGraphPatchError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceGraphPatchError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains {actual_len} items; maximum is {max_len}")]
    TooManyItems {
        field: &'static str,
        actual_len: usize,
        max_len: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("inference interface contract error: {0}")]
    InferenceContract(#[from] InferenceInterfaceContractError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceUpdateProposal {
    #[serde(default = "default_contract_version")]
    pub contract_version: u32,
    pub proposal_id: GraphPatchProposalId,
    pub node_id: WorkflowNodeId,
    pub current_descriptor_fingerprint: InferenceInterfaceFingerprint,
    pub drift_report: InferenceInterfaceDriftReport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operations: Vec<InferenceInterfaceGraphPatchOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_edges: Vec<InferenceInterfaceAffectedEdge>,
    pub requires_confirmation: bool,
    pub destructive: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<InferenceInterfaceDiagnostic>,
}

impl InferenceInterfaceUpdateProposal {
    pub fn validate(&self) -> Result<(), InferenceInterfaceGraphPatchError> {
        if self.contract_version != INFERENCE_INTERFACE_CONTRACT_VERSION {
            return Err(InferenceInterfaceGraphPatchError::InvalidField {
                field: "proposal.contract_version",
                reason: "unsupported inference interface contract version",
            });
        }
        self.drift_report.validate()?;
        validate_collection_len("proposal.operations", self.operations.len(), MAX_OPERATIONS)?;
        validate_collection_len(
            "proposal.affected_edges",
            self.affected_edges.len(),
            MAX_AFFECTED_EDGES,
        )?;
        validate_collection_len(
            "proposal.diagnostics",
            self.diagnostics.len(),
            MAX_DIAGNOSTICS,
        )?;
        for operation in &self.operations {
            operation.validate()?;
        }
        for edge in &self.affected_edges {
            edge.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        if self.destructive && !self.requires_confirmation {
            return Err(InferenceInterfaceGraphPatchError::InvalidField {
                field: "proposal.requires_confirmation",
                reason: "destructive proposals require explicit confirmation",
            });
        }
        if self
            .operations
            .iter()
            .any(InferenceInterfaceGraphPatchOperation::destructive)
            && !self.destructive
        {
            return Err(InferenceInterfaceGraphPatchError::InvalidField {
                field: "proposal.destructive",
                reason: "destructive operations must mark the proposal destructive",
            });
        }
        if self.drift_report.blocking && self.operations.is_empty() {
            return Err(InferenceInterfaceGraphPatchError::InvalidField {
                field: "proposal.operations",
                reason: "blocking drift proposals must include patch operations",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "operation",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum InferenceInterfaceGraphPatchOperation {
    ReplaceAuthoredSnapshot {
        node_id: WorkflowNodeId,
        snapshot: AuthoredInferenceInterfaceSnapshot,
    },
    RemoveInvalidEdge {
        edge: InferenceInterfaceAffectedEdge,
        reason: InferenceInterfaceEdgeRemovalReason,
    },
    ClearInvalidLiteral {
        node_id: WorkflowNodeId,
        port_id: InferencePortId,
        reason: InferenceInterfaceLiteralRemovalReason,
    },
}

impl InferenceInterfaceGraphPatchOperation {
    fn validate(&self) -> Result<(), InferenceInterfaceGraphPatchError> {
        match self {
            Self::ReplaceAuthoredSnapshot { snapshot, .. } => {
                snapshot.validate().map_err(Into::into)
            }
            Self::RemoveInvalidEdge { edge, .. } => edge.validate(),
            Self::ClearInvalidLiteral { .. } => Ok(()),
        }
    }

    pub fn destructive(&self) -> bool {
        !matches!(self, Self::ReplaceAuthoredSnapshot { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct InferenceInterfaceAffectedEdge {
    pub edge_id: WorkflowEdgeId,
    pub source_node_id: WorkflowNodeId,
    pub source_port_id: InferencePortId,
    pub target_node_id: WorkflowNodeId,
    pub target_port_id: InferencePortId,
}

impl InferenceInterfaceAffectedEdge {
    fn validate(&self) -> Result<(), InferenceInterfaceGraphPatchError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceInterfaceEdgeRemovalReason {
    SourcePortRemoved,
    TargetPortRemoved,
    PortTypeChanged,
    RequirementChanged,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InferenceInterfaceLiteralRemovalReason {
    PortRemoved,
    PortTypeChanged,
    OptionUnavailable,
    DefaultChanged,
}

validated_graph_id!(GraphPatchProposalId, "proposal_id");
validated_graph_id!(WorkflowEdgeId, "edge_id");

fn default_contract_version() -> u32 {
    INFERENCE_INTERFACE_CONTRACT_VERSION
}

fn validate_collection_len(
    field: &'static str,
    actual_len: usize,
    max_len: usize,
) -> Result<(), InferenceInterfaceGraphPatchError> {
    if actual_len > max_len {
        return Err(InferenceInterfaceGraphPatchError::TooManyItems {
            field,
            actual_len,
            max_len,
        });
    }
    Ok(())
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, InferenceInterfaceGraphPatchError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(InferenceInterfaceGraphPatchError::MissingField { field });
    }
    if value.len() > MAX_ID_LEN {
        return Err(InferenceInterfaceGraphPatchError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':' | '/'))
    {
        return Err(InferenceInterfaceGraphPatchError::InvalidIdentifier { field });
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_inference_interface_contracts::{
        InferenceAvailability, InferenceDriftSeverity, InferenceInterfaceDriftChangeKind,
        InferencePortDirection, InferencePortRequirement, InferenceScalarType, InferenceTaskKind,
        InferenceValueType,
    };

    #[test]
    fn proposal_validates_snapshot_replacement() {
        let proposal = proposal_fixture(vec![
            InferenceInterfaceGraphPatchOperation::ReplaceAuthoredSnapshot {
                node_id: node_id("infer_1"),
                snapshot: snapshot_fixture(),
            },
        ]);

        proposal.validate().expect("proposal should validate");
    }

    #[test]
    fn destructive_operation_requires_confirmation() {
        let mut proposal = proposal_fixture(vec![
            InferenceInterfaceGraphPatchOperation::RemoveInvalidEdge {
                edge: affected_edge_fixture(),
                reason: InferenceInterfaceEdgeRemovalReason::TargetPortRemoved,
            },
        ]);
        proposal.destructive = true;
        proposal.requires_confirmation = false;

        assert_eq!(
            proposal
                .validate()
                .expect_err("destructive change must fail"),
            InferenceInterfaceGraphPatchError::InvalidField {
                field: "proposal.requires_confirmation",
                reason: "destructive proposals require explicit confirmation"
            }
        );
    }

    #[test]
    fn proposal_rejects_unknown_fields() {
        let json = serde_json::json!({
            "proposal_id": "proposal.1",
            "node_id": "infer_1",
            "current_descriptor_fingerprint": "iface.test.v1",
            "drift_report": drift_report_fixture(),
            "requires_confirmation": false,
            "destructive": false,
            "legacy_metadata": {}
        });

        let error = serde_json::from_value::<InferenceInterfaceUpdateProposal>(json)
            .expect_err("unknown fields must be rejected");
        assert!(error.to_string().contains("unknown field"));
    }

    fn proposal_fixture(
        operations: Vec<InferenceInterfaceGraphPatchOperation>,
    ) -> InferenceInterfaceUpdateProposal {
        InferenceInterfaceUpdateProposal {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            proposal_id: GraphPatchProposalId::parse("proposal.1").unwrap(),
            node_id: node_id("infer_1"),
            current_descriptor_fingerprint: fingerprint(),
            drift_report: drift_report_fixture(),
            operations,
            affected_edges: vec![affected_edge_fixture()],
            requires_confirmation: true,
            destructive: true,
            diagnostics: Vec::new(),
        }
    }

    fn drift_report_fixture() -> InferenceInterfaceDriftReport {
        InferenceInterfaceDriftReport {
            authored_fingerprint: InferenceInterfaceFingerprint::parse("iface.old.v1").unwrap(),
            current_fingerprint: fingerprint(),
            severity: InferenceDriftSeverity::Blocking,
            blocking: true,
            changes: vec![
                pantograph_inference_interface_contracts::InferenceInterfaceDriftChange {
                    kind: InferenceInterfaceDriftChangeKind::PortRemoved,
                    port_id: Some(port_id("prompt")),
                    message: "prompt was removed from current descriptor".to_string(),
                },
            ],
            diagnostics: Vec::new(),
        }
    }

    fn snapshot_fixture() -> AuthoredInferenceInterfaceSnapshot {
        AuthoredInferenceInterfaceSnapshot {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            descriptor_fingerprint: fingerprint(),
            task_kind: InferenceTaskKind::parse("image_generation").unwrap(),
            inputs: vec![
                pantograph_inference_interface_contracts::AuthoredInferencePortSnapshot {
                    port_id: port_id("prompt"),
                    label: "Prompt".to_string(),
                    direction: InferencePortDirection::Input,
                    requirement: InferencePortRequirement::Required,
                    value_type: InferenceValueType::Scalar(InferenceScalarType::String),
                    default: None,
                    availability: InferenceAvailability::available(),
                },
            ],
            outputs: Vec::new(),
        }
    }

    fn affected_edge_fixture() -> InferenceInterfaceAffectedEdge {
        InferenceInterfaceAffectedEdge {
            edge_id: WorkflowEdgeId::parse("edge.1").unwrap(),
            source_node_id: node_id("prompt_1"),
            source_port_id: port_id("text"),
            target_node_id: node_id("infer_1"),
            target_port_id: port_id("prompt"),
        }
    }

    fn fingerprint() -> InferenceInterfaceFingerprint {
        InferenceInterfaceFingerprint::parse("iface.test.v1").unwrap()
    }

    fn node_id(value: &str) -> WorkflowNodeId {
        WorkflowNodeId::parse(value).unwrap()
    }

    fn port_id(value: &str) -> InferencePortId {
        InferencePortId::parse(value).unwrap()
    }
}
