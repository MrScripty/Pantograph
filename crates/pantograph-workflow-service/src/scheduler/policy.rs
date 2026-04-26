use std::cmp::Ordering;

use crate::workflow::WorkflowServiceError;

use super::store::WorkflowExecutionSessionQueuedRun;
use super::{
    WorkflowExecutionSessionRuntimeSelectionTarget, WorkflowExecutionSessionRuntimeUnloadCandidate,
    WorkflowSchedulerDecisionReason,
};

const STARVATION_BYPASS_THRESHOLD: u32 = 2;
const WARM_REUSE_FAIRNESS_WINDOW: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowExecutionSessionAdmissionRuntimePosture {
    Loaded,
    Unloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowExecutionSessionWarmCompatibility {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowExecutionSessionAdmissionCandidate {
    pub(crate) workflow_run_id: String,
    pub(crate) priority: i32,
    pub(crate) enqueued_tick: u64,
    pub(crate) starvation_bypass_count: u32,
    pub(crate) queue_position: usize,
    pub(crate) affine_runtime_reuse: bool,
    pub(crate) warm_session_compatibility: WorkflowExecutionSessionWarmCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowExecutionSessionAdmissionInput {
    pub(crate) has_active_run: bool,
    pub(crate) runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture,
    pub(crate) usage_profile: Option<String>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) candidates: Vec<WorkflowExecutionSessionAdmissionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowExecutionSessionAdmissionDecision {
    pub(crate) admitted_workflow_run_id: Option<String>,
    pub(crate) reason: Option<WorkflowSchedulerDecisionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowExecutionSessionCompatibilityKey {
    usage_profile: Option<String>,
    required_backends: Vec<String>,
    required_models: Vec<String>,
}

impl WorkflowExecutionSessionCompatibilityKey {
    fn is_empty(&self) -> bool {
        self.usage_profile.is_none()
            && self.required_backends.is_empty()
            && self.required_models.is_empty()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PriorityThenFifoSchedulerPolicy;

pub fn select_runtime_unload_candidate_by_affinity(
    target: &WorkflowExecutionSessionRuntimeSelectionTarget,
    candidates: &[WorkflowExecutionSessionRuntimeUnloadCandidate],
) -> Option<WorkflowExecutionSessionRuntimeUnloadCandidate> {
    PriorityThenFifoSchedulerPolicy.select_runtime_unload_candidate(target, candidates)
}

impl PriorityThenFifoSchedulerPolicy {
    pub(crate) fn predicted_admission_decision(
        &self,
        input: &WorkflowExecutionSessionAdmissionInput,
    ) -> Option<WorkflowExecutionSessionAdmissionDecision> {
        let candidate = self.select_admission_candidate(input)?;
        Some(WorkflowExecutionSessionAdmissionDecision {
            admitted_workflow_run_id: Some(candidate.workflow_run_id.clone()),
            reason: Some(self.admission_reason(input, candidate)),
        })
    }

    pub(crate) fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
        candidates: &[WorkflowExecutionSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowExecutionSessionRuntimeUnloadCandidate> {
        candidates
            .iter()
            .cloned()
            .min_by(|left, right| self.compare_runtime_unload_candidates(target, left, right))
    }

    pub(crate) fn placement_index_for_enqueue(
        &self,
        queue: &[WorkflowExecutionSessionQueuedRun],
        queued: &WorkflowExecutionSessionQueuedRun,
    ) -> usize {
        queue
            .iter()
            .position(|existing| self.compare_runs(queued, existing) == Ordering::Less)
            .unwrap_or(queue.len())
    }

    pub(crate) fn refresh_queue(&self, queue: &mut [WorkflowExecutionSessionQueuedRun]) {
        queue.sort_by(|left, right| self.compare_runs(left, right));

        for index in 0..queue.len() {
            let reason = self.reason_for_queue_position(queue, index);
            queue[index].scheduler_decision_reason = reason;
        }
    }

    pub(crate) fn admission_decision(
        &self,
        input: &WorkflowExecutionSessionAdmissionInput,
        workflow_run_id: &str,
    ) -> Result<WorkflowExecutionSessionAdmissionDecision, WorkflowServiceError> {
        if input.has_active_run {
            return self.pending_or_not_found(input, workflow_run_id);
        }

        let Some(candidate) = self.select_admission_candidate(input) else {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in queue",
                workflow_run_id
            )));
        };

        if candidate.workflow_run_id == workflow_run_id {
            return Ok(WorkflowExecutionSessionAdmissionDecision {
                admitted_workflow_run_id: Some(candidate.workflow_run_id.clone()),
                reason: Some(self.admission_reason(input, candidate)),
            });
        }

        self.pending_or_not_found(input, workflow_run_id)
    }

    pub(crate) fn select_admission_candidate<'a>(
        &self,
        input: &'a WorkflowExecutionSessionAdmissionInput,
    ) -> Option<&'a WorkflowExecutionSessionAdmissionCandidate> {
        if input.has_active_run {
            return None;
        }

        if let Some(candidate) = self.select_warm_reuse_candidate_within_fairness_window(input) {
            return Some(candidate);
        }

        input
            .candidates
            .iter()
            .min_by(|left, right| self.compare_admission_candidates(left, right))
    }

    fn compare_runs(
        &self,
        left: &WorkflowExecutionSessionQueuedRun,
        right: &WorkflowExecutionSessionQueuedRun,
    ) -> Ordering {
        self.effective_priority(right)
            .cmp(&self.effective_priority(left))
            .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
            .then_with(|| left.workflow_run_id.cmp(&right.workflow_run_id))
    }

    fn compare_admission_candidates(
        &self,
        left: &WorkflowExecutionSessionAdmissionCandidate,
        right: &WorkflowExecutionSessionAdmissionCandidate,
    ) -> Ordering {
        self.admission_effective_priority(right)
            .cmp(&self.admission_effective_priority(left))
            .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
            .then_with(|| left.workflow_run_id.cmp(&right.workflow_run_id))
    }

    fn compare_runtime_unload_candidates(
        &self,
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
        left: &WorkflowExecutionSessionRuntimeUnloadCandidate,
        right: &WorkflowExecutionSessionRuntimeUnloadCandidate,
    ) -> Ordering {
        self.runtime_affinity_rank(target, left)
            .cmp(&self.runtime_affinity_rank(target, right))
            .then_with(|| left.keep_alive.cmp(&right.keep_alive))
            .then_with(|| left.access_tick.cmp(&right.access_tick))
            .then_with(|| left.run_count.cmp(&right.run_count))
            .then_with(|| left.session_id.cmp(&right.session_id))
    }

    fn runtime_affinity_rank(
        &self,
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
        candidate: &WorkflowExecutionSessionRuntimeUnloadCandidate,
    ) -> (bool, bool, bool, bool, bool, bool, bool, bool) {
        let target_key = Self::compatibility_key_from_target(target);
        let candidate_key = Self::compatibility_key_from_candidate(candidate);
        let same_workflow = candidate.workflow_id == target.workflow_id;
        let same_usage_profile = target_key.usage_profile.is_some()
            && candidate_key.usage_profile == target_key.usage_profile;
        let exact_required_backends = !target_key.required_backends.is_empty()
            && candidate_key.required_backends == target_key.required_backends;
        let shared_required_backends = !target_key.required_backends.is_empty()
            && candidate
                .required_backends
                .iter()
                .any(|backend| target_key.required_backends.contains(backend));
        let exact_required_models = !target_key.required_models.is_empty()
            && candidate_key.required_models == target_key.required_models;
        let shared_required_models = !target_key.required_models.is_empty()
            && candidate
                .required_models
                .iter()
                .any(|model_id| target_key.required_models.contains(model_id));
        let exact_compatibility_identity = !target_key.is_empty() && candidate_key == target_key;
        let shared_compatibility_identity =
            same_usage_profile || shared_required_backends || shared_required_models;
        (
            same_workflow,
            exact_compatibility_identity,
            shared_compatibility_identity,
            same_usage_profile,
            exact_required_backends,
            shared_required_backends,
            exact_required_models,
            shared_required_models,
        )
    }

    fn compatibility_key_from_target(
        target: &WorkflowExecutionSessionRuntimeSelectionTarget,
    ) -> WorkflowExecutionSessionCompatibilityKey {
        WorkflowExecutionSessionCompatibilityKey {
            usage_profile: target.usage_profile.clone(),
            required_backends: target.required_backends.clone(),
            required_models: target.required_models.clone(),
        }
    }

    fn compatibility_key_from_candidate(
        candidate: &WorkflowExecutionSessionRuntimeUnloadCandidate,
    ) -> WorkflowExecutionSessionCompatibilityKey {
        WorkflowExecutionSessionCompatibilityKey {
            usage_profile: candidate.usage_profile.clone(),
            required_backends: candidate.required_backends.clone(),
            required_models: candidate.required_models.clone(),
        }
    }

    fn select_warm_reuse_candidate_within_fairness_window<'a>(
        &self,
        input: &'a WorkflowExecutionSessionAdmissionInput,
    ) -> Option<&'a WorkflowExecutionSessionAdmissionCandidate> {
        let highest_effective_priority = input
            .candidates
            .iter()
            .map(|candidate| self.admission_effective_priority(candidate))
            .max()?;

        let priority_band = input
            .candidates
            .iter()
            .filter(|candidate| {
                self.admission_effective_priority(candidate) == highest_effective_priority
            })
            .collect::<Vec<_>>();
        let band_head = priority_band
            .iter()
            .copied()
            .min_by(|left, right| self.compare_admission_candidates(left, right))?;
        if band_head.starvation_bypass_count > 0 {
            return None;
        }

        priority_band
            .into_iter()
            .filter(|candidate| {
                candidate.queue_position
                    <= band_head
                        .queue_position
                        .saturating_add(WARM_REUSE_FAIRNESS_WINDOW)
            })
            .filter(|candidate| {
                candidate.warm_session_compatibility
                    == WorkflowExecutionSessionWarmCompatibility::Compatible
            })
            .min_by(|left, right| {
                left.queue_position
                    .cmp(&right.queue_position)
                    .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
                    .then_with(|| left.workflow_run_id.cmp(&right.workflow_run_id))
            })
    }

    fn admission_reason(
        &self,
        input: &WorkflowExecutionSessionAdmissionInput,
        candidate: &WorkflowExecutionSessionAdmissionCandidate,
    ) -> WorkflowSchedulerDecisionReason {
        match input.runtime_posture {
            WorkflowExecutionSessionAdmissionRuntimePosture::Unloaded => {
                WorkflowSchedulerDecisionReason::ColdStartRequired
            }
            WorkflowExecutionSessionAdmissionRuntimePosture::Loaded => {
                match candidate.warm_session_compatibility {
                    WorkflowExecutionSessionWarmCompatibility::Compatible => {
                        WorkflowSchedulerDecisionReason::WarmSessionReused
                    }
                    WorkflowExecutionSessionWarmCompatibility::Incompatible
                    | WorkflowExecutionSessionWarmCompatibility::Unknown => {
                        WorkflowSchedulerDecisionReason::RuntimeReloadRequired
                    }
                }
            }
        }
    }

    fn effective_priority(&self, queued: &WorkflowExecutionSessionQueuedRun) -> i32 {
        queued
            .priority
            .saturating_add(self.starvation_priority_boost(queued))
    }

    fn starvation_priority_boost(&self, queued: &WorkflowExecutionSessionQueuedRun) -> i32 {
        (queued.starvation_bypass_count / STARVATION_BYPASS_THRESHOLD).min(i32::MAX as u32) as i32
    }

    fn admission_effective_priority(
        &self,
        candidate: &WorkflowExecutionSessionAdmissionCandidate,
    ) -> i32 {
        candidate
            .priority
            .saturating_add(self.admission_starvation_priority_boost(candidate))
    }

    fn admission_starvation_priority_boost(
        &self,
        candidate: &WorkflowExecutionSessionAdmissionCandidate,
    ) -> i32 {
        (candidate.starvation_bypass_count / STARVATION_BYPASS_THRESHOLD).min(i32::MAX as u32)
            as i32
    }

    fn pending_or_not_found(
        &self,
        input: &WorkflowExecutionSessionAdmissionInput,
        workflow_run_id: &str,
    ) -> Result<WorkflowExecutionSessionAdmissionDecision, WorkflowServiceError> {
        if input
            .candidates
            .iter()
            .any(|item| item.workflow_run_id == workflow_run_id)
        {
            return Ok(WorkflowExecutionSessionAdmissionDecision {
                admitted_workflow_run_id: None,
                reason: None,
            });
        }

        Err(WorkflowServiceError::QueueItemNotFound(format!(
            "queue item '{}' not found in queue",
            workflow_run_id
        )))
    }

    fn reason_for_queue_position(
        &self,
        queue: &[WorkflowExecutionSessionQueuedRun],
        index: usize,
    ) -> WorkflowSchedulerDecisionReason {
        let item = &queue[index];
        if index == 0 {
            let promoted_over_higher_base_priority = self.starvation_priority_boost(item) > 0
                && queue
                    .iter()
                    .skip(1)
                    .any(|other| other.priority > item.priority);
            if promoted_over_higher_base_priority {
                WorkflowSchedulerDecisionReason::StarvationProtection
            } else {
                WorkflowSchedulerDecisionReason::HighestPriorityFirst
            }
        } else if queue[..index].iter().any(|ahead| {
            self.effective_priority(ahead) == self.effective_priority(item)
                && ahead.enqueued_tick < item.enqueued_tick
        }) {
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak
        } else {
            WorkflowSchedulerDecisionReason::WaitingForHigherPriority
        }
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
