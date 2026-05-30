use std::collections::{BTreeMap, BTreeSet};

use inference::CapabilityAvailabilityId;
use pantograph_dependency_environment_service::{
    DependencyReadinessWorkItem, DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyBindingStatusRow, DependencyBindingStatusState, DependencyEnvironmentFailureState,
    DependencyEnvironmentId, DependencyEnvironmentInstallState, DependencyEnvironmentOperation,
    DependencyEnvironmentOperationState, DependencyEnvironmentReadinessState,
    DependencyEnvironmentRef, DependencyEnvironmentResult, DependencyEnvironmentValidationState,
    DependencyPlanningDiagnostic, DependencyPlanningDiagnosticCode, DependencyPlanningSeverity,
    DependencyRequirement, DependencyRequirementBinding,
};

use crate::dependency_environment_probe_selector::ProbeShapeError;
use crate::dependency_readiness::PythonPackageReadinessSnapshot;
use crate::package_readiness_provider::{
    PackageReadinessProbeFailure, PackageReadinessProbeOutcome,
    PackageReadinessProviderDiagnosticCode,
};

const DEFAULT_PYTHON_ENVIRONMENT_ID: &str = "python:default-host";

pub(crate) fn dependency_environment_result_from_probe_outcome(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    outcome: PackageReadinessProbeOutcome,
) -> DependencyEnvironmentResult {
    match outcome {
        PackageReadinessProbeOutcome::Snapshot(snapshot) => {
            result_from_python_snapshot(item, payload, snapshot)
        }
        PackageReadinessProbeOutcome::Failed(failures) => {
            result_from_probe_failures(item, payload, failures)
        }
    }
}
fn result_from_python_snapshot(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    snapshot: PythonPackageReadinessSnapshot,
) -> DependencyEnvironmentResult {
    if !snapshot.python_available {
        return unavailable_probe_result(
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
    let selected_bindings = selected_bindings(&payload);
    let statuses = selected_bindings
        .iter()
        .map(|binding| {
            binding_status_from_python_snapshot(binding, &requirement_by_name, &snapshot)
        })
        .collect::<Vec<_>>();
    let all_ready = statuses
        .iter()
        .all(|status| status.state == DependencyBindingStatusState::Ready);
    let request = item.request.as_request();

    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: if all_ready {
            DependencyEnvironmentReadinessState::Ready
        } else {
            DependencyEnvironmentReadinessState::Missing
        },
        install_state: if all_ready {
            DependencyEnvironmentInstallState::Installed
        } else {
            DependencyEnvironmentInstallState::NotInstalled
        },
        validation_state: DependencyEnvironmentValidationState::Valid,
        failure_state: None,
        dependency_requirements_id: Some(payload.dependency_requirements_id.clone()),
        environment_ref: Some(environment_ref_for_request(item)),
        requirements: payload.requirements,
        bindings: payload.bindings,
        selected_binding_ids: payload.selected_binding_ids,
        binding_statuses: statuses,
        operation: Some(DependencyEnvironmentOperation {
            state: if all_ready {
                DependencyEnvironmentOperationState::Succeeded
            } else {
                DependencyEnvironmentOperationState::Blocked
            },
            started_at_ms: None,
            completed_at_ms: None,
        }),
        validation_errors: Vec::new(),
        diagnostics: Vec::new(),
    }
}
fn result_from_probe_failures(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    failures: Vec<PackageReadinessProbeFailure>,
) -> DependencyEnvironmentResult {
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
    let statuses = selected_bindings(&payload)
        .iter()
        .map(|binding| DependencyBindingStatusRow {
            binding_id: binding.binding_id.clone(),
            state: status_state_for_probe_failure(
                failures
                    .first()
                    .map(|failure| failure.code)
                    .unwrap_or(PackageReadinessProviderDiagnosticCode::ProbeProcessFailed),
            ),
            validation_state: DependencyEnvironmentValidationState::Unavailable,
            checked_at_ms: None,
            installed_at_ms: None,
            diagnostics: vec![diagnostic.clone()],
        })
        .collect();
    diagnostic_probe_result(item, payload, diagnostic, statuses)
}
fn binding_status_from_python_snapshot(
    binding: &DependencyRequirementBinding,
    requirement_by_name: &BTreeMap<
        pantograph_dependency_planning::DependencyRequirementName,
        &DependencyRequirement,
    >,
    snapshot: &PythonPackageReadinessSnapshot,
) -> DependencyBindingStatusRow {
    let installed = requirement_by_name
        .get(&binding.requirement_name)
        .and_then(|requirement| CapabilityAvailabilityId::parse(requirement.name.as_str()).ok())
        .is_some_and(|dependency_id| snapshot.installed_package_ids.contains(&dependency_id));
    DependencyBindingStatusRow {
        binding_id: binding.binding_id.clone(),
        state: if installed {
            DependencyBindingStatusState::Ready
        } else {
            DependencyBindingStatusState::Missing
        },
        validation_state: DependencyEnvironmentValidationState::Valid,
        checked_at_ms: None,
        installed_at_ms: None,
        diagnostics: Vec::new(),
    }
}
pub(crate) fn invalid_probe_shape_result(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    error: ProbeShapeError,
) -> DependencyEnvironmentResult {
    let request = item.request.as_request();
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Invalid,
        install_state: DependencyEnvironmentInstallState::Blocked,
        validation_state: DependencyEnvironmentValidationState::Invalid,
        failure_state: Some(DependencyEnvironmentFailureState::InvalidRequest),
        dependency_requirements_id: Some(payload.dependency_requirements_id.clone()),
        environment_ref: Some(environment_ref_for_request(item)),
        requirements: payload.requirements.clone(),
        bindings: payload.bindings.clone(),
        selected_binding_ids: payload.selected_binding_ids.clone(),
        binding_statuses: Vec::new(),
        operation: Some(DependencyEnvironmentOperation {
            state: DependencyEnvironmentOperationState::Blocked,
            started_at_ms: None,
            completed_at_ms: None,
        }),
        validation_errors: Vec::new(),
        diagnostics: vec![DependencyPlanningDiagnostic {
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
        }],
    }
}

fn unavailable_probe_result(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    diagnostic_code: DependencyPlanningDiagnosticCode,
    message: String,
    field_path: &'static str,
) -> DependencyEnvironmentResult {
    let diagnostic = diagnostic(item, diagnostic_code, message, field_path);
    diagnostic_probe_result(item, payload, diagnostic, Vec::new())
}

fn diagnostic_probe_result(
    item: &DependencyReadinessWorkItem,
    payload: DependencyRequirementsPayload,
    diagnostic: DependencyPlanningDiagnostic,
    binding_statuses: Vec<DependencyBindingStatusRow>,
) -> DependencyEnvironmentResult {
    let request = item.request.as_request();
    DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Unavailable,
        install_state: DependencyEnvironmentInstallState::Blocked,
        validation_state: DependencyEnvironmentValidationState::Unavailable,
        failure_state: Some(DependencyEnvironmentFailureState::EnvironmentUnavailable),
        dependency_requirements_id: Some(payload.dependency_requirements_id.clone()),
        environment_ref: Some(environment_ref_for_request(item)),
        requirements: payload.requirements,
        bindings: payload.bindings,
        selected_binding_ids: payload.selected_binding_ids,
        binding_statuses,
        operation: Some(DependencyEnvironmentOperation {
            state: DependencyEnvironmentOperationState::Blocked,
            started_at_ms: None,
            completed_at_ms: None,
        }),
        validation_errors: Vec::new(),
        diagnostics: vec![diagnostic],
    }
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

fn status_state_for_probe_failure(
    code: PackageReadinessProviderDiagnosticCode,
) -> DependencyBindingStatusState {
    match code {
        PackageReadinessProviderDiagnosticCode::InvalidPackageId
        | PackageReadinessProviderDiagnosticCode::UnsupportedDependencyKind => {
            DependencyBindingStatusState::Invalid
        }
        PackageReadinessProviderDiagnosticCode::ProbeNotImplemented
        | PackageReadinessProviderDiagnosticCode::UnsupportedPlatform => {
            DependencyBindingStatusState::NotImplemented
        }
        PackageReadinessProviderDiagnosticCode::MissingPackage => {
            DependencyBindingStatusState::Missing
        }
        PackageReadinessProviderDiagnosticCode::PythonUnavailable
        | PackageReadinessProviderDiagnosticCode::ProbeTimedOut
        | PackageReadinessProviderDiagnosticCode::ProbeProcessFailed => {
            DependencyBindingStatusState::Unavailable
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

fn environment_ref_for_request(item: &DependencyReadinessWorkItem) -> DependencyEnvironmentRef {
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
