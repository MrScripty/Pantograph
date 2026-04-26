use std::collections::HashMap;

use uuid::Uuid;

use crate::graph::WorkflowExecutionSessionKind;
use crate::technical_fit::WorkflowTechnicalFitOverride;
use crate::workflow::{
    WorkflowExecutionSessionRunRequest, WorkflowOutputTarget, WorkflowPortBinding,
    WorkflowRuntimeIssue, WorkflowServiceError,
};

use super::policy::{
    WorkflowExecutionSessionAdmissionCandidate, WorkflowExecutionSessionAdmissionInput,
    WorkflowExecutionSessionAdmissionRuntimePosture, WorkflowExecutionSessionWarmCompatibility,
};
use super::{
    PriorityThenFifoSchedulerPolicy, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionQueueItemStatus, WorkflowExecutionSessionRuntimeUnloadCandidate,
    WorkflowExecutionSessionState, WorkflowExecutionSessionSummary,
    WorkflowSchedulerAdmissionOutcome, WorkflowSchedulerDecisionReason,
};
use crate::WorkflowRunId;

pub(crate) const WORKFLOW_SESSION_QUEUE_POLL_MS: u64 = 10;

#[path = "store_diagnostics.rs"]
mod store_diagnostics;

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionQueuedRun {
    pub(crate) workflow_run_id: String,
    pub(super) enqueued_at_ms: u64,
    pub(crate) inputs: Vec<WorkflowPortBinding>,
    pub(crate) output_targets: Option<Vec<WorkflowOutputTarget>>,
    pub(crate) override_selection: Option<WorkflowTechnicalFitOverride>,
    pub(crate) timeout_ms: Option<u64>,
    pub(crate) priority: i32,
    pub(super) scheduler_decision_reason: WorkflowSchedulerDecisionReason,
    pub(crate) enqueued_tick: u64,
    pub(super) starvation_bypass_count: u32,
}

#[derive(Debug, Clone)]
struct WorkflowExecutionSessionActiveRun {
    workflow_run_id: String,
    enqueued_at_ms: u64,
    dequeued_at_ms: u64,
    priority: i32,
    scheduler_decision_reason: WorkflowSchedulerDecisionReason,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionPreflightCache {
    pub(crate) graph_fingerprint: String,
    pub(crate) runtime_capability_fingerprint: String,
    pub(crate) override_selection: Option<WorkflowTechnicalFitOverride>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) blocking_runtime_issues: Vec<WorkflowRuntimeIssue>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionRecord {
    pub(crate) workflow_id: String,
    pub(crate) usage_profile: Option<String>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) keep_alive: bool,
    pub(crate) runtime_loaded: bool,
    active_run: Option<WorkflowExecutionSessionActiveRun>,
    queue: Vec<WorkflowExecutionSessionQueuedRun>,
    pub(crate) access_tick: u64,
    pub(crate) last_accessed_at_ms: u64,
    pub(crate) run_count: u64,
    pub(crate) preflight_cache: Option<WorkflowExecutionSessionPreflightCache>,
}

impl WorkflowExecutionSessionRecord {
    pub(crate) fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowExecutionSessionStaleCleanupCandidate {
    pub(crate) session_id: String,
    last_accessed_at_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionDequeuedRun {
    pub(crate) workflow_id: String,
    pub(crate) queued: WorkflowExecutionSessionQueuedRun,
}

pub(crate) fn unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionCloseState {
    pub(crate) workflow_id: String,
    pub(crate) runtime_loaded: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowExecutionSessionRunFinishState {
    pub(crate) workflow_id: String,
    pub(crate) unload_runtime: bool,
}

#[derive(Debug)]
pub(crate) struct WorkflowExecutionSessionStore {
    pub(crate) max_sessions: usize,
    pub(crate) max_loaded_sessions: usize,
    tick: u64,
    pub(crate) active: HashMap<String, WorkflowExecutionSessionRecord>,
}

impl WorkflowExecutionSessionStore {
    pub(crate) fn new(max_sessions: usize, max_loaded_sessions: usize) -> Self {
        let max_sessions = max_sessions.max(1);
        let max_loaded_sessions = max_loaded_sessions.max(1).min(max_sessions);
        Self {
            max_sessions,
            max_loaded_sessions,
            tick: 0,
            active: HashMap::new(),
        }
    }

    fn next_tick(&mut self) -> u64 {
        self.tick = self.tick.saturating_add(1);
        self.tick
    }

    pub(crate) fn create_session(
        &mut self,
        workflow_id: String,
        usage_profile: Option<String>,
        required_backends: Vec<String>,
        required_models: Vec<String>,
        keep_alive: bool,
    ) -> Result<String, WorkflowServiceError> {
        if self.active.len() >= self.max_sessions {
            return Err(WorkflowServiceError::scheduler_session_capacity_reached(
                self.active.len(),
                self.max_sessions,
            ));
        }

        let session_id = Uuid::new_v4().to_string();
        let now_ms = unix_timestamp_ms();
        let access_tick = self.next_tick();
        let state = WorkflowExecutionSessionRecord {
            workflow_id,
            usage_profile,
            required_backends: normalize_affinity_values(required_backends),
            required_models: normalize_affinity_values(required_models),
            keep_alive,
            runtime_loaded: false,
            active_run: None,
            queue: Vec::new(),
            access_tick,
            last_accessed_at_ms: now_ms,
            run_count: 0,
            preflight_cache: None,
        };
        self.active.insert(session_id.clone(), state);
        Ok(session_id)
    }

    pub(crate) fn loaded_session_count(&self) -> usize {
        self.active
            .values()
            .filter(|state| state.runtime_loaded)
            .count()
    }

    pub(crate) fn runtime_unload_candidates(
        &self,
        exclude_session_id: &str,
    ) -> Vec<WorkflowExecutionSessionRuntimeUnloadCandidate> {
        self.active
            .iter()
            .filter(|(session_id, state)| {
                state.runtime_loaded
                    && state.active_run.is_none()
                    && session_id.as_str() != exclude_session_id
            })
            .map(
                |(session_id, state)| WorkflowExecutionSessionRuntimeUnloadCandidate {
                    session_id: session_id.clone(),
                    workflow_id: state.workflow_id.clone(),
                    usage_profile: state.usage_profile.clone(),
                    required_backends: state.required_backends.clone(),
                    required_models: state.required_models.clone(),
                    keep_alive: state.keep_alive,
                    access_tick: state.access_tick,
                    run_count: state.run_count,
                },
            )
            .collect()
    }

    pub(crate) fn mark_runtime_loaded(
        &mut self,
        session_id: &str,
        loaded: bool,
    ) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        state.runtime_loaded = loaded;
        Self::mark_session_access(state, tick);
        Ok(())
    }

    pub(crate) fn invalidate_all_loaded_session_runtimes(&mut self) -> Vec<String> {
        let session_ids = self
            .active
            .iter()
            .filter_map(|(session_id, state)| state.runtime_loaded.then_some(session_id.clone()))
            .collect::<Vec<_>>();

        for session_id in &session_ids {
            let _ = self.mark_runtime_loaded(session_id, false);
        }

        session_ids
    }

    pub(crate) fn touch_session(&mut self, session_id: &str) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        Self::mark_session_access(state, tick);
        Ok(())
    }

    pub(crate) fn session_summary(
        &self,
        session_id: &str,
    ) -> Result<WorkflowExecutionSessionSummary, WorkflowServiceError> {
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        Ok(WorkflowExecutionSessionSummary {
            session_id: session_id.to_string(),
            workflow_id: state.workflow_id.clone(),
            session_kind: WorkflowExecutionSessionKind::Workflow,
            usage_profile: state.usage_profile.clone(),
            keep_alive: state.keep_alive,
            state: session_state_from_record(state),
            queued_runs: state.queue.len(),
            run_count: state.run_count,
        })
    }

    pub(crate) fn cached_preflight(
        &self,
        session_id: &str,
    ) -> Result<Option<WorkflowExecutionSessionPreflightCache>, WorkflowServiceError> {
        Ok(self
            .active
            .get(session_id)
            .ok_or_else(|| {
                WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
            })?
            .preflight_cache
            .clone())
    }

    pub(crate) fn cache_preflight(
        &mut self,
        session_id: &str,
        cache: WorkflowExecutionSessionPreflightCache,
    ) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        state.preflight_cache = Some(cache);
        state.required_backends = state
            .preflight_cache
            .as_ref()
            .map(|cache| cache.required_backends.clone())
            .unwrap_or_default();
        state.required_models = state
            .preflight_cache
            .as_ref()
            .map(|cache| cache.required_models.clone())
            .unwrap_or_default();
        Self::mark_session_access(state, tick);
        Ok(())
    }

    pub(crate) fn update_runtime_affinity_basis(
        &mut self,
        session_id: &str,
        required_backends: Vec<String>,
        required_models: Vec<String>,
    ) -> Result<(), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        state.required_backends = normalize_affinity_values(required_backends);
        state.required_models = normalize_affinity_values(required_models);
        Self::mark_session_access(state, tick);
        Ok(())
    }

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

    pub(crate) fn enqueue_run(
        &mut self,
        session_id: &str,
        request: &WorkflowExecutionSessionRunRequest,
    ) -> Result<String, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        let policy = PriorityThenFifoSchedulerPolicy;
        let workflow_run_id = WorkflowRunId::generate().to_string();
        let queued = WorkflowExecutionSessionQueuedRun {
            workflow_run_id: workflow_run_id.clone(),
            enqueued_at_ms: unix_timestamp_ms(),
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
        state.active_run = Some(WorkflowExecutionSessionActiveRun {
            workflow_run_id: queued.workflow_run_id.clone(),
            enqueued_at_ms: queued.enqueued_at_ms,
            dequeued_at_ms: unix_timestamp_ms(),
            priority: queued.priority,
            scheduler_decision_reason: decision.reason.ok_or_else(|| {
                WorkflowServiceError::Internal(format!(
                    "admitted queue item '{}' in session '{}' missing scheduler reason",
                    admitted_workflow_run_id, session_id
                ))
            })?,
        });
        Self::mark_session_access(state, tick);
        Ok(Some(WorkflowExecutionSessionDequeuedRun {
            workflow_id: state.workflow_id.clone(),
            queued,
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

    pub(crate) fn set_keep_alive(
        &mut self,
        session_id: &str,
        keep_alive: bool,
    ) -> Result<(WorkflowExecutionSessionState, Option<String>), WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        state.keep_alive = keep_alive;
        Self::mark_session_access(state, tick);

        let unload_workflow_id =
            if !keep_alive && state.runtime_loaded && state.active_run.is_none() {
                state.runtime_loaded = false;
                Some(state.workflow_id.clone())
            } else {
                None
            };

        Ok((session_state_from_record(state), unload_workflow_id))
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

    pub(crate) fn stale_cleanup_candidates(
        &self,
        now_ms: u64,
        idle_timeout_ms: u64,
    ) -> Vec<WorkflowExecutionSessionStaleCleanupCandidate> {
        let mut candidates = self
            .active
            .iter()
            .filter(|(_, state)| {
                !state.keep_alive
                    && !state.runtime_loaded
                    && state.active_run.is_none()
                    && state.queue.is_empty()
                    && state.last_accessed_at_ms.saturating_add(idle_timeout_ms) <= now_ms
            })
            .map(
                |(session_id, state)| WorkflowExecutionSessionStaleCleanupCandidate {
                    session_id: session_id.clone(),
                    last_accessed_at_ms: state.last_accessed_at_ms,
                },
            )
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.last_accessed_at_ms
                .cmp(&right.last_accessed_at_ms)
                .then_with(|| left.session_id.cmp(&right.session_id))
        });
        candidates
    }

    pub(crate) fn close_stale_session_if_unchanged(
        &mut self,
        candidate: &WorkflowExecutionSessionStaleCleanupCandidate,
        now_ms: u64,
        idle_timeout_ms: u64,
    ) -> bool {
        let Some(state) = self.active.get(candidate.session_id.as_str()) else {
            return false;
        };
        if state.keep_alive
            || state.runtime_loaded
            || state.active_run.is_some()
            || !state.queue.is_empty()
            || state.last_accessed_at_ms != candidate.last_accessed_at_ms
            || state.last_accessed_at_ms.saturating_add(idle_timeout_ms) > now_ms
        {
            return false;
        }

        self.active.remove(candidate.session_id.as_str());
        true
    }

    fn mark_session_access(state: &mut WorkflowExecutionSessionRecord, tick: u64) {
        state.access_tick = tick;
        state.last_accessed_at_ms = unix_timestamp_ms();
    }

    pub(crate) fn close_session(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowExecutionSessionCloseState, WorkflowServiceError> {
        let Some(state) = self.active.get(session_id) else {
            return Err(WorkflowServiceError::SessionNotFound(format!(
                "session '{}' not found",
                session_id
            )));
        };
        if state.active_run.is_some() {
            return Err(WorkflowServiceError::scheduler_busy(format!(
                "session '{}' is currently running",
                session_id
            )));
        }

        let removed = self.active.remove(session_id).expect("session exists");
        Ok(WorkflowExecutionSessionCloseState {
            workflow_id: removed.workflow_id,
            runtime_loaded: removed.runtime_loaded,
        })
    }

    fn admission_input_from_state(
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

fn normalize_affinity_values(values: Vec<String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn session_state_from_record(
    state: &WorkflowExecutionSessionRecord,
) -> WorkflowExecutionSessionState {
    if state.active_run.is_some() {
        WorkflowExecutionSessionState::Running
    } else if state.runtime_loaded {
        WorkflowExecutionSessionState::IdleLoaded
    } else {
        WorkflowExecutionSessionState::IdleUnloaded
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
