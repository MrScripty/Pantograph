use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SchedulerContractError;
use crate::intent::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};

const MAX_ID_LEN: usize = 128;
const MAX_DIAGNOSTIC_TEXT_LEN: usize = 1024;

/// Current contract version for durable scheduler task state.
pub const SCHEDULER_TASK_STATE_CONTRACT_VERSION: u16 = 1;

macro_rules! scheduler_task_state_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, SchedulerContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $name {
            type Err = SchedulerContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = SchedulerContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

scheduler_task_state_id!(SchedulerTaskStateTransitionId, "transition_id");
scheduler_task_state_id!(SchedulerNonRuntimeTaskKind, "non_runtime_task_kind");
scheduler_task_state_id!(SchedulerSourceInputTaskKind, "source_input_task_kind");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerTaskStateKind {
    AwaitingInputs,
    InputUnavailable,
    Invalid,
    Ready,
    WaitingDependencyReadiness,
    WaitingResources,
    WaitingBatch,
    Running,
    PausedDeferred,
    RetryableFailed,
    TerminalFailed,
    Completed,
}

impl SchedulerTaskStateKind {
    fn can_transition_to(self, next: SchedulerTaskStateKind) -> bool {
        use SchedulerTaskStateKind::{
            AwaitingInputs, Completed, InputUnavailable, Invalid, PausedDeferred, Ready,
            RetryableFailed, Running, TerminalFailed, WaitingBatch, WaitingDependencyReadiness,
            WaitingResources,
        };

        match self {
            AwaitingInputs => matches!(
                next,
                Ready | InputUnavailable | Invalid | TerminalFailed | Completed
            ),
            InputUnavailable => matches!(next, AwaitingInputs | TerminalFailed),
            Invalid => matches!(next, TerminalFailed),
            Ready => matches!(
                next,
                WaitingDependencyReadiness
                    | WaitingResources
                    | WaitingBatch
                    | Running
                    | PausedDeferred
                    | TerminalFailed
            ),
            WaitingDependencyReadiness => {
                matches!(
                    next,
                    Ready | PausedDeferred | RetryableFailed | TerminalFailed
                )
            }
            WaitingResources => matches!(
                next,
                Ready | WaitingBatch | Running | PausedDeferred | RetryableFailed | TerminalFailed
            ),
            WaitingBatch => matches!(
                next,
                Ready | Running | PausedDeferred | RetryableFailed | TerminalFailed
            ),
            Running => matches!(
                next,
                Completed | RetryableFailed | TerminalFailed | PausedDeferred | WaitingResources
            ),
            PausedDeferred => matches!(
                next,
                Ready
                    | WaitingDependencyReadiness
                    | WaitingResources
                    | WaitingBatch
                    | TerminalFailed
            ),
            RetryableFailed => {
                matches!(next, Ready | WaitingDependencyReadiness | TerminalFailed)
            }
            TerminalFailed | Completed => false,
        }
    }

    fn can_be_initial(self) -> bool {
        matches!(
            self,
            SchedulerTaskStateKind::AwaitingInputs
                | SchedulerTaskStateKind::InputUnavailable
                | SchedulerTaskStateKind::Invalid
                | SchedulerTaskStateKind::WaitingDependencyReadiness
                | SchedulerTaskStateKind::Ready
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerTaskStateDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerTaskStateDiagnosticCode {
    AwaitingInputs,
    InputUnavailable,
    InvalidTask,
    TaskDeferred,
    RetryableFailure,
    TerminalFailure,
    SchedulerPolicyError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTaskStateDiagnostic {
    pub severity: SchedulerTaskStateDiagnosticSeverity,
    pub code: SchedulerTaskStateDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl SchedulerTaskStateDiagnostic {
    fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_text("task_state_diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("task_state_diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Path-free execution intent for one non-runtime workflow DAG task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerNonRuntimeTaskIntent {
    #[serde(default = "default_scheduler_task_state_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub task_kind: SchedulerNonRuntimeTaskKind,
}

/// Path-free materialization intent for one request-provided workflow input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerSourceInputTaskIntent {
    #[serde(default = "default_scheduler_task_state_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub task_kind: SchedulerSourceInputTaskKind,
}

impl SchedulerSourceInputTaskIntent {
    /// Validates this raw source-input intent before task-state policy consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        Ok(())
    }
}

impl SchedulerNonRuntimeTaskIntent {
    /// Validates this raw non-runtime execution intent before task-state policy
    /// consumes it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        Ok(())
    }
}

/// Typed executable payload for scheduler task states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "execution_kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum SchedulerTaskExecutionIntent {
    Runtime {
        task_intent: SchedulableTaskIntent,
    },
    SourceInput {
        task_intent: SchedulerSourceInputTaskIntent,
    },
    NonRuntime {
        task_intent: SchedulerNonRuntimeTaskIntent,
    },
}

impl SchedulerTaskExecutionIntent {
    #[must_use]
    pub fn runtime_task_intent(&self) -> Option<&SchedulableTaskIntent> {
        match self {
            Self::Runtime { task_intent } => Some(task_intent),
            Self::SourceInput { .. } | Self::NonRuntime { .. } => None,
        }
    }

    #[must_use]
    pub fn source_input_task_intent(&self) -> Option<&SchedulerSourceInputTaskIntent> {
        match self {
            Self::SourceInput { task_intent } => Some(task_intent),
            Self::Runtime { .. } | Self::NonRuntime { .. } => None,
        }
    }

    #[must_use]
    pub fn source_input_task_kind(&self) -> Option<&SchedulerSourceInputTaskKind> {
        self.source_input_task_intent()
            .map(|task_intent| &task_intent.task_kind)
    }

    #[must_use]
    pub fn non_runtime_task_intent(&self) -> Option<&SchedulerNonRuntimeTaskIntent> {
        match self {
            Self::NonRuntime { task_intent } => Some(task_intent),
            Self::Runtime { .. } | Self::SourceInput { .. } => None,
        }
    }

    #[must_use]
    pub fn non_runtime_task_kind(&self) -> Option<&SchedulerNonRuntimeTaskKind> {
        self.non_runtime_task_intent()
            .map(|task_intent| &task_intent.task_kind)
    }

    fn validate_for_task(
        &self,
        workflow_id: &SchedulerWorkflowId,
        workflow_run_id: &SchedulerWorkflowRunId,
        node_id: &SchedulerNodeId,
        task_id: &SchedulerTaskId,
    ) -> Result<(), SchedulerContractError> {
        match self {
            Self::Runtime { task_intent } => {
                task_intent.validate()?;
                validate_runtime_correlation(
                    workflow_id,
                    workflow_run_id,
                    node_id,
                    task_id,
                    task_intent,
                )
            }
            Self::SourceInput { task_intent } => {
                task_intent.validate()?;
                validate_source_input_correlation(
                    workflow_id,
                    workflow_run_id,
                    node_id,
                    task_id,
                    task_intent,
                )
            }
            Self::NonRuntime { task_intent } => {
                task_intent.validate()?;
                validate_non_runtime_correlation(
                    workflow_id,
                    workflow_run_id,
                    node_id,
                    task_id,
                    task_intent,
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum SchedulerTaskState {
    AwaitingInputs {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    InputUnavailable {
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    Invalid {
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    Ready {
        execution_intent: SchedulerTaskExecutionIntent,
    },
    WaitingDependencyReadiness {
        execution_intent: SchedulerTaskExecutionIntent,
    },
    WaitingResources {
        execution_intent: SchedulerTaskExecutionIntent,
    },
    WaitingBatch {
        execution_intent: SchedulerTaskExecutionIntent,
    },
    Running {
        execution_intent: SchedulerTaskExecutionIntent,
    },
    PausedDeferred {
        execution_intent: SchedulerTaskExecutionIntent,
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    RetryableFailed {
        execution_intent: SchedulerTaskExecutionIntent,
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    TerminalFailed {
        diagnostics: Vec<SchedulerTaskStateDiagnostic>,
    },
    Completed {
        execution_intent: SchedulerTaskExecutionIntent,
    },
}

impl SchedulerTaskState {
    #[must_use]
    pub fn kind(&self) -> SchedulerTaskStateKind {
        match self {
            SchedulerTaskState::AwaitingInputs { .. } => SchedulerTaskStateKind::AwaitingInputs,
            SchedulerTaskState::InputUnavailable { .. } => SchedulerTaskStateKind::InputUnavailable,
            SchedulerTaskState::Invalid { .. } => SchedulerTaskStateKind::Invalid,
            SchedulerTaskState::Ready { .. } => SchedulerTaskStateKind::Ready,
            SchedulerTaskState::WaitingDependencyReadiness { .. } => {
                SchedulerTaskStateKind::WaitingDependencyReadiness
            }
            SchedulerTaskState::WaitingResources { .. } => SchedulerTaskStateKind::WaitingResources,
            SchedulerTaskState::WaitingBatch { .. } => SchedulerTaskStateKind::WaitingBatch,
            SchedulerTaskState::Running { .. } => SchedulerTaskStateKind::Running,
            SchedulerTaskState::PausedDeferred { .. } => SchedulerTaskStateKind::PausedDeferred,
            SchedulerTaskState::RetryableFailed { .. } => SchedulerTaskStateKind::RetryableFailed,
            SchedulerTaskState::TerminalFailed { .. } => SchedulerTaskStateKind::TerminalFailed,
            SchedulerTaskState::Completed { .. } => SchedulerTaskStateKind::Completed,
        }
    }

    #[must_use]
    pub fn execution_intent(&self) -> Option<&SchedulerTaskExecutionIntent> {
        match self {
            SchedulerTaskState::Ready { execution_intent }
            | SchedulerTaskState::WaitingDependencyReadiness { execution_intent }
            | SchedulerTaskState::WaitingResources { execution_intent }
            | SchedulerTaskState::WaitingBatch { execution_intent }
            | SchedulerTaskState::Running { execution_intent }
            | SchedulerTaskState::PausedDeferred {
                execution_intent, ..
            }
            | SchedulerTaskState::RetryableFailed {
                execution_intent, ..
            }
            | SchedulerTaskState::Completed { execution_intent } => Some(execution_intent),
            SchedulerTaskState::AwaitingInputs { .. }
            | SchedulerTaskState::InputUnavailable { .. }
            | SchedulerTaskState::Invalid { .. }
            | SchedulerTaskState::TerminalFailed { .. } => None,
        }
    }

    #[must_use]
    pub fn task_intent(&self) -> Option<&SchedulableTaskIntent> {
        self.execution_intent()
            .and_then(SchedulerTaskExecutionIntent::runtime_task_intent)
    }

    fn validate_for_task(
        &self,
        workflow_id: &SchedulerWorkflowId,
        workflow_run_id: &SchedulerWorkflowRunId,
        node_id: &SchedulerNodeId,
        task_id: &SchedulerTaskId,
    ) -> Result<(), SchedulerContractError> {
        if let Some(execution_intent) = self.execution_intent() {
            execution_intent.validate_for_task(workflow_id, workflow_run_id, node_id, task_id)?;
        }
        validate_diagnostics(self)
    }
}

/// Durable task-state record persisted for replay and recovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTaskStateRecord {
    #[serde(default = "default_scheduler_task_state_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub state: SchedulerTaskState,
    pub state_version: u64,
    pub last_transition_id: SchedulerTaskStateTransitionId,
}

impl SchedulerTaskStateRecord {
    /// Validates a raw persisted task-state record before scheduler replay.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        if self.state_version == 0 {
            return Err(SchedulerContractError::InvalidField {
                field: "state_version",
                reason: "task state version must be greater than zero",
            });
        }
        self.state.validate_for_task(
            &self.workflow_id,
            &self.workflow_run_id,
            &self.node_id,
            &self.task_id,
        )
    }
}

/// Idempotent task-state transition event persisted for replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerTaskStateTransition {
    #[serde(default = "default_scheduler_task_state_contract_version")]
    pub contract_version: u16,
    pub transition_id: SchedulerTaskStateTransitionId,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_previous_state: Option<SchedulerTaskStateKind>,
    pub next_state: SchedulerTaskState,
}

impl SchedulerTaskStateTransition {
    /// Validates a raw task-state transition before applying it.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.next_state.validate_for_task(
            &self.workflow_id,
            &self.workflow_run_id,
            &self.node_id,
            &self.task_id,
        )?;
        if self.expected_previous_state.is_none() && !self.next_state.kind().can_be_initial() {
            return Err(SchedulerContractError::InvalidField {
                field: "next_state",
                reason: "initial task-state transition must create an initial state",
            });
        }
        if let Some(previous) = self.expected_previous_state {
            validate_state_transition(previous, self.next_state.kind())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum SchedulerTaskStateTransitionApplyResult {
    Applied(SchedulerTaskStateRecord),
    AlreadyApplied(SchedulerTaskStateRecord),
}

/// Pure contract logic for deterministic replay. Persistence, locking, and
/// worker lifecycle ownership belong to later infrastructure slices.
pub fn apply_scheduler_task_state_transition(
    current: Option<&SchedulerTaskStateRecord>,
    transition: SchedulerTaskStateTransition,
) -> Result<SchedulerTaskStateTransitionApplyResult, SchedulerContractError> {
    transition.validate()?;
    match current {
        None => apply_initial_transition(transition),
        Some(record) => apply_existing_transition(record, transition),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerTaskStateRecord(SchedulerTaskStateRecord);

impl ValidatedSchedulerTaskStateRecord {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerTaskStateRecord {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerTaskStateRecord {
        self.0
    }
}

impl TryFrom<SchedulerTaskStateRecord> for ValidatedSchedulerTaskStateRecord {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerTaskStateRecord) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerTaskStateTransition(SchedulerTaskStateTransition);

impl ValidatedSchedulerTaskStateTransition {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerTaskStateTransition {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerTaskStateTransition {
        self.0
    }
}

impl TryFrom<SchedulerTaskStateTransition> for ValidatedSchedulerTaskStateTransition {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerTaskStateTransition) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn apply_initial_transition(
    transition: SchedulerTaskStateTransition,
) -> Result<SchedulerTaskStateTransitionApplyResult, SchedulerContractError> {
    if transition.expected_previous_state.is_some() {
        return Err(SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "initial task-state transition must not expect existing state",
        });
    }
    Ok(SchedulerTaskStateTransitionApplyResult::Applied(
        record_from_transition(transition, 1),
    ))
}

fn apply_existing_transition(
    record: &SchedulerTaskStateRecord,
    transition: SchedulerTaskStateTransition,
) -> Result<SchedulerTaskStateTransitionApplyResult, SchedulerContractError> {
    record.validate()?;
    validate_same_task(record, &transition)?;
    if record.last_transition_id == transition.transition_id {
        if record.state != transition.next_state {
            return Err(SchedulerContractError::InvalidField {
                field: "transition_id",
                reason: "duplicate task-state transition id must replay the same next state",
            });
        }
        return Ok(SchedulerTaskStateTransitionApplyResult::AlreadyApplied(
            record.clone(),
        ));
    }
    let Some(expected_previous_state) = transition.expected_previous_state else {
        return Err(SchedulerContractError::MissingField {
            field: "expected_previous_state",
        });
    };
    if record.state.kind() != expected_previous_state {
        return Err(SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "task-state transition previous state must match persisted task state",
        });
    }
    validate_state_transition(record.state.kind(), transition.next_state.kind())?;
    Ok(SchedulerTaskStateTransitionApplyResult::Applied(
        record_from_transition(transition, record.state_version + 1),
    ))
}

fn record_from_transition(
    transition: SchedulerTaskStateTransition,
    state_version: u64,
) -> SchedulerTaskStateRecord {
    SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: transition.workflow_id,
        workflow_run_id: transition.workflow_run_id,
        node_id: transition.node_id,
        task_id: transition.task_id,
        state: transition.next_state,
        state_version,
        last_transition_id: transition.transition_id,
    }
}

fn validate_same_task(
    record: &SchedulerTaskStateRecord,
    transition: &SchedulerTaskStateTransition,
) -> Result<(), SchedulerContractError> {
    if record.workflow_id != transition.workflow_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "task-state transition workflow id must match persisted record",
        });
    }
    if record.workflow_run_id != transition.workflow_run_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "task-state transition workflow run id must match persisted record",
        });
    }
    if record.node_id != transition.node_id {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "task-state transition node id must match persisted record",
        });
    }
    if record.task_id != transition.task_id {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "task-state transition task id must match persisted record",
        });
    }
    Ok(())
}

fn validate_runtime_correlation(
    workflow_id: &SchedulerWorkflowId,
    workflow_run_id: &SchedulerWorkflowRunId,
    node_id: &SchedulerNodeId,
    task_id: &SchedulerTaskId,
    task_intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    if workflow_id.as_ref() != task_intent.workflow_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "task state workflow id must match task intent",
        });
    }
    if workflow_run_id.as_ref() != task_intent.workflow_run_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "task state workflow run id must match task intent",
        });
    }
    if node_id.as_ref() != task_intent.node_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "task state node id must match task intent",
        });
    }
    if task_id.as_ref() != task_intent.task_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "task state task id must match task intent",
        });
    }
    Ok(())
}

fn validate_non_runtime_correlation(
    workflow_id: &SchedulerWorkflowId,
    workflow_run_id: &SchedulerWorkflowRunId,
    node_id: &SchedulerNodeId,
    task_id: &SchedulerTaskId,
    task_intent: &SchedulerNonRuntimeTaskIntent,
) -> Result<(), SchedulerContractError> {
    if workflow_id.as_ref() != task_intent.workflow_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "task state workflow id must match non-runtime task intent",
        });
    }
    if workflow_run_id.as_ref() != task_intent.workflow_run_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "task state workflow run id must match non-runtime task intent",
        });
    }
    if node_id.as_ref() != task_intent.node_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "task state node id must match non-runtime task intent",
        });
    }
    if task_id.as_ref() != task_intent.task_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "task state task id must match non-runtime task intent",
        });
    }
    Ok(())
}

fn validate_source_input_correlation(
    workflow_id: &SchedulerWorkflowId,
    workflow_run_id: &SchedulerWorkflowRunId,
    node_id: &SchedulerNodeId,
    task_id: &SchedulerTaskId,
    task_intent: &SchedulerSourceInputTaskIntent,
) -> Result<(), SchedulerContractError> {
    if workflow_id.as_ref() != task_intent.workflow_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "task state workflow id must match source-input task intent",
        });
    }
    if workflow_run_id.as_ref() != task_intent.workflow_run_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "task state workflow run id must match source-input task intent",
        });
    }
    if node_id.as_ref() != task_intent.node_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "task state node id must match source-input task intent",
        });
    }
    if task_id.as_ref() != task_intent.task_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "task state task id must match source-input task intent",
        });
    }
    Ok(())
}

fn validate_diagnostics(state: &SchedulerTaskState) -> Result<(), SchedulerContractError> {
    let diagnostics = match state {
        SchedulerTaskState::AwaitingInputs { diagnostics }
        | SchedulerTaskState::InputUnavailable { diagnostics }
        | SchedulerTaskState::Invalid { diagnostics }
        | SchedulerTaskState::PausedDeferred { diagnostics, .. }
        | SchedulerTaskState::RetryableFailed { diagnostics, .. }
        | SchedulerTaskState::TerminalFailed { diagnostics } => diagnostics,
        SchedulerTaskState::Ready { .. }
        | SchedulerTaskState::WaitingDependencyReadiness { .. }
        | SchedulerTaskState::WaitingResources { .. }
        | SchedulerTaskState::WaitingBatch { .. }
        | SchedulerTaskState::Running { .. }
        | SchedulerTaskState::Completed { .. } => return Ok(()),
    };

    if matches!(
        state,
        SchedulerTaskState::InputUnavailable { .. }
            | SchedulerTaskState::Invalid { .. }
            | SchedulerTaskState::PausedDeferred { .. }
            | SchedulerTaskState::RetryableFailed { .. }
            | SchedulerTaskState::TerminalFailed { .. }
    ) && diagnostics.is_empty()
    {
        return Err(SchedulerContractError::MissingField {
            field: "task_state.diagnostics",
        });
    }
    for diagnostic in diagnostics {
        diagnostic.validate()?;
    }
    Ok(())
}

fn validate_state_transition(
    previous: SchedulerTaskStateKind,
    next: SchedulerTaskStateKind,
) -> Result<(), SchedulerContractError> {
    if previous == next {
        return Err(SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "task-state transition must advance to a different state",
        });
    }
    if previous.can_transition_to(next) {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "task-state transition is not allowed from the previous state",
        })
    }
}

fn default_scheduler_task_state_contract_version() -> u16 {
    SCHEDULER_TASK_STATE_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_TASK_STATE_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler task state contract version",
        })
    }
}

fn validate_identifier(field: &'static str, value: &str) -> Result<String, SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(SchedulerContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text(field: &'static str, value: &str) -> Result<(), SchedulerContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(SchedulerContractError::MissingField { field });
    }
    if trimmed.len() > MAX_DIAGNOSTIC_TEXT_LEN {
        return Err(SchedulerContractError::FieldTooLong {
            field,
            max_len: MAX_DIAGNOSTIC_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(SchedulerContractError::InvalidText { field });
    }
    Ok(())
}
