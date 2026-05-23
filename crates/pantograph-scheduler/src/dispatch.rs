use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use pantograph_dependency_planning::{
    DependencyEnvironmentRef, DependencyPlanningContractError, DeviceIntentId, PumasModelRef,
    RuntimeIntentId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SchedulerContractError;
use crate::intent::{SchedulableTaskIntent, SchedulerTraitSetting};
use crate::readiness::SchedulerDependencyReadinessProof;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;

/// Current contract version for scheduler dispatch decisions.
pub const SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION: u16 = 1;

macro_rules! dispatch_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, SchedulerContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            #[must_use]
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
            type Err = SchedulerContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = SchedulerContractError;

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

dispatch_id!(SchedulerRuntimeVariantId, "runtime_variant_id");
dispatch_id!(SchedulerBatchingGroupId, "batching_group_id");
dispatch_id!(SchedulerReservationLeaseId, "reservation_lease_id");

/// Dispatch decision diagnostic severity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerDispatchDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Stable diagnostic code for scheduler dispatch decisions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerDispatchDiagnosticCode {
    RuntimeSelected,
    DeviceSelected,
    ModelArtifactSelected,
    DependencyProofAccepted,
    BatchingGroupAssigned,
    ReservationLeaseAssigned,
    RuntimeRequirementSatisfied,
    DeviceRequirementSatisfied,
    SchedulerPolicyTrace,
}

/// Bounded diagnostic emitted with a dispatch decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchDiagnostic {
    pub severity: SchedulerDispatchDiagnosticSeverity,
    pub code: SchedulerDispatchDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerDispatchDiagnostic {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("dispatch_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("dispatch_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Scheduler-selected dispatch decision for one ready task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDispatchDecision {
    #[serde(default = "default_scheduler_dispatch_decision_contract_version")]
    pub contract_version: u16,
    pub workflow_id: crate::SchedulerWorkflowId,
    pub workflow_run_id: crate::SchedulerWorkflowRunId,
    pub node_id: crate::SchedulerNodeId,
    pub task_id: crate::SchedulerTaskId,
    pub task_intent: SchedulableTaskIntent,
    pub selected_runtime_id: RuntimeIntentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_runtime_variant_id: Option<SchedulerRuntimeVariantId>,
    pub selected_device_ids: Vec<DeviceIntentId>,
    pub selected_model_ref: PumasModelRef,
    pub readiness_proof: SchedulerDependencyReadinessProof,
    pub environment_ref: DependencyEnvironmentRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batching_group_id: Option<SchedulerBatchingGroupId>,
    pub reservation_lease_id: SchedulerReservationLeaseId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_trait_settings: Vec<SchedulerTraitSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerDispatchDiagnostic>,
}

impl SchedulerDispatchDecision {
    /// Validates this raw dispatch decision before runtime host handoff.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        self.selected_model_ref
            .validate()
            .map_err(map_dependency_error)?;
        self.readiness_proof
            .validate_for_intent(&self.task_intent)?;
        self.environment_ref
            .validate()
            .map_err(map_dependency_error)?;
        validate_correlation(self)?;
        validate_selected_runtime_and_devices(self)?;
        validate_selected_model_ref(self)?;
        validate_environment_ref(self)?;
        validate_runtime_trait_settings(self)?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        Ok(())
    }
}

/// Validated dispatch decision for scheduler host handoff consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerDispatchDecision(SchedulerDispatchDecision);

impl ValidatedSchedulerDispatchDecision {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerDispatchDecision {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerDispatchDecision {
        self.0
    }
}

impl TryFrom<SchedulerDispatchDecision> for ValidatedSchedulerDispatchDecision {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerDispatchDecision) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn validate_correlation(
    decision: &SchedulerDispatchDecision,
) -> Result<(), SchedulerContractError> {
    if decision.workflow_id != decision.task_intent.workflow_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "dispatch decision workflow id must match task intent",
        });
    }
    if decision.workflow_run_id != decision.task_intent.workflow_run_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "dispatch decision workflow run id must match task intent",
        });
    }
    if decision.node_id != decision.task_intent.node_id {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "dispatch decision node id must match task intent",
        });
    }
    if decision.task_id != decision.task_intent.task_id {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "dispatch decision task id must match task intent",
        });
    }
    Ok(())
}

fn validate_selected_runtime_and_devices(
    decision: &SchedulerDispatchDecision,
) -> Result<(), SchedulerContractError> {
    if let Some(requested_runtime_id) = &decision.task_intent.constraints.requested_runtime_id {
        if requested_runtime_id != &decision.selected_runtime_id {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_runtime_id",
                reason: "selected runtime must satisfy the task intent runtime requirement",
            });
        }
    }
    if decision.selected_device_ids.is_empty() {
        return Err(SchedulerContractError::MissingField {
            field: "selected_device_ids",
        });
    }
    let mut seen = BTreeSet::new();
    for device_id in &decision.selected_device_ids {
        if !seen.insert(device_id) {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_device_ids",
                reason: "selected device ids must not contain duplicates",
            });
        }
    }
    if let Some(requested_device_id) = &decision.task_intent.constraints.requested_device_id {
        if !seen.contains(requested_device_id) {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_device_ids",
                reason: "selected devices must satisfy the task intent device requirement",
            });
        }
    }
    Ok(())
}

fn validate_selected_model_ref(
    decision: &SchedulerDispatchDecision,
) -> Result<(), SchedulerContractError> {
    if decision.selected_model_ref.model_id != decision.task_intent.model_ref.model_id {
        return Err(SchedulerContractError::InvalidField {
            field: "selected_model_ref.model_id",
            reason: "selected model id must match task intent model id",
        });
    }
    if let Some(requested_artifact_id) = &decision.task_intent.model_ref.selected_artifact_id {
        if Some(requested_artifact_id) != decision.selected_model_ref.selected_artifact_id.as_ref()
        {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_model_ref.selected_artifact_id",
                reason: "selected artifact id must satisfy task intent artifact requirement",
            });
        }
    }
    if let Some(requested_artifact_path) = &decision.task_intent.model_ref.selected_artifact_path {
        if Some(requested_artifact_path)
            != decision.selected_model_ref.selected_artifact_path.as_ref()
        {
            return Err(SchedulerContractError::InvalidField {
                field: "selected_model_ref.selected_artifact_path",
                reason: "selected artifact path must satisfy task intent artifact requirement",
            });
        }
    }
    Ok(())
}

fn validate_environment_ref(
    decision: &SchedulerDispatchDecision,
) -> Result<(), SchedulerContractError> {
    let Some(proof_environment_ref) = &decision.readiness_proof.preflight_result.environment_ref
    else {
        return Err(SchedulerContractError::MissingField {
            field: "readiness_proof.preflight_result.environment_ref",
        });
    };
    if proof_environment_ref != &decision.environment_ref {
        return Err(SchedulerContractError::InvalidField {
            field: "environment_ref",
            reason: "dispatch decision environment ref must match readiness proof",
        });
    }
    Ok(())
}

fn validate_runtime_trait_settings(
    decision: &SchedulerDispatchDecision,
) -> Result<(), SchedulerContractError> {
    for trait_setting in &decision.runtime_trait_settings {
        trait_setting
            .value
            .validate_for_capability_hint()
            .map_err(|_| SchedulerContractError::InvalidField {
                field: "runtime_trait_settings",
                reason: "runtime trait setting is invalid",
            })?;
    }
    Ok(())
}

fn default_scheduler_dispatch_decision_contract_version() -> u16 {
    SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler dispatch decision contract version",
        })
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<String, SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(SchedulerContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}

fn map_dependency_error(error: DependencyPlanningContractError) -> SchedulerContractError {
    match error {
        DependencyPlanningContractError::MissingField { field } => {
            SchedulerContractError::MissingField { field }
        }
        DependencyPlanningContractError::FieldTooLong { field, max_len } => {
            SchedulerContractError::FieldTooLong { field, max_len }
        }
        DependencyPlanningContractError::InvalidIdentifier { field } => {
            SchedulerContractError::InvalidIdentifier { field }
        }
        DependencyPlanningContractError::InvalidText { field } => {
            SchedulerContractError::InvalidText { field }
        }
        DependencyPlanningContractError::InvalidField { field, reason } => {
            SchedulerContractError::InvalidField { field, reason }
        }
        _ => SchedulerContractError::InvalidField {
            field: "dependency_planning",
            reason: "dependency planning contract value is invalid",
        },
    }
}
