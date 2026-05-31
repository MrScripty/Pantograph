use async_trait::async_trait;
use pantograph_scheduler::{
    SchedulerContractError, SchedulerDispatchCandidateId, SchedulerNodeId,
    SchedulerReservationLeaseId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;
const MAX_RESERVATION_LIFECYCLE_DIAGNOSTICS: usize = 64;

/// Current contract version for runtime dispatch reservation lifecycle events.
pub const RESERVATION_LIFECYCLE_CONTRACT_VERSION: u16 = 1;

/// Workflow/scheduler outcome that drives runtime-registry reservation cleanup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReservationLifecycleOutcome {
    CandidateUnselected,
    CandidateRequestRejected,
    DispatchStarted,
    RuntimeHostDispatchRejected,
    RuntimeHostCompleted,
    RuntimeHostFailed,
    WorkflowCancelled,
    RetryDeferred,
    SessionClosed,
    DuplicateReplay,
}

impl ReservationLifecycleOutcome {
    fn requires_diagnostics(&self) -> bool {
        matches!(
            self,
            Self::CandidateRequestRejected
                | Self::RuntimeHostDispatchRejected
                | Self::RuntimeHostFailed
                | Self::WorkflowCancelled
                | Self::RetryDeferred
                | Self::SessionClosed
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReservationLifecycleDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReservationLifecycleDiagnosticCode {
    CandidateUnselected,
    RequestRejected,
    DispatchStarted,
    RuntimeHostRejected,
    RuntimeHostCompleted,
    RuntimeHostFailed,
    WorkflowCancelled,
    RetryDeferred,
    SessionClosed,
    DuplicateReplay,
    LeaseNotFound,
    LeaseOwnerMismatch,
    RegistryReleaseFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReservationLifecycleDiagnostic {
    pub severity: ReservationLifecycleDiagnosticSeverity,
    pub code: ReservationLifecycleDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl ReservationLifecycleDiagnostic {
    fn validate(&self) -> Result<(), ReservationLifecycleContractError> {
        validate_text("diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Application-owned reservation lifecycle event for one scheduler lease.
///
/// This contract is path-free. It carries workflow/task/candidate correlation
/// and the scheduler lease id, but never graph paths, `ModelRefV2`, reduced
/// execution-plan fields, runtime-host load targets, or provider-private facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReservationLifecycleEvent {
    #[serde(default = "default_reservation_lifecycle_contract_version")]
    pub contract_version: u16,
    pub lifecycle_event_id: String,
    pub reservation_lease_id: SchedulerReservationLeaseId,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub outcome: ReservationLifecycleOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<SchedulerDispatchCandidateId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ReservationLifecycleDiagnostic>,
}

impl ReservationLifecycleEvent {
    pub fn validate(&self) -> Result<(), ReservationLifecycleContractError> {
        validate_contract_version(self.contract_version)?;
        validate_identifier("lifecycle_event_id", &self.lifecycle_event_id)?;
        validate_diagnostics(&self.diagnostics)?;
        if self.outcome.requires_diagnostics() && self.diagnostics.is_empty() {
            return Err(ReservationLifecycleContractError::MissingField {
                field: "diagnostics",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedReservationLifecycleEvent(ReservationLifecycleEvent);

impl ValidatedReservationLifecycleEvent {
    #[must_use]
    pub fn as_ref(&self) -> &ReservationLifecycleEvent {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> ReservationLifecycleEvent {
        self.0
    }
}

impl TryFrom<ReservationLifecycleEvent> for ValidatedReservationLifecycleEvent {
    type Error = ReservationLifecycleContractError;

    fn try_from(value: ReservationLifecycleEvent) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ReservationLifecycleApplicationState {
    Applied,
    AlreadyApplied,
    Failed,
}

/// Infrastructure-owned response after applying a reservation lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ReservationLifecycleApplication {
    #[serde(default = "default_reservation_lifecycle_contract_version")]
    pub contract_version: u16,
    pub lifecycle_event_id: String,
    pub reservation_lease_id: SchedulerReservationLeaseId,
    pub state: ReservationLifecycleApplicationState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<ReservationLifecycleDiagnostic>,
}

impl ReservationLifecycleApplication {
    pub fn validate(&self) -> Result<(), ReservationLifecycleContractError> {
        validate_contract_version(self.contract_version)?;
        validate_identifier("lifecycle_event_id", &self.lifecycle_event_id)?;
        validate_diagnostics(&self.diagnostics)?;
        if self.state == ReservationLifecycleApplicationState::Failed && self.diagnostics.is_empty()
        {
            return Err(ReservationLifecycleContractError::MissingField {
                field: "diagnostics",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedReservationLifecycleApplication(ReservationLifecycleApplication);

impl ValidatedReservationLifecycleApplication {
    #[must_use]
    pub fn as_ref(&self) -> &ReservationLifecycleApplication {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> ReservationLifecycleApplication {
        self.0
    }
}

impl TryFrom<ReservationLifecycleApplication> for ValidatedReservationLifecycleApplication {
    type Error = ReservationLifecycleContractError;

    fn try_from(value: ReservationLifecycleApplication) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

/// Port implemented by the runtime-registry infrastructure owner.
#[async_trait]
pub trait ReservationLifecyclePort: Send + Sync {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReservationLifecyclePortError {
    #[error("reservation lifecycle port failed: {message}")]
    Failed { message: String },
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReservationLifecycleContractError {
    #[error("missing required field `{field}`")]
    MissingField { field: &'static str },
    #[error("invalid field `{field}`: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("field `{field}` is too long; max {max_len} bytes")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("invalid identifier field `{field}`")]
    InvalidIdentifier { field: &'static str },
    #[error("invalid text field `{field}`")]
    InvalidText { field: &'static str },
    #[error("reservation lifecycle value has {actual} diagnostics, maximum is {max}")]
    TooManyDiagnostics { actual: usize, max: usize },
    #[error(transparent)]
    Scheduler(#[from] SchedulerContractError),
}

fn default_reservation_lifecycle_contract_version() -> u16 {
    RESERVATION_LIFECYCLE_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), ReservationLifecycleContractError> {
    if value == RESERVATION_LIFECYCLE_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(ReservationLifecycleContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported reservation lifecycle contract version",
        })
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), ReservationLifecycleContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_ID_LEN
        || trimmed
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':')))
    {
        return Err(ReservationLifecycleContractError::InvalidIdentifier { field });
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
) -> Result<(), ReservationLifecycleContractError> {
    if value.trim().is_empty() {
        return Err(ReservationLifecycleContractError::InvalidText { field });
    }
    if value.len() > MAX_TEXT_LEN {
        return Err(ReservationLifecycleContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    Ok(())
}

fn validate_diagnostics(
    diagnostics: &[ReservationLifecycleDiagnostic],
) -> Result<(), ReservationLifecycleContractError> {
    if diagnostics.len() > MAX_RESERVATION_LIFECYCLE_DIAGNOSTICS {
        return Err(ReservationLifecycleContractError::TooManyDiagnostics {
            actual: diagnostics.len(),
            max: MAX_RESERVATION_LIFECYCLE_DIAGNOSTICS,
        });
    }
    for diagnostic in diagnostics {
        diagnostic.validate()?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "reservation_lifecycle_tests.rs"]
mod tests;
