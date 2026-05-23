use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef};
use pantograph_scheduler::{
    apply_scheduler_task_state_transition, SchedulableTaskIntent, SchedulerContractError,
    SchedulerNodeId, SchedulerRuntimeDeviceConstraints, SchedulerTaskId, SchedulerTaskState,
    SchedulerTaskStateDiagnostic, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionApplyResult,
    SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
    ValidatedSchedulerTaskStateRecord, ValidatedSchedulerTaskStateTransition,
    SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};

#[test]
fn valid_task_state_transition_fixture_decodes_validates_and_applies() {
    let transition: SchedulerTaskStateTransition =
        serde_json::from_str(include_str!("fixtures/task_state_transition_ready.json"))
            .expect("fixture must match scheduler task-state transition contract");

    let validated = ValidatedSchedulerTaskStateTransition::try_from(transition.clone())
        .expect("task-state transition fixture must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION
    );

    let result = apply_scheduler_task_state_transition(None, transition)
        .expect("initial ready transition must create task-state record");
    let SchedulerTaskStateTransitionApplyResult::Applied(record) = result else {
        panic!("initial transition should be applied");
    };

    assert_eq!(record.state.kind(), SchedulerTaskStateKind::Ready);
    assert_eq!(record.state_version, 1);
    let _validated_record = ValidatedSchedulerTaskStateRecord::try_from(record)
        .expect("applied task-state record must validate");
}

#[test]
fn pre_intent_task_state_does_not_require_task_intent() {
    let transition = task_transition_to(None, awaiting_inputs_state());
    let result = apply_scheduler_task_state_transition(None, transition)
        .expect("awaiting inputs is a valid pre-intent initial state");

    let SchedulerTaskStateTransitionApplyResult::Applied(record) = result else {
        panic!("initial awaiting-inputs transition should apply");
    };
    assert_eq!(record.state.kind(), SchedulerTaskStateKind::AwaitingInputs);
    assert!(record.state.task_intent().is_none());
}

#[test]
fn rejects_path_shaped_task_state_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "transition_id": "transition.001",
        "workflow_id": "workflow.image_generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "model_path": "/models/juggernaut",
        "local_load_path": "/models/juggernaut/model.safetensors",
        "next_state": {
            "kind": "awaiting_inputs"
        }
    });

    let error = serde_json::from_value::<SchedulerTaskStateTransition>(value)
        .expect_err("task-state transition must reject path-shaped fields");

    assert!(
        error.to_string().contains("unknown field `model_path`")
            || error
                .to_string()
                .contains("unknown field `local_load_path`"),
        "unexpected error: {error}"
    );
}

#[test]
fn duplicate_transition_is_idempotent() {
    let transition: SchedulerTaskStateTransition =
        serde_json::from_str(include_str!("fixtures/task_state_transition_ready.json"))
            .expect("fixture must decode");
    let record = match apply_scheduler_task_state_transition(None, transition.clone())
        .expect("initial transition must apply")
    {
        SchedulerTaskStateTransitionApplyResult::Applied(record) => record,
        SchedulerTaskStateTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };

    let result = apply_scheduler_task_state_transition(Some(&record), transition)
        .expect("duplicate transition must replay idempotently");

    assert_eq!(
        result,
        SchedulerTaskStateTransitionApplyResult::AlreadyApplied(record)
    );
}

#[test]
fn duplicate_transition_id_must_match_persisted_next_state() {
    let transition: SchedulerTaskStateTransition =
        serde_json::from_str(include_str!("fixtures/task_state_transition_ready.json"))
            .expect("fixture must decode");
    let record = match apply_scheduler_task_state_transition(None, transition.clone())
        .expect("initial transition must apply")
    {
        SchedulerTaskStateTransitionApplyResult::Applied(record) => record,
        SchedulerTaskStateTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };
    let mut replay = transition;
    replay.next_state = awaiting_inputs_state();

    let error = apply_scheduler_task_state_transition(Some(&record), replay)
        .expect_err("duplicate transition id must not hide different next state");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "transition_id",
            reason: "duplicate task-state transition id must replay the same next state"
        }
    );
}

#[test]
fn next_transition_requires_matching_previous_state() {
    let record = task_record_with_state(awaiting_inputs_state());
    let next_transition = task_transition_to(
        Some(SchedulerTaskStateKind::Ready),
        running_state(task_intent("run.001", "task.001")),
    );

    let error = apply_scheduler_task_state_transition(Some(&record), next_transition)
        .expect_err("transition must match persisted state");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "task-state transition previous state must match persisted task state"
        }
    );
}

#[test]
fn terminal_states_do_not_transition_to_running() {
    let current = task_record_with_state(completed_state(task_intent("run.001", "task.001")));
    let next_transition = task_transition_to(
        Some(SchedulerTaskStateKind::Completed),
        running_state(task_intent("run.001", "task.001")),
    );

    let error = apply_scheduler_task_state_transition(Some(&current), next_transition)
        .expect_err("completed task state must be terminal");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "task-state transition is not allowed from the previous state"
        }
    );
}

#[test]
fn task_state_contract_allows_only_declared_transitions() {
    for previous in all_task_state_kinds() {
        for next in all_task_state_kinds() {
            let current = task_record_with_state(state_for_kind(*previous));
            let applied = apply_scheduler_task_state_transition(
                Some(&current),
                task_transition_to(Some(*previous), state_for_kind(*next)),
            );
            let allowed = allowed_next_states(*previous).contains(next);
            assert_eq!(
                applied.is_ok(),
                allowed,
                "transition from {previous:?} to {next:?} should be allowed={allowed}"
            );
            if let Ok(SchedulerTaskStateTransitionApplyResult::Applied(record)) = applied {
                assert_eq!(record.state.kind(), *next);
                assert_eq!(record.state_version, current.state_version + 1);
            }
        }
    }
}

#[test]
fn task_state_transition_replay_is_idempotent_for_matching_transition_id() {
    let current = task_record_with_state(ready_state(task_intent("run.001", "task.001")));
    let replay = apply_scheduler_task_state_transition(
        Some(&current),
        SchedulerTaskStateTransition {
            transition_id: current.last_transition_id.clone(),
            next_state: current.state.clone(),
            expected_previous_state: Some(SchedulerTaskStateKind::AwaitingInputs),
            ..task_transition_to(
                Some(SchedulerTaskStateKind::AwaitingInputs),
                ready_state(task_intent("run.001", "task.001")),
            )
        },
    )
    .expect("matching transition id replay");

    assert!(matches!(
        replay,
        SchedulerTaskStateTransitionApplyResult::AlreadyApplied(_)
    ));
}

#[test]
fn task_state_transition_rejects_stale_expected_previous_state() {
    let current = task_record_with_state(ready_state(task_intent("run.001", "task.001")));
    let error = apply_scheduler_task_state_transition(
        Some(&current),
        task_transition_to(
            Some(SchedulerTaskStateKind::AwaitingInputs),
            terminal_failed_state(),
        ),
    )
    .expect_err("stale expected state must fail");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "task-state transition previous state must match persisted task state"
        }
    );
}

fn task_record_with_state(state: SchedulerTaskState) -> SchedulerTaskStateRecord {
    SchedulerTaskStateRecord {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow.image_generation").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run.001").expect("run id"),
        node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
        task_id: SchedulerTaskId::parse("task.001").expect("task id"),
        state,
        state_version: 7,
        last_transition_id: SchedulerTaskStateTransitionId::parse("transition.existing")
            .expect("test transition id must parse"),
    }
}

fn task_transition_to(
    expected_previous_state: Option<SchedulerTaskStateKind>,
    next_state: SchedulerTaskState,
) -> SchedulerTaskStateTransition {
    SchedulerTaskStateTransition {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        transition_id: SchedulerTaskStateTransitionId::parse("transition.next")
            .expect("test transition id must parse"),
        workflow_id: SchedulerWorkflowId::parse("workflow.image_generation").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run.001").expect("run id"),
        node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
        task_id: SchedulerTaskId::parse("task.001").expect("task id"),
        expected_previous_state,
        next_state,
    }
}

fn all_task_state_kinds() -> &'static [SchedulerTaskStateKind] {
    use SchedulerTaskStateKind::{
        AwaitingInputs, Completed, InputUnavailable, Invalid, PausedDeferred, Ready,
        RetryableFailed, Running, TerminalFailed, WaitingBatch, WaitingDependencyReadiness,
        WaitingResources,
    };

    &[
        AwaitingInputs,
        InputUnavailable,
        Invalid,
        Ready,
        WaitingDependencyReadiness,
        WaitingResources,
        WaitingBatch,
        Running,
        PausedDeferred,
        RetryableFailed,
        TerminalFailed,
        Completed,
    ]
}

fn allowed_next_states(previous: SchedulerTaskStateKind) -> &'static [SchedulerTaskStateKind] {
    use SchedulerTaskStateKind::{
        AwaitingInputs, Completed, InputUnavailable, Invalid, PausedDeferred, Ready,
        RetryableFailed, Running, TerminalFailed, WaitingBatch, WaitingDependencyReadiness,
        WaitingResources,
    };

    match previous {
        AwaitingInputs => &[Ready, InputUnavailable, Invalid, TerminalFailed],
        InputUnavailable => &[AwaitingInputs, TerminalFailed],
        Invalid => &[TerminalFailed],
        Ready => &[
            WaitingDependencyReadiness,
            WaitingResources,
            WaitingBatch,
            Running,
            PausedDeferred,
            TerminalFailed,
        ],
        WaitingDependencyReadiness => &[Ready, PausedDeferred, RetryableFailed, TerminalFailed],
        WaitingResources => &[
            Ready,
            WaitingBatch,
            Running,
            PausedDeferred,
            RetryableFailed,
            TerminalFailed,
        ],
        WaitingBatch => &[
            Ready,
            Running,
            PausedDeferred,
            RetryableFailed,
            TerminalFailed,
        ],
        Running => &[
            Completed,
            RetryableFailed,
            TerminalFailed,
            PausedDeferred,
            WaitingResources,
        ],
        PausedDeferred => &[
            Ready,
            WaitingDependencyReadiness,
            WaitingResources,
            WaitingBatch,
            TerminalFailed,
        ],
        RetryableFailed => &[Ready, WaitingDependencyReadiness, TerminalFailed],
        TerminalFailed | Completed => &[],
        _ => panic!("test matrix must be updated for new task state: {previous:?}"),
    }
}

fn state_for_kind(kind: SchedulerTaskStateKind) -> SchedulerTaskState {
    let intent = task_intent("run.001", "task.001");
    match kind {
        SchedulerTaskStateKind::AwaitingInputs => awaiting_inputs_state(),
        SchedulerTaskStateKind::InputUnavailable => SchedulerTaskState::InputUnavailable {
            diagnostics: diagnostics(),
        },
        SchedulerTaskStateKind::Invalid => SchedulerTaskState::Invalid {
            diagnostics: diagnostics(),
        },
        SchedulerTaskStateKind::Ready => ready_state(intent),
        SchedulerTaskStateKind::WaitingDependencyReadiness => {
            SchedulerTaskState::WaitingDependencyReadiness {
                task_intent: intent,
            }
        }
        SchedulerTaskStateKind::WaitingResources => SchedulerTaskState::WaitingResources {
            task_intent: intent,
        },
        SchedulerTaskStateKind::WaitingBatch => SchedulerTaskState::WaitingBatch {
            task_intent: intent,
        },
        SchedulerTaskStateKind::Running => running_state(intent),
        SchedulerTaskStateKind::PausedDeferred => SchedulerTaskState::PausedDeferred {
            task_intent: intent,
            diagnostics: diagnostics(),
        },
        SchedulerTaskStateKind::RetryableFailed => SchedulerTaskState::RetryableFailed {
            task_intent: intent,
            diagnostics: diagnostics(),
        },
        SchedulerTaskStateKind::TerminalFailed => terminal_failed_state(),
        SchedulerTaskStateKind::Completed => completed_state(intent),
        _ => panic!("test helper must be updated for new task state: {kind:?}"),
    }
}

fn awaiting_inputs_state() -> SchedulerTaskState {
    SchedulerTaskState::AwaitingInputs {
        diagnostics: Vec::new(),
    }
}

fn ready_state(task_intent: SchedulableTaskIntent) -> SchedulerTaskState {
    SchedulerTaskState::Ready { task_intent }
}

fn running_state(task_intent: SchedulableTaskIntent) -> SchedulerTaskState {
    SchedulerTaskState::Running { task_intent }
}

fn completed_state(task_intent: SchedulableTaskIntent) -> SchedulerTaskState {
    SchedulerTaskState::Completed { task_intent }
}

fn terminal_failed_state() -> SchedulerTaskState {
    SchedulerTaskState::TerminalFailed {
        diagnostics: diagnostics(),
    }
}

fn diagnostics() -> Vec<SchedulerTaskStateDiagnostic> {
    vec![SchedulerTaskStateDiagnostic {
        severity: SchedulerTaskStateDiagnosticSeverity::Error,
        code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
        message: "test scheduler diagnostic".to_string(),
        hint: None,
    }]
}

fn task_intent(workflow_run_id: &str, task_id: &str) -> SchedulableTaskIntent {
    SchedulableTaskIntent {
        contract_version: SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
        workflow_id: SchedulerWorkflowId::parse("workflow.image_generation").expect("workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse(workflow_run_id).expect("run id"),
        node_id: SchedulerNodeId::parse("node.llm_inference").expect("node id"),
        task_id: SchedulerTaskId::parse(task_id).expect("task id"),
        fairness_key: Some("user.local".parse().expect("fairness key")),
        task_type: DependencyTaskId::parse("image_generation").expect("task type"),
        model_ref: PumasModelRef {
            model_id: "pumas://models/juggernaut-xl-v10".to_string(),
            revision: None,
            selected_artifact_id: Some("artifact.diffusers.bundle".to_string()),
            selected_artifact_path: Some(
                "pumas://artifacts/juggernaut-xl-v10/diffusers".to_string(),
            ),
            migration_diagnostics: Vec::new(),
        },
        constraints: SchedulerRuntimeDeviceConstraints {
            requested_runtime_id: Some("diffusers-pytorch".parse().expect("runtime id")),
            requested_device_id: Some("cuda:0".parse().expect("device id")),
        },
        trait_settings: Vec::new(),
        dependency_override_patches: Vec::new(),
        estimate_hints: Vec::new(),
    }
}
