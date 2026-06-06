use std::sync::Arc;

use super::task_execution_runtime::WorkflowTaskExecutionRuntimeOwner;
use super::WorkflowService;

/// Composition-root entrypoint for production workflow session execution.
#[must_use]
pub struct WorkflowSessionExecutionRuntime {
    service: Arc<WorkflowService>,
    task_execution_runtime_owner: WorkflowTaskExecutionRuntimeOwner,
}

impl WorkflowSessionExecutionRuntime {
    pub fn new(service: WorkflowService) -> Self {
        Self::from_shared_service(Arc::new(service))
    }

    pub fn from_shared_service(service: Arc<WorkflowService>) -> Self {
        let task_execution_runtime_owner =
            WorkflowTaskExecutionRuntimeOwner::new(Arc::clone(&service));
        Self {
            service,
            task_execution_runtime_owner,
        }
    }

    pub fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }

    pub(super) fn task_execution_runtime_owner(&self) -> &WorkflowTaskExecutionRuntimeOwner {
        &self.task_execution_runtime_owner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::task_execution_worker::{
        WorkflowTaskExecutionWorkerRuntimeBranchCommand,
        WorkflowTaskExecutionWorkerRuntimeBranchStartReason,
    };
    use crate::workflow::WorkflowOutputTarget;

    #[test]
    fn session_execution_runtime_owns_shared_service_and_runtime_owner() {
        let service = Arc::new(WorkflowService::new());

        let runtime = WorkflowSessionExecutionRuntime::from_shared_service(Arc::clone(&service));

        assert!(Arc::ptr_eq(&service, &runtime.service()));
        assert!(Arc::ptr_eq(
            &service,
            &runtime.task_execution_runtime_owner().service()
        ));
    }

    #[test]
    fn session_execution_runtime_builds_runtime_branch_context_from_owned_runtime_owner() {
        let runtime = WorkflowSessionExecutionRuntime::new(WorkflowService::new());
        let command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
            session_id: "session-1".to_string(),
            workflow_run_id: "run-1".to_string(),
            workflow_id: "workflow-1".to_string(),
            output_targets: Some(vec![WorkflowOutputTarget {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
            }]),
            timeout_ms: Some(500),
            start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
        };

        let context = runtime
            .task_execution_runtime_owner()
            .runtime_branch_context(command.clone());

        assert!(Arc::ptr_eq(&runtime.service(), &context.service()));
        assert_eq!(context.command(), &command);
    }
}
