use std::collections::BTreeMap;

use pantograph_scheduler::{
    apply_scheduler_task_state_transition, SchedulerTaskExecutionIntent, SchedulerTaskStateKind,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionApplyResult,
};

use crate::workflow::{
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskResult, WorkflowServiceError,
};

use super::{
    unix_timestamp_ms, WorkflowExecutionSessionStore, WorkflowExecutionSessionTaskAttempt,
    WorkflowSchedulerTaskAttemptId,
};

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

    /// Atomically record a successful task result and complete the matching
    /// scheduler task on the active run.
    #[allow(dead_code)]
    pub(crate) fn complete_active_run_scheduler_task(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
        transition: SchedulerTaskStateTransition,
        result: WorkflowSchedulerTaskResult,
    ) -> Result<SchedulerTaskStateTransitionApplyResult, WorkflowServiceError> {
        let tick = self.next_tick();
        validate_task_result_for_active_run(&result, workflow_run_id)?;
        validate_completion_transition_for_result(&transition, &result)?;

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

        let task_id = result.task_id.clone();
        validate_matching_task_attempt(
            &active_run.scheduler_task_attempts,
            &task_id,
            attempt_id,
            "completion",
        )?;
        if active_run.scheduler_task_results.contains_key(&task_id) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task result '{}' is already recorded",
                task_id
            )));
        }

        let current = active_run
            .scheduler_task_records
            .get(&task_id)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no active task-state record",
                    task_id
                ))
            })?;
        if current.state.kind() != SchedulerTaskStateKind::Running {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be running before completion, found {:?}",
                task_id,
                current.state.kind()
            )));
        }

        let apply_result = apply_scheduler_task_state_transition(Some(current), transition)
            .map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "invalid scheduler task completion transition: {error}"
                ))
            })?;
        let SchedulerTaskStateTransitionApplyResult::Applied(record) = &apply_result else {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' completion transition was already applied",
                task_id
            )));
        };

        active_run
            .scheduler_task_records
            .insert(task_id.clone(), record.clone());
        active_run.scheduler_task_attempts.remove(&task_id);
        active_run.scheduler_task_results.insert(task_id, result);
        Self::mark_session_access(state, tick);
        Ok(apply_result)
    }

    /// Atomically start a scheduler task attempt on the active run.
    pub(crate) fn start_active_run_scheduler_task_attempt(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        transition: SchedulerTaskStateTransition,
    ) -> Result<
        (
            SchedulerTaskStateTransitionApplyResult,
            WorkflowSchedulerTaskAttemptId,
        ),
        WorkflowServiceError,
    > {
        let tick = self.next_tick();
        validate_start_attempt_transition(&transition)?;

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

        let task_id = transition.task_id.as_str().to_string();
        if active_run.scheduler_task_attempts.contains_key(&task_id) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' already has an active attempt",
                task_id
            )));
        }
        let current = active_run
            .scheduler_task_records
            .get(&task_id)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no active task-state record",
                    task_id
                ))
            })?;
        if current.state.kind() != SchedulerTaskStateKind::Ready {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be ready before starting an attempt, found {:?}",
                task_id,
                current.state.kind()
            )));
        }

        let apply_result = apply_scheduler_task_state_transition(Some(current), transition)
            .map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "invalid scheduler task attempt start transition: {error}"
                ))
            })?;
        let SchedulerTaskStateTransitionApplyResult::Applied(record) = &apply_result else {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' attempt start transition was already applied",
                task_id
            )));
        };

        let attempt_id = WorkflowSchedulerTaskAttemptId::new();
        active_run
            .scheduler_task_records
            .insert(task_id.clone(), record.clone());
        active_run.scheduler_task_attempts.insert(
            task_id.clone(),
            WorkflowExecutionSessionTaskAttempt {
                attempt_id: attempt_id.clone(),
                started_at_ms: unix_timestamp_ms(),
            },
        );
        Self::mark_session_access(state, tick);
        Ok((apply_result, attempt_id))
    }

    /// Atomically fail the matching active scheduler task attempt.
    pub(crate) fn fail_active_run_scheduler_task_attempt(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        attempt_id: &WorkflowSchedulerTaskAttemptId,
        transition: SchedulerTaskStateTransition,
    ) -> Result<SchedulerTaskStateTransitionApplyResult, WorkflowServiceError> {
        let tick = self.next_tick();
        validate_terminal_attempt_transition(&transition, "failure")?;

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

        let task_id = transition.task_id.as_str().to_string();
        validate_matching_task_attempt(
            &active_run.scheduler_task_attempts,
            &task_id,
            attempt_id,
            "failure",
        )?;
        let current = active_run
            .scheduler_task_records
            .get(&task_id)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no active task-state record",
                    task_id
                ))
            })?;
        if current.state.kind() != SchedulerTaskStateKind::Running {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be running before failure, found {:?}",
                task_id,
                current.state.kind()
            )));
        }

        let apply_result = apply_scheduler_task_state_transition(Some(current), transition)
            .map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "invalid scheduler task attempt failure transition: {error}"
                ))
            })?;
        let SchedulerTaskStateTransitionApplyResult::Applied(record) = &apply_result else {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' failure transition was already applied",
                task_id
            )));
        };

        active_run
            .scheduler_task_records
            .insert(task_id.clone(), record.clone());
        active_run.scheduler_task_attempts.remove(&task_id);
        Self::mark_session_access(state, tick);
        Ok(apply_result)
    }

    /// Atomically materialize a request-provided source input as a completed
    /// scheduler task result and completed source-input task state.
    pub(crate) fn materialize_active_run_source_input_task(
        &mut self,
        session_id: &str,
        workflow_run_id: &str,
        transition: SchedulerTaskStateTransition,
        result: WorkflowSchedulerTaskResult,
    ) -> Result<SchedulerTaskStateTransitionApplyResult, WorkflowServiceError> {
        let tick = self.next_tick();
        validate_task_result_for_active_run(&result, workflow_run_id)?;
        validate_source_input_materialization_transition_for_result(&transition, &result)?;

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

        let task_id = result.task_id.clone();
        if active_run.scheduler_task_results.contains_key(&task_id) {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task result '{}' is already recorded",
                task_id
            )));
        }

        let task_graph = active_run.scheduler_task_graph.as_ref().ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "workflow run '{}' has no scheduler task graph",
                workflow_run_id
            ))
        })?;
        let task = task_graph
            .tasks
            .iter()
            .find(|task| task.task_id.as_str() == task_id)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler source-input task '{}' is not present in the task graph",
                    task_id
                ))
            })?;
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::SourceInput {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be a source-input task before source materialization",
                task_id
            )));
        }
        if task.source_input_task_template.is_none() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler source-input task '{}' has no typed source-input template",
                task_id
            )));
        }

        let current = active_run
            .scheduler_task_records
            .get(&task_id)
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no active task-state record",
                    task_id
                ))
            })?;
        if current.state.kind() != SchedulerTaskStateKind::AwaitingInputs {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler source-input task '{}' must be awaiting inputs before materialization, found {:?}",
                task_id,
                current.state.kind()
            )));
        }

        let apply_result = apply_scheduler_task_state_transition(Some(current), transition)
            .map_err(|error| {
                WorkflowServiceError::Internal(format!(
                    "invalid scheduler source-input materialization transition: {error}"
                ))
            })?;
        let SchedulerTaskStateTransitionApplyResult::Applied(record) = &apply_result else {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "scheduler source-input task '{}' materialization transition was already applied",
                task_id
            )));
        };

        active_run
            .scheduler_task_records
            .insert(task_id.clone(), record.clone());
        active_run.scheduler_task_results.insert(task_id, result);
        Self::mark_session_access(state, tick);
        Ok(apply_result)
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

fn validate_start_attempt_transition(
    transition: &SchedulerTaskStateTransition,
) -> Result<(), WorkflowServiceError> {
    if transition.expected_previous_state != Some(SchedulerTaskStateKind::Ready) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' attempt start must expect ready state",
            transition.task_id.as_str()
        )));
    }
    if transition.next_state.kind() != SchedulerTaskStateKind::Running {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' attempt start must enter running state",
            transition.task_id.as_str()
        )));
    }
    Ok(())
}

fn validate_terminal_attempt_transition(
    transition: &SchedulerTaskStateTransition,
    operation: &str,
) -> Result<(), WorkflowServiceError> {
    if transition.expected_previous_state != Some(SchedulerTaskStateKind::Running) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' {operation} must expect running state",
            transition.task_id.as_str()
        )));
    }
    if transition.next_state.kind() != SchedulerTaskStateKind::TerminalFailed {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' {operation} must enter terminal-failed state",
            transition.task_id.as_str()
        )));
    }
    Ok(())
}

fn validate_matching_task_attempt(
    attempts: &BTreeMap<String, WorkflowExecutionSessionTaskAttempt>,
    task_id: &str,
    attempt_id: &WorkflowSchedulerTaskAttemptId,
    operation: &str,
) -> Result<(), WorkflowServiceError> {
    let attempt = attempts.get(task_id).ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{task_id}' has no active attempt for {operation}"
        ))
    })?;
    if attempt.attempt_id != *attempt_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{task_id}' {operation} attempt '{}' does not match active attempt '{}'",
            attempt_id.as_str(),
            attempt.attempt_id.as_str()
        )));
    }
    Ok(())
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

fn validate_completion_transition_for_result(
    transition: &SchedulerTaskStateTransition,
    result: &WorkflowSchedulerTaskResult,
) -> Result<(), WorkflowServiceError> {
    if transition.expected_previous_state != Some(SchedulerTaskStateKind::Running) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' completion must expect running state",
            result.task_id
        )));
    }
    match result.status {
        crate::workflow::WorkflowSchedulerTaskResultStatus::Completed => {
            if transition.next_state.kind() != SchedulerTaskStateKind::Completed {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' completed result transition must end completed",
                    result.task_id
                )));
            }
        }
        crate::workflow::WorkflowSchedulerTaskResultStatus::Failed
        | crate::workflow::WorkflowSchedulerTaskResultStatus::Unavailable
        | crate::workflow::WorkflowSchedulerTaskResultStatus::Invalid => {
            if transition.next_state.kind() != SchedulerTaskStateKind::TerminalFailed {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' failed result transition must end terminal-failed",
                    result.task_id
                )));
            }
        }
    }
    validate_transition_result_correlation(transition, result, "completion")
}

fn validate_source_input_materialization_transition_for_result(
    transition: &SchedulerTaskStateTransition,
    result: &WorkflowSchedulerTaskResult,
) -> Result<(), WorkflowServiceError> {
    if result.status != crate::workflow::WorkflowSchedulerTaskResultStatus::Completed {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler source-input task result '{}' must be completed for source materialization",
            result.task_id
        )));
    }
    if transition.expected_previous_state != Some(SchedulerTaskStateKind::AwaitingInputs) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler source-input task '{}' materialization must expect awaiting inputs state",
            result.task_id
        )));
    }
    if transition.next_state.kind() != SchedulerTaskStateKind::Completed {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler source-input task '{}' materialization transition must end completed",
            result.task_id
        )));
    }
    let Some(SchedulerTaskExecutionIntent::SourceInput { .. }) =
        transition.next_state.execution_intent()
    else {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler source-input task '{}' materialization must carry source-input intent",
            result.task_id
        )));
    };
    validate_transition_result_correlation(transition, result, "materialization")
}

fn validate_transition_result_correlation(
    transition: &SchedulerTaskStateTransition,
    result: &WorkflowSchedulerTaskResult,
    operation: &str,
) -> Result<(), WorkflowServiceError> {
    if transition.workflow_id.as_str() != result.workflow_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task result '{}' workflow id '{}' does not match {operation} transition '{}'",
            result.task_id,
            result.workflow_id,
            transition.workflow_id.as_str()
        )));
    }
    if transition.workflow_run_id.as_str() != result.workflow_run_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task result '{}' workflow run id '{}' does not match {operation} transition '{}'",
            result.task_id,
            result.workflow_run_id,
            transition.workflow_run_id.as_str()
        )));
    }
    if transition.node_id.as_str() != result.node_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task result '{}' node id '{}' does not match {operation} transition '{}'",
            result.task_id,
            result.node_id,
            transition.node_id.as_str()
        )));
    }
    if transition.task_id.as_str() != result.task_id {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task result '{}' does not match {operation} transition task '{}'",
            result.task_id,
            transition.task_id.as_str()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
    use pantograph_scheduler::{
        SchedulableTaskIntent, SchedulerNodeId, SchedulerRuntimeDeviceConstraints,
        SchedulerSourceInputTaskIntent, SchedulerSourceInputTaskKind, SchedulerTaskExecutionIntent,
        SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateKind, SchedulerTaskStateRecord,
        SchedulerTaskStateTransition, SchedulerTaskStateTransitionApplyResult,
        SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use crate::workflow::{
        WorkflowExecutionSessionRunRequest, WorkflowPortBinding,
        WorkflowSchedulerSourceInputTemplate, WorkflowSchedulerTask,
        WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
        WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultOutput,
        WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
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

    fn source_input_task_result(
        task_id: &str,
        workflow_run_id: &str,
    ) -> WorkflowSchedulerTaskResult {
        WorkflowSchedulerTaskResult {
            schema_version: crate::workflow::WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: "workflow-task-results".to_string(),
            workflow_run_id: workflow_run_id.to_string(),
            node_id: task_id.to_string(),
            task_id: task_id.to_string(),
            status: WorkflowSchedulerTaskResultStatus::Completed,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: "text".to_string(),
                value: WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string()),
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

    fn task_graph(workflow_run_id: &str, task_id: &str) -> WorkflowSchedulerTaskGraph {
        let workflow_id = SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id");
        let workflow_run_id = SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id,
                workflow_run_id,
                node_id: SchedulerNodeId::parse(task_id).expect("node id"),
                task_id: SchedulerTaskId::parse(task_id).expect("task id"),
                node_type: "llm-inference".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                diagnostics: Vec::new(),
            }],
        }
    }

    fn source_input_task_graph(workflow_run_id: &str, task_id: &str) -> WorkflowSchedulerTaskGraph {
        let workflow_id = SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id");
        let workflow_run_id = SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id,
                workflow_run_id,
                node_id: SchedulerNodeId::parse(task_id).expect("node id"),
                task_id: SchedulerTaskId::parse(task_id).expect("task id"),
                node_type: "text-input".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: Some(WorkflowSchedulerSourceInputTemplate::Text {
                    port_id: "text".to_string(),
                }),
                inference_descriptor_fingerprint: None,
                diagnostics: Vec::new(),
            }],
        }
    }

    fn task_intent(workflow_run_id: &str, task_id: &str) -> SchedulableTaskIntent {
        SchedulableTaskIntent {
            contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            fairness_key: None,
            task_type: DependencyTaskId::parse("image_generation").expect("task type"),
            model_ref: PumasModelRef {
                model_id: "image/example/tiny-diffusion".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers-bundle".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            constraints: SchedulerRuntimeDeviceConstraints::default(),
            trait_settings: Vec::new(),
            dependency_override_patches: Vec::new(),
            estimate_hints: Vec::new(),
        }
    }

    fn task_state(
        state: SchedulerTaskStateKind,
        workflow_run_id: &str,
        task_id: &str,
    ) -> SchedulerTaskState {
        let execution_intent = SchedulerTaskExecutionIntent::Runtime {
            task_intent: task_intent(workflow_run_id, task_id),
        };
        match state {
            SchedulerTaskStateKind::AwaitingInputs => SchedulerTaskState::AwaitingInputs {
                diagnostics: Vec::new(),
            },
            SchedulerTaskStateKind::InputUnavailable => SchedulerTaskState::InputUnavailable {
                diagnostics: vec![pantograph_scheduler::SchedulerTaskStateDiagnostic {
                    severity: pantograph_scheduler::SchedulerTaskStateDiagnosticSeverity::Error,
                    code: pantograph_scheduler::SchedulerTaskStateDiagnosticCode::InputUnavailable,
                    message: "source input is unavailable".to_string(),
                    hint: None,
                }],
            },
            SchedulerTaskStateKind::Ready => SchedulerTaskState::Ready { execution_intent },
            SchedulerTaskStateKind::Running => SchedulerTaskState::Running { execution_intent },
            SchedulerTaskStateKind::Completed => SchedulerTaskState::Completed { execution_intent },
            other => panic!("unsupported test state {other:?}"),
        }
    }

    fn task_record(
        workflow_run_id: &str,
        task_id: &str,
        state: SchedulerTaskStateKind,
    ) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            state: task_state(state, workflow_run_id, task_id),
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse(format!("initial:{task_id}"))
                .expect("transition id"),
        }
    }

    fn completion_transition(
        workflow_run_id: &str,
        task_id: &str,
        transition_id: &str,
    ) -> SchedulerTaskStateTransition {
        SchedulerTaskStateTransition {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: SchedulerTaskStateTransitionId::parse(transition_id)
                .expect("transition id"),
            workflow_id: SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            expected_previous_state: Some(SchedulerTaskStateKind::Running),
            next_state: task_state(SchedulerTaskStateKind::Completed, workflow_run_id, task_id),
        }
    }

    fn running_transition(
        workflow_run_id: &str,
        task_id: &str,
        transition_id: &str,
    ) -> SchedulerTaskStateTransition {
        SchedulerTaskStateTransition {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: SchedulerTaskStateTransitionId::parse(transition_id)
                .expect("transition id"),
            workflow_id: SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            expected_previous_state: Some(SchedulerTaskStateKind::Ready),
            next_state: task_state(SchedulerTaskStateKind::Running, workflow_run_id, task_id),
        }
    }

    fn source_input_materialization_transition(
        workflow_run_id: &str,
        task_id: &str,
        transition_id: &str,
    ) -> SchedulerTaskStateTransition {
        SchedulerTaskStateTransition {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            transition_id: SchedulerTaskStateTransitionId::parse(transition_id)
                .expect("transition id"),
            workflow_id: SchedulerWorkflowId::parse("workflow-task-results").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            expected_previous_state: Some(SchedulerTaskStateKind::AwaitingInputs),
            next_state: SchedulerTaskState::Completed {
                execution_intent: SchedulerTaskExecutionIntent::SourceInput {
                    task_intent: SchedulerSourceInputTaskIntent {
                        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                        workflow_id: SchedulerWorkflowId::parse("workflow-task-results")
                            .expect("workflow id"),
                        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id)
                            .expect("run id"),
                        node_id: SchedulerNodeId::parse(task_id).expect("node id"),
                        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
                        task_kind: SchedulerSourceInputTaskKind::parse("text-input")
                            .expect("source-input task kind"),
                    },
                },
            },
        }
    }

    fn store_running_task(
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) -> WorkflowSchedulerTaskAttemptId {
        store
            .set_active_run_scheduler_task_state(
                session_id,
                workflow_run_id,
                task_graph(workflow_run_id, task_id),
                vec![task_record(
                    workflow_run_id,
                    task_id,
                    SchedulerTaskStateKind::Ready,
                )],
            )
            .expect("set ready task");
        let (_applied, attempt_id) = store
            .start_active_run_scheduler_task_attempt(
                session_id,
                workflow_run_id,
                running_transition(workflow_run_id, task_id, "transition.running"),
            )
            .expect("start task attempt");
        attempt_id
    }

    fn store_awaiting_source_input_task(
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) {
        store
            .set_active_run_scheduler_task_state(
                session_id,
                workflow_run_id,
                source_input_task_graph(workflow_run_id, task_id),
                vec![task_record(
                    workflow_run_id,
                    task_id,
                    SchedulerTaskStateKind::AwaitingInputs,
                )],
            )
            .expect("set source input task");
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

    #[test]
    fn active_run_complete_scheduler_task_records_result_and_completed_state() {
        let (mut store, session_id, workflow_run_id) = active_store();
        let attempt_id =
            store_running_task(&mut store, &session_id, &workflow_run_id, "model-task");

        let applied = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &attempt_id,
                completion_transition(&workflow_run_id, "model-task", "transition.completed"),
                task_result("model-task", &workflow_run_id),
            )
            .expect("complete task");

        assert!(matches!(
            applied,
            SchedulerTaskStateTransitionApplyResult::Applied(_)
        ));
        let (_, records) = store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("task state")
            .expect("active task state");
        assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Completed);
        assert_eq!(records[0].state_version, 3);
        let results = store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "model-task");
    }

    #[test]
    fn active_run_materializes_source_input_result_and_completed_state() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store_awaiting_source_input_task(&mut store, &session_id, &workflow_run_id, "prompt");

        let applied = store
            .materialize_active_run_source_input_task(
                &session_id,
                &workflow_run_id,
                source_input_materialization_transition(
                    &workflow_run_id,
                    "prompt",
                    "transition.source-input-materialized",
                ),
                source_input_task_result("prompt", &workflow_run_id),
            )
            .expect("materialize source input");

        assert!(matches!(
            applied,
            SchedulerTaskStateTransitionApplyResult::Applied(_)
        ));
        let (_, records) = store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("task state")
            .expect("active task state");
        assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Completed);
        assert!(records[0]
            .state
            .execution_intent()
            .expect("source input intent")
            .source_input_task_intent()
            .is_some());
        let results = store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].task_id, "prompt");
        assert_eq!(
            results[0].outputs[0].value,
            WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
        );
    }

    #[test]
    fn active_run_materialize_source_input_rejects_non_source_task_without_result() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store_running_task(&mut store, &session_id, &workflow_run_id, "model-task");

        let error = store
            .materialize_active_run_source_input_task(
                &session_id,
                &workflow_run_id,
                source_input_materialization_transition(
                    &workflow_run_id,
                    "model-task",
                    "transition.source-input-materialized",
                ),
                source_input_task_result("model-task", &workflow_run_id),
            )
            .expect_err("non-source input task should be rejected");

        assert!(error.message().contains("must be a source-input task"));
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }

    #[test]
    fn active_run_materialize_source_input_rejects_wrong_state_without_result() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                &workflow_run_id,
                source_input_task_graph(&workflow_run_id, "prompt"),
                vec![task_record(
                    &workflow_run_id,
                    "prompt",
                    SchedulerTaskStateKind::InputUnavailable,
                )],
            )
            .expect("set unavailable source input task");

        let error = store
            .materialize_active_run_source_input_task(
                &session_id,
                &workflow_run_id,
                source_input_materialization_transition(
                    &workflow_run_id,
                    "prompt",
                    "transition.source-input-materialized",
                ),
                source_input_task_result("prompt", &workflow_run_id),
            )
            .expect_err("unavailable source input task should be rejected");

        assert!(error.message().contains("must be awaiting inputs"));
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }

    #[test]
    fn active_run_materialize_source_input_rejects_non_source_transition_without_result() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store_awaiting_source_input_task(&mut store, &session_id, &workflow_run_id, "prompt");
        let mut transition = source_input_materialization_transition(
            &workflow_run_id,
            "prompt",
            "transition.source-input-materialized",
        );
        transition.next_state = SchedulerTaskState::Completed {
            execution_intent: SchedulerTaskExecutionIntent::NonRuntime {
                task_intent: pantograph_scheduler::SchedulerNonRuntimeTaskIntent {
                    contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                    workflow_id: SchedulerWorkflowId::parse("workflow-task-results")
                        .expect("workflow id"),
                    workflow_run_id: SchedulerWorkflowRunId::parse(&workflow_run_id)
                        .expect("run id"),
                    node_id: SchedulerNodeId::parse("prompt").expect("node id"),
                    task_id: SchedulerTaskId::parse("prompt").expect("task id"),
                    task_kind: pantograph_scheduler::SchedulerNonRuntimeTaskKind::parse(
                        "text-output",
                    )
                    .expect("non-runtime task kind"),
                },
            },
        };

        let error = store
            .materialize_active_run_source_input_task(
                &session_id,
                &workflow_run_id,
                transition,
                source_input_task_result("prompt", &workflow_run_id),
            )
            .expect_err("non-source transition should be rejected");

        assert!(error.message().contains("source-input intent"));
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }

    #[test]
    fn active_run_complete_scheduler_task_rejects_non_running_state_without_result() {
        let (mut store, session_id, workflow_run_id) = active_store();
        store
            .set_active_run_scheduler_task_state(
                &session_id,
                &workflow_run_id,
                task_graph(&workflow_run_id, "model-task"),
                vec![task_record(
                    &workflow_run_id,
                    "model-task",
                    SchedulerTaskStateKind::Ready,
                )],
            )
            .expect("set ready task");

        let error = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &WorkflowSchedulerTaskAttemptId::new(),
                completion_transition(&workflow_run_id, "model-task", "transition.completed"),
                task_result("model-task", &workflow_run_id),
            )
            .expect_err("ready task completion should be rejected");

        assert!(error
            .message()
            .contains("has no active attempt for completion"));
        let (_, records) = store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("task state")
            .expect("active task state");
        assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Ready);
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }

    #[test]
    fn active_run_complete_scheduler_task_rejects_wrong_node_without_result() {
        let (mut store, session_id, workflow_run_id) = active_store();
        let attempt_id =
            store_running_task(&mut store, &session_id, &workflow_run_id, "model-task");
        let mut result = task_result("model-task", &workflow_run_id);
        result.node_id = "other-node".to_string();

        let error = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &attempt_id,
                completion_transition(&workflow_run_id, "model-task", "transition.completed"),
                result,
            )
            .expect_err("wrong node result should be rejected");

        assert!(error.message().contains("node id"));
        let (_, records) = store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("task state")
            .expect("active task state");
        assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Running);
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }

    #[test]
    fn active_run_complete_scheduler_task_rejects_duplicate_success() {
        let (mut store, session_id, workflow_run_id) = active_store();
        let attempt_id =
            store_running_task(&mut store, &session_id, &workflow_run_id, "model-task");
        let _ = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &attempt_id,
                completion_transition(&workflow_run_id, "model-task", "transition.completed"),
                task_result("model-task", &workflow_run_id),
            )
            .expect("complete task");

        let error = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &attempt_id,
                completion_transition(&workflow_run_id, "model-task", "transition.completed-again"),
                task_result("model-task", &workflow_run_id),
            )
            .expect_err("duplicate success should be rejected");

        assert!(error
            .message()
            .contains("has no active attempt for completion"));
        let results = store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn active_run_complete_scheduler_task_rejects_non_completed_result_without_state_change() {
        let (mut store, session_id, workflow_run_id) = active_store();
        let attempt_id =
            store_running_task(&mut store, &session_id, &workflow_run_id, "model-task");
        let mut result = task_result("model-task", &workflow_run_id);
        result.status = WorkflowSchedulerTaskResultStatus::Unavailable;
        result.outputs.clear();

        let error = store
            .complete_active_run_scheduler_task(
                &session_id,
                &workflow_run_id,
                &attempt_id,
                completion_transition(&workflow_run_id, "model-task", "transition.completed"),
                result,
            )
            .expect_err("non-completed result should be rejected");

        assert!(error
            .message()
            .contains("failed result transition must end terminal-failed"));
        let (_, records) = store
            .active_run_scheduler_task_state(&session_id, &workflow_run_id)
            .expect("task state")
            .expect("active task state");
        assert_eq!(records[0].state.kind(), SchedulerTaskStateKind::Running);
        assert!(store
            .active_run_scheduler_task_results(&session_id, &workflow_run_id)
            .expect("read results")
            .is_empty());
    }
}
