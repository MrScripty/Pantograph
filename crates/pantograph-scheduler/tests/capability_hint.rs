use pantograph_scheduler::{
    SchedulerCapabilityHintSnapshot, SchedulerContractError,
    ValidatedSchedulerCapabilityHintSnapshot, SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION,
};

#[test]
fn valid_capability_fixture_decodes_and_validates() {
    let fixture = include_str!("fixtures/capability_hint_snapshot_valid.json");
    let snapshot: SchedulerCapabilityHintSnapshot =
        serde_json::from_str(fixture).expect("fixture must match capability hint contract");

    let validated = ValidatedSchedulerCapabilityHintSnapshot::try_from(snapshot)
        .expect("fixture must validate before graph/editor consumers use it");

    assert_eq!(
        validated.as_ref().contract_version,
        SCHEDULER_CAPABILITY_HINT_CONTRACT_VERSION
    );
    assert_eq!(validated.as_ref().runtimes.len(), 2);
    assert_eq!(validated.as_ref().trait_options.len(), 1);
}

#[test]
fn rejects_final_scheduler_decision_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "task_type": "image_generation",
        "selected_runtime_id": "diffusers-pytorch"
    });

    let error = serde_json::from_value::<SchedulerCapabilityHintSnapshot>(value)
        .expect_err("capability hints must not expose final dispatch decisions");

    assert!(
        error
            .to_string()
            .contains("unknown field `selected_runtime_id`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_load_target_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "task_type": "image_generation",
        "local_load_path": "/models/juggernaut"
    });

    let error = serde_json::from_value::<SchedulerCapabilityHintSnapshot>(value)
        .expect_err("capability hints must not expose executable load targets");

    assert!(
        error
            .to_string()
            .contains("unknown field `local_load_path`"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_empty_diagnostic_text() {
    let mut snapshot: SchedulerCapabilityHintSnapshot =
        serde_json::from_str(include_str!("fixtures/capability_hint_snapshot_valid.json"))
            .expect("fixture must decode");
    snapshot.runtimes[1].diagnostics[0].message = " ".to_string();

    let error = ValidatedSchedulerCapabilityHintSnapshot::try_from(snapshot)
        .expect_err("empty diagnostic text must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::MissingField {
            field: "capability_diagnostic.message"
        }
    );
}

#[test]
fn rejects_unsupported_capability_contract_version() {
    let mut snapshot: SchedulerCapabilityHintSnapshot =
        serde_json::from_str(include_str!("fixtures/capability_hint_snapshot_valid.json"))
            .expect("fixture must decode");
    snapshot.contract_version = 2;

    let error = ValidatedSchedulerCapabilityHintSnapshot::try_from(snapshot)
        .expect_err("unsupported versions must be rejected");

    assert_eq!(
        error,
        SchedulerContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported scheduler capability hint contract version"
        }
    );
}
