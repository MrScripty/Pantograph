use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::request::DependencyBindingId;
use crate::result::DependencyPlanningDiagnostic;

use super::provider_source::{
    validate_provider_source_alternatives, DependencyProviderSourceAlternative,
};
use super::scalar::{
    validate_dependency_name, validate_dependency_text, validate_diagnostics,
    validate_optional_dependency_text, validate_validation_field_path, DependencyBindingProfileId,
    DependencyOperationTimestampMs, DependencyRequirementName, DependencyValidationFieldPath,
    DeviceObservationId, DeviceToolchainSourceId, HostPlatformSourceId, ManagedRuntimeSourceId,
    RuntimeFeatureSourceId, RuntimeSourceId, RuntimeVariantSourceId, SystemPackageManagerSourceId,
    SystemPackageSourceId,
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

/// Managed-runtime-specific requirement facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ManagedRuntimeRequirementDetails {
    pub managed_binary_id: ManagedRuntimeSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_key: Option<String>,
}

impl ManagedRuntimeRequirementDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "managed_runtime_requirement.version",
            self.version.as_deref(),
        )?;
        validate_optional_dependency_text(
            "managed_runtime_requirement.platform_key",
            self.platform_key.as_deref(),
        )
    }
}

/// Runtime-feature-specific requirement facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeFeatureRequirementDetails {
    pub runtime_id: RuntimeSourceId,
    pub feature_id: RuntimeFeatureSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
}

impl RuntimeFeatureRequirementDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        Ok(())
    }
}

/// Device-toolchain-specific requirement facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DeviceToolchainRequirementDetails {
    pub toolchain_id: DeviceToolchainSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceObservationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeSourceId>,
}

impl DeviceToolchainRequirementDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        Ok(())
    }
}

/// System-package-specific requirement facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SystemPackageRequirementDetails {
    pub package_id: SystemPackageSourceId,
    pub package_manager_id: SystemPackageManagerSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<HostPlatformSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_manager_version_constraint: Option<String>,
}

impl SystemPackageRequirementDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "system_package_requirement.architecture",
            self.architecture.as_deref(),
        )?;
        validate_optional_dependency_text(
            "system_package_requirement.package_manager_version_constraint",
            self.package_manager_version_constraint.as_deref(),
        )
    }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_runtime: Option<ManagedRuntimeRequirementDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_feature: Option<RuntimeFeatureRequirementDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_toolchain: Option<DeviceToolchainRequirementDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_package: Option<SystemPackageRequirementDetails>,
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
        if let Some(managed_runtime) = &self.managed_runtime {
            if self.kind != DependencyRequirementKind::RuntimeManagedBinary {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirement.managed_runtime",
                    reason: "managed runtime details are allowed only for runtime managed binary requirements",
                });
            }
            managed_runtime.validate()?;
        }
        if let Some(runtime_feature) = &self.runtime_feature {
            if self.kind != DependencyRequirementKind::RuntimeFeature {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirement.runtime_feature",
                    reason:
                        "runtime feature details are allowed only for runtime feature requirements",
                });
            }
            runtime_feature.validate()?;
        }
        if let Some(device_toolchain) = &self.device_toolchain {
            if self.kind != DependencyRequirementKind::DeviceToolchain {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirement.device_toolchain",
                    reason: "device toolchain details are allowed only for device toolchain requirements",
                });
            }
            device_toolchain.validate()?;
        }
        if let Some(system_package) = &self.system_package {
            if self.kind != DependencyRequirementKind::SystemPackage {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_requirement.system_package",
                    reason:
                        "system package details are allowed only for system package requirements",
                });
            }
            system_package.validate()?;
        }
        match self.kind {
            DependencyRequirementKind::PythonPackage => {}
            DependencyRequirementKind::RuntimeManagedBinary if self.managed_runtime.is_some() => {}
            DependencyRequirementKind::RuntimeFeature if self.runtime_feature.is_some() => {}
            DependencyRequirementKind::DeviceToolchain if self.device_toolchain.is_some() => {}
            DependencyRequirementKind::SystemPackage if self.system_package.is_some() => {}
            DependencyRequirementKind::RuntimeManagedBinary => {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "dependency_requirement.managed_runtime",
                });
            }
            DependencyRequirementKind::RuntimeFeature => {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "dependency_requirement.runtime_feature",
                });
            }
            DependencyRequirementKind::DeviceToolchain => {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "dependency_requirement.device_toolchain",
                });
            }
            DependencyRequirementKind::SystemPackage => {
                return Err(DependencyPlanningContractError::MissingField {
                    field: "dependency_requirement.system_package",
                });
            }
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

/// Managed-runtime-specific binding facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ManagedRuntimeBindingDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_binary_id: Option<ManagedRuntimeSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_key: Option<String>,
}

impl ManagedRuntimeBindingDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "managed_runtime_binding.selected_version",
            self.selected_version.as_deref(),
        )?;
        validate_optional_dependency_text(
            "managed_runtime_binding.platform_key",
            self.platform_key.as_deref(),
        )
    }
}

/// Runtime-feature-specific binding facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeFeatureBindingDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<RuntimeFeatureSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
}

impl RuntimeFeatureBindingDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        Ok(())
    }
}

/// Device-toolchain-specific binding facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DeviceToolchainBindingDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain_id: Option<DeviceToolchainSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceObservationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeSourceId>,
}

impl DeviceToolchainBindingDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        Ok(())
    }
}

/// System-package-specific binding facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SystemPackageBindingDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_id: Option<SystemPackageSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_manager_id: Option<SystemPackageManagerSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<HostPlatformSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub architecture: Option<String>,
}

impl SystemPackageBindingDetails {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_optional_dependency_text(
            "system_package_binding.architecture",
            self.architecture.as_deref(),
        )
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub managed_runtime: Option<ManagedRuntimeBindingDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_feature: Option<RuntimeFeatureBindingDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_toolchain: Option<DeviceToolchainBindingDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_package: Option<SystemPackageBindingDetails>,
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
        if let Some(managed_runtime) = &self.managed_runtime {
            if self.environment_kind != DependencyEnvironmentKind::ManagedBinary {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_binding.managed_runtime",
                    reason: "managed runtime details are allowed only for managed binary bindings",
                });
            }
            managed_runtime.validate()?;
        }
        if let Some(runtime_feature) = &self.runtime_feature {
            if self.environment_kind != DependencyEnvironmentKind::RuntimeFeature {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_binding.runtime_feature",
                    reason: "runtime feature details are allowed only for runtime feature bindings",
                });
            }
            runtime_feature.validate()?;
        }
        if let Some(device_toolchain) = &self.device_toolchain {
            if self.environment_kind != DependencyEnvironmentKind::DeviceToolchain {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_binding.device_toolchain",
                    reason:
                        "device toolchain details are allowed only for device toolchain bindings",
                });
            }
            device_toolchain.validate()?;
        }
        if let Some(system_package) = &self.system_package {
            if self.environment_kind != DependencyEnvironmentKind::SystemPackage {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "dependency_binding.system_package",
                    reason: "system package details are allowed only for system package bindings",
                });
            }
            system_package.validate()?;
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<DependencyProviderSourceAlternative>,
}

impl DependencyBindingStatusRow {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_diagnostics(&self.diagnostics)?;
        validate_provider_source_alternatives(&self.alternatives)
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
