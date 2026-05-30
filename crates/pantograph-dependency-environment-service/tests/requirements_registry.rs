use pantograph_dependency_environment_service::{
    resolve_dependency_requirements_payload, DependencyRequirementsPayload,
    DependencyRequirementsRegistryEntry, DependencyRequirementsRegistryError,
    InMemoryDependencyRequirementsRegistry,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentInstallState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentRequest, DependencyEnvironmentResult,
    DependencyEnvironmentValidationState, DependencyRequirementsId, DependencyTaskId,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyEnvironmentResult,
};

const RESOLVE_REQUEST: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
);
const READY_RESULT: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_ready_result.json"
);

#[test]
fn registry_resolves_fresh_payload_for_matching_validated_request() {
    let request = validated_request_with_requirements();
    let payload = payload_from_ready_result();
    let registry = InMemoryDependencyRequirementsRegistry::new();
    registry.insert_payload(payload.clone());

    let resolved = resolve_dependency_requirements_payload(&registry, &request)
        .expect("fresh registry payload should resolve");

    assert_eq!(resolved, payload);
    assert_eq!(registry.len(), 1);
}

#[test]
fn registry_fails_closed_when_request_has_no_requirements_id() {
    let request = validated_request_without_requirements();
    let registry = InMemoryDependencyRequirementsRegistry::new();

    let error = resolve_dependency_requirements_payload(&registry, &request)
        .expect_err("missing id should fail closed");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::MissingRequirementsId
    );
    assert!(registry.is_empty());
}

#[test]
fn registry_fails_closed_when_payload_is_missing() {
    let request = validated_request_with_requirements();
    let registry = InMemoryDependencyRequirementsRegistry::new();

    let error = resolve_dependency_requirements_payload(&registry, &request)
        .expect_err("missing payload should fail closed");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::MissingPayload {
            dependency_requirements_id: request
                .as_request()
                .dependency_requirements_id
                .clone()
                .expect("requirements id"),
        }
    );
}

#[test]
fn registry_fails_closed_when_payload_is_stale() {
    let request = validated_request_with_requirements();
    let payload = payload_from_ready_result();
    let registry = InMemoryDependencyRequirementsRegistry::new();
    registry.insert_entry(DependencyRequirementsRegistryEntry::stale(payload));

    let error = resolve_dependency_requirements_payload(&registry, &request)
        .expect_err("stale payload should fail closed");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::StalePayload {
            dependency_requirements_id: request
                .as_request()
                .dependency_requirements_id
                .clone()
                .expect("requirements id"),
        }
    );
}

#[test]
fn registry_fails_closed_when_payload_identity_does_not_match_request() {
    let mut request = validated_request_with_requirements().as_request().clone();
    request.planning_request.task_id =
        DependencyTaskId::parse("other_image_generation").expect("task id");
    request.identity_key =
        pantograph_dependency_planning::DependencyPlanningIdentityKey::from_planning_request(
            &request.planning_request,
        )
        .expect("identity key");
    let request =
        ValidatedDependencyEnvironmentRequest::try_from(request).expect("request should validate");
    let payload = payload_from_ready_result();
    let registry = InMemoryDependencyRequirementsRegistry::new();
    registry.insert_payload(payload);

    let error = resolve_dependency_requirements_payload(&registry, &request)
        .expect_err("mismatched payload should fail closed");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::MismatchedPayload {
            dependency_requirements_id: request
                .as_request()
                .dependency_requirements_id
                .clone()
                .expect("requirements id"),
            field: "dependency_requirements_payload.identity_key",
        }
    );
}

#[test]
fn payload_extraction_rejects_result_without_requirements_id() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.dependency_requirements_id = None;
    let result =
        ValidatedDependencyEnvironmentResult::try_from(result).expect_err("result should fail");

    assert_eq!(
        result,
        pantograph_dependency_planning::DependencyPlanningContractError::MissingField {
            field: "dependency_requirements_id",
        }
    );
}

#[test]
fn payload_validation_rejects_selected_binding_without_binding_row() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.bindings.clear();
    let result =
        ValidatedDependencyEnvironmentResult::try_from(result).expect("non-ready proof not needed");

    let error = DependencyRequirementsPayload::from_result(&result)
        .expect_err("payload without bindings should be rejected");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::InvalidPayload {
            field: "dependency_requirements_payload.bindings",
            reason: "requirements payload must include at least one binding",
        }
    );
}

#[test]
fn payload_extraction_rejects_non_ready_result_state() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.readiness_state = DependencyEnvironmentReadinessState::Unavailable;
    result.validation_state = DependencyEnvironmentValidationState::Unavailable;
    let result =
        ValidatedDependencyEnvironmentResult::try_from(result).expect("result should validate");

    let error = DependencyRequirementsPayload::from_result(&result)
        .expect_err("non-ready result should not seed requirements payload");

    assert_eq!(
        error,
        DependencyRequirementsRegistryError::InvalidResultState {
            field: "dependency_environment_result.readiness_state",
            reason:
                "requirements payloads may only be seeded from resolved or ready dependency environment results",
        }
    );
}

#[test]
fn payload_extraction_accepts_valid_resolved_result_state() {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.readiness_state = DependencyEnvironmentReadinessState::Resolved;
    result.install_state = DependencyEnvironmentInstallState::NotRequested;
    result.environment_ref = None;
    result.binding_statuses.clear();
    result.operation = None;
    let result =
        ValidatedDependencyEnvironmentResult::try_from(result).expect("result should validate");

    let payload = DependencyRequirementsPayload::from_result(&result)
        .expect("valid resolved result should seed requirements payload");

    assert_eq!(
        payload.dependency_requirements_id,
        result
            .as_result()
            .dependency_requirements_id
            .clone()
            .expect("requirements id")
    );
    assert_eq!(payload.requirements.len(), 1);
    assert_eq!(payload.bindings.len(), 1);
}

fn payload_from_ready_result() -> DependencyRequirementsPayload {
    let result = validated_ready_result();
    DependencyRequirementsPayload::from_result(&result).expect("ready result should yield payload")
}

fn validated_ready_result() -> ValidatedDependencyEnvironmentResult {
    let result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    ValidatedDependencyEnvironmentResult::try_from(result).expect("ready result should validate")
}

fn validated_request_with_requirements() -> ValidatedDependencyEnvironmentRequest {
    let mut request: DependencyEnvironmentRequest =
        serde_json::from_str(RESOLVE_REQUEST).expect("request fixture should decode");
    request.dependency_requirements_id = Some(
        DependencyRequirementsId::parse("tiny-sd:pytorch:linux-x86_64:torch-diffusers")
            .expect("requirements id"),
    );
    ValidatedDependencyEnvironmentRequest::try_from(request).expect("request should validate")
}

fn validated_request_without_requirements() -> ValidatedDependencyEnvironmentRequest {
    let request: DependencyEnvironmentRequest =
        serde_json::from_str(RESOLVE_REQUEST).expect("request fixture should decode");
    ValidatedDependencyEnvironmentRequest::try_from(request).expect("request should validate")
}
