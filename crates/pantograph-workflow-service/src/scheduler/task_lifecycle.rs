// The first durable lifecycle-supervisor slice lands the synchronous state
// owner before the session runner consumes it in the next lease/cancellation
// slices.
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationContext, RuntimeHostExecutionCancellationHandle,
    RuntimeHostExecutionCancellationSignal, RuntimeHostExecutionCancellationSnapshot,
    RuntimeHostExecutionCancellationState,
};
use pantograph_scheduler::SchedulerTaskId;
use tokio::task::AbortHandle;

use crate::workflow::WorkflowServiceError;

use super::{
    lifecycle::{
        WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRecord,
        WorkflowSchedulerLifecycleComponentRegistryHandle,
        WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
    },
    WorkflowSchedulerTaskAttemptId,
};

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
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
}

impl WorkflowSchedulerTaskLifecycleManager {
    pub(crate) fn new(owner_id: WorkflowSchedulerTaskLifecycleOwnerId) -> Self {
        let scheduler_lifecycle = WorkflowSchedulerLifecycleComponentRegistryHandle::new(
            WorkflowSchedulerLifecycleOwnerId::parse(owner_id.as_str())
                .expect("task lifecycle owner id must be valid scheduler lifecycle owner id"),
        );
        Self::new_with_scheduler_lifecycle(owner_id, scheduler_lifecycle)
    }

    pub(crate) fn new_with_scheduler_lifecycle(
        owner_id: WorkflowSchedulerTaskLifecycleOwnerId,
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    ) -> Self {
        Self {
            owner_id,
            shutdown_state: WorkflowSchedulerTaskLifecycleShutdownState::Running,
            active_task_handles: BTreeMap::new(),
            scheduler_lifecycle,
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

    pub(crate) fn runtime_host_dispatch_lifecycle_component(
        &self,
    ) -> Result<WorkflowSchedulerLifecycleComponentRecord, WorkflowServiceError> {
        self.scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch)
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
            runtime_host_cancellation_state: RuntimeHostExecutionCancellationState::Running,
            runtime_host_cancellation_reason: None,
            runtime_host_cancellation_signal: None,
            task_supervisor_abort_handle: None,
        };
        self.active_task_handles.insert(task_key, record.clone());
        Ok(record)
    }

    pub(crate) fn track_task_supervisor_abort_handle(
        &mut self,
        task_id: &SchedulerTaskId,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
        abort_handle: AbortHandle,
    ) -> Result<(), WorkflowServiceError> {
        let tracked = self.matching_task_handle_mut(task_id, attempt_id)?;
        tracked.task_supervisor_abort_handle = Some(abort_handle);
        self.scheduler_lifecycle.update_component_state(
            WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
            WorkflowSchedulerLifecycleComponentState::Running,
        )?;
        Ok(())
    }

    pub(crate) fn runtime_host_cancellation(
        &mut self,
        task_id: &SchedulerTaskId,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
        execution_request_id: impl AsRef<str>,
    ) -> Result<
        (
            RuntimeHostExecutionCancellationContext,
            RuntimeHostExecutionCancellationHandle,
        ),
        WorkflowServiceError,
    > {
        let tracked = self.matching_task_handle_mut(task_id, attempt_id)?;
        let cancellation_context =
            RuntimeHostExecutionCancellationContext::workflow_service(execution_request_id);
        let pending_state = tracked.runtime_host_cancellation_state.clone();
        let pending_reason = tracked.runtime_host_cancellation_reason.clone();
        let signal = tracked
            .runtime_host_cancellation_signal
            .get_or_insert_with(|| {
                Arc::new(WorkflowSchedulerTaskRuntimeHostCancellationSignal::new(
                    cancellation_context.cancellation_context_id.clone(),
                    pending_state,
                    pending_reason,
                ))
            })
            .clone();
        Ok((
            cancellation_context,
            RuntimeHostExecutionCancellationHandle::with_signal(signal),
        ))
    }

    pub(crate) fn request_task_cancellation(
        &mut self,
        task_id: &SchedulerTaskId,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
        reason: impl Into<String>,
    ) -> Result<(), WorkflowServiceError> {
        let tracked = self.matching_task_handle_mut(task_id, attempt_id)?;
        let reason = Some(reason.into());
        tracked.runtime_host_cancellation_state =
            RuntimeHostExecutionCancellationState::CancellationRequested;
        tracked.runtime_host_cancellation_reason = reason.clone();
        let Some(signal) = tracked.runtime_host_cancellation_signal.as_ref() else {
            return Ok(());
        };
        signal.update_state(
            RuntimeHostExecutionCancellationState::CancellationRequested,
            reason,
        )
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

        let completed = self
            .active_task_handles
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
            })?;
        self.refresh_runtime_host_dispatch_lifecycle_component()?;
        Ok(completed)
    }

    pub(crate) fn begin_shutdown(&mut self) -> WorkflowSchedulerTaskLifecycleShutdownState {
        if self.shutdown_state == WorkflowSchedulerTaskLifecycleShutdownState::Running {
            self.shutdown_state = WorkflowSchedulerTaskLifecycleShutdownState::ShuttingDown;
            let _ = self.scheduler_lifecycle.update_component_state(
                WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
                WorkflowSchedulerLifecycleComponentState::ShuttingDown,
            );
            for record in self.active_task_handles.values_mut() {
                let reason =
                    Some("workflow-service task lifecycle owner is shutting down".to_string());
                record.runtime_host_cancellation_state =
                    RuntimeHostExecutionCancellationState::ShutdownRequested;
                record.runtime_host_cancellation_reason = reason.clone();
                if let Some(signal) = record.runtime_host_cancellation_signal.as_ref() {
                    let _ = signal.update_state(
                        RuntimeHostExecutionCancellationState::ShutdownRequested,
                        reason,
                    );
                }
            }
        }
        self.shutdown_state
    }

    pub(crate) fn abort_task_supervisors(&mut self) -> usize {
        let mut aborted = 0;
        for record in self.active_task_handles.values_mut() {
            if let Some(abort_handle) = record.task_supervisor_abort_handle.as_ref() {
                abort_handle.abort();
                aborted += 1;
            }
        }
        aborted
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
        self.scheduler_lifecycle.update_component_state(
            WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
            WorkflowSchedulerLifecycleComponentState::Shutdown,
        )?;
        Ok(self.shutdown_state)
    }

    fn refresh_runtime_host_dispatch_lifecycle_component(
        &mut self,
    ) -> Result<(), WorkflowServiceError> {
        if self.shutdown_state != WorkflowSchedulerTaskLifecycleShutdownState::Running {
            return Ok(());
        }
        let has_runtime_supervisor = self
            .active_task_handles
            .values()
            .any(|record| record.task_supervisor_abort_handle.is_some());
        let state = if has_runtime_supervisor {
            WorkflowSchedulerLifecycleComponentState::Running
        } else {
            WorkflowSchedulerLifecycleComponentState::NotStarted
        };
        self.scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::RuntimeHostDispatch,
                state,
            )
            .map(|_record| ())
    }

    fn matching_task_handle_mut(
        &mut self,
        task_id: &SchedulerTaskId,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
    ) -> Result<&mut WorkflowSchedulerTaskLifecycleHandleRecord, WorkflowServiceError> {
        let tracked = self
            .active_task_handles
            .get_mut(task_id.as_str())
            .ok_or_else(|| {
                lifecycle_error(WorkflowSchedulerTaskLifecycleDiagnostic::error(
                    WorkflowSchedulerTaskLifecycleDiagnosticCode::TaskHandleNotTracked,
                    format!(
                        "task lifecycle handle is not tracked for task '{}'",
                        task_id.as_str()
                    ),
                    Some("Only tracked task attempts can receive lifecycle signals.".to_string()),
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
                    Some("Ignore stale lifecycle signal from older task attempts.".to_string()),
                ),
            ));
        }

        Ok(tracked)
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

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSchedulerTaskLifecycleHandleRecord {
    pub(crate) owner_id: WorkflowSchedulerTaskLifecycleOwnerId,
    pub(crate) task_id: SchedulerTaskId,
    pub(crate) attempt_id: WorkflowSchedulerTaskAttemptId,
    runtime_host_cancellation_state: RuntimeHostExecutionCancellationState,
    runtime_host_cancellation_reason: Option<String>,
    runtime_host_cancellation_signal:
        Option<Arc<WorkflowSchedulerTaskRuntimeHostCancellationSignal>>,
    task_supervisor_abort_handle: Option<AbortHandle>,
}

#[derive(Debug)]
struct WorkflowSchedulerTaskRuntimeHostCancellationSignal {
    snapshot: Mutex<RuntimeHostExecutionCancellationSnapshot>,
}

impl WorkflowSchedulerTaskRuntimeHostCancellationSignal {
    fn new(
        cancellation_context_id: String,
        state: RuntimeHostExecutionCancellationState,
        reason: Option<String>,
    ) -> Self {
        Self {
            snapshot: Mutex::new(RuntimeHostExecutionCancellationSnapshot {
                cancellation_context_id,
                state,
                reason,
            }),
        }
    }

    fn update_state(
        &self,
        state: RuntimeHostExecutionCancellationState,
        reason: Option<String>,
    ) -> Result<(), WorkflowServiceError> {
        let mut snapshot = self.snapshot.lock().map_err(|_error| {
            WorkflowServiceError::Internal(
                "scheduler task cancellation signal lock was poisoned".to_string(),
            )
        })?;
        snapshot.state = state;
        snapshot.reason = reason;
        snapshot
            .validate()
            .map_err(|error| WorkflowServiceError::Internal(error.to_string()))
    }
}

impl RuntimeHostExecutionCancellationSignal for WorkflowSchedulerTaskRuntimeHostCancellationSignal {
    fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_else(|_error| RuntimeHostExecutionCancellationSnapshot {
                cancellation_context_id: "runtime-host-cancellation.poisoned".to_string(),
                state: RuntimeHostExecutionCancellationState::ShutdownRequested,
                reason: Some("scheduler task cancellation signal lock was poisoned".to_string()),
            })
    }
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
