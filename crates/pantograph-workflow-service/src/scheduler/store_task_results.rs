use std::collections::BTreeMap;

use crate::workflow::{WorkflowSchedulerTaskResult, WorkflowServiceError};

use super::WorkflowExecutionSessionStore;

impl WorkflowExecutionSessionStore {
    /// Stage scheduler task results on the active run until durable ledger
    /// replay replaces this storage boundary.
    #[allow(dead_code)]
    pub(crate) fn set_active_run_scheduler_task_results(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        results: Vec<WorkflowSchedulerTaskResult>,
    ) -> Result<(), WorkflowServiceError> {
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let active_run = state.active_run.as_mut().ok_or_else(|| {
            WorkflowServiceError::QueueItemNotFound(format!(
                "session '{}' has no active workflow run",
                session_id
            ))
        })?;
        if active_run.workflow_run_id != workflow_run_id {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "workflow run '{}' is not active in session '{}'",
                workflow_run_id, session_id
            )));
        }

        let mut indexed = BTreeMap::new();
        for result in results {
            validate_task_result_for_active_run(&result, workflow_run_id)?;
            let task_id = result.task_id.clone();
            if indexed.insert(task_id.clone(), result).is_some() {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "duplicate scheduler task result '{}'",
                    task_id
                )));
            }
        }
        active_run.scheduler_task_results = indexed;
        Ok(())
    }

    /// Record one scheduler task result on the active run.
    #[allow(dead_code)]
    pub(crate) fn record_active_run_scheduler_task_result(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        result: WorkflowSchedulerTaskResult,
    ) -> Result<(), WorkflowServiceError> {
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let active_run = state.active_run.as_mut().ok_or_else(|| {
            WorkflowServiceError::QueueItemNotFound(format!(
                "session '{}' has no active workflow run",
                session_id
            ))
        })?;
        if active_run.workflow_run_id != workflow_run_id {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "workflow run '{}' is not active in session '{}'",
                workflow_run_id, session_id
            )));
        }
        validate_task_result_for_active_run(&result, workflow_run_id)?;
        active_run
            .scheduler_task_results
            .insert(result.task_id.clone(), result);
        Ok(())
    }

    /// Read staged scheduler task results for the active run.
    #[allow(dead_code)]
    pub(crate) fn active_run_scheduler_task_results(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<Vec<WorkflowSchedulerTaskResult>, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let active_run = state.active_run.as_ref().ok_or_else(|| {
            WorkflowServiceError::QueueItemNotFound(format!(
                "session '{}' has no active workflow run",
                session_id
            ))
        })?;
        if active_run.workflow_run_id != workflow_run_id {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "workflow run '{}' is not active in session '{}'",
                workflow_run_id, session_id
            )));
        }
        let results = active_run
            .scheduler_task_results
            .values()
            .cloned()
            .collect();
        Self::mark_session_access(state, tick);
        Ok(results)
    }
}

fn validate_task_result_for_active_run(
    result: &WorkflowSchedulerTaskResult,
    workflow_run_id: &str,
) -> Result<(), WorkflowServiceError> {
    result.validate().map_err(|error| {
        WorkflowServiceError::InvalidRequest(format!("invalid scheduler task result: {error}"))
    })?;
    if result.workflow_run_id != workflow_run_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task result '{}' belongs to workflow run '{}', expected '{}'",
            result.task_id, result.workflow_run_id, workflow_run_id
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::PumasModelRef;

    use crate::workflow::{
        WorkflowExecutionSessionRunRequest, WorkflowPortBinding, WorkflowSchedulerTaskResult,
        WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
        WorkflowSchedulerTaskResultValue,
    };

    use super::*;

    fn task_result(task_id: &str, workflow_run_id: &str) -> WorkflowSchedulerTaskResult {
        WorkflowSchedulerTaskResult {
            schema_version: crate::workflow::WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: "workflow-task-results".to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            node_id: task_id.to_string(),
            task_id: task_id.to_string(),
            status: WorkflowSchedulerTaskResultStatus::Completed,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: "pumas_model_ref".to_string(),
                value: WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
                    model_id: "image/example/tiny-diffusion".to_string(),
                    revision: Some("main".to_string()),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                }),
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        }
    }

    fn active_store() -> (WorkflowExecutionSessionStore, String, String) {
        let mut store = WorkflowExecutionSessionStore::new(4, 2);
        let session_id = store
            .create_session(
                "workflow-task-results".to_string(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                true,
            )
            .expect("create session");
        let request = WorkflowExecutionSessionRunRequest {
            session_id: session_id.clone(),
            workflow_semantic_version: "0.1.0".to_string(),
            inputs: Vec::<WorkflowPortBinding>::new(),
            output_targets: None,
            override_selection: None,
            timeout_ms: None,
            priority: None,
        };
        let workflow_run_id = store
            .enqueue_run(&session_id, &request)
            .expect("enqueue run");
        store
            .begin_queued_run(&session_id, &workflow_run_id)
            .expect("begin queued run")
            .expect("active run");
        (store, session_id, workflow_run_id)
    }

    #[test]
    fn active_run_scheduler_task_results_round_trip_validated_results() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store
            .record_active_run_scheduler_task_result(
                &session_id,
                &workflow_run_id,
                task_result("model-task", &workflow_run_id),
            )
            .expect("record task result");

        let results = store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read task results");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "model-task");
        assert_eq!(results[0].workflow_run_id, workflow_run_id);
    }

    #[test]
    fn active_run_scheduler_task_results_reject_wrong_run() {
        let (mut store, session_id, workflow_run_id) = active_store();
        let error = store
            .record_active_run_scheduler_task_result(
                &session_id,
                &workflow_run_id,
                task_result("model-task", "other-run"),
            )
            .expect_err("wrong run result should be rejected");

        assert!(error.message().contains("expected"));
    }
}
