use serde::{Deserialize, Serialize};

use crate::error::SchedulerContractError;

const MAX_TEXT_LEN: usize = 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerResourceKind {
    SystemRam,
    SystemSwap,
    DeviceVram,
    DeviceSharedMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerRuntimeReadinessState {
    Ready,
    Starting,
    NotInstalled,
    NotImplemented,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerModelResidencyState {
    Resident,
    Loading,
    Evicting,
    NotResident,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerResourceFitState {
    Fits,
    WaitingForResources,
    ImpossibleFit,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerResourceDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerResourceDiagnosticCode {
    ObservationUnavailable,
    ObservationStale,
    CollectorNotInstalled,
    CollectorNotSupported,
    PermissionDenied,
    ReservationOverflow,
    ImpossibleFit,
    RuntimeNotReady,
    ResidencyUnavailable,
    BatchingMemoryOverflow,
    SchedulerResourcePolicyError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerResourceDiagnostic {
    pub severity: SchedulerResourceDiagnosticSeverity,
    pub code: SchedulerResourceDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerResourceDiagnostic {
    pub(crate) fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("resource_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("resource_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}
