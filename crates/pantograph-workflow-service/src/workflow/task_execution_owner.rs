use std::time::Duration;

use crate::scheduler::WorkflowExecutionSessionDequeuedRun;
use pantograph_runtime_attribution::WorkflowRunSnapshotRecord;

use super::session_scheduler_runner::WorkflowSchedulerSessionRunner;
use super::{
    WorkflowErrorDiagnosticsLink, WorkflowExecutionSessionSummary, WorkflowHost,
    WorkflowPortBinding, WorkflowRunResponse, WorkflowSchedulerTaskRunSummary, WorkflowService,
    WorkflowServiceError,
};

pub(super) struct WorkflowTaskExecutionOwner;

impl WorkflowTaskExecutionOwner {
    pub(super) async fn run_non_runtime_to_completion<H: WorkflowHost>(
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
                Err(_) => Err(WorkflowServiceError::RuntimeTimeout(format!(
                    "workflow run exceeded timeout_ms {}",
                    timeout_ms
                ))),
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
}
