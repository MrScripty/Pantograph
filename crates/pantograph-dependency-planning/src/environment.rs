use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::preflight::DependencyPlanningIdentityKey;
use crate::request::{
    DependencyPlanningRequest, DependencyRequirementsId, DeviceIntentId, RuntimeIntentId,
};
use crate::result::DependencyPlanningDiagnostic;

const MAX_ENVIRONMENT_ID_LEN: usize = 128;

macro_rules! environment_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
                validate_environment_identifier($field, value.as_ref()).map(Self)
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

environment_id!(DependencyEnvironmentId, "dependency_environment_id");
environment_id!(
    DependencyEnvironmentManifestId,
    "dependency_environment_manifest_id"
);

/// Typed dependency-environment action requested by graph or frontend callers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentAction {
    Resolve,
    Check,
    Install,
}

/// Dependency-environment readiness state reported after resolve/check/install.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentReadinessState {
    Unknown,
    Resolved,
    Ready,
    Missing,
    Unavailable,
    Invalid,
    Failed,
    NotImplemented,
}

/// Dependency-environment install state reported by host dependency actions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentInstallState {
    NotRequested,
    NotInstalled,
    Installing,
    Installed,
    Failed,
    Blocked,
    NotImplemented,
}

/// Validation state for dependency-environment contracts and resolved facts.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentValidationState {
    Valid,
    Invalid,
    Stale,
    Unavailable,
    NotImplemented,
}

/// High-level failure state for dependency-environment results.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentFailureState {
    InvalidRequest,
    RequirementsUnavailable,
    EnvironmentUnavailable,
    CheckFailed,
    InstallFailed,
    NotImplemented,
    InternalError,
}

/// Stable environment reference returned by dependency-environment operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyEnvironmentRef {
    pub environment_id: DependencyEnvironmentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<DependencyEnvironmentManifestId>,
}

impl DependencyEnvironmentRef {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if let Some(manifest_id) = &self.manifest_id {
            validate_environment_identifier(
                "dependency_environment_manifest_id",
                manifest_id.as_str(),
            )?;
        }
        Ok(())
    }
}

/// Typed request for dependency-environment resolve/check/install operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyEnvironmentRequest {
    #[serde(default = "default_dependency_environment_contract_version")]
    pub contract_version: u32,
    pub action: DependencyEnvironmentAction,
    pub identity_key: DependencyPlanningIdentityKey,
    pub planning_request: DependencyPlanningRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_ref: Option<DependencyEnvironmentRef>,
}

impl DependencyEnvironmentRequest {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.contract_version != 1 {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_request.contract_version",
                reason: "only dependency environment contract version 1 is supported",
            });
        }
        self.identity_key.validate()?;
        self.planning_request.validate()?;
        if self
            .planning_request
            .model_ref
            .selected_artifact_path
            .is_some()
        {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "planning_request.model_ref.selected_artifact_path",
                reason:
                    "dependency environment request identity must not carry selected artifact paths",
            });
        }
        if let Some(environment_ref) = &self.environment_ref {
            environment_ref.validate()?;
        }
        self.validate_identity_matches_planning_request()?;
        if matches!(
            self.action,
            DependencyEnvironmentAction::Check | DependencyEnvironmentAction::Install
        ) && self.dependency_requirements_id.is_none()
        {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_requirements_id",
            });
        }
        Ok(())
    }

    fn validate_identity_matches_planning_request(
        &self,
    ) -> Result<(), DependencyPlanningContractError> {
        let identity = &self.identity_key;
        let request = &self.planning_request;

        if identity.model_ref != request.model_ref {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.model_ref",
                reason: "identity key model ref must match planning request model ref",
            });
        }
        if identity.task_id != request.task_id {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.task_id",
                reason: "identity key task id must match planning request task id",
            });
        }
        if identity.task_type != request.task_type {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.task_type",
                reason: "identity key task type must match planning request task type",
            });
        }
        if identity.expected_artifact_kind != request.expected_artifact_kind {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.expected_artifact_kind",
                reason: "identity key artifact kind must match planning request artifact kind",
            });
        }
        if identity.platform_context != request.platform_context {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.platform_context",
                reason:
                    "identity key platform context must match planning request platform context",
            });
        }
        if identity.selected_binding_ids != request.selected_binding_ids {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "identity_key.selected_binding_ids",
                reason:
                    "identity key selected bindings must match planning request selected bindings",
            });
        }
        validate_optional_runtime_match(
            "identity_key.selected_runtime_id",
            identity.selected_runtime_id.as_ref(),
            request.scheduler_intent.requested_runtime_id.as_ref(),
        )?;
        validate_optional_device_match(
            "identity_key.selected_device_id",
            identity.selected_device_id.as_ref(),
            request.scheduler_intent.requested_device_id.as_ref(),
        )?;
        Ok(())
    }
}

/// Typed result for dependency-environment resolve/check/install operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyEnvironmentResult {
    #[serde(default = "default_dependency_environment_contract_version")]
    pub contract_version: u32,
    pub action: DependencyEnvironmentAction,
    pub identity_key: DependencyPlanningIdentityKey,
    pub readiness_state: DependencyEnvironmentReadinessState,
    pub install_state: DependencyEnvironmentInstallState,
    pub validation_state: DependencyEnvironmentValidationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_state: Option<DependencyEnvironmentFailureState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_ref: Option<DependencyEnvironmentRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyEnvironmentResult {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.contract_version != 1 {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.contract_version",
                reason: "only dependency environment contract version 1 is supported",
            });
        }
        self.identity_key.validate()?;
        if let Some(environment_ref) = &self.environment_ref {
            environment_ref.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyEnvironmentRequest(DependencyEnvironmentRequest);

impl ValidatedDependencyEnvironmentRequest {
    pub fn into_inner(self) -> DependencyEnvironmentRequest {
        self.0
    }

    pub fn as_request(&self) -> &DependencyEnvironmentRequest {
        &self.0
    }
}

impl TryFrom<DependencyEnvironmentRequest> for ValidatedDependencyEnvironmentRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyEnvironmentRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyEnvironmentRequest {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_path_shaped_request_fields(&value)?;
        let request: DependencyEnvironmentRequest =
            serde_json::from_value(value).map_err(|_| {
                DependencyPlanningContractError::InvalidField {
                    field: "dependency_environment_request",
                    reason: "request JSON did not match dependency environment contract",
                }
            })?;
        Self::try_from(request)
    }
}

fn default_dependency_environment_contract_version() -> u32 {
    1
}

fn validate_environment_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ENVIRONMENT_ID_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_ENVIRONMENT_ID_LEN,
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

fn validate_optional_runtime_match(
    field: &'static str,
    selected: Option<&RuntimeIntentId>,
    requested: Option<&RuntimeIntentId>,
) -> Result<(), DependencyPlanningContractError> {
    let (Some(selected), Some(requested)) = (selected, requested) else {
        return Ok(());
    };
    if selected == requested {
        Ok(())
    } else {
        Err(DependencyPlanningContractError::InvalidField {
            field,
            reason: "selected runtime must match requested runtime when both are present",
        })
    }
}

fn validate_optional_device_match(
    field: &'static str,
    selected: Option<&DeviceIntentId>,
    requested: Option<&DeviceIntentId>,
) -> Result<(), DependencyPlanningContractError> {
    let (Some(selected), Some(requested)) = (selected, requested) else {
        return Ok(());
    };
    if selected == requested {
        Ok(())
    } else {
        Err(DependencyPlanningContractError::InvalidField {
            field,
            reason: "selected device must match requested device when both are present",
        })
    }
}

fn reject_path_shaped_request_fields(
    value: &serde_json::Value,
) -> Result<(), DependencyPlanningContractError> {
    fn visit(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Object(object) => object.iter().any(|(key, child)| {
                matches!(
                    key.as_str(),
                    "model_path"
                        | "modelPath"
                        | "entry_path"
                        | "entryPath"
                        | "selected_artifact_path"
                        | "selectedArtifactPath"
                        | "local_load_path"
                        | "localLoadPath"
                ) || visit(child)
            }),
            serde_json::Value::Array(items) => items.iter().any(visit),
            _ => false,
        }
    }

    if visit(value) {
        Err(DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_request",
            reason: "request must not contain path-shaped dependency identity fields",
        })
    } else {
        Ok(())
    }
}
