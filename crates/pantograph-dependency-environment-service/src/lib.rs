//! Dependency-environment service facade.
//!
//! This crate owns the canonical service boundary for
//! `ValidatedDependencyEnvironmentRequest -> ValidatedDependencyEnvironmentResult`.
//! It does not inspect files, call Pumas directly, create runtimes, spawn
//! background tasks, or adapt retired `ModelDependencyRequest` payloads.

use std::sync::Arc;

use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentFailureState,
    DependencyEnvironmentInstallState, DependencyEnvironmentOperation,
    DependencyEnvironmentOperationState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentResult, DependencyEnvironmentValidationState,
    DependencyPlanningContractError, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningSeverity,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyEnvironmentResult,
};

/// Fallible dependency-environment service errors.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyEnvironmentServiceError {
    #[error("dependency environment provider returned invalid result: {0}")]
    InvalidProviderResult(DependencyPlanningContractError),
    #[error("dependency environment action is not supported by this service version")]
    UnsupportedAction,
}

/// Provider boundary used by the dependency-environment service facade.
///
/// Providers may resolve, check, or install dependency environments from a
/// validated canonical request. Concrete providers are wired by the backend
/// composition root; this trait must not be implemented by graph editor,
/// Tauri, node-engine, or embedded-runtime compatibility adapters.
pub trait DependencyEnvironmentProvider: Send + Sync {
    fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult;

    fn check(&self, request: &ValidatedDependencyEnvironmentRequest)
        -> DependencyEnvironmentResult;

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult;
}

impl<T> DependencyEnvironmentProvider for Arc<T>
where
    T: DependencyEnvironmentProvider + ?Sized,
{
    fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        (**self).resolve(request)
    }

    fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        (**self).check(request)
    }

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        (**self).install(request)
    }
}

pub type SharedDependencyEnvironmentProvider = Arc<dyn DependencyEnvironmentProvider>;
pub type SharedDependencyEnvironmentService =
    DependencyEnvironmentService<SharedDependencyEnvironmentProvider>;

/// Canonical dependency-environment service facade.
#[derive(Debug, Clone)]
pub struct DependencyEnvironmentService<P> {
    provider: P,
}

impl<P> DependencyEnvironmentService<P>
where
    P: DependencyEnvironmentProvider,
{
    #[must_use]
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Runs the action declared by the validated request.
    ///
    /// # Errors
    ///
    /// Returns `InvalidProviderResult` when the provider emits a result that
    /// violates the shared dependency-environment result contract.
    pub fn handle(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> Result<ValidatedDependencyEnvironmentResult, DependencyEnvironmentServiceError> {
        match request.as_request().action {
            DependencyEnvironmentAction::Resolve => self.resolve(request),
            DependencyEnvironmentAction::Check => self.check(request),
            DependencyEnvironmentAction::Install => self.install(request),
            _ => Err(DependencyEnvironmentServiceError::UnsupportedAction),
        }
    }

    /// Resolves dependency-environment requirements for a validated request.
    ///
    /// # Errors
    ///
    /// Returns `InvalidProviderResult` when the provider emits a result that
    /// violates the shared dependency-environment result contract.
    pub fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> Result<ValidatedDependencyEnvironmentResult, DependencyEnvironmentServiceError> {
        validate_provider_result(self.provider.resolve(request))
    }

    /// Checks dependency-environment readiness for a validated request.
    ///
    /// # Errors
    ///
    /// Returns `InvalidProviderResult` when the provider emits a result that
    /// violates the shared dependency-environment result contract.
    pub fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> Result<ValidatedDependencyEnvironmentResult, DependencyEnvironmentServiceError> {
        validate_provider_result(self.provider.check(request))
    }

    /// Installs dependency-environment requirements for a validated request.
    ///
    /// # Errors
    ///
    /// Returns `InvalidProviderResult` when the provider emits a result that
    /// violates the shared dependency-environment result contract.
    pub fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> Result<ValidatedDependencyEnvironmentResult, DependencyEnvironmentServiceError> {
        validate_provider_result(self.provider.install(request))
    }
}

/// No-I/O provider for systems where dependency-environment execution is not
/// yet available.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotImplementedDependencyEnvironmentProvider;

impl DependencyEnvironmentProvider for NotImplementedDependencyEnvironmentProvider {
    fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        not_implemented_result(request)
    }

    fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        not_implemented_result(request)
    }

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        not_implemented_result(request)
    }
}

fn validate_provider_result(
    result: DependencyEnvironmentResult,
) -> Result<ValidatedDependencyEnvironmentResult, DependencyEnvironmentServiceError> {
    ValidatedDependencyEnvironmentResult::try_from(result)
        .map_err(DependencyEnvironmentServiceError::InvalidProviderResult)
}

fn not_implemented_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    let request = request.as_request();
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::NotImplemented,
        install_state: DependencyEnvironmentInstallState::NotImplemented,
        validation_state: DependencyEnvironmentValidationState::NotImplemented,
        failure_state: Some(DependencyEnvironmentFailureState::NotImplemented),
        dependency_requirements_id: request.dependency_requirements_id.clone(),
        environment_ref: request.environment_ref.clone(),
        requirements: Vec::new(),
        bindings: Vec::new(),
        selected_binding_ids: request.identity_key.selected_binding_ids.clone(),
        binding_statuses: Vec::new(),
        operation: Some(DependencyEnvironmentOperation {
            state: DependencyEnvironmentOperationState::Blocked,
            started_at_ms: None,
            completed_at_ms: None,
        }),
        validation_errors: Vec::new(),
        diagnostics: vec![DependencyPlanningDiagnostic {
            code: DependencyPlanningDiagnosticCode::NotImplemented,
            severity: DependencyPlanningSeverity::Error,
            message: "Dependency environment service provider is not implemented.".to_string(),
            model_id: Some(request.identity_key.model_ref.model_id.clone()),
            runtime_id: request
                .identity_key
                .scheduler_intent
                .requested_runtime_id
                .clone(),
            device_id: request
                .identity_key
                .scheduler_intent
                .requested_device_id
                .clone(),
            field_path: Some("dependency_environment.provider".to_string()),
        }],
    }
}
