use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{ModelArtifactKind, PumasModelRef};

const MAX_ID_LEN: usize = 128;
const MAX_CONTEXT_LEN: usize = 256;

macro_rules! validated_id {
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

validated_id!(DependencyTaskId, "task_id");
validated_id!(RuntimeIntentId, "runtime_id");
validated_id!(DeviceIntentId, "device_id");
validated_id!(DependencyBindingId, "dependency_binding_id");

/// Scheduler-facing intent supplied by a graph or caller.
///
/// These fields influence scheduler selection. They do not bypass scheduler
/// policy and do not authorize node-engine or frontend code to choose an
/// executable runtime/device directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SchedulerIntent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_runtime_id: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_device_id: Option<DeviceIntentId>,
}

/// Bounded caller context used for diagnostics and traceability.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyPlanningCallerContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

impl DependencyPlanningCallerContext {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_context("caller_context.workflow_id", self.workflow_id.as_deref())?;
        validate_optional_context("caller_context.node_id", self.node_id.as_deref())?;
        validate_optional_context("caller_context.port_id", self.port_id.as_deref())?;
        validate_optional_context("caller_context.run_id", self.run_id.as_deref())?;
        Ok(())
    }
}

/// Typed dependency planning request crossing graph, host, and scheduler seams.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyPlanningRequest {
    pub model_ref: PumasModelRef,
    pub task_id: DependencyTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_type: Option<DependencyTaskId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_artifact_kind: Option<ModelArtifactKind>,
    #[serde(default, skip_serializing_if = "SchedulerIntent::is_empty")]
    pub scheduler_intent: SchedulerIntent,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
    #[serde(
        default,
        skip_serializing_if = "DependencyPlanningCallerContext::is_empty"
    )]
    pub caller_context: DependencyPlanningCallerContext,
}

impl DependencyPlanningRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        self.model_ref.validate()?;
        self.caller_context.validate()?;
        Ok(())
    }
}

impl SchedulerIntent {
    fn is_empty(&self) -> bool {
        self.requested_runtime_id.is_none() && self.requested_device_id.is_none()
    }
}

impl DependencyPlanningCallerContext {
    fn is_empty(&self) -> bool {
        self.workflow_id.is_none()
            && self.node_id.is_none()
            && self.port_id.is_none()
            && self.run_id.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyPlanningRequest(DependencyPlanningRequest);

impl ValidatedDependencyPlanningRequest {
    pub fn into_inner(self) -> DependencyPlanningRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyPlanningRequest {
        &self.0
    }
}

impl TryFrom<DependencyPlanningRequest> for ValidatedDependencyPlanningRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyPlanningRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyPlanningRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let request: DependencyPlanningRequest = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_planning_request",
                reason: "request JSON did not match dependency planning contract",
            }
        })?;
        Self::try_from(request)
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_optional_context(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), DependencyPlanningContractError> {
    let Some(value) = value else {
        return Ok(());
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_CONTEXT_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_CONTEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DependencyPlanningContractError::InvalidText { field });
    }
    Ok(())
}
