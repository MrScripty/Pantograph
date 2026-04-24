use crate::workflow::{WorkflowOutputTarget, WorkflowPortBinding};

use super::*;

fn queued_run(
    queue_id: &str,
    priority: i32,
    enqueued_tick: u64,
    starvation_bypass_count: u32,
) -> WorkflowExecutionSessionQueuedRun {
    WorkflowExecutionSessionQueuedRun {
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
) -> WorkflowExecutionSessionRuntimeSelectionTarget {
    WorkflowExecutionSessionRuntimeSelectionTarget {
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
) -> WorkflowExecutionSessionRuntimeUnloadCandidate {
    WorkflowExecutionSessionRuntimeUnloadCandidate {
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
    warm_session_compatibility: WorkflowExecutionSessionWarmCompatibility,
) -> WorkflowExecutionSessionAdmissionCandidate {
    WorkflowExecutionSessionAdmissionCandidate {
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
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
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
                WorkflowExecutionSessionWarmCompatibility::Compatible,
            ),
            admission_candidate(
                "higher-priority",
                2,
                2,
                0,
                1,
                false,
                WorkflowExecutionSessionWarmCompatibility::Incompatible,
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
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
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
                WorkflowExecutionSessionWarmCompatibility::Compatible,
            ),
            admission_candidate(
                "pending",
                0,
                2,
                0,
                1,
                false,
                WorkflowExecutionSessionWarmCompatibility::Incompatible,
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
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
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
            WorkflowExecutionSessionWarmCompatibility::Compatible,
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
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Unloaded,
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
            WorkflowExecutionSessionWarmCompatibility::Unknown,
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

#[test]
fn admission_decision_prefers_warm_reuse_within_bounded_fairness_window() {
    let policy = PriorityThenFifoSchedulerPolicy;
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
        usage_profile: Some("interactive".to_string()),
        required_backends: vec!["llama_cpp".to_string()],
        required_models: vec!["model-a".to_string()],
        candidates: vec![
            admission_candidate(
                "head-cold",
                1,
                1,
                0,
                0,
                false,
                WorkflowExecutionSessionWarmCompatibility::Incompatible,
            ),
            admission_candidate(
                "next-warm",
                1,
                2,
                0,
                1,
                true,
                WorkflowExecutionSessionWarmCompatibility::Compatible,
            ),
        ],
    };

    let decision = policy
        .admission_decision(&input, "next-warm")
        .expect("admission decision");

    assert_eq!(decision.admitted_queue_id.as_deref(), Some("next-warm"));
    assert_eq!(
        decision.reason,
        Some(WorkflowSchedulerDecisionReason::WarmSessionReused)
    );
}

#[test]
fn admission_decision_preserves_starved_head_over_warm_reuse_candidate() {
    let policy = PriorityThenFifoSchedulerPolicy;
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
        usage_profile: Some("interactive".to_string()),
        required_backends: vec!["llama_cpp".to_string()],
        required_models: vec!["model-a".to_string()],
        candidates: vec![
            admission_candidate(
                "starved-head",
                0,
                1,
                4,
                0,
                false,
                WorkflowExecutionSessionWarmCompatibility::Incompatible,
            ),
            admission_candidate(
                "warm-follower",
                2,
                2,
                0,
                1,
                true,
                WorkflowExecutionSessionWarmCompatibility::Compatible,
            ),
        ],
    };

    let decision = policy
        .admission_decision(&input, "warm-follower")
        .expect("pending decision");

    assert_eq!(decision.admitted_queue_id, None);
    assert_eq!(decision.reason, None);
}

#[test]
fn admission_decision_preserves_fifo_when_warm_reuse_candidate_is_outside_window() {
    let policy = PriorityThenFifoSchedulerPolicy;
    let input = WorkflowExecutionSessionAdmissionInput {
        has_active_run: false,
        runtime_posture: WorkflowExecutionSessionAdmissionRuntimePosture::Loaded,
        usage_profile: Some("interactive".to_string()),
        required_backends: vec!["llama_cpp".to_string()],
        required_models: vec!["model-a".to_string()],
        candidates: vec![
            admission_candidate(
                "head-cold",
                1,
                1,
                0,
                0,
                false,
                WorkflowExecutionSessionWarmCompatibility::Incompatible,
            ),
            admission_candidate(
                "middle-cold",
                1,
                2,
                0,
                1,
                false,
                WorkflowExecutionSessionWarmCompatibility::Unknown,
            ),
            admission_candidate(
                "far-warm",
                1,
                3,
                0,
                2,
                true,
                WorkflowExecutionSessionWarmCompatibility::Compatible,
            ),
        ],
    };

    let decision = policy
        .admission_decision(&input, "far-warm")
        .expect("pending decision");

    assert_eq!(decision.admitted_queue_id, None);
    assert_eq!(decision.reason, None);
}
