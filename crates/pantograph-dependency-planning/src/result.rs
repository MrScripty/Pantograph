use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{PumasArtifactLoadTarget, PumasModelRef};
use crate::request::{DeviceIntentId, RuntimeIntentId};

/// Normalized planning state returned by host dependency planning.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyPlanningState {
    Ready,
    Unavailable,
    Invalid,
    Stale,
    Ambiguous,
    NeedsDetail,
    Missing,
    NotImplemented,
}

impl DependencyPlanningState {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Unavailable => "unavailable",
            Self::Invalid => "invalid",
            Self::Stale => "stale",
            Self::Ambiguous => "ambiguous",
            Self::NeedsDetail => "needs_detail",
            Self::Missing => "missing",
            Self::NotImplemented => "not_implemented",
        }
    }
}

/// Typed diagnostic code for dependency planning failures and warnings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyPlanningDiagnosticCode {
    InvalidRequest,
    MissingPumasModelRef,
    InvalidPumasModelRef,
    MissingSelectedArtifact,
    PumasUnavailable,
    ArtifactMissing,
    ArtifactInvalid,
    ArtifactStale,
    ArtifactAmbiguous,
    ArtifactNeedsDetail,
    RuntimeUnavailable,
    DeviceUnavailable,
    NotImplemented,
    InternalError,
}

/// Severity for dependency planning diagnostics.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DependencyPlanningSeverity {
    Info,
    Warning,
    Error,
}

/// Structured diagnostic emitted by dependency planning.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyPlanningDiagnostic {
    pub code: DependencyPlanningDiagnosticCode,
    pub severity: DependencyPlanningSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<RuntimeIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<DeviceIntentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
}

/// Dependency planning result returned by the host/planner boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct DependencyPlanningResult {
    pub state: DependencyPlanningState,
    pub model_ref: PumasModelRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_target: Option<PumasArtifactLoadTarget>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DependencyPlanningDiagnostic>,
}

impl DependencyPlanningResult {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        self.model_ref.validate()?;
        match (self.state, self.load_target.as_ref()) {
            (DependencyPlanningState::Ready, Some(target)) => target.validate_for_handoff(),
            (DependencyPlanningState::Ready, None) => {
                Err(DependencyPlanningContractError::ReadyResultMissingLoadTarget)
            }
            (_, Some(_)) => Err(
                DependencyPlanningContractError::NonReadyResultHasLoadTarget {
                    state: self.state.label(),
                },
            ),
            (_, None) => Ok(()),
        }
    }
}
