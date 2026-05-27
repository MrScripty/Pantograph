use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::preflight::{
    reject_executable_dependency_payload_fields, reject_path_shaped_dependency_fields,
    validate_contract_version, DependencyPreflightResult,
};
use crate::readiness::DependencyReadinessRequest;
use crate::request::{
    validate_identifier, DependencyBindingId, DependencyOverrideFingerprint,
    DependencyRequirementsId,
};

macro_rules! readiness_execution_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $name {
            type Err = DependencyPlanningContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = DependencyPlanningContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

readiness_execution_id!(DependencyReadinessWorkflowId, "workflow_id");
readiness_execution_id!(DependencyReadinessWorkflowRunId, "workflow_run_id");
readiness_execution_id!(DependencyReadinessSchedulerTaskId, "scheduler_task_id");
readiness_execution_id!(DependencyReadinessNodeId, "node_id");
readiness_execution_id!(DependencyReadinessGraphRevision, "graph_revision");
readiness_execution_id!(
    DependencyReadinessValidationSessionId,
    "validation_session_id"
);
readiness_execution_id!(
    DependencyReadinessValidationSnapshotId,
    "validation_snapshot_id"
);
readiness_execution_id!(
    DependencyReadinessDescriptorFingerprint,
    "descriptor_fingerprint"
);
readiness_execution_id!(DependencyReadinessProofId, "readiness_proof_id");
readiness_execution_id!(DependencyReadinessCorrelationId, "correlation_id");

/// Version of a scheduler-admitted dependency readiness proof.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
#[must_use]
pub struct DependencyReadinessProofVersion(u32);

impl DependencyReadinessProofVersion {
    pub fn parse(value: u32) -> Result<Self, DependencyPlanningContractError> {
        if value == 0 {
            Err(DependencyPlanningContractError::InvalidField {
                field: "readiness_proof_version",
                reason: "readiness proof version must be greater than zero",
            })
        } else {
            Ok(Self(value))
        }
    }

    pub fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for DependencyReadinessProofVersion {
    type Error = DependencyPlanningContractError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// Path-free execution freshness identity for dependency readiness admission.
///
/// This context links a dependency readiness request/proof to the active
/// scheduler task and executable validation snapshot that authorized it. It
/// deliberately carries ids and fingerprints only.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub struct DependencyReadinessExecutionContext {
    #[serde(default = "default_dependency_readiness_execution_contract_version")]
    pub contract_version: u32,
    pub workflow_id: DependencyReadinessWorkflowId,
    pub workflow_run_id: DependencyReadinessWorkflowRunId,
    pub scheduler_task_id: DependencyReadinessSchedulerTaskId,
    pub node_id: DependencyReadinessNodeId,
    pub graph_revision: DependencyReadinessGraphRevision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_session_id: Option<DependencyReadinessValidationSessionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validation_snapshot_id: Option<DependencyReadinessValidationSnapshotId>,
    pub descriptor_fingerprint: DependencyReadinessDescriptorFingerprint,
    pub dependency_requirements_id: DependencyRequirementsId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_override_fingerprint: Option<DependencyOverrideFingerprint>,
    pub correlation_id: DependencyReadinessCorrelationId,
}

impl DependencyReadinessExecutionContext {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_readiness_execution_context.contract_version",
            "only dependency readiness execution context contract version 1 is supported",
        )?;
        if self.validation_session_id.is_none() && self.validation_snapshot_id.is_none() {
            return Err(DependencyPlanningContractError::MissingField {
                field: "validation_session_id_or_validation_snapshot_id",
            });
        }
        validate_unique_binding_ids(&self.selected_binding_ids)
    }

    fn validate_matches_readiness_request(
        &self,
        request: &DependencyReadinessRequest,
    ) -> Result<(), DependencyPlanningContractError> {
        self.validate()?;
        request.validate()?;
        if self.selected_binding_ids != request.identity_key.selected_binding_ids {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "execution_context.selected_binding_ids",
                reason: "execution context selected bindings must match readiness identity key",
            });
        }
        let caller_context = &request.planning_request.caller_context;
        if let Some(workflow_id) = &caller_context.workflow_id {
            if workflow_id != self.workflow_id.as_str() {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "execution_context.workflow_id",
                    reason: "execution context workflow id must match readiness caller context",
                });
            }
        }
        if let Some(run_id) = &caller_context.run_id {
            if run_id != self.workflow_run_id.as_str() {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "execution_context.workflow_run_id",
                    reason: "execution context workflow run id must match readiness caller context",
                });
            }
        }
        if let Some(node_id) = &caller_context.node_id {
            if node_id != self.node_id.as_str() {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "execution_context.node_id",
                    reason: "execution context node id must match readiness caller context",
                });
            }
        }
        Ok(())
    }

    fn validate_matches_preflight_result(
        &self,
        result: &DependencyPreflightResult,
    ) -> Result<(), DependencyPlanningContractError> {
        self.validate()?;
        result.validate()?;
        if self.selected_binding_ids != result.identity_key.selected_binding_ids {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "execution_context.selected_binding_ids",
                reason: "execution context selected bindings must match preflight identity key",
            });
        }
        if let Some(result_requirements_id) = &result.dependency_requirements_id {
            if result_requirements_id != &self.dependency_requirements_id {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "execution_context.dependency_requirements_id",
                    reason: "execution context requirements id must match preflight result proof",
                });
            }
        }
        Ok(())
    }
}

/// Provider-facing readiness request plus execution freshness context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub struct DependencyReadinessRequestEnvelope {
    #[serde(default = "default_dependency_readiness_execution_contract_version")]
    pub contract_version: u32,
    pub execution_context: DependencyReadinessExecutionContext,
    pub readiness_request: DependencyReadinessRequest,
}

impl DependencyReadinessRequestEnvelope {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_readiness_request_envelope.contract_version",
            "only dependency readiness request envelope contract version 1 is supported",
        )?;
        self.execution_context
            .validate_matches_readiness_request(&self.readiness_request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyReadinessExecutionContext(DependencyReadinessExecutionContext);

impl ValidatedDependencyReadinessExecutionContext {
    pub fn into_inner(self) -> DependencyReadinessExecutionContext {
        self.0
    }

    pub fn as_context(&self) -> &DependencyReadinessExecutionContext {
        &self.0
    }
}

impl TryFrom<DependencyReadinessExecutionContext> for ValidatedDependencyReadinessExecutionContext {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyReadinessExecutionContext) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyReadinessExecutionContext {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_forbidden_execution_envelope_fields(
            &value,
            "dependency_readiness_execution_context",
        )?;
        let context: DependencyReadinessExecutionContext =
            serde_json::from_value(value).map_err(|_| {
                DependencyPlanningContractError::InvalidField {
                    field: "dependency_readiness_execution_context",
                    reason: "context JSON did not match dependency readiness execution contract",
                }
            })?;
        Self::try_from(context)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyReadinessRequestEnvelope(DependencyReadinessRequestEnvelope);

impl ValidatedDependencyReadinessRequestEnvelope {
    pub fn into_inner(self) -> DependencyReadinessRequestEnvelope {
        self.0
    }

    pub fn as_envelope(&self) -> &DependencyReadinessRequestEnvelope {
        &self.0
    }
}

impl TryFrom<DependencyReadinessRequestEnvelope> for ValidatedDependencyReadinessRequestEnvelope {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyReadinessRequestEnvelope) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyReadinessRequestEnvelope {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_forbidden_execution_envelope_fields(
            &value,
            "dependency_readiness_request_envelope",
        )?;
        let envelope: DependencyReadinessRequestEnvelope =
            serde_json::from_value(value).map_err(|_| {
                DependencyPlanningContractError::InvalidField {
                    field: "dependency_readiness_request_envelope",
                    reason: "request envelope JSON did not match dependency readiness contract",
                }
            })?;
        Self::try_from(envelope)
    }
}

/// Scheduler-facing readiness proof plus execution freshness context.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub struct DependencyReadinessProofEnvelope {
    #[serde(default = "default_dependency_readiness_execution_contract_version")]
    pub contract_version: u32,
    pub execution_context: DependencyReadinessExecutionContext,
    pub preflight_result: DependencyPreflightResult,
    pub readiness_proof_id: DependencyReadinessProofId,
    pub readiness_proof_version: DependencyReadinessProofVersion,
}

impl DependencyReadinessProofEnvelope {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(
            self.contract_version,
            "dependency_readiness_proof_envelope.contract_version",
            "only dependency readiness proof envelope contract version 1 is supported",
        )?;
        self.execution_context
            .validate_matches_preflight_result(&self.preflight_result)?;
        self.readiness_proof_version.validate()
    }

    pub fn validate_matches_request_envelope(
        &self,
        request_envelope: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<(), DependencyPlanningContractError> {
        self.validate()?;
        let request_envelope = request_envelope.as_envelope();
        request_envelope.validate()?;
        if self.execution_context != request_envelope.execution_context {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_readiness_proof_envelope.execution_context",
                reason: "proof envelope execution context must match readiness request envelope",
            });
        }
        if self.preflight_result.identity_key != request_envelope.readiness_request.identity_key {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_readiness_proof_envelope.preflight_result.identity_key",
                reason: "proof identity key must match readiness request identity key",
            });
        }
        Ok(())
    }
}

impl DependencyReadinessProofVersion {
    fn validate(self) -> Result<(), DependencyPlanningContractError> {
        Self::parse(self.0).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyReadinessProofEnvelope(DependencyReadinessProofEnvelope);

impl ValidatedDependencyReadinessProofEnvelope {
    pub fn into_inner(self) -> DependencyReadinessProofEnvelope {
        self.0
    }

    pub fn as_envelope(&self) -> &DependencyReadinessProofEnvelope {
        &self.0
    }
}

impl TryFrom<DependencyReadinessProofEnvelope> for ValidatedDependencyReadinessProofEnvelope {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyReadinessProofEnvelope) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyReadinessProofEnvelope {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_forbidden_execution_envelope_fields(&value, "dependency_readiness_proof_envelope")?;
        let envelope: DependencyReadinessProofEnvelope =
            serde_json::from_value(value).map_err(|_| {
                DependencyPlanningContractError::InvalidField {
                    field: "dependency_readiness_proof_envelope",
                    reason: "proof envelope JSON did not match dependency readiness contract",
                }
            })?;
        Self::try_from(envelope)
    }
}

fn default_dependency_readiness_execution_contract_version() -> u32 {
    1
}

fn validate_unique_binding_ids(
    selected_binding_ids: &[DependencyBindingId],
) -> Result<(), DependencyPlanningContractError> {
    let mut seen = std::collections::BTreeSet::new();
    for id in selected_binding_ids {
        if !seen.insert(id.as_str()) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "execution_context.selected_binding_ids",
                reason: "selected binding ids must be unique",
            });
        }
    }
    Ok(())
}

fn reject_forbidden_execution_envelope_fields(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<(), DependencyPlanningContractError> {
    reject_path_shaped_dependency_fields(
        value,
        field,
        "dependency readiness execution envelopes must not contain path-shaped fields",
    )?;
    reject_executable_dependency_payload_fields(
        value,
        field,
        "dependency readiness execution envelopes must not contain executable handoff fields",
    )
}
