use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::SchedulerContractError;
use crate::intent::{
    SchedulableTaskIntent, SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};

const MAX_ID_LEN: usize = 128;

/// Current contract version for durable scheduler queue state.
pub const SCHEDULER_QUEUE_STATE_CONTRACT_VERSION: u16 = 1;

macro_rules! scheduler_queue_id {
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

scheduler_queue_id!(SchedulerQueueTransitionId, "transition_id");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SchedulerQueueTaskState {
    Pending,
    Ready,
    Blocked,
    WaitingDependencyReadiness,
    WaitingResources,
    WaitingBatch,
    Running,
    PausedDeferred,
    RetryableFailed,
    TerminalFailed,
    Completed,
}

impl SchedulerQueueTaskState {
    fn can_transition_to(self, next: SchedulerQueueTaskState) -> bool {
        use SchedulerQueueTaskState::{Blocked, Completed, PausedDeferred, Pending, Ready};
        use SchedulerQueueTaskState::{RetryableFailed, Running, TerminalFailed, WaitingBatch};
        use SchedulerQueueTaskState::{WaitingDependencyReadiness, WaitingResources};

        match self {
            Pending => matches!(
                next,
                Ready | Blocked | WaitingDependencyReadiness | TerminalFailed
            ),
            Ready => matches!(
                next,
                WaitingResources | WaitingBatch | Running | PausedDeferred | TerminalFailed
            ),
            Blocked => matches!(
                next,
                WaitingDependencyReadiness | Ready | PausedDeferred | TerminalFailed
            ),
            WaitingDependencyReadiness => matches!(
                next,
                Ready | PausedDeferred | RetryableFailed | TerminalFailed
            ),
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
            RetryableFailed => matches!(
                next,
                Pending | Ready | WaitingDependencyReadiness | TerminalFailed
            ),
            TerminalFailed | Completed => false,
        }
    }
}

/// Durable queue record persisted for replay and recovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerQueueTaskRecord {
    #[serde(default = "default_scheduler_queue_state_contract_version")]
    pub contract_version: u16,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub task_intent: SchedulableTaskIntent,
    pub state: SchedulerQueueTaskState,
    pub state_version: u64,
    pub last_transition_id: SchedulerQueueTransitionId,
}

impl SchedulerQueueTaskRecord {
    /// Validates a raw persisted queue record before scheduler replay.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        if self.state_version == 0 {
            return Err(SchedulerContractError::InvalidField {
                field: "state_version",
                reason: "queue state version must be greater than zero",
            });
        }
        self.task_intent.validate()?;
        validate_correlation(
            self.workflow_id.as_ref(),
            self.workflow_run_id.as_ref(),
            self.node_id.as_ref(),
            self.task_id.as_ref(),
            &self.task_intent,
        )
    }
}

/// Idempotent queue transition event persisted for replay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct SchedulerQueueTransition {
    #[serde(default = "default_scheduler_queue_state_contract_version")]
    pub contract_version: u16,
    pub transition_id: SchedulerQueueTransitionId,
    pub workflow_id: SchedulerWorkflowId,
    pub workflow_run_id: SchedulerWorkflowRunId,
    pub node_id: SchedulerNodeId,
    pub task_id: SchedulerTaskId,
    pub task_intent: SchedulableTaskIntent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_previous_state: Option<SchedulerQueueTaskState>,
    pub next_state: SchedulerQueueTaskState,
}

impl SchedulerQueueTransition {
    /// Validates a raw queue transition before applying it to persisted state.
    pub fn validate(&self) -> Result<(), SchedulerContractError> {
        validate_contract_version(self.contract_version)?;
        self.task_intent.validate()?;
        validate_correlation(
            self.workflow_id.as_ref(),
            self.workflow_run_id.as_ref(),
            self.node_id.as_ref(),
            self.task_id.as_ref(),
            &self.task_intent,
        )?;
        if self.expected_previous_state.is_none()
            && self.next_state != SchedulerQueueTaskState::Pending
        {
            return Err(SchedulerContractError::InvalidField {
                field: "next_state",
                reason: "initial queue transition must create pending state",
            });
        }
        if let Some(previous) = self.expected_previous_state {
            validate_state_transition(previous, self.next_state)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum SchedulerQueueTransitionApplyResult {
    Applied(SchedulerQueueTaskRecord),
    AlreadyApplied(SchedulerQueueTaskRecord),
}

/// Pure contract logic for deterministic replay. Persistence, locking, and
/// worker lifecycle ownership belong to later infrastructure slices.
pub fn apply_scheduler_queue_transition(
    current: Option<&SchedulerQueueTaskRecord>,
    transition: SchedulerQueueTransition,
) -> Result<SchedulerQueueTransitionApplyResult, SchedulerContractError> {
    transition.validate()?;
    match current {
        None => apply_initial_transition(transition),
        Some(record) => apply_existing_transition(record, transition),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerQueueTaskRecord(SchedulerQueueTaskRecord);

impl ValidatedSchedulerQueueTaskRecord {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerQueueTaskRecord {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerQueueTaskRecord {
        self.0
    }
}

impl TryFrom<SchedulerQueueTaskRecord> for ValidatedSchedulerQueueTaskRecord {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerQueueTaskRecord) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedSchedulerQueueTransition(SchedulerQueueTransition);

impl ValidatedSchedulerQueueTransition {
    #[must_use]
    pub fn as_ref(&self) -> &SchedulerQueueTransition {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> SchedulerQueueTransition {
        self.0
    }
}

impl TryFrom<SchedulerQueueTransition> for ValidatedSchedulerQueueTransition {
    type Error = SchedulerContractError;

    fn try_from(value: SchedulerQueueTransition) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

fn apply_initial_transition(
    transition: SchedulerQueueTransition,
) -> Result<SchedulerQueueTransitionApplyResult, SchedulerContractError> {
    if transition.expected_previous_state.is_some() {
        return Err(SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "initial queue transition must not expect existing state",
        });
    }
    Ok(SchedulerQueueTransitionApplyResult::Applied(
        record_from_transition(transition, 1),
    ))
}

fn apply_existing_transition(
    record: &SchedulerQueueTaskRecord,
    transition: SchedulerQueueTransition,
) -> Result<SchedulerQueueTransitionApplyResult, SchedulerContractError> {
    record.validate()?;
    validate_same_task(record, &transition)?;
    if record.last_transition_id == transition.transition_id {
        if record.state != transition.next_state {
            return Err(SchedulerContractError::InvalidField {
                field: "transition_id",
                reason: "duplicate queue transition id must replay the same next state",
            });
        }
        return Ok(SchedulerQueueTransitionApplyResult::AlreadyApplied(
            record.clone(),
        ));
    }
    let Some(expected_previous_state) = transition.expected_previous_state else {
        return Err(SchedulerContractError::MissingField {
            field: "expected_previous_state",
        });
    };
    if record.state != expected_previous_state {
        return Err(SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "queue transition previous state must match persisted task state",
        });
    }
    validate_state_transition(record.state, transition.next_state)?;
    Ok(SchedulerQueueTransitionApplyResult::Applied(
        record_from_transition(transition, record.state_version + 1),
    ))
}

fn record_from_transition(
    transition: SchedulerQueueTransition,
    state_version: u64,
) -> SchedulerQueueTaskRecord {
    SchedulerQueueTaskRecord {
        contract_version: SCHEDULER_QUEUE_STATE_CONTRACT_VERSION,
        workflow_id: transition.workflow_id,
        workflow_run_id: transition.workflow_run_id,
        node_id: transition.node_id,
        task_id: transition.task_id,
        task_intent: transition.task_intent,
        state: transition.next_state,
        state_version,
        last_transition_id: transition.transition_id,
    }
}

fn validate_same_task(
    record: &SchedulerQueueTaskRecord,
    transition: &SchedulerQueueTransition,
) -> Result<(), SchedulerContractError> {
    if record.workflow_id != transition.workflow_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "queue transition workflow id must match persisted record",
        });
    }
    if record.workflow_run_id != transition.workflow_run_id {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "queue transition workflow run id must match persisted record",
        });
    }
    if record.node_id != transition.node_id {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "queue transition node id must match persisted record",
        });
    }
    if record.task_id != transition.task_id {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "queue transition task id must match persisted record",
        });
    }
    if record.task_intent != transition.task_intent {
        return Err(SchedulerContractError::InvalidField {
            field: "task_intent",
            reason: "queue transition task intent must match persisted record",
        });
    }
    Ok(())
}

fn validate_correlation(
    workflow_id: &str,
    workflow_run_id: &str,
    node_id: &str,
    task_id: &str,
    task_intent: &SchedulableTaskIntent,
) -> Result<(), SchedulerContractError> {
    if workflow_id != task_intent.workflow_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "queue task workflow id must match task intent",
        });
    }
    if workflow_run_id != task_intent.workflow_run_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "workflow_run_id",
            reason: "queue task workflow run id must match task intent",
        });
    }
    if node_id != task_intent.node_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "node_id",
            reason: "queue task node id must match task intent",
        });
    }
    if task_id != task_intent.task_id.as_ref() {
        return Err(SchedulerContractError::InvalidField {
            field: "task_id",
            reason: "queue task id must match task intent",
        });
    }
    Ok(())
}

fn validate_state_transition(
    previous: SchedulerQueueTaskState,
    next: SchedulerQueueTaskState,
) -> Result<(), SchedulerContractError> {
    if previous == next {
        return Err(SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "queue transition must advance to a different state",
        });
    }
    if previous.can_transition_to(next) {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "queue transition is not allowed from the previous state",
        })
    }
}

fn default_scheduler_queue_state_contract_version() -> u16 {
    SCHEDULER_QUEUE_STATE_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), SchedulerContractError> {
    if value == SCHEDULER_QUEUE_STATE_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler queue state contract version",
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
