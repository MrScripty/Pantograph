use std::time::Duration;

use crate::scheduler::WorkflowExecutionSessionDequeuedRun;
use pantograph_runtime_attribution::WorkflowRunSnapshotRecord;

use super::session_scheduler_runner::WorkflowSchedulerSessionRunner;
use super::task_execution_runtime::WorkflowTaskExecutionRuntimeBranchContext;
use super::{
    WorkflowErrorDiagnosticsLink, WorkflowExecutionSessionSummary, WorkflowHost,
    WorkflowPortBinding, WorkflowRunResponse, WorkflowSchedulerTaskRunSummary, WorkflowService,
    WorkflowServiceError,
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
        let finish_state = {
            let mut store = service.session_store_guard()?;
            store.finish_run(session_id, workflow_run_id)?
        };
        if let Err(record_error) = service.record_run_terminal_event_if_configured(
            session,
            run_snapshot,
            workflow_run_id,
            Some(&queued_workflow_semantic_version),
            &run_result,
        ) {
            if let Err(error) = run_result {
                return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                    workflow_run_id: Some(workflow_run_id.to_string()),
                    diagnostic_event_id: None,
                    diagnostics_unavailable: Some(record_error.message().to_string()),
                }));
            }
            return Err(record_error);
        }
        if let Ok(response) = run_result.as_ref() {
            service.record_workflow_io_artifact_events_if_configured(
                session,
                run_snapshot,
                workflow_run_id,
                &queued_workflow_semantic_version,
                &queued_workflow_inputs,
                &response.outputs,
            )?;
        }
        debug_assert!(!finish_state.unload_runtime);
        run_result
    }

    pub(super) async fn run_runtime_branch_until_dispatch_boundary<H: WorkflowHost + ?Sized>(
        context: &WorkflowTaskExecutionRuntimeBranchContext,
        host: &H,
        session: &WorkflowExecutionSessionSummary,
        run_snapshot: Option<&WorkflowRunSnapshotRecord>,
        queued_run: &WorkflowExecutionSessionDequeuedRun,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let service = context.service();
        let command = context.command();
        service.record_run_started_event_if_configured(session, run_snapshot, queued_run)?;
        let run_started_at = std::time::Instant::now();
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        let runner = WorkflowSchedulerSessionRunner::new(service.as_ref());
        let run_future = runner.run_until_runtime_dispatch_boundary(
            host,
            &command.session_id,
            &command.workflow_run_id,
            &command.workflow_id,
            &queued_run.queued.inputs,
            command.output_targets.as_deref(),
            summary,
            run_started_at,
        );
        let run_result = if let Some(timeout_ms) = command.timeout_ms {
            match tokio::time::timeout(Duration::from_millis(timeout_ms), run_future).await {
                Ok(result) => result,
                Err(_) => Err(WorkflowServiceError::RuntimeTimeout(format!(
                    "workflow run exceeded timeout_ms {}",
                    timeout_ms
                ))),
            }
        } else {
            run_future.await
        };
        if run_result
            .as_ref()
            .is_err_and(WorkflowServiceError::is_runtime_dependency_readiness_pending)
        {
            return run_result;
        }
        service.finish_failed_workflow_run_after_admission(
            &command.session_id,
            &command.workflow_run_id,
        )?;
        if let Err(record_error) = service.record_run_terminal_event_if_configured(
            session,
            run_snapshot,
            &command.workflow_run_id,
            Some(&queued_workflow_semantic_version),
            &run_result,
        ) {
            if let Err(error) = run_result {
                return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                    workflow_run_id: Some(command.workflow_run_id.clone()),
                    diagnostic_event_id: None,
                    diagnostics_unavailable: Some(record_error.message().to_string()),
                }));
            }
            return Err(record_error);
        }
        run_result
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
        let run_result =
            service.fail_unhandled_scheduler_session_classes(session_id, workflow_run_id, summary);
        service.finish_failed_workflow_run_after_admission(session_id, workflow_run_id)?;
        if let Err(record_error) = service.record_run_terminal_event_if_configured(
            session,
            run_snapshot,
            workflow_run_id,
            Some(&queued_workflow_semantic_version),
            &run_result,
        ) {
            if let Err(error) = run_result {
                return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                    workflow_run_id: Some(workflow_run_id.to_string()),
                    diagnostic_event_id: None,
                    diagnostics_unavailable: Some(record_error.message().to_string()),
                }));
            }
            return Err(record_error);
        }
        run_result
    }
}
