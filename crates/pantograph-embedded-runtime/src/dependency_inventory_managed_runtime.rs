use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

#[cfg(feature = "standalone")]
use std::path::PathBuf;

use async_trait::async_trait;
use inference::{
    ManagedBinaryId, ManagedBinaryInstallState, ManagedRuntimeReadinessState,
    ManagedRuntimeSnapshot, ManagedRuntimeVersionStatus,
};
use pantograph_dependency_environment_service::{
    DependencyReadinessWorkItem, DependencyRequirementsPayload,
};
use pantograph_dependency_planning::{
    DependencyBindingId, DependencyEnvironmentValidationState,
    DependencyInventoryObservationFreshness, DependencyInventoryObservationRow,
    DependencyInventoryObservationState, DependencyPlanningDiagnostic,
    DependencyPlanningDiagnosticCode, DependencyPlanningSeverity, DependencyRequirement,
    DependencyRequirementBinding, ManagedRuntimeRequirementDetails,
};

use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};

#[async_trait]
pub(crate) trait ManagedRuntimeSnapshotSource: Send + Sync {
    async fn list_snapshots(&self) -> Result<Vec<ManagedRuntimeSnapshot>, String>;
}

#[cfg(feature = "standalone")]
pub(crate) struct BlockingManagedRuntimeSnapshotSource {
    app_data_dir: PathBuf,
}

#[cfg(feature = "standalone")]
impl BlockingManagedRuntimeSnapshotSource {
    #[must_use]
    pub(crate) fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }
}

#[cfg(feature = "standalone")]
#[async_trait]
impl ManagedRuntimeSnapshotSource for BlockingManagedRuntimeSnapshotSource {
    async fn list_snapshots(&self) -> Result<Vec<ManagedRuntimeSnapshot>, String> {
        let app_data_dir = self.app_data_dir.clone();
        tokio::task::spawn_blocking(move || {
            inference::list_managed_runtime_snapshots(&app_data_dir)
        })
        .await
        .map_err(|error| {
            format!("managed-runtime dependency inventory source task failed: {error}")
        })?
    }
}

pub(crate) struct ManagedRuntimeDependencyInventoryProvider {
    source: Arc<dyn ManagedRuntimeSnapshotSource>,
}

impl ManagedRuntimeDependencyInventoryProvider {
    #[must_use]
    pub(crate) fn new(source: Arc<dyn ManagedRuntimeSnapshotSource>) -> Self {
        Self { source }
    }
}

#[async_trait]
impl DependencyInventoryProvider for ManagedRuntimeDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        match self.source.list_snapshots().await {
            Ok(snapshots) => observe_managed_runtime_payload(&request, &snapshots),
            Err(error) => unavailable_observations(
                &request.item,
                &request.payload,
                format!("Managed-runtime inventory source is unavailable: {error}."),
                "dependency_environment.managed_runtime.source",
            ),
        }
    }
}

fn observe_managed_runtime_payload(
    request: &DependencyInventoryRequest,
    snapshots: &[ManagedRuntimeSnapshot],
) -> DependencyInventoryObservation {
    let snapshots_by_id = snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot))
        .collect::<HashMap<_, _>>();
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
                "Selected managed-runtime binding references an unknown requirement.".to_string(),
                "dependency_environment.bindings.requirement_name",
            );
            rows.push(row(
                binding.binding_id,
                DependencyInventoryObservationState::Invalid,
                DependencyEnvironmentValidationState::Invalid,
                vec![diagnostic.clone()],
            ));
            diagnostics.push(diagnostic);
            continue;
        };

        let observation =
            observe_managed_runtime_binding(&request.item, &binding, requirement, &snapshots_by_id);
        diagnostics.extend(observation.diagnostics.iter().cloned());
        rows.push(observation);
    }

    DependencyInventoryObservation::new(rows, diagnostics)
}

fn observe_managed_runtime_binding(
    item: &DependencyReadinessWorkItem,
    binding: &DependencyRequirementBinding,
    requirement: &DependencyRequirement,
    snapshots_by_id: &HashMap<ManagedBinaryId, &ManagedRuntimeSnapshot>,
) -> DependencyInventoryObservationRow {
    let Some(details) = requirement.managed_runtime.as_ref() else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Managed-runtime requirements must include managed runtime details.",
            "dependency_environment.requirements.managed_runtime",
        );
    };
    let Some(managed_binary_id) = managed_binary_id_from_source(&details.managed_binary_id) else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Managed-runtime requirement id is not a supported managed binary id.",
            "dependency_environment.requirements.managed_runtime.managed_binary_id",
        );
    };
    if let Some(binding_details) = binding.managed_runtime.as_ref() {
        if let Some(binding_id) = binding_details.managed_binary_id.as_ref() {
            if binding_id != &details.managed_binary_id {
                return invalid_row(
                    item,
                    binding.binding_id.clone(),
                    "Managed-runtime binding id does not match the requirement id.",
                    "dependency_environment.bindings.managed_runtime.managed_binary_id",
                );
            }
        }
    }
    let Some(snapshot) = snapshots_by_id.get(&managed_binary_id).copied() else {
        return observation_with_diagnostic(
            item,
            binding.binding_id.clone(),
            DependencyInventoryObservationState::Missing,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::ArtifactMissing,
            "Managed-runtime snapshot is missing for the requested managed binary.",
            "dependency_environment.managed_runtime.snapshot",
        );
    };

    match constraints_for_binding(binding, details) {
        Ok(constraints) => observation_from_snapshot(item, binding, snapshot, &constraints),
        Err(error) => invalid_row(
            item,
            binding.binding_id.clone(),
            error.message,
            error.field_path,
        ),
    }
}

#[derive(Debug, Default)]
struct ManagedRuntimeConstraints<'a> {
    version: Option<&'a str>,
    runtime_variant_id: Option<&'a str>,
    platform_key: Option<&'a str>,
}

impl ManagedRuntimeConstraints<'_> {
    fn has_version_scope(&self) -> bool {
        self.version.is_some() || self.runtime_variant_id.is_some() || self.platform_key.is_some()
    }
}

#[derive(Debug)]
struct ConstraintError {
    message: &'static str,
    field_path: &'static str,
}

fn constraints_for_binding<'a>(
    binding: &'a DependencyRequirementBinding,
    requirement: &'a ManagedRuntimeRequirementDetails,
) -> Result<ManagedRuntimeConstraints<'a>, ConstraintError> {
    let binding_details = binding.managed_runtime.as_ref();
    Ok(ManagedRuntimeConstraints {
        version: merge_optional_constraint(
            requirement.version.as_deref(),
            binding_details.and_then(|details| details.selected_version.as_deref()),
            "Managed-runtime binding selected version does not match the requirement version.",
            "dependency_environment.bindings.managed_runtime.selected_version",
        )?,
        runtime_variant_id: merge_optional_constraint(
            requirement
                .runtime_variant_id
                .as_ref()
                .map(|value| value.as_str()),
            binding_details
                .and_then(|details| details.runtime_variant_id.as_ref())
                .map(|value| value.as_str()),
            "Managed-runtime binding runtime variant does not match the requirement runtime variant.",
            "dependency_environment.bindings.managed_runtime.runtime_variant_id",
        )?,
        platform_key: merge_optional_constraint(
            requirement.platform_key.as_deref(),
            binding_details.and_then(|details| details.platform_key.as_deref()),
            "Managed-runtime binding platform key does not match the requirement platform key.",
            "dependency_environment.bindings.managed_runtime.platform_key",
        )?,
    })
}

fn merge_optional_constraint<'a>(
    requirement_value: Option<&'a str>,
    binding_value: Option<&'a str>,
    message: &'static str,
    field_path: &'static str,
) -> Result<Option<&'a str>, ConstraintError> {
    match (requirement_value, binding_value) {
        (Some(left), Some(right)) if left != right => Err(ConstraintError {
            message,
            field_path,
        }),
        (_, Some(value)) | (Some(value), None) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn observation_from_snapshot(
    item: &DependencyReadinessWorkItem,
    binding: &DependencyRequirementBinding,
    snapshot: &ManagedRuntimeSnapshot,
    constraints: &ManagedRuntimeConstraints<'_>,
) -> DependencyInventoryObservationRow {
    if constraints.has_version_scope() {
        let versions = matching_versions(snapshot, constraints);
        match versions.as_slice() {
            [] => observation_with_diagnostic(
                item,
                binding.binding_id.clone(),
                DependencyInventoryObservationState::Missing,
                DependencyEnvironmentValidationState::Valid,
                DependencyPlanningDiagnosticCode::ArtifactMissing,
                "Managed-runtime version, variant, or platform constraint did not match an available runtime version.",
                "dependency_environment.managed_runtime.version",
            ),
            [version] => observation_from_state(
                item,
                binding.binding_id.clone(),
                version.readiness_state,
                snapshot.available && version.executable_ready,
                version.install_state,
            ),
            _ => invalid_row(
                item,
                binding.binding_id.clone(),
                "Managed-runtime version, variant, or platform constraint matched multiple runtime versions.",
                "dependency_environment.managed_runtime.version",
            ),
        }
    } else {
        observation_from_state(
            item,
            binding.binding_id.clone(),
            snapshot.readiness_state,
            snapshot.available,
            snapshot.install_state,
        )
    }
}

fn matching_versions<'a>(
    snapshot: &'a ManagedRuntimeSnapshot,
    constraints: &ManagedRuntimeConstraints<'_>,
) -> Vec<&'a ManagedRuntimeVersionStatus> {
    snapshot
        .versions
        .iter()
        .filter(|version| {
            version.runtime_key == snapshot.id.key()
                && constraints
                    .version
                    .is_none_or(|expected| version.version.as_deref() == Some(expected))
                && constraints
                    .runtime_variant_id
                    .is_none_or(|expected| version.runtime_variant_id.as_str() == expected)
                && constraints
                    .platform_key
                    .is_none_or(|expected| version.platform_key == expected)
        })
        .collect()
}

fn observation_from_state(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    readiness_state: ManagedRuntimeReadinessState,
    available: bool,
    install_state: ManagedBinaryInstallState,
) -> DependencyInventoryObservationRow {
    match readiness_state {
        ManagedRuntimeReadinessState::Ready if available => row(
            binding_id,
            DependencyInventoryObservationState::Ready,
            DependencyEnvironmentValidationState::Valid,
            Vec::new(),
        ),
        ManagedRuntimeReadinessState::Ready | ManagedRuntimeReadinessState::Missing
            if install_state == ManagedBinaryInstallState::Missing =>
        {
            observation_with_diagnostic(
                item,
                binding_id,
                DependencyInventoryObservationState::Missing,
                DependencyEnvironmentValidationState::Valid,
                DependencyPlanningDiagnosticCode::ArtifactMissing,
                "Managed runtime is missing or its executable is not ready.",
                "dependency_environment.managed_runtime.install_state",
            )
        }
        ManagedRuntimeReadinessState::Downloading
        | ManagedRuntimeReadinessState::Extracting
        | ManagedRuntimeReadinessState::Validating
        | ManagedRuntimeReadinessState::Unknown => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Unavailable,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "Managed runtime is not in a terminal ready state.",
            "dependency_environment.managed_runtime.readiness_state",
        ),
        ManagedRuntimeReadinessState::Failed => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Failed,
            DependencyEnvironmentValidationState::Unavailable,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "Managed runtime readiness check failed.",
            "dependency_environment.managed_runtime.readiness_state",
        ),
        ManagedRuntimeReadinessState::Unsupported
        | ManagedRuntimeReadinessState::Ready
        | ManagedRuntimeReadinessState::Missing => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Unavailable,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "Managed runtime is unavailable or unsupported on this host.",
            "dependency_environment.managed_runtime.readiness_state",
        ),
    }
}

fn unavailable_observations(
    item: &DependencyReadinessWorkItem,
    payload: &DependencyRequirementsPayload,
    message: String,
    field_path: &'static str,
) -> DependencyInventoryObservation {
    let diagnostic = diagnostic(
        item,
        DependencyPlanningDiagnosticCode::RuntimeUnavailable,
        message,
        field_path,
    );
    let rows = selected_bindings(payload)
        .into_iter()
        .map(|binding| {
            row(
                binding.binding_id,
                DependencyInventoryObservationState::Unavailable,
                DependencyEnvironmentValidationState::Unavailable,
                vec![diagnostic.clone()],
            )
        })
        .collect::<Vec<_>>();
    DependencyInventoryObservation::new(rows, vec![diagnostic])
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
) -> DependencyInventoryObservationRow {
    let diagnostic = diagnostic(item, code, message.into(), field_path);
    row(binding_id, state, validation_state, vec![diagnostic])
}

fn row(
    binding_id: DependencyBindingId,
    state: DependencyInventoryObservationState,
    validation_state: DependencyEnvironmentValidationState,
    diagnostics: Vec<DependencyPlanningDiagnostic>,
) -> DependencyInventoryObservationRow {
    DependencyInventoryObservationRow {
        binding_id,
        state,
        validation_state,
        freshness: DependencyInventoryObservationFreshness::Fresh,
        checked_at_ms: None,
        installed_at_ms: None,
        diagnostics,
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

fn managed_binary_id_from_source(
    source_id: &pantograph_dependency_planning::ManagedRuntimeSourceId,
) -> Option<ManagedBinaryId> {
    ManagedBinaryId::all()
        .iter()
        .copied()
        .find(|id| id.key() == source_id.as_str())
}
