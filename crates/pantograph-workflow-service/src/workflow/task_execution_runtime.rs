use std::sync::Arc;

use super::task_execution_worker::{
    WorkflowTaskExecutionWorker, WorkflowTaskExecutionWorkerCommand,
    WorkflowTaskExecutionWorkerDiagnostic, WorkflowTaskExecutionWorkerDiagnosticCode,
    WorkflowTaskExecutionWorkerOutcome,
};
use super::{WorkflowService, WorkflowServiceError};

pub(super) struct WorkflowTaskExecutionRuntimeOwner {
    service: Arc<WorkflowService>,
    task_execution_worker: tokio::sync::Mutex<WorkflowTaskExecutionRuntimeWorkerState>,
}

enum WorkflowTaskExecutionRuntimeWorkerState {
    NotStarted,
    Running(WorkflowTaskExecutionWorker),
    Shutdown,
}

impl WorkflowTaskExecutionRuntimeOwner {
    pub(super) fn new(service: Arc<WorkflowService>) -> Self {
        Self {
            service,
            task_execution_worker: tokio::sync::Mutex::new(
                WorkflowTaskExecutionRuntimeWorkerState::NotStarted,
            ),
        }
    }

    pub(super) fn service(&self) -> Arc<WorkflowService> {
        Arc::clone(&self.service)
    }

    pub(super) async fn ensure_task_execution_worker_started(
        &self,
    ) -> Result<(), WorkflowServiceError> {
        let mut worker = self.task_execution_worker.lock().await;
        match &*worker {
            WorkflowTaskExecutionRuntimeWorkerState::Running(_) => return Ok(()),
            WorkflowTaskExecutionRuntimeWorkerState::Shutdown => {
                return Err(WorkflowServiceError::Internal(
                    "task execution worker cannot be restarted after shutdown".to_string(),
                ));
            }
            WorkflowTaskExecutionRuntimeWorkerState::NotStarted => {}
        }

        let scheduler_lifecycle = self
            .service
            .scheduler_task_orchestrator
            .scheduler_lifecycle_handle();
        *worker = WorkflowTaskExecutionRuntimeWorkerState::Running(
            WorkflowTaskExecutionWorker::spawn(scheduler_lifecycle)?,
        );
        Ok(())
    }

    pub(super) async fn try_enqueue_task_execution_command(
        &self,
        command: WorkflowTaskExecutionWorkerCommand,
    ) -> Result<(), WorkflowTaskExecutionWorkerOutcome> {
        let worker = self.task_execution_worker.lock().await;
        match &*worker {
            WorkflowTaskExecutionRuntimeWorkerState::Running(worker) => worker.try_enqueue(command),
            WorkflowTaskExecutionRuntimeWorkerState::NotStarted => Err(worker_unavailable_outcome(
                WorkflowTaskExecutionWorkerDiagnosticCode::WorkerUnavailable,
                "task execution worker has not started",
            )),
            WorkflowTaskExecutionRuntimeWorkerState::Shutdown => Err(worker_unavailable_outcome(
                WorkflowTaskExecutionWorkerDiagnosticCode::ShutdownRequested,
                "task execution worker is shut down",
            )),
        }
    }

    pub(super) async fn shutdown_task_execution_worker(&self) -> Result<(), WorkflowServiceError> {
        let worker = {
            let mut state = self.task_execution_worker.lock().await;
            match std::mem::replace(
                &mut *state,
                WorkflowTaskExecutionRuntimeWorkerState::Shutdown,
            ) {
                WorkflowTaskExecutionRuntimeWorkerState::Running(worker) => Some(worker),
                WorkflowTaskExecutionRuntimeWorkerState::NotStarted
                | WorkflowTaskExecutionRuntimeWorkerState::Shutdown => None,
            }
        };
        if let Some(worker) = worker {
            worker.shutdown().await?;
        }
        Ok(())
    }
}

fn worker_unavailable_outcome(
    code: WorkflowTaskExecutionWorkerDiagnosticCode,
    message: impl Into<String>,
) -> WorkflowTaskExecutionWorkerOutcome {
    WorkflowTaskExecutionWorkerOutcome::WorkerUnavailable(WorkflowTaskExecutionWorkerDiagnostic {
        code,
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::{
        WorkflowSchedulerLifecycleComponentKind, WorkflowSchedulerLifecycleComponentState,
    };
    use crate::workflow::WorkflowSchedulerTaskExecutionClass;

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

    #[tokio::test]
    async fn runtime_owner_returns_typed_diagnostics_when_worker_is_unavailable() {
        let service = Arc::new(WorkflowService::new());
        let owner = WorkflowTaskExecutionRuntimeOwner::new(service);

        let error = owner
            .try_enqueue_task_execution_command(task_attempt_command())
            .await
            .expect_err("worker should be unavailable before startup");

        let WorkflowTaskExecutionWorkerOutcome::WorkerUnavailable(diagnostic) = error else {
            panic!("expected worker-unavailable outcome");
        };

        assert_eq!(
            diagnostic.code,
            WorkflowTaskExecutionWorkerDiagnosticCode::WorkerUnavailable
        );
        assert!(diagnostic.message.contains("has not started"));
    }

    #[tokio::test]
    async fn runtime_owner_returns_typed_diagnostics_after_shutdown() {
        let service = Arc::new(WorkflowService::new());
        let owner = WorkflowTaskExecutionRuntimeOwner::new(service);

        owner
            .ensure_task_execution_worker_started()
            .await
            .expect("start task execution worker");
        owner
            .shutdown_task_execution_worker()
            .await
            .expect("shutdown task execution worker");

        let error = owner
            .try_enqueue_task_execution_command(task_attempt_command())
            .await
            .expect_err("worker should reject commands after shutdown");

        let WorkflowTaskExecutionWorkerOutcome::WorkerUnavailable(diagnostic) = error else {
            panic!("expected worker-unavailable outcome");
        };

        assert_eq!(
            diagnostic.code,
            WorkflowTaskExecutionWorkerDiagnosticCode::ShutdownRequested
        );
        assert!(diagnostic.message.contains("shut down"));
    }

    fn task_attempt_command() -> WorkflowTaskExecutionWorkerCommand {
        WorkflowTaskExecutionWorkerCommand::execute_task_attempt(
            "session-1",
            "run-1",
            "task-1",
            WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            Some(500),
        )
    }
}
