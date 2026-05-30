use std::collections::{BTreeMap, BTreeSet};

use inference::CapabilityAvailabilityId;
use pantograph_dependency_environment_service::{
    DependencyReadinessWorkItem, DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentId, DependencyEnvironmentRef, DependencyEnvironmentValidationState,
    DependencyInventoryObservationFreshness, DependencyInventoryObservationRow,
    DependencyInventoryObservationState, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningSeverity, DependencyRequirement,
    DependencyRequirementBinding,
};

use crate::dependency_environment_probe_selector::ProbeShapeError;
use crate::dependency_readiness::PythonPackageReadinessSnapshot;
use crate::package_readiness_provider::{
    PackageReadinessProbeFailure, PackageReadinessProbeOutcome,
    PackageReadinessProviderDiagnosticCode,
};

const DEFAULT_PYTHON_ENVIRONMENT_ID: &str = "python:default-host";

pub(crate) fn dependency_inventory_observations_from_probe_outcome(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    outcome: PackageReadinessProbeOutcome,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    match outcome {
        PackageReadinessProbeOutcome::Snapshot(snapshot) => {
            observations_from_python_snapshot(item, payload, snapshot)
        }
        PackageReadinessProbeOutcome::Failed(failures) => {
            observations_from_probe_failures(item, payload, failures)
        }
    }
}

fn observations_from_python_snapshot(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    snapshot: PythonPackageReadinessSnapshot,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    if !snapshot.python_available {
        return observations_from_diagnostic(
            item,
            payload,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            snapshot
                .unavailable_reason
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "Python runtime is not available.".to_string()),
            "dependency_environment.probe.python",
        );
    }

    let requirement_by_name = payload
        .requirements
        .iter()
        .map(|requirement| (requirement.name.clone(), requirement))
        .collect::<BTreeMap<_, _>>();
    let selected_bindings = selected_bindings(payload);
    let rows = selected_bindings
        .iter()
        .map(|binding| observation_from_python_snapshot(binding, &requirement_by_name, &snapshot))
        .collect::<Vec<_>>();
    (rows, Vec::new())
}

pub(crate) fn observations_from_probe_failures(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    failures: Vec<PackageReadinessProbeFailure>,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    let diagnostic = failures.first().map_or_else(
        || {
            diagnostic(
                item,
                DependencyPlanningDiagnosticCode::RuntimeUnavailable,
                "Dependency readiness probe failed without details.".to_string(),
                "dependency_environment.probe",
            )
        },
        |failure| diagnostic_from_probe_failure(item, failure),
    );
    let rows = selected_bindings(payload)
        .iter()
        .map(|binding| DependencyInventoryObservationRow {
            binding_id: binding.binding_id.clone(),
            state: observation_state_for_probe_failure(
                failures
                    .first()
                    .map(|failure| failure.code)
                    .unwrap_or(PackageReadinessProviderDiagnosticCode::ProbeProcessFailed),
            ),
            validation_state: observation_validation_state_for_probe_failure(
                failures
                    .first()
                    .map(|failure| failure.code)
                    .unwrap_or(PackageReadinessProviderDiagnosticCode::ProbeProcessFailed),
            ),
            freshness: DependencyInventoryObservationFreshness::Fresh,
            checked_at_ms: None,
            installed_at_ms: None,
            diagnostics: vec![diagnostic.clone()],
            alternatives: Vec::new(),
        })
        .collect();
    (rows, vec![diagnostic])
}

fn observation_from_python_snapshot(
    binding: &DependencyRequirementBinding,
    requirement_by_name: &BTreeMap<
        pantograph_dependency_planning::DependencyRequirementName,
        &DependencyRequirement,
    >,
    snapshot: &PythonPackageReadinessSnapshot,
) -> DependencyInventoryObservationRow {
    let installed = requirement_by_name
        .get(&binding.requirement_name)
        .and_then(|requirement| CapabilityAvailabilityId::parse(requirement.name.as_str()).ok())
        .is_some_and(|dependency_id| snapshot.installed_package_ids.contains(&dependency_id));
    DependencyInventoryObservationRow {
        binding_id: binding.binding_id.clone(),
        state: if installed {
            DependencyInventoryObservationState::Ready
        } else {
            DependencyInventoryObservationState::Missing
        },
        validation_state: DependencyEnvironmentValidationState::Valid,
        freshness: DependencyInventoryObservationFreshness::Fresh,
        checked_at_ms: None,
        installed_at_ms: None,
        diagnostics: Vec::new(),
        alternatives: Vec::new(),
    }
}
pub(crate) fn invalid_probe_shape_observations(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    error: ProbeShapeError,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    let request = item.request.as_request();
    let diagnostic = DependencyPlanningDiagnostic {
        code: DependencyPlanningDiagnosticCode::InvalidRequest,
        severity: DependencyPlanningSeverity::Error,
        message: error.message.to_string(),
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
        field_path: Some(error.field_path.to_string()),
    };
    observations_from_row_state(
        payload,
        DependencyInventoryObservationState::Invalid,
        DependencyEnvironmentValidationState::Invalid,
        diagnostic,
    )
}

fn observations_from_diagnostic(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    diagnostic_code: DependencyPlanningDiagnosticCode,
    message: String,
    field_path: &'static str,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    let diagnostic = diagnostic(item, diagnostic_code, message, field_path);
    observations_from_row_state(
        payload,
        DependencyInventoryObservationState::Unavailable,
        DependencyEnvironmentValidationState::Unavailable,
        diagnostic,
    )
}

fn observations_from_row_state(
    payload: &DependencyRequirementsPayload,
    state: DependencyInventoryObservationState,
    validation_state: DependencyEnvironmentValidationState,
    diagnostic: DependencyPlanningDiagnostic,
) -> (
    Vec<DependencyInventoryObservationRow>,
    Vec<DependencyPlanningDiagnostic>,
) {
    let rows = selected_bindings(payload)
        .iter()
        .map(|binding| DependencyInventoryObservationRow {
            binding_id: binding.binding_id.clone(),
            state,
            validation_state,
            freshness: DependencyInventoryObservationFreshness::Fresh,
            checked_at_ms: None,
            installed_at_ms: None,
            diagnostics: vec![diagnostic.clone()],
            alternatives: Vec::new(),
        })
        .collect();
    (rows, vec![diagnostic])
}

fn diagnostic_from_probe_failure(
    item: &DependencyReadinessWorkItem,
    failure: &PackageReadinessProbeFailure,
) -> DependencyPlanningDiagnostic {
    diagnostic(
        item,
        diagnostic_code_for_probe_failure(failure.code),
        failure.reason.to_string(),
        field_path_for_probe_failure(failure.code),
    )
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

fn diagnostic_code_for_probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
) -> DependencyPlanningDiagnosticCode {
    match code {
        PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            DependencyPlanningDiagnosticCode::RuntimeUnavailable
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            DependencyPlanningDiagnosticCode::ArtifactMissing
        }
        PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind
        | PackageReadinessProviderDiagnosticCode::InvalidPackageId => {
            DependencyPlanningDiagnosticCode::InvalidRequest
        }
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented
        | PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            DependencyPlanningDiagnosticCode::NotImplemented
        }
    }
}

fn field_path_for_probe_failure(code: PackageReadinessProviderDiagnosticCode) -> &'static str {
    match code {
        PackageReadinessProviderDiagnosticCode::InvalidPackageId => {
            "dependency_environment.requirements.name"
        }
        PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            "dependency_environment.requirements.kind"
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            "dependency_environment.requirements"
        }
        PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::ProbeNotImplemented
        | PackageReadinessProviderDiagnosticCode::UnsupportedPlatform
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            "dependency_environment.probe"
        }
    }
}

fn observation_state_for_probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
) -> DependencyInventoryObservationState {
    match code {
        PackageReadinessProviderDiagnosticCode::InvalidPackageId
        | PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            DependencyInventoryObservationState::Invalid
        }
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented
        | PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            DependencyInventoryObservationState::NotImplemented
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            DependencyInventoryObservationState::Missing
        }
        PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            DependencyInventoryObservationState::Unavailable
        }
    }
}

fn observation_validation_state_for_probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
) -> DependencyEnvironmentValidationState {
    match code {
        PackageReadinessProviderDiagnosticCode::InvalidPackageId
        | PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            DependencyEnvironmentValidationState::Invalid
        }
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented
        | PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            DependencyEnvironmentValidationState::NotImplemented
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            DependencyEnvironmentValidationState::Valid
        }
        PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            DependencyEnvironmentValidationState::Unavailable
        }
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

pub(crate) fn environment_ref_for_request(
    item: &DependencyReadinessWorkItem,
) -> DependencyEnvironmentRef {
    item.request
        .as_request()
        .environment_ref
        .clone()
        .unwrap_or_else(default_python_environment_ref)
}

fn default_python_environment_ref() -> DependencyEnvironmentRef {
    DependencyEnvironmentRef {
        environment_id: DependencyEnvironmentId::parse(DEFAULT_PYTHON_ENVIRONMENT_ID)
            .expect("default python environment id is valid"),
        manifest_id: None,
    }
}
