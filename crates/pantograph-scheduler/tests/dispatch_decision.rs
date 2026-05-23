use pantograph_scheduler::{
    SchedulerContractError, SchedulerDispatchDecision, SchedulerDispatchDiagnostic,
    SchedulerDispatchDiagnosticCode, SchedulerDispatchDiagnosticSeverity,
    ValidatedSchedulerDispatchDecision, SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION,
};

#[test]
fn valid_dispatch_decision_fixture_decodes_and_validates() {
    let decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must match scheduler dispatch decision contract");

    let validated = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect("fixture must validate before runtime host handoff consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_DISPATCH_DECISION_CONTRACT_VERSION
    );
    assert_eq!(validated.as_ref().selected_device_ids.len(), 1);
}

#[test]
fn rejects_executable_path_and_worker_launch_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "workflow_id": "workflow.image_generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "local_load_path": "/models/juggernaut/model.safetensors",
        "worker_command": "python worker.py",
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
        "selected_runtime_id": "diffusers-pytorch",
        "selected_device_ids": ["cuda:0"],
        "selected_model_ref": {
            "model_id": "pumas://models/juggernaut-xl-v10"
        },
        "readiness_proof": {
            "preflight_result": {
                "contract_version": 1,
                "identity_key": {
                    "model_ref": {
                        "model_id": "pumas://models/juggernaut-xl-v10"
                    },
                    "task_id": "image_generation"
                },
                "readiness_state": "ready",
                "dependency_requirements_id": "deps.image_generation",
                "environment_ref": {
                    "environment_id": "env.image_generation"
                }
            }
        },
        "environment_ref": {
            "environment_id": "env.image_generation"
        },
        "reservation_lease_id": "reservation.001"
    });

    let error = serde_json::from_value::<SchedulerDispatchDecision>(value)
        .expect_err("dispatch decisions must reject executable paths and worker launch fields");

    assert!(
        error
            .to_string()
            .contains("unknown field `local_load_path`")
            || error.to_string().contains("unknown field `worker_command`"),
        "unexpected error: {error}"
    );
}

#[test]
fn selected_runtime_must_satisfy_hard_requirement() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.selected_runtime_id = "other-runtime".parse().expect("test runtime id must parse");

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("selected runtime must satisfy hard runtime requirement");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_runtime_id",
            reason: "selected runtime must satisfy the task intent runtime requirement"
        }
    );
}

#[test]
fn selected_devices_must_satisfy_hard_requirement() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.selected_device_ids = vec!["cuda:1".parse().expect("test device id must parse")];

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("selected devices must satisfy hard device requirement");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_device_ids",
            reason: "selected devices must satisfy the task intent device requirement"
        }
    );
}

#[test]
fn selected_devices_must_not_be_empty_or_duplicate() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.selected_device_ids.clear();

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("dispatch decision must select at least one device");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "selected_device_ids"
        }
    );

    let mut duplicate: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    duplicate
        .selected_device_ids
        .push(duplicate.selected_device_ids[0].clone());

    let error = ValidatedSchedulerDispatchDecision::try_from(duplicate)
        .expect_err("dispatch decision must not duplicate device ids");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_device_ids",
            reason: "selected device ids must not contain duplicates"
        }
    );
}

#[test]
fn selected_model_must_match_task_model_requirement() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.selected_model_ref.model_id = "pumas://models/other".to_string();

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("selected model must match task model requirement");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "selected_model_ref.model_id",
            reason: "selected model id must match task intent model id"
        }
    );
}

#[test]
fn environment_ref_must_match_readiness_proof() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.environment_ref.environment_id =
        "env.other".parse().expect("test environment id must parse");

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("dispatch environment must match readiness proof");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "environment_ref",
            reason: "dispatch decision environment ref must match readiness proof"
        }
    );
}

#[test]
fn rejects_empty_dispatch_diagnostic_text() {
    let mut decision: SchedulerDispatchDecision =
        serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
            .expect("fixture must decode");
    decision.diagnostics.push(SchedulerDispatchDiagnostic {
        severity: SchedulerDispatchDiagnosticSeverity::Info,
        code: SchedulerDispatchDiagnosticCode::SchedulerPolicyTrace,
        message: " ".to_string(),
        hint: None,
    });

    let error = ValidatedSchedulerDispatchDecision::try_from(decision)
        .expect_err("dispatch diagnostics must be bounded non-empty text");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "dispatch_diagnostic.message"
        }
    );
}
