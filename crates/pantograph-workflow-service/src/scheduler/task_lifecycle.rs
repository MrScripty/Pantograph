// The first durable lifecycle-supervisor slice lands the synchronous state
// owner before the session runner consumes it in the next lease/cancellation
// slices.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;

use pantograph_scheduler::SchedulerTaskId;

use crate::workflow::WorkflowServiceError;

use super::WorkflowSchedulerTaskAttemptId;

/// Workflow-service task lifecycle owner.
///
/// This first slice is a synchronous state core. It owns task handle records
/// and shutdown state only; async task spawning, cancellation tokens, retry,
/// replay, and diagnostics-ledger writes belong to later slices.
#[derive(Debug, Clone)]
pub(crate) struct WorkflowSchedulerTaskLifecycleManager {
    owner_id: WorkflowSchedulerTaskLifecycleOwnerId,
    shutdown_state: WorkflowSchedulerTaskLifecycleShutdownState,
    active_task_handles: BTreeMap<String, WorkflowSchedulerTaskLifecycleHandleRecord>,
}

impl WorkflowSchedulerTaskLifecycleManager {
    pub(crate) fn new(owner_id: WorkflowSchedulerTaskLifecycleOwnerId) -> Self {
        Self {
            owner_id,
            shutdown_state: WorkflowSchedulerTaskLifecycleShutdownState::Running,
            active_task_handles: BTreeMap::new(),
        }
    }

    pub(crate) fn owner_id(&self) -> &WorkflowSchedulerTaskLifecycleOwnerId {
        &self.owner_id
    }

    pub(crate) fn shutdown_state(&self) -> WorkflowSchedulerTaskLifecycleShutdownState {
        self.shutdown_state
    }

    pub(crate) fn active_task_handle_count(&self) -> usize {
        self.active_task_handles.len()
    }

    pub(crate) fn active_task_handle(
        &self,
        task_id: &SchedulerTaskId,
    ) -> Option<&WorkflowSchedulerTaskLifecycleHandleRecord> {
        self.active_task_handles.get(task_id.as_str())
    }

    pub(crate) fn track_task_handle(
        &mut self,
        task_id: SchedulerTaskId,
        attempt_id: WorkflowSchedulerTaskAttemptId,
    ) -> Result<WorkflowSchedulerTaskLifecycleHandleRecord, WorkflowServiceError> {
        if self.shutdown_state != WorkflowSchedulerTaskLifecycleShutdownState::Running {
            return Err(lifecycle_error(
                WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::LifecycleOwnerShuttingDown,
                    "task lifecycle owner is shutting down",
                    Some(
                        "Start new task attempts only while the lifecycle owner is running."
                            .to_string(),
                    ),
                ),
            ));
        }

        let task_key = task_id.as_str().to_string();
        if self.active_task_handles.contains_key(&task_key) {
            return Err(lifecycle_error(
                WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::TaskHandleAlreadyTracked,
                    format!(
                        "task lifecycle handle is already tracked for task '{}'",
                        task_id.as_str()
                    ),
                    Some(
                        "Complete or cancel the existing attempt before tracking another handle."
                            .to_string(),
                    ),
                ),
            ));
        }

        let record = WorkflowSchedulerTaskLifecycleHandleRecord {
            owner_id: self.owner_id.clone(),
            task_id,
            attempt_id,
        };
        self.active_task_handles.insert(task_key, record.clone());
        Ok(record)
    }

    pub(crate) fn complete_task_handle(
        &mut self,
        task_id: &SchedulerTaskId,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
    ) -> Result<WorkflowSchedulerTaskLifecycleHandleRecord, WorkflowServiceError> {
        let tracked = self
            .active_task_handles
            .get(task_id.as_str())
            .ok_or_else(|| {
                lifecycle_error(WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::TaskHandleNotTracked,
                    format!(
                        "task lifecycle handle is not tracked for task '{}'",
                        task_id.as_str()
                    ),
                    Some("Only the active tracked attempt can complete a task handle.".to_string()),
                ))
            })?;

        if tracked.attempt_id != *attempt_id {
            return Err(lifecycle_error(
                WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::StaleTaskHandleAttempt,
                    format!(
                        "task lifecycle handle for task '{}' is owned by attempt '{}', not '{}'",
                        task_id.as_str(),
                        tracked.attempt_id.as_str(),
                        attempt_id.as_str()
                    ),
                    Some("Ignore stale completion from older task attempts.".to_string()),
                ),
            ));
        }

        self.active_task_handles
            .remove(task_id.as_str())
            .ok_or_else(|| {
                lifecycle_error(WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::TaskHandleNotTracked,
                    format!(
                        "task lifecycle handle is not tracked for task '{}'",
                        task_id.as_str()
                    ),
                    None,
                ))
            })
    }

    pub(crate) fn begin_shutdown(&mut self) -> WorkflowSchedulerTaskLifecycleShutdownState {
        if self.shutdown_state == WorkflowSchedulerTaskLifecycleShutdownState::Running {
            self.shutdown_state = WorkflowSchedulerTaskLifecycleShutdownState::ShuttingDown;
        }
        self.shutdown_state
    }

    pub(crate) fn finish_shutdown(
        &mut self,
    ) -> Result<WorkflowSchedulerTaskLifecycleShutdownState, WorkflowServiceError> {
        if !self.active_task_handles.is_empty() {
            return Err(lifecycle_error(
                WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::ActiveTaskHandlesRemain,
                    "task lifecycle owner cannot finish shutdown while handles remain active",
                    Some(
                        "Cancel or complete all tracked task attempts before final shutdown."
                            .to_string(),
                    ),
                ),
            ));
        }

        self.shutdown_state = WorkflowSchedulerTaskLifecycleShutdownState::Shutdown;
        Ok(self.shutdown_state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct WorkflowSchedulerTaskLifecycleOwnerId(String);

impl WorkflowSchedulerTaskLifecycleOwnerId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, WorkflowServiceError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(lifecycle_error(
                WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::InvalidLifecycleOwnerId,
                    "task lifecycle owner id must not be blank",
                    Some(
                        "Use the workflow-service lifecycle owner identity for this process."
                            .to_string(),
                    ),
                ),
            ));
        }

        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerTaskLifecycleShutdownState {
    Running,
    ShuttingDown,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSchedulerTaskLifecycleHandleRecord {
    pub(crate) owner_id: WorkflowSchedulerTaskLifecycleOwnerId,
    pub(crate) task_id: SchedulerTaskId,
    pub(crate) attempt_id: WorkflowSchedulerTaskAttemptId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerTaskLifecycleDiagnosticSeverity {
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSchedulerTaskLifecycleDiagnosticCode {
    InvalidLifecycleOwnerId,
    LifecycleOwnerShuttingDown,
    TaskHandleAlreadyTracked,
    TaskHandleNotTracked,
    StaleTaskHandleAttempt,
    ActiveTaskHandlesRemain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSchedulerTaskLifecycleDiagnostic {
    pub(crate) severity: WorkflowSchedulerTaskLifecycleDiagnosticSeverity,
    pub(crate) code: WorkflowSchedulerTaskLifecycleDiagnosticCode,
    pub(crate) message: String,
    pub(crate) hint: Option<String>,
}

impl WorkflowSchedulerTaskLifecycleDiagnostic {
    fn error(
        code: WorkflowSchedulerTaskLifecycleDiagnosticCode,
        message: impl Into<String>,
        hint: Option<String>,
    ) -> Self {
        Self {
            severity: WorkflowSchedulerTaskLifecycleDiagnosticSeverity::Error,
            code,
            message: message.into(),
            hint,
        }
    }
}

fn lifecycle_error(diagnostic: WorkflowSchedulerTaskLifecycleDiagnostic) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!(
        "task lifecycle error: {:?}: {}{}",
        diagnostic.code,
        diagnostic.message,
        diagnostic
            .hint
            .as_ref()
            .map(|hint| format!(" Hint: {hint}"))
            .unwrap_or_default()
    ))
}

#[cfg(test)]
#[path = "task_lifecycle_tests.rs"]
mod tests;
