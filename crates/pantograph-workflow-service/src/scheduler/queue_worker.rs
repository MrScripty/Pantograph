#![cfg_attr(not(test), allow(dead_code))]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState,
};
use crate::scheduler::{WorkflowExecutionSessionDequeuedRun, WorkflowExecutionSessionStore};
use crate::workflow::WorkflowServiceError;

use super::store::WORKFLOW_SESSION_QUEUE_POLL_MS;

/// Workflow-service owner for the scheduler queue worker lifecycle.
///
/// The worker owns queue lifecycle state and the queue admission polling loop.
/// Later slices move execution progression and completion signaling behind the
/// same owner.
#[derive(Debug)]
pub(crate) struct WorkflowSchedulerQueueWorker {
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    wake_notify: Arc<tokio::sync::Notify>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    join_handle: tokio::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
    observed_wakes: Arc<AtomicU64>,
}

impl WorkflowSchedulerQueueWorker {
    pub(crate) fn spawn(
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    ) -> Result<Self, WorkflowServiceError> {
        let runtime_handle = tokio::runtime::Handle::try_current().map_err(|_| {
            WorkflowServiceError::Internal(
                "scheduler queue worker requires an active Tokio runtime".to_string(),
            )
        })?;
        Self::spawn_with_handle(scheduler_lifecycle, runtime_handle)
    }

    pub(crate) fn spawn_with_handle(
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
        runtime_handle: tokio::runtime::Handle,
    ) -> Result<Self, WorkflowServiceError> {
        scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::QueueWorker,
                WorkflowSchedulerLifecycleComponentState::Running,
            )
            .map(|_record| ())?;

        let wake_notify = Arc::new(tokio::sync::Notify::new());
        let observed_wakes = Arc::new(AtomicU64::new(0));
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let join_handle = runtime_handle.spawn(queue_worker_loop(
            scheduler_lifecycle.clone(),
            Arc::clone(&wake_notify),
            shutdown_rx,
            Arc::clone(&observed_wakes),
        ));

        Ok(Self {
            scheduler_lifecycle,
            wake_notify,
            shutdown_tx,
            join_handle: tokio::sync::Mutex::new(Some(join_handle)),
            observed_wakes,
        })
    }

    pub(crate) fn wake(&self) {
        self.wake_notify.notify_one();
    }

    pub(crate) async fn admit_queued_run(
        command: WorkflowSchedulerQueueAdmissionCommand,
    ) -> Result<WorkflowExecutionSessionDequeuedRun, WorkflowServiceError> {
        loop {
            let maybe_queued = {
                let mut store = command.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal(
                        "workflow execution session store lock poisoned".to_string(),
                    )
                })?;
                store.begin_queued_run(&command.session_id, &command.workflow_run_id)?
            };
            if let Some(queued) = maybe_queued {
                return Ok(queued);
            }
            tokio::time::sleep(std::time::Duration::from_millis(
                WORKFLOW_SESSION_QUEUE_POLL_MS,
            ))
            .await;
        }
    }

    pub(crate) async fn shutdown(&self) -> Result<(), WorkflowServiceError> {
        self.mark_shutting_down_if_running()?;
        let _ = self.shutdown_tx.send(true);
        if let Some(join_handle) = self.join_handle.lock().await.take() {
            join_handle.await.map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "scheduler queue worker join failed during shutdown: {error}"
                ))
            })?;
        }
        self.mark_shutdown()
    }

    #[cfg(test)]
    pub(crate) fn observed_wake_count(&self) -> u64 {
        self.observed_wakes.load(Ordering::SeqCst)
    }

    fn mark_shutting_down_if_running(&self) -> Result<(), WorkflowServiceError> {
        let current = self
            .scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::QueueWorker)?;
        if current.state == WorkflowSchedulerLifecycleComponentState::Shutdown {
            return Ok(());
        }
        self.scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::QueueWorker,
                WorkflowSchedulerLifecycleComponentState::ShuttingDown,
            )
            .map(|_record| ())
    }

    fn mark_shutdown(&self) -> Result<(), WorkflowServiceError> {
        self.scheduler_lifecycle
            .update_component_state(
                WorkflowSchedulerLifecycleComponentKind::QueueWorker,
                WorkflowSchedulerLifecycleComponentState::Shutdown,
            )
            .map(|_record| ())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSchedulerQueueAdmissionCommand {
    session_store: Arc<Mutex<WorkflowExecutionSessionStore>>,
    session_id: String,
    workflow_run_id: String,
}

impl WorkflowSchedulerQueueAdmissionCommand {
    pub(crate) fn new(
        session_store: Arc<Mutex<WorkflowExecutionSessionStore>>,
        session_id: impl Into<String>,
        workflow_run_id: impl Into<String>,
    ) -> Self {
        Self {
            session_store,
            session_id: session_id.into(),
            workflow_run_id: workflow_run_id.into(),
        }
    }
}

async fn queue_worker_loop(
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    wake_notify: Arc<tokio::sync::Notify>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    observed_wakes: Arc<AtomicU64>,
) {
    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            _ = wake_notify.notified() => {
                observed_wakes.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    let _ = scheduler_lifecycle.update_component_state(
        WorkflowSchedulerLifecycleComponentKind::QueueWorker,
        WorkflowSchedulerLifecycleComponentState::Shutdown,
    );
}

#[cfg(test)]
#[path = "queue_worker_tests.rs"]
mod tests;
