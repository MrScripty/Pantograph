use std::sync::Arc;

use super::task_execution_worker::WorkflowTaskExecutionWorker;
use super::{WorkflowService, WorkflowServiceError};

pub(super) struct WorkflowTaskExecutionRuntimeOwner {
    service: Arc<WorkflowService>,
    task_execution_worker: tokio::sync::Mutex<Option<WorkflowTaskExecutionWorker>>,
}

impl WorkflowTaskExecutionRuntimeOwner {
    pub(super) fn new(service: Arc<WorkflowService>) -> Self {
        Self {
            service,
            task_execution_worker: tokio::sync::Mutex::new(None),
        }
    }

    pub(super) fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }

    pub(super) async fn ensure_task_execution_worker_started(
        &self,
    ) -> Result<(), WorkflowServiceError> {
        let mut worker = self.task_execution_worker.lock().await;
        if worker.is_some() {
            return Ok(());
        }

        let scheduler_lifecycle = self
            .service
            .scheduler_task_orchestrator
            .scheduler_lifecycle_handle();
        *worker = Some(WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle)?);
        Ok(())
    }

    pub(super) async fn shutdown_task_execution_worker(&self) -> Result<(), WorkflowServiceError> {
        let worker = self.task_execution_worker.lock().await.take();
        if let Some(worker) = worker {
            worker.shutdown().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{
        WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentState,
    };

    #[tokio::test]
    async fn runtime_owner_holds_service_and_worker_without_service_self_reference() {
        let service = Arc::new(WorkflowService::new());
        let owner = WorkflowTaskExecutionRuntimeOwner::new(Arc::clone(&service));

        assert!(Arc::ptr_eq(&service, &owner.service()));

        owner
            .ensure_task_execution_worker_started()
            .await
            .expect("start task execution worker");
        owner
            .ensure_task_execution_worker_started()
            .await
            .expect("second start should reuse worker");

        assert_eq!(
            service
                .scheduler_task_orchestrator
                .scheduler_lifecycle_handle()
                .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)
                .expect("task execution worker component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Running
        );

        owner
            .shutdown_task_execution_worker()
            .await
            .expect("shutdown task execution worker");
        owner
            .shutdown_task_execution_worker()
            .await
            .expect("second shutdown should be idempotent");

        assert_eq!(
            service
                .scheduler_task_orchestrator
                .scheduler_lifecycle_handle()
                .component(WorkflowSchedulerLifecycleComponentKind::TaskExecutionWorker)
                .expect("task execution worker component")
                .state,
            WorkflowSchedulerLifecycleComponentState::Shutdown
        );
    }
}
