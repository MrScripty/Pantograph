// This worker owner lands before queue progression is migrated from the
// request path into composition-root wiring.
#![cfg_attr(not(test), allow(dead_code))]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState,
};
use crate::workflow::WorkflowServiceError;

/// Workflow-service owner for the scheduler queue worker lifecycle.
///
/// This slice introduces only the bounded worker owner and wake/shutdown
/// mechanics. Queue progression remains in the existing request path until the
/// next migration slice moves that business loop behind this owner.
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
