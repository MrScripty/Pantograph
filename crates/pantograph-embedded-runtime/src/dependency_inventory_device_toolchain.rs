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
    DependencyRequirement, DependencyRequirementBinding, DeviceToolchainProviderSourceRow,
    DeviceToolchainProviderSourceSnapshot, DeviceToolchainRequirementDetails,
    ValidatedDeviceToolchainProviderSourceSnapshot,
};

use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};
use crate::dependency_inventory_device_toolchain_source::DeviceToolchainProviderSource;

pub(crate) struct DeviceToolchainDependencyInventoryProvider {
    source: Arc<dyn DeviceToolchainProviderSource>,
}

impl DeviceToolchainDependencyInventoryProvider {
    #[must_use]
    pub(crate) fn new(source: Arc<dyn DeviceToolchainProviderSource>) -> Self {
        Self { source }
    }
}

#[async_trait]
impl DependencyInventoryProvider for DeviceToolchainDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        match self.source.snapshot().await {
            Ok(snapshot) => {
                match ValidatedDeviceToolchainProviderSourceSnapshot::try_from(snapshot) {
                    Ok(snapshot) => {
                        observe_device_toolchain_payload(&request, snapshot.as_snapshot())
                    }
                    Err(error) => unavailable_observations(
                        &request.item,
                        &request.payload,
                        format!("Device-toolchain inventory source is invalid: {error}."),
                        "dependency_environment.device_toolchain.source",
                    ),
                }
            }
            Err(error) => unavailable_observations(
                &request.item,
                &request.payload,
                format!("Device-toolchain inventory source is unavailable: {error}."),
                "dependency_environment.device_toolchain.source",
            ),
        }
    }
}

fn observe_device_toolchain_payload(
    request: &DependencyInventoryRequest,
    snapshot: &DeviceToolchainProviderSourceSnapshot,
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
                "Selected device-toolchain binding references an unknown requirement.".to_string(),
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
            observe_device_toolchain_binding(&request.item, &binding, requirement, snapshot);
        diagnostics.extend(observation.diagnostics.iter().cloned());
        rows.push(observation);
    }

    DependencyInventoryObservation::new(rows, diagnostics)
}

fn observe_device_toolchain_binding(
    item: &DependencyReadinessWorkItem,
    binding: &DependencyRequirementBinding,
    requirement: &DependencyRequirement,
    snapshot: &DeviceToolchainProviderSourceSnapshot,
) -> DependencyInventoryObservationRow {
    let Some(details) = requirement.device_toolchain.as_ref() else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Device-toolchain requirements must include device toolchain details.",
            "dependency_environment.requirements.device_toolchain",
        );
    };
    let Ok(key) = key_for_binding(binding, details) else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Device-toolchain binding constraints do not match the requirement.",
            "dependency_environment.bindings.device_toolchain",
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
            "Device-toolchain source facts are missing for the requested toolchain.",
            "dependency_environment.device_toolchain.source",
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
struct DeviceToolchainSourceKey {
    toolchain_id: String,
    runtime_id: Option<String>,
    device_id: Option<String>,
}

fn key_for_binding(
    binding: &DependencyRequirementBinding,
    requirement: &DeviceToolchainRequirementDetails,
) -> Result<DeviceToolchainSourceKey, ()> {
    let binding_details = binding.device_toolchain.as_ref();
    let toolchain_id = merge_optional_constraint(
        Some(requirement.toolchain_id.as_str()),
        binding_details
            .and_then(|details| details.toolchain_id.as_ref())
            .map(|value| value.as_str()),
    )?
    .expect("requirement toolchain id is present");
    let runtime_id = merge_optional_constraint(
        requirement.runtime_id.as_ref().map(|value| value.as_str()),
        binding_details
            .and_then(|details| details.runtime_id.as_ref())
            .map(|value| value.as_str()),
    )?;
    let device_id = merge_optional_constraint(
        requirement.device_id.as_ref().map(|value| value.as_str()),
        binding_details
            .and_then(|details| details.device_id.as_ref())
            .map(|value| value.as_str()),
    )?;
    Ok(DeviceToolchainSourceKey {
        toolchain_id: toolchain_id.to_string(),
        runtime_id: runtime_id.map(ToString::to_string),
        device_id: device_id.map(ToString::to_string),
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
    key: &DeviceToolchainSourceKey,
    source_rows: &'a [DeviceToolchainProviderSourceRow],
) -> Vec<&'a DeviceToolchainProviderSourceRow> {
    source_rows
        .iter()
        .filter(|row| row.toolchain_id.as_str() == key.toolchain_id)
        .filter(|row| {
            key.runtime_id.as_ref().is_none_or(|runtime_id| {
                row.runtime_id.as_ref().map(|value| value.as_str()) == Some(runtime_id.as_str())
            })
        })
        .filter(|row| {
            key.device_id.as_ref().is_none_or(|device_id| {
                row.device_id.as_ref().map(|value| value.as_str()) == Some(device_id.as_str())
            })
        })
        .collect()
}

fn observation_from_source_row(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    source_row: &DeviceToolchainProviderSourceRow,
) -> DependencyInventoryObservationRow {
    match (source_row.state, source_row.freshness) {
        (_, DependencyInventoryObservationFreshness::Stale)
        | (DependencyProviderSourceState::Stale, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Stale,
            DependencyPlanningDiagnosticCode::ArtifactStale,
            "Device-toolchain source facts are stale.",
            "dependency_environment.device_toolchain.source",
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
            "Device-toolchain source facts are missing.",
            "dependency_environment.device_toolchain.source",
            source_row.alternatives.clone(),
        ),
        (DependencyProviderSourceState::Failed, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Failed,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "Device-toolchain source reported a failure.",
            "dependency_environment.device_toolchain.source",
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
            "Device-toolchain source is not ready for the requested toolchain.",
            "dependency_environment.device_toolchain.source",
            source_row.alternatives.clone(),
        ),
    }
}

fn ready_alternatives(
    rows: &[DeviceToolchainProviderSourceRow],
) -> Vec<pantograph_dependency_planning::DependencyProviderSourceAlternative> {
    rows.iter()
        .filter(|row| row.state == DependencyProviderSourceState::Ready)
        .map(
            |row| pantograph_dependency_planning::DependencyProviderSourceAlternative {
                runtime_id: row.runtime_id.clone(),
                runtime_variant_id: None,
                feature_id: None,
                toolchain_id: Some(row.toolchain_id.clone()),
                device_class: row.device_class.clone(),
                device_id: row.device_id.clone(),
                system_package_id: None,
                package_manager_id: None,
                platform_id: None,
                reason: Some("Device toolchain is available on this runtime.".to_string()),
            },
        )
        .take(8)
        .collect()
}

fn unavailable_observations(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    message: String,
    field_path: &'static str,
) -> DependencyInventoryObservation {
    let mut rows = Vec::new();
    let mut diagnostics = Vec::new();
    for binding in selected_bindings(payload) {
        let diagnostic = diagnostic(
            item,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            message.clone(),
            field_path,
        );
        rows.push(row(
            binding.binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Valid,
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
