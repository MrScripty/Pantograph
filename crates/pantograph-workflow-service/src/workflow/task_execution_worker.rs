use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::scheduler::{
    unix_timestamp_ms, WorkflowSchedulerLifecycleComponentKind,
    WorkflowSchedulerLifecycleComponentRegistryHandle, WorkflowSchedulerLifecycleComponentState,
};

use super::runtime_branch_rehydration::{
    rehydrate_runtime_branch_execution_context, WorkflowRuntimeBranchRehydrationDiagnostic,
    WorkflowRuntimeBranchRehydrationDiagnosticCode,
};
use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventClaimOutcome,
    WorkflowRuntimeBranchTaskEventClaimOwnerId, WorkflowRuntimeBranchTaskEventDiagnostic,
    WorkflowRuntimeBranchTaskEventDiagnosticCode, WorkflowRuntimeBranchTaskEventId,
    WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventRepository,
};
use super::session_scheduler_runner::WorkflowPreDispatchPreparationBoundary;
use super::task_execution_owner::WorkflowTaskExecutionOwner;
use super::{
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunResponse,
    WorkflowSchedulerTaskExecutionClass, WorkflowService, WorkflowServiceError,
};

const TASK_EXECUTION_WORKER_COMMAND_CAPACITY: usize = 64;
const RUNTIME_BRANCH_TASK_EVENT_CLAIM_LEASE_MS: u64 = 30_000;
const RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS: u64 = 1_000;
const TASK_EXECUTION_WORKER_CLAIM_OWNER_ID: &str = "workflow-service.task-execution-worker";

#[derive(Debug)]
pub(super) struct WorkflowTaskExecutionWorker {
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    runtime_branch_environment: WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    command_tx: tokio::sync::mpsc::Sender<WorkflowTaskExecutionWorkerCommand>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    observed_task_attempt_commands: Arc<AtomicU64>,
    observed_runtime_branch_commands: Arc<AtomicU64>,
}

impl WorkflowTaskExecutionWorker {
    pub(super) fn spawn(
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
        runtime_branch_environment: WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    ) -> Result<Self, WorkflowServiceError> {
        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|_| {
            WorkflowServiceError::Internal(
                "task execution worker requires an active Tokio runtime".to_string(),
            )
        })?;
        Self::spawn_with_handle(
            scheduler_lifecycle,
            runtime_handle,
            runtime_branch_environment,
        )
    }

    pub(super) fn spawn_with_handle(
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
        runtime_handle: tokio::runtime::Handle,
        runtime_branch_environment: WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    ) -> Result<Self, WorkflowServiceError> {
        scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker,
                WorkflowSchedulerLifecycleComponentState::Running,
            )
            .map(|_record| ())?;

        let (command_tx, command_rx) =
            tokio::sync::mpsc::channel(TASK_EXECUTION_WORKER_COMMAND_CAPACITY);
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let observed_task_attempt_commands = Arc::new(AtomicU64::new(0));
        let observed_runtime_branch_commands = Arc::new(AtomicU64::new(0));
        let join_handle = runtime_handle.spawn(task_execution_worker_loop(
            scheduler_lifecycle.clone(),
            runtime_branch_environment.clone(),
            command_rx,
            shutdown_rx,
            Arc::clone(&observed_task_attempt_commands),
            Arc::clone(&observed_runtime_branch_commands),
        ));

        Ok(Self {
            scheduler_lifecycle,
            runtime_branch_environment,
            command_tx,
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
            observed_task_attempt_commands,
            observed_runtime_branch_commands,
        })
    }

    pub(super) fn try_enqueue(
        &self,
        command: WorkflowTaskExecutionWorkerCommand,
    ) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
        self.command_tx.try_send(command).map_err(|error| {
            WorkflowTaskExecutionWorkerOutcome::worker_unavailable(format!(
                "task execution worker command queue unavailable: {error}"
            ))
        })
    }

    pub(super) async fn shutdown(&self) -> Result<(), WorkflowServiceError> {
        self.mark_shutting_down_if_running()?;
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            join_handle.await.map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "task execution worker join failed during shutdown: {error}"
                ))
            })?;
        }
        self.mark_shutdown()
    }

    #[cfg(test)]
    pub(super) fn observed_task_attempt_command_count(&self) -> u64 {
        self.observed_task_attempt_commands.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(super) fn observed_runtime_branch_command_count(&self) -> u64 {
        self.observed_runtime_branch_commands.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(super) fn runtime_branch_environment_service(&self) -> Arc<WorkflowService> {
        self.runtime_branch_environment.service()
    }

    #[cfg(test)]
    pub(super) fn runtime_branch_environment_host(&self) -> Arc<dyn WorkflowHost> {
        self.runtime_branch_environment.host()
    }

    fn mark_shutting_down_if_running(&self) -> Result<(), WorkflowServiceError> {
        let current = self
            .scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)?;
        if current.state == WorkflowSchedulerLifecycleComponentState::Shutdown {
            return Ok(());
        }
        self.scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker,
                WorkflowSchedulerLifecycleComponentState::ShuttingDown,
            )
            .map(|_record| ())
    }

    fn mark_shutdown(&self) -> Result<(), WorkflowServiceError> {
        self.scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker,
                WorkflowSchedulerLifecycleComponentState::Shutdown,
            )
            .map(|_record| ())
    }
}

#[derive(Clone)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
    service: Arc<WorkflowService>,
    host: Arc<dyn WorkflowHost>,
}

impl WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
    pub(super) fn new(service: Arc<WorkflowService>, host: Arc<dyn WorkflowHost>) -> Self {
        Self { service, host }
    }

    pub(super) fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }

    pub(super) fn host(&self) -> Arc<dyn WorkflowHost> {
        Arc::clone(&self.host)
    }
}

impl fmt::Debug for WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkflowTaskExecutionWorkerRuntimeBranchEnvironment")
            .field("service", &"<shared WorkflowService>")
            .field("host", &"<shared WorkflowHost>")
            .finish()
    }
}

#[derive(Debug)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerCommand {
    ExecuteTaskAttempt(WorkflowTaskExecutionWorkerTaskAttemptCommand),
    ExecuteRuntimeBranch(WorkflowTaskExecutionWorkerRuntimeBranchRequest),
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
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchCommand {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) workflow_id: String,
    pub(super) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(super) timeout_ms: Option<u64>,
    pub(super) start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason,
}

#[derive(Debug)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchRequest {
    pub(super) command: WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
}

#[derive(Debug)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder {
    completion_tx: tokio::sync::oneshot::Sender<WorkflowTaskExecutionWorkerOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerRuntimeBranchStartReason {
    Started,
    Redispatched,
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

#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerOutcome {
    TaskTerminal(WorkflowTaskExecutionWorkerTerminalOutcome),
    TaskDeferred(WorkflowTaskExecutionWorkerDeferredOutcome),
    RuntimeBranchCompleted(WorkflowTaskExecutionWorkerRuntimeBranchCompletedOutcome),
    RuntimeBranchFailed(WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome),
    RuntimeBranchDeferred(WorkflowTaskExecutionWorkerRuntimeBranchDeferredOutcome),
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

#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchCompletedOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) response: WorkflowRunResponse,
    pub(super) diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) error_message: String,
    pub(super) diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchDeferredOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) reason: WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason,
    pub(super) deferred_task_ids: Vec<String>,
    pub(super) diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason {
    DependencyReadinessPending,
    RuntimeDispatchUnavailable,
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
    RuntimeBranchEventUnavailable,
    RuntimeBranchEventClaimFailed,
    RuntimeBranchRehydrationFailed,
    RuntimeBranchDispatchUnavailable,
    RuntimeBranchFailed,
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

    pub(super) fn execute_runtime_branch(
        command: WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
    ) -> Self {
        Self::ExecuteRuntimeBranch(WorkflowTaskExecutionWorkerRuntimeBranchRequest {
            command,
            completion_responder,
        })
    }

    pub(super) const fn shutdown(reason: WorkflowTaskExecutionWorkerShutdownReason) -> Self {
        Self::Shutdown(WorkflowTaskExecutionWorkerShutdownCommand { reason })
    }
}

impl WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder {
    pub(super) fn channel() -> (
        Self,
        tokio::sync::oneshot::Receiver<WorkflowTaskExecutionWorkerOutcome>,
    ) {
        let (completion_tx, completion_rx) = tokio::sync::oneshot::channel();
        (Self { completion_tx }, completion_rx)
    }

    pub(super) fn complete(
        self,
        outcome: WorkflowTaskExecutionWorkerOutcome,
    ) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
        self.completion_tx.send(outcome)
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

    pub(super) fn runtime_branch_completed(
        command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        response: WorkflowRunResponse,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::RuntimeBranchCompleted(WorkflowTaskExecutionWorkerRuntimeBranchCompletedOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            response,
            diagnostics,
        })
    }

    pub(super) fn runtime_branch_failed(
        command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        error_message: impl Into<String>,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::RuntimeBranchFailed(WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            error_message: error_message.into(),
            diagnostics,
        })
    }

    pub(super) fn runtime_branch_deferred(
        command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        reason: WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason,
        deferred_task_ids: Vec<String>,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::RuntimeBranchDeferred(WorkflowTaskExecutionWorkerRuntimeBranchDeferredOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            reason,
            deferred_task_ids,
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

async fn task_execution_worker_loop(
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    runtime_branch_environment: WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    mut command_rx: tokio::sync::mpsc::Receiver<WorkflowTaskExecutionWorkerCommand>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    observed_task_attempt_commands: Arc<AtomicU64>,
    observed_runtime_branch_commands: Arc<AtomicU64>,
) {
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            maybe_command = command_rx.recv() => {
                match maybe_command {
                    Some(WorkflowTaskExecutionWorkerCommand::ExecuteTaskAttempt(_command)) => {
                        observed_task_attempt_commands.fetch_add(1, Ordering::SeqCst);
                    }
                    Some(WorkflowTaskExecutionWorkerCommand::ExecuteRuntimeBranch(request)) => {
                        observed_runtime_branch_commands.fetch_add(1, Ordering::SeqCst);
                        let outcome = claim_and_execute_runtime_branch_event(
                            &runtime_branch_environment,
                            &request.command,
                        ).await;
                        let _ = request.completion_responder.complete(outcome);
                    }
                    Some(WorkflowTaskExecutionWorkerCommand::Shutdown(_)) | None => {
                        break;
                    }
                }
            }
        }
    }

    let _ = scheduler_lifecycle.update_component_state(
        WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker,
        WorkflowSchedulerLifecycleComponentState::Shutdown,
    );
}

async fn claim_and_execute_runtime_branch_event(
    environment: &WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
) -> WorkflowTaskExecutionWorkerOutcome {
    let service = environment.service();
    let now_ms = unix_timestamp_ms();
    let claimed =
        match claim_runtime_branch_task_event_for_worker(service.as_ref(), command, now_ms) {
            Ok(Some(claimed)) => claimed,
            Ok(None) => {
                let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventUnavailable,
                    "no due runtime branch task event is available for workflow run",
                );
                return WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch task event is not available for worker claim",
                    vec![diagnostic],
                );
            }
            Err(diagnostic) => {
                return WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch task event claim failed",
                    vec![diagnostic],
                );
            }
        };

    let dispatching_record = match mark_claimed_runtime_branch_task_event_dispatching(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        now_ms,
    ) {
        Ok(record) => record,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event dispatching persistence failed",
                vec![diagnostic],
            );
        }
    };

    let active_run_inputs = match runtime_branch_active_run_inputs(service.as_ref(), command) {
        Ok(inputs) => inputs,
        Err(error) => {
            return fail_runtime_branch_preparation_error(
                command,
                service.as_ref(),
                &dispatching_record.event_id,
                &claimed.claim,
                error,
            );
        }
    };
    let preparation_boundary = WorkflowPreDispatchPreparationBoundary::new(service.as_ref());
    if let Err(error) = preparation_boundary.materialize_external_inputs(
        &command.session_id,
        &command.workflow_run_id,
        &active_run_inputs,
    ) {
        return fail_runtime_branch_preparation_error(
            command,
            service.as_ref(),
            &dispatching_record.event_id,
            &claimed.claim,
            error,
        );
    }
    let preparation = match preparation_boundary
        .prepare_runtime_dispatch(&command.session_id, &command.workflow_run_id)
        .await
    {
        Ok(preparation) => preparation,
        Err(error) if error.is_runtime_dependency_readiness_pending() => {
            return defer_runtime_branch_dependency_readiness(
                command,
                service.as_ref(),
                &dispatching_record.event_id,
                &claimed.claim,
                runtime_dependency_pending_task_ids(&error).unwrap_or_default(),
                error.to_string(),
            );
        }
        Err(error) => {
            return fail_runtime_branch_preparation_error(
                command,
                service.as_ref(),
                &dispatching_record.event_id,
                &claimed.claim,
                error,
            );
        }
    };
    if !preparation.deferred_task_ids().is_empty() {
        return defer_runtime_branch_dependency_readiness(
            command,
            service.as_ref(),
            &dispatching_record.event_id,
            &claimed.claim,
            preparation.deferred_task_ids().to_vec(),
            format!(
                "runtime dependency readiness is pending for scheduler task(s): {}",
                preparation.deferred_task_ids().join(", ")
            ),
        );
    }

    let rehydrated = match rehydrate_runtime_branch_execution_context(
        service.as_ref(),
        &dispatching_record,
        &claimed.claim,
    ) {
        Ok(context) => context,
        Err(diagnostic) => {
            let mut diagnostics = vec![runtime_branch_rehydration_diagnostic(diagnostic)];
            if let Err(release_diagnostic) = release_claimed_runtime_branch_task_event(
                service.as_ref(),
                &claimed.record.event_id,
                &claimed.claim,
                now_ms,
            ) {
                diagnostics.push(release_diagnostic);
            }
            return WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch execution context rehydration failed",
                diagnostics,
            );
        }
    };

    if let Err(diagnostic) = mark_claimed_runtime_branch_task_event_running(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        unix_timestamp_ms(),
    ) {
        return WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            "runtime branch task event running persistence failed",
            vec![diagnostic],
        );
    }

    let host = environment.host();
    let run_result =
        WorkflowTaskExecutionOwner::run_rehydrated_runtime_branch_until_dispatch_boundary(
            service.as_ref(),
            host.as_ref(),
            command,
            &rehydrated,
        )
        .await;

    match run_result {
        Ok(response) => match complete_claimed_runtime_branch_task_event(
            service.as_ref(),
            &claimed.record.event_id,
            &claimed.claim,
            unix_timestamp_ms(),
        ) {
            Ok(_record) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_completed(
                command,
                response,
                Vec::new(),
            ),
            Err(diagnostic) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event completion failed",
                vec![diagnostic],
            ),
        },
        Err(error) if error.is_runtime_dependency_readiness_pending() => {
            let deferred_task_ids = runtime_dependency_pending_task_ids(&error)
                .filter(|task_ids| !task_ids.is_empty())
                .unwrap_or_else(|| vec![rehydrated.runtime_task_id.clone()]);
            defer_runtime_branch_dependency_readiness(
                command,
                service.as_ref(),
                &claimed.record.event_id,
                &claimed.claim,
                deferred_task_ids,
                error.to_string(),
            )
        }
        Err(error) => match fail_claimed_runtime_branch_task_event(
            service.as_ref(),
            &claimed.record.event_id,
            &claimed.claim,
            unix_timestamp_ms(),
        ) {
            Ok(_record) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                error.to_string(),
                vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                    error.to_string(),
                )],
            ),
            Err(diagnostic) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event failure persistence failed",
                vec![diagnostic],
            ),
        },
    }
}

fn runtime_dependency_pending_task_ids(error: &WorkflowServiceError) -> Option<Vec<String>> {
    match error {
        WorkflowServiceError::RuntimeDependencyReadinessPending { task_ids, .. } => {
            Some(task_ids.clone())
        }
        WorkflowServiceError::WithDiagnostics { source, .. }
        | WorkflowServiceError::WithRuntimeDiagnosticPhase { source, .. } => {
            runtime_dependency_pending_task_ids(source)
        }
        _ => None,
    }
}

fn runtime_branch_active_run_inputs(
    service: &WorkflowService,
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
    let store = service.session_store_guard()?;
    Ok(store
        .active_run_context(&command.session_id, &command.workflow_run_id)?
        .inputs)
}

fn fail_runtime_branch_preparation_error(
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    error: WorkflowServiceError,
) -> WorkflowTaskExecutionWorkerOutcome {
    match fail_claimed_runtime_branch_task_event(service, event_id, claim, unix_timestamp_ms()) {
        Ok(_record) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            error.to_string(),
            vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                error.to_string(),
            )],
        ),
        Err(diagnostic) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            "runtime branch task event failure persistence failed",
            vec![diagnostic],
        ),
    }
}

fn defer_runtime_branch_dependency_readiness(
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    deferred_task_ids: Vec<String>,
    message: String,
) -> WorkflowTaskExecutionWorkerOutcome {
    let mut diagnostics = vec![WorkflowTaskExecutionWorkerDiagnostic::new(
        WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
        message,
    )];
    let deferred_at_ms = unix_timestamp_ms();
    let retry_ready_at_ms =
        deferred_at_ms.saturating_add(RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS);
    match defer_claimed_runtime_branch_task_event(
        service,
        event_id,
        claim,
        deferred_at_ms,
        retry_ready_at_ms,
    ) {
        Ok(_record) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_deferred(
            command,
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending,
            deferred_task_ids,
            diagnostics,
        ),
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event defer persistence failed",
                diagnostics,
            )
        }
    }
}

fn runtime_branch_rehydration_diagnostic(
    diagnostic: WorkflowRuntimeBranchRehydrationDiagnostic,
) -> WorkflowTaskExecutionWorkerDiagnostic {
    let code = match diagnostic.code {
        WorkflowRuntimeBranchRehydrationDiagnosticCode::ClaimMismatch
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::ActiveRunUnavailable
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskStateUnavailable
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskRunSummaryInvalid
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::RuntimeTaskUnavailable
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptUnavailable
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::TaskAttemptSourceContextInvalid
        | WorkflowRuntimeBranchRehydrationDiagnosticCode::CorrelationMismatch => {
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchRehydrationFailed
        }
    };
    WorkflowTaskExecutionWorkerDiagnostic::new(
        code,
        format!(
            "runtime branch rehydration diagnostic ({:?}): {}",
            diagnostic.code, diagnostic.message
        ),
    )
}

fn claim_runtime_branch_task_event_for_worker(
    service: &WorkflowService,
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    now_ms: u64,
) -> Result<Option<WorkflowRuntimeBranchTaskEventClaimOutcome>, WorkflowTaskExecutionWorkerDiagnostic>
{
    let owner_id =
        WorkflowRuntimeBranchTaskEventClaimOwnerId::parse(TASK_EXECUTION_WORKER_CLAIM_OWNER_ID)
            .map_err(runtime_branch_event_diagnostic)?;
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .claim_next_due_for_workflow_run(
            &command.workflow_run_id,
            owner_id,
            now_ms,
            RUNTIME_BRANCH_TASK_EVENT_CLAIM_LEASE_MS,
        )
        .map_err(runtime_branch_event_diagnostic)
}

fn release_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .release_claim(event_id, claim, now_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn defer_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    deferred_at_ms: u64,
    ready_at_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .defer_until(event_id, claim, deferred_at_ms, ready_at_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn mark_claimed_runtime_branch_task_event_dispatching(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .mark_dispatching(event_id, claim, now_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn mark_claimed_runtime_branch_task_event_running(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .mark_running(event_id, claim, now_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn complete_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .complete(event_id, claim, now_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn fail_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
) -> Result<WorkflowRuntimeBranchTaskEventRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_branch_task_event_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed,
                "runtime branch task-event repository lock poisoned",
            )
        })?;
    repository
        .fail(event_id, claim, now_ms)
        .map_err(runtime_branch_event_diagnostic)
}

fn runtime_branch_event_diagnostic(
    diagnostic: WorkflowRuntimeBranchTaskEventDiagnostic,
) -> WorkflowTaskExecutionWorkerDiagnostic {
    let code = match diagnostic.code {
        WorkflowRuntimeBranchTaskEventDiagnosticCode::DuplicateEvent
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::EventNotFound
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidEvent
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::MissingClaim
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::StaleClaim => {
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventClaimFailed
        }
        WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::LeaseExpired
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent
        | WorkflowRuntimeBranchTaskEventDiagnosticCode::InvalidTransition => {
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventUnavailable
        }
    };
    WorkflowTaskExecutionWorkerDiagnostic::new(
        code,
        format!(
            "runtime branch task-event diagnostic ({:?}): {}",
            diagnostic.code, diagnostic.message
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{
        WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
        WorkflowSchedulerLifecycleComponentState, WorkflowSchedulerLifecycleOwnerId,
    };
    use crate::workflow::runtime_branch_task_event::{
        WorkflowRuntimeBranchTaskEventRequest, WorkflowRuntimeBranchTaskEventState,
    };
    use crate::workflow::{
        WorkflowExecutionSessionRunRequest, WorkflowPortBinding, WorkflowRunHandle,
        WorkflowRunOptions, WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
        WorkflowSchedulerTaskGraph, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateRecord,
        SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };
    use std::time::Duration;

    struct WorkerHost;

    #[async_trait::async_trait]
    impl WorkflowHost for WorkerHost {
        async fn run_workflow(
            &self,
            _workflow_id: &str,
            _inputs: &[WorkflowPortBinding],
            _output_targets: Option<&[WorkflowOutputTarget]>,
            _run_options: WorkflowRunOptions,
            _run_handle: WorkflowRunHandle,
        ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
            Err(WorkflowServiceError::Internal(
                "task execution worker test host should not execute workflows".to_string(),
            ))
        }
    }

    #[tokio::test]
    async fn task_execution_worker_marks_running_until_shutdown() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let worker =
            WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle.clone(), runtime_environment())
                .expect("spawn task execution worker");

        assert_eq!(
            scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)
                .expect("task execution worker component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Running
        );

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");

        assert_eq!(
            scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)
                .expect("task execution worker component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Shutdown
        );
    }

    #[tokio::test]
    async fn task_execution_worker_observes_task_attempt_command_without_executing_task() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let worker = WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle, runtime_environment())
            .expect("spawn task execution worker");

        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_task_attempt(
                "session-1",
                "run-1",
                "task-1",
                WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                Some(500),
            ))
            .expect("enqueue task attempt command");

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if worker.observed_task_attempt_command_count() > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("worker should observe command");

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[tokio::test]
    async fn task_execution_worker_shutdown_is_idempotent() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let worker =
            WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle.clone(), runtime_environment())
                .expect("spawn task execution worker");

        worker
            .shutdown()
            .await
            .expect("first shutdown should complete");
        worker
            .shutdown()
            .await
            .expect("second shutdown should complete");

        assert_eq!(
            scheduler_lifecycle
                .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)
                .expect("task execution worker component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Shutdown
        );
    }

    #[test]
    fn task_execution_worker_spawn_requires_active_tokio_runtime() {
        let error =
            WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle(), runtime_environment())
                .expect_err("task execution worker spawn should require runtime");

        assert!(
            error
                .to_string()
                .contains("task execution worker requires an active Tokio runtime"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn task_execution_worker_owns_runtime_branch_environment() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let service = Arc::new(WorkflowService::new());
        let host = test_host();
        let worker = WorkflowTaskExecutionWorker::spawn(
            scheduler_lifecycle,
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
                Arc::clone(&service),
                Arc::clone(&host),
            ),
        )
        .expect("spawn task execution worker");

        assert!(Arc::ptr_eq(
            &service,
            &worker.runtime_branch_environment_service()
        ));
        assert!(Arc::ptr_eq(
            &host,
            &worker.runtime_branch_environment_host()
        ));

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[tokio::test]
    async fn task_execution_worker_observes_runtime_branch_command_with_owned_environment() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let worker = WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle, runtime_environment())
            .expect("spawn task execution worker");
        let (completion_responder, completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();

        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command(),
                completion_responder,
            ))
            .expect("enqueue runtime branch command");

        let outcome = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if worker.observed_runtime_branch_command_count() > 0 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            completion_rx.await.expect("runtime branch completion")
        })
        .await
        .expect("worker should observe runtime branch command");
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = outcome else {
            panic!("expected fail-closed runtime branch outcome");
        };
        assert_eq!(
            outcome.diagnostics,
            vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventUnavailable,
                "no due runtime branch task event is available for workflow run",
            )]
        );
        assert!(
            outcome
                .error_message
                .contains("not available for worker claim"),
            "unexpected error message: {}",
            outcome.error_message
        );

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[tokio::test]
    async fn task_execution_worker_executes_runtime_branch_and_fails_invalid_dispatch_state_before_running(
    ) {
        let scheduler_lifecycle = scheduler_lifecycle();
        let service = Arc::new(WorkflowService::new());
        let session_id = prepare_active_runtime_run(service.as_ref());
        let event_id =
            WorkflowRuntimeBranchTaskEventId::parse("runtime-branch-task-event.run-1.image-task")
                .expect("event id");
        let record =
            WorkflowRuntimeBranchTaskEventRecord::ready(WorkflowRuntimeBranchTaskEventRequest {
                event_id: event_id.clone(),
                session_id: session_id.clone(),
                workflow_id: "workflow-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                scheduler_task_id: "image-task".to_string(),
                scheduler_task_attempt_id: None,
                attempt_generation: 1,
                queued_input_keys: vec!["prompt:prompt".to_string()],
                output_targets: None,
                timeout_ms: Some(500),
                batching_key: Some("runtime-branch-task.workflow-1.image-task".to_string()),
                batch_eligibility: None,
                ready_at_ms: unix_timestamp_ms().saturating_sub(1),
            })
            .expect("runtime branch task event record");
        service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .enqueue(record)
            .expect("enqueue runtime branch task event");
        let worker = WorkflowTaskExecutionWorker::spawn(
            scheduler_lifecycle,
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
                Arc::clone(&service),
                test_host(),
            ),
        )
        .expect("spawn task execution worker");
        let (completion_responder, completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id,
            workflow_run_id: "run-1".to_string(),
            workflow_id: "workflow-1".to_string(),
            output_targets: None,
            timeout_ms: Some(500),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Redispatched,
        };

        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                command,
                completion_responder,
            ))
            .expect("enqueue runtime branch command");

        let outcome = tokio::time::timeout(Duration::from_secs(1), async {
            completion_rx.await.expect("runtime branch completion")
        })
        .await
        .expect("runtime branch command should complete");
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = outcome else {
            panic!("expected runtime branch failed outcome: {outcome:?}");
        };
        assert!(
            outcome
                .error_message
                .contains("runtime scheduler task 'image-task' was not admitted for dispatch"),
            "unexpected error: {}",
            outcome.error_message
        );
        assert_eq!(outcome.diagnostics.len(), 1);
        assert_eq!(
            outcome.diagnostics[0].code,
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed
        );
        assert!(
            outcome.diagnostics[0]
                .message
                .contains("runtime scheduler task 'image-task' was not admitted for dispatch"),
            "unexpected diagnostic: {}",
            outcome.diagnostics[0].message
        );
        let persisted = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .get(&event_id)
            .expect("runtime branch task event");
        assert_eq!(persisted.state, WorkflowRuntimeBranchTaskEventState::Failed);
        assert!(persisted.claim.is_some());
        assert!(persisted.dispatching_at_ms.is_some());
        assert!(persisted.running_at_ms.is_none());
        assert!(persisted.deferred_at_ms.is_none());
        assert!(persisted.failed_at_ms.is_some());

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[test]
    fn runtime_branch_dependency_defer_sets_retry_ready_time() {
        let service = WorkflowService::new();
        let event_id =
            WorkflowRuntimeBranchTaskEventId::parse("runtime-branch-task-event.run-1.image-task")
                .expect("event id");
        let record =
            WorkflowRuntimeBranchTaskEventRecord::ready(WorkflowRuntimeBranchTaskEventRequest {
                event_id: event_id.clone(),
                session_id: "session-1".to_string(),
                workflow_id: "workflow-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                scheduler_task_id: "image-task".to_string(),
                scheduler_task_attempt_id: None,
                attempt_generation: 1,
                queued_input_keys: vec!["prompt:prompt".to_string()],
                output_targets: None,
                timeout_ms: Some(500),
                batching_key: Some("runtime-branch-task.workflow-1.image-task".to_string()),
                batch_eligibility: None,
                ready_at_ms: 100,
            })
            .expect("runtime branch task event record");
        service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .enqueue(record)
            .expect("enqueue runtime branch task event");
        let claimed = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .claim_event(
                &event_id,
                WorkflowRuntimeBranchTaskEventClaimOwnerId::parse(
                    TASK_EXECUTION_WORKER_CLAIM_OWNER_ID,
                )
                .expect("owner id"),
                110,
                80,
            )
            .expect("event claims");
        let dispatching = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .mark_dispatching(&event_id, &claimed.claim, 120)
            .expect("event marks dispatching");

        let deferred = defer_claimed_runtime_branch_task_event(
            &service,
            &dispatching.event_id,
            &claimed.claim,
            130,
            130_u64.saturating_add(RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS),
        )
        .expect("event defers");

        assert_eq!(
            deferred.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert_eq!(deferred.deferred_at_ms, Some(130));
        assert_eq!(
            deferred.ready_at_ms,
            130 + RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS
        );
        assert!(deferred.claim.is_none());
    }

    fn prepare_active_runtime_run(service: &WorkflowService) -> String {
        let mut store = service.session_store_guard().expect("session store");
        let session_id = store
            .create_session(
                "workflow-1".to_string(),
                None,
                None,
                vec!["pytorch".to_string()],
                vec!["stable-diffusion-xl".to_string()],
                true,
            )
            .expect("create session");
        let run_request = WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: None,
            override_selection: None,
            timeout_ms: Some(500),
            priority: None,
        };
        let workflow_run_id = store
            .enqueue_run_with_id(&session_id, &run_request, "run-1".to_string())
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &workflow_run_id)
            .expect("begin queued run")
            .expect("dequeued run");
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                &workflow_run_id,
                runtime_task_graph(),
                vec![runtime_task_record()],
            )
            .expect("set runtime task state");
        session_id
    }

    fn runtime_task_graph() -> WorkflowSchedulerTaskGraph {
        let workflow_id = SchedulerWorkflowId::parse("workflow-1").expect("workflow id");
        let workflow_run_id = SchedulerWorkflowRunId::parse("run-1").expect("workflow run id");
        let task_id = SchedulerTaskId::parse("image-task").expect("task id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id,
                workflow_run_id,
                node_id: SchedulerNodeId::parse("image-task").expect("node id"),
                task_id,
                node_type: "llm-inference".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                diagnostics: Vec::new(),
            }],
        }
    }

    fn runtime_task_record() -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-1").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run-1").expect("workflow run id"),
            node_id: SchedulerNodeId::parse("image-task").expect("node id"),
            task_id: SchedulerTaskId::parse("image-task").expect("task id"),
            state: SchedulerTaskState::AwaitingInputs {
                diagnostics: Vec::new(),
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("transition.initial")
                .expect("transition id"),
        }
    }

    fn scheduler_lifecycle() -> WorkflowSchedulerLifecycleComponentRegistryHandle {
        WorkflowSchedulerLifecycleComponentRegistryHandle::new(
            WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.task-execution-worker.test")
                .expect("scheduler lifecycle owner id"),
        )
    }

    fn runtime_environment() -> WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
        WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
            Arc::new(WorkflowService::new()),
            test_host(),
        )
    }

    fn test_host() -> Arc<dyn WorkflowHost> {
        Arc::new(WorkerHost)
    }

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
    fn execute_runtime_branch_command_is_run_scoped() {
        let output_targets = vec![WorkflowOutputTarget {
            node_id: "image-output".to_string(),
            port_id: "image".to_string(),
        }];
        let branch_command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: "run-1".to_string(),
            workflow_id: "workflow-1".to_string(),
            output_targets: Some(output_targets.clone()),
            timeout_ms: Some(1_000),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
        };
        let (completion_responder, _completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let command = WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
            branch_command,
            completion_responder,
        );

        let WorkflowTaskExecutionWorkerCommand::ExecuteRuntimeBranch(request) = command else {
            panic!("expected runtime branch command");
        };
        let command = request.command;

        assert_eq!(command.session_id, "session-1");
        assert_eq!(command.workflow_run_id, "run-1");
        assert_eq!(command.workflow_id, "workflow-1");
        assert_eq!(command.output_targets, Some(output_targets));
        assert_eq!(command.timeout_ms, Some(1_000));
        assert_eq!(
            command.start_reason,
            WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started
        );
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
    fn runtime_branch_completed_outcome_preserves_run_scope_and_response() {
        let command = runtime_branch_command();
        let response = WorkflowRunResponse {
            workflow_run_id: command.workflow_run_id.clone(),
            outputs: Vec::new(),
            timing_ms: 42,
        };
        let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
            "non-fatal runtime branch diagnostic",
        );

        let outcome = WorkflowTaskExecutionWorkerOutcome::runtime_branch_completed(
            &command,
            response.clone(),
            vec![diagnostic.clone()],
        );

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(outcome) = outcome else {
            panic!("expected runtime branch completed outcome");
        };

        assert_eq!(outcome.session_id, command.session_id);
        assert_eq!(outcome.workflow_run_id, command.workflow_run_id);
        assert_eq!(outcome.response, response);
        assert_eq!(outcome.diagnostics, vec![diagnostic]);
    }

    #[test]
    fn runtime_branch_failed_outcome_preserves_typed_diagnostics() {
        let command = runtime_branch_command();
        let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeDispatchTimedOut,
            "runtime branch dispatch timed out",
        );

        let outcome = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            &command,
            "runtime branch failed",
            vec![diagnostic.clone()],
        );

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = outcome else {
            panic!("expected runtime branch failed outcome");
        };

        assert_eq!(outcome.session_id, command.session_id);
        assert_eq!(outcome.workflow_run_id, command.workflow_run_id);
        assert_eq!(outcome.error_message, "runtime branch failed");
        assert_eq!(outcome.diagnostics, vec![diagnostic]);
    }

    #[test]
    fn runtime_branch_deferred_outcome_preserves_pending_task_ids() {
        let command = runtime_branch_command();
        let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::QueueClosed,
            "dependency readiness still pending",
        );

        let outcome = WorkflowTaskExecutionWorkerOutcome::runtime_branch_deferred(
            &command,
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending,
            vec!["runtime-task-1".to_string()],
            vec![diagnostic.clone()],
        );

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(outcome) = outcome else {
            panic!("expected runtime branch deferred outcome");
        };

        assert_eq!(outcome.session_id, command.session_id);
        assert_eq!(outcome.workflow_run_id, command.workflow_run_id);
        assert_eq!(
            outcome.reason,
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending
        );
        assert_eq!(outcome.deferred_task_ids, vec!["runtime-task-1"]);
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

    fn runtime_branch_command() -> WorkflowTaskExecutionWorkerRuntimeBranchCommand {
        WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: "run-1".to_string(),
            workflow_id: "workflow-1".to_string(),
            output_targets: None,
            timeout_ms: Some(500),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Redispatched,
        }
    }
}
