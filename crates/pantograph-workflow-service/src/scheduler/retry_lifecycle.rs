#[cfg(test)]
use crate::scheduler::lifecycle::WorkflowSchedulerLifecycleComponentRecord;
use crate::scheduler::lifecycle::{
    WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentRegistryHandle,
    WorkflowSchedulerLifecycleComponentState,
};
use crate::workflow::WorkflowServiceError;

/// Workflow-service owner for retry-loop lifecycle state.
///
/// This helper owns only the coarse retry component state around an existing
/// retry sweep. It does not schedule retries, spawn work, decide retry policy,
/// or emit public diagnostics.
#[derive(Debug, Clone)]
pub(crate) struct WorkflowSchedulerRetryLifecycle {
    scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
}

impl WorkflowSchedulerRetryLifecycle {
    pub(crate) fn new(
        scheduler_lifecycle: WorkflowSchedulerLifecycleComponentRegistryHandle,
    ) -> Self {
        Self {
            scheduler_lifecycle,
        }
    }

    pub(crate) fn run_retry_loop<T>(
        &self,
        action: impl FnOnce() -> Result<T, WorkflowServiceError>,
    ) -> Result<T, WorkflowServiceError> {
        self.mark_retry_loop(WorkflowSchedulerLifecycleComponentState::Running)?;
        let result = action();
        let reset_result =
            self.mark_retry_loop(WorkflowSchedulerLifecycleComponentState::NotStarted);
        match (result, reset_result) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_value), Err(error)) => Err(error),
            (Err(error), Err(_reset_error)) => Err(error),
        }
    }

    #[cfg(test)]
    pub(crate) fn retry_loop_lifecycle_component(
        &self,
    ) -> Result<WorkflowSchedulerLifecycleComponentRecord, WorkflowServiceError> {
        self.scheduler_lifecycle
            .component(WorkflowSchedulerLifecycleComponentKind::RetryLoop)
    }

    fn mark_retry_loop(
        &self,
        state: WorkflowSchedulerLifecycleComponentState,
    ) -> Result<(), WorkflowServiceError> {
        self.scheduler_lifecycle
            .update_component_state(WorkflowSchedulerLifecycleComponentKind::RetryLoop, state)
            .map(|_record| ())
    }
}

#[cfg(test)]
#[path = "retry_lifecycle_tests.rs"]
mod tests;
