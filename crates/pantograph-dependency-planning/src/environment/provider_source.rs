use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::result::DependencyPlanningDiagnostic;

use super::observation::DependencyInventoryObservationFreshness;
use super::scalar::{
    validate_diagnostics, validate_optional_dependency_text, DependencyOperationTimestampMs,
    DeviceClassSourceId, DeviceObservationId, DeviceToolchainSourceId, RuntimeFeatureSourceId,
    RuntimeSourceId, RuntimeVariantSourceId,
};

const PROVIDER_SOURCE_CONTRACT_VERSION: u32 = 1;
const MAX_PROVIDER_ALTERNATIVES: usize = 8;

pub const RUNTIME_FEATURE_STREAMING: &str = "streaming";
pub const RUNTIME_FEATURE_DEVICE_SELECTION: &str = "device_selection";
pub const RUNTIME_FEATURE_EXTERNAL_CONNECTION: &str = "external_connection";
pub const RUNTIME_FEATURE_KV_CACHE: &str = "kv_cache";
pub const RUNTIME_FEATURE_CUSTOM_CODE: &str = "custom_code";
pub const RUNTIME_FEATURE_PREPROCESSING: &str = "preprocessing";
pub const RUNTIME_FEATURE_POSTPROCESSING: &str = "postprocessing";
pub const RUNTIME_FEATURE_REQUEST_LIFECYCLE: &str = "request_lifecycle";

pub const DEVICE_TOOLCHAIN_CUDA_RUNTIME: &str = "cuda_runtime";
pub const DEVICE_TOOLCHAIN_METAL_RUNTIME: &str = "metal_runtime";
pub const DEVICE_TOOLCHAIN_MPS_RUNTIME: &str = "mps_runtime";
pub const DEVICE_TOOLCHAIN_LLAMACPP_DEVICE_INVENTORY: &str = "llamacpp_device_inventory";
pub const DEVICE_TOOLCHAIN_PYTORCH_DEVICE_PROBE: &str = "pytorch_device_probe";

pub const DEVICE_CLASS_CPU: &str = "cpu";
pub const DEVICE_CLASS_CUDA: &str = "cuda";
pub const DEVICE_CLASS_METAL: &str = "metal";
pub const DEVICE_CLASS_MPS: &str = "mps";

/// Source-owned readiness state for provider snapshot rows.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyProviderSourceState {
    Ready,
    Missing,
    Unavailable,
    Unsupported,
    Unknown,
    Probing,
    Degraded,
    Stale,
    Failed,
}

/// Bounded alternative fact for unavailable explicit provider constraints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyProviderSourceAlternative {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_id: Option<RuntimeFeatureSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toolchain_id: Option<DeviceToolchainSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<DeviceClassSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceObservationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl DependencyProviderSourceAlternative {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.runtime_id.is_none()
            && self.runtime_variant_id.is_none()
            && self.feature_id.is_none()
            && self.toolchain_id.is_none()
            && self.device_class.is_none()
            && self.device_id.is_none()
        {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_provider_source_alternative",
            });
        }
        if let Some(feature_id) = &self.feature_id {
            validate_runtime_feature_id(feature_id)?;
        }
        if let Some(toolchain_id) = &self.toolchain_id {
            validate_device_toolchain_id(toolchain_id)?;
        }
        if let Some(device_class) = &self.device_class {
            validate_device_class(device_class)?;
        }
        validate_optional_dependency_text(
            "dependency_provider_source_alternative.reason",
            self.reason.as_deref(),
        )
    }
}

/// One source-owned runtime-feature fact row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeFeatureProviderSourceRow {
    pub runtime_id: RuntimeSourceId,
    pub feature_id: RuntimeFeatureSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<RuntimeVariantSourceId>,
    pub state: DependencyProviderSourceState,
    pub freshness: DependencyInventoryObservationFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<DependencyProviderSourceAlternative>,
}

impl RuntimeFeatureProviderSourceRow {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_runtime_feature_id(&self.feature_id)?;
        validate_source_row_state(
            "runtime_feature_provider_source.diagnostics",
            self.state,
            self.freshness,
            &self.diagnostics,
        )?;
        validate_provider_source_alternatives(&self.alternatives)
    }

    fn key(&self) -> (String, String, Option<String>) {
        (
            self.runtime_id.as_str().to_string(),
            self.feature_id.as_str().to_string(),
            self.runtime_variant_id
                .as_ref()
                .map(|runtime_variant_id| runtime_variant_id.as_str().to_string()),
        )
    }
}

/// Runtime-feature provider source snapshot consumed by inventory providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeFeatureProviderSourceSnapshot {
    #[serde(default = "default_provider_source_contract_version")]
    pub contract_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<RuntimeFeatureProviderSourceRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl RuntimeFeatureProviderSourceSnapshot {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(self.contract_version, "runtime_feature_provider_source")?;
        validate_diagnostics(&self.diagnostics)?;
        let mut keys = BTreeSet::new();
        for row in &self.rows {
            row.validate()?;
            if !keys.insert(row.key()) {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "runtime_feature_provider_source.rows",
                    reason: "runtime feature source rows must be unique by runtime, feature, and variant",
                });
            }
        }
        Ok(())
    }
}

/// One source-owned device-toolchain fact row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DeviceToolchainProviderSourceRow {
    pub toolchain_id: DeviceToolchainSourceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<DeviceClassSourceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceObservationId>,
    pub state: DependencyProviderSourceState,
    pub freshness: DependencyInventoryObservationFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<DependencyProviderSourceAlternative>,
}

impl DeviceToolchainProviderSourceRow {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_device_toolchain_id(&self.toolchain_id)?;
        if let Some(device_class) = &self.device_class {
            validate_device_class(device_class)?;
        }
        validate_source_row_state(
            "device_toolchain_provider_source.diagnostics",
            self.state,
            self.freshness,
            &self.diagnostics,
        )?;
        validate_provider_source_alternatives(&self.alternatives)
    }

    fn key(&self) -> (String, Option<String>, Option<String>, Option<String>) {
        (
            self.toolchain_id.as_str().to_string(),
            self.runtime_id
                .as_ref()
                .map(|runtime_id| runtime_id.as_str().to_string()),
            self.device_class
                .as_ref()
                .map(|device_class| device_class.as_str().to_string()),
            self.device_id
                .as_ref()
                .map(|device_id| device_id.as_str().to_string()),
        )
    }
}

/// Device-toolchain provider source snapshot consumed by inventory providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DeviceToolchainProviderSourceSnapshot {
    #[serde(default = "default_provider_source_contract_version")]
    pub contract_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<DeviceToolchainProviderSourceRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DeviceToolchainProviderSourceSnapshot {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_contract_version(self.contract_version, "device_toolchain_provider_source")?;
        validate_diagnostics(&self.diagnostics)?;
        let mut keys = BTreeSet::new();
        for row in &self.rows {
            row.validate()?;
            if !keys.insert(row.key()) {
                return Err(DependencyPlanningContractError::InvalidField {
                    field: "device_toolchain_provider_source.rows",
                    reason:
                        "device toolchain source rows must be unique by toolchain, runtime, device class, and device id",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedRuntimeFeatureProviderSourceSnapshot(RuntimeFeatureProviderSourceSnapshot);

impl ValidatedRuntimeFeatureProviderSourceSnapshot {
    pub fn into_inner(self) -> RuntimeFeatureProviderSourceSnapshot {
        self.0
    }

    pub fn as_snapshot(&self) -> &RuntimeFeatureProviderSourceSnapshot {
        &self.0
    }
}

impl TryFrom<RuntimeFeatureProviderSourceSnapshot>
    for ValidatedRuntimeFeatureProviderSourceSnapshot
{
    type Error = DependencyPlanningContractError;

    fn try_from(value: RuntimeFeatureProviderSourceSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedRuntimeFeatureProviderSourceSnapshot {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let snapshot: RuntimeFeatureProviderSourceSnapshot = serde_json::from_value(value)
            .map_err(|_| DependencyPlanningContractError::InvalidField {
                field: "runtime_feature_provider_source",
                reason: "source JSON did not match runtime feature provider source contract",
            })?;
        Self::try_from(snapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDeviceToolchainProviderSourceSnapshot(DeviceToolchainProviderSourceSnapshot);

impl ValidatedDeviceToolchainProviderSourceSnapshot {
    pub fn into_inner(self) -> DeviceToolchainProviderSourceSnapshot {
        self.0
    }

    pub fn as_snapshot(&self) -> &DeviceToolchainProviderSourceSnapshot {
        &self.0
    }
}

impl TryFrom<DeviceToolchainProviderSourceSnapshot>
    for ValidatedDeviceToolchainProviderSourceSnapshot
{
    type Error = DependencyPlanningContractError;

    fn try_from(value: DeviceToolchainProviderSourceSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDeviceToolchainProviderSourceSnapshot {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let snapshot: DeviceToolchainProviderSourceSnapshot = serde_json::from_value(value)
            .map_err(|_| DependencyPlanningContractError::InvalidField {
                field: "device_toolchain_provider_source",
                reason: "source JSON did not match device toolchain provider source contract",
            })?;
        Self::try_from(snapshot)
    }
}

fn default_provider_source_contract_version() -> u32 {
    PROVIDER_SOURCE_CONTRACT_VERSION
}

fn validate_contract_version(
    contract_version: u32,
    field: &'static str,
) -> Result<(), DependencyPlanningContractError> {
    if contract_version == PROVIDER_SOURCE_CONTRACT_VERSION {
        return Ok(());
    }
    Err(DependencyPlanningContractError::InvalidField {
        field,
        reason: "only dependency provider source contract version 1 is supported",
    })
}

fn validate_source_row_state(
    diagnostics_field: &'static str,
    state: DependencyProviderSourceState,
    freshness: DependencyInventoryObservationFreshness,
    diagnostics: &[DependencyPlanningDiagnostic],
) -> Result<(), DependencyPlanningContractError> {
    if (state == DependencyProviderSourceState::Stale
        || freshness == DependencyInventoryObservationFreshness::Stale)
        && diagnostics.is_empty()
    {
        return Err(DependencyPlanningContractError::MissingField {
            field: diagnostics_field,
        });
    }
    validate_diagnostics(diagnostics)
}

pub(super) fn validate_provider_source_alternatives(
    alternatives: &[DependencyProviderSourceAlternative],
) -> Result<(), DependencyPlanningContractError> {
    if alternatives.len() > MAX_PROVIDER_ALTERNATIVES {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field: "dependency_provider_source_alternatives",
            max_len: MAX_PROVIDER_ALTERNATIVES,
        });
    }
    for alternative in alternatives {
        alternative.validate()?;
    }
    Ok(())
}

fn validate_runtime_feature_id(
    feature_id: &RuntimeFeatureSourceId,
) -> Result<(), DependencyPlanningContractError> {
    if known_runtime_feature_ids().contains(&feature_id.as_str()) {
        return Ok(());
    }
    Err(DependencyPlanningContractError::InvalidField {
        field: "runtime_feature.source_id",
        reason: "runtime feature source id is not in the canonical provider-source vocabulary",
    })
}

fn validate_device_toolchain_id(
    toolchain_id: &DeviceToolchainSourceId,
) -> Result<(), DependencyPlanningContractError> {
    if known_device_toolchain_ids().contains(&toolchain_id.as_str()) {
        return Ok(());
    }
    Err(DependencyPlanningContractError::InvalidField {
        field: "device_toolchain.source_id",
        reason: "device toolchain source id is not in the canonical provider-source vocabulary",
    })
}

fn validate_device_class(
    device_class: &DeviceClassSourceId,
) -> Result<(), DependencyPlanningContractError> {
    if known_device_classes().contains(&device_class.as_str()) {
        return Ok(());
    }
    Err(DependencyPlanningContractError::InvalidField {
        field: "device_class.source_id",
        reason: "device class source id is not in the canonical provider-source vocabulary",
    })
}

pub fn known_runtime_feature_ids() -> &'static [&'static str] {
    &[
        RUNTIME_FEATURE_STREAMING,
        RUNTIME_FEATURE_DEVICE_SELECTION,
        RUNTIME_FEATURE_EXTERNAL_CONNECTION,
        RUNTIME_FEATURE_KV_CACHE,
        RUNTIME_FEATURE_CUSTOM_CODE,
        RUNTIME_FEATURE_PREPROCESSING,
        RUNTIME_FEATURE_POSTPROCESSING,
        RUNTIME_FEATURE_REQUEST_LIFECYCLE,
    ]
}

pub fn known_device_toolchain_ids() -> &'static [&'static str] {
    &[
        DEVICE_TOOLCHAIN_CUDA_RUNTIME,
        DEVICE_TOOLCHAIN_METAL_RUNTIME,
        DEVICE_TOOLCHAIN_MPS_RUNTIME,
        DEVICE_TOOLCHAIN_LLAMACPP_DEVICE_INVENTORY,
        DEVICE_TOOLCHAIN_PYTORCH_DEVICE_PROBE,
    ]
}

pub fn known_device_classes() -> &'static [&'static str] {
    &[
        DEVICE_CLASS_CPU,
        DEVICE_CLASS_CUDA,
        DEVICE_CLASS_METAL,
        DEVICE_CLASS_MPS,
    ]
}
