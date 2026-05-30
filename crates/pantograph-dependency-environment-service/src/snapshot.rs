use std::sync::{Arc, Mutex};

use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentFailureState,
    DependencyEnvironmentInstallState, DependencyEnvironmentOperation,
    DependencyEnvironmentOperationState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentRef, DependencyEnvironmentResult, DependencyEnvironmentValidationState,
    DependencyPlanningContractError, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningIdentityKey, DependencyPlanningRequest,
    DependencyPlanningSeverity, DependencyRequirementsId, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyEnvironmentResult,
};

use crate::{DependencyEnvironmentProvider, DependencyReadinessWorkItem};

/// Freshness state for a backend-owned dependency-readiness snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyEnvironmentReadinessSnapshotStatus {
    Fresh,
    Stale,
}

/// Backend-owned dependency-readiness snapshot keyed by canonical request data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyEnvironmentReadinessSnapshot {
    pub action: DependencyEnvironmentAction,
    pub identity_key: DependencyPlanningIdentityKey,
    pub planning_request: DependencyPlanningRequest,
    pub dependency_requirements_id: Option<DependencyRequirementsId>,
    pub request_environment_ref: Option<DependencyEnvironmentRef>,
    pub result: DependencyEnvironmentResult,
    pub status: DependencyEnvironmentReadinessSnapshotStatus,
}

impl DependencyEnvironmentReadinessSnapshot {
    /// Creates a snapshot whose key is derived from the validated request.
    ///
    /// # Errors
    ///
    /// Returns `InvalidSnapshotResult` when the result is not a validated
    /// dependency-environment result, and `MismatchedSnapshot` when the result
    /// does not belong to the request key.
    pub fn for_request(
        request: &ValidatedDependencyEnvironmentRequest,
        result: DependencyEnvironmentResult,
        status: DependencyEnvironmentReadinessSnapshotStatus,
    ) -> Result<Self, DependencyEnvironmentSnapshotStoreError> {
        let request = request.as_request();
        let snapshot = Self {
            action: request.action,
            identity_key: request.identity_key.clone(),
            planning_request: request.planning_request.clone(),
            dependency_requirements_id: request.dependency_requirements_id.clone(),
            request_environment_ref: request.environment_ref.clone(),
            result,
            status,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Creates a fresh non-ready snapshot for queued work that has not yet
    /// been probed by a host package/runtime producer.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the generated snapshot is not internally
    /// consistent with the queued request.
    pub fn unavailable_for_work_item(
        item: &DependencyReadinessWorkItem,
    ) -> Result<Self, DependencyEnvironmentSnapshotStoreError> {
        Self::for_request(
            &item.request,
            producer_unavailable_result(&item.request),
            DependencyEnvironmentReadinessSnapshotStatus::Fresh,
        )
    }

    fn validate(&self) -> Result<(), DependencyEnvironmentSnapshotStoreError> {
        let validated_result = ValidatedDependencyEnvironmentResult::try_from(self.result.clone())
            .map_err(DependencyEnvironmentSnapshotStoreError::InvalidSnapshotResult)?;
        let result = validated_result.as_result();
        let derived_identity =
            DependencyPlanningIdentityKey::from_planning_request(&self.planning_request)
                .map_err(DependencyEnvironmentSnapshotStoreError::InvalidSnapshotKey)?;
        if self.identity_key != derived_identity {
            return Err(
                DependencyEnvironmentSnapshotStoreError::MismatchedSnapshot {
                    field: "dependency_environment_snapshot.planning_request",
                },
            );
        }
        if result.action != self.action {
            return Err(
                DependencyEnvironmentSnapshotStoreError::MismatchedSnapshot {
                    field: "dependency_environment_snapshot.result.action",
                },
            );
        }
        if result.identity_key != self.identity_key {
            return Err(
                DependencyEnvironmentSnapshotStoreError::MismatchedSnapshot {
                    field: "dependency_environment_snapshot.result.identity_key",
                },
            );
        }
        if result.dependency_requirements_id != self.dependency_requirements_id {
            return Err(
                DependencyEnvironmentSnapshotStoreError::MismatchedSnapshot {
                    field: "dependency_environment_snapshot.result.dependency_requirements_id",
                },
            );
        }
        if result.selected_binding_ids != self.identity_key.selected_binding_ids {
            return Err(
                DependencyEnvironmentSnapshotStoreError::MismatchedSnapshot {
                    field: "dependency_environment_snapshot.result.selected_binding_ids",
                },
            );
        }
        Ok(())
    }

    fn matches_request(&self, request: &ValidatedDependencyEnvironmentRequest) -> bool {
        let request = request.as_request();
        self.action == request.action
            && self.identity_key == request.identity_key
            && self.dependency_requirements_id == request.dependency_requirements_id
            && self.request_environment_ref == request.environment_ref
    }

    fn has_matching_identity(&self, request: &ValidatedDependencyEnvironmentRequest) -> bool {
        let request = request.as_request();
        self.action == request.action && self.identity_key == request.identity_key
    }
}

/// Errors returned while inserting backend-owned readiness snapshots.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DependencyEnvironmentSnapshotStoreError {
    #[error("dependency environment readiness snapshot key is invalid: {0}")]
    InvalidSnapshotKey(DependencyPlanningContractError),
    #[error("dependency environment readiness snapshot result is invalid: {0}")]
    InvalidSnapshotResult(DependencyPlanningContractError),
    #[error("dependency environment readiness snapshot field does not match request key: {field}")]
    MismatchedSnapshot { field: &'static str },
}

/// Synchronous path-free provider backed by validated readiness snapshots.
#[derive(Debug, Clone, Default)]
pub struct DependencyEnvironmentReadinessSnapshotProvider {
    snapshots: Arc<Mutex<Vec<DependencyEnvironmentReadinessSnapshot>>>,
}

impl DependencyEnvironmentReadinessSnapshotProvider {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts or replaces a readiness snapshot for the same request key.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the snapshot is not internally consistent.
    pub fn insert_snapshot(
        &self,
        snapshot: DependencyEnvironmentReadinessSnapshot,
    ) -> Result<(), DependencyEnvironmentSnapshotStoreError> {
        snapshot.validate()?;
        let mut snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        snapshots.retain(|existing| !existing.matches_snapshot_key(&snapshot));
        snapshots.push(snapshot);
        Ok(())
    }

    #[must_use]
    pub fn snapshot_count(&self) -> usize {
        self.snapshots
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    fn resolve_snapshot(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        let snapshots = self
            .snapshots
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.matches_request(request))
        {
            return match snapshot.status {
                DependencyEnvironmentReadinessSnapshotStatus::Fresh => snapshot.result.clone(),
                DependencyEnvironmentReadinessSnapshotStatus::Stale => {
                    stale_snapshot_result(request)
                }
            };
        }
        if snapshots
            .iter()
            .any(|snapshot| snapshot.has_matching_identity(request))
        {
            return mismatched_snapshot_result(request);
        }
        missing_snapshot_result(request)
    }
}

impl DependencyEnvironmentReadinessSnapshot {
    fn matches_snapshot_key(&self, other: &Self) -> bool {
        self.action == other.action
            && self.identity_key == other.identity_key
            && self.dependency_requirements_id == other.dependency_requirements_id
            && self.request_environment_ref == other.request_environment_ref
    }
}

impl DependencyEnvironmentProvider for DependencyEnvironmentReadinessSnapshotProvider {
    fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve_snapshot(request)
    }

    fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve_snapshot(request)
    }

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve_snapshot(request)
    }
}

fn missing_snapshot_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    diagnostic_result(
        request,
        DependencyEnvironmentReadinessState::Missing,
        DependencyEnvironmentValidationState::Unavailable,
        DependencyEnvironmentFailureState::RequirementsUnavailable,
        DependencyPlanningDiagnosticCode::InternalError,
        "No fresh dependency readiness snapshot matches the request.",
        "dependency_environment.snapshot",
    )
}

fn stale_snapshot_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    diagnostic_result(
        request,
        DependencyEnvironmentReadinessState::Unavailable,
        DependencyEnvironmentValidationState::Stale,
        DependencyEnvironmentFailureState::RequirementsUnavailable,
        DependencyPlanningDiagnosticCode::ArtifactStale,
        "Dependency readiness snapshot is stale.",
        "dependency_environment.snapshot.status",
    )
}

fn mismatched_snapshot_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    diagnostic_result(
        request,
        DependencyEnvironmentReadinessState::Invalid,
        DependencyEnvironmentValidationState::Invalid,
        DependencyEnvironmentFailureState::InvalidRequest,
        DependencyPlanningDiagnosticCode::InvalidRequest,
        "Dependency readiness snapshot identity matched but request details did not.",
        "dependency_environment.snapshot.key",
    )
}

fn producer_unavailable_result(
    request: &ValidatedDependencyEnvironmentRequest,
) -> DependencyEnvironmentResult {
    diagnostic_result(
        request,
        DependencyEnvironmentReadinessState::Unavailable,
        DependencyEnvironmentValidationState::Unavailable,
        DependencyEnvironmentFailureState::RequirementsUnavailable,
        DependencyPlanningDiagnosticCode::RuntimeUnavailable,
        "Dependency readiness producer has not published host probe evidence yet.",
        "dependency_environment.producer",
    )
}

fn diagnostic_result(
    request: &ValidatedDependencyEnvironmentRequest,
    readiness_state: DependencyEnvironmentReadinessState,
    validation_state: DependencyEnvironmentValidationState,
    failure_state: DependencyEnvironmentFailureState,
    diagnostic_code: DependencyPlanningDiagnosticCode,
    message: &'static str,
    field_path: &'static str,
) -> DependencyEnvironmentResult {
    let request = request.as_request();
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state,
        install_state: DependencyEnvironmentInstallState::NotRequested,
        validation_state,
        failure_state: Some(failure_state),
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
            code: diagnostic_code,
            severity: DependencyPlanningSeverity::Error,
            message: message.to_string(),
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
            field_path: Some(field_path.to_string()),
        }],
    }
}
