use serde::{Deserialize, Serialize};

use crate::error::SchedulerContractError;
use crate::queue::SchedulerQueueTaskState;
use crate::{SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId};

const MAX_TEXT_LEN: usize = 1024;

/// Current contract version for scheduler task lifecycle diagnostics.
pub const SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION: u16 = 1;

/// Severity for scheduler task lifecycle diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerTaskLifecycleDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Stable diagnostic code for scheduler task lifecycle state explanation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerTaskLifecycleDiagnosticCode {
    TaskPending,
    TaskReady,
    TaskBlocked,
    TaskDeferred,
    TaskCompleted,
    WaitingDependencyReadiness,
    WaitingResources,
    WaitingBatch,
    RetryableFailure,
    TerminalFailure,
    RuntimeUnavailable,
    DeviceUnavailable,
    DependencyUnavailable,
    ResourceUnavailable,
    SchedulerPolicyError,
}

impl SchedulerTaskLifecycleDiagnosticCode {
    fn is_compatible_with(&self, state: SchedulerQueueTaskState) -> bool {
        use SchedulerQueueTaskState::{
            Blocked, Completed, PausedDeferred, Pending, Ready, RetryableFailed, Running,
            TerminalFailed, WaitingBatch, WaitingDependencyReadiness, WaitingResources,
        };
        use SchedulerTaskLifecycleDiagnosticCode::{
            DependencyUnavailable, DeviceUnavailable, ResourceUnavailable, RetryableFailure,
            RuntimeUnavailable, SchedulerPolicyError, TaskBlocked, TaskCompleted, TaskDeferred,
            TaskPending, TaskReady, TerminalFailure, WaitingBatch as WaitingBatchCode,
            WaitingDependencyReadiness as WaitingDependencyReadinessCode,
            WaitingResources as WaitingResourcesCode,
        };

        match state {
            Pending => matches!(self, TaskPending | SchedulerPolicyError),
            Ready => matches!(self, TaskReady | SchedulerPolicyError),
            Blocked => matches!(
                self,
                TaskBlocked | DependencyUnavailable | SchedulerPolicyError
            ),
            WaitingDependencyReadiness => matches!(
                self,
                WaitingDependencyReadinessCode | DependencyUnavailable | SchedulerPolicyError
            ),
            WaitingResources => matches!(
                self,
                WaitingResourcesCode
                    | ResourceUnavailable
                    | DeviceUnavailable
                    | SchedulerPolicyError
            ),
            WaitingBatch => matches!(self, WaitingBatchCode | SchedulerPolicyError),
            Running => matches!(self, SchedulerPolicyError),
            PausedDeferred => matches!(
                self,
                TaskDeferred
                    | WaitingDependencyReadinessCode
                    | WaitingResourcesCode
                    | WaitingBatchCode
                    | SchedulerPolicyError
            ),
            RetryableFailed => matches!(
                self,
                RetryableFailure
                    | RuntimeUnavailable
                    | DeviceUnavailable
                    | DependencyUnavailable
                    | ResourceUnavailable
                    | SchedulerPolicyError
            ),
            TerminalFailed => matches!(
                self,
                TerminalFailure
                    | RuntimeUnavailable
                    | DeviceUnavailable
                    | DependencyUnavailable
                    | ResourceUnavailable
                    | SchedulerPolicyError
            ),
            Completed => matches!(self, TaskCompleted),
        }
    }
}

/// Bounded scheduler task lifecycle diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTaskLifecycleDiagnostic {
    pub severity: SchedulerTaskLifecycleDiagnosticSeverity,
    pub code: SchedulerTaskLifecycleDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerTaskLifecycleDiagnostic {
    fn validate_for_state(
        &self,
        state: SchedulerQueueTaskState,
    ) -> Result<(), SchedulerContractError> {
        validate_text("task_lifecycle_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("task_lifecycle_diagnostic.hint", hint)?;
        }
        if !self.code.is_compatible_with(state) {
            return Err(SchedulerContractError::InvalidField {
                field: "task_lifecycle_diagnostic.code",
                reason: "diagnostic code is not compatible with queue task state",
            });
        }
        Ok(())
    }
}

/// Backend-owned diagnostic snapshot for one scheduler queue task state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTaskLifecycleDiagnosticSnapshot {
    #[serde(default = "default_scheduler_task_lifecycle_diagnostic_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub state: SchedulerQueueTaskState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<SchedulerTaskLifecycleDiagnostic>,
}

impl SchedulerTaskLifecycleDiagnosticSnapshot {
    /// Validates a raw diagnostic snapshot before graph/run inspection uses it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        if state_requires_diagnostics(self.state) && self.diagnostics.is_empty() {
            return Err(SchedulerContractError::MissingField {
                field: "task_lifecycle.diagnostics",
            });
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate_for_state(self.state)?;
        }
        Ok(())
    }
}

/// Validated task lifecycle diagnostics for graph editor and run inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerTaskLifecycleDiagnosticSnapshot(
    SchedulerTaskLifecycleDiagnosticSnapshot,
);

impl ValidatedSchedulerTaskLifecycleDiagnosticSnapshot {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerTaskLifecycleDiagnosticSnapshot {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerTaskLifecycleDiagnosticSnapshot {
        self.0
    }
}

impl TryFrom<SchedulerTaskLifecycleDiagnosticSnapshot>
    for ValidatedSchedulerTaskLifecycleDiagnosticSnapshot
{
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerTaskLifecycleDiagnosticSnapshot) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn state_requires_diagnostics(state: SchedulerQueueTaskState) -> bool {
    use SchedulerQueueTaskState::{
        Blocked, Completed, PausedDeferred, RetryableFailed, TerminalFailed, WaitingBatch,
        WaitingDependencyReadiness, WaitingResources,
    };

    matches!(
        state,
        Blocked
            | WaitingDependencyReadiness
            | WaitingResources
            | WaitingBatch
            | PausedDeferred
            | RetryableFailed
            | TerminalFailed
            | Completed
    )
}

fn default_scheduler_task_lifecycle_diagnostic_contract_version() -> u16 {
    SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler task lifecycle diagnostic contract version",
        })
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
