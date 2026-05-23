use pantograph_scheduler::{
    SchedulerContractError, SchedulerQueueTaskState, SchedulerTaskLifecycleDiagnostic,
    SchedulerTaskLifecycleDiagnosticCode, SchedulerTaskLifecycleDiagnosticSeverity,
    SchedulerTaskLifecycleDiagnosticSnapshot, ValidatedSchedulerTaskLifecycleDiagnosticSnapshot,
    SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION,
};

#[test]
fn valid_lifecycle_fixture_decodes_and_validates() {
    let snapshot: SchedulerTaskLifecycleDiagnosticSnapshot = serde_json::from_str(include_str!(
        "fixtures/task_lifecycle_waiting_resources.json"
    ))
    .expect("fixture must match scheduler task lifecycle diagnostic contract");

    let validated = ValidatedSchedulerTaskLifecycleDiagnosticSnapshot::try_from(snapshot)
        .expect("fixture must validate before graph or run inspection consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().state,
        SchedulerQueueTaskState::WaitingResources
    );
}

#[test]
fn rejects_path_and_runtime_host_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "workflow_id": "workflow.image_generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "state": "waiting_resources",
        "model_path": "/models/juggernaut",
        "reservation_id": "reservation.001",
        "diagnostics": [
            {
                "severity": "info",
                "code": "waiting_resources",
                "message": "Task is waiting for scheduler resource admission."
            }
        ]
    });

    let error = serde_json::from_value::<SchedulerTaskLifecycleDiagnosticSnapshot>(value)
        .expect_err("lifecycle diagnostics must reject path and host-internal fields");

    assert!(
        error.to_string().contains("unknown field `model_path`")
            || error.to_string().contains("unknown field `reservation_id`"),
        "unexpected error: {error}"
    );
}

#[test]
fn waiting_states_require_diagnostics() {
    let snapshot = SchedulerTaskLifecycleDiagnosticSnapshot {
        contract_version: SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION,
        workflow_id: "workflow.image_generation"
            .parse()
            .expect("test workflow id must parse"),
        workflow_run_id: "run.001".parse().expect("test run id must parse"),
        node_id: "node.llm_inference"
            .parse()
            .expect("test node id must parse"),
        task_id: "task.001".parse().expect("test task id must parse"),
        state: SchedulerQueueTaskState::WaitingBatch,
        diagnostics: Vec::new(),
    };

    let error = ValidatedSchedulerTaskLifecycleDiagnosticSnapshot::try_from(snapshot)
        .expect_err("waiting lifecycle state must carry scheduler diagnostics");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "task_lifecycle.diagnostics"
        }
    );
}

#[test]
fn completed_state_requires_completed_diagnostic_code() {
    let mut snapshot: SchedulerTaskLifecycleDiagnosticSnapshot = serde_json::from_str(
        include_str!("fixtures/task_lifecycle_waiting_resources.json"),
    )
    .expect("fixture must decode");
    snapshot.state = SchedulerQueueTaskState::Completed;

    let error = ValidatedSchedulerTaskLifecycleDiagnosticSnapshot::try_from(snapshot)
        .expect_err("completed lifecycle state must not carry resource-waiting diagnostics");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "task_lifecycle_diagnostic.code",
            reason: "diagnostic code is not compatible with queue task state"
        }
    );
}

#[test]
fn rejects_empty_lifecycle_diagnostic_text() {
    let mut snapshot: SchedulerTaskLifecycleDiagnosticSnapshot = serde_json::from_str(
        include_str!("fixtures/task_lifecycle_waiting_resources.json"),
    )
    .expect("fixture must decode");
    snapshot.diagnostics[0].message = " ".to_string();

    let error = ValidatedSchedulerTaskLifecycleDiagnosticSnapshot::try_from(snapshot)
        .expect_err("empty lifecycle diagnostic message must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "task_lifecycle_diagnostic.message"
        }
    );
}

#[test]
fn completed_state_accepts_completed_diagnostic_code() {
    let snapshot = SchedulerTaskLifecycleDiagnosticSnapshot {
        contract_version: SCHEDULER_TASK_LIFECYCLE_DIAGNOSTIC_CONTRACT_VERSION,
        workflow_id: "workflow.image_generation"
            .parse()
            .expect("test workflow id must parse"),
        workflow_run_id: "run.001".parse().expect("test run id must parse"),
        node_id: "node.llm_inference"
            .parse()
            .expect("test node id must parse"),
        task_id: "task.001".parse().expect("test task id must parse"),
        state: SchedulerQueueTaskState::Completed,
        diagnostics: vec![SchedulerTaskLifecycleDiagnostic {
            severity: SchedulerTaskLifecycleDiagnosticSeverity::Info,
            code: SchedulerTaskLifecycleDiagnosticCode::TaskCompleted,
            message: "Task completed successfully.".to_string(),
            hint: None,
        }],
    };

    let _validated_snapshot = ValidatedSchedulerTaskLifecycleDiagnosticSnapshot::try_from(snapshot)
        .expect("completed lifecycle state should accept completed diagnostic code");
}
