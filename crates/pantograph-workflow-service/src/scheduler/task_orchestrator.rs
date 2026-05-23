use pantograph_runtime_host_contracts::{
    RuntimeHostDispatchError, SchedulerRuntimeHostDispatcher, ValidatedRuntimeHostExecutionResponse,
};
use pantograph_scheduler::SchedulerRuntimeHandoff;
use thiserror::Error;

/// Workflow-service async shell for scheduler task orchestration.
///
/// This type owns application-layer calls into lower-level scheduler and
/// runtime-host contracts. Scheduler policy remains in `pantograph-scheduler`;
/// runtime execution remains behind the shared runtime-host port.
#[derive(Clone)]
#[must_use]
#[allow(dead_code)]
pub(crate) struct WorkflowSchedulerTaskOrchestrator {
    runtime_host_dispatcher: SchedulerRuntimeHostDispatcher,
}

#[allow(dead_code)]
impl WorkflowSchedulerTaskOrchestrator {
    pub(crate) fn new(runtime_host_dispatcher: SchedulerRuntimeHostDispatcher) -> Self {
        Self {
            runtime_host_dispatcher,
        }
    }

    pub(crate) async fn dispatch_runtime_handoff(
        &self,
        execution_request_id: impl Into<String>,
        handoff: SchedulerRuntimeHandoff,
    ) -> Result<ValidatedRuntimeHostExecutionResponse, WorkflowSchedulerTaskOrchestratorError> {
        self.runtime_host_dispatcher
            .dispatch(execution_request_id, handoff)
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::RuntimeHostDispatch)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
#[allow(dead_code)]
pub(crate) enum WorkflowSchedulerTaskOrchestratorError {
    #[error("runtime-host dispatch failed")]
    RuntimeHostDispatch(RuntimeHostDispatchError),
}

#[cfg(test)]
#[path = "task_orchestrator_tests.rs"]
mod tests;
