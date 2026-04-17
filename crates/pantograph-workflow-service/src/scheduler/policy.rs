use std::cmp::Ordering;

use crate::workflow::WorkflowServiceError;

use super::store::WorkflowSessionQueuedRun;
use super::{
    WorkflowSchedulerDecisionReason, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate,
};

const STARVATION_BYPASS_THRESHOLD: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSessionAdmissionRuntimePosture {
    Loaded,
    Unloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowSessionWarmCompatibility {
    Compatible,
    Incompatible,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSessionAdmissionCandidate {
    pub(crate) queue_id: String,
    pub(crate) priority: i32,
    pub(crate) enqueued_tick: u64,
    pub(crate) starvation_bypass_count: u32,
    pub(crate) queue_position: usize,
    pub(crate) affine_runtime_reuse: bool,
    pub(crate) warm_session_compatibility: WorkflowSessionWarmCompatibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSessionAdmissionInput {
    pub(crate) has_active_run: bool,
    pub(crate) runtime_posture: WorkflowSessionAdmissionRuntimePosture,
    pub(crate) usage_profile: Option<String>,
    pub(crate) required_backends: Vec<String>,
    pub(crate) required_models: Vec<String>,
    pub(crate) candidates: Vec<WorkflowSessionAdmissionCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowSessionAdmissionDecision {
    pub(crate) admitted_queue_id: Option<String>,
    pub(crate) reason: Option<WorkflowSchedulerDecisionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowSessionCompatibilityKey {
    usage_profile: Option<String>,
    required_backends: Vec<String>,
    required_models: Vec<String>,
}

impl WorkflowSessionCompatibilityKey {
    fn is_empty(&self) -> bool {
        self.usage_profile.is_none()
            && self.required_backends.is_empty()
            && self.required_models.is_empty()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct PriorityThenFifoSchedulerPolicy;

pub fn select_runtime_unload_candidate_by_affinity(
    target: &WorkflowSessionRuntimeSelectionTarget,
    candidates: &[WorkflowSessionRuntimeUnloadCandidate],
) -> Option<WorkflowSessionRuntimeUnloadCandidate> {
    PriorityThenFifoSchedulerPolicy.select_runtime_unload_candidate(target, candidates)
}

impl PriorityThenFifoSchedulerPolicy {
    pub(crate) fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowSessionRuntimeUnloadCandidate> {
        candidates
            .iter()
            .cloned()
            .min_by(|left, right| self.compare_runtime_unload_candidates(target, left, right))
    }

    pub(crate) fn placement_index_for_enqueue(
        &self,
        queue: &[WorkflowSessionQueuedRun],
        queued: &WorkflowSessionQueuedRun,
    ) -> usize {
        queue
            .iter()
            .position(|existing| self.compare_runs(queued, existing) == Ordering::Less)
            .unwrap_or(queue.len())
    }

    pub(crate) fn refresh_queue(&self, queue: &mut [WorkflowSessionQueuedRun]) {
        queue.sort_by(|left, right| self.compare_runs(left, right));

        for index in 0..queue.len() {
            let reason = self.reason_for_queue_position(queue, index);
            queue[index].scheduler_decision_reason = reason;
        }
    }

    pub(crate) fn admission_decision(
        &self,
        input: &WorkflowSessionAdmissionInput,
        queue_id: &str,
    ) -> Result<WorkflowSessionAdmissionDecision, WorkflowServiceError> {
        if input.has_active_run {
            return self.pending_or_not_found(input, queue_id);
        }

        let Some(candidate) = self.select_admission_candidate(input) else {
            return Err(WorkflowServiceError::QueueItemNotFound(format!(
                "queue item '{}' not found in queue",
                queue_id
            )));
        };

        if candidate.queue_id == queue_id {
            return Ok(WorkflowSessionAdmissionDecision {
                admitted_queue_id: Some(candidate.queue_id.clone()),
                reason: Some(self.admission_reason(input, candidate)),
            });
        }

        self.pending_or_not_found(input, queue_id)
    }

    pub(crate) fn select_admission_candidate<'a>(
        &self,
        input: &'a WorkflowSessionAdmissionInput,
    ) -> Option<&'a WorkflowSessionAdmissionCandidate> {
        if input.has_active_run {
            return None;
        }

        input
            .candidates
            .iter()
            .min_by(|left, right| self.compare_admission_candidates(left, right))
    }

    fn compare_runs(
        &self,
        left: &WorkflowSessionQueuedRun,
        right: &WorkflowSessionQueuedRun,
    ) -> Ordering {
        self.effective_priority(right)
            .cmp(&self.effective_priority(left))
            .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
            .then_with(|| left.queue_id.cmp(&right.queue_id))
    }

    fn compare_admission_candidates(
        &self,
        left: &WorkflowSessionAdmissionCandidate,
        right: &WorkflowSessionAdmissionCandidate,
    ) -> Ordering {
        self.admission_effective_priority(right)
            .cmp(&self.admission_effective_priority(left))
            .then_with(|| left.enqueued_tick.cmp(&right.enqueued_tick))
            .then_with(|| left.queue_id.cmp(&right.queue_id))
    }

    fn compare_runtime_unload_candidates(
        &self,
        target: &WorkflowSessionRuntimeSelectionTarget,
        left: &WorkflowSessionRuntimeUnloadCandidate,
        right: &WorkflowSessionRuntimeUnloadCandidate,
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
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidate: &WorkflowSessionRuntimeUnloadCandidate,
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
        target: &WorkflowSessionRuntimeSelectionTarget,
    ) -> WorkflowSessionCompatibilityKey {
        WorkflowSessionCompatibilityKey {
            usage_profile: target.usage_profile.clone(),
            required_backends: target.required_backends.clone(),
            required_models: target.required_models.clone(),
        }
    }

    fn compatibility_key_from_candidate(
        candidate: &WorkflowSessionRuntimeUnloadCandidate,
    ) -> WorkflowSessionCompatibilityKey {
        WorkflowSessionCompatibilityKey {
            usage_profile: candidate.usage_profile.clone(),
            required_backends: candidate.required_backends.clone(),
            required_models: candidate.required_models.clone(),
        }
    }

    fn admission_reason(
        &self,
        input: &WorkflowSessionAdmissionInput,
        candidate: &WorkflowSessionAdmissionCandidate,
    ) -> WorkflowSchedulerDecisionReason {
        match input.runtime_posture {
            WorkflowSessionAdmissionRuntimePosture::Unloaded => {
                WorkflowSchedulerDecisionReason::ColdStartRequired
            }
            WorkflowSessionAdmissionRuntimePosture::Loaded => {
                match candidate.warm_session_compatibility {
                    WorkflowSessionWarmCompatibility::Compatible => {
                        WorkflowSchedulerDecisionReason::WarmSessionReused
                    }
                    WorkflowSessionWarmCompatibility::Incompatible
                    | WorkflowSessionWarmCompatibility::Unknown => {
                        WorkflowSchedulerDecisionReason::RuntimeReloadRequired
                    }
                }
            }
        }
    }

    fn effective_priority(&self, queued: &WorkflowSessionQueuedRun) -> i32 {
        queued
            .priority
            .saturating_add(self.starvation_priority_boost(queued))
    }

    fn starvation_priority_boost(&self, queued: &WorkflowSessionQueuedRun) -> i32 {
        (queued.starvation_bypass_count / STARVATION_BYPASS_THRESHOLD).min(i32::MAX as u32) as i32
    }

    fn admission_effective_priority(&self, candidate: &WorkflowSessionAdmissionCandidate) -> i32 {
        candidate
            .priority
            .saturating_add(self.admission_starvation_priority_boost(candidate))
    }

    fn admission_starvation_priority_boost(
        &self,
        candidate: &WorkflowSessionAdmissionCandidate,
    ) -> i32 {
        (candidate.starvation_bypass_count / STARVATION_BYPASS_THRESHOLD).min(i32::MAX as u32)
            as i32
    }

    fn pending_or_not_found(
        &self,
        input: &WorkflowSessionAdmissionInput,
        queue_id: &str,
    ) -> Result<WorkflowSessionAdmissionDecision, WorkflowServiceError> {
        if input
            .candidates
            .iter()
            .any(|item| item.queue_id == queue_id)
        {
            return Ok(WorkflowSessionAdmissionDecision {
                admitted_queue_id: None,
                reason: None,
            });
        }

        Err(WorkflowServiceError::QueueItemNotFound(format!(
            "queue item '{}' not found in queue",
            queue_id
        )))
    }

    fn reason_for_queue_position(
        &self,
        queue: &[WorkflowSessionQueuedRun],
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
mod tests {
    use crate::workflow::{WorkflowOutputTarget, WorkflowPortBinding};

    use super::*;

    fn queued_run(
        queue_id: &str,
        priority: i32,
        enqueued_tick: u64,
        starvation_bypass_count: u32,
    ) -> WorkflowSessionQueuedRun {
        WorkflowSessionQueuedRun {
            queue_id: queue_id.to_string(),
            run_id: Some(queue_id.to_string()),
            enqueued_at_ms: 0,
            inputs: Vec::<WorkflowPortBinding>::new(),
            output_targets: Some(Vec::<WorkflowOutputTarget>::new()),
            override_selection: None,
            timeout_ms: None,
            priority,
            scheduler_decision_reason: WorkflowSchedulerDecisionReason::WaitingForHigherPriority,
            enqueued_tick,
            starvation_bypass_count,
        }
    }

    fn runtime_target(
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        required_backends: &[&str],
        required_models: &[&str],
    ) -> WorkflowSessionRuntimeSelectionTarget {
        WorkflowSessionRuntimeSelectionTarget {
            session_id: session_id.to_string(),
            workflow_id: workflow_id.to_string(),
            usage_profile: usage_profile.map(str::to_string),
            required_backends: required_backends
                .iter()
                .map(|backend| backend.to_string())
                .collect(),
            required_models: required_models
                .iter()
                .map(|model_id| model_id.to_string())
                .collect(),
        }
    }

    fn unload_candidate(
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        required_backends: &[&str],
        required_models: &[&str],
        keep_alive: bool,
        access_tick: u64,
    ) -> WorkflowSessionRuntimeUnloadCandidate {
        WorkflowSessionRuntimeUnloadCandidate {
            session_id: session_id.to_string(),
            workflow_id: workflow_id.to_string(),
            usage_profile: usage_profile.map(str::to_string),
            required_backends: required_backends
                .iter()
                .map(|backend| backend.to_string())
                .collect(),
            required_models: required_models
                .iter()
                .map(|model_id| model_id.to_string())
                .collect(),
            keep_alive,
            access_tick,
            run_count: 0,
        }
    }

    fn admission_candidate(
        queue_id: &str,
        priority: i32,
        enqueued_tick: u64,
        starvation_bypass_count: u32,
        queue_position: usize,
        affine_runtime_reuse: bool,
        warm_session_compatibility: WorkflowSessionWarmCompatibility,
    ) -> WorkflowSessionAdmissionCandidate {
        WorkflowSessionAdmissionCandidate {
            queue_id: queue_id.to_string(),
            priority,
            enqueued_tick,
            starvation_bypass_count,
            queue_position,
            affine_runtime_reuse,
            warm_session_compatibility,
        }
    }

    #[test]
    fn refresh_queue_promotes_starved_run_over_newer_higher_priority_items() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let mut queue = vec![
            queued_run("high-1", 2, 2, 0),
            queued_run("high-2", 2, 3, 0),
            queued_run("starved", 0, 1, 4),
        ];

        policy.refresh_queue(&mut queue);

        assert_eq!(queue[0].queue_id, "starved");
        assert_eq!(
            queue[0].scheduler_decision_reason,
            WorkflowSchedulerDecisionReason::StarvationProtection
        );
        assert_eq!(queue[1].queue_id, "high-1");
        assert_eq!(
            queue[1].scheduler_decision_reason,
            WorkflowSchedulerDecisionReason::FifoPriorityTieBreak
        );
        assert_eq!(queue[2].queue_id, "high-2");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_non_affine_idle_sessions() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target("session-target", "wf-a", Some("interactive"), &[], &[]);
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-same",
                        "wf-a",
                        Some("interactive"),
                        &[],
                        &[],
                        false,
                        1,
                    ),
                    unload_candidate("session-other", "wf-b", Some("batch"), &[], &[], true, 10),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-other");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_usage_mismatch_before_same_profile() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target("session-target", "wf-a", Some("interactive"), &[], &[]);
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-same-profile",
                        "wf-a",
                        Some("interactive"),
                        &[],
                        &[],
                        false,
                        1,
                    ),
                    unload_candidate(
                        "session-other-profile",
                        "wf-a",
                        Some("batch"),
                        &[],
                        &[],
                        true,
                        10,
                    ),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-other-profile");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_usage_mismatch_across_workflows() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target(
            "session-target",
            "wf-target",
            Some("interactive"),
            &["llama_cpp"],
            &["model-a"],
        );
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-same-profile",
                        "wf-a",
                        Some("interactive"),
                        &["llama_cpp"],
                        &["model-a"],
                        false,
                        1,
                    ),
                    unload_candidate(
                        "session-other-profile",
                        "wf-b",
                        Some("batch"),
                        &["llama_cpp"],
                        &["model-a"],
                        true,
                        10,
                    ),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-other-profile");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_unrelated_models_before_shared_models() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target(
            "session-target",
            "wf-target",
            Some("interactive"),
            &["llama_cpp"],
            &["model-a"],
        );
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-shared-model",
                        "wf-shared",
                        Some("batch"),
                        &["llama_cpp"],
                        &["model-a"],
                        false,
                        1,
                    ),
                    unload_candidate(
                        "session-other-model",
                        "wf-other",
                        Some("batch"),
                        &["pytorch"],
                        &["model-b"],
                        true,
                        10,
                    ),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-other-model");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_partial_model_overlap_before_exact_match() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target(
            "session-target",
            "wf-target",
            Some("interactive"),
            &["llama_cpp"],
            &["model-a", "model-b"],
        );
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-exact-models",
                        "wf-other-exact",
                        Some("batch"),
                        &["llama_cpp"],
                        &["model-a", "model-b"],
                        false,
                        1,
                    ),
                    unload_candidate(
                        "session-partial-models",
                        "wf-other-partial",
                        Some("batch"),
                        &["llama_cpp"],
                        &["model-a"],
                        true,
                        10,
                    ),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-partial-models");
    }

    #[test]
    fn select_runtime_unload_candidate_prefers_unrelated_backends_before_shared_backends() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let target = runtime_target(
            "session-target",
            "wf-target",
            Some("interactive"),
            &["llama_cpp"],
            &["model-a"],
        );
        let selected = policy
            .select_runtime_unload_candidate(
                &target,
                &[
                    unload_candidate(
                        "session-shared-backend",
                        "wf-shared",
                        Some("batch"),
                        &["llama_cpp"],
                        &["model-z"],
                        false,
                        1,
                    ),
                    unload_candidate(
                        "session-other-backend",
                        "wf-other",
                        Some("batch"),
                        &["pytorch"],
                        &["model-a"],
                        true,
                        10,
                    ),
                ],
            )
            .expect("candidate");

        assert_eq!(selected.session_id, "session-other-backend");
    }

    #[test]
    fn admission_decision_selects_highest_priority_candidate_from_admission_input() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let input = WorkflowSessionAdmissionInput {
            has_active_run: false,
            runtime_posture: WorkflowSessionAdmissionRuntimePosture::Loaded,
            usage_profile: Some("interactive".to_string()),
            required_backends: vec!["llama_cpp".to_string()],
            required_models: vec!["model-a".to_string()],
            candidates: vec![
                admission_candidate(
                    "lower-priority",
                    0,
                    1,
                    0,
                    0,
                    true,
                    WorkflowSessionWarmCompatibility::Compatible,
                ),
                admission_candidate(
                    "higher-priority",
                    2,
                    2,
                    0,
                    1,
                    false,
                    WorkflowSessionWarmCompatibility::Incompatible,
                ),
            ],
        };

        let decision = policy
            .admission_decision(&input, "higher-priority")
            .expect("admission decision");

        assert_eq!(
            decision.admitted_queue_id.as_deref(),
            Some("higher-priority")
        );
        assert_eq!(
            decision.reason,
            Some(WorkflowSchedulerDecisionReason::RuntimeReloadRequired)
        );
    }

    #[test]
    fn admission_decision_keeps_pending_candidate_when_another_item_is_selected() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let input = WorkflowSessionAdmissionInput {
            has_active_run: false,
            runtime_posture: WorkflowSessionAdmissionRuntimePosture::Loaded,
            usage_profile: Some("interactive".to_string()),
            required_backends: vec!["llama_cpp".to_string()],
            required_models: vec!["model-a".to_string()],
            candidates: vec![
                admission_candidate(
                    "selected",
                    2,
                    1,
                    0,
                    0,
                    true,
                    WorkflowSessionWarmCompatibility::Compatible,
                ),
                admission_candidate(
                    "pending",
                    0,
                    2,
                    0,
                    1,
                    false,
                    WorkflowSessionWarmCompatibility::Incompatible,
                ),
            ],
        };

        let decision = policy
            .admission_decision(&input, "pending")
            .expect("pending decision");

        assert_eq!(decision.admitted_queue_id, None);
        assert_eq!(decision.reason, None);
    }

    #[test]
    fn admission_decision_reports_warm_session_reuse_for_loaded_compatible_runtime() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let input = WorkflowSessionAdmissionInput {
            has_active_run: false,
            runtime_posture: WorkflowSessionAdmissionRuntimePosture::Loaded,
            usage_profile: Some("interactive".to_string()),
            required_backends: vec!["llama_cpp".to_string()],
            required_models: vec!["model-a".to_string()],
            candidates: vec![admission_candidate(
                "selected",
                2,
                1,
                0,
                0,
                true,
                WorkflowSessionWarmCompatibility::Compatible,
            )],
        };

        let decision = policy
            .admission_decision(&input, "selected")
            .expect("admission decision");

        assert_eq!(
            decision.reason,
            Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
        );
    }

    #[test]
    fn admission_decision_reports_cold_start_when_runtime_is_unloaded() {
        let policy = PriorityThenFifoSchedulerPolicy;
        let input = WorkflowSessionAdmissionInput {
            has_active_run: false,
            runtime_posture: WorkflowSessionAdmissionRuntimePosture::Unloaded,
            usage_profile: Some("interactive".to_string()),
            required_backends: vec!["llama_cpp".to_string()],
            required_models: vec!["model-a".to_string()],
            candidates: vec![admission_candidate(
                "selected",
                2,
                1,
                0,
                0,
                false,
                WorkflowSessionWarmCompatibility::Unknown,
            )],
        };

        let decision = policy
            .admission_decision(&input, "selected")
            .expect("admission decision");

        assert_eq!(
            decision.reason,
            Some(WorkflowSchedulerDecisionReason::ColdStartRequired)
        );
    }
}
