use serde::{Deserialize, Serialize};

use crate::error::DependencyPlanningContractError;
use crate::model_ref::{PumasArtifactLoadTarget, PumasModelRef};
use crate::request::{DeviceIntentId, RuntimeIntentId};

const MAX_DIAGNOSTIC_MESSAGE_LEN: usize = 256;
const MAX_DIAGNOSTIC_FIELD_PATH_LEN: usize = 256;

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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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

impl DependencyPlanningDiagnostic {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_diagnostic_text("dependency_diagnostic.message", &self.message)?;
        if let Some(field_path) = &self.field_path {
            validate_diagnostic_field_path("dependency_diagnostic.field_path", field_path)?;
        }
        Ok(())
    }
}

/// Dependency planning result returned by the host/planner boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
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

fn validate_diagnostic_text(
    field: &'static str,
    value: &str,
) -> Result<(), DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_DIAGNOSTIC_MESSAGE_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_DIAGNOSTIC_MESSAGE_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DependencyPlanningContractError::InvalidText { field });
    }
    Ok(())
}

fn validate_diagnostic_field_path(
    field: &'static str,
    value: &str,
) -> Result<(), DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_DIAGNOSTIC_FIELD_PATH_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_DIAGNOSTIC_FIELD_PATH_LEN,
        });
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(DependencyPlanningContractError::InvalidField {
            field,
            reason: "validation field paths must be contract fields, not filesystem paths",
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']' | '-' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(())
}
