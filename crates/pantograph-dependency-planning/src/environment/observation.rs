use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::preflight::DependencyPlanningIdentityKey;
use crate::request::{DependencyBindingId, DependencyRequirementsId};
use crate::result::DependencyPlanningDiagnostic;

use super::payload::{
    DependencyBindingStatusRow, DependencyBindingStatusState, DependencyEnvironmentOperation,
    DependencyEnvironmentOperationState, DependencyRequirement, DependencyRequirementBinding,
};
use super::provider_source::{
    validate_provider_source_alternatives, DependencyProviderSourceAlternative,
};
use super::scalar::{
    validate_diagnostics, validate_unique_binding_ids, DependencyOperationTimestampMs,
};
use super::state::{
    DependencyEnvironmentAction, DependencyEnvironmentFailureState,
    DependencyEnvironmentInstallState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentValidationState,
};
use super::{
    DependencyEnvironmentRef, DependencyEnvironmentResult, ValidatedDependencyEnvironmentResult,
};

/// Freshness attached to one provider-owned dependency observation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyInventoryObservationFreshness {
    Fresh,
    Stale,
}

/// Provider-owned readiness observation for one selected dependency binding.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyInventoryObservationState {
    Ready,
    Missing,
    Unavailable,
    Invalid,
    Failed,
    NotImplemented,
}

/// One provider evidence row for one selected binding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyInventoryObservationRow {
    pub binding_id: DependencyBindingId,
    pub state: DependencyInventoryObservationState,
    pub validation_state: DependencyEnvironmentValidationState,
    pub freshness: DependencyInventoryObservationFreshness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_at_ms: Option<DependencyOperationTimestampMs>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<DependencyProviderSourceAlternative>,
}

impl DependencyInventoryObservationRow {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.freshness == DependencyInventoryObservationFreshness::Stale
            && self.diagnostics.is_empty()
        {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_inventory_observation.diagnostics",
            });
        }
        validate_diagnostics(&self.diagnostics)?;
        validate_provider_source_alternatives(&self.alternatives)
    }
}

/// Contract input for projecting provider observations into one result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct DependencyInventoryObservationProjection {
    #[serde(default = "default_dependency_inventory_observation_contract_version")]
    pub contract_version: u32,
    pub action: DependencyEnvironmentAction,
    pub identity_key: DependencyPlanningIdentityKey,
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
    pub observations: Vec<DependencyInventoryObservationRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyInventoryObservationProjection {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        if self.contract_version != 1 {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_inventory_observation_projection.contract_version",
                reason: "only dependency inventory observation projection contract version 1 is supported",
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
        for observation in &self.observations {
            observation.validate()?;
        }
        validate_diagnostics(&self.diagnostics)?;
        validate_observation_coverage(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedDependencyInventoryObservationProjection(
    DependencyInventoryObservationProjection,
);

impl ValidatedDependencyInventoryObservationProjection {
    pub fn into_inner(self) -> DependencyInventoryObservationProjection {
        self.0
    }

    pub fn as_projection(&self) -> &DependencyInventoryObservationProjection {
        &self.0
    }
}

impl TryFrom<DependencyInventoryObservationProjection>
    for ValidatedDependencyInventoryObservationProjection
{
    type Error = DependencyPlanningContractError;

    fn try_from(value: DependencyInventoryObservationProjection) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

impl TryFrom<serde_json::Value> for ValidatedDependencyInventoryObservationProjection {
    type Error = DependencyPlanningContractError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        let projection: DependencyInventoryObservationProjection = serde_json::from_value(value)
            .map_err(|_| DependencyPlanningContractError::InvalidField {
                field: "dependency_inventory_observation_projection",
                reason: "projection JSON did not match dependency inventory observation contract",
            })?;
        Self::try_from(projection)
    }
}

pub fn dependency_environment_result_from_inventory_observations(
    projection: &ValidatedDependencyInventoryObservationProjection,
) -> Result<ValidatedDependencyEnvironmentResult, DependencyPlanningContractError> {
    let projection = projection.as_projection();
    let aggregate = AggregateObservationState::from_observations(&projection.observations);
    let mut diagnostics = projection.diagnostics.clone();
    for observation in &projection.observations {
        diagnostics.extend(observation.diagnostics.iter().cloned());
    }

    let result = DependencyEnvironmentResult {
        contract_version: 1,
        action: projection.action,
        identity_key: projection.identity_key.clone(),
        readiness_state: aggregate.readiness_state(),
        install_state: aggregate.install_state(),
        validation_state: aggregate.validation_state(),
        failure_state: aggregate.failure_state(),
        dependency_requirements_id: projection.dependency_requirements_id.clone(),
        environment_ref: projection.environment_ref.clone(),
        requirements: projection.requirements.clone(),
        bindings: projection.bindings.clone(),
        selected_binding_ids: projection.selected_binding_ids.clone(),
        binding_statuses: binding_statuses_from_observations(&projection.observations),
        operation: Some(DependencyEnvironmentOperation {
            state: aggregate.operation_state(),
            started_at_ms: None,
            completed_at_ms: None,
        }),
        validation_errors: Vec::new(),
        diagnostics,
    };
    ValidatedDependencyEnvironmentResult::try_from(result)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AggregateObservationState {
    Ready,
    Missing,
    Unavailable,
    Invalid,
    Failed,
    NotImplemented,
    Stale,
}

impl AggregateObservationState {
    fn from_observations(observations: &[DependencyInventoryObservationRow]) -> Self {
        if observations
            .iter()
            .any(|observation| observation.state == DependencyInventoryObservationState::Invalid)
        {
            return Self::Invalid;
        }
        if observations.iter().any(|observation| {
            observation.freshness == DependencyInventoryObservationFreshness::Stale
                || observation.validation_state == DependencyEnvironmentValidationState::Stale
        }) {
            return Self::Stale;
        }
        if observations.iter().any(|observation| {
            observation.state == DependencyInventoryObservationState::NotImplemented
        }) {
            return Self::NotImplemented;
        }
        if observations
            .iter()
            .any(|observation| observation.state == DependencyInventoryObservationState::Failed)
        {
            return Self::Failed;
        }
        if observations.iter().any(|observation| {
            observation.state == DependencyInventoryObservationState::Unavailable
        }) {
            return Self::Unavailable;
        }
        if observations
            .iter()
            .any(|observation| observation.state == DependencyInventoryObservationState::Missing)
        {
            return Self::Missing;
        }
        Self::Ready
    }

    fn readiness_state(self) -> DependencyEnvironmentReadinessState {
        match self {
            Self::Ready => DependencyEnvironmentReadinessState::Ready,
            Self::Missing => DependencyEnvironmentReadinessState::Missing,
            Self::Unavailable | Self::Stale => DependencyEnvironmentReadinessState::Unavailable,
            Self::Invalid => DependencyEnvironmentReadinessState::Invalid,
            Self::Failed => DependencyEnvironmentReadinessState::Failed,
            Self::NotImplemented => DependencyEnvironmentReadinessState::NotImplemented,
        }
    }

    fn install_state(self) -> DependencyEnvironmentInstallState {
        match self {
            Self::Ready => DependencyEnvironmentInstallState::Installed,
            Self::Missing => DependencyEnvironmentInstallState::NotInstalled,
            Self::Unavailable | Self::Invalid | Self::Stale => {
                DependencyEnvironmentInstallState::Blocked
            }
            Self::Failed => DependencyEnvironmentInstallState::Failed,
            Self::NotImplemented => DependencyEnvironmentInstallState::NotImplemented,
        }
    }

    fn validation_state(self) -> DependencyEnvironmentValidationState {
        match self {
            Self::Ready | Self::Missing | Self::Failed => {
                DependencyEnvironmentValidationState::Valid
            }
            Self::Unavailable => DependencyEnvironmentValidationState::Unavailable,
            Self::Invalid => DependencyEnvironmentValidationState::Invalid,
            Self::NotImplemented => DependencyEnvironmentValidationState::NotImplemented,
            Self::Stale => DependencyEnvironmentValidationState::Stale,
        }
    }

    fn failure_state(self) -> Option<DependencyEnvironmentFailureState> {
        match self {
            Self::Ready | Self::Missing => None,
            Self::Unavailable | Self::Stale => {
                Some(DependencyEnvironmentFailureState::EnvironmentUnavailable)
            }
            Self::Invalid => Some(DependencyEnvironmentFailureState::InvalidRequest),
            Self::Failed => Some(DependencyEnvironmentFailureState::CheckFailed),
            Self::NotImplemented => Some(DependencyEnvironmentFailureState::NotImplemented),
        }
    }

    fn operation_state(self) -> DependencyEnvironmentOperationState {
        match self {
            Self::Ready => DependencyEnvironmentOperationState::Succeeded,
            Self::Failed => DependencyEnvironmentOperationState::Failed,
            Self::Missing
            | Self::Unavailable
            | Self::Invalid
            | Self::NotImplemented
            | Self::Stale => DependencyEnvironmentOperationState::Blocked,
        }
    }
}

fn validate_observation_coverage(
    projection: &DependencyInventoryObservationProjection,
) -> Result<(), DependencyPlanningContractError> {
    let selected_binding_ids = projection
        .selected_binding_ids
        .iter()
        .collect::<BTreeSet<_>>();
    let binding_ids = projection
        .bindings
        .iter()
        .map(|binding| &binding.binding_id)
        .collect::<BTreeSet<_>>();
    for selected_id in &selected_binding_ids {
        if !binding_ids.contains(selected_id) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_inventory_observation_projection.selected_binding_ids",
                reason: "selected binding id must reference a binding row",
            });
        }
    }

    let mut observations_by_binding = BTreeMap::new();
    for observation in &projection.observations {
        if !selected_binding_ids.contains(&observation.binding_id) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_inventory_observation.binding_id",
                reason: "observation binding id must reference a selected binding",
            });
        }
        if observations_by_binding
            .insert(&observation.binding_id, observation)
            .is_some()
        {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_inventory_observation.binding_id",
                reason: "observation binding ids must be unique",
            });
        }
    }

    for selected_id in selected_binding_ids {
        if !observations_by_binding.contains_key(selected_id) {
            return Err(DependencyPlanningContractError::MissingField {
                field: "dependency_inventory_observation",
            });
        }
    }
    Ok(())
}

fn binding_statuses_from_observations(
    observations: &[DependencyInventoryObservationRow],
) -> Vec<DependencyBindingStatusRow> {
    observations
        .iter()
        .map(|observation| DependencyBindingStatusRow {
            binding_id: observation.binding_id.clone(),
            state: binding_state_from_observation(observation.state),
            validation_state: observation.validation_state,
            checked_at_ms: observation.checked_at_ms,
            installed_at_ms: observation.installed_at_ms,
            diagnostics: observation.diagnostics.clone(),
            alternatives: observation.alternatives.clone(),
        })
        .collect()
}

fn binding_state_from_observation(
    state: DependencyInventoryObservationState,
) -> DependencyBindingStatusState {
    match state {
        DependencyInventoryObservationState::Ready => DependencyBindingStatusState::Ready,
        DependencyInventoryObservationState::Missing => DependencyBindingStatusState::Missing,
        DependencyInventoryObservationState::Unavailable => {
            DependencyBindingStatusState::Unavailable
        }
        DependencyInventoryObservationState::Invalid => DependencyBindingStatusState::Invalid,
        DependencyInventoryObservationState::Failed => DependencyBindingStatusState::Failed,
        DependencyInventoryObservationState::NotImplemented => {
            DependencyBindingStatusState::NotImplemented
        }
    }
}

fn default_dependency_inventory_observation_contract_version() -> u32 {
    1
}
