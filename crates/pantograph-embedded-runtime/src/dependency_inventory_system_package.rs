use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyReadinessWorkItem, DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyBindingId, DependencyEnvironmentValidationState,
    DependencyInventoryObservationFreshness, DependencyInventoryObservationRow,
    DependencyInventoryObservationState, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningSeverity, DependencyProviderSourceState,
    DependencyRequirement, DependencyRequirementBinding, SystemPackageProviderSourceRow,
    SystemPackageProviderSourceSnapshot, SystemPackageRequirementDetails,
    ValidatedSystemPackageProviderSourceSnapshot,
};

use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};
use crate::dependency_inventory_system_package_source::{
    SystemPackageProviderSource, SystemPackageProviderSourceError,
};

pub(crate) struct SystemPackageDependencyInventoryProvider {
    source: Arc<dyn SystemPackageProviderSource>,
}

impl SystemPackageDependencyInventoryProvider {
    #[must_use]
    pub(crate) fn new(source: Arc<dyn SystemPackageProviderSource>) -> Self {
        Self { source }
    }
}

#[async_trait]
impl DependencyInventoryProvider for SystemPackageDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        match self.source.snapshot().await {
            Ok(snapshot) => {
                match ValidatedSystemPackageProviderSourceSnapshot::try_from(snapshot) {
                    Ok(snapshot) => {
                        observe_system_package_payload(&request, snapshot.as_snapshot())
                    }
                    Err(error) => source_error_observations(
                        &request.item,
                        &request.payload,
                        DependencyInventoryObservationState::Unavailable,
                        DependencyEnvironmentValidationState::Valid,
                        DependencyPlanningDiagnosticCode::RuntimeUnavailable,
                        format!("System-package inventory source is invalid: {error}."),
                    ),
                }
            }
            Err(error) => observations_from_source_error(&request.item, &request.payload, error),
        }
    }
}

fn observe_system_package_payload(
    request: &DependencyInventoryRequest,
    snapshot: &SystemPackageProviderSourceSnapshot,
) -> DependencyInventoryObservation {
    let requirements_by_name = request
        .payload
        .requirements
        .iter()
        .map(|requirement| (requirement.name.clone(), requirement))
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for binding in selected_bindings(&request.payload) {
        let Some(requirement) = requirements_by_name.get(&binding.requirement_name) else {
            let diagnostic = diagnostic(
                &request.item,
                DependencyPlanningDiagnosticCode::InvalidRequest,
                "Selected system-package binding references an unknown requirement.".to_string(),
                "dependency_environment.bindings.requirement_name",
            );
            rows.push(row(
                binding.binding_id,
                DependencyInventoryObservationState::Invalid,
                DependencyEnvironmentValidationState::Invalid,
                vec![diagnostic.clone()],
                Vec::new(),
            ));
            diagnostics.push(diagnostic);
            continue;
        };

        let observation =
            observe_system_package_binding(&request.item, &binding, requirement, snapshot);
        diagnostics.extend(observation.diagnostics.iter().cloned());
        rows.push(observation);
    }

    DependencyInventoryObservation::new(rows, diagnostics)
}

fn observe_system_package_binding(
    item: &DependencyReadinessWorkItem,
    binding: &DependencyRequirementBinding,
    requirement: &DependencyRequirement,
    snapshot: &SystemPackageProviderSourceSnapshot,
) -> DependencyInventoryObservationRow {
    let Some(details) = requirement.system_package.as_ref() else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "System-package requirements must include system package details.",
            "dependency_environment.requirements.system_package",
        );
    };
    if details.package_manager_version_constraint.is_some() {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "System-package manager version constraints are not supported by the current inventory source.",
            "dependency_environment.requirements.system_package.package_manager_version_constraint",
        );
    }
    let Ok(key) = key_for_binding(binding, details) else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "System-package binding constraints do not match the requirement.",
            "dependency_environment.bindings.system_package",
        );
    };
    let matching_rows = matching_source_rows(&key, &snapshot.rows);
    if matching_rows.is_empty() {
        return observation_with_diagnostic(
            item,
            binding.binding_id.clone(),
            DependencyInventoryObservationState::Missing,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::ArtifactMissing,
            "System-package source facts are missing for the requested package.",
            "dependency_environment.system_package.source",
            ready_alternatives(&snapshot.rows),
        );
    }
    if let Some(source_row) = matching_rows
        .iter()
        .copied()
        .find(|row| row.state == DependencyProviderSourceState::Ready)
    {
        return observation_from_source_row(item, binding.binding_id.clone(), source_row);
    }
    observation_from_source_row(item, binding.binding_id.clone(), matching_rows[0])
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SystemPackageSourceKey {
    package_id: String,
    package_manager_id: String,
    platform_id: Option<String>,
    architecture: Option<String>,
}

fn key_for_binding(
    binding: &DependencyRequirementBinding,
    requirement: &SystemPackageRequirementDetails,
) -> Result<SystemPackageSourceKey, ()> {
    let binding_details = binding.system_package.as_ref();
    let package_id = merge_optional_constraint(
        Some(requirement.package_id.as_str()),
        binding_details
            .and_then(|details| details.package_id.as_ref())
            .map(|value| value.as_str()),
    )?
    .expect("requirement package id is present");
    let package_manager_id = merge_optional_constraint(
        Some(requirement.package_manager_id.as_str()),
        binding_details
            .and_then(|details| details.package_manager_id.as_ref())
            .map(|value| value.as_str()),
    )?
    .expect("requirement package manager id is present");
    let platform_id = merge_optional_constraint(
        requirement.platform_id.as_ref().map(|value| value.as_str()),
        binding_details
            .and_then(|details| details.platform_id.as_ref())
            .map(|value| value.as_str()),
    )?;
    let architecture = merge_optional_constraint(
        requirement.architecture.as_deref(),
        binding_details.and_then(|details| details.architecture.as_deref()),
    )?;
    Ok(SystemPackageSourceKey {
        package_id: package_id.to_string(),
        package_manager_id: package_manager_id.to_string(),
        platform_id: platform_id.map(ToString::to_string),
        architecture: architecture.map(ToString::to_string),
    })
}

fn merge_optional_constraint<'a>(
    requirement_value: Option<&'a str>,
    binding_value: Option<&'a str>,
) -> Result<Option<&'a str>, ()> {
    match (requirement_value, binding_value) {
        (Some(left), Some(right)) if left != right => Err(()),
        (_, Some(value)) | (Some(value), None) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn matching_source_rows<'a>(
    key: &SystemPackageSourceKey,
    source_rows: &'a [SystemPackageProviderSourceRow],
) -> Vec<&'a SystemPackageProviderSourceRow> {
    source_rows
        .iter()
        .filter(|row| row.package_id.as_str() == key.package_id)
        .filter(|row| row.package_manager_id.as_str() == key.package_manager_id)
        .filter(|row| {
            key.platform_id
                .as_ref()
                .is_none_or(|platform_id| row.platform_id.as_str() == platform_id.as_str())
        })
        .filter(|row| {
            key.architecture.as_ref().is_none_or(|architecture| {
                row.architecture.as_deref() == Some(architecture.as_str())
            })
        })
        .collect()
}

fn observation_from_source_row(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    source_row: &SystemPackageProviderSourceRow,
) -> DependencyInventoryObservationRow {
    match (source_row.state, source_row.freshness) {
        (_, DependencyInventoryObservationFreshness::Stale)
        | (DependencyProviderSourceState::Stale, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Stale,
            DependencyPlanningDiagnosticCode::ArtifactStale,
            "System-package source facts are stale.",
            "dependency_environment.system_package.source",
            source_row.alternatives.clone(),
        ),
        (DependencyProviderSourceState::Ready, _) => row(
            binding_id,
            DependencyInventoryObservationState::Ready,
            DependencyEnvironmentValidationState::Valid,
            source_row.diagnostics.clone(),
            source_row.alternatives.clone(),
        ),
        (DependencyProviderSourceState::Missing, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Missing,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::ArtifactMissing,
            "System-package source facts are missing.",
            "dependency_environment.system_package.source",
            source_row.alternatives.clone(),
        ),
        (DependencyProviderSourceState::Failed, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Failed,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "System-package source reported a failure.",
            "dependency_environment.system_package.source",
            source_row.alternatives.clone(),
        ),
        (DependencyProviderSourceState::Unsupported, _)
        | (DependencyProviderSourceState::Unavailable, _)
        | (DependencyProviderSourceState::Unknown, _)
        | (DependencyProviderSourceState::Probing, _)
        | (DependencyProviderSourceState::Degraded, _)
        | (_, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "System-package source is not ready for the requested package.",
            "dependency_environment.system_package.source",
            source_row.alternatives.clone(),
        ),
    }
}

fn ready_alternatives(
    rows: &[SystemPackageProviderSourceRow],
) -> Vec<pantograph_dependency_planning::DependencyProviderSourceAlternative> {
    rows.iter()
        .filter(|row| row.state == DependencyProviderSourceState::Ready)
        .map(
            |row| pantograph_dependency_planning::DependencyProviderSourceAlternative {
                runtime_id: None,
                runtime_variant_id: None,
                feature_id: None,
                toolchain_id: None,
                device_class: None,
                device_id: None,
                system_package_id: Some(row.package_id.clone()),
                package_manager_id: Some(row.package_manager_id.clone()),
                platform_id: Some(row.platform_id.clone()),
                reason: Some("System package is available on this host platform.".to_string()),
            },
        )
        .take(8)
        .collect()
}

fn observations_from_source_error(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    error: SystemPackageProviderSourceError,
) -> DependencyInventoryObservation {
    let (state, validation_state, code) = match error {
        SystemPackageProviderSourceError::NotImplemented(_) => (
            DependencyInventoryObservationState::NotImplemented,
            DependencyEnvironmentValidationState::NotImplemented,
            DependencyPlanningDiagnosticCode::NotImplemented,
        ),
        SystemPackageProviderSourceError::Unavailable(_) => (
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Unavailable,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
        ),
    };
    source_error_observations(
        item,
        payload,
        state,
        validation_state,
        code,
        error.message().to_string(),
    )
}

fn source_error_observations(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    state: DependencyInventoryObservationState,
    validation_state: DependencyEnvironmentValidationState,
    code: DependencyPlanningDiagnosticCode,
    message: String,
) -> DependencyInventoryObservation {
    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for binding in selected_bindings(payload) {
        let diagnostic = diagnostic(
            item,
            code.clone(),
            message.clone(),
            "dependency_environment.system_package.source",
        );
        rows.push(row(
            binding.binding_id,
            state,
            validation_state,
            vec![diagnostic.clone()],
            Vec::new(),
        ));
        diagnostics.push(diagnostic);
    }
    DependencyInventoryObservation::new(rows, diagnostics)
}

fn invalid_row(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    message: &'static str,
    field_path: &'static str,
) -> DependencyInventoryObservationRow {
    observation_with_diagnostic(
        item,
        binding_id,
        DependencyInventoryObservationState::Invalid,
        DependencyEnvironmentValidationState::Invalid,
        DependencyPlanningDiagnosticCode::InvalidRequest,
        message,
        field_path,
        Vec::new(),
    )
}

fn observation_with_diagnostic(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    state: DependencyInventoryObservationState,
    validation_state: DependencyEnvironmentValidationState,
    code: DependencyPlanningDiagnosticCode,
    message: impl Into<String>,
    field_path: &'static str,
    alternatives: Vec<pantograph_dependency_planning::DependencyProviderSourceAlternative>,
) -> DependencyInventoryObservationRow {
    let diagnostic = diagnostic(item, code, message.into(), field_path);
    row(
        binding_id,
        state,
        validation_state,
        vec![diagnostic],
        alternatives,
    )
}

fn row(
    binding_id: DependencyBindingId,
    state: DependencyInventoryObservationState,
    validation_state: DependencyEnvironmentValidationState,
    diagnostics: Vec<DependencyPlanningDiagnostic>,
    alternatives: Vec<pantograph_dependency_planning::DependencyProviderSourceAlternative>,
) -> DependencyInventoryObservationRow {
    DependencyInventoryObservationRow {
        binding_id,
        state,
        validation_state,
        freshness: DependencyInventoryObservationFreshness::Fresh,
        checked_at_ms: None,
        installed_at_ms: None,
        diagnostics,
        alternatives,
    }
}

fn diagnostic(
    item: &DependencyReadinessWorkItem,
    code: DependencyPlanningDiagnosticCode,
    message: String,
    field_path: &'static str,
) -> DependencyPlanningDiagnostic {
    let request = item.request.as_request();
    DependencyPlanningDiagnostic {
        code,
        severity: DependencyPlanningSeverity::Error,
        message,
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
    }
}

fn selected_bindings(payload: &DependencyRequirementsPayload) -> Vec<DependencyRequirementBinding> {
    let selected_ids = payload.selected_binding_ids.iter().collect::<BTreeSet<_>>();
    payload
        .bindings
        .iter()
        .filter(|binding| selected_ids.contains(&binding.binding_id))
        .cloned()
        .collect()
}
