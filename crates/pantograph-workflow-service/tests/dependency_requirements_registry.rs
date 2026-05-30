use std::sync::Arc;

use pantograph_dependency_environment_service::{
    DependencyRequirementsRegistry, InMemoryDependencyRequirementsRegistry,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentReadinessState, DependencyEnvironmentResult,
    DependencyEnvironmentValidationState, ValidatedDependencyEnvironmentResult,
};
use pantograph_workflow_service::{WorkflowErrorCode, WorkflowService};

const READY_RESULT: &str = include_str!(
    "../../pantograph-dependency-planning/tests/fixtures/dependency_environment_ready_result.json"
);

#[test]
fn workflow_service_seeds_requirements_registry_from_validated_ready_result() {
    let registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    let service = WorkflowService::new().with_dependency_requirements_registry(registry.clone());
    let result = validated_ready_result();
    let requirements_id = result
        .as_result()
        .dependency_requirements_id
        .clone()
        .expect("ready fixture has requirements id");

    service
        .store_dependency_requirements_payload_from_result(&result)
        .expect("ready result should seed registry");

    let entry = registry
        .lookup_requirements(&requirements_id)
        .expect("registry entry should be stored");
    assert_eq!(entry.payload.dependency_requirements_id, requirements_id);
    assert_eq!(entry.payload.identity_key, result.as_result().identity_key);
    assert_eq!(
        entry.payload.selected_binding_ids,
        result.as_result().selected_binding_ids
    );
    assert_eq!(entry.payload.requirements.len(), 1);
    assert_eq!(entry.payload.bindings.len(), 1);
}

#[test]
fn workflow_service_rejects_registry_seed_from_non_ready_result() {
    let registry = Arc::new(InMemoryDependencyRequirementsRegistry::new());
    let service = WorkflowService::new().with_dependency_requirements_registry(registry.clone());
    let mut result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    result.readiness_state = DependencyEnvironmentReadinessState::Unavailable;
    result.validation_state = DependencyEnvironmentValidationState::Unavailable;
    let result =
        ValidatedDependencyEnvironmentResult::try_from(result).expect("result should validate");

    let error = service
        .store_dependency_requirements_payload_from_result(&result)
        .expect_err("non-ready result should not seed registry");

    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    assert!(
        error
            .message()
            .contains("requirements payloads may only be seeded from ready"),
        "unexpected error: {error}"
    );
    assert!(registry.is_empty());
}

fn validated_ready_result() -> ValidatedDependencyEnvironmentResult {
    let result: DependencyEnvironmentResult =
        serde_json::from_str(READY_RESULT).expect("ready fixture should decode");
    ValidatedDependencyEnvironmentResult::try_from(result).expect("ready result should validate")
}
