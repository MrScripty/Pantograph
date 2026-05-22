use pantograph_dependency_planning::{
    DependencyBindingId, DependencyPlanningContractError, DependencyPlanningIdentityKey,
    DependencyPlanningRequest, DependencyReadinessPolicy, ValidatedDependencyReadinessRequest,
};

const READINESS_REQUEST: &str = include_str!("fixtures/dependency_readiness_request.json");

#[test]
fn dependency_readiness_request_fixture_decodes_and_validates() {
    let value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");
    let validated = ValidatedDependencyReadinessRequest::try_from(value)
        .expect("readiness request fixture should validate");

    assert_eq!(validated.as_request().contract_version, 1);
    assert_eq!(
        validated.as_request().identity_key.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        validated.as_request().policy,
        DependencyReadinessPolicy::AutoInstallMissing
    );
}

#[test]
fn dependency_readiness_request_validation_does_not_require_proof_fields() {
    let value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");

    let _validated = ValidatedDependencyReadinessRequest::try_from(value)
        .expect("readiness request input should not require environment proof fields");
}

#[test]
fn dependency_planning_identity_key_constructor_matches_readiness_fixture() {
    let request: pantograph_dependency_planning::DependencyReadinessRequest =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should decode");
    let identity_key =
        DependencyPlanningIdentityKey::from_planning_request(&request.planning_request)
            .expect("path-free planning request should produce identity key");

    assert_eq!(identity_key, request.identity_key);
}

#[test]
fn dependency_planning_identity_key_constructor_rejects_selected_artifact_path() {
    let mut request: DependencyPlanningRequest = serde_json::from_value(serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd",
            "selected_artifact_path": "image/stable-diffusion/tiny-sd/model_index.json"
        },
        "task_id": "image_generation"
    }))
    .expect("planning request shape should decode before path-free identity validation");

    request.selected_binding_ids.clear();

    assert_eq!(
        DependencyPlanningIdentityKey::from_planning_request(&request)
            .expect_err("identity keys must stay path-free"),
        DependencyPlanningContractError::InvalidField {
            field: "pumas_model_ref.selected_artifact_path",
            reason: "path-free dependency identity must not carry selected artifact paths"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_unknown_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");
    value["legacy_mode"] = serde_json::json!("install");

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(value)
            .expect_err("readiness request must reject unknown fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_request",
            reason: "request JSON did not match dependency readiness contract"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_path_shaped_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");
    value["planning_request"]["model_ref"]["selected_artifact_path"] =
        serde_json::json!("image/stable-diffusion/tiny-sd/model_index.json");

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(value)
            .expect_err("readiness request must reject path-shaped fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness",
            reason: "readiness payload must not contain path-shaped dependency identity fields"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_executable_handoff_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");
    value["model_package_facts"] = serde_json::json!({
        "package_facts_contract_version": 1,
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        }
    });

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(value)
            .expect_err("readiness request must reject executable handoff fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness",
            reason: "readiness payload must not contain executable dependency handoff fields"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_identity_request_mismatch() {
    let mut request: pantograph_dependency_planning::DependencyReadinessRequest =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should decode");
    request.planning_request.task_id = "audio_transcription"
        .parse()
        .expect("test task id is valid");

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(request)
            .expect_err("identity key and planning request must match"),
        DependencyPlanningContractError::InvalidField {
            field: "identity_key.task_id",
            reason: "identity key task id must match planning request task id"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_duplicate_selected_binding_ids() {
    let mut request: pantograph_dependency_planning::DependencyReadinessRequest =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should decode");
    let duplicate: DependencyBindingId =
        "torch-diffusers".parse().expect("test binding id is valid");
    request
        .identity_key
        .selected_binding_ids
        .push(duplicate.clone());
    request
        .planning_request
        .selected_binding_ids
        .push(duplicate);

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(request)
            .expect_err("selected binding ids must be unique"),
        DependencyPlanningContractError::InvalidField {
            field: "identity_key.selected_binding_ids",
            reason: "selected binding ids must be unique"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_unsupported_contract_version() {
    let mut request: pantograph_dependency_planning::DependencyReadinessRequest =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should decode");
    request.contract_version = 2;

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(request)
            .expect_err("unsupported readiness request version should fail"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_request.contract_version",
            reason: "only dependency readiness request contract version 1 is supported"
        }
    );
}

#[test]
fn dependency_readiness_request_rejects_missing_policy() {
    let mut value: serde_json::Value =
        serde_json::from_str(READINESS_REQUEST).expect("readiness request fixture should parse");
    value
        .as_object_mut()
        .expect("fixture root is object")
        .remove("policy");

    assert_eq!(
        ValidatedDependencyReadinessRequest::try_from(value)
            .expect_err("readiness request policy is required"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_readiness_request",
            reason: "request JSON did not match dependency readiness contract"
        }
    );
}
