use std::sync::Arc;

use super::task_execution_runtime::WorkflowTaskExecutionRuntimeOwner;
use super::{
    WorkflowExecutionSessionRunRequest, WorkflowHost, WorkflowRunResponse, WorkflowService,
    WorkflowServiceError,
};

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

    pub async fn run_workflow_execution_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.service
            .run_workflow_execution_session_with_runtime_owner(
                host,
                request,
                Some(self.task_execution_runtime_owner()),
            )
            .await
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
    use crate::workflow::{
        WorkflowOutputTarget, WorkflowPortBinding, WorkflowRunHandle, WorkflowRunOptions,
    };
    use async_trait::async_trait;

    struct DelegatingHost;

    #[async_trait]
    impl WorkflowHost for DelegatingHost {
        async fn run_workflow(
            &self,
            _workflow_id: &str,
            _inputs: &[WorkflowPortBinding],
            _output_targets: Option<&[WorkflowOutputTarget]>,
            _run_options: WorkflowRunOptions,
            _run_handle: WorkflowRunHandle,
        ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
            Err(WorkflowServiceError::Internal(
                "delegation test host should not execute workflow".to_string(),
            ))
        }
    }

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

    #[tokio::test]
    async fn session_execution_runtime_delegates_session_run_to_workflow_service() {
        let runtime = WorkflowSessionExecutionRuntime::new(WorkflowService::new());

        let error = runtime
            .run_workflow_execution_session(
                &DelegatingHost,
                WorkflowExecutionSessionRunRequest {
                    session_id: " ".to_string(),
                    workflow_semantic_version: "1.0.0".to_string(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: None,
                },
            )
            .await
            .expect_err("empty session id should be rejected by WorkflowService");

        let WorkflowServiceError::InvalidRequest(message) = error else {
            panic!("expected invalid request error");
        };
        assert!(message.contains("session_id must be non-empty"));
    }
}
