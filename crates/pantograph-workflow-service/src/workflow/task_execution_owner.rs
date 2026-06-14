use std::time::Duration;

use crate::scheduler::WorkflowExecutionSessionDequeuedRun;
use pantograph_runtime_attribution::WorkflowRunSnapshotRecord;

use super::session_scheduler_runner::WorkflowSchedulerSessionRunner;
use super::workflow_run_finalization::{
    finalize_admitted_workflow_run, WorkflowRunFinalizationRequest,
};
use super::{
    WorkflowExecutionSessionSummary, WorkflowHost, WorkflowPortBinding, WorkflowRunResponse,
    WorkflowSchedulerTaskRunSummary, WorkflowService, WorkflowServiceError,
};

pub(super) struct WorkflowTaskExecutionOwner;

impl WorkflowTaskExecutionOwner {
    pub(super) fn ensure_task_execution_available(
        service: &WorkflowService,
    ) -> Result<(), WorkflowServiceError> {
        service
            .scheduler_task_orchestrator
            .ensure_task_execution_available()
    }

    pub(super) async fn run_non_runtime_to_completion<H: WorkflowHost + ?Sized>(
        service: &WorkflowService,
        host: &H,
        session: &WorkflowExecutionSessionSummary,
        run_snapshot: Option<&WorkflowRunSnapshotRecord>,
        session_id: &str,
        workflow_run_id: &str,
        queued_run: &WorkflowExecutionSessionDequeuedRun,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        service.record_run_started_event_if_configured(session, run_snapshot, queued_run)?;
        let run_started_at = std::time::Instant::now();
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        let queued_workflow_inputs: Vec<WorkflowPortBinding> = queued_run.queued.inputs.clone();
        let runner = WorkflowSchedulerSessionRunner::new(service);
        let run_future = runner.run_non_runtime_only(
            host,
            session_id,
            workflow_run_id,
            &queued_run.workflow_id,
            &queued_run.queued.inputs,
            queued_run.queued.output_targets.as_deref(),
            summary,
            run_started_at,
        );
        let run_result = if let Some(timeout_ms) = queued_run.queued.timeout_ms {
            match tokio::time::timeout(Duration::from_millis(timeout_ms), run_future).await {
                Ok(result) => result,
                Err(_) => {
                    let message = format!("workflow run exceeded timeout_ms {}", timeout_ms);
                    {
                        let mut store = service.session_store_guard()?;
                        service
                            .scheduler_task_orchestrator
                            .cancel_running_tasks_for_workflow_timeout(
                                &mut store,
                                session_id,
                                workflow_run_id,
                                &message,
                            )
                            .map_err(|error| {
                                WorkflowServiceError::RuntimeTimeout(format!(
                                    "{message}; scheduler task timeout cleanup failed: {error}"
                                ))
                            })?;
                    }
                    Err(WorkflowServiceError::RuntimeTimeout(message))
                }
            }
        } else {
            run_future.await
        };
        let finalization = finalize_admitted_workflow_run(
            service,
            WorkflowRunFinalizationRequest {
                session,
                run_snapshot,
                session_id,
                workflow_run_id,
                workflow_semantic_version: &queued_workflow_semantic_version,
                io_artifact_inputs: Some(&queued_workflow_inputs),
                run_result,
            },
        )?;
        debug_assert!(!finalization.unload_runtime);
        finalization.run_result
    }

    pub(super) fn fail_unhandled_scheduler_classes_to_completion(
        service: &WorkflowService,
        session: &WorkflowExecutionSessionSummary,
        run_snapshot: Option<&WorkflowRunSnapshotRecord>,
        session_id: &str,
        workflow_run_id: &str,
        queued_run: &WorkflowExecutionSessionDequeuedRun,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        service.record_run_started_event_if_configured(session, run_snapshot, queued_run)?;
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        let queued_workflow_inputs = queued_run.queued.inputs.clone();
        let run_result =
            service.fail_unhandled_scheduler_session_classes(session_id, workflow_run_id, summary);
        let finalization = finalize_admitted_workflow_run(
            service,
            WorkflowRunFinalizationRequest {
                session,
                run_snapshot,
                session_id,
                workflow_run_id,
                workflow_semantic_version: &queued_workflow_semantic_version,
                io_artifact_inputs: Some(&queued_workflow_inputs),
                run_result,
            },
        )?;
        finalization.run_result
    }
}
