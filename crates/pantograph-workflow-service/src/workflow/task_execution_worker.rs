use super::WorkflowSchedulerTaskExecutionClass;

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerCommand {
    ExecuteTaskAttempt(WorkflowTaskExecutionWorkerTaskAttemptCommand),
    Shutdown(WorkflowTaskExecutionWorkerShutdownCommand),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerTaskAttemptCommand {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) task_id: String,
    pub(super) execution_class: WorkflowSchedulerTaskExecutionClass,
    pub(super) timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerShutdownCommand {
    pub(super) reason: WorkflowTaskExecutionWorkerShutdownReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerShutdownReason {
    ServiceShutdown,
    QueueClosed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerOutcome {
    TaskTerminal(WorkflowTaskExecutionWorkerTerminalOutcome),
    TaskDeferred(WorkflowTaskExecutionWorkerDeferredOutcome),
    WorkerUnavailable(WorkflowTaskExecutionWorkerDiagnostic),
    ShutdownAccepted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerTerminalOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) task_id: String,
    pub(super) status: WorkflowTaskExecutionWorkerTerminalStatus,
    pub(super) diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerTerminalStatus {
    Completed,
    Failed,
    TimedOut,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerDeferredOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) task_id: String,
    pub(super) reason: WorkflowTaskExecutionWorkerDeferredReason,
    pub(super) diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerDeferredReason {
    DependencyReadinessPending,
    ResourceReservationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerDiagnostic {
    pub(super) code: WorkflowTaskExecutionWorkerDiagnosticCode,
    pub(super) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerDiagnosticCode {
    WorkerUnavailable,
    QueueClosed,
    ShutdownRequested,
    RuntimeDispatchTimedOut,
    RuntimeSupervisorCancelled,
    ReservationReconcileFailed,
    TaskLifecycleHandleReleaseFailed,
}

impl WorkflowTaskExecutionWorkerCommand {
    pub(super) fn execute_task_attempt(
        session_id: impl Into<String>,
        workflow_run_id: impl Into<String>,
        task_id: impl Into<String>,
        execution_class: WorkflowSchedulerTaskExecutionClass,
        timeout_ms: Option<u64>,
    ) -> Self {
        Self::ExecuteTaskAttempt(WorkflowTaskExecutionWorkerTaskAttemptCommand {
            session_id: session_id.into(),
            workflow_run_id: workflow_run_id.into(),
            task_id: task_id.into(),
            execution_class,
            timeout_ms,
        })
    }

    pub(super) const fn shutdown(reason: WorkflowTaskExecutionWorkerShutdownReason) -> Self {
        Self::Shutdown(WorkflowTaskExecutionWorkerShutdownCommand { reason })
    }
}

impl WorkflowTaskExecutionWorkerOutcome {
    pub(super) fn task_terminal(
        command: &WorkflowTaskExecutionWorkerTaskAttemptCommand,
        status: WorkflowTaskExecutionWorkerTerminalStatus,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::TaskTerminal(WorkflowTaskExecutionWorkerTerminalOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            task_id: command.task_id.clone(),
            status,
            diagnostics,
        })
    }

    pub(super) fn task_deferred(
        command: &WorkflowTaskExecutionWorkerTaskAttemptCommand,
        reason: WorkflowTaskExecutionWorkerDeferredReason,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::TaskDeferred(WorkflowTaskExecutionWorkerDeferredOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            task_id: command.task_id.clone(),
            reason,
            diagnostics,
        })
    }

    pub(super) fn worker_unavailable(
        message: impl Into<String>,
    ) -> WorkflowTaskExecutionWorkerOutcome {
        Self::WorkerUnavailable(WorkflowTaskExecutionWorkerDiagnostic {
            code: WorkflowTaskExecutionWorkerDiagnosticCode::WorkerUnavailable,
            message: message.into(),
        })
    }
}

impl WorkflowTaskExecutionWorkerDiagnostic {
    pub(super) fn new(
        code: WorkflowTaskExecutionWorkerDiagnosticCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowSchedulerTaskExecutionClass;

    #[test]
    fn execute_task_attempt_command_is_task_scoped() {
        let command = WorkflowTaskExecutionWorkerCommand::execute_task_attempt(
            "session-1",
            "run-1",
            "task-1",
            WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            Some(250),
        );

        let WorkflowTaskExecutionWorkerCommand::ExecuteTaskAttempt(command) = command else {
            panic!("expected task attempt command");
        };

        assert_eq!(command.session_id, "session-1");
        assert_eq!(command.workflow_run_id, "run-1");
        assert_eq!(command.task_id, "task-1");
        assert_eq!(
            command.execution_class,
            WorkflowSchedulerTaskExecutionClass::RuntimeInference
        );
        assert_eq!(command.timeout_ms, Some(250));
    }

    #[test]
    fn terminal_outcome_preserves_task_scope_and_diagnostics() {
        let command = WorkflowTaskExecutionWorkerTaskAttemptCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            timeout_ms: Some(1),
        };
        let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeDispatchTimedOut,
            "runtime dispatch timed out",
        );

        let outcome = WorkflowTaskExecutionWorkerOutcome::task_terminal(
            &command,
            WorkflowTaskExecutionWorkerTerminalStatus::TimedOut,
            vec![diagnostic.clone()],
        );

        let WorkflowTaskExecutionWorkerOutcome::TaskTerminal(outcome) = outcome else {
            panic!("expected terminal outcome");
        };

        assert_eq!(outcome.session_id, command.session_id);
        assert_eq!(outcome.workflow_run_id, command.workflow_run_id);
        assert_eq!(outcome.task_id, command.task_id);
        assert_eq!(
            outcome.status,
            WorkflowTaskExecutionWorkerTerminalStatus::TimedOut
        );
        assert_eq!(outcome.diagnostics, vec![diagnostic]);
    }

    #[test]
    fn deferred_outcome_is_typed_without_fallback_completion() {
        let command = WorkflowTaskExecutionWorkerTaskAttemptCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: "run-1".to_string(),
            task_id: "task-1".to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            timeout_ms: None,
        };
        let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::QueueClosed,
            "queue closed before admission",
        );

        let outcome = WorkflowTaskExecutionWorkerOutcome::task_deferred(
            &command,
            WorkflowTaskExecutionWorkerDeferredReason::ResourceReservationUnavailable,
            vec![diagnostic.clone()],
        );

        let WorkflowTaskExecutionWorkerOutcome::TaskDeferred(outcome) = outcome else {
            panic!("expected deferred outcome");
        };

        assert_eq!(
            outcome.reason,
            WorkflowTaskExecutionWorkerDeferredReason::ResourceReservationUnavailable
        );
        assert_eq!(outcome.diagnostics, vec![diagnostic]);
    }

    #[test]
    fn shutdown_command_carries_worker_lifecycle_reason() {
        let command = WorkflowTaskExecutionWorkerCommand::shutdown(
            WorkflowTaskExecutionWorkerShutdownReason::ServiceShutdown,
        );

        let WorkflowTaskExecutionWorkerCommand::Shutdown(command) = command else {
            panic!("expected shutdown command");
        };

        assert_eq!(
            command.reason,
            WorkflowTaskExecutionWorkerShutdownReason::ServiceShutdown
        );
    }
}
