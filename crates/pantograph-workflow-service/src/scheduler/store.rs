use std::collections::HashMap;

use uuid::Uuid;

use crate::graph::WorkflowSessionKind;
use crate::technical_fit::WorkflowTechnicalFitOverride;
use crate::workflow::{
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowRuntimeIssue, WorkflowServiceError,
    WorkflowSessionRunRequest,
};

use super::{
    PriorityThenFifoSchedulerPolicy, WorkflowSchedulerAdmissionOutcome,
    WorkflowSchedulerDecisionReason, WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowSessionState, WorkflowSessionSummary,
};

pub(crate) const WORKFLOW_SESSION_QUEUE_POLL_MS: u64 = 10;

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionQueuedRun {
    pub(crate) queue_id: String,
    pub(crate) run_id: Option<String>,
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
struct WorkflowSessionActiveRun {
    queue_id: String,
    run_id: Option<String>,
    enqueued_at_ms: u64,
    dequeued_at_ms: u64,
    priority: i32,
    scheduler_decision_reason: WorkflowSchedulerDecisionReason,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionPreflightCache {
    pub(crate) graph_fingerprint: String,
    pub(crate) runtime_capability_fingerprint: String,
    pub(crate) override_selection: Option<WorkflowTechnicalFitOverride>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) blocking_runtime_issues: Vec<WorkflowRuntimeIssue>,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionRecord {
    pub(crate) workflow_id: String,
    pub(crate) usage_profile: Option<String>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) keep_alive: bool,
    pub(crate) runtime_loaded: bool,
    active_run: Option<WorkflowSessionActiveRun>,
    queue: Vec<WorkflowSessionQueuedRun>,
    pub(crate) access_tick: u64,
    pub(crate) last_accessed_at_ms: u64,
    pub(crate) run_count: u64,
    pub(crate) preflight_cache: Option<WorkflowSessionPreflightCache>,
}

impl WorkflowSessionRecord {
    pub(crate) fn queue_len(&self) -> usize {
        self.queue.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSessionStaleCleanupCandidate {
    pub(crate) session_id: String,
    last_accessed_at_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionDequeuedRun {
    pub(crate) workflow_id: String,
    pub(crate) queued: WorkflowSessionQueuedRun,
}

pub(crate) fn unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionCloseState {
    pub(crate) workflow_id: String,
    pub(crate) runtime_loaded: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowSessionRunFinishState {
    pub(crate) workflow_id: String,
    pub(crate) unload_runtime: bool,
}

#[derive(Debug)]
pub(crate) struct WorkflowSessionStore {
    pub(crate) max_sessions: usize,
    pub(crate) max_loaded_sessions: usize,
    tick: u64,
    pub(crate) active: HashMap<String, WorkflowSessionRecord>,
}

impl WorkflowSessionStore {
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
            return Err(WorkflowServiceError::SchedulerBusy(format!(
                "session capacity {} reached; close an existing session before creating another",
                self.max_sessions
            )));
        }

        let session_id = Uuid::new_v4().to_string();
        let now_ms = unix_timestamp_ms();
        let access_tick = self.next_tick();
        let state = WorkflowSessionRecord {
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
    ) -> Vec<WorkflowSessionRuntimeUnloadCandidate> {
        self.active
            .iter()
            .filter(|(session_id, state)| {
                state.runtime_loaded
                    && state.active_run.is_none()
                    && session_id.as_str() != exclude_session_id
            })
            .map(
                |(session_id, state)| WorkflowSessionRuntimeUnloadCandidate {
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
    ) -> Result<WorkflowSessionSummary, WorkflowServiceError> {
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;
        Ok(WorkflowSessionSummary {
            session_id: session_id.to_string(),
            workflow_id: state.workflow_id.clone(),
            session_kind: WorkflowSessionKind::Workflow,
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
    ) -> Result<Option<WorkflowSessionPreflightCache>, WorkflowServiceError> {
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
        cache: WorkflowSessionPreflightCache,
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
    ) -> Result<Vec<WorkflowSessionQueueItem>, WorkflowServiceError> {
        let state = self.active.get(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        let mut items =
            Vec::with_capacity(state.queue.len() + usize::from(state.active_run.is_some()));
        if let Some(active_run) = state.active_run.as_ref() {
            items.push(WorkflowSessionQueueItem {
                queue_id: active_run.queue_id.clone(),
                run_id: active_run.run_id.clone(),
                enqueued_at_ms: Some(active_run.enqueued_at_ms),
                dequeued_at_ms: Some(active_run.dequeued_at_ms),
                priority: active_run.priority,
                queue_position: Some(0),
                scheduler_admission_outcome: Some(WorkflowSchedulerAdmissionOutcome::Admitted),
                scheduler_decision_reason: Some(active_run.scheduler_decision_reason),
                status: WorkflowSessionQueueItemStatus::Running,
            });
        }

        let pending_offset = items.len();
        for (index, queued) in state.queue.iter().enumerate() {
            items.push(WorkflowSessionQueueItem {
                queue_id: queued.queue_id.clone(),
                run_id: queued.run_id.clone(),
                enqueued_at_ms: Some(queued.enqueued_at_ms),
                dequeued_at_ms: None,
                priority: queued.priority,
                queue_position: Some(pending_offset + index),
                scheduler_admission_outcome: Some(WorkflowSchedulerAdmissionOutcome::Queued),
                scheduler_decision_reason: Some(queued.scheduler_decision_reason),
                status: WorkflowSessionQueueItemStatus::Pending,
            });
        }
        Ok(items)
    }

    pub(crate) fn enqueue_run(
        &mut self,
        session_id: &str,
        request: &WorkflowSessionRunRequest,
    ) -> Result<String, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        let policy = PriorityThenFifoSchedulerPolicy;
        let queue_id = Uuid::new_v4().to_string();
        let queued = WorkflowSessionQueuedRun {
            queue_id: queue_id.clone(),
            run_id: request
                .run_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
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
        let queued = queued;
        state.queue.insert(insert_index, queued);
        for item in state.queue.iter_mut().skip(insert_index + 1) {
            item.starvation_bypass_count = item.starvation_bypass_count.saturating_add(1);
        }
        policy.refresh_queue(&mut state.queue);
        Self::mark_session_access(state, tick);
        Ok(queue_id)
    }

    pub(crate) fn begin_queued_run(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<Option<WorkflowSessionDequeuedRun>, WorkflowServiceError> {
        let tick = self.next_tick();
        let state = self.active.get_mut(session_id).ok_or_else(|| {
            WorkflowServiceError::SessionNotFound(format!("session '{}' not found", session_id))
        })?;

        if let Some(active_run) = state.active_run.as_ref() {
            if active_run.queue_id == queue_id
                || state.queue.iter().any(|item| item.queue_id == queue_id)
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

        let Some(front) = state.queue.first() else {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        };
        if front.queue_id != queue_id {
            if policy.admission_reason(&state.queue, queue_id)?.is_none() {
                return Ok(None);
            }
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in session '{}'",
                queue_id, session_id
            )));
        }

        let queued = state.queue.remove(0);
        for item in &mut state.queue {
            item.starvation_bypass_count = item.starvation_bypass_count.saturating_add(1);
        }
        policy.refresh_queue(&mut state.queue);
        state.active_run = Some(WorkflowSessionActiveRun {
            queue_id: queued.queue_id.clone(),
            run_id: queued.run_id.clone(),
            enqueued_at_ms: queued.enqueued_at_ms,
            dequeued_at_ms: unix_timestamp_ms(),
            priority: queued.priority,
            scheduler_decision_reason: WorkflowSchedulerDecisionReason::AdmittedForExecution,
        });
        Self::mark_session_access(state, tick);
        Ok(Some(WorkflowSessionDequeuedRun {
            workflow_id: state.workflow_id.clone(),
            queued,
        }))
    }

    pub(crate) fn finish_run(
        &mut self,
        session_id: &str,
        queue_id: &str,
    ) -> Result<WorkflowSessionRunFinishState, WorkflowServiceError> {
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
        if active_run.queue_id != queue_id {
            return Err(WorkflowServiceError::Internal(format!(
                "session '{}' active run '{}' does not match '{}'",
                session_id, active_run.queue_id, queue_id
            )));
        }

        let unload_runtime = state.runtime_loaded && !state.keep_alive;
        state.active_run = None;
        Self::mark_session_access(state, tick);
        state.run_count = state.run_count.saturating_add(1);
        if unload_runtime {
            state.runtime_loaded = false;
        }
        Ok(WorkflowSessionRunFinishState {
            workflow_id: state.workflow_id.clone(),
            unload_runtime,
        })
    }

    pub(crate) fn set_keep_alive(
        &mut self,
        session_id: &str,
        keep_alive: bool,
    ) -> Result<(WorkflowSessionState, Option<String>), WorkflowServiceError> {
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
            .map(|active| active.queue_id.as_str())
            == Some(queue_id)
        {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "queue item '{}' is currently running",
                queue_id
            )));
        }

        let original_len = state.queue.len();
        state.queue.retain(|item| item.queue_id != queue_id);
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
            .map(|active| active.queue_id.as_str())
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
            .position(|item| item.queue_id == queue_id)
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
    ) -> Vec<WorkflowSessionStaleCleanupCandidate> {
        let mut candidates = self
            .active
            .iter()
            .filter(|(_, state)| {
                !state.keep_alive
                    && !state.runtime_loaded
                    && state.active_run.is_none()
                    && state.last_accessed_at_ms.saturating_add(idle_timeout_ms) <= now_ms
            })
            .map(|(session_id, state)| WorkflowSessionStaleCleanupCandidate {
                session_id: session_id.clone(),
                last_accessed_at_ms: state.last_accessed_at_ms,
            })
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
        candidate: &WorkflowSessionStaleCleanupCandidate,
        now_ms: u64,
        idle_timeout_ms: u64,
    ) -> bool {
        let Some(state) = self.active.get(candidate.session_id.as_str()) else {
            return false;
        };
        if state.keep_alive
            || state.runtime_loaded
            || state.active_run.is_some()
            || state.last_accessed_at_ms != candidate.last_accessed_at_ms
            || state.last_accessed_at_ms.saturating_add(idle_timeout_ms) > now_ms
        {
            return false;
        }

        self.active.remove(candidate.session_id.as_str());
        true
    }

    fn mark_session_access(state: &mut WorkflowSessionRecord, tick: u64) {
        state.access_tick = tick;
        state.last_accessed_at_ms = unix_timestamp_ms();
    }

    pub(crate) fn close_session(
        &mut self,
        session_id: &str,
    ) -> Result<WorkflowSessionCloseState, WorkflowServiceError> {
        let Some(state) = self.active.get(session_id) else {
            return Err(WorkflowServiceError::SessionNotFound(format!(
                "session '{}' not found",
                session_id
            )));
        };
        if state.active_run.is_some() {
            return Err(WorkflowServiceError::SchedulerBusy(format!(
                "session '{}' is currently running",
                session_id
            )));
        }

        let removed = self.active.remove(session_id).expect("session exists");
        Ok(WorkflowSessionCloseState {
            workflow_id: removed.workflow_id,
            runtime_loaded: removed.runtime_loaded,
        })
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

fn session_state_from_record(state: &WorkflowSessionRecord) -> WorkflowSessionState {
    if state.active_run.is_some() {
        WorkflowSessionState::Running
    } else if state.runtime_loaded {
        WorkflowSessionState::IdleLoaded
    } else {
        WorkflowSessionState::IdleUnloaded
    }
}
