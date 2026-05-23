use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerContractError, ValidatedSchedulableTaskIntent,
    SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
};

#[test]
fn valid_fixture_decodes_and_validates() {
    let fixture = include_str!("fixtures/schedulable_task_intent_valid.json");
    let intent: SchedulableTaskIntent =
        serde_json::from_str(fixture).expect("fixture must match scheduler task intent contract");

    let validated = ValidatedSchedulableTaskIntent::try_from(intent)
        .expect("fixture must validate before scheduler policy consumes it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION
    );
    assert_eq!(
        validated.as_ref().model_ref.model_id,
        "pumas://models/juggernaut-xl-v10"
    );
    assert_eq!(validated.as_ref().estimate_hints.len(), 2);
}

#[test]
fn rejects_path_shaped_top_level_identity_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "workflow_id": "workflow.image_generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "task_type": "image_generation",
        "model_path": "/models/juggernaut",
        "model_ref": {
            "model_id": "pumas://models/juggernaut-xl-v10"
        }
    });

    let error = serde_json::from_value::<SchedulableTaskIntent>(value)
        .expect_err("model_path must not be accepted by scheduler task intent");

    assert!(
        error.to_string().contains("unknown field `model_path`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_invalid_identifiers_before_scheduler_policy() {
    let value = serde_json::json!({
        "contract_version": 1,
        "workflow_id": "workflow/image-generation",
        "workflow_run_id": "run.001",
        "node_id": "node.llm_inference",
        "task_id": "task.001",
        "task_type": "image_generation",
        "model_ref": {
            "model_id": "pumas://models/juggernaut-xl-v10"
        }
    });

    let error = serde_json::from_value::<SchedulableTaskIntent>(value)
        .expect_err("invalid workflow id must fail at the boundary");

    assert!(
        error.to_string().contains("workflow_id"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_zero_estimate_values() {
    let mut intent: SchedulableTaskIntent =
        serde_json::from_str(include_str!("fixtures/schedulable_task_intent_valid.json"))
            .expect("fixture must decode");
    intent.estimate_hints[0].value = 0;

    let error = ValidatedSchedulableTaskIntent::try_from(intent)
        .expect_err("zero estimate values must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "estimate_hint.value",
            reason: "estimate values must be greater than zero"
        }
    );
}

#[test]
fn rejects_unsupported_contract_version() {
    let mut intent: SchedulableTaskIntent =
        serde_json::from_str(include_str!("fixtures/schedulable_task_intent_valid.json"))
            .expect("fixture must decode");
    intent.contract_version = 2;

    let error = ValidatedSchedulableTaskIntent::try_from(intent)
        .expect_err("unsupported versions must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported schedulable task intent contract version"
        }
    );
}
