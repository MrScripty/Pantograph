use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::scheduler::{
    unix_timestamp_ms, WorkflowSchedulerLifecycleComponentKind,
    WorkflowSchedulerLifecycleComponentRegistryHandle, WorkflowSchedulerLifecycleComponentState,
};

use super::runtime_branch_batch_execution::{
    WorkflowRuntimeBranchBatchExecutionDiagnostic,
    WorkflowRuntimeBranchBatchExecutionDiagnosticCode, WorkflowRuntimeBranchBatchExecutionFailure,
    WorkflowRuntimeBranchBatchExecutionMember, WorkflowRuntimeBranchBatchExecutionOwner,
    WorkflowRuntimeBranchBatchMemberExecutionOutcome,
    WorkflowRuntimeBranchBatchMemberExecutionOutcomeState,
    WorkflowRuntimeBranchBatchResponderFanOut,
};
use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventClaim, WorkflowRuntimeBranchTaskEventClaimOutcome,
    WorkflowRuntimeBranchTaskEventClaimOwnerId, WorkflowRuntimeBranchTaskEventDiagnostic,
    WorkflowRuntimeBranchTaskEventDiagnosticCode, WorkflowRuntimeBranchTaskEventId,
    WorkflowRuntimeBranchTaskEventRecord, WorkflowRuntimeBranchTaskEventRepository,
};
use super::runtime_dispatch_assignment::{
    WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest,
    WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
    WorkflowRuntimeDispatchAssignmentBatchBrokerRequest,
    WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest,
    WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
    WorkflowRuntimeDispatchAssignmentBatchClaimOwnerId,
    WorkflowRuntimeDispatchAssignmentDiagnostic, WorkflowRuntimeDispatchAssignmentDiagnosticCode,
    WorkflowRuntimeDispatchAssignmentId, WorkflowRuntimeDispatchAssignmentRecord,
    WorkflowRuntimeDispatchAssignmentRepository, WorkflowRuntimeDispatchAssignmentRequest,
    WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_BATCH_BROKER_WAIT_WINDOW_MS,
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
const RUNTIME_BRANCH_BATCH_BROKER_MIN_ASSIGNMENTS: usize = 2;
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
    session_id: String,
    workflow_run_id: String,
    workflow_id: String,
    completion_responder: WorkflowTaskExecutionWorkerRuntimeBranchCompletionResponder,
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

enum WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult {
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
            session_id: command.session_id.clone(),
            workflow_run_id: command.workflow_run_id.clone(),
            workflow_id: command.workflow_id.clone(),
            completion_responder,
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
        let registered = responders.remove(&registration.key).ok_or_else(|| {
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

        let mut first_error = None;
        for (completion_responder, outcome) in pending_notifications {
            if let Err(outcome) = completion_responder.complete(outcome) {
                first_error.get_or_insert(outcome);
            }
        }
        match first_error {
            Some(outcome) => Err(outcome),
            None => Ok(()),
        }
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
                            let mut registration = match runtime_branch_responder_registry
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
                            let outcome = claim_and_execute_runtime_branch_event(
                                &runtime_branch_environment,
                                &command,
                                &runtime_branch_responder_registry,
                                &mut registration,
                            ).await;
                            if let WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::CompleteResponder(outcome) = outcome {
                                if let Some(assignment_id) =
                                    registration.runtime_dispatch_assignment_id.clone()
                                {
                                    let completion =
                                        WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
                                            assignment_id,
                                            session_id: registration.session_id,
                                            workflow_run_id: registration.workflow_run_id,
                                            workflow_id: registration.workflow_id,
                                            outcome,
                                        };
                                    let _ = runtime_branch_responder_registry
                                        .complete_runtime_dispatch_assignments(vec![completion]);
                                } else {
                                    let _ = runtime_branch_responder_registry
                                        .complete(registration, outcome);
                                }
                            }
                        });
                    }
                    Some(WorkflowTaskExecutionWorkerCommand::Shutdown(_)) | None => {
                        accepting_commands = false;
                    }
                }
            }
            Some(join_result) = runtime_branch_tasks.join_next(), if !runtime_branch_tasks.is_empty() => {
                let _ = join_result;
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
    let now_ms = unix_timestamp_ms();
    let claimed =
        match claim_runtime_branch_task_event_for_worker(service.as_ref(), command, now_ms) {
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

    let dispatching_record = match mark_claimed_runtime_branch_task_event_dispatching(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        now_ms,
    ) {
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

    let active_run_inputs = match runtime_branch_active_run_inputs(service.as_ref(), command) {
        Ok(inputs) => inputs,
        Err(error) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                fail_runtime_branch_preparation_error(
                    command,
                    service.as_ref(),
                    &dispatching_record.event_id,
                    &claimed.claim,
                    error,
                ),
            );
        }
    };
    let preparation_boundary = WorkflowPreDispatchPreparationBoundary::new(service.as_ref());
    if let Err(error) = preparation_boundary.materialize_external_inputs(
        &command.session_id,
        &command.workflow_run_id,
        &active_run_inputs,
    ) {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            fail_runtime_branch_preparation_error(
                command,
                service.as_ref(),
                &dispatching_record.event_id,
                &claimed.claim,
                error,
            ),
        );
    }
    let preparation = match preparation_boundary
        .prepare_runtime_dispatch(&command.session_id, &command.workflow_run_id)
        .await
    {
        Ok(preparation) => preparation,
        Err(error) if error.is_runtime_dependency_readiness_pending() => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                defer_runtime_branch_dependency_readiness(
                    command,
                    service.as_ref(),
                    &dispatching_record.event_id,
                    &claimed.claim,
                    runtime_dependency_pending_task_ids(&error).unwrap_or_default(),
                    error.to_string(),
                ),
            );
        }
        Err(error) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                fail_runtime_branch_preparation_error(
                    command,
                    service.as_ref(),
                    &dispatching_record.event_id,
                    &claimed.claim,
                    error,
                ),
            );
        }
    };
    if !preparation.deferred_task_ids().is_empty() {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            defer_runtime_branch_dependency_readiness(
                command,
                service.as_ref(),
                &dispatching_record.event_id,
                &claimed.claim,
                preparation.deferred_task_ids().to_vec(),
                format!(
                    "runtime dependency readiness is pending for scheduler task(s): {}",
                    preparation.deferred_task_ids().join(", ")
                ),
            ),
        );
    }
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
                    error,
                ),
            );
        }
    };
    let evidence_record = match record_runtime_branch_selected_candidate_fact(
        service.as_ref(),
        &dispatching_record.event_id,
        &claimed.claim,
        started_dispatch.selected_candidate_fact.clone(),
    ) {
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
    match runtime_branch_responder_registry.attach_runtime_dispatch_assignment(
        runtime_branch_responder_registration,
        &dispatch_assignment.assignment_id,
    ) {
        Ok(registration) => {
            *runtime_branch_responder_registration = registration;
        }
        Err(outcome) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome);
        }
    }
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
        unix_timestamp_ms(),
    ) {
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

    if let Err(diagnostic) = mark_claimed_runtime_branch_task_event_running(
        service.as_ref(),
        &claimed.record.event_id,
        &claimed.claim,
        unix_timestamp_ms(),
    ) {
        return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
            WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
                command,
                "runtime branch task event running persistence failed",
                vec![diagnostic],
            ),
        );
    }

    let broker_decision = match evaluate_runtime_branch_batch_broker(
        service.as_ref(),
        &dispatch_assignment.assignment_id,
        unix_timestamp_ms(),
    ) {
        Ok(decision) => decision,
        Err(diagnostic) => {
            return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                fail_runtime_branch_dispatch_diagnostic(
                    command,
                    service.as_ref(),
                    &claimed.record.event_id,
                    &claimed.claim,
                    "runtime branch batch broker decision failed",
                    diagnostic,
                ),
            );
        }
    };

    match broker_decision {
        WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::WaitingForPeers { .. } => {
            if let Err(diagnostic) = record_runtime_branch_batch_broker_wait_window(
                service.as_ref(),
                broker_decision,
                unix_timestamp_ms(),
            ) {
                return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                    fail_runtime_branch_dispatch_diagnostic(
                        command,
                        service.as_ref(),
                        &claimed.record.event_id,
                        &claimed.claim,
                        "runtime branch batch broker wait-window persistence failed",
                        diagnostic,
                    ),
                );
            }
            WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch
        }
        WorkflowRuntimeDispatchAssignmentBatchBrokerDecision::ReadyToClaim { .. } => {
            let claim_outcome = match claim_runtime_branch_batch_broker_decision(
                service.as_ref(),
                broker_decision,
                unix_timestamp_ms(),
            ) {
                Ok(claim_outcome) => claim_outcome,
                Err(diagnostic) => {
                    return WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(
                        fail_runtime_branch_dispatch_diagnostic(
                            command,
                            service.as_ref(),
                            &claimed.record.event_id,
                            &claimed.claim,
                            "runtime branch batch broker claim failed",
                            diagnostic,
                        ),
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
                Ok(()) => {
                    WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch
                }
                Err(outcome) => {
                    WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::complete(outcome)
                }
            }
        }
    }
}

fn fail_runtime_branch_dispatch_diagnostic(
    command: &WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    error_message: impl Into<String>,
    diagnostic: WorkflowTaskExecutionWorkerDiagnostic,
) -> WorkflowTaskExecutionWorkerOutcome {
    match fail_claimed_runtime_branch_task_event(service, event_id, claim, unix_timestamp_ms()) {
        Ok(_record) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            error_message,
            vec![diagnostic],
        ),
        Err(failure_diagnostic) => WorkflowTaskExecutionWorkerOutcome::runtime_branch_failed(
            command,
            "runtime branch task event failure persistence failed",
            vec![diagnostic, failure_diagnostic],
        ),
    }
}

async fn execute_runtime_branch_batch_claim(
    environment: &WorkflowTaskExecutionWorkerRuntimeBranchEnvironment,
    runtime_branch_responder_registry: &WorkflowTaskExecutionWorkerRuntimeBranchResponderRegistry,
    claim_outcome: WorkflowRuntimeDispatchAssignmentBatchClaimOutcome,
) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
    let service = environment.service();
    let host = environment.host();
    let assignments = claim_outcome.assignments.clone();
    let owner = WorkflowRuntimeBranchBatchExecutionOwner::new(
        &service.scheduler_task_orchestrator,
        runtime_branch_responder_registry,
    );
    let member_outcomes = match owner
        .execute_claimed_batch(service.as_ref(), host.as_ref(), claim_outcome)
        .await
    {
        Ok(outcome) => outcome.member_outcomes,
        Err(failure) => batch_failure_member_outcomes(&assignments, failure),
    };
    let completions =
        runtime_branch_batch_member_completions(service.as_ref(), &assignments, member_outcomes);
    runtime_branch_responder_registry.complete_runtime_dispatch_assignments(completions)
}

fn evaluate_runtime_branch_batch_broker(
    service: &WorkflowService,
    assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    now_ms: u64,
) -> Result<
    WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
    WorkflowTaskExecutionWorkerDiagnostic,
> {
    let repository = service
        .runtime_dispatch_assignment_repository
        .lock()
        .map_err(|_| {
            WorkflowTaskExecutionWorkerDiagnostic::new(
                WorkflowTaskExecutionWorkerDiagnosticCode::RuntimeBranchDispatchUnavailable,
                "runtime dispatch-assignment repository lock poisoned",
            )
        })?;
    repository
        .evaluate_running_batch_broker_decision(
            WorkflowRuntimeDispatchAssignmentBatchBrokerRequest {
                anchor_assignment_id: assignment_id.clone(),
                now_ms,
                min_assignments: RUNTIME_BRANCH_BATCH_BROKER_MIN_ASSIGNMENTS,
                max_assignments: RUNTIME_BRANCH_BATCH_BROKER_MAX_ASSIGNMENTS,
            },
        )
        .map_err(runtime_dispatch_assignment_diagnostic)
}

fn claim_runtime_branch_batch_broker_decision(
    service: &WorkflowService,
    decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
    now_ms: u64,
) -> Result<WorkflowRuntimeDispatchAssignmentBatchClaimOutcome, WorkflowTaskExecutionWorkerDiagnostic>
{
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
    repository
        .claim_batch_broker_decision(WorkflowRuntimeDispatchAssignmentBatchBrokerClaimRequest {
            decision,
            owner_id,
            now_ms,
            lease_duration_ms: RUNTIME_BRANCH_BATCH_CLAIM_LEASE_MS,
        })
        .map_err(runtime_dispatch_assignment_diagnostic)
}

fn record_runtime_branch_batch_broker_wait_window(
    service: &WorkflowService,
    decision: WorkflowRuntimeDispatchAssignmentBatchBrokerDecision,
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
        .record_batch_broker_waiting_decision(
            WorkflowRuntimeDispatchAssignmentBatchBrokerWaitRequest {
                decision,
                now_ms,
                wait_window_duration_ms:
                    WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_BATCH_BROKER_WAIT_WINDOW_MS,
            },
        )
        .map_err(runtime_dispatch_assignment_diagnostic)
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
            ))
        })
        .collect()
}

fn runtime_branch_batch_member_completion(
    service: &WorkflowService,
    assignment: &WorkflowRuntimeDispatchAssignmentRecord,
    member_outcome: WorkflowRuntimeBranchBatchMemberExecutionOutcome,
) -> WorkflowTaskExecutionWorkerRuntimeBranchResponderAssignmentCompletion {
    let mut diagnostics = member_outcome
        .diagnostics
        .iter()
        .cloned()
        .map(runtime_branch_batch_execution_diagnostic)
        .collect::<Vec<_>>();
    let outcome = match member_outcome.state {
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Completed => {
            match complete_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(),
            ) {
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
                    .saturating_add(RUNTIME_BRANCH_DEPENDENCY_READINESS_RETRY_DELAY_MS),
            ) {
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
        WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Cancelled
        | WorkflowRuntimeBranchBatchMemberExecutionOutcomeState::Failed => {
            match fail_claimed_runtime_branch_task_event(
                service,
                &assignment.runtime_branch_event_id,
                &assignment.runtime_branch_claim,
                unix_timestamp_ms(),
            ) {
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

fn record_runtime_branch_selected_candidate_fact(
    service: &WorkflowService,
    event_id: &WorkflowRuntimeBranchTaskEventId,
    claim: &WorkflowRuntimeBranchTaskEventClaim,
    selected_candidate_fact: WorkflowRuntimeDispatchCandidateFact,
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
        .record_selected_candidate_fact(event_id, claim, selected_candidate_fact)
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
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::BatchCompatibilityRejected
        | WorkflowRuntimeDispatchAssignmentDiagnosticCode::InvalidBatchBrokerWaitWindow => {
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
    use crate::workflow::runtime_dispatch_assignment::WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode;
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
        RuntimeHostBatchExecutionMemberResponse, RuntimeHostBatchExecutionMemberState,
        RuntimeHostBatchExecutionPort, RuntimeHostBatchExecutionRequest,
        RuntimeHostBatchExecutionResponse, RuntimeHostBatchExecutionState,
        RuntimeHostBatchMemberReservationDisposition, RuntimeHostBatchMemberRetryDisposition,
        RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionOutput,
        RuntimeHostExecutionOutputValue, RuntimeHostExecutionPortError,
        RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
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
    async fn task_execution_worker_batches_runtime_branch_peers_and_fans_out_responses() {
        let batch_port = Arc::new(RecordingWorkerBatchExecutionPort::default());
        let service = Arc::new(
            WorkflowService::new()
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    SingleCanonicalRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_batch_execution_port(batch_port.clone()),
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

        assert!(matches!(
            first_result,
            WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(25), &mut first_completion_rx)
                .await
                .is_err(),
            "first branch responder must wait for a compatible batch peer"
        );
        assert!(
            batch_port.requests().is_empty(),
            "waiting for peers must not dispatch a one-member runtime-host batch"
        );
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &first_event_id),
            WorkflowRuntimeBranchTaskEventState::Running
        );
        let first_waiting_assignment =
            runtime_dispatch_assignment_for_event(service.as_ref(), &first_event_id);
        let first_wait_window = first_waiting_assignment
            .batch_broker_wait_window
            .as_ref()
            .expect("first branch wait window");
        assert_eq!(
            first_wait_window.required_assignments,
            RUNTIME_BRANCH_BATCH_BROKER_MIN_ASSIGNMENTS
        );
        assert_eq!(
            first_wait_window.expiry_diagnostic.code,
            WorkflowRuntimeDispatchAssignmentBatchBrokerWaitExpiryDiagnosticCode::BatchWindowExpired
        );
        assert_eq!(
            first_wait_window.expires_at_ms,
            first_wait_window
                .waiting_since_ms
                .saturating_add(WORKFLOW_RUNTIME_DISPATCH_ASSIGNMENT_BATCH_BROKER_WAIT_WINDOW_MS,)
        );

        let second_result = claim_and_execute_runtime_branch_event(
            &environment,
            &second_command,
            &registry,
            &mut second_registration,
        )
        .await;

        assert!(matches!(
            second_result,
            WorkflowTaskExecutionWorkerRuntimeBranchExecutionResult::ResponderRetainedForBatch
        ));
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
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].members.len(), 2);
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &first_event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert_eq!(
            runtime_branch_event_state(service.as_ref(), &second_event_id),
            WorkflowRuntimeBranchTaskEventState::Completed
        );
        assert!(
            runtime_dispatch_assignment_for_event(service.as_ref(), &first_event_id)
                .batch_broker_wait_window
                .is_none(),
            "claiming the batch must clear the prior wait window"
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

    const WORKER_BATCH_WORKFLOW_ID: &str = "workflow.image_generation";
    const WORKER_BATCH_NODE_ID: &str = "node.llm_inference";
    const WORKER_BATCH_TASK_ID: &str = "task.image_generation.001";

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
            .attach_runtime_dispatch_assignment(&registration, &assignment_id)
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
            .attach_runtime_dispatch_assignment(&first_registration, &first_assignment_id)
            .expect("attach first assignment responder");
        let _second_assignment_registration = registry
            .attach_runtime_dispatch_assignment(&second_registration, &second_assignment_id)
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
            .attach_runtime_dispatch_assignment(&first_registration, &assignment_id)
            .expect("attach first responder");

        let failure = registry
            .attach_runtime_dispatch_assignment(&second_registration, &assignment_id)
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
    }

    fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
        crate::graph::WorkflowRuntimeSourceContext {
            operation_type: "image-generation.txt2img".to_string(),
            context_shape_key: "txt2img.1024x1024.steps30".to_string(),
            cancellation_mode: "per-run-fanout".to_string(),
        }
    }
}
