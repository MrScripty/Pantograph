use std::fmt;
use std::str::FromStr;

use pantograph_dependency_planning::{
    DependencyOverridePatchV1, DependencyPlanningContractError, DependencyTaskId, DeviceIntentId,
    PumasModelRef, RuntimeIntentId,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SchedulerContractError;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;
const MAX_ESTIMATE_VALUE: u64 = i64::MAX as u64;

/// Current contract version for ready workflow node task intent.
pub const SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION: u16 = 1;

macro_rules! scheduler_id {
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

scheduler_id!(SchedulerWorkflowId, "workflow_id");
scheduler_id!(SchedulerWorkflowRunId, "workflow_run_id");
scheduler_id!(SchedulerNodeId, "node_id");
scheduler_id!(SchedulerTaskId, "task_id");
scheduler_id!(SchedulerFairnessKey, "fairness_key");
scheduler_id!(SchedulerTraitId, "trait_id");

/// Optional hard runtime/device requirements from graph intent.
///
/// Omitted values mean scheduler policy decides. Present values are hard
/// requirements that scheduler policy must either honor or reject with typed
/// diagnostics.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerRuntimeDeviceConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<DeviceIntentId>,
}

/// Typed optional trait setting supplied by graph intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTraitSetting {
    pub trait_id: SchedulerTraitId,
    pub value: SchedulerTraitValue,
}

impl SchedulerTraitSetting {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        self.value.validate()
    }
}

/// Value kinds allowed in scheduler task trait settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum SchedulerTraitValue {
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
}

impl SchedulerTraitValue {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        if let SchedulerTraitValue::String(value) = self {
            validate_text("trait_setting.value", value)?;
        }
        Ok(())
    }
}

/// Bounded scheduler estimate hint kind supplied before resource admission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerEstimateHintKind {
    InputPixels,
    OutputPixels,
    BatchSize,
    PeakRamBytes,
    PeakVramBytes,
    WarmupCostMs,
}

/// Bounded scheduler estimate hint supplied by graph/node planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerEstimateHint {
    pub kind: SchedulerEstimateHintKind,
    pub value: u64,
}

impl SchedulerEstimateHint {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.value == 0 {
            return Err(SchedulerContractError::InvalidField {
                field: "estimate_hint.value",
                reason: "estimate values must be greater than zero",
            });
        }
        if self.value > MAX_ESTIMATE_VALUE {
            return Err(SchedulerContractError::InvalidField {
                field: "estimate_hint.value",
                reason: "estimate values must fit signed ledger/resource arithmetic",
            });
        }
        Ok(())
    }
}

/// Path-free task intent for one ready workflow DAG node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulableTaskIntent {
    #[serde(default = "default_schedulable_task_intent_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fairness_key: Option<SchedulerFairnessKey>,
    pub task_type: DependencyTaskId,
    pub model_ref: PumasModelRef,
    #[serde(default)]
    pub constraints: SchedulerRuntimeDeviceConstraints,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub trait_settings: Vec<SchedulerTraitSetting>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependency_override_patches: Vec<DependencyOverridePatchV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimate_hints: Vec<SchedulerEstimateHint>,
}

impl SchedulableTaskIntent {
    /// Validates this raw boundary DTO before scheduler policy consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.contract_version != SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION {
            return Err(SchedulerContractError::InvalidField {
                field: "contract_version",
                reason: "unsupported schedulable task intent contract version",
            });
        }
        self.model_ref
            .validate()
            .map_err(map_dependency_planning_error)?;
        for trait_setting in &self.trait_settings {
            trait_setting.validate()?;
        }
        for override_patch in &self.dependency_override_patches {
            override_patch
                .validate()
                .map_err(map_dependency_planning_error)?;
        }
        for estimate_hint in &self.estimate_hints {
            estimate_hint.validate()?;
        }
        Ok(())
    }
}

fn default_schedulable_task_intent_contract_version() -> u16 {
    SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION
}

/// Validated schedulable task intent for internal scheduler policy.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulableTaskIntent(SchedulableTaskIntent);

impl ValidatedSchedulableTaskIntent {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulableTaskIntent {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulableTaskIntent {
        self.0
    }
}

impl TryFrom<SchedulableTaskIntent> for ValidatedSchedulableTaskIntent {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulableTaskIntent) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn map_dependency_planning_error(error: DependencyPlanningContractError) -> SchedulerContractError {
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
