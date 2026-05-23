use pantograph_scheduler::{
    apply_scheduler_queue_transition, SchedulerContractError, SchedulerQueueTaskRecord,
    SchedulerQueueTaskState, SchedulerQueueTransition, SchedulerQueueTransitionApplyResult,
    ValidatedSchedulerQueueTaskRecord, ValidatedSchedulerQueueTransition,
    SCHEDULER_QUEUE_STATE_CONTRACT_VERSION,
};

#[test]
fn valid_queue_transition_fixture_decodes_validates_and_applies() {
    let transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must match scheduler queue transition contract");

    let validated = ValidatedSchedulerQueueTransition::try_from(transition.clone())
        .expect("queue transition fixture must validate");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_QUEUE_STATE_CONTRACT_VERSION
    );

    let result = apply_scheduler_queue_transition(None, transition)
        .expect("initial queue transition must create pending record");
    let SchedulerQueueTransitionApplyResult::Applied(record) = result else {
        panic!("initial transition should be applied");
    };

    assert_eq!(record.state, SchedulerQueueTaskState::Pending);
    assert_eq!(record.state_version, 1);
    let _validated_record = ValidatedSchedulerQueueTaskRecord::try_from(record)
        .expect("applied queue record must validate");
}

#[test]
fn rejects_path_shaped_queue_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "transition_id": "transition.001",
        "workflow_id": "workflow.image_generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "model_path": "/models/juggernaut",
        "task_intent": {
            "contract_version": 1,
            "workflow_id": "workflow.image_generation",
            "workflow_run_id": "run.001",
            "node_id": "node.llm_inference",
            "task_id": "task.001",
            "task_type": "image_generation",
            "model_ref": {
                "model_id": "pumas://models/juggernaut-xl-v10"
            }
        },
        "local_load_path": "/models/juggernaut/model.safetensors",
        "next_state": "pending"
    });

    let error = serde_json::from_value::<SchedulerQueueTransition>(value)
        .expect_err("queue transition must reject path-shaped fields");

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
    let transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    let record = match apply_scheduler_queue_transition(None, transition.clone())
        .expect("initial transition must apply")
    {
        SchedulerQueueTransitionApplyResult::Applied(record) => record,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };

    let result = apply_scheduler_queue_transition(Some(&record), transition)
        .expect("duplicate transition must replay idempotently");

    assert_eq!(
        result,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(record)
    );
}

#[test]
fn duplicate_transition_id_must_match_persisted_task_intent() {
    let transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    let record = match apply_scheduler_queue_transition(None, transition.clone())
        .expect("initial transition must apply")
    {
        SchedulerQueueTransitionApplyResult::Applied(record) => record,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };
    let mut replay = transition;
    replay.task_intent.model_ref.model_id = "pumas://models/other".to_string();

    let error = apply_scheduler_queue_transition(Some(&record), replay)
        .expect_err("duplicate transition id must not hide different task intent");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "task_intent",
            reason: "queue transition task intent must match persisted record"
        }
    );
}

#[test]
fn next_transition_requires_matching_previous_state() {
    let initial: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    let record = match apply_scheduler_queue_transition(None, initial)
        .expect("initial transition must apply")
    {
        SchedulerQueueTransitionApplyResult::Applied(record) => record,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };
    let mut next_transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    next_transition.transition_id = "transition.002"
        .parse()
        .expect("test transition id must parse");
    next_transition.expected_previous_state = Some(SchedulerQueueTaskState::Ready);
    next_transition.next_state = SchedulerQueueTaskState::Running;

    let error = apply_scheduler_queue_transition(Some(&record), next_transition)
        .expect_err("transition must match persisted state");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "queue transition previous state must match persisted task state"
        }
    );
}

#[test]
fn terminal_states_do_not_transition_to_running() {
    let initial: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    let mut record = match apply_scheduler_queue_transition(None, initial)
        .expect("initial transition must apply")
    {
        SchedulerQueueTransitionApplyResult::Applied(record) => record,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };
    record.state = SchedulerQueueTaskState::Completed;
    record.state_version = 4;
    record.last_transition_id = "transition.completed"
        .parse()
        .expect("test transition id must parse");

    let mut next_transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    next_transition.transition_id = "transition.after_completed"
        .parse()
        .expect("test transition id must parse");
    next_transition.expected_previous_state = Some(SchedulerQueueTaskState::Completed);
    next_transition.next_state = SchedulerQueueTaskState::Running;

    let error = apply_scheduler_queue_transition(Some(&record), next_transition)
        .expect_err("completed queue task must be terminal");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "queue transition is not allowed from the previous state"
        }
    );
}

fn queue_record_with_state(state: SchedulerQueueTaskState) -> SchedulerQueueTaskRecord {
    let transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    let mut record = match apply_scheduler_queue_transition(None, transition)
        .expect("initial transition must apply")
    {
        SchedulerQueueTransitionApplyResult::Applied(record) => record,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_) => {
            panic!("first transition cannot be already applied")
        }
    };
    record.state = state;
    record.state_version = 7;
    record.last_transition_id = "transition.existing"
        .parse()
        .expect("test transition id must parse");
    record
}

fn queue_transition_to(
    expected_previous_state: Option<SchedulerQueueTaskState>,
    next_state: SchedulerQueueTaskState,
) -> SchedulerQueueTransition {
    let mut transition: SchedulerQueueTransition =
        serde_json::from_str(include_str!("fixtures/queue_transition_pending.json"))
            .expect("fixture must decode");
    transition.transition_id = "transition.next"
        .parse()
        .expect("test transition id must parse");
    transition.expected_previous_state = expected_previous_state;
    transition.next_state = next_state;
    transition
}

fn all_queue_states() -> &'static [SchedulerQueueTaskState] {
    use SchedulerQueueTaskState::{
        Blocked, Completed, PausedDeferred, Pending, Ready, RetryableFailed, Running,
        TerminalFailed, WaitingBatch, WaitingDependencyReadiness, WaitingResources,
    };

    &[
        Pending,
        Ready,
        Blocked,
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

fn allowed_queue_next_states(
    previous: SchedulerQueueTaskState,
) -> &'static [SchedulerQueueTaskState] {
    use SchedulerQueueTaskState::{
        Blocked, Completed, PausedDeferred, Pending, Ready, RetryableFailed, Running,
        TerminalFailed, WaitingBatch, WaitingDependencyReadiness, WaitingResources,
    };

    match previous {
        Pending => &[Ready, Blocked, WaitingDependencyReadiness, TerminalFailed],
        Ready => &[
            WaitingResources,
            WaitingBatch,
            Running,
            PausedDeferred,
            TerminalFailed,
        ],
        Blocked => &[
            WaitingDependencyReadiness,
            Ready,
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
        RetryableFailed => &[Pending, Ready, WaitingDependencyReadiness, TerminalFailed],
        TerminalFailed | Completed => &[],
        _ => panic!("test matrix must be updated for new queue state: {previous:?}"),
    }
}

#[test]
fn queue_initial_transition_only_creates_pending_state() {
    let applied = apply_scheduler_queue_transition(
        None,
        queue_transition_to(None, SchedulerQueueTaskState::Pending),
    )
    .expect("initial pending transition");

    let SchedulerQueueTransitionApplyResult::Applied(record) = applied else {
        panic!("initial transition must apply");
    };
    assert_eq!(record.state, SchedulerQueueTaskState::Pending);
    assert_eq!(record.state_version, 1);

    let error = apply_scheduler_queue_transition(
        None,
        queue_transition_to(None, SchedulerQueueTaskState::Ready),
    )
    .expect_err("initial non-pending transition must fail");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "next_state",
            reason: "initial queue transition must create pending state"
        }
    );
}

#[test]
fn queue_contract_allows_only_declared_task_state_transitions() {
    for previous in all_queue_states() {
        for next in all_queue_states() {
            let current = queue_record_with_state(*previous);
            let applied = apply_scheduler_queue_transition(
                Some(&current),
                queue_transition_to(Some(*previous), *next),
            );
            let allowed = allowed_queue_next_states(*previous).contains(next);
            assert_eq!(
                applied.is_ok(),
                allowed,
                "transition from {previous:?} to {next:?} should be allowed={allowed}"
            );
            if let Ok(SchedulerQueueTransitionApplyResult::Applied(record)) = applied {
                assert_eq!(record.state, *next);
                assert_eq!(record.state_version, current.state_version + 1);
            }
        }
    }
}

#[test]
fn queue_transition_replay_is_idempotent_for_matching_transition_id() {
    let current = queue_record_with_state(SchedulerQueueTaskState::Ready);
    let replay = apply_scheduler_queue_transition(
        Some(&current),
        SchedulerQueueTransition {
            transition_id: current.last_transition_id.clone(),
            next_state: current.state,
            expected_previous_state: Some(SchedulerQueueTaskState::Pending),
            ..queue_transition_to(
                Some(SchedulerQueueTaskState::Pending),
                SchedulerQueueTaskState::Ready,
            )
        },
    )
    .expect("matching transition id replay");

    assert!(matches!(
        replay,
        SchedulerQueueTransitionApplyResult::AlreadyApplied(_)
    ));
}

#[test]
fn queue_transition_rejects_stale_expected_previous_state() {
    let current = queue_record_with_state(SchedulerQueueTaskState::Ready);
    let error = apply_scheduler_queue_transition(
        Some(&current),
        queue_transition_to(
            Some(SchedulerQueueTaskState::Pending),
            SchedulerQueueTaskState::TerminalFailed,
        ),
    )
    .expect_err("stale expected state must fail");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "expected_previous_state",
            reason: "queue transition previous state must match persisted task state"
        }
    );
}
