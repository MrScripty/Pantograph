use std::collections::BTreeSet;

use pantograph_dependency_planning::{
    DependencyPlanningContractError, DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::dispatch::{SchedulerBatchingGroupId, SchedulerReservationLeaseId};
use crate::error::SchedulerContractError;
use crate::intent::{SchedulerTaskId, SchedulerWorkflowRunId};
use crate::resource_types::{
    SchedulerModelResidencyState, SchedulerResourceDiagnostic, SchedulerResourceFitState,
    SchedulerResourceKind, SchedulerRuntimeReadinessState,
};

const MAX_TIME_MS: u64 = i64::MAX as u64;

pub const SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION: u16 = 1;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SchedulerResourceObservationError {
    #[error(transparent)]
    Contract(#[from] SchedulerContractError),
    #[error("resource observer failed: {0}")]
    Observer(String),
}

pub trait SchedulerResourceObserver {
    fn observe(
        &self,
    ) -> Result<ValidatedSchedulerResourceResidencySnapshot, SchedulerResourceObservationError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerDeviceResourceSnapshot {
    pub device_id: DeviceIntentId,
    pub resource_kind: SchedulerResourceKind,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub reserved_bytes: u64,
}

impl SchedulerDeviceResourceSnapshot {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_positive_bytes("device_resource.total_bytes", self.total_bytes)?;
        self.available_bytes
            .checked_add(self.reserved_bytes)
            .ok_or(SchedulerContractError::InvalidField {
                field: "device_resource.available_bytes",
                reason: "available plus reserved bytes must not overflow",
            })?;
        if self.available_bytes + self.reserved_bytes > self.total_bytes {
            return Err(SchedulerContractError::InvalidField {
                field: "device_resource.available_bytes",
                reason: "available plus reserved bytes must not exceed total bytes",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerResourceReservation {
    pub reservation_lease_id: SchedulerReservationLeaseId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub task_id: SchedulerTaskId,
    pub device_id: DeviceIntentId,
    pub resource_kind: SchedulerResourceKind,
    pub reserved_bytes: u64,
}

impl SchedulerResourceReservation {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_positive_bytes("resource_reservation.reserved_bytes", self.reserved_bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerRuntimeReadiness {
    pub runtime_id: RuntimeIntentId,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_ids: Vec<DeviceIntentId>,
    pub state: SchedulerRuntimeReadinessState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerResourceDiagnostic>,
}

impl SchedulerRuntimeReadiness {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_diagnostics(&self.diagnostics)?;
        if matches!(
            self.state,
            SchedulerRuntimeReadinessState::NotInstalled
                | SchedulerRuntimeReadinessState::NotImplemented
                | SchedulerRuntimeReadinessState::Failed
                | SchedulerRuntimeReadinessState::Unknown
        ) && self.diagnostics.is_empty()
        {
            return Err(SchedulerContractError::MissingField {
                field: "runtime_readiness.diagnostics",
            });
        }
        let mut seen = BTreeSet::new();
        for device_id in &self.device_ids {
            if !seen.insert(device_id) {
                return Err(SchedulerContractError::InvalidField {
                    field: "runtime_readiness.device_ids",
                    reason: "runtime readiness device ids must not contain duplicates",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerModelResidency {
    pub model_ref: PumasModelRef,
    pub runtime_id: RuntimeIntentId,
    pub device_id: DeviceIntentId,
    pub state: SchedulerModelResidencyState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resident_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerResourceDiagnostic>,
}

impl SchedulerModelResidency {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        self.model_ref.validate().map_err(map_dependency_error)?;
        if let Some(resident_bytes) = self.resident_bytes {
            validate_positive_bytes("model_residency.resident_bytes", resident_bytes)?;
        }
        validate_diagnostics(&self.diagnostics)?;
        if matches!(
            self.state,
            SchedulerModelResidencyState::Unavailable | SchedulerModelResidencyState::Unknown
        ) && self.diagnostics.is_empty()
        {
            return Err(SchedulerContractError::MissingField {
                field: "model_residency.diagnostics",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerLoadWarmupEstimate {
    pub model_ref: PumasModelRef,
    pub runtime_id: RuntimeIntentId,
    pub device_id: DeviceIntentId,
    pub load_time_ms: u64,
    pub warmup_time_ms: u64,
    pub peak_additional_bytes: u64,
}

impl SchedulerLoadWarmupEstimate {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        self.model_ref.validate().map_err(map_dependency_error)?;
        validate_time_ms("load_warmup_estimate.load_time_ms", self.load_time_ms)?;
        validate_time_ms("load_warmup_estimate.warmup_time_ms", self.warmup_time_ms)?;
        validate_positive_bytes(
            "load_warmup_estimate.peak_additional_bytes",
            self.peak_additional_bytes,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerBatchingMemoryImpact {
    pub batching_group_id: SchedulerBatchingGroupId,
    pub runtime_id: RuntimeIntentId,
    pub device_id: DeviceIntentId,
    pub batch_size: u64,
    pub additional_reserved_bytes: u64,
}

impl SchedulerBatchingMemoryImpact {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.batch_size == 0 {
            return Err(SchedulerContractError::InvalidField {
                field: "batching_memory_impact.batch_size",
                reason: "batch size must be greater than zero",
            });
        }
        validate_positive_bytes(
            "batching_memory_impact.additional_reserved_bytes",
            self.additional_reserved_bytes,
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerResourceFitAssessment {
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub task_id: SchedulerTaskId,
    pub state: SchedulerResourceFitState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerResourceDiagnostic>,
}

impl SchedulerResourceFitAssessment {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_diagnostics(&self.diagnostics)?;
        if matches!(
            self.state,
            SchedulerResourceFitState::WaitingForResources
                | SchedulerResourceFitState::ImpossibleFit
                | SchedulerResourceFitState::Unknown
        ) && self.diagnostics.is_empty()
        {
            return Err(SchedulerContractError::MissingField {
                field: "resource_fit_assessment.diagnostics",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerResourceResidencySnapshot {
    #[serde(default = "default_scheduler_resource_residency_contract_version")]
    pub contract_version: u16,
    pub observed_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub device_resources: Vec<SchedulerDeviceResourceSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_reservations: Vec<SchedulerResourceReservation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runtime_readiness: Vec<SchedulerRuntimeReadiness>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_residency: Vec<SchedulerModelResidency>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub load_warmup_estimates: Vec<SchedulerLoadWarmupEstimate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub batching_memory_impacts: Vec<SchedulerBatchingMemoryImpact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fit_assessments: Vec<SchedulerResourceFitAssessment>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerResourceDiagnostic>,
}

impl SchedulerResourceResidencySnapshot {
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        if self.contract_version != SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION {
            return Err(SchedulerContractError::InvalidField {
                field: "contract_version",
                reason: "unsupported scheduler resource residency contract version",
            });
        }
        if self.observed_at_unix_ms == 0 || self.observed_at_unix_ms > MAX_TIME_MS {
            return Err(SchedulerContractError::InvalidField {
                field: "observed_at_unix_ms",
                reason: "observation timestamp must be a positive unix millisecond value",
            });
        }
        validate_device_resources(&self.device_resources)?;
        validate_reservations(&self.active_reservations)?;
        self.runtime_readiness
            .iter()
            .try_for_each(SchedulerRuntimeReadiness::validate)?;
        self.model_residency
            .iter()
            .try_for_each(SchedulerModelResidency::validate)?;
        self.load_warmup_estimates
            .iter()
            .try_for_each(SchedulerLoadWarmupEstimate::validate)?;
        self.batching_memory_impacts
            .iter()
            .try_for_each(SchedulerBatchingMemoryImpact::validate)?;
        self.fit_assessments
            .iter()
            .try_for_each(SchedulerResourceFitAssessment::validate)?;
        validate_diagnostics(&self.diagnostics)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerResourceResidencySnapshot(SchedulerResourceResidencySnapshot);

impl ValidatedSchedulerResourceResidencySnapshot {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerResourceResidencySnapshot {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerResourceResidencySnapshot {
        self.0
    }
}

impl TryFrom<SchedulerResourceResidencySnapshot> for ValidatedSchedulerResourceResidencySnapshot {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerResourceResidencySnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn validate_device_resources(
    resources: &[SchedulerDeviceResourceSnapshot],
) -> Result<(), SchedulerContractError> {
    let mut seen = BTreeSet::new();
    for resource in resources {
        resource.validate()?;
        if !seen.insert((resource.device_id.as_str(), &resource.resource_kind)) {
            return Err(SchedulerContractError::InvalidField {
                field: "device_resources",
                reason: "device resource observations must be unique by device and resource kind",
            });
        }
    }
    Ok(())
}

fn validate_reservations(
    reservations: &[SchedulerResourceReservation],
) -> Result<(), SchedulerContractError> {
    let mut seen = BTreeSet::new();
    for reservation in reservations {
        reservation.validate()?;
        if !seen.insert(reservation.reservation_lease_id.as_str()) {
            return Err(SchedulerContractError::InvalidField {
                field: "active_reservations",
                reason: "active reservation lease ids must not contain duplicates",
            });
        }
    }
    Ok(())
}

fn validate_diagnostics(
    diagnostics: &[SchedulerResourceDiagnostic],
) -> Result<(), SchedulerContractError> {
    for diagnostic in diagnostics {
        diagnostic.validate()?;
    }
    Ok(())
}

fn validate_positive_bytes(field: &'static str, value: u64) -> Result<(), SchedulerContractError> {
    if value == 0 {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "byte values must be greater than zero",
        });
    }
    Ok(())
}

fn validate_time_ms(field: &'static str, value: u64) -> Result<(), SchedulerContractError> {
    if value > MAX_TIME_MS {
        return Err(SchedulerContractError::InvalidField {
            field,
            reason: "time values must fit signed ledger/resource arithmetic",
        });
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

fn default_scheduler_resource_residency_contract_version() -> u16 {
    SCHEDULER_RESOURCE_RESIDENCY_CONTRACT_VERSION
}
