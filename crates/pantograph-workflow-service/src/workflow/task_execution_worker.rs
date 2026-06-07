use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::scheduler::{
    unix_timestamp_ms, WorkflowSchedulerLifecycleComponentKind,
    WorkflowSchedulerLifecycleComponentRegistryHandle, WorkflowSchedulerLifecycleComponentState,
};

use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventClaimOutcome,
    WorkflowRuntimeBranchTaskEventClaimOwnerId, WorkflowRuntimeBranchTaskEventDiagnostic,
    WorkflowRuntimeBranchTaskEventDiagnosticCode, WorkflowRuntimeBranchTaskEventId,
    WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventRepository,
};
use super::{
    WorkflowOutputTarget, WorkflowRunResponse, WorkflowSchedulerTaskExecutionClass,
    WorkflowService, WorkflowServiceError,
};

const TASK_EXECUTION_WORKER_COMMAND_CAPACITY: usize = 64;
const RUNTIME_BRANCH_TASK_EVENT_CLAIM_LEASE_MS: u64 = 30_000;
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
}

impl WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
    pub(super) fn new(service: Arc<WorkflowService>) -> Self {
        Self { service }
    }

    pub(super) fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }
}

impl fmt::Debug for WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WorkflowTaskExecutionWorkerRuntimeBranchEnvironment")
            .field("service", &"<shared WorkflowService>")
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
                        let service = runtime_branch_environment.service();
                        observed_runtime_branch_commands.fetch_add(1, Ordering::SeqCst);
                        let outcome = claim_and_defer_runtime_branch_event(
                            service.as_ref(),
                            &request.command,
                        );
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

fn claim_and_defer_runtime_branch_event(
    service: &WorkflowService,
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
) -> WorkflowTaskExecutionWorkerOutcome {
    let now_ms = unix_timestamp_ms();
    let claimed = match claim_runtime_branch_task_event_for_worker(service, command, now_ms) {
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

    let deferred = defer_claimed_runtime_branch_task_event(
        service,
        &claimed.record.event_id,
        &claimed.claim,
        now_ms,
    );
    match deferred {
        Ok(record) => {
            let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime branch dispatch execution has not moved into the task-execution worker loop yet",
            );
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_deferred(
                command,
                WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::RuntimeDispatchUnavailable,
                vec![record.scheduler_task_id],
                vec![diagnostic],
            )
        }
        Err(diagnostic) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            "runtime branch task event defer failed after claim",
            vec![diagnostic],
        ),
    }
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

fn defer_claimed_runtime_branch_task_event(
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
        .defer(event_id, claim, now_ms)
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
    use crate::workflow::WorkflowSchedulerTaskExecutionClass;
    use std::time::Duration;

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
        let worker = WorkflowTaskExecutionWorker::spawn(
            scheduler_lifecycle,
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(Arc::clone(&service)),
        )
        .expect("spawn task execution worker");

        assert!(Arc::ptr_eq(
            &service,
            &worker.runtime_branch_environment_service()
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
    async fn task_execution_worker_claims_and_defers_runtime_branch_task_event() {
        let scheduler_lifecycle = scheduler_lifecycle();
        let service = Arc::new(WorkflowService::new());
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
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(Arc::clone(&service)),
        )
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
            completion_rx.await.expect("runtime branch completion")
        })
        .await
        .expect("runtime branch command should complete");
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(outcome) = outcome else {
            panic!("expected runtime branch deferred outcome");
        };
        assert_eq!(
            outcome.reason,
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::RuntimeDispatchUnavailable
        );
        assert_eq!(outcome.deferred_task_ids, vec!["image-task"]);
        assert_eq!(
            outcome.diagnostics,
            vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime branch dispatch execution has not moved into the task-execution worker loop yet",
            )]
        );
        let persisted = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .get(&event_id)
            .expect("runtime branch task event");
        assert_eq!(
            persisted.state,
            WorkflowRuntimeBranchTaskEventState::Deferred
        );
        assert!(persisted.deferred_at_ms.is_some());

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    fn scheduler_lifecycle() -> WorkflowSchedulerLifecycleComponentRegistryHandle {
        WorkflowSchedulerLifecycleComponentRegistryHandle::new(
            WorkflowSchedulerLifecycleOwnerId::parse("workflow-service.task-execution-worker.test")
                .expect("scheduler lifecycle owner id"),
        )
    }

    fn runtime_environment() -> WorkflowTaskExecutionWorkerRuntimeBranchEnvironment {
        WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(Arc::new(WorkflowService::new()))
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
