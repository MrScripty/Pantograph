use pantograph_dependency_planning::{
    DependencyPlanningContractError, DependencyPlanningDiagnosticCode, DependencyPlanningRequest,
    DependencyPlanningResult, DependencyPlanningState, ModelArtifactKind, PumasArtifactEntryPath,
    PumasArtifactEntryPathError, PumasArtifactLoadPathKind, ValidatedDependencyPlanningRequest,
};

const VALID_REQUEST: &str = include_str!("fixtures/dependency_planning_request.json");
const READY_RESULT: &str = include_str!("fixtures/dependency_planning_ready_result.json");
const UNAVAILABLE_RESULT: &str =
    include_str!("fixtures/dependency_planning_unavailable_result.json");

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
    assert_eq!(validated.as_request().dependency_override_patches.len(), 1);
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
