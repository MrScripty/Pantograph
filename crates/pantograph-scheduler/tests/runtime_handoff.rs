use pantograph_scheduler::{
    SchedulerContractError, SchedulerRuntimeHandoff, SchedulerRuntimeHandoffSelection,
    SchedulerRuntimeHandoffState, ValidatedSchedulerRuntimeHandoff,
    SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
};

#[test]
fn valid_runtime_handoff_fixture_decodes_and_validates() {
    let fixture = include_str!("fixtures/runtime_handoff_readiness_admitted.json");
    let handoff: SchedulerRuntimeHandoff =
        serde_json::from_str(fixture).expect("fixture must match runtime handoff contract");

    let validated = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect("fixture must validate before runtime host consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().state,
        SchedulerRuntimeHandoffState::ReadinessAdmitted
    );
    assert!(validated.as_ref().dispatch_selection.is_none());
}

#[test]
fn rejects_path_and_load_target_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
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
        "state": "readiness_admitted",
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
        "local_load_path": "/models/juggernaut/model.safetensors"
    });

    let error = serde_json::from_value::<SchedulerRuntimeHandoff>(value)
        .expect_err("runtime handoff must reject path and load-target fields");

    assert!(
        error.to_string().contains("unknown field `model_path`")
            || error
                .to_string()
                .contains("unknown field `local_load_path`"),
        "unexpected error: {error}"
    );
}

#[test]
fn correlation_must_match_task_intent() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.workflow_id = "workflow.other"
        .parse()
        .expect("test workflow id must parse");

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("runtime handoff correlation must match task intent");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "workflow_id",
            reason: "runtime handoff workflow id must match task intent"
        }
    );
}

#[test]
fn environment_ref_must_match_readiness_proof() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.environment_ref.environment_id =
        "env.other".parse().expect("test environment id must parse");

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("runtime handoff environment must match proof");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "environment_ref",
            reason: "runtime handoff environment ref must match readiness proof"
        }
    );
}

#[test]
fn readiness_admitted_handoff_must_not_carry_dispatch_selection() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.dispatch_selection = Some(SchedulerRuntimeHandoffSelection {
        selected_runtime_id: "diffusers-pytorch"
            .parse()
            .expect("test runtime id must parse"),
        selected_device_id: Some("cuda:0".parse().expect("test device id must parse")),
    });

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("readiness-admitted handoff must not carry dispatch selection");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "dispatch_selection",
            reason: "readiness-admitted handoff must not carry dispatch selection"
        }
    );
}

#[test]
fn dispatch_selected_handoff_requires_selection() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.state = SchedulerRuntimeHandoffState::DispatchSelected;

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("dispatch-selected handoff must carry scheduler selection");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "dispatch_selection"
        }
    );
}

#[test]
fn dispatch_selection_must_satisfy_hard_runtime_requirement() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.state = SchedulerRuntimeHandoffState::DispatchSelected;
    handoff.dispatch_selection = Some(SchedulerRuntimeHandoffSelection {
        selected_runtime_id: "other-runtime".parse().expect("test runtime id must parse"),
        selected_device_id: Some("cuda:0".parse().expect("test device id must parse")),
    });

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("dispatch selection must satisfy hard runtime requirements");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "dispatch_selection.selected_runtime_id",
            reason: "selected runtime must satisfy the task intent runtime requirement"
        }
    );
}

#[test]
fn rejects_unsupported_runtime_handoff_contract_version() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.contract_version = 2;

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("unsupported versions must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler runtime handoff contract version"
        }
    );
}
