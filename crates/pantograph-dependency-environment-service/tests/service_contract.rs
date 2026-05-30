use pantograph_dependency_environment_service::{
    DependencyEnvironmentProvider, DependencyEnvironmentReadinessSnapshot,
    DependencyEnvironmentReadinessSnapshotProvider, DependencyEnvironmentReadinessSnapshotStatus,
    DependencyEnvironmentService, DependencyEnvironmentServiceError,
    NotImplementedDependencyEnvironmentProvider,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentFailureState, DependencyEnvironmentInstallState,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRequest, DependencyEnvironmentResult,
    DependencyEnvironmentValidationState, DependencyPlanningContractError,
    DependencyPlanningDiagnosticCode, DependencyRequirementsId,
    ValidatedDependencyEnvironmentRequest,
};

const RESOLVE_REQUEST: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
);
const READY_RESULT: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_ready_result.json"
);

#[test]
fn not_implemented_provider_returns_validated_diagnostic_result() {
    let request = validated_request(RESOLVE_REQUEST);
    let service = DependencyEnvironmentService::new(NotImplementedDependencyEnvironmentProvider);

    let result = service
        .handle(&request)
        .expect("not-implemented provider output should validate");

    assert_eq!(
        result.as_result().readiness_state,
        DependencyEnvironmentReadinessState::NotImplemented
    );
    assert_eq!(
        result.as_result().install_state,
        DependencyEnvironmentInstallState::NotImplemented
    );
    assert_eq!(
        result.as_result().validation_state,
        DependencyEnvironmentValidationState::NotImplemented
    );
    assert_eq!(
        result.as_result().failure_state,
        Some(DependencyEnvironmentFailureState::NotImplemented)
    );
    assert_eq!(
        result
            .as_result()
            .diagnostics
            .iter()
            .map(|diagnostic| &diagnostic.code)
            .collect::<Vec<_>>(),
        vec![&DependencyPlanningDiagnosticCode::NotImplemented]
    );
    assert_eq!(
        result.as_result().selected_binding_ids,
        request.as_request().identity_key.selected_binding_ids
    );
}

#[test]
fn service_rejects_provider_result_that_is_not_semantically_valid() {
    let request = validated_request(RESOLVE_REQUEST);
    let ready_without_environment = ReadyWithoutEnvironmentProvider;
    let service = DependencyEnvironmentService::new(ready_without_environment);

    assert_eq!(
        service
            .handle(&request)
            .expect_err("service should validate provider output"),
        DependencyEnvironmentServiceError::InvalidProviderResult(
            DependencyPlanningContractError::MissingField {
                field: "environment_ref"
            }
        )
    );
}

#[test]
fn validated_result_boundary_rejects_path_shaped_json_fields() {
    let mut value: serde_json::Value =
        serde_json::from_str(READY_RESULT).expect("ready fixture should parse");
    value
        .as_object_mut()
        .expect("ready fixture should be an object")
        .insert(
            "model_path".to_string(),
            serde_json::json!("/models/tiny-sd"),
        );

    assert_eq!(
        pantograph_dependency_planning::ValidatedDependencyEnvironmentResult::try_from(value)
            .expect_err("validated result boundary should reject path-shaped fields"),
        DependencyPlanningContractError::InvalidField {
            field: "dependency_environment_result",
            reason: "result must not contain path-shaped dependency identity fields"
        }
    );
}

#[test]
fn snapshot_provider_returns_fresh_matching_snapshot() {
    let request = validated_request_with_requirements();
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &request,
                ready_result_for_request(&request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("snapshot should validate"),
        )
        .expect("insert snapshot");
    let service = DependencyEnvironmentService::new(provider);

    let result = service.handle(&request).expect("snapshot should resolve");

    assert_eq!(
        result.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Ready
    );
    assert_eq!(
        result.as_result().dependency_requirements_id,
        request.as_request().dependency_requirements_id
    );
}

#[test]
fn snapshot_provider_fails_closed_when_snapshot_is_missing() {
    let request = validated_request_with_requirements();
    let service =
        DependencyEnvironmentService::new(DependencyEnvironmentReadinessSnapshotProvider::new());

    let result = service
        .handle(&request)
        .expect("missing snapshot should still produce a valid result");

    assert_eq!(
        result.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Missing
    );
    assert_eq!(
        result.as_result().validation_state,
        DependencyEnvironmentValidationState::Unavailable
    );
    assert_eq!(
        result.as_result().failure_state,
        Some(DependencyEnvironmentFailureState::RequirementsUnavailable)
    );
}

#[test]
fn snapshot_provider_fails_closed_when_snapshot_is_stale() {
    let request = validated_request_with_requirements();
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &request,
                ready_result_for_request(&request),
                DependencyEnvironmentReadinessSnapshotStatus::Stale,
            )
            .expect("snapshot should validate"),
        )
        .expect("insert snapshot");
    let service = DependencyEnvironmentService::new(provider);

    let result = service
        .handle(&request)
        .expect("stale snapshot should still produce a valid result");

    assert_eq!(
        result.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Unavailable
    );
    assert_eq!(
        result.as_result().validation_state,
        DependencyEnvironmentValidationState::Stale
    );
    assert_eq!(
        result.as_result().failure_state,
        Some(DependencyEnvironmentFailureState::RequirementsUnavailable)
    );
}

#[test]
fn snapshot_provider_fails_closed_when_snapshot_key_is_mismatched() {
    let request = validated_request_with_requirements();
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &request,
                ready_result_for_request(&request),
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .expect("snapshot should validate"),
        )
        .expect("insert snapshot");
    let mut mismatched_request = request.as_request().clone();
    mismatched_request.dependency_requirements_id =
        Some(DependencyRequirementsId::parse("tiny-sd:pytorch:alternate").expect("valid id"));
    let mismatched_request = ValidatedDependencyEnvironmentRequest::try_from(mismatched_request)
        .expect("mismatched request should still validate");
    let service = DependencyEnvironmentService::new(provider);

    let result = service
        .handle(&mismatched_request)
        .expect("mismatch should still produce a valid result");

    assert_eq!(
        result.as_result().readiness_state,
        DependencyEnvironmentReadinessState::Invalid
    );
    assert_eq!(
        result.as_result().validation_state,
        DependencyEnvironmentValidationState::Invalid
    );
    assert_eq!(
        result.as_result().failure_state,
        Some(DependencyEnvironmentFailureState::InvalidRequest)
    );
}

fn validated_request(fixture: &str) -> ValidatedDependencyEnvironmentRequest {
    let value: serde_json::Value =
        serde_json::from_str(fixture).expect("request fixture should parse");
    ValidatedDependencyEnvironmentRequest::try_from(value).expect("request fixture should validate")
}

fn validated_request_with_requirements() -> ValidatedDependencyEnvironmentRequest {
    let mut request: DependencyEnvironmentRequest =
        serde_json::from_str(RESOLVE_REQUEST).expect("request fixture should decode");
    request.dependency_requirements_id =
        Some(DependencyRequirementsId::parse("tiny-sd:pytorch:linux-x86-64").expect("valid id"));
    ValidatedDependencyEnvironmentRequest::try_from(request).expect("request should validate")
}

fn ready_result_for_request(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.action = request.as_request().action;
    result.identity_key = request.as_request().identity_key.clone();
    result.dependency_requirements_id = request.as_request().dependency_requirements_id.clone();
    result.selected_binding_ids = request
        .as_request()
        .identity_key
        .selected_binding_ids
        .clone();
    result
}

#[derive(Debug, Clone, Copy)]
struct ReadyWithoutEnvironmentProvider;

impl DependencyEnvironmentProvider for ReadyWithoutEnvironmentProvider {
    fn resolve(
        &self,
        _request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        let mut result: DependencyEnvironmentResult =
            serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
        result.environment_ref = None;
        result
    }

    fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve(request)
    }

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve(request)
    }
}
