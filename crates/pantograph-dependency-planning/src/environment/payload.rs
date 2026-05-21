use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::request::DependencyBindingId;
use crate::result::DependencyPlanningDiagnostic;

use super::scalar::{
    validate_dependency_name, validate_dependency_text, validate_diagnostics,
    validate_optional_dependency_text, validate_validation_field_path, DependencyBindingProfileId,
    DependencyOperationTimestampMs, DependencyRequirementName, DependencyValidationFieldPath,
};
use super::state::DependencyEnvironmentValidationState;

/// Requirement kind represented by the shared dependency-environment contract.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyRequirementKind {
    PythonPackage,
    RuntimeManagedBinary,
    SystemPackage,
    RuntimeFeature,
    DeviceToolchain,
}

/// Dependency environment class represented by a resolved binding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentKind {
    Python,
    ManagedBinary,
    SystemPackage,
    RuntimeFeature,
    DeviceToolchain,
}

/// Per-binding status state for check/install operations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyBindingStatusState {
    Unknown,
    Ready,
    Missing,
    Unavailable,
    Invalid,
    Installing,
    Installed,
    Failed,
    NotImplemented,
}

/// Dependency-environment operation state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentOperationState {
    NotStarted,
    Running,
    Succeeded,
    Failed,
    Blocked,
}

/// Typed validation code for dependency-environment validation errors.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyEnvironmentValidationCode {
    InvalidRequirement,
    InvalidBinding,
    InvalidEnvironment,
    MissingBinding,
    DuplicateBinding,
    InvalidTimestamp,
    UnsupportedFeature,
}

/// Python/package-manager-specific requirement facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PythonRequirementDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_requires: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<PythonPackageManagerKind>,
}

impl PythonRequirementDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "python_requirement.import_name",
            self.import_name.as_deref(),
        )?;
        validate_optional_dependency_text(
            "python_requirement.python_requires",
            self.python_requires.as_deref(),
        )?;
        Ok(())
    }
}

/// Python package-manager kind for Python-specific dependency detail structs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PythonPackageManagerKind {
    Pip,
    Uv,
    Conda,
}

/// Shared dependency requirement row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyRequirement {
    pub name: DependencyRequirementName,
    pub kind: DependencyRequirementKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_constraint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonRequirementDetails>,
}

impl DependencyRequirement {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_dependency_name("dependency_requirement.name", self.name.as_str())?;
        validate_optional_dependency_text(
            "dependency_requirement.version_constraint",
            self.version_constraint.as_deref(),
        )?;
        if let Some(python) = &self.python {
            if self.kind != DependencyRequirementKind::PythonPackage {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirement.python",
                    reason: "python details are allowed only for python package requirements",
                });
            }
            python.validate()?;
        }
        Ok(())
    }
}

/// Python/package-manager-specific binding facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PythonBindingDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python_executable_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub index_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_index_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<String>,
}

impl PythonBindingDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "python_binding.python_executable_override",
            self.python_executable_override.as_deref(),
        )?;
        validate_optional_dependency_text("python_binding.index_url", self.index_url.as_deref())?;
        for extra_index_url in &self.extra_index_urls {
            validate_dependency_text("python_binding.extra_index_urls", extra_index_url)?;
        }
        validate_optional_dependency_text("python_binding.marker", self.marker.as_deref())?;
        Ok(())
    }
}

/// Shared dependency binding row selected or checked by dependency environments.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyRequirementBinding {
    pub binding_id: DependencyBindingId,
    pub requirement_name: DependencyRequirementName,
    pub environment_kind: DependencyEnvironmentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<DependencyBindingProfileId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub python: Option<PythonBindingDetails>,
}

impl DependencyRequirementBinding {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if let Some(profile_id) = &self.profile_id {
            let _validated = DependencyBindingProfileId::parse(profile_id.as_str())?;
        }
        if let Some(python) = &self.python {
            if self.environment_kind != DependencyEnvironmentKind::Python {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_binding.python",
                    reason: "python details are allowed only for python environment bindings",
                });
            }
            python.validate()?;
        }
        Ok(())
    }
}

/// Dependency-environment operation timing and terminal state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyEnvironmentOperation {
    pub state: DependencyEnvironmentOperationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<DependencyOperationTimestampMs>,
}

impl DependencyEnvironmentOperation {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if let (Some(started), Some(completed)) = (self.started_at_ms, self.completed_at_ms) {
            if completed.get() < started.get() {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_operation.completed_at_ms",
                    reason:
                        "operation completion timestamp must not be earlier than start timestamp",
                });
            }
        }
        Ok(())
    }
}

/// Per-binding dependency check/install status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyBindingStatusRow {
    pub binding_id: DependencyBindingId,
    pub state: DependencyBindingStatusState,
    pub validation_state: DependencyEnvironmentValidationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyBindingStatusRow {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_diagnostics(&self.diagnostics)
    }
}

/// Typed validation error row for dependency-environment result payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyEnvironmentValidationError {
    pub code: DependencyEnvironmentValidationCode,
    pub field_path: DependencyValidationFieldPath,
    pub message: String,
}

impl DependencyEnvironmentValidationError {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_validation_field_path(
            "dependency_validation_error.field_path",
            self.field_path.as_str(),
        )?;
        validate_dependency_text("dependency_validation_error.message", &self.message)
    }
}
