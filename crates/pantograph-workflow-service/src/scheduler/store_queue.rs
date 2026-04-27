use crate::technical_fit::WorkflowTechnicalFitOverride;
use crate::workflow::{
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionQueueItemStatus,
    WorkflowExecutionSessionRunRequest, WorkflowServiceError,
};
#[cfg(test)]
use crate::WorkflowRunId;

use super::super::policy::{
    WorkflowExecutionSessionAdmissionCandidate, WorkflowExecutionSessionAdmissionInput,
    WorkflowExecutionSessionAdmissionRuntimePosture, WorkflowExecutionSessionWarmCompatibility,
};
use super::super::{
    PriorityThenFifoSchedulerPolicy, WorkflowSchedulerAdmissionOutcome,
    WorkflowSchedulerDecisionReason,
};
use super::{
    unix_timestamp_ms, WorkflowExecutionSessionActiveRun, WorkflowExecutionSessionDequeuedRun,
    WorkflowExecutionSessionQueuedRun, WorkflowExecutionSessionRecord,
    WorkflowExecutionSessionRunFinishState, WorkflowExecutionSessionStore,
};

impl WorkflowExecutionSessionStore {
    pub(crate) fn list_queue(
        &self,
        session_id: &str,
    ) -> Result<Vec<WorkflowExecutionSessionQueueItem>, WorkflowServiceError> {
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        let mut items =
            Vec::with_capacity(state.queue.len() + usize::from(state.active_run.is_some()));
        if let Some(active_run) = state.active_run.as_ref() {
            items.push(WorkflowExecutionSessionQueueItem {
                workflow_run_id: active_run.workflow_run_id.clone(),
                enqueued_at_ms: Some(active_run.enqueued_at_ms),
                dequeued_at_ms: Some(active_run.dequeued_at_ms),
                priority: active_run.priority,
                queue_position: Some(0),
                scheduler_admission_outcome: Some(WorkflowSchedulerAdmissionOutcome::Admitted),
                scheduler_decision_reason: Some(active_run.scheduler_decision_reason),
                status: WorkflowExecutionSessionQueueItemStatus::Running,
            });
        }

        let pending_offset = items.len();
        for (index, queued) in state.queue.iter().enumerate() {
            items.push(WorkflowExecutionSessionQueueItem {
                workflow_run_id: queued.workflow_run_id.clone(),
                enqueued_at_ms: Some(queued.enqueued_at_ms),
                dequeued_at_ms: None,
                priority: queued.priority,
                queue_position: Some(pending_offset + index),
                scheduler_admission_outcome: Some(WorkflowSchedulerAdmissionOutcome::Queued),
                scheduler_decision_reason: Some(queued.scheduler_decision_reason),
                status: WorkflowExecutionSessionQueueItemStatus::Pending,
            });
        }
        Ok(items)
    }

    #[cfg(test)]
    pub(crate) fn enqueue_run(
        &mut self,
        session_id: &str,
        request: &WorkflowExecutionSessionRunRequest,
    ) -> Result<String, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::generate().to_string();
        self.enqueue_run_with_id(session_id, request, workflow_run_id)
    }

    pub(crate) fn enqueue_run_with_id(
        &mut self,
        session_id: &str,
        request: &WorkflowExecutionSessionRunRequest,
        workflow_run_id: String,
    ) -> Result<String, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        let policy = PriorityThenFifoSchedulerPolicy;
        let queued = WorkflowExecutionSessionQueuedRun {
            workflow_run_id: workflow_run_id.clone(),
            enqueued_at_ms: unix_timestamp_ms(),
            workflow_semantic_version: request.workflow_semantic_version.clone(),
            inputs: request.inputs.clone(),
            output_targets: request.output_targets.clone(),
            override_selection: request
                .override_selection
                .as_ref()
                .and_then(WorkflowTechnicalFitOverride::normalized),
            timeout_ms: request.timeout_ms,
            priority: request.priority.unwrap_or(0),
            scheduler_decision_reason: WorkflowSchedulerDecisionReason::HighestPriorityFirst,
            enqueued_tick: tick,
            starvation_bypass_count: 0,
        };

        let insert_index = policy.placement_index_for_enqueue(&state.queue, &queued);
        state.queue.insert(insert_index, queued);
        for item in state.queue.iter_mut().skip(insert_index + 1) {
            item.starvation_bypass_count = item.starvation_bypass_count.saturating_add(1);
        }
        policy.refresh_queue(&mut state.queue);
        Self::mark_session_access(state, tick);
        Ok(workflow_run_id)
    }

    pub(crate) fn queued_run_is_admission_candidate(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<bool, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if let Some(active_run) = state.active_run.as_ref() {
            if active_run.workflow_run_id == queue_id
                || state
                    .queue
                    .iter()
                    .any(|item| item.workflow_run_id == queue_id)
            {
                Self::mark_session_access(state, tick);
                return Ok(false);
            }
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        }

        if !state
            .queue
            .iter()
            .any(|item| item.workflow_run_id == queue_id)
        {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        }

        let policy = PriorityThenFifoSchedulerPolicy;
        policy.refresh_queue(&mut state.queue);
        let admission_input = Self::admission_input_from_state(state);
        let candidate = policy
            .predicted_admission_decision(&admission_input)
            .and_then(|decision| decision.admitted_workflow_run_id)
            .as_deref()
            == Some(queue_id);
        Self::mark_session_access(state, tick);
        Ok(candidate)
    }

    pub(crate) fn set_queue_decision_reason_if_present(
        &mut self,
        session_id: &str,
        queue_id: &str,
        reason: WorkflowSchedulerDecisionReason,
    ) -> Result<bool, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let Some(queued) = state
            .queue
            .iter_mut()
            .find(|queued| queued.workflow_run_id == queue_id)
        else {
            return Ok(false);
        };
        queued.scheduler_decision_reason = reason;
        Self::mark_session_access(state, tick);
        Ok(true)
    }

    pub(crate) fn begin_queued_run(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<Option<WorkflowExecutionSessionDequeuedRun>, WorkflowServiceError> {
        let tick = self.next_tick();
        let capacity_blocked = {
            let state = self.active.get(session_id).ok_or_else(|| {
                WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
            })?;
            !state.runtime_loaded
                && state.active_run.is_none()
                && self.loaded_session_count() >= self.max_loaded_sessions
                && self.runtime_unload_candidates(session_id).is_empty()
        };
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if let Some(active_run) = state.active_run.as_ref() {
            if active_run.workflow_run_id == queue_id
                || state
                    .queue
                    .iter()
                    .any(|item| item.workflow_run_id == queue_id)
            {
                return Ok(None);
            }
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        }

        let policy = PriorityThenFifoSchedulerPolicy;
        policy.refresh_queue(&mut state.queue);
        let admission_input = Self::admission_input_from_state(state);
        if capacity_blocked {
            if let Some(admitted_workflow_run_id) = policy
                .predicted_admission_decision(&admission_input)
                .and_then(|decision| decision.admitted_workflow_run_id)
            {
                if let Some(queued) = state
                    .queue
                    .iter_mut()
                    .find(|queued| queued.workflow_run_id == admitted_workflow_run_id)
                {
                    queued.scheduler_decision_reason =
                        WorkflowSchedulerDecisionReason::WaitingForRuntimeCapacity;
                }
            }
            Self::mark_session_access(state, tick);
            return Ok(None);
        }
        let decision = policy.admission_decision(&admission_input, queue_id)?;
        let Some(admitted_workflow_run_id) = decision.admitted_workflow_run_id.as_deref() else {
            return Ok(None);
        };
        let admitted_index = state
            .queue
            .iter()
            .position(|queued| queued.workflow_run_id == admitted_workflow_run_id)
            .ok_or_else(|| {
                WorkflowServiceError::Internal(format!(
                    "admitted queue item '{}' missing from session '{}'",
                    admitted_workflow_run_id, session_id
                ))
            })?;

        let queued = state.queue.remove(admitted_index);
        for item in &mut state.queue {
            item.starvation_bypass_count = item.starvation_bypass_count.saturating_add(1);
        }
        policy.refresh_queue(&mut state.queue);
        let dequeued_at_ms = unix_timestamp_ms();
        let scheduler_decision_reason = decision.reason.ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "admitted queue item '{}' in session '{}' missing scheduler reason",
                admitted_workflow_run_id, session_id
            ))
        })?;
        state.active_run = Some(WorkflowExecutionSessionActiveRun {
            workflow_run_id: queued.workflow_run_id.clone(),
            enqueued_at_ms: queued.enqueued_at_ms,
            dequeued_at_ms,
            priority: queued.priority,
            scheduler_decision_reason,
        });
        Self::mark_session_access(state, tick);
        Ok(Some(WorkflowExecutionSessionDequeuedRun {
            workflow_id: state.workflow_id.clone(),
            enqueued_at_ms: queued.enqueued_at_ms,
            queued,
            dequeued_at_ms,
            scheduler_decision_reason,
        }))
    }

    pub(crate) fn finish_run(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<WorkflowExecutionSessionRunFinishState, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        let Some(active_run) = state.active_run.as_ref() else {
            return Err(WorkflowServiceError::Internal(format!(
                "session '{}' has no active run",
                session_id
            )));
        };
        if active_run.workflow_run_id != queue_id {
            return Err(WorkflowServiceError::Internal(format!(
                "session '{}' active run '{}' does not match '{}'",
                session_id, active_run.workflow_run_id, queue_id
            )));
        }

        let unload_runtime = state.runtime_loaded && !state.keep_alive;
        state.active_run = None;
        Self::mark_session_access(state, tick);
        state.run_count = state.run_count.saturating_add(1);
        if unload_runtime {
            state.runtime_loaded = false;
        }
        Ok(WorkflowExecutionSessionRunFinishState {
            workflow_id: state.workflow_id.clone(),
            unload_runtime,
        })
    }

    pub(crate) fn cancel_queue_item(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if state
            .active_run
            .as_ref()
            .map(|active| active.workflow_run_id.as_str())
            == Some(queue_id)
        {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "queue item '{}' is currently running",
                queue_id
            )));
        }

        let original_len = state.queue.len();
        state.queue.retain(|item| item.workflow_run_id != queue_id);
        if state.queue.len() == original_len {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        }
        PriorityThenFifoSchedulerPolicy.refresh_queue(&mut state.queue);
        Self::mark_session_access(state, tick);
        Ok(())
    }

    pub(crate) fn reprioritize_queue_item(
        &mut self,
        session_id: &str,
        queue_id: &str,
        priority: i32,
    ) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if state
            .active_run
            .as_ref()
            .map(|active| active.workflow_run_id.as_str())
            == Some(queue_id)
        {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "queue item '{}' is currently running",
                queue_id
            )));
        }

        let Some(item_index) = state
            .queue
            .iter()
            .position(|item| item.workflow_run_id == queue_id)
        else {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        };

        let mut queued = state.queue.remove(item_index);
        queued.priority = priority;
        let policy = PriorityThenFifoSchedulerPolicy;
        let insert_index = policy.placement_index_for_enqueue(&state.queue, &queued);
        state.queue.insert(insert_index, queued);
        if insert_index < item_index {
            for item in state
                .queue
                .iter_mut()
                .skip(insert_index + 1)
                .take(item_index - insert_index)
            {
                item.starvation_bypass_count = item.starvation_bypass_count.saturating_add(1);
            }
        }
        policy.refresh_queue(&mut state.queue);
        Self::mark_session_access(state, tick);
        Ok(())
    }

    pub(super) fn admission_input_from_state(
        state: &WorkflowExecutionSessionRecord,
    ) -> WorkflowExecutionSessionAdmissionInput {
        WorkflowExecutionSessionAdmissionInput {
            has_active_run: state.active_run.is_some(),
            runtime_posture: if state.runtime_loaded {
                WorkflowExecutionSessionAdmissionRuntimePosture::Loaded
            } else {
                WorkflowExecutionSessionAdmissionRuntimePosture::Unloaded
            },
            usage_profile: state.usage_profile.clone(),
            required_backends: state.required_backends.clone(),
            required_models: state.required_models.clone(),
            candidates: state
                .queue
                .iter()
                .enumerate()
                .map(|(queue_position, queued)| {
                    let warm_session_compatibility =
                        Self::warm_session_compatibility(state, queued);
                    WorkflowExecutionSessionAdmissionCandidate {
                        workflow_run_id: queued.workflow_run_id.clone(),
                        priority: queued.priority,
                        enqueued_tick: queued.enqueued_tick,
                        starvation_bypass_count: queued.starvation_bypass_count,
                        queue_position,
                        affine_runtime_reuse: state.runtime_loaded
                            && warm_session_compatibility
                                != WorkflowExecutionSessionWarmCompatibility::Incompatible,
                        warm_session_compatibility,
                    }
                })
                .collect(),
        }
    }

    fn warm_session_compatibility(
        state: &WorkflowExecutionSessionRecord,
        queued: &WorkflowExecutionSessionQueuedRun,
    ) -> WorkflowExecutionSessionWarmCompatibility {
        if !state.runtime_loaded {
            return WorkflowExecutionSessionWarmCompatibility::Unknown;
        }

        let Some(override_selection) = queued.override_selection.as_ref() else {
            return WorkflowExecutionSessionWarmCompatibility::Compatible;
        };

        if let Some(backend_key) = override_selection.backend_key.as_deref() {
            if state.required_backends.is_empty() {
                return WorkflowExecutionSessionWarmCompatibility::Unknown;
            }
            if !state
                .required_backends
                .iter()
                .any(|required| required == backend_key)
            {
                return WorkflowExecutionSessionWarmCompatibility::Incompatible;
            }
        }

        if let Some(model_id) = override_selection.model_id.as_deref() {
            if state.required_models.is_empty() {
                return WorkflowExecutionSessionWarmCompatibility::Unknown;
            }
            if !state
                .required_models
                .iter()
                .any(|required| required == model_id)
            {
                return WorkflowExecutionSessionWarmCompatibility::Incompatible;
            }
        }

        WorkflowExecutionSessionWarmCompatibility::Compatible
    }
}
