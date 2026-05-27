use pantograph_dependency_planning::{
    DependencyPlanningContractError, ValidatedDependencyReadinessExecutionContext,
    ValidatedDependencyReadinessProofEnvelope, ValidatedDependencyReadinessRequestEnvelope,
};

const REQUEST_ENVELOPE: &str = include_str!("fixtures/dependency_readiness_request_envelope.json");
const PROOF_ENVELOPE: &str =
    include_str!("fixtures/dependency_readiness_proof_envelope_ready.json");

#[test]
fn dependency_readiness_request_envelope_fixture_decodes_and_validates() {
    let value: serde_json::Value =
        serde_json::from_str(REQUEST_ENVELOPE).expect("request envelope fixture should parse");
    let validated = ValidatedDependencyReadinessRequestEnvelope::try_from(value)
        .expect("request envelope fixture should validate");

    let envelope = validated.as_envelope();
    assert_eq!(envelope.contract_version, 1);
    assert_eq!(
        envelope.execution_context.workflow_id.as_str(),
        "workflow-image"
    );
    assert_eq!(envelope.execution_context.workflow_run_id.as_str(), "run-1");
    assert_eq!(
        envelope.execution_context.descriptor_fingerprint.as_str(),
        "descriptor-tiny-sd-v1"
    );
    assert_eq!(
        envelope.readiness_request.identity_key.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
}

#[test]
fn dependency_readiness_proof_envelope_fixture_matches_request_envelope() {
    let request_value: serde_json::Value =
        serde_json::from_str(REQUEST_ENVELOPE).expect("request envelope fixture should parse");
    let request_envelope = ValidatedDependencyReadinessRequestEnvelope::try_from(request_value)
        .expect("request envelope fixture should validate");

    let proof_value: serde_json::Value =
        serde_json::from_str(PROOF_ENVELOPE).expect("proof envelope fixture should parse");
    let proof_envelope = ValidatedDependencyReadinessProofEnvelope::try_from(proof_value)
        .expect("proof envelope fixture should validate");

    proof_envelope
        .as_envelope()
        .validate_matches_request_envelope(&request_envelope)
        .expect("proof envelope should match request envelope");
}

#[test]
fn dependency_readiness_execution_context_rejects_unknown_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(REQUEST_ENVELOPE).expect("request envelope fixture should parse");
    value["execution_context"]["legacy_mode"] = serde_json::json!("ready");
    let context = value
        .as_object_mut()
        .expect("request envelope root should be object")
        .remove("execution_context")
        .expect("execution context should be present");

    assert_eq!(
        ValidatedDependencyReadinessExecutionContext::try_from(context)
            .expect_err("execution context must reject unknown fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_execution_context",
            reason: "context JSON did not match dependency readiness execution contract"
        }
    );
}

#[test]
fn dependency_readiness_request_envelope_rejects_missing_validation_identity() {
    let mut value: serde_json::Value =
        serde_json::from_str(REQUEST_ENVELOPE).expect("request envelope fixture should parse");
    value["execution_context"]
        .as_object_mut()
        .expect("execution context should be object")
        .remove("validation_session_id");
    value["execution_context"]
        .as_object_mut()
        .expect("execution context should be object")
        .remove("validation_snapshot_id");

    assert_eq!(
        ValidatedDependencyReadinessRequestEnvelope::try_from(value)
            .expect_err("execution context must carry validation freshness identity"),
        DependencyPlanningContractError::MissingField {
            field: "validation_session_id_or_validation_snapshot_id"
        }
    );
}

#[test]
fn dependency_readiness_request_envelope_rejects_caller_context_mismatch() {
    let mut value: serde_json::Value =
        serde_json::from_str(REQUEST_ENVELOPE).expect("request envelope fixture should parse");
    value["execution_context"]["workflow_run_id"] = serde_json::json!("run-2");

    assert_eq!(
        ValidatedDependencyReadinessRequestEnvelope::try_from(value)
            .expect_err("execution context must match readiness caller context"),
        DependencyPlanningContractError::InvalidField {
            field: "execution_context.workflow_run_id",
            reason: "execution context workflow run id must match readiness caller context"
        }
    );
}

#[test]
fn dependency_readiness_proof_envelope_rejects_requirement_drift() {
    let mut value: serde_json::Value =
        serde_json::from_str(PROOF_ENVELOPE).expect("proof envelope fixture should parse");
    value["preflight_result"]["dependency_requirements_id"] =
        serde_json::json!("tiny-sd:pytorch:linux-x86_64:other-binding");

    assert_eq!(
        ValidatedDependencyReadinessProofEnvelope::try_from(value)
            .expect_err("proof envelope must reject requirements id drift"),
        DependencyPlanningContractError::InvalidField {
            field: "execution_context.dependency_requirements_id",
            reason: "execution context requirements id must match preflight result proof"
        }
    );
}

#[test]
fn dependency_readiness_proof_envelope_rejects_zero_proof_version() {
    let mut value: serde_json::Value =
        serde_json::from_str(PROOF_ENVELOPE).expect("proof envelope fixture should parse");
    value["readiness_proof_version"] = serde_json::json!(0);

    assert_eq!(
        ValidatedDependencyReadinessProofEnvelope::try_from(value)
            .expect_err("proof envelope must reject zero proof versions"),
        DependencyPlanningContractError::InvalidField {
            field: "readiness_proof_version",
            reason: "readiness proof version must be greater than zero"
        }
    );
}

#[test]
fn dependency_readiness_proof_envelope_rejects_executable_handoff_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(PROOF_ENVELOPE).expect("proof envelope fixture should parse");
    value["load_target"] = serde_json::json!({
        "local_load_path": "/models/tiny-sd"
    });

    assert_eq!(
        ValidatedDependencyReadinessProofEnvelope::try_from(value)
            .expect_err("proof envelope must reject executable handoff fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_proof_envelope",
            reason: "dependency readiness execution envelopes must not contain path-shaped fields"
        }
    );
}

#[test]
fn dependency_readiness_proof_envelope_rejects_raw_readiness_request_payloads() {
    let mut value: serde_json::Value =
        serde_json::from_str(PROOF_ENVELOPE).expect("proof envelope fixture should parse");
    value["readiness_request"] = serde_json::json!({
        "dependency_override_patches": [
            {
                "binding_id": "torch-diffusers",
                "scope": "binding",
                "fields": {
                    "python_executable": "/tmp/python"
                }
            }
        ]
    });

    assert_eq!(
        ValidatedDependencyReadinessProofEnvelope::try_from(value)
            .expect_err("scheduler proof must not carry raw provider request payloads"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_proof_envelope",
            reason: "dependency readiness execution envelopes must not contain executable handoff fields"
        }
    );
}
