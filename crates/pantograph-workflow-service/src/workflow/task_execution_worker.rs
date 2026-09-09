use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::scheduler::{
    unix_timestamp_ms, WorkflowSchedulerLifecycleComponentKind,
    WorkflowSchedulerLifecycleComponentRegistryHandle, WorkflowSchedulerLifecycleComponentState,
};

use super::runtime_branch_batch_execution::{
    WorkflowRuntimeBranchBatchClaimOwnership, WorkflowRuntimeBranchBatchExecutionDiagnostic,
    WorkflowRuntimeBranchBatchExecutionDiagnosticCode, WorkflowRuntimeBranchBatchExecutionFailure,
    WorkflowRuntimeBranchBatchExecutionMember, WorkflowRuntimeBranchBatchExecutionOwner,
    WorkflowRuntimeBranchBatchMemberExecutionOutcome,
    WorkflowRuntimeBranchBatchMemberExecutionOutcomeState,
    WorkflowRuntimeBranchBatchResponderFanOut,
};
use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchOwnedEventClaim, WorkflowRuntimeBranchTaskEventClaim,
    WorkflowRuntimeBranchTaskEventClaimOutcome, WorkflowRuntimeBranchTaskEventClaimOwnerId,
    WorkflowRuntimeBranchTaskEventDiagnostic, WorkflowRuntimeBranchTaskEventDiagnosticCode,
    WorkflowRuntimeBranchTaskEventId, WorkflowRuntimeBranchTaskEventRecord,
    WorkflowRuntimeBranchTaskEventRepository, WorkflowRuntimeBranchTaskEventState,
    WorkflowRuntimeClaimOwnership,
};
use super::runtime_dispatch_assignment::{
    WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest,
    WorkflowRuntimeDispatchAssignmentBatchBrokerRequest,
    WorkflowRuntimeDispatchAssignmentBatchClaim,
    WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
    WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
    WorkflowRuntimeDispatchAssignmentBatchReadyDecision,
    WorkflowRuntimeDispatchAssignmentDiagnostic, WorkflowRuntimeDispatchAssignmentDiagnosticCode,
    WorkflowRuntimeDispatchAssignmentId, WorkflowRuntimeDispatchAssignmentRecord,
    WorkflowRuntimeDispatchAssignmentRepository, WorkflowRuntimeDispatchAssignmentRequest,
    WorkflowRuntimeDispatchAssignmentState,
};
use super::runtime_dispatch_selection::WorkflowRuntimeDispatchCandidateFact;
use super::session_scheduler_runner::WorkflowPreDispatchPreparationBoundary;
use super::{
    WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunResponse,
    WorkflowSchedulerTaskExecutionClass, WorkflowService, WorkflowServiceError,
};

const TASK_EXECUTION_WORKER_COMMAND_CAPACITY: usize = 64;
const RUNTIME_BRANCH_TASK_EVENT_CLAIM_LEASE_MS: u64 = 30_000;
const RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS: u64 = 1_000;
const TASK_EXECUTION_WORKER_CLAIM_OWNER_ID: &str = "workflow-service.task-execution-worker";
const RUNTIME_BRANCH_BATCH_BROKER_MAX_ASSIGNMENTS: usize = 8;
const RUNTIME_BRANCH_BATCH_CLAIM_LEASE_MS: u64 = 1_000;
const RUNTIME_BRANCH_BATCH_CLAIM_OWNER_ID: &str =
    "workflow-service.task-execution-worker.batch-broker";

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
        if *self.shutdown_tx.borrow() {
            return Err(WorkflowTaskExecutionWorkerOutcome::worker_unavailable(
                "task execution worker is shutting down",
            ));
        }
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

#[derive(Clone, Default)]
struct WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry {
    responders: Arc<
        Mutex<
            BTreeMap<
                WorkflowTaskExecutionWorkerRuntimeBranchResponderKey,
                WorkflowTaskExecutionWorkerRuntimeBranchRegisteredResponder,
            >,
        >,
    >,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WorkflowTaskExecutionWorkerRuntimeBranchResponderKey(String);

struct WorkflowTaskExecutionWorkerRuntimeBranchRegisteredResponder {
    command: WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    session_id: String,
    workflow_run_id: String,
    workflow_id: String,
    runtime_dispatch_assignment_id: Option<WorkflowRuntimeDispatchAssignmentId>,
    completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
    event_ownership: Option<WorkflowRuntimeBranchOwnedEventClaim>,
    execution_task_id: Option<tokio::task::Id>,
    event_claim: Option<(
        WorkflowRuntimeBranchTaskEventId,
        WorkflowRuntimeBranchTaskEventClaim,
    )>,
    batch_claim: Option<WorkflowRuntimeDispatchAssignmentBatchClaim>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration {
    key: WorkflowTaskExecutionWorkerRuntimeBranchResponderKey,
    session_id: String,
    workflow_run_id: String,
    workflow_id: String,
    runtime_dispatch_assignment_id: Option<WorkflowRuntimeDispatchAssignmentId>,
}

#[derive(Debug)]
struct WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistrationFailure {
    completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
    outcome: WorkflowTaskExecutionWorkerOutcome,
}

struct WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
    assignment_id: WorkflowRuntimeDispatchAssignmentId,
    session_id: String,
    workflow_run_id: String,
    workflow_id: String,
    outcome: WorkflowTaskExecutionWorkerOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentRegistration {
    assignment_id: WorkflowRuntimeDispatchAssignmentId,
    session_id: String,
    workflow_run_id: String,
    workflow_id: String,
}

struct WorkflowRuntimeBranchContinuation {
    command: WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    registration: WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
}

enum WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult {
    Continue(Vec<WorkflowRuntimeBranchContinuation>),
    CompleteResponder(WorkflowTaskExecutionWorkerOutcome),
    ResponderRetainedForBatch,
}

impl WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult {
    fn complete(outcome: WorkflowTaskExecutionWorkerOutcome) -> Self {
        Self::CompleteResponder(outcome)
    }
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
    RuntimeBranchCancelled(WorkflowTaskExecutionWorkerRuntimeBranchCancelledOutcome),
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
pub(super) struct WorkflowTaskExecutionWorkerRuntimeBranchCancelledOutcome {
    pub(super) session_id: String,
    pub(super) workflow_run_id: String,
    pub(super) message: String,
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
    RuntimeBranchResponderRegistrationFailed,
    RuntimeBranchResponderUnavailable,
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

impl WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry {
    fn new() -> Self {
        Self::default()
    }

    fn register_workflow_run(
        &self,
        command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
    ) -> Result<
        WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
        WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistrationFailure,
    > {
        let key = WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::workflow_run(
            command.workflow_run_id.as_str(),
        );
        let registered = WorkflowTaskExecutionWorkerRuntimeBranchRegisteredResponder {
            command: command.clone(),
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            workflow_id: command.workflow_id.clone(),
            runtime_dispatch_assignment_id: None,
            completion_responder,
            event_ownership: None,
            execution_task_id: tokio::task::try_id(),
            event_claim: None,
            batch_claim: None,
        };
        let mut responders = match self.responders.lock() {
            Ok(responders) => responders,
            Err(_) => {
                return Err(
                    WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistrationFailure {
                        completion_responder: registered.completion_responder,
                        outcome: runtime_branch_responder_failure_outcome(
                            &command.session_id,
                            &command.workflow_run_id,
                            &command.workflow_id,
                            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed,
                            "runtime branch responder registry lock poisoned",
                        ),
                    },
                );
            }
        };
        if responders.contains_key(&key) {
            return Err(WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistrationFailure {
                completion_responder: registered.completion_responder,
                outcome: runtime_branch_responder_failure_outcome(
                    &command.session_id,
                    &command.workflow_run_id,
                    &command.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed,
                    format!(
                        "runtime branch responder is already registered for workflow run '{}'",
                        command.workflow_run_id
                    ),
                ),
            });
        }
        responders.insert(key.clone(), registered);
        Ok(
            WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration {
                key,
                session_id: command.session_id.clone(),
                workflow_run_id: command.workflow_run_id.clone(),
                workflow_id: command.workflow_id.clone(),
                runtime_dispatch_assignment_id: None,
            },
        )
    }

    fn complete(
        &self,
        registration: WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
        outcome: WorkflowTaskExecutionWorkerOutcome,
    ) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
        let registered = {
            let mut responders = self.responders.lock().map_err(|_| {
                runtime_branch_responder_failure_outcome(
                    &registration.session_id,
                    &registration.workflow_run_id,
                    &registration.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                    "runtime branch responder registry lock poisoned",
                )
            })?;
            responders.remove(&registration.key).ok_or_else(|| {
                runtime_branch_responder_failure_outcome(
                    &registration.session_id,
                    &registration.workflow_run_id,
                    &registration.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                    format!(
                        "runtime branch responder is not registered for workflow run '{}'",
                        registration.workflow_run_id
                    ),
                )
            })?
        };
        if registered.session_id != registration.session_id
            || registered.workflow_run_id != registration.workflow_run_id
            || registered.workflow_id != registration.workflow_id
        {
            return Err(runtime_branch_responder_failure_outcome(
                &registration.session_id,
                &registration.workflow_run_id,
                &registration.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                format!(
                    "runtime branch responder registration for workflow run '{}' changed before completion",
                    registration.workflow_run_id
                ),
            ));
        }
        registered.completion_responder.complete(outcome)
    }

    fn attach_runtime_dispatch_assignment(
        &self,
        registration: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
        event_ownership: Option<WorkflowRuntimeBranchOwnedEventClaim>,
    ) -> Result<
        WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
        WorkflowTaskExecutionWorkerOutcome,
    > {
        let assignment_key =
            WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                assignment_id,
            );
        let mut responders = self.responders.lock().map_err(|_| {
            runtime_branch_responder_failure_outcome(
                &registration.session_id,
                &registration.workflow_run_id,
                &registration.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                "runtime branch responder registry lock poisoned",
            )
        })?;
        if responders.contains_key(&assignment_key) {
            return Err(runtime_branch_responder_failure_outcome(
                &registration.session_id,
                &registration.workflow_run_id,
                &registration.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed,
                format!(
                    "runtime branch responder is already attached to dispatch assignment '{}'",
                    assignment_id.as_str()
                ),
            ));
        }
        let mut registered = responders.remove(&registration.key).ok_or_else(|| {
            runtime_branch_responder_failure_outcome(
                &registration.session_id,
                &registration.workflow_run_id,
                &registration.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                format!(
                    "runtime branch responder for workflow run '{}' is not registered for assignment attachment",
                    registration.workflow_run_id
                ),
            )
        })?;
        registered.runtime_dispatch_assignment_id = Some(assignment_id.clone());
        registered.event_ownership = event_ownership;
        responders.insert(assignment_key.clone(), registered);
        Ok(
            WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration {
                key: assignment_key,
                session_id: registration.session_id.clone(),
                workflow_run_id: registration.workflow_run_id.clone(),
                workflow_id: registration.workflow_id.clone(),
                runtime_dispatch_assignment_id: Some(assignment_id.clone()),
            },
        )
    }

    fn continue_workflow_run(
        &self,
        member: &WorkflowRuntimeBranchBatchMemberExecutionOutcome,
    ) -> Result<WorkflowRuntimeBranchContinuation, Box<WorkflowTaskExecutionWorkerOutcome>> {
        let failure = || {
            Box::new(runtime_branch_responder_failure_outcome(
                &member.session_id,
                &member.workflow_run_id,
                &member.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                "runtime branch responder unavailable for continuation",
            ))
        };
        let mut responders = self.responders.lock().map_err(|_| failure())?;
        let assignment_key =
            WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                &member.assignment_id,
            );
        let key = WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::workflow_run(
            &member.workflow_run_id,
        );
        if responders.contains_key(&key)
            || !responders.get(&assignment_key).is_some_and(|registered| {
                registered.session_id == member.session_id
                    && registered.workflow_id == member.workflow_id
                    && registered.workflow_run_id == member.workflow_run_id
                    && registered.execution_task_id == tokio::task::try_id()
            })
        {
            return Err(failure());
        }
        let mut registered = responders.remove(&assignment_key).ok_or_else(failure)?;
        registered.runtime_dispatch_assignment_id = None;
        registered.event_ownership = None;
        registered.event_claim = None;
        registered.batch_claim = None;
        let continuation = WorkflowRuntimeBranchContinuation {
            command: registered.command.clone(),
            registration: WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration {
                key: key.clone(),
                session_id: registered.session_id.clone(),
                workflow_run_id: registered.workflow_run_id.clone(),
                workflow_id: registered.workflow_id.clone(),
                runtime_dispatch_assignment_id: None,
            },
        };
        responders.insert(key, registered);
        Ok(continuation)
    }

    fn record_claim_identity(
        &self,
        registration: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
        event: &WorkflowRuntimeBranchOwnedEventClaim,
    ) {
        if let Ok(mut responders) = self.responders.lock() {
            if let Some(registered) = responders.get_mut(&registration.key) {
                registered.event_claim = Some((event.event_id.clone(), event.claim.clone()));
            }
        }
    }

    fn fail_registered_event(
        &self,
        service: &WorkflowService,
        registration: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
    ) -> Vec<WorkflowTaskExecutionWorkerDiagnostic> {
        match self.responders.lock() {
            Ok(responders) => responders
                .get(&registration.key)
                .map(|registered| settle_failed_registration(service, registered))
                .unwrap_or_default(),
            Err(_) => vec![settlement_diagnostic(
                "responder registry lock poisoned during failure settlement",
            )],
        }
    }

    fn supervise_task_exit(
        &self,
        service: &WorkflowService,
        task_id: tokio::task::Id,
        message: &str,
    ) {
        let pending = {
            let mut responders = self
                .responders
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let keys = responders
                .iter()
                .filter(|(_, registered)| registered.execution_task_id == Some(task_id))
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            keys.into_iter()
                .filter_map(|key| responders.remove(&key))
                .collect::<Vec<_>>()
        };
        for registered in pending {
            let diagnostics = settle_failed_registration(service, &registered);
            let mut outcome = runtime_branch_responder_failure_outcome(
                &registered.session_id,
                &registered.workflow_run_id,
                &registered.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                message,
            );
            if let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(failed) = &mut outcome {
                failed.diagnostics.extend(diagnostics);
            }
            let _ = registered.completion_responder.complete(outcome);
        }
    }

    fn complete_runtime_dispatch_assignments(
        &self,
        completions: Vec<WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion>,
    ) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
        if completions.is_empty() {
            return Ok(());
        }

        let mut seen_assignment_ids = BTreeSet::new();
        for completion in &completions {
            if !seen_assignment_ids.insert(completion.assignment_id.clone()) {
                return Err(runtime_branch_responder_failure_outcome(
                    &completion.session_id,
                    &completion.workflow_run_id,
                    &completion.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed,
                    format!(
                        "runtime branch responder assignment completion contains duplicate dispatch assignment '{}'",
                        completion.assignment_id.as_str()
                    ),
                ));
            }
        }

        let mut responders = self.responders.lock().map_err(|_| {
            let completion = completions
                .first()
                .expect("non-empty runtime branch responder completions");
            runtime_branch_responder_failure_outcome(
                &completion.session_id,
                &completion.workflow_run_id,
                &completion.workflow_id,
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                "runtime branch responder registry lock poisoned",
            )
        })?;

        for completion in &completions {
            let key =
                WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                    &completion.assignment_id,
                );
            let registered = responders.get(&key).ok_or_else(|| {
                runtime_branch_responder_failure_outcome(
                    &completion.session_id,
                    &completion.workflow_run_id,
                    &completion.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                    format!(
                        "runtime branch responder is not registered for dispatch assignment '{}'",
                        completion.assignment_id.as_str()
                    ),
                )
            })?;
            if registered.session_id != completion.session_id
                || registered.workflow_run_id != completion.workflow_run_id
                || registered.workflow_id != completion.workflow_id
            {
                return Err(runtime_branch_responder_failure_outcome(
                    &completion.session_id,
                    &completion.workflow_run_id,
                    &completion.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                    format!(
                        "runtime branch responder registration for dispatch assignment '{}' changed before completion",
                        completion.assignment_id.as_str()
                    ),
                ));
            }
        }

        let mut pending_notifications = Vec::with_capacity(completions.len());
        for completion in completions {
            let key =
                WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                    &completion.assignment_id,
                );
            let registered = responders.remove(&key).ok_or_else(|| {
                runtime_branch_responder_failure_outcome(
                    &completion.session_id,
                    &completion.workflow_run_id,
                    &completion.workflow_id,
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                    format!(
                        "runtime branch responder is not registered for dispatch assignment '{}'",
                        completion.assignment_id.as_str()
                    ),
                )
            })?;
            pending_notifications.push((registered.completion_responder, completion.outcome));
        }
        drop(responders);

        for (completion_responder, outcome) in pending_notifications {
            // Delivery cannot undo settlement or stop another batch member's continuation.
            let _ = completion_responder.complete(outcome);
        }
        Ok(())
    }

    fn runtime_dispatch_assignment_registrations(
        &self,
    ) -> Vec<WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentRegistration> {
        let Ok(responders) = self.responders.lock() else {
            return Vec::new();
        };
        responders
            .values()
            .filter_map(|registered| {
                registered
                    .runtime_dispatch_assignment_id
                    .as_ref()
                    .map(|assignment_id| {
                        WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentRegistration {
                            assignment_id: assignment_id.clone(),
                            session_id: registered.session_id.clone(),
                            workflow_run_id: registered.workflow_run_id.clone(),
                            workflow_id: registered.workflow_id.clone(),
                        }
                    })
            })
            .collect()
    }

    #[cfg(test)]
    fn active_responder_count(&self) -> usize {
        self.responders
            .lock()
            .expect("runtime branch responder registry lock")
            .len()
    }
}

impl WorkflowTaskExecutionWorkerRuntimeBranchResponderKey {
    fn workflow_run(workflow_run_id: &str) -> Self {
        Self(format!("workflow-run:{workflow_run_id}"))
    }

    fn runtime_dispatch_assignment(assignment_id: &WorkflowRuntimeDispatchAssignmentId) -> Self {
        Self(format!(
            "runtime-dispatch-assignment:{}",
            assignment_id.as_str()
        ))
    }
}

fn settlement_diagnostic(message: impl Into<String>) -> WorkflowTaskExecutionWorkerDiagnostic {
    WorkflowTaskExecutionWorkerDiagnostic::new(
        WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
        message,
    )
}

fn settle_failed_registration(
    service: &WorkflowService,
    registered: &WorkflowTaskExecutionWorkerRuntimeBranchRegisteredResponder,
) -> Vec<WorkflowTaskExecutionWorkerDiagnostic> {
    let now_ms = unix_timestamp_ms();
    let mut diagnostics = Vec::new();
    if let Some((event_id, claim)) = &registered.event_claim {
        match service.runtime_branch_task_event_repository.lock() {
            Ok(mut repository) => {
                let active = repository.get(event_id).is_some_and(|record| {
                    matches!(
                        record.state,
                        WorkflowRuntimeBranchTaskEventState::Claimed
                            | WorkflowRuntimeBranchTaskEventState::Dispatching
                            | WorkflowRuntimeBranchTaskEventState::Running
                    )
                });
                if active {
                    let result = if let Some(owned) = &registered.event_ownership {
                        repository
                            .fail(event_id, claim, now_ms, Some(&owned.proof))
                            .map(|_| ())
                    } else {
                        repository.fail_abandoned(event_id, claim, now_ms)
                    };
                    if let Err(error) = result {
                        diagnostics.push(runtime_branch_event_diagnostic(error));
                    }
                }
            }
            Err(_) => diagnostics.push(settlement_diagnostic(
                "task-event repository lock poisoned during failure settlement",
            )),
        }
    }
    match service.runtime_dispatch_assignment_repository.lock() {
        Ok(mut repository) => {
            if let Some(claim) = &registered.batch_claim {
                if let Err(error) = repository.fail_abandoned_batch(claim, now_ms) {
                    diagnostics.push(runtime_dispatch_assignment_diagnostic(error));
                }
            } else if let Some(assignment_id) = &registered.runtime_dispatch_assignment_id {
                if repository
                    .get(assignment_id)
                    .is_some_and(|record| !record.state.is_terminal())
                {
                    if let Err(error) = repository.mark_failed(assignment_id, now_ms) {
                        diagnostics.push(runtime_dispatch_assignment_diagnostic(error));
                    }
                }
            }
        }
        Err(_) => diagnostics.push(settlement_diagnostic(
            "assignment repository lock poisoned during failure settlement",
        )),
    }
    diagnostics
}

fn settle_failed_owned_assignment(
    service: &WorkflowService,
    assignment: &WorkflowRuntimeDispatchAssignmentRecord,
    ownership: &WorkflowRuntimeBranchBatchClaimOwnership,
) -> Result<(), WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_dispatch_assignment_repository
        .lock()
        .map_err(|_| {
            settlement_diagnostic(
                "assignment repository lock poisoned during owned failure settlement",
            )
        })?;
    let claim = assignment
        .batch_claim
        .as_ref()
        .ok_or_else(|| settlement_diagnostic("failed batch member has no batch fence"))?;
    let already_settled = repository
        .get(&assignment.assignment_id)
        .is_some_and(|record| {
            record.batch_claim.as_ref() == Some(claim) && record.state.is_terminal()
        });
    if !already_settled {
        let _record = repository
            .finish_owned_assignment(
                &assignment.assignment_id,
                claim,
                &ownership.batch,
                WorkflowRuntimeDispatchAssignmentState::Failed,
                unix_timestamp_ms(),
            )
            .map_err(runtime_dispatch_assignment_diagnostic)?;
    }
    Ok(())
}

fn runtime_branch_responder_failure_outcome(
    session_id: &str,
    workflow_run_id: &str,
    workflow_id: &str,
    code: WorkflowTaskExecutionWorkerDiagnosticCode,
    message: impl Into<String>,
) -> WorkflowTaskExecutionWorkerOutcome {
    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
            session_id: session_id.to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            error_message: format!(
                "runtime branch responder registry failed for workflow '{workflow_id}'"
            ),
            diagnostics: vec![WorkflowTaskExecutionWorkerDiagnostic::new(code, message)],
        },
    )
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

    pub(super) fn runtime_branch_cancelled(
        command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        message: impl Into<String>,
        diagnostics: Vec<WorkflowTaskExecutionWorkerDiagnostic>,
    ) -> Self {
        Self::RuntimeBranchCancelled(WorkflowTaskExecutionWorkerRuntimeBranchCancelledOutcome {
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            message: message.into(),
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

async fn drive_runtime_branch_continuations(
    environment: &WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    registry: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry,
    mut pending: Vec<WorkflowRuntimeBranchContinuation>,
) {
    while let Some(WorkflowRuntimeBranchContinuation {
        command,
        mut registration,
    }) = pending.pop()
    {
        let outcome = claim_and_execute_runtime_branch_event(
            environment,
            &command,
            registry,
            &mut registration,
        )
        .await;
        if let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations) =
            outcome
        {
            pending.extend(continuations);
        } else if let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::CompleteResponder(
            mut outcome,
        ) = outcome
        {
            if let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(failed) = &mut outcome {
                failed.diagnostics.extend(
                    registry.fail_registered_event(environment.service().as_ref(), &registration),
                );
            }
            if let Some(assignment_id) = registration.runtime_dispatch_assignment_id.clone() {
                let completion =
                    WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                        assignment_id,
                        session_id: registration.session_id,
                        workflow_run_id: registration.workflow_run_id,
                        workflow_id: registration.workflow_id,
                        outcome,
                    };
                let _ = registry.complete_runtime_dispatch_assignments(vec![completion]);
            } else {
                let _ = registry.complete(registration, outcome);
            }
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
    let mut runtime_branch_tasks = tokio::task::JoinSet::new();
    let runtime_branch_responder_registry =
        WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
    let mut accepting_commands = true;

    loop {
        if !accepting_commands && runtime_branch_tasks.is_empty() {
            break;
        }

        tokio::select! {
            changed = shutdown_rx.changed(), if accepting_commands => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    accepting_commands = false;
                }
            }
            maybe_command = command_rx.recv(), if accepting_commands => {
                match maybe_command {
                    Some(WorkflowTaskExecutionWorkerCommand::ExecuteTaskAttempt(_command)) => {
                        observed_task_attempt_commands.fetch_add(1, Ordering::SeqCst);
                    }
                    Some(WorkflowTaskExecutionWorkerCommand::ExecuteRuntimeBranch(request)) => {
                        observed_runtime_branch_commands.fetch_add(1, Ordering::SeqCst);
                        let runtime_branch_environment = runtime_branch_environment.clone();
                        let runtime_branch_responder_registry =
                            runtime_branch_responder_registry.clone();
                        runtime_branch_tasks.spawn(async move {
                            let WorkflowTaskExecutionWorkerRuntimeBranchRequest {
                                command,
                                completion_responder,
                            } = request;
                            let registration = match runtime_branch_responder_registry
                                .register_workflow_run(&command, completion_responder)
                            {
                                Ok(registration) => registration,
                                Err(failure) => {
                                    let _ = failure
                                        .completion_responder
                                        .complete(failure.outcome);
                                    return;
                                }
                            };
                            drive_runtime_branch_continuations(
                                &runtime_branch_environment, &runtime_branch_responder_registry,
                                vec![WorkflowRuntimeBranchContinuation { command, registration }],
                            ).await;
                        });
                    }
                    Some(WorkflowTaskExecutionWorkerCommand::Shutdown(_)) | None => {
                        accepting_commands = false;
                    }
                }
            }
            Some(join_result) = runtime_branch_tasks.join_next_with_id(), if !runtime_branch_tasks.is_empty() => {
                match join_result {
                    Ok((task_id, ())) => runtime_branch_responder_registry.supervise_task_exit(runtime_branch_environment.service().as_ref(), task_id, "runtime branch execution returned without settling its registered response"),
                    Err(error) => runtime_branch_responder_registry.supervise_task_exit(runtime_branch_environment.service().as_ref(), error.id(), &format!("runtime branch execution task failed: {error}")),
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
    runtime_branch_responder_registry: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry,
    runtime_branch_responder_registration: &mut WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistration,
) -> WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult {
    let service = environment.service();
    let preparation_boundary = WorkflowPreDispatchPreparationBoundary::new(service.as_ref());
    let preparation = async {
        let inputs = runtime_branch_active_run_inputs(service.as_ref(), command)?;
        preparation_boundary.materialize_external_inputs(
            &command.session_id,
            &command.workflow_run_id,
            &inputs,
        )?;
        preparation_boundary
            .prepare_runtime_dispatch(&command.session_id, &command.workflow_run_id)
            .await
    }
    .await;
    let preparation = match preparation {
        Ok(preparation) => preparation,
        Err(error) => {
            let outcome = if error.is_runtime_dependency_readiness_pending() {
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_deferred(command,
                    WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending,
                    runtime_dependency_pending_task_ids(&error).unwrap_or_default(),
                    vec![WorkflowTaskExecutionWorkerDiagnostic::new(WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable, error.to_string())])
            } else {
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    error.to_string(),
                    vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                        WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                        error.to_string(),
                    )],
                )
            };
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome);
        }
    };
    if preparation.all_tasks_completed() {
        let outcome = match super::runtime_branch_batch_execution::finalize_continued_scheduler_run(
            service.as_ref(),
            environment.host().as_ref(),
            &command.session_id,
            &command.workflow_run_id,
            &command.workflow_id,
        )
        .await
        {
            Ok(response) => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(
                WorkflowTaskExecutionWorkerRuntimeBranchCompletedOutcome {
                    session_id: command.session_id.clone(),
                    workflow_run_id: command.workflow_run_id.clone(),
                    response,
                    diagnostics: Vec::new(),
                },
            ),
            Err(error) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                error.to_string(),
                vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                    error.to_string(),
                )],
            ),
        };
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome);
    }
    let Some(task_id) = preparation.next_ready_task_id() else {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_deferred(
                command,
                WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending,
                preparation.deferred_task_ids().to_vec(),
                vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                    WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                    format!(
                        "runtime dependency readiness is pending for scheduler task(s): {}",
                        preparation.deferred_task_ids().join(", ")
                    ),
                )],
            ),
        );
    };
    let now_ms = unix_timestamp_ms();
    let (claimed, proof) = match claim_runtime_branch_task_event_for_worker(
        service.as_ref(),
        command,
        task_id,
        now_ms,
    ) {
        Ok(Some(claimed)) => claimed,
        Ok(None) => {
            let diagnostic = WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchEventUnavailable,
                "no due runtime branch task event is available for workflow run",
            );
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch task event is not available for worker claim",
                    vec![diagnostic],
                ),
            );
        }
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch task event claim failed",
                    vec![diagnostic],
                ),
            );
        }
    };

    let mut event_ownership = Some(WorkflowRuntimeBranchOwnedEventClaim {
        event_id: claimed.record.event_id.clone(),
        claim: claimed.claim.clone(),
        proof,
    });
    runtime_branch_responder_registry.record_claim_identity(
        runtime_branch_responder_registration,
        event_ownership.as_ref().expect("local event ownership"),
    );
    let result = async {
    let dispatching_record = match mark_claimed_runtime_branch_task_event_dispatching(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        now_ms, event_ownership.as_ref().map(|owned| &owned.proof)) {
        Ok(record) => record,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch task event dispatching persistence failed",
                    vec![diagnostic],
                ),
            );
        }
    };

    let started_dispatch = match preparation_boundary
        .start_runtime_branch_dispatch_attempt(
            &command.session_id,
            &command.workflow_run_id,
            &dispatching_record.scheduler_task_id,
            preparation.admitted_runtime_readiness(),
            scheduler_transition_from_runtime_branch_start_reason(command.start_reason),
        )
        .await
    {
        Ok(started_dispatch) => started_dispatch,
        Err(error) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                fail_runtime_branch_preparation_error(
                    command,
                    service.as_ref(),
                    &dispatching_record.event_id,
                    &claimed.claim,
                    error, event_ownership.as_ref().map(|owned| &owned.proof)),
            );
        }
    };
    let evidence_record = match record_runtime_branch_selected_candidate_fact(
        service.as_ref(),
        &dispatching_record.event_id,
        &claimed.claim,
        started_dispatch.selected_candidate_fact.clone(), event_ownership.as_ref().map(|owned| &owned.proof)) {
        Ok(record) => record,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch selected-candidate evidence persistence failed",
                    vec![diagnostic],
                ),
            );
        }
    };
    let dispatch_assignment = match create_runtime_branch_dispatch_assignment(
        service.as_ref(),
        &evidence_record,
        &claimed.claim,
        &started_dispatch,
        unix_timestamp_ms(),
    ) {
        Ok(record) => record,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch dispatch-assignment persistence failed",
                    vec![diagnostic],
                ),
            );
        }
    };
    let _linked_record = match link_runtime_branch_dispatch_assignment(
        service.as_ref(),
        &evidence_record.event_id,
        &claimed.claim,
        dispatch_assignment.assignment_id.clone(),
        started_dispatch
            .started_runtime_task
            .attempt_id()
            .as_str()
            .to_string(),
        unix_timestamp_ms(), event_ownership.as_ref().map(|owned| &owned.proof)) {
        Ok(record) => record,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                    command,
                    "runtime branch dispatch-assignment link persistence failed",
                    vec![diagnostic],
                ),
            );
        }
    };

    if let Err(diagnostic) = mark_claimed_runtime_branch_task_event_running(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        unix_timestamp_ms(), event_ownership.as_ref().map(|owned| &owned.proof)) {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event running persistence failed",
                vec![diagnostic],
            ),
        );
    }

    match runtime_branch_responder_registry.attach_runtime_dispatch_assignment(
        runtime_branch_responder_registration,
        &dispatch_assignment.assignment_id,
        event_ownership.take()) {
        Ok(registration) => {
            *runtime_branch_responder_registration = registration;
        }
        Err(outcome) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome);
        }
    }
    if let Err(diagnostic) = mark_runtime_branch_dispatch_assignment_running(
        service.as_ref(),
        &dispatch_assignment.assignment_id,
        unix_timestamp_ms(),
    ) {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch dispatch-assignment running persistence failed",
                vec![diagnostic],
            ),
        );
    }

            let claim_outcome = match claim_runtime_branch_batch_broker_decision(
                service.as_ref(),
                runtime_branch_responder_registry,
                &dispatch_assignment.assignment_id,
                unix_timestamp_ms(),
            ) {
                Ok(claim_outcome) => claim_outcome,
                Err(diagnostic) => {
                    if assignment_has_batch_owner(service.as_ref(), &dispatch_assignment.assignment_id) { return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch; }
                    return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                        WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(command, "runtime branch batch broker claim failed", vec![diagnostic]),
                    );
                }
            };
            match execute_runtime_branch_batch_claim(
                environment,
                runtime_branch_responder_registry,
                claim_outcome,
            )
            .await
            {
                Ok(continuations) if continuations.is_empty() => WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch,
                Ok(continuations) => WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations),
                Err(outcome) => {
                    WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome)
                }
            }
    }.await;
    if let Some(owned) = event_ownership {
        let _ = service
            .runtime_branch_task_event_repository
            .lock()
            .map(|mut repository| {
                repository.fail(
                    &owned.event_id,
                    &owned.claim,
                    unix_timestamp_ms(),
                    Some(&owned.proof),
                )
            });
    }
    result
}

async fn execute_runtime_branch_batch_claim(
    environment: &WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    runtime_branch_responder_registry: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry,
    claimed: (
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeBranchBatchClaimOwnership,
    ),
) -> Result<Vec<WorkflowRuntimeBranchContinuation>, WorkflowTaskExecutionWorkerOutcome> {
    let (claim_outcome, mut ownership) = claimed;
    let service = environment.service();
    let assignments = claim_outcome.assignments.clone();
    let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
        &service.scheduler_task_orchestrator,
        runtime_branch_responder_registry,
    );
    let member_outcomes = match owner
        .execute_claimed_batch(service.as_ref(), claim_outcome, &mut ownership)
        .await
    {
        Ok(outcome) => outcome.member_outcomes,
        Err(failure) => batch_failure_member_outcomes(&assignments, failure),
    };
    let mut continuations = Vec::new();
    let mut terminal_outcomes = Vec::new();
    for mut outcome in member_outcomes {
        if outcome.state == WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Continue {
            let assignment = assignments
                .iter()
                .find(|assignment| assignment.assignment_id == outcome.assignment_id)
                .expect("batch outcome assignment exists");
            let proof = ownership
                .events
                .iter()
                .find(|event| event.event_id == assignment.runtime_branch_event_id)
                .map(|event| &event.proof);
            match complete_claimed_runtime_branch_task_event(
                service.as_ref(),
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(),
                proof,
            ) {
                Ok(_) => {
                    continuations.push(
                        runtime_branch_responder_registry
                            .continue_workflow_run(&outcome)
                            .map_err(|outcome| *outcome)?,
                    );
                    continue;
                }
                Err(diagnostic) => {
                    outcome.state = WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed;
                    outcome.diagnostics.push(WorkflowRuntimeBranchBatchExecutionDiagnostic {
                        code: WorkflowRuntimeBranchBatchExecutionDiagnosticCode::WorkflowRunFinalizationInvalid,
                        message: diagnostic.message,
                    });
                }
            }
        }
        terminal_outcomes.push(outcome);
    }
    let completions = runtime_branch_batch_member_completions(
        service.as_ref(),
        &assignments,
        terminal_outcomes,
        &ownership,
    );
    runtime_branch_responder_registry.complete_runtime_dispatch_assignments(completions)?;
    Ok(continuations)
}

fn assignment_has_batch_owner(
    service: &WorkflowService,
    assignment_id: &WorkflowRuntimeDispatchAssignmentId,
) -> bool {
    service
        .runtime_dispatch_assignment_repository
        .lock()
        .is_ok_and(|repository| {
            repository
                .get(assignment_id)
                .is_some_and(|assignment| assignment.batch_claim.is_some())
        })
}

fn claim_runtime_branch_batch_broker_decision(
    service: &WorkflowService,
    registry: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry,
    assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    now_ms: u64,
) -> Result<
    (
        WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
        WorkflowRuntimeBranchBatchClaimOwnership,
    ),
    WorkflowTaskExecutionWorkerDiagnostic,
> {
    let mut responders = registry.responders.lock().map_err(|_| {
        WorkflowTaskExecutionWorkerDiagnostic::new(
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
            "runtime branch responder registry lock poisoned",
        )
    })?;
    let owner_id = WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId::parse(
        RUNTIME_BRANCH_BATCH_CLAIM_OWNER_ID,
    )
    .map_err(runtime_dispatch_assignment_diagnostic)?;
    let mut repository = service
        .runtime_dispatch_assignment_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime dispatch-assignment repository lock poisoned",
            )
        })?;
    let decision = repository
        .evaluate_running_batch_broker_decision(
            WorkflowRuntimeDispatchAssignmentBatchBrokerRequest {
                anchor_assignment_id: assignment_id.clone(),
                now_ms,
                max_assignments: RUNTIME_BRANCH_BATCH_BROKER_MAX_ASSIGNMENTS,
            },
        )
        .map_err(runtime_dispatch_assignment_diagnostic)?;
    let WorkflowRuntimeDispatchAssignmentBatchReadyDecision { assignments } = &decision;
    for assignment in assignments {
        let key = WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
            &assignment.assignment_id,
        );
        let valid = responders.get(&key).is_some_and(|registered| {
            registered.session_id == assignment.session_id
                && registered.workflow_run_id == assignment.workflow_run_id
                && registered.event_ownership.as_ref().is_some_and(|owned| {
                    owned.event_id == assignment.runtime_branch_event_id
                        && owned.claim == assignment.runtime_branch_claim
                })
        });
        if !valid {
            return Err(WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderUnavailable,
                "batch requires every current assignment responder and event proof",
            ));
        }
    }
    let outcome = repository
        .claim_batch_broker_decision(WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest {
            decision,
            owner_id,
            now_ms,
            lease_duration_ms: RUNTIME_BRANCH_BATCH_CLAIM_LEASE_MS,
        })
        .map_err(runtime_dispatch_assignment_diagnostic)?;
    let proof = repository
        .own_batch_claim(&outcome, now_ms)
        .map_err(runtime_dispatch_assignment_diagnostic)?;
    let events = outcome
        .assignments
        .iter()
        .map(|assignment| {
            let key =
                WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                    &assignment.assignment_id,
                );
            let registered = responders.get_mut(&key).expect("validated batch responder");
            registered.execution_task_id = tokio::task::try_id();
            registered.batch_claim = Some(outcome.batch_claim.clone());
            registered
                .event_ownership
                .take()
                .expect("validated batch event ownership")
        })
        .collect();
    Ok((
        outcome,
        WorkflowRuntimeBranchBatchClaimOwnership {
            batch: proof,
            events,
        },
    ))
}

fn batch_failure_member_outcomes(
    assignments: &[WorkflowRuntimeDispatchAssignmentRecord],
    failure: WorkflowRuntimeBranchBatchExecutionFailure,
) -> Vec<WorkflowRuntimeBranchBatchMemberExecutionOutcome> {
    let mut member_outcomes = failure.member_outcomes;
    let mut completed_assignment_ids = member_outcomes
        .iter()
        .map(|outcome| outcome.assignment_id.clone())
        .collect::<BTreeSet<_>>();
    for assignment in assignments {
        if completed_assignment_ids.insert(assignment.assignment_id.clone()) {
            member_outcomes.push(WorkflowRuntimeBranchBatchMemberExecutionOutcome {
                assignment_id: assignment.assignment_id.clone(),
                session_id: assignment.session_id.clone(),
                workflow_id: assignment.workflow_id.clone(),
                workflow_run_id: assignment.workflow_run_id.clone(),
                state: WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed,
                completed_response: None,
                diagnostics: failure.diagnostics.clone(),
            });
        }
    }
    member_outcomes
}

fn runtime_branch_batch_member_completions(
    service: &WorkflowService,
    assignments: &[WorkflowRuntimeDispatchAssignmentRecord],
    member_outcomes: Vec<WorkflowRuntimeBranchBatchMemberExecutionOutcome>,
    ownership: &WorkflowRuntimeBranchBatchClaimOwnership,
) -> Vec<WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion> {
    member_outcomes
        .into_iter()
        .filter_map(|member_outcome| {
            let assignment = assignments
                .iter()
                .find(|assignment| assignment.assignment_id == member_outcome.assignment_id)?;
            Some(runtime_branch_batch_member_completion(
                service,
                assignment,
                member_outcome,
                ownership,
            ))
        })
        .collect()
}

fn runtime_branch_batch_member_completion(
    service: &WorkflowService,
    assignment: &WorkflowRuntimeDispatchAssignmentRecord,
    member_outcome: WorkflowRuntimeBranchBatchMemberExecutionOutcome,
    ownership: &WorkflowRuntimeBranchBatchClaimOwnership,
) -> WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
    let proof = ownership
        .events
        .iter()
        .find(|owned| owned.event_id == assignment.runtime_branch_event_id)
        .map(|owned| &owned.proof);
    let mut diagnostics = member_outcome
        .diagnostics
        .iter()
        .cloned()
        .map(runtime_branch_batch_execution_diagnostic)
        .collect::<Vec<_>>();
    if member_outcome.state == WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed {
        if let Err(diagnostic) = settle_failed_owned_assignment(service, assignment, ownership) {
            diagnostics.push(diagnostic);
        }
    }
    let outcome = match member_outcome.state {
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Continue
        | WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Completed => {
            match complete_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(), proof) {
                Ok(_record) => match member_outcome.completed_response {
                    Some(response) => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(
                        WorkflowTaskExecutionWorkerRuntimeBranchCompletedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            response,
                            diagnostics,
                        },
                    ),
                    None => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            error_message:
                                "runtime branch batch completed without workflow run response"
                                    .to_string(),
                            diagnostics: vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                                "runtime branch batch completed without workflow run response",
                            )],
                        },
                    ),
                },
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            error_message:
                                "runtime branch task event completion failed after batch"
                                    .to_string(),
                            diagnostics,
                        },
                    )
                }
            }
        }
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Deferred
        | WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Retryable => {
            let deferred_at_ms = unix_timestamp_ms();
            match defer_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                deferred_at_ms,
                deferred_at_ms
                    .saturating_add(RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS), proof) {
                Ok(_record) => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(
                    WorkflowTaskExecutionWorkerRuntimeBranchDeferredOutcome {
                        session_id: member_outcome.session_id.clone(),
                        workflow_run_id: member_outcome.workflow_run_id.clone(),
                        reason:
                            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::RuntimeDispatchUnavailable,
                        deferred_task_ids: Vec::new(),
                        diagnostics,
                    },
                ),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            error_message:
                                "runtime branch task event defer failed after batch".to_string(),
                            diagnostics,
                        },
                    )
                }
            }
        }
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Cancelled => {
            let message = diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone())
                .unwrap_or_else(|| "runtime branch batch member cancelled".to_string());
            match fail_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(), proof) {
                Ok(_record) => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCancelled(
                    WorkflowTaskExecutionWorkerRuntimeBranchCancelledOutcome {
                        session_id: member_outcome.session_id.clone(),
                        workflow_run_id: member_outcome.workflow_run_id.clone(),
                        message,
                        diagnostics,
                    },
                ),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            error_message:
                                "runtime branch task event cancellation persistence failed after batch"
                                    .to_string(),
                            diagnostics,
                        },
                    )
                }
            }
        }
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed => {
            match fail_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(), proof) {
                Ok(_record) => WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                    WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                        session_id: member_outcome.session_id.clone(),
                        workflow_run_id: member_outcome.workflow_run_id.clone(),
                        error_message: "runtime branch batch member failed".to_string(),
                        diagnostics,
                    },
                ),
                Err(diagnostic) => {
                    diagnostics.push(diagnostic);
                    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(
                        WorkflowTaskExecutionWorkerRuntimeBranchFailedOutcome {
                            session_id: member_outcome.session_id.clone(),
                            workflow_run_id: member_outcome.workflow_run_id.clone(),
                            error_message:
                                "runtime branch task event failure persistence failed after batch"
                                    .to_string(),
                            diagnostics,
                        },
                    )
                }
            }
        }
    };
    WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
        assignment_id: member_outcome.assignment_id,
        session_id: member_outcome.session_id,
        workflow_run_id: member_outcome.workflow_run_id,
        workflow_id: member_outcome.workflow_id,
        outcome,
    }
}

fn runtime_branch_batch_execution_diagnostic(
    diagnostic: WorkflowRuntimeBranchBatchExecutionDiagnostic,
) -> WorkflowTaskExecutionWorkerDiagnostic {
    WorkflowTaskExecutionWorkerDiagnostic::new(
        WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
        format!(
            "runtime branch batch execution diagnostic ({:?}): {}",
            diagnostic.code, diagnostic.message
        ),
    )
}

impl WorkflowRuntimeBranchBatchResponderFanOut
    for WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry
{
    fn ensure_assignment_responders_registered(
        &self,
        members: &[WorkflowRuntimeBranchBatchExecutionMember],
    ) -> Result<(), WorkflowRuntimeBranchBatchExecutionDiagnostic> {
        let responders = self.responders.lock().map_err(|_| {
            WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ResponderFanOutUnavailable,
                "runtime branch responder registry lock poisoned",
            )
        })?;
        for member in members {
            let key =
                WorkflowTaskExecutionWorkerRuntimeBranchResponderKey::runtime_dispatch_assignment(
                    &member.assignment_id,
                );
            let registered = responders.get(&key).ok_or_else(|| {
                WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ResponderFanOutUnavailable,
                    format!(
                        "runtime branch responder is not registered for dispatch assignment '{}'",
                        member.assignment_id.as_str()
                    ),
                )
            })?;
            if registered.session_id != member.session_id
                || registered.workflow_run_id != member.workflow_run_id
                || registered.workflow_id != member.workflow_id
            {
                return Err(WorkflowRuntimeBranchBatchExecutionDiagnostic::new(
                    WorkflowRuntimeBranchBatchExecutionDiagnosticCode::ResponderFanOutUnavailable,
                    format!(
                        "runtime branch responder registration for dispatch assignment '{}' changed before batch execution",
                        member.assignment_id.as_str()
                    ),
                ));
            }
        }
        Ok(())
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
    proof: Option<&WorkflowRuntimeClaimOwnership>,
) -> WorkflowTaskExecutionWorkerOutcome {
    match fail_claimed_runtime_branch_task_event(
        service,
        event_id,
        claim,
        unix_timestamp_ms(),
        proof,
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
    }
}

#[cfg(test)]
fn defer_runtime_branch_dependency_readiness(
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    deferred_task_ids: Vec<String>,
    message: String,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        proof,
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

fn claim_runtime_branch_task_event_for_worker(
    service: &WorkflowService,
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    task_id: &str,
    now_ms: u64,
) -> Result<
    Option<(
        WorkflowRuntimeBranchTaskEventClaimOutcome,
        WorkflowRuntimeClaimOwnership,
    )>,
    WorkflowTaskExecutionWorkerDiagnostic,
> {
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
        .claim_owned_for_workflow_task(
            &command.workflow_run_id,
            task_id,
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
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .release_claim(event_id, claim, now_ms, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn defer_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    deferred_at_ms: u64,
    ready_at_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .defer_until(event_id, claim, deferred_at_ms, ready_at_ms, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn mark_claimed_runtime_branch_task_event_dispatching(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .mark_dispatching(event_id, claim, now_ms, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn record_runtime_branch_selected_candidate_fact(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .record_selected_candidate_fact(event_id, claim, selected_candidate_fact, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn create_runtime_branch_dispatch_assignment(
    service: &WorkflowService,
    record: &WorkflowRuntimeBranchTaskEventRecord,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    started_dispatch: &super::session_scheduler_runner::WorkflowStartedRuntimeDispatchAttempt,
    created_at_ms: u64,
) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let request = WorkflowRuntimeDispatchAssignmentRequest {
        assignment_id: WorkflowRuntimeDispatchAssignmentId::new(),
        runtime_branch_event_id: record.event_id.clone(),
        session_id: record.session_id.clone(),
        workflow_id: record.workflow_id.clone(),
        workflow_run_id: record.workflow_run_id.clone(),
        scheduler_task_id: record.scheduler_task_id.clone(),
        scheduler_task_attempt_id: started_dispatch
            .started_runtime_task
            .attempt_id()
            .as_str()
            .to_string(),
        scheduler_task_attempt_started_at_ms: started_dispatch.started_runtime_task.started_at_ms(),
        task_attempt_generation: record.attempt_generation,
        timeout_ms: record.timeout_ms,
        runtime_source_context: record.runtime_source_context.clone(),
        runtime_branch_claim: claim.clone(),
        readiness_proof: started_dispatch
            .selected_dispatch
            .runtime_handoff()
            .readiness_proof
            .clone(),
        selected_candidate_fact: started_dispatch.selected_candidate_fact.clone(),
        selected_runtime_handoff: started_dispatch.selected_dispatch.runtime_handoff().clone(),
        reservation_lease_id: started_dispatch
            .selected_dispatch
            .reservation_lease_id()
            .clone(),
        selected_candidate_id: started_dispatch.selected_dispatch.candidate_id().cloned(),
        created_at_ms,
    };
    let mut repository = service
        .runtime_dispatch_assignment_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime dispatch-assignment repository lock poisoned",
            )
        })?;
    repository
        .create(request)
        .map_err(runtime_dispatch_assignment_diagnostic)
}

fn link_runtime_branch_dispatch_assignment(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    assignment_id: WorkflowRuntimeDispatchAssignmentId,
    scheduler_task_attempt_id: String,
    linked_at_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .link_dispatch_assignment(
            event_id,
            claim,
            assignment_id,
            scheduler_task_attempt_id,
            linked_at_ms,
            proof,
        )
        .map_err(runtime_branch_event_diagnostic)
}

fn runtime_dispatch_assignment_diagnostic(
    diagnostic: WorkflowRuntimeDispatchAssignmentDiagnostic,
) -> WorkflowTaskExecutionWorkerDiagnostic {
    let code = match diagnostic.code {
        WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidAssignment
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::DuplicateAssignment
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::DuplicateActiveAssignment
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotFound
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidTransition
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::TaskAttemptFactInvalid
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchClaim
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::AssignmentNotRunning
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::AlreadyBatchClaimed
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::MissingTaskAttemptFact
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::BatchCompatibilityRejected => {
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable
        }
    };
    WorkflowTaskExecutionWorkerDiagnostic::new(
        code,
        format!(
            "runtime dispatch-assignment diagnostic ({:?}): {}",
            diagnostic.code, diagnostic.message
        ),
    )
}

fn mark_runtime_branch_dispatch_assignment_running(
    service: &WorkflowService,
    assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    now_ms: u64,
) -> Result<WorkflowRuntimeDispatchAssignmentRecord, WorkflowTaskExecutionWorkerDiagnostic> {
    let mut repository = service
        .runtime_dispatch_assignment_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime dispatch-assignment repository lock poisoned",
            )
        })?;
    repository
        .mark_running(assignment_id, now_ms)
        .map_err(runtime_dispatch_assignment_diagnostic)
}

fn mark_claimed_runtime_branch_task_event_running(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .mark_running(event_id, claim, now_ms, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn complete_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .complete(event_id, claim, now_ms, proof)
        .map_err(runtime_branch_event_diagnostic)
}

fn fail_claimed_runtime_branch_task_event(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    now_ms: u64,
    proof: Option<&WorkflowRuntimeClaimOwnership>,
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
        .fail(event_id, claim, now_ms, proof)
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

fn scheduler_transition_from_runtime_branch_start_reason(
    start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason,
) -> pantograph_diagnostics_ledger::SchedulerTaskAttemptLifecycleTransition {
    match start_reason {
        WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started => {
            pantograph_diagnostics_ledger::SchedulerTaskAttemptLifecycleTransition::Started
        }
        WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Redispatched => {
            pantograph_diagnostics_ledger::SchedulerTaskAttemptLifecycleTransition::Redispatched
        }
    }
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
    use crate::workflow::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentState;
    use crate::workflow::runtime_dispatch_selection::{
        ValidatedWorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateFact,
        WorkflowRuntimeDispatchCandidateFactBundle, WorkflowRuntimeDispatchCandidateProvider,
        WorkflowRuntimeDispatchCandidateProviderError, WorkflowRuntimeDispatchCandidateSet,
        WorkflowRuntimeDispatchLoadState,
        WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
    };
    use crate::workflow::{
        WorkflowExecutionSessionRunRequest, WorkflowIoNode, WorkflowIoPort, WorkflowIoResponse,
        WorkflowPortBinding, WorkflowRunHandle, WorkflowRunOptions, WorkflowSchedulerTask,
        WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };
    use pantograph_dependency_planning::{
        DependencyReadinessProofEnvelope, DependencyReadinessProofId,
        DependencyReadinessWorkflowRunId,
    };
    use pantograph_runtime_host_contracts::{
        ReservationLifecycleApplication, ReservationLifecycleApplicationState,
        ReservationLifecycleEvent, ReservationLifecycleOutcome, ReservationLifecyclePort,
        ReservationLifecyclePortError, RuntimeHostBatchExecutionMemberResponse,
        RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
        RuntimeHostBatchExecutionRequest, RuntimeHostBatchExecutionResponse,
        RuntimeHostBatchExecutionState, RuntimeHostBatchMemberReservationDisposition,
        RuntimeHostBatchMemberRetryDisposition, RuntimeHostExecutionCancellationHandle,
        RuntimeHostExecutionOutput, RuntimeHostExecutionOutputValue, RuntimeHostExecutionPortError,
        RESERVATION_LIFECYCLE_CONTRACT_VERSION, RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
    };
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerDispatchCandidateId, SchedulerNodeId,
        SchedulerReservationLeaseId, SchedulerResourceFitAssessment, SchedulerResourceFitState,
        SchedulerResourceKind, SchedulerResourceReservation, SchedulerRuntimeHandoff,
        SchedulerTaskExecutionIntent, SchedulerTaskId, SchedulerTaskState,
        SchedulerTaskStateRecord, SchedulerTaskStateTransitionId, SchedulerWorkflowId,
        SchedulerWorkflowRunId, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };
    use std::time::Duration;

    struct WorkerHost;

    #[derive(Default)]
    struct RecordingReservationLifecyclePort {
        events: std::sync::Mutex<Vec<ReservationLifecycleEvent>>,
    }

    impl RecordingReservationLifecyclePort {
        fn events(&self) -> Vec<ReservationLifecycleEvent> {
            self.events
                .lock()
                .expect("reservation lifecycle events lock")
                .clone()
        }
    }

    #[async_trait::async_trait]
    impl ReservationLifecyclePort for RecordingReservationLifecyclePort {
        async fn apply_reservation_lifecycle(
            &self,
            event: ReservationLifecycleEvent,
        ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
            self.events
                .lock()
                .expect("reservation lifecycle events lock")
                .push(event.clone());
            Ok(ReservationLifecycleApplication {
                contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
                lifecycle_event_id: event.lifecycle_event_id,
                reservation_lease_id: event.reservation_lease_id,
                state: ReservationLifecycleApplicationState::Applied,
                diagnostics: Vec::new(),
            })
        }
    }

    #[async_trait::async_trait]
    impl WorkflowHost for WorkerHost {
        async fn workflow_io(
            &self,
            _workflow_id: &str,
        ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
            Ok(WorkflowIoResponse {
                inputs: Vec::new(),
                outputs: vec![WorkflowIoNode {
                    node_id: "node.llm_inference".to_string(),
                    node_type: "llm-inference".to_string(),
                    name: Some("Image".to_string()),
                    description: None,
                    ports: vec![WorkflowIoPort {
                        port_id: "image".to_string(),
                        name: Some("Image".to_string()),
                        description: None,
                        data_type: Some("string".to_string()),
                        required: Some(true),
                        multiple: Some(false),
                    }],
                }],
            })
        }

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
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                outcome.error_message.clone(),
            )]
        );
        assert!(
            outcome.error_message.contains("session"),
            "unexpected error message: {}",
            outcome.error_message
        );

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[tokio::test]
    async fn task_execution_worker_accepts_multiple_runtime_branch_commands_with_separate_responders(
    ) {
        let scheduler_lifecycle = scheduler_lifecycle();
        let worker = WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle, runtime_environment())
            .expect("spawn task execution worker");
        let (first_responder, first_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let (second_responder, second_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();

        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command_for_run("run-1"),
                first_responder,
            ))
            .expect("enqueue first runtime branch command");
        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command_for_run("run-2"),
                second_responder,
            ))
            .expect("enqueue second runtime branch command");

        let (first_outcome, second_outcome) = tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if worker.observed_runtime_branch_command_count() >= 2 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            (
                first_completion_rx
                    .await
                    .expect("first runtime branch completion"),
                second_completion_rx
                    .await
                    .expect("second runtime branch completion"),
            )
        })
        .await
        .expect("worker should complete both runtime branch commands");

        assert_runtime_branch_event_unavailable(first_outcome, "run-1");
        assert_runtime_branch_event_unavailable(second_outcome, "run-2");

        worker
            .shutdown()
            .await
            .expect("shutdown task execution worker");
    }

    #[tokio::test]
    async fn task_execution_worker_dispatches_later_peers_independently_and_completes_each() {
        let batch_port = Arc::new(RecordingWorkerBatchExecutionPort::default());
        let reservation_lifecycle_port = Arc::new(RecordingReservationLifecyclePort::default());
        let service = Arc::new(
            WorkflowService::new()
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    SingleCanonicalRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_batch_execution_port(batch_port.clone())
                .with_reservation_lifecycle_port(reservation_lifecycle_port.clone()),
        );
        let first_session_id =
            prepare_ready_runtime_branch_run(service.as_ref(), "run.2026-05-22.101");
        let second_session_id =
            prepare_ready_runtime_branch_run(service.as_ref(), "run.2026-05-22.102");
        let first_event_id = enqueue_ready_runtime_branch_event(
            service.as_ref(),
            &first_session_id,
            "run.2026-05-22.101",
        );
        let second_event_id = enqueue_ready_runtime_branch_event(
            service.as_ref(),
            &second_session_id,
            "run.2026-05-22.102",
        );
        let environment = WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
            Arc::clone(&service),
            test_host(),
        );
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let first_command =
            runtime_branch_command_for_session_run(&first_session_id, "run.2026-05-22.101");
        let second_command =
            runtime_branch_command_for_session_run(&second_session_id, "run.2026-05-22.102");
        let (first_responder, mut first_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let (second_responder, mut second_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let mut first_registration = registry
            .register_workflow_run(&first_command, first_responder)
            .expect("register first responder");
        let mut second_registration = registry
            .register_workflow_run(&second_command, second_responder)
            .expect("register second responder");

        let first_result = claim_and_execute_runtime_branch_event(
            &environment,
            &first_command,
            &registry,
            &mut first_registration,
        )
        .await;

        let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations) =
            first_result
        else {
            panic!("successful task must continue its run");
        };
        drive_runtime_branch_continuations(&environment, &registry, continuations).await;
        assert_eq!(
            batch_port.requests().len(),
            1,
            "first ready branch dispatches immediately"
        );
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &first_event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );

        let second_result = claim_and_execute_runtime_branch_event(
            &environment,
            &second_command,
            &registry,
            &mut second_registration,
        )
        .await;

        let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations) =
            second_result
        else {
            panic!("successful task must continue its run");
        };
        drive_runtime_branch_continuations(&environment, &registry, continuations).await;
        let first_outcome = tokio::time::timeout(Duration::from_secs(1), &mut first_completion_rx)
            .await
            .expect("first branch completion should be fanned out")
            .expect("first branch completion responder");
        let second_outcome =
            tokio::time::timeout(Duration::from_secs(1), &mut second_completion_rx)
                .await
                .expect("second branch completion should be fanned out")
                .expect("second branch completion responder");

        assert_runtime_branch_completed_response(
            first_outcome,
            "run.2026-05-22.101",
            "image for run.2026-05-22.101",
        );
        assert_runtime_branch_completed_response(
            second_outcome,
            "run.2026-05-22.102",
            "image for run.2026-05-22.102",
        );
        let requests = batch_port.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].members.len(), 1);
        assert_eq!(requests[1].members.len(), 1);
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &first_event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &second_event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(
            reservation_lifecycle_port
                .events()
                .iter()
                .filter(|event| event.outcome == ReservationLifecycleOutcome::DispatchStarted)
                .count(),
            2,
            "worker grouped dispatch must mark each reservation as dispatch-started"
        );
        assert_eq!(registry.active_responder_count(), 0);
    }

    #[tokio::test]
    async fn task_execution_worker_dispatches_singleton_without_peer_or_timer() {
        let batch_port = Arc::new(RecordingWorkerBatchExecutionPort::default());
        let service = Arc::new(
            WorkflowService::new()
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    SingleCanonicalRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_batch_execution_port(batch_port.clone())
                .with_reservation_lifecycle_port(Arc::new(
                    RecordingReservationLifecyclePort::default(),
                )),
        );
        let session_id = prepare_ready_runtime_branch_run(service.as_ref(), "run.2026-05-22.103");
        let event_id =
            enqueue_ready_runtime_branch_event(service.as_ref(), &session_id, "run.2026-05-22.103");
        let environment = WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
            Arc::clone(&service),
            test_host(),
        );
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let command = runtime_branch_command_for_session_run(&session_id, "run.2026-05-22.103");
        let (responder, mut completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let mut registration = registry
            .register_workflow_run(&command, responder)
            .expect("register responder");

        let first_result = claim_and_execute_runtime_branch_event(
            &environment,
            &command,
            &registry,
            &mut registration,
        )
        .await;

        let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::Continue(continuations) =
            first_result
        else {
            panic!("successful task must continue its run");
        };
        drive_runtime_branch_continuations(&environment, &registry, continuations).await;
        let completed = tokio::time::timeout(Duration::from_secs(1), &mut completion_rx)
            .await
            .expect("singleton completion")
            .expect("singleton responder");
        assert_runtime_branch_completed_response(
            completed,
            "run.2026-05-22.103",
            "image for run.2026-05-22.103",
        );
        let requests = batch_port.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].members.len(), 1);
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(
            runtime_dispatch_assignment_for_event(service.as_ref(), &event_id).state,
            WorkflowRuntimeDispatchAssignmentState::Completed
        );
        assert_eq!(registry.active_responder_count(), 0);
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
                runtime_source_context: runtime_source_context(),
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
                .contains("runtime scheduler task 'image-task' cannot continue from Invalid"),
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
                .contains("runtime scheduler task 'image-task' cannot continue from Invalid"),
            "unexpected diagnostic: {}",
            outcome.diagnostics[0].message
        );
        let persisted = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .get(&event_id)
            .expect("runtime branch task event");
        assert_eq!(persisted.state, WorkflowRuntimeBranchTaskEventState::Ready);
        assert!(persisted.claim.is_none());
        assert!(persisted.dispatching_at_ms.is_none());
        assert!(persisted.running_at_ms.is_none());
        assert!(persisted.deferred_at_ms.is_none());
        assert!(persisted.failed_at_ms.is_none());

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
                runtime_source_context: runtime_source_context(),
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
            .mark_dispatching(&event_id, &claimed.claim, 120, None)
            .expect("event marks dispatching");

        let deferred = defer_claimed_runtime_branch_task_event(
            &service,
            &dispatching.event_id,
            &claimed.claim,
            130,
            130_u64.saturating_add(RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS),
            None,
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

    const WORKER_BATCH_WORKFLOW_ID: &str = "workflow.image_generation";
    const WORKER_BATCH_NODE_ID: &str = "node.llm_inference";
    const WORKER_BATCH_TASK_ID: &str = "task.image_generation.001";

    #[derive(Clone, Copy)]
    enum ControlledHostOutcome {
        Cancelled,
        Panic,
        Retryable,
        Deferred,
        RejectedDeferred,
        Accepted,
    }

    struct ControlledWorkerBatchPort {
        entered: Mutex<
            Option<
                tokio::sync::oneshot::Sender<(
                    RuntimeHostBatchExecutionRequest,
                    RuntimeHostExecutionCancellationHandle,
                )>,
            >,
        >,
        settle: tokio::sync::Mutex<Option<tokio::sync::oneshot::Receiver<ControlledHostOutcome>>>,
    }

    #[async_trait::async_trait]
    impl RuntimeHostBatchExecutionPort for ControlledWorkerBatchPort {
        async fn execute_runtime_host_batch_request(
            &self,
            request: RuntimeHostBatchExecutionRequest,
            cancellation: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
            self.entered
                .lock()
                .expect("entered channel")
                .take()
                .expect("exactly one dispatch")
                .send((request.clone(), cancellation))
                .unwrap_or_else(|_| panic!("test observes dispatch"));
            let settle = self.settle.lock().await.take().expect("one host execution");
            match settle.await.expect("controlled host outcome") {
                ControlledHostOutcome::Panic => panic!("injected owned host failure"),
                outcome @ (ControlledHostOutcome::Retryable
                | ControlledHostOutcome::Deferred
                | ControlledHostOutcome::RejectedDeferred
                | ControlledHostOutcome::Accepted) => {
                    let mut response = runtime_host_batch_response_from_request(&request);
                    let (batch_state, member_state, retry) = match outcome {
                        ControlledHostOutcome::Retryable => (
                            RuntimeHostBatchExecutionState::Failed,
                            RuntimeHostBatchExecutionMemberState::Failed,
                            RuntimeHostBatchMemberRetryDisposition::Retryable,
                        ),
                        ControlledHostOutcome::Deferred => (
                            RuntimeHostBatchExecutionState::Deferred,
                            RuntimeHostBatchExecutionMemberState::Deferred,
                            RuntimeHostBatchMemberRetryDisposition::Deferred,
                        ),
                        ControlledHostOutcome::RejectedDeferred => (
                            RuntimeHostBatchExecutionState::Rejected,
                            RuntimeHostBatchExecutionMemberState::Rejected,
                            RuntimeHostBatchMemberRetryDisposition::Deferred,
                        ),
                        ControlledHostOutcome::Accepted => (
                            RuntimeHostBatchExecutionState::Accepted,
                            RuntimeHostBatchExecutionMemberState::Accepted,
                            RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                        ),
                        _ => unreachable!("controlled handback variants"),
                    };
                    response.state = batch_state;
                    response.diagnostics = vec![pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnostic {
                        severity: pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticSeverity::Info,
                        code: pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticCode::HandoffRejected,
                        message: "controlled host handback".to_string(), hint: None,
                    }];
                    for member in &mut response.members {
                        member.state = member_state.clone();
                        member.retry_disposition = retry.clone();
                        member.reservation_disposition =
                            RuntimeHostBatchMemberReservationDisposition::DeferredToScheduler;
                        member.outputs.clear();
                        member.diagnostics = response.diagnostics.clone();
                    }
                    response.validate().expect("valid controlled host response");
                    Ok(response)
                }
                ControlledHostOutcome::Cancelled => {
                    let mut response = runtime_host_batch_response_from_request(&request);
                    response.state = RuntimeHostBatchExecutionState::Cancelled;
                    for member in &mut response.members {
                        member.state = RuntimeHostBatchExecutionMemberState::Cancelled;
                        member.outputs.clear();
                        member.reservation_disposition =
                            RuntimeHostBatchMemberReservationDisposition::Released;
                        member.diagnostics = vec![pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnostic {
                            severity: pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticSeverity::Info,
                            code: pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticCode::CancellationRequested,
                            message: "controlled host stopped after cancellation".to_string(), hint: None,
                        }];
                    }
                    Ok(response)
                }
            }
        }
    }

    struct ControlledWorkerRun {
        service: Arc<WorkflowService>,
        worker: WorkflowTaskExecutionWorker,
        lifecycle: Arc<RecordingReservationLifecyclePort>,
        event_id: WorkflowRuntimeBranchTaskEventId,
        session_id: String,
        completion: tokio::sync::oneshot::Receiver<WorkflowTaskExecutionWorkerOutcome>,
        entered: tokio::sync::oneshot::Receiver<(
            RuntimeHostBatchExecutionRequest,
            RuntimeHostExecutionCancellationHandle,
        )>,
        settle: tokio::sync::oneshot::Sender<ControlledHostOutcome>,
    }

    fn controlled_worker_run(run_id: &str) -> ControlledWorkerRun {
        let (entered_tx, entered) = tokio::sync::oneshot::channel();
        let (settle, settle_rx) = tokio::sync::oneshot::channel();
        let port = Arc::new(ControlledWorkerBatchPort {
            entered: Mutex::new(Some(entered_tx)),
            settle: tokio::sync::Mutex::new(Some(settle_rx)),
        });
        let lifecycle = Arc::new(RecordingReservationLifecyclePort::default());
        let service = Arc::new(
            WorkflowService::new()
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    SingleCanonicalRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_batch_execution_port(port)
                .with_reservation_lifecycle_port(lifecycle.clone()),
        );
        let session_id = prepare_ready_runtime_branch_run(service.as_ref(), run_id);
        let event_id = enqueue_ready_runtime_branch_event(service.as_ref(), &session_id, run_id);
        let worker = WorkflowTaskExecutionWorker::spawn(
            scheduler_lifecycle(),
            WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(service.clone(), test_host()),
        )
        .expect("worker");
        let (responder, completion) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        worker
            .try_enqueue(WorkflowTaskExecutionWorkerCommand::execute_runtime_branch(
                runtime_branch_command_for_session_run(&session_id, run_id),
                responder,
            ))
            .expect("enqueue");
        ControlledWorkerRun {
            service,
            worker,
            lifecycle,
            event_id,
            session_id,
            completion,
            entered,
            settle,
        }
    }

    fn assert_live_claims_exclude_expired_competitors(
        service: &WorkflowService,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) {
        let assignment = runtime_dispatch_assignment_for_event(service, event_id);
        let now_ms = assignment.runtime_branch_claim.lease_expires_at_ms + 1;
        let event_error = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("events")
            .claim_event(
                event_id,
                WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("competing.worker")
                    .expect("owner"),
                now_ms,
                30_000,
            )
            .expect_err("live event excludes expired reclaim");
        assert_eq!(
            event_error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::AlreadyClaimed
        );
        let batch_error = service
            .runtime_dispatch_assignment_repository
            .lock()
            .expect("assignments")
            .claim_compatible_running_batch(
                &assignment.assignment_id,
                WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId::parse("competing.batch")
                    .expect("owner"),
                now_ms,
                1_000,
                8,
            )
            .expect_err("live batch excludes expired reclaim");
        assert_eq!(
            batch_error.code,
            WorkflowRuntimeDispatchAssignmentDiagnosticCode::AlreadyBatchClaimed
        );
    }

    #[tokio::test]
    async fn task_execution_worker_settles_old_assignment_before_retry_or_defer() {
        for (index, action, expected_assignment) in [
            (
                0,
                ControlledHostOutcome::Retryable,
                WorkflowRuntimeDispatchAssignmentState::Failed,
            ),
            (
                1,
                ControlledHostOutcome::RejectedDeferred,
                WorkflowRuntimeDispatchAssignmentState::Failed,
            ),
            (
                2,
                ControlledHostOutcome::Deferred,
                WorkflowRuntimeDispatchAssignmentState::Deferred,
            ),
        ] {
            let run_id = format!("run.2026-05-22.{}", 210 + index);
            let run = controlled_worker_run(&run_id);
            let _entered = run.entered.await.expect("host entry");
            let old = runtime_dispatch_assignment_for_event(&run.service, &run.event_id);
            run.settle
                .send(action)
                .unwrap_or_else(|_| panic!("live host"));
            let outcome = tokio::time::timeout(Duration::from_secs(1), run.completion)
                .await
                .expect("handback completion")
                .expect("responder");
            assert!(
                matches!(
                    outcome,
                    WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(_)
                ),
                "{outcome:?}"
            );
            run.worker.shutdown().await.expect("drain handback");
            assert_eq!(
                runtime_dispatch_assignment_by_id(&run.service, &old.assignment_id).state,
                expected_assignment
            );
            let mut events = run
                .service
                .runtime_branch_task_event_repository
                .lock()
                .expect("events");
            let deferred = events.get(&run.event_id).expect("event");
            assert_eq!(
                deferred.state,
                WorkflowRuntimeBranchTaskEventState::Deferred
            );
            let retry = events
                .claim_event(
                    &run.event_id,
                    WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("next.attempt")
                        .expect("owner"),
                    deferred.ready_at_ms,
                    30_000,
                )
                .expect("new attempt is admitted");
            assert!(retry.claim.attempt_generation > old.runtime_branch_claim.attempt_generation);
            assert!(
                events
                    .complete(
                        &run.event_id,
                        &old.runtime_branch_claim,
                        deferred.ready_at_ms,
                        None
                    )
                    .is_err(),
                "old attempt cannot publish after handback"
            );
            assert_eq!(
                run.lifecycle.events().len(),
                2,
                "host handback preserves its scheduler reservation disposition"
            );
        }
    }

    #[tokio::test]
    async fn task_execution_worker_fences_accepted_host_response_without_retry_or_release() {
        let run = controlled_worker_run("run.2026-05-22.213");
        let _entered = run.entered.await.expect("host entry");
        run.settle
            .send(ControlledHostOutcome::Accepted)
            .unwrap_or_else(|_| panic!("live host"));
        let outcome = tokio::time::timeout(Duration::from_secs(1), run.completion)
            .await
            .expect("indeterminate failure")
            .expect("responder");
        assert!(
            matches!(
                outcome,
                WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(_)
            ),
            "{outcome:?}"
        );
        run.worker
            .shutdown()
            .await
            .expect("drain indeterminate branch");
        assert_eq!(
            runtime_branch_event_state(&run.service, &run.event_id),
            WorkflowRuntimeBranchTaskEventState::Failed
        );
        assert_eq!(
            runtime_dispatch_assignment_for_event(&run.service, &run.event_id).state,
            WorkflowRuntimeDispatchAssignmentState::Failed
        );
        assert_eq!(
            run.lifecycle.events().len(),
            1,
            "Accepted does not establish host stop or authorize reservation release"
        );
        assert!(run
            .service
            .session_store_guard()
            .expect("store")
            .active_run_scheduler_task_results(&run.session_id, "run.2026-05-22.213")
            .expect("results")
            .is_empty());
    }

    #[tokio::test]
    async fn task_execution_worker_cancellation_and_shutdown_retain_claims_until_host_settles() {
        let run = controlled_worker_run("run.2026-05-22.201");
        let (request, cancellation) = tokio::time::timeout(Duration::from_secs(1), run.entered)
            .await
            .expect("host entry")
            .expect("entered request");
        assert_eq!(request.members.len(), 1);
        let assignment = runtime_dispatch_assignment_for_event(&run.service, &run.event_id);
        assert_live_claims_exclude_expired_competitors(&run.service, &run.event_id);
        run.service
            .scheduler_task_orchestrator
            .request_started_runtime_task_cancellation(
                &request.members[0].handoff.task_id,
                &crate::scheduler::WorkflowSchedulerTaskAttemptId::parse(
                    &assignment.scheduler_task_attempt_id,
                )
                .expect("attempt"),
                "test cancellation",
            )
            .expect("cancel request");
        assert_eq!(cancellation.snapshot().state, pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::CancellationRequested);
        let mut shutdown = Box::pin(run.worker.shutdown());
        tokio::select! { biased;
            result = &mut shutdown => panic!("shutdown released live host ownership early: {result:?}"),
            _ = tokio::task::yield_now() => {}
        }
        assert_live_claims_exclude_expired_competitors(&run.service, &run.event_id);
        assert_eq!(
            run.lifecycle.events().len(),
            1,
            "only dispatch-started exists while host is still running"
        );
        assert_eq!(
            run.lifecycle.events()[0].outcome,
            ReservationLifecycleOutcome::DispatchStarted
        );
        run.settle
            .send(ControlledHostOutcome::Cancelled)
            .unwrap_or_else(|_| panic!("host still awaits settlement"));
        let outcome = tokio::time::timeout(Duration::from_secs(1), run.completion)
            .await
            .expect("cancel completion")
            .expect("responder outcome");
        assert!(
            matches!(
                outcome,
                WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCancelled(_)
            ),
            "{outcome:?}"
        );
        shutdown.await.expect("shutdown after host completion");
        assert_eq!(
            runtime_branch_event_state(&run.service, &run.event_id),
            WorkflowRuntimeBranchTaskEventState::Failed
        );
        assert_eq!(
            runtime_dispatch_assignment_for_event(&run.service, &run.event_id).state,
            WorkflowRuntimeDispatchAssignmentState::Cancelled
        );
        assert_eq!(
            run.service
                .scheduler_task_orchestrator
                .active_task_lifecycle_handle_count()
                .expect("active handles"),
            0
        );
        assert_eq!(
            run.lifecycle.events().len(),
            2,
            "reservation terminal handling follows actual host settlement"
        );
    }

    #[tokio::test]
    async fn task_execution_worker_supervises_host_panic_and_fences_replay_without_release() {
        let run = controlled_worker_run("run.2026-05-22.202");
        let _entered = tokio::time::timeout(Duration::from_secs(1), run.entered)
            .await
            .expect("host entry")
            .expect("entered request");
        let assignment = runtime_dispatch_assignment_for_event(&run.service, &run.event_id);
        run.settle
            .send(ControlledHostOutcome::Panic)
            .unwrap_or_else(|_| panic!("host still live"));
        let outcome = tokio::time::timeout(Duration::from_secs(1), run.completion)
            .await
            .expect("supervised failure")
            .expect("failure responder");
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(failure) = outcome else {
            panic!("expected supervised failure");
        };
        assert!(failure
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("injected owned host failure")));
        run.worker
            .shutdown()
            .await
            .expect("supervised task drained");
        assert_eq!(
            runtime_branch_event_state(&run.service, &run.event_id),
            WorkflowRuntimeBranchTaskEventState::Failed
        );
        assert_eq!(
            runtime_dispatch_assignment_for_event(&run.service, &run.event_id).state,
            WorkflowRuntimeDispatchAssignmentState::Failed
        );
        let error = run
            .service
            .runtime_branch_task_event_repository
            .lock()
            .expect("events")
            .claim_event(
                &run.event_id,
                WorkflowRuntimeBranchTaskEventClaimOwnerId::parse("retry.worker").expect("owner"),
                assignment.runtime_branch_claim.lease_expires_at_ms + 1,
                30_000,
            )
            .expect_err("failed host ownership cannot replay");
        assert_eq!(
            error.code,
            WorkflowRuntimeBranchTaskEventDiagnosticCode::TerminalEvent
        );
        assert_eq!(
            run.lifecycle.events().len(),
            1,
            "a panic is not evidence that external work stopped or its reservation can be released"
        );
        assert!(
            run.service
                .session_store_guard()
                .expect("store")
                .active_run_scheduler_task_results(&run.session_id, "run.2026-05-22.202")
                .expect("results")
                .is_empty(),
            "no old success is published after panic"
        );
    }

    #[derive(Default)]
    struct RecordingWorkerBatchExecutionPort {
        requests: Mutex<Vec<RuntimeHostBatchExecutionRequest>>,
    }

    impl RecordingWorkerBatchExecutionPort {
        fn requests(&self) -> Vec<RuntimeHostBatchExecutionRequest> {
            self.requests
                .lock()
                .expect("runtime host batch requests")
                .clone()
        }
    }

    #[async_trait::async_trait]
    impl RuntimeHostBatchExecutionPort for RecordingWorkerBatchExecutionPort {
        async fn execute_runtime_host_batch_request(
            &self,
            request: RuntimeHostBatchExecutionRequest,
            _cancellation: RuntimeHostExecutionCancellationHandle,
        ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
            self.requests
                .lock()
                .expect("runtime host batch requests")
                .push(request.clone());
            Ok(runtime_host_batch_response_from_request(&request))
        }
    }

    struct SingleCanonicalRuntimeDispatchCandidateProvider;

    impl WorkflowRuntimeDispatchCandidateProvider for SingleCanonicalRuntimeDispatchCandidateProvider {
        fn runtime_dispatch_candidates(
            &self,
            task: &WorkflowSchedulerTask,
            _ready_record: &SchedulerTaskStateRecord,
            readiness_proof: &DependencyReadinessProofEnvelope,
        ) -> Result<
            WorkflowRuntimeDispatchCandidateSet,
            WorkflowRuntimeDispatchCandidateProviderError,
        > {
            let intent = task.schedulable_intent.as_ref().ok_or_else(|| {
                worker_candidate_provider_error(
                    "worker batch test runtime task is missing a schedulable intent",
                )
            })?;
            let selected_runtime_id =
                intent
                    .constraints
                    .requested_runtime_id
                    .clone()
                    .ok_or_else(|| {
                        worker_candidate_provider_error(
                            "worker batch test runtime task is missing a requested runtime id",
                        )
                    })?;
            let selected_device_id =
                intent
                    .constraints
                    .requested_device_id
                    .clone()
                    .ok_or_else(|| {
                        worker_candidate_provider_error(
                            "worker batch test runtime task is missing a requested device id",
                        )
                    })?;
            let environment_ref = readiness_proof
                .preflight_result
                .environment_ref
                .clone()
                .ok_or_else(|| {
                    worker_candidate_provider_error(
                        "worker batch test readiness proof is missing an environment reference",
                    )
                })?;
            let reservation = SchedulerResourceReservation {
                reservation_lease_id: SchedulerReservationLeaseId::parse(format!(
                    "reservation.{}",
                    intent.workflow_run_id.as_str()
                ))
                .map_err(|error| worker_candidate_provider_error(error.to_string()))?,
                workflow_run_id: intent.workflow_run_id.clone(),
                task_id: intent.task_id.clone(),
                device_id: selected_device_id.clone(),
                resource_kind: SchedulerResourceKind::DeviceVram,
                reserved_bytes: 8_589_934_592,
            };
            let fact = WorkflowRuntimeDispatchCandidateFact {
                candidate_id: SchedulerDispatchCandidateId::parse("candidate.runtime_worker_test")
                    .map_err(|error| worker_candidate_provider_error(error.to_string()))?,
                selected_runtime_id,
                selected_runtime_variant_id: None,
                selected_backend_key: "test-runtime".to_string(),
                runtime_family: "test-runtime".to_string(),
                resolved_load_target: format!("test:{}", intent.model_ref.model_id),
                runtime_residency_key: format!("test-runtime:{}", intent.model_ref.model_id),
                loaded_runtime_memory_estimate_bytes: 8_589_934_592,
                runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
                runtime_instance_id: Some("runtime.worker-batch-test.001".to_string()),
                selected_device_ids: vec![selected_device_id],
                selected_model_ref: intent.model_ref.clone(),
                runtime_trait_settings: Vec::new(),
                environment_ref,
                reservations: vec![reservation],
                resource_fit_assessment: SchedulerResourceFitAssessment {
                    workflow_run_id: intent.workflow_run_id.clone(),
                    task_id: intent.task_id.clone(),
                    state: SchedulerResourceFitState::Fits,
                    diagnostics: Vec::new(),
                },
                batching_group_id: None,
            };
            let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
                WorkflowRuntimeDispatchCandidateFactBundle {
                    contract_version:
                        WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                    facts: vec![fact],
                    diagnostics: Vec::new(),
                },
            )
            .map_err(|error| worker_candidate_provider_error(error.to_string()))?;
            Ok(WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle))
        }
    }

    fn worker_candidate_provider_error(
        message: impl Into<String>,
    ) -> WorkflowRuntimeDispatchCandidateProviderError {
        WorkflowRuntimeDispatchCandidateProviderError::Failed {
            message: message.into(),
        }
    }

    fn runtime_host_batch_response_from_request(
        request: &RuntimeHostBatchExecutionRequest,
    ) -> RuntimeHostBatchExecutionResponse {
        RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: request.batch_execution_request_id.clone(),
            state: RuntimeHostBatchExecutionState::Completed,
            diagnostics: Vec::new(),
            members: request
                .members
                .iter()
                .map(|member| RuntimeHostBatchExecutionMemberResponse {
                    execution_request_id: member.execution_request_id.clone(),
                    assignment_id: member.assignment_id.clone(),
                    workflow_id: member.handoff.workflow_id.clone(),
                    workflow_run_id: member.handoff.workflow_run_id.clone(),
                    node_id: member.handoff.node_id.clone(),
                    task_id: member.handoff.task_id.clone(),
                    state: RuntimeHostBatchExecutionMemberState::Completed,
                    retry_disposition: RuntimeHostBatchMemberRetryDisposition::NotRetryable,
                    reservation_disposition:
                        RuntimeHostBatchMemberReservationDisposition::RetainedForRuntimeReuse,
                    outputs: vec![RuntimeHostExecutionOutput {
                        port_id: "image".to_string(),
                        value: RuntimeHostExecutionOutputValue::String(format!(
                            "image for {}",
                            member.handoff.workflow_run_id
                        )),
                    }],
                    diagnostics: Vec::new(),
                    terminal_metadata: None,
                })
                .collect(),
        }
    }

    fn prepare_ready_runtime_branch_run(
        service: &WorkflowService,
        workflow_run_id: &str,
    ) -> String {
        let mut store = service.session_store_guard().expect("session store");
        let session_id = store
            .create_session(
                WORKER_BATCH_WORKFLOW_ID.to_string(),
                None,
                None,
                vec!["pytorch".to_string()],
                vec!["stable-diffusion-xl".to_string()],
                true,
            )
            .expect("create session");
        let run_request = run_request_for_ready_runtime_branch(&session_id);
        let queued_run_id = store
            .enqueue_run_with_id(&session_id, &run_request, workflow_run_id.to_string())
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &queued_run_id)
            .expect("begin queued run")
            .expect("dequeued run");
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                workflow_run_id,
                ready_runtime_task_graph(workflow_run_id),
                vec![ready_runtime_task_record(workflow_run_id)],
            )
            .expect("set runtime task state");
        store
            .record_active_run_runtime_dispatch_readiness_proof(
                &session_id,
                workflow_run_id,
                WORKER_BATCH_TASK_ID,
                readiness_proof_for_run(workflow_run_id),
            )
            .expect("record readiness proof");
        session_id
    }

    fn run_request_for_ready_runtime_branch(
        session_id: &str,
    ) -> WorkflowExecutionSessionRunRequest {
        WorkflowExecutionSessionRunRequest {
            session_id: session_id.to_string(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::new(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: WORKER_BATCH_NODE_ID.to_string(),
                port_id: "image".to_string(),
            }]),
            override_selection: None,
            timeout_ms: Some(500),
            priority: None,
        }
    }

    fn ready_runtime_task_graph(workflow_run_id: &str) -> WorkflowSchedulerTaskGraph {
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: SchedulerWorkflowId::parse(WORKER_BATCH_WORKFLOW_ID).expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                .expect("workflow run id"),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id: SchedulerWorkflowId::parse(WORKER_BATCH_WORKFLOW_ID)
                    .expect("workflow id"),
                workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                    .expect("workflow run id"),
                node_id: SchedulerNodeId::parse(WORKER_BATCH_NODE_ID).expect("node id"),
                task_id: SchedulerTaskId::parse(WORKER_BATCH_TASK_ID).expect("task id"),
                node_type: "llm-inference".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: Some(task_intent_for_run(workflow_run_id)),
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                runtime_source_context: Some(runtime_source_context()),
                diagnostics: Vec::new(),
            }],
        }
    }

    fn ready_runtime_task_record(workflow_run_id: &str) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse(WORKER_BATCH_WORKFLOW_ID).expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                .expect("workflow run id"),
            node_id: SchedulerNodeId::parse(WORKER_BATCH_NODE_ID).expect("node id"),
            task_id: SchedulerTaskId::parse(WORKER_BATCH_TASK_ID).expect("task id"),
            state: SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::Runtime {
                    task_intent: task_intent_for_run(workflow_run_id),
                },
            },
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse(format!(
                "transition.ready.{workflow_run_id}"
            ))
            .expect("transition id"),
        }
    }

    fn task_intent_for_run(workflow_run_id: &str) -> SchedulableTaskIntent {
        let mut intent = runtime_handoff_fixture().task_intent;
        intent.workflow_id =
            SchedulerWorkflowId::parse(WORKER_BATCH_WORKFLOW_ID).expect("workflow id");
        intent.workflow_run_id =
            SchedulerWorkflowRunId::parse(workflow_run_id).expect("workflow run id");
        intent.node_id = SchedulerNodeId::parse(WORKER_BATCH_NODE_ID).expect("node id");
        intent.task_id = SchedulerTaskId::parse(WORKER_BATCH_TASK_ID).expect("task id");
        intent
    }

    fn readiness_proof_for_run(workflow_run_id: &str) -> DependencyReadinessProofEnvelope {
        let mut proof = runtime_handoff_fixture().readiness_proof;
        proof.execution_context.workflow_run_id =
            DependencyReadinessWorkflowRunId::parse(workflow_run_id)
                .expect("readiness workflow run id");
        proof.readiness_proof_id =
            DependencyReadinessProofId::parse(format!("readiness-proof.{workflow_run_id}"))
                .expect("readiness proof id");
        proof.validate().expect("readiness proof");
        proof
    }

    fn runtime_handoff_fixture() -> SchedulerRuntimeHandoff {
        serde_json::from_str(include_str!(
            "../../../pantograph-scheduler/tests/fixtures/runtime_handoff_readiness_admitted.json"
        ))
        .expect("runtime handoff")
    }

    fn enqueue_ready_runtime_branch_event(
        service: &WorkflowService,
        session_id: &str,
        workflow_run_id: &str,
    ) -> WorkflowRuntimeBranchTaskEventId {
        let event_id = WorkflowRuntimeBranchTaskEventId::parse(format!(
            "runtime-branch-task-event.{workflow_run_id}.image-task"
        ))
        .expect("event id");
        let record =
            WorkflowRuntimeBranchTaskEventRecord::ready(WorkflowRuntimeBranchTaskEventRequest {
                event_id: event_id.clone(),
                session_id: session_id.to_string(),
                workflow_id: WORKER_BATCH_WORKFLOW_ID.to_string(),
                workflow_run_id: workflow_run_id.to_string(),
                scheduler_task_id: WORKER_BATCH_TASK_ID.to_string(),
                scheduler_task_attempt_id: None,
                attempt_generation: 1,
                queued_input_keys: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: WORKER_BATCH_NODE_ID.to_string(),
                    port_id: "image".to_string(),
                }]),
                timeout_ms: Some(500),
                batching_key: Some(format!(
                    "runtime-branch-task.{WORKER_BATCH_WORKFLOW_ID}.{WORKER_BATCH_TASK_ID}"
                )),
                runtime_source_context: runtime_source_context(),
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
        event_id
    }

    fn runtime_branch_command_for_session_run(
        session_id: &str,
        workflow_run_id: &str,
    ) -> WorkflowTaskExecutionWorkerRuntimeBranchCommand {
        WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: session_id.to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            workflow_id: WORKER_BATCH_WORKFLOW_ID.to_string(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: WORKER_BATCH_NODE_ID.to_string(),
                port_id: "image".to_string(),
            }]),
            timeout_ms: Some(500),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
        }
    }

    fn runtime_branch_event_state(
        service: &WorkflowService,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> WorkflowRuntimeBranchTaskEventState {
        service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .get(event_id)
            .expect("runtime branch task event")
            .state
    }

    fn runtime_dispatch_assignment_for_event(
        service: &WorkflowService,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> WorkflowRuntimeDispatchAssignmentRecord {
        let assignment_id = service
            .runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch task event repository")
            .get(event_id)
            .expect("runtime branch task event")
            .dispatch_assignment_link
            .expect("runtime branch dispatch assignment link")
            .assignment_id;
        service
            .runtime_dispatch_assignment_repository
            .lock()
            .expect("runtime dispatch assignment repository")
            .get(&assignment_id)
            .expect("runtime dispatch assignment")
    }

    fn runtime_dispatch_assignment_by_id(
        service: &WorkflowService,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    ) -> WorkflowRuntimeDispatchAssignmentRecord {
        service
            .runtime_dispatch_assignment_repository
            .lock()
            .expect("runtime dispatch assignment repository")
            .get(assignment_id)
            .expect("runtime dispatch assignment")
    }

    fn assert_runtime_branch_completed_response(
        outcome: WorkflowTaskExecutionWorkerOutcome,
        workflow_run_id: &str,
        image: &str,
    ) {
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(outcome) = outcome else {
            panic!("expected runtime branch completed outcome");
        };
        assert_eq!(outcome.workflow_run_id, workflow_run_id);
        assert_eq!(outcome.response.workflow_run_id, workflow_run_id);
        let output = outcome
            .response
            .outputs
            .iter()
            .find(|output| output.node_id == WORKER_BATCH_NODE_ID && output.port_id == "image")
            .expect("image output");
        assert_eq!(output.value.as_str(), Some(image));
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
                runtime_source_context: None,
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

    #[tokio::test]
    async fn runtime_branch_responder_registry_completes_registered_workflow_run() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let command = runtime_branch_command_for_run("run.registry");
        let (completion_responder, completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let registration = registry
            .register_workflow_run(&command, completion_responder)
            .expect("register workflow run responder");
        assert_eq!(registry.active_responder_count(), 1);
        let expected = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            &command,
            "runtime branch failed after dispatch",
            Vec::new(),
        );

        registry
            .complete(registration, expected.clone())
            .expect("complete registered responder");

        assert_eq!(registry.active_responder_count(), 0);
        assert_eq!(
            completion_rx.await.expect("registered responder outcome"),
            expected
        );
    }

    #[tokio::test]
    async fn runtime_branch_responder_registry_attaches_responder_to_dispatch_assignment() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let command = runtime_branch_command_for_run("run.assignment");
        let assignment_id = WorkflowRuntimeDispatchAssignmentId::parse("assignment.registry")
            .expect("assignment id");
        let (completion_responder, completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let registration = registry
            .register_workflow_run(&command, completion_responder)
            .expect("register workflow run responder");
        let expected = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            &command,
            "runtime branch failed after assignment",
            Vec::new(),
        );

        let assignment_registration = registry
            .attach_runtime_dispatch_assignment(&registration, &assignment_id, None)
            .expect("attach assignment responder");

        assert_eq!(registry.active_responder_count(), 1);
        registry
            .complete_runtime_dispatch_assignments(vec![
                WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                    assignment_id: assignment_registration
                        .runtime_dispatch_assignment_id
                        .expect("attached assignment id"),
                    session_id: command.session_id.clone(),
                    workflow_run_id: command.workflow_run_id.clone(),
                    workflow_id: command.workflow_id.clone(),
                    outcome: expected.clone(),
                },
            ])
            .expect("fan out attached responder");

        assert_eq!(registry.active_responder_count(), 0);
        assert_eq!(
            completion_rx.await.expect("attached responder outcome"),
            expected
        );
    }

    #[tokio::test]
    async fn runtime_branch_dropped_terminal_receiver_preserves_other_run_continuation() {
        let port = Arc::new(RecordingWorkerBatchExecutionPort::default());
        let service = Arc::new(
            WorkflowService::new()
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    SingleCanonicalRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_batch_execution_port(port.clone())
                .with_reservation_lifecycle_port(Arc::new(
                    RecordingReservationLifecyclePort::default(),
                )),
        );
        let run_id = "run.continue.connected";
        let session_id = prepare_ready_runtime_branch_run(service.as_ref(), run_id);
        let _event_id = enqueue_ready_runtime_branch_event(service.as_ref(), &session_id, run_id);
        let environment = WorkflowTaskExecutionWorkerRuntimeBranchEnvironment::new(
            Arc::clone(&service),
            test_host(),
        );
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let command = runtime_branch_command_for_session_run(&session_id, run_id);
        let (responder, receiver) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let registration = registry
            .register_workflow_run(&command, responder)
            .expect("connected run");
        let assignment_id = WorkflowRuntimeDispatchAssignmentId::parse("assignment.previous")
            .expect("previous assignment");
        registry
            .attach_runtime_dispatch_assignment(&registration, &assignment_id, None)
            .expect("previous assignment responder");
        let continuation = registry
            .continue_workflow_run(&WorkflowRuntimeBranchBatchMemberExecutionOutcome {
                assignment_id,
                session_id: session_id.clone(),
                workflow_id: command.workflow_id.clone(),
                workflow_run_id: run_id.to_string(),
                state: WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Continue,
                completed_response: None,
                diagnostics: Vec::new(),
            })
            .expect("retain successful member");

        let disconnected = runtime_branch_command_for_run("run.disconnected");
        let (responder, disconnected_receiver) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let registration = registry
            .register_workflow_run(&disconnected, responder)
            .expect("disconnected run");
        let assignment_id = WorkflowRuntimeDispatchAssignmentId::parse("assignment.disconnected")
            .expect("terminal assignment");
        registry
            .attach_runtime_dispatch_assignment(&registration, &assignment_id, None)
            .expect("terminal assignment responder");
        drop(disconnected_receiver);
        registry
            .complete_runtime_dispatch_assignments(vec![
                WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                    assignment_id,
                    session_id: disconnected.session_id.clone(),
                    workflow_run_id: disconnected.workflow_run_id.clone(),
                    workflow_id: disconnected.workflow_id.clone(),
                    outcome: WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                        &disconnected,
                        "terminal peer failed",
                        Vec::new(),
                    ),
                },
            ])
            .expect("disconnected terminal notification must not discard successful continuation");

        drive_runtime_branch_continuations(&environment, &registry, vec![continuation]).await;
        assert_runtime_branch_completed_response(
            receiver.await.expect("connected run response"),
            run_id,
            "image for run.continue.connected",
        );
        assert_eq!(port.requests().len(), 1);
        assert_eq!(registry.active_responder_count(), 0);
    }

    #[tokio::test]
    async fn runtime_branch_continuations_retain_each_run_responder_and_supervised_owner() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let task_registry = registry.clone();
        let (task_id, mut receivers) = tokio::spawn(async move {
            let mut receivers = Vec::new();
            for run_id in ["run.continue.first", "run.continue.second"] {
                let command = runtime_branch_command_for_run(run_id);
                let (responder, receiver) =
                    WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
                let registration = task_registry
                    .register_workflow_run(&command, responder)
                    .expect("run registration");
                let assignment_id =
                    WorkflowRuntimeDispatchAssignmentId::parse(format!("assignment.{run_id}"))
                        .expect("assignment id");
                task_registry
                    .attach_runtime_dispatch_assignment(&registration, &assignment_id, None)
                    .expect("assignment registration");
                let continuation = task_registry
                    .continue_workflow_run(&WorkflowRuntimeBranchBatchMemberExecutionOutcome {
                        assignment_id,
                        session_id: command.session_id.clone(),
                        workflow_id: command.workflow_id.clone(),
                        workflow_run_id: command.workflow_run_id.clone(),
                        state: WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Continue,
                        completed_response: None,
                        diagnostics: Vec::new(),
                    })
                    .expect("continue same responder");
                assert_eq!(continuation.command, command);
                assert_eq!(continuation.registration.key, registration.key);
                receivers.push(receiver);
            }
            (tokio::task::id(), receivers)
        })
        .await
        .expect("supervised task");
        for receiver in &mut receivers {
            assert!(matches!(
                receiver.try_recv(),
                Err(tokio::sync::oneshot::error::TryRecvError::Empty)
            ));
        }
        assert_eq!(registry.active_responder_count(), 2);
        registry.supervise_task_exit(
            &WorkflowService::new(),
            task_id,
            "continuation owner failed",
        );
        for receiver in receivers {
            let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) =
                receiver.await.expect("supervised response")
            else {
                panic!("unfinished continuation must fail on owner exit");
            };
            assert!(outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message == "continuation owner failed"));
        }
        assert_eq!(registry.active_responder_count(), 0);
    }

    #[tokio::test]
    async fn runtime_branch_responder_registry_fans_out_assignment_completions() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let first_command = runtime_branch_command_for_run("run.fanout.first");
        let second_command = runtime_branch_command_for_run("run.fanout.second");
        let first_assignment_id =
            WorkflowRuntimeDispatchAssignmentId::parse("assignment.fanout.first")
                .expect("first assignment id");
        let second_assignment_id =
            WorkflowRuntimeDispatchAssignmentId::parse("assignment.fanout.second")
                .expect("second assignment id");
        let (first_responder, first_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let (second_responder, second_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let first_registration = registry
            .register_workflow_run(&first_command, first_responder)
            .expect("register first workflow run responder");
        let second_registration = registry
            .register_workflow_run(&second_command, second_responder)
            .expect("register second workflow run responder");
        let _first_assignment_registration = registry
            .attach_runtime_dispatch_assignment(&first_registration, &first_assignment_id, None)
            .expect("attach first assignment responder");
        let _second_assignment_registration = registry
            .attach_runtime_dispatch_assignment(&second_registration, &second_assignment_id, None)
            .expect("attach second assignment responder");
        let first_expected = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            &first_command,
            "first runtime branch failed after batch",
            Vec::new(),
        );
        let second_expected = WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            &second_command,
            "second runtime branch failed after batch",
            Vec::new(),
        );

        registry
            .complete_runtime_dispatch_assignments(vec![
                WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                    assignment_id: first_assignment_id,
                    session_id: first_command.session_id.clone(),
                    workflow_run_id: first_command.workflow_run_id.clone(),
                    workflow_id: first_command.workflow_id.clone(),
                    outcome: first_expected.clone(),
                },
                WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                    assignment_id: second_assignment_id,
                    session_id: second_command.session_id.clone(),
                    workflow_run_id: second_command.workflow_run_id.clone(),
                    workflow_id: second_command.workflow_id.clone(),
                    outcome: second_expected.clone(),
                },
            ])
            .expect("fan out assignment completions");

        assert_eq!(registry.active_responder_count(), 0);
        assert_eq!(
            first_completion_rx.await.expect("first fan-out outcome"),
            first_expected
        );
        assert_eq!(
            second_completion_rx.await.expect("second fan-out outcome"),
            second_expected
        );
    }

    #[test]
    fn runtime_branch_responder_registry_rejects_duplicate_workflow_run_registration() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let command = runtime_branch_command_for_run("run.duplicate");
        let (first_responder, _first_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let (second_responder, _second_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let _registration = registry
            .register_workflow_run(&command, first_responder)
            .expect("register first responder");

        let failure = registry
            .register_workflow_run(&command, second_responder)
            .expect_err("duplicate workflow-run responder must fail closed");

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = failure.outcome
        else {
            panic!("expected runtime branch failure");
        };
        assert_eq!(outcome.workflow_run_id, "run.duplicate");
        assert_eq!(
            outcome.diagnostics[0].code,
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed
        );
        assert!(
            outcome.diagnostics[0]
                .message
                .contains("already registered"),
            "unexpected diagnostic: {}",
            outcome.diagnostics[0].message
        );
        assert_eq!(registry.active_responder_count(), 1);
    }

    #[test]
    fn runtime_branch_responder_registry_rejects_duplicate_assignment_attachment() {
        let registry = WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry::new();
        let first_command = runtime_branch_command_for_run("run.assignment.first");
        let second_command = runtime_branch_command_for_run("run.assignment.second");
        let assignment_id = WorkflowRuntimeDispatchAssignmentId::parse("assignment.duplicate")
            .expect("assignment id");
        let (first_responder, _first_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let (second_responder, _second_completion_rx) =
            WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder::channel();
        let first_registration = registry
            .register_workflow_run(&first_command, first_responder)
            .expect("register first responder");
        let second_registration = registry
            .register_workflow_run(&second_command, second_responder)
            .expect("register second responder");
        let _attached = registry
            .attach_runtime_dispatch_assignment(&first_registration, &assignment_id, None)
            .expect("attach first responder");

        let failure = registry
            .attach_runtime_dispatch_assignment(&second_registration, &assignment_id, None)
            .expect_err("duplicate assignment responder must fail closed");

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = failure else {
            panic!("expected runtime branch failure");
        };
        assert_eq!(outcome.workflow_run_id, "run.assignment.second");
        assert_eq!(
            outcome.diagnostics[0].code,
            WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchResponderRegistrationFailed
        );
        assert!(
            outcome.diagnostics[0].message.contains("already attached"),
            "unexpected diagnostic: {}",
            outcome.diagnostics[0].message
        );
        assert_eq!(registry.active_responder_count(), 2);
    }

    fn runtime_branch_command() -> WorkflowTaskExecutionWorkerRuntimeBranchCommand {
        runtime_branch_command_for_run("run-1")
    }

    fn runtime_branch_command_for_run(
        workflow_run_id: &str,
    ) -> WorkflowTaskExecutionWorkerRuntimeBranchCommand {
        WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            workflow_id: "workflow-1".to_string(),
            output_targets: None,
            timeout_ms: Some(500),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Redispatched,
        }
    }

    fn assert_runtime_branch_event_unavailable(
        outcome: WorkflowTaskExecutionWorkerOutcome,
        workflow_run_id: &str,
    ) {
        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = outcome else {
            panic!("expected fail-closed runtime branch outcome");
        };
        assert_eq!(outcome.workflow_run_id, workflow_run_id);
        assert_eq!(
            outcome.diagnostics,
            vec![WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchFailed,
                outcome.error_message.clone(),
            )]
        );
        assert!(
            outcome.error_message.contains("session"),
            "unexpected error message: {}",
            outcome.error_message
        );
    }

    fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
        crate::graph::WorkflowRuntimeSourceContext {
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }
}
