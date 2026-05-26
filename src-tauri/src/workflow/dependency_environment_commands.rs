use pantograph_dependency_planning::{
    DependencyEnvironmentFailureState, DependencyEnvironmentInstallState,
    DependencyEnvironmentReadinessState, DependencyEnvironmentRequest, DependencyEnvironmentResult,
    DependencyEnvironmentValidationState, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningSeverity,
    ValidatedDependencyEnvironmentRequest,
};

const NOT_IMPLEMENTED_MESSAGE: &str =
    "Canonical dependency-environment execution is not implemented yet";

pub async fn run_dependency_environment_action(
    request: DependencyEnvironmentRequest,
) -> Result<DependencyEnvironmentResult, String> {
    let request = ValidatedDependencyEnvironmentRequest::try_from(request)
        .map_err(|error| error.to_string())?
        .into_inner();

    let result = not_implemented_result(request);
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}

fn not_implemented_result(request: DependencyEnvironmentRequest) -> DependencyEnvironmentResult {
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key,
        readiness_state: DependencyEnvironmentReadinessState::NotImplemented,
        install_state: DependencyEnvironmentInstallState::NotImplemented,
        validation_state: DependencyEnvironmentValidationState::NotImplemented,
        failure_state: Some(DependencyEnvironmentFailureState::NotImplemented),
        dependency_requirements_id: request.dependency_requirements_id,
        environment_ref: request.environment_ref,
        requirements: Vec::new(),
        bindings: Vec::new(),
        selected_binding_ids: request.planning_request.selected_binding_ids,
        binding_statuses: Vec::new(),
        operation: None,
        validation_errors: Vec::new(),
        diagnostics: vec![DependencyPlanningDiagnostic {
            code: DependencyPlanningDiagnosticCode::NotImplemented,
            severity: DependencyPlanningSeverity::Error,
            message: NOT_IMPLEMENTED_MESSAGE.to_string(),
            model_id: Some(request.planning_request.model_ref.model_id),
            runtime_id: request
                .planning_request
                .scheduler_intent
                .requested_runtime_id,
            device_id: request
                .planning_request
                .scheduler_intent
                .requested_device_id,
            field_path: Some("dependency_environment_request.action".to_string()),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve_request() -> DependencyEnvironmentRequest {
        serde_json::from_str(include_str!(
            "../../../crates/pantograph-dependency-planning/tests/fixtures/dependency_environment_resolve_request.json"
        ))
        .expect("fixture decodes")
    }

    #[tokio::test]
    async fn dependency_environment_action_returns_typed_not_implemented_result() {
        let result = run_dependency_environment_action(resolve_request())
            .await
            .expect("typed result");

        assert_eq!(
            result.readiness_state,
            DependencyEnvironmentReadinessState::NotImplemented
        );
        assert_eq!(
            result.install_state,
            DependencyEnvironmentInstallState::NotImplemented
        );
        assert_eq!(
            result.validation_state,
            DependencyEnvironmentValidationState::NotImplemented
        );
        assert_eq!(
            result.failure_state,
            Some(DependencyEnvironmentFailureState::NotImplemented)
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(
            result.diagnostics[0].code,
            DependencyPlanningDiagnosticCode::NotImplemented
        );
        assert_eq!(
            result.diagnostics[0]
                .runtime_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("pytorch")
        );
        assert_eq!(
            result.diagnostics[0]
                .device_id
                .as_ref()
                .map(|id| id.as_str()),
            Some("cuda:0")
        );
        assert_eq!(result.selected_binding_ids.len(), 1);
        result.validate().expect("result contract remains valid");
    }

    #[tokio::test]
    async fn dependency_environment_action_rejects_path_shaped_legacy_requests() {
        let legacy_request = serde_json::json!({
            "action": "resolve",
            "mode": "manual",
            "modelPath": "/models/model.gguf"
        });
        let error = serde_json::from_value::<DependencyEnvironmentRequest>(legacy_request)
            .expect_err("legacy request must not deserialize");

        assert!(
            error.to_string().contains("unknown field"),
            "unexpected error: {error}"
        );
    }
}
