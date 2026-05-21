use pantograph_dependency_planning::{
    DependencyPlanningContractError, DependencyPlanningDiagnosticCode,
    DependencyPlanningIdentityKey, DependencyPlanningPlatformContext, DependencyPlanningRequest,
    DependencyPlanningResult, DependencyPlanningState, DependencyPreflightModelRef,
    ModelArtifactKind, PumasArtifactEntryPath, PumasArtifactEntryPathError,
    PumasArtifactLoadPathKind, ValidatedDependencyPlanningRequest,
};

const VALID_REQUEST: &str = include_str!("fixtures/dependency_planning_request.json");
const READY_RESULT: &str = include_str!("fixtures/dependency_planning_ready_result.json");
const UNAVAILABLE_RESULT: &str =
    include_str!("fixtures/dependency_planning_unavailable_result.json");
const IDENTITY_KEY: &str = include_str!("fixtures/dependency_planning_identity_key.json");
const PREFLIGHT_MODEL_REF: &str = include_str!("fixtures/dependency_preflight_model_ref.json");

#[test]
fn dependency_planning_request_fixture_decodes_and_validates() {
    let request: DependencyPlanningRequest =
        serde_json::from_str(VALID_REQUEST).expect("request fixture should decode");
    let validated = ValidatedDependencyPlanningRequest::try_from(request)
        .expect("request fixture should validate");

    assert_eq!(
        validated.as_request().model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(validated.as_request().task_id.as_str(), "image_generation");
    assert_eq!(
        validated.as_request().expected_artifact_kind,
        Some(ModelArtifactKind::DiffusersBundle)
    );
    assert_eq!(
        validated
            .as_request()
            .platform_context
            .as_ref()
            .map(|context| context.platform_key.as_str()),
        Some("linux-x86_64")
    );
    assert_eq!(
        validated
            .as_request()
            .caller_context
            .source_node_type
            .as_ref()
            .map(|node_type| node_type.as_str()),
        Some("llm-inference")
    );
    assert_eq!(validated.as_request().dependency_override_patches.len(), 1);
}

#[test]
fn dependency_planning_platform_context_derives_stable_os_arch_key() {
    let context = DependencyPlanningPlatformContext::from_os_arch("Linux", "X86_64")
        .expect("os/arch should form a platform key");

    assert_eq!(context.platform_key.as_str(), "linux-x86_64");
}

#[test]
fn dependency_planning_request_rejects_raw_platform_context_json() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        },
        "task_id": "image_generation",
        "platform_context": {
            "os": "linux",
            "arch": "x86_64"
        }
    });

    let error = serde_json::from_value::<DependencyPlanningRequest>(value)
        .expect_err("platform context must use the typed platform_key field");

    assert!(error.to_string().contains("platform_key"));
}

#[test]
fn dependency_planning_result_ready_fixture_requires_load_target() {
    let result: DependencyPlanningResult =
        serde_json::from_str(READY_RESULT).expect("ready result fixture should decode");

    result.validate().expect("ready result should validate");
    assert_eq!(result.state, DependencyPlanningState::Ready);
    let target = result
        .load_target
        .as_ref()
        .expect("ready result should carry load target");
    assert_eq!(target.artifact_kind, ModelArtifactKind::DiffusersBundle);
    assert_eq!(target.load_path_kind, PumasArtifactLoadPathKind::Directory);
}

#[test]
fn dependency_planning_result_unavailable_fixture_has_typed_diagnostics() {
    let result: DependencyPlanningResult =
        serde_json::from_str(UNAVAILABLE_RESULT).expect("unavailable result fixture should decode");

    result
        .validate()
        .expect("unavailable result without load target should validate");
    assert_eq!(result.state, DependencyPlanningState::Unavailable);
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|diagnostic| &diagnostic.code),
        Some(&DependencyPlanningDiagnosticCode::PumasUnavailable)
    );
}

#[test]
fn dependency_planning_request_rejects_empty_pumas_model_id() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": " "
        },
        "task_id": "image_generation"
    });

    let request: DependencyPlanningRequest =
        serde_json::from_value(value).expect("shape should decode before validation");
    let error = ValidatedDependencyPlanningRequest::try_from(request)
        .expect_err("empty model id should fail validation");

    assert_eq!(
        error,
        DependencyPlanningContractError::MissingField {
            field: "pumas_model_ref.model_id"
        }
    );
}

#[test]
fn dependency_planning_request_rejects_raw_local_artifact_entry_paths() {
    let error = PumasArtifactEntryPath::parse("/models/tiny-sd")
        .expect_err("absolute local path should not be a Pumas entry path");

    assert_eq!(error, PumasArtifactEntryPathError::LocalPath);
}

#[test]
fn ready_result_without_target_is_invalid() {
    let result: DependencyPlanningResult = serde_json::from_value(serde_json::json!({
        "state": "ready",
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        }
    }))
    .expect("ready shape should decode");

    assert_eq!(
        result.validate().expect_err("ready result requires target"),
        DependencyPlanningContractError::ReadyResultMissingLoadTarget
    );
}

#[test]
fn dependency_planning_identity_key_fixture_decodes_and_validates() {
    let identity_key: DependencyPlanningIdentityKey =
        serde_json::from_str(IDENTITY_KEY).expect("identity key fixture should decode");

    identity_key
        .validate()
        .expect("path-free identity key should validate");
    assert_eq!(
        identity_key.model_ref.model_id,
        "image/stable-diffusion/tiny-sd"
    );
    assert_eq!(
        identity_key.model_ref.selected_artifact_id.as_deref(),
        Some("diffusers-bundle")
    );
    assert_eq!(identity_key.model_ref.selected_artifact_path, None);
    assert_eq!(identity_key.task_id.as_str(), "image_generation");
    assert_eq!(
        identity_key
            .selected_runtime_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("pytorch")
    );
}

#[test]
fn dependency_preflight_model_ref_fixture_decodes_and_validates() {
    let model_ref: DependencyPreflightModelRef =
        serde_json::from_str(PREFLIGHT_MODEL_REF).expect("preflight fixture should decode");

    model_ref
        .validate()
        .expect("path-free preflight model ref should validate");
    assert_eq!(model_ref.contract_version, 1);
    assert_eq!(
        model_ref
            .dependency_requirements_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("tiny-sd:pytorch:linux-x86_64:torch-diffusers")
    );
    assert_eq!(model_ref.diagnostics.len(), 1);
}

#[test]
fn dependency_preflight_model_ref_rejects_load_target_fields() {
    let value = serde_json::json!({
        "contract_version": 1,
        "identity_key": {
            "model_ref": {
                "model_id": "image/stable-diffusion/tiny-sd"
            },
            "task_id": "image_generation"
        },
        "load_target": {
            "local_load_path": "/models/tiny-sd"
        }
    });

    serde_json::from_value::<DependencyPreflightModelRef>(value)
        .expect_err("preflight identity must not deserialize load target fields");
}

#[test]
fn dependency_planning_identity_key_rejects_model_path_fields() {
    let value = serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd"
        },
        "task_id": "image_generation",
        "model_path": "/models/tiny-sd"
    });

    serde_json::from_value::<DependencyPlanningIdentityKey>(value)
        .expect_err("identity key must not deserialize model_path fields");
}

#[test]
fn dependency_planning_identity_key_rejects_selected_artifact_path_identity() {
    let identity_key: DependencyPlanningIdentityKey = serde_json::from_value(serde_json::json!({
        "model_ref": {
            "model_id": "image/stable-diffusion/tiny-sd",
            "selected_artifact_path": "image/stable-diffusion/tiny-sd/model_index.json"
        },
        "task_id": "image_generation"
    }))
    .expect("shape decodes before path-free validation");

    assert_eq!(
        identity_key
            .validate()
            .expect_err("selected artifact path is not path-free identity"),
        DependencyPlanningContractError::InvalidField {
            field: "pumas_model_ref.selected_artifact_path",
            reason: "path-free dependency identity must not carry selected artifact paths"
        }
    );
}
