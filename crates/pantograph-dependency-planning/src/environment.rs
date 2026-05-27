use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::preflight::DependencyPlanningIdentityKey;
use crate::request::{DependencyBindingId, DependencyPlanningRequest, DependencyRequirementsId};
use crate::result::DependencyPlanningDiagnostic;

const MAX_ENVIRONMENT_ID_LEN: usize = 128;

mod payload;
mod scalar;
mod state;

pub use payload::{
    DependencyBindingStatusRow, DependencyBindingStatusState, DependencyEnvironmentKind,
    DependencyEnvironmentOperation, DependencyEnvironmentOperationState,
    DependencyEnvironmentValidationCode, DependencyEnvironmentValidationError,
    DependencyRequirement, DependencyRequirementBinding, DependencyRequirementKind,
    PythonBindingDetails, PythonPackageManagerKind, PythonRequirementDetails,
};
use scalar::{validate_diagnostics, validate_unique_binding_ids};
pub use scalar::{
    DependencyBindingProfileId, DependencyOperationTimestampMs, DependencyRequirementName,
    DependencyValidationFieldPath,
};
pub use state::{
    DependencyEnvironmentAction, DependencyEnvironmentFailureState,
    DependencyEnvironmentInstallState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentValidationState,
};

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
        self.identity_key
            .validate_matches_planning_request(&self.planning_request)?;
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
    pub requirements: Vec<DependencyRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<DependencyRequirementBinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selected_binding_ids: Vec<DependencyBindingId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binding_statuses: Vec<DependencyBindingStatusRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<DependencyEnvironmentOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<DependencyEnvironmentValidationError>,
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
        validate_unique_binding_ids(&self.selected_binding_ids)?;
        for requirement in &self.requirements {
            requirement.validate()?;
        }
        for binding in &self.bindings {
            binding.validate()?;
        }
        for status in &self.binding_statuses {
            status.validate()?;
        }
        if let Some(operation) = &self.operation {
            operation.validate()?;
        }
        for error in &self.validation_errors {
            error.validate()?;
        }
        validate_diagnostics(&self.diagnostics)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyEnvironmentResult(DependencyEnvironmentResult);

impl ValidatedDependencyEnvironmentResult {
    pub fn into_inner(self) -> DependencyEnvironmentResult {
        self.0
    }

    pub fn as_result(&self) -> &DependencyEnvironmentResult {
        &self.0
    }
}

impl TryFrom<DependencyEnvironmentResult> for ValidatedDependencyEnvironmentResult {
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyEnvironmentResult) -> Result<Self, Self::Error> {
        value.validate()?;
        validate_environment_result_semantics(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyEnvironmentResult {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        reject_path_shaped_result_fields(&value)?;
        let result: DependencyEnvironmentResult = serde_json::from_value(value).map_err(|_| {
            DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result",
                reason: "result JSON did not match dependency environment contract",
            }
        })?;
        Self::try_from(result)
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

fn reject_path_shaped_result_fields(
    value: &serde_json::Value,
) -> Result<(), DependencyPlanningContractError> {
    if contains_path_shaped_fields(value) {
        Err(DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_result",
            reason: "result must not contain path-shaped dependency identity fields",
        })
    } else {
        Ok(())
    }
}

fn contains_path_shaped_fields(value: &serde_json::Value) -> bool {
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
            ) || contains_path_shaped_fields(child)
        }),
        serde_json::Value::Array(items) => items.iter().any(contains_path_shaped_fields),
        _ => false,
    }
}

fn validate_environment_result_semantics(
    result: &DependencyEnvironmentResult,
) -> Result<(), DependencyPlanningContractError> {
    if result.readiness_state == DependencyEnvironmentReadinessState::Ready {
        if result.dependency_requirements_id.is_none() {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_requirements_id",
            });
        }
        if result.environment_ref.is_none() {
            return Err(DependencyPlanningContractError::MissingField {
                field: "environment_ref",
            });
        }
        if result.validation_state != DependencyEnvironmentValidationState::Valid {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.validation_state",
                reason: "ready dependency environment results must be valid",
            });
        }
        if result.install_state != DependencyEnvironmentInstallState::Installed {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.install_state",
                reason: "ready dependency environment results must be installed",
            });
        }
        if let Some(operation) = &result.operation {
            if operation.state != DependencyEnvironmentOperationState::Succeeded {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_environment_result.operation.state",
                    reason: "ready dependency environment operations must be succeeded",
                });
            }
        }
    }

    if result.validation_state == DependencyEnvironmentValidationState::Invalid
        && result.validation_errors.is_empty()
        && result.diagnostics.is_empty()
    {
        return Err(DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_result.validation_errors",
            reason:
                "invalid dependency environment results require validation errors or diagnostics",
        });
    }

    if result.readiness_state == DependencyEnvironmentReadinessState::NotImplemented
        || result.install_state == DependencyEnvironmentInstallState::NotImplemented
        || result.validation_state == DependencyEnvironmentValidationState::NotImplemented
    {
        if result.failure_state != Some(DependencyEnvironmentFailureState::NotImplemented) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.failure_state",
                reason: "not-implemented dependency environment results require not_implemented failure state",
            });
        }
        if result.diagnostics.is_empty() {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.diagnostics",
                reason: "not-implemented dependency environment results require diagnostics",
            });
        }
    }

    Ok(())
}
