use std::sync::Arc;

use super::task_execution_runtime::WorkflowTaskExecutionRuntimeOwner;
use super::task_execution_worker::{
    WorkflowTaskExecutionWorkerOutcome, WorkflowTaskExecutionWorkerRuntimeBranchCommand,
};
use super::{
    WorkflowExecutionSessionBootstrapRecoveryResult, WorkflowExecutionSessionRunRequest,
    WorkflowHost, WorkflowRunResponse, WorkflowService, WorkflowServiceError,
};

/// Composition-root entrypoint for production workflow session execution.
#[must_use]
pub struct WorkflowSessionExecutionRuntime {
    service: Arc<WorkflowService>,
    host: Arc<dyn WorkflowHost>,
    task_execution_runtime_owner: WorkflowTaskExecutionRuntimeOwner,
}

impl WorkflowSessionExecutionRuntime {
    pub fn new<H>(service: WorkflowService, host: Arc<H>) -> Self
    where
        H: WorkflowHost + 'static,
    {
        Self::from_shared_service(Arc::new(service), host)
    }

    pub fn from_shared_service<H>(service: Arc<WorkflowService>, host: Arc<H>) -> Self
    where
        H: WorkflowHost + 'static,
    {
        let host: Arc<dyn WorkflowHost> = host;
        Self::from_shared_service_and_host(service, host)
    }

    pub fn from_shared_service_and_host(
        service: Arc<WorkflowService>,
        host: Arc<dyn WorkflowHost>,
    ) -> Self {
        let task_execution_runtime_owner =
            WorkflowTaskExecutionRuntimeOwner::new(Arc::clone(&service), Arc::clone(&host));
        Self {
            service,
            host,
            task_execution_runtime_owner,
        }
    }

    pub fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }

    pub fn host(&self) -> Arc<dyn WorkflowHost> {
        Arc::clone(&self.host)
    }

    pub async fn run_workflow_execution_session(
        &self,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.service
            .run_workflow_execution_session_with_runtime_owner(
                self.host.as_ref(),
                request,
                Some(self.task_execution_runtime_owner()),
            )
            .await
    }

    pub async fn recover_workflow_execution_session_bootstrap(
        &self,
    ) -> Result<WorkflowExecutionSessionBootstrapRecoveryResult, WorkflowServiceError> {
        self.service
            .recover_workflow_execution_session_bootstrap_with_runtime_owner(
                self.host.as_ref(),
                Some(self.task_execution_runtime_owner()),
            )
            .await
    }

    pub(super) fn task_execution_runtime_owner(&self) -> &WorkflowTaskExecutionRuntimeOwner {
        &self.task_execution_runtime_owner
    }

    pub(super) async fn enqueue_runtime_branch_and_wait(
        &self,
        command: WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    ) -> Result<WorkflowTaskExecutionWorkerOutcome, WorkflowServiceError> {
        self.task_execution_runtime_owner
            .enqueue_runtime_branch_and_wait(command)
            .await
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
        let host = Arc::new(DelegatingHost);

        let runtime =
            WorkflowSessionExecutionRuntime::from_shared_service(Arc::clone(&service), host);

        assert!(Arc::ptr_eq(&service, &runtime.service()));
        assert!(Arc::ptr_eq(
            &service,
            &runtime.task_execution_runtime_owner().service()
        ));
        assert!(Arc::ptr_eq(
            &runtime.host(),
            &runtime.task_execution_runtime_owner().host()
        ));
    }

    #[tokio::test]
    async fn session_execution_runtime_delegates_session_run_to_workflow_service() {
        let runtime =
            WorkflowSessionExecutionRuntime::new(WorkflowService::new(), Arc::new(DelegatingHost));

        let error = runtime
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: " ".to_string(),
                workflow_semantic_version: "1.0.0".to_string(),
                inputs: Vec::new(),
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            })
            .await
            .expect_err("empty session id should be rejected by WorkflowService");

        let WorkflowServiceError::InvalidRequest(message) = error else {
            panic!("expected invalid request error");
        };
        assert!(message.contains("session_id must be non-empty"));
    }

    #[tokio::test]
    async fn session_execution_runtime_enqueues_and_awaits_runtime_branch_completion() {
        let runtime =
            WorkflowSessionExecutionRuntime::new(WorkflowService::new(), Arc::new(DelegatingHost));

        let outcome = runtime
            .enqueue_runtime_branch_and_wait(WorkflowTaskExecutionWorkerRuntimeBranchCommand {
                session_id: "session-1".to_string(),
                workflow_run_id: "run-1".to_string(),
                workflow_id: "workflow-1".to_string(),
                output_targets: None,
                timeout_ms: Some(500),
                start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
            })
            .await
            .expect("enqueue runtime branch");

        let WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) = outcome else {
            panic!("expected fail-closed runtime branch outcome");
        };
        assert!(
            outcome
                .error_message
                .contains("not available for worker claim"),
            "unexpected error message: {}",
            outcome.error_message
        );

        runtime
            .task_execution_runtime_owner()
            .shutdown_task_execution_worker()
            .await
            .expect("shutdown task execution worker");
    }
}
