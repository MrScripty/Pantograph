use pantograph_scheduler::{
    SchedulerContractError, SchedulerDispatchDecision, SchedulerRuntimeHandoff,
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
    assert!(validated.as_ref().dispatch_decision.is_none());
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
fn readiness_admitted_handoff_must_not_carry_dispatch_decision() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.dispatch_decision = Some(valid_dispatch_decision());

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("readiness-admitted handoff must not carry dispatch decision");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "dispatch_decision",
            reason: "readiness-admitted handoff must not carry dispatch decision"
        }
    );
}

#[test]
fn dispatch_selected_handoff_requires_dispatch_decision() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.state = SchedulerRuntimeHandoffState::DispatchSelected;

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("dispatch-selected handoff must carry scheduler dispatch decision");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "dispatch_decision"
        }
    );
}

#[test]
fn dispatch_selected_handoff_accepts_matching_dispatch_decision() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.state = SchedulerRuntimeHandoffState::DispatchSelected;
    handoff.dispatch_decision = Some(matching_dispatch_decision_for(&handoff));

    let validated = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect("matching dispatch decision should validate for runtime host handoff");

    assert_eq!(
        validated.as_ref().state,
        SchedulerRuntimeHandoffState::DispatchSelected
    );
    assert!(validated.as_ref().dispatch_decision.is_some());
}

#[test]
fn dispatch_decision_must_match_handoff_environment_ref() {
    let mut handoff: SchedulerRuntimeHandoff = serde_json::from_str(include_str!(
        "fixtures/runtime_handoff_readiness_admitted.json"
    ))
    .expect("fixture must decode");
    handoff.state = SchedulerRuntimeHandoffState::DispatchSelected;
    let mut decision = matching_dispatch_decision_for(&handoff);
    decision.environment_ref.environment_id =
        "env.other".parse().expect("test environment id must parse");
    handoff.dispatch_decision = Some(decision);

    let error = ValidatedSchedulerRuntimeHandoff::try_from(handoff)
        .expect_err("dispatch decision environment must match runtime handoff");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "environment_ref",
            reason: "dispatch decision environment ref must match readiness proof"
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

fn valid_dispatch_decision() -> SchedulerDispatchDecision {
    serde_json::from_str(include_str!("fixtures/dispatch_decision_valid.json"))
        .expect("dispatch decision fixture must decode")
}

fn matching_dispatch_decision_for(handoff: &SchedulerRuntimeHandoff) -> SchedulerDispatchDecision {
    let mut decision = valid_dispatch_decision();
    decision.workflow_id = handoff.workflow_id.clone();
    decision.workflow_run_id = handoff.workflow_run_id.clone();
    decision.node_id = handoff.node_id.clone();
    decision.task_id = handoff.task_id.clone();
    decision.task_intent = handoff.task_intent.clone();
    decision.selected_model_ref = handoff.task_intent.model_ref.clone();
    decision.readiness_proof = handoff.readiness_proof.clone();
    decision.environment_ref = handoff.environment_ref.clone();
    decision
}
