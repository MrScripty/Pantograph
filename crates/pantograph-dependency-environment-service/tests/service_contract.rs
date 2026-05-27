use pantograph_dependency_environment_service::{
    DependencyEnvironmentProvider, DependencyEnvironmentService, DependencyEnvironmentServiceError,
    NotImplementedDependencyEnvironmentProvider,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentFailureState, DependencyEnvironmentInstallState,
    DependencyEnvironmentReadinessState, DependencyEnvironmentResult,
    DependencyEnvironmentValidationState, DependencyPlanningContractError,
    DependencyPlanningDiagnosticCode, ValidatedDependencyEnvironmentRequest,
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

fn validated_request(fixture: &str) -> ValidatedDependencyEnvironmentRequest {
    let value: serde_json::Value =
        serde_json::from_str(fixture).expect("request fixture should parse");
    ValidatedDependencyEnvironmentRequest::try_from(value).expect("request fixture should validate")
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
