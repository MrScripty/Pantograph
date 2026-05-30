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
    DependencyRequirement, DependencyRequirementBinding, RuntimeFeatureProviderSourceRow,
    RuntimeFeatureProviderSourceSnapshot, RuntimeFeatureRequirementDetails,
    ValidatedRuntimeFeatureProviderSourceSnapshot,
};

use crate::dependency_inventory::{
    DependencyInventoryObservation, DependencyInventoryProvider, DependencyInventoryRequest,
};
use crate::dependency_inventory_runtime_feature_source::RuntimeFeatureProviderSource;

pub(crate) struct RuntimeFeatureDependencyInventoryProvider {
    source: Arc<dyn RuntimeFeatureProviderSource>,
}

impl RuntimeFeatureDependencyInventoryProvider {
    #[must_use]
    pub(crate) fn new(source: Arc<dyn RuntimeFeatureProviderSource>) -> Self {
        Self { source }
    }
}

#[async_trait]
impl DependencyInventoryProvider for RuntimeFeatureDependencyInventoryProvider {
    async fn observe(&self, request: DependencyInventoryRequest) -> DependencyInventoryObservation {
        match self.source.snapshot().await {
            Ok(snapshot) => match ValidatedRuntimeFeatureProviderSourceSnapshot::try_from(snapshot)
            {
                Ok(snapshot) => observe_runtime_feature_payload(&request, snapshot.as_snapshot()),
                Err(error) => unavailable_observations(
                    &request.item,
                    &request.payload,
                    format!("Runtime-feature inventory source is invalid: {error}."),
                    "dependency_environment.runtime_feature.source",
                ),
            },
            Err(error) => unavailable_observations(
                &request.item,
                &request.payload,
                format!("Runtime-feature inventory source is unavailable: {error}."),
                "dependency_environment.runtime_feature.source",
            ),
        }
    }
}

fn observe_runtime_feature_payload(
    request: &DependencyInventoryRequest,
    snapshot: &RuntimeFeatureProviderSourceSnapshot,
) -> DependencyInventoryObservation {
    let source_rows = snapshot
        .rows
        .iter()
        .map(|row| (source_key(row), row))
        .collect::<BTreeMap<_, _>>();
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
                "Selected runtime-feature binding references an unknown requirement.".to_string(),
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
            observe_runtime_feature_binding(&request.item, &binding, requirement, &source_rows);
        diagnostics.extend(observation.diagnostics.iter().cloned());
        rows.push(observation);
    }

    DependencyInventoryObservation::new(rows, diagnostics)
}

fn observe_runtime_feature_binding(
    item: &DependencyReadinessWorkItem,
    binding: &DependencyRequirementBinding,
    requirement: &DependencyRequirement,
    source_rows: &BTreeMap<RuntimeFeatureSourceKey, &RuntimeFeatureProviderSourceRow>,
) -> DependencyInventoryObservationRow {
    let Some(details) = requirement.runtime_feature.as_ref() else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Runtime-feature requirements must include runtime feature details.",
            "dependency_environment.requirements.runtime_feature",
        );
    };
    let Ok(key) = key_for_binding(binding, details) else {
        return invalid_row(
            item,
            binding.binding_id.clone(),
            "Runtime-feature binding constraints do not match the requirement.",
            "dependency_environment.bindings.runtime_feature",
        );
    };
    let Some(source_row) = source_rows.get(&key).copied() else {
        return observation_with_diagnostic(
            item,
            binding.binding_id.clone(),
            DependencyInventoryObservationState::Missing,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::ArtifactMissing,
            "Runtime-feature source facts are missing for the requested runtime feature.",
            "dependency_environment.runtime_feature.source",
        );
    };
    observation_from_source_row(item, binding.binding_id.clone(), source_row)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RuntimeFeatureSourceKey {
    runtime_id: String,
    feature_id: String,
    runtime_variant_id: Option<String>,
}

fn source_key(row: &RuntimeFeatureProviderSourceRow) -> RuntimeFeatureSourceKey {
    RuntimeFeatureSourceKey {
        runtime_id: row.runtime_id.as_str().to_string(),
        feature_id: row.feature_id.as_str().to_string(),
        runtime_variant_id: row
            .runtime_variant_id
            .as_ref()
            .map(|runtime_variant_id| runtime_variant_id.as_str().to_string()),
    }
}

fn key_for_binding(
    binding: &DependencyRequirementBinding,
    requirement: &RuntimeFeatureRequirementDetails,
) -> Result<RuntimeFeatureSourceKey, ()> {
    let binding_details = binding.runtime_feature.as_ref();
    let runtime_id = merge_optional_constraint(
        Some(requirement.runtime_id.as_str()),
        binding_details
            .and_then(|details| details.runtime_id.as_ref())
            .map(|value| value.as_str()),
    )?
    .expect("requirement runtime id is present");
    let feature_id = merge_optional_constraint(
        Some(requirement.feature_id.as_str()),
        binding_details
            .and_then(|details| details.feature_id.as_ref())
            .map(|value| value.as_str()),
    )?
    .expect("requirement feature id is present");
    let runtime_variant_id = merge_optional_constraint(
        requirement
            .runtime_variant_id
            .as_ref()
            .map(|value| value.as_str()),
        binding_details
            .and_then(|details| details.runtime_variant_id.as_ref())
            .map(|value| value.as_str()),
    )?;
    Ok(RuntimeFeatureSourceKey {
        runtime_id: runtime_id.to_string(),
        feature_id: feature_id.to_string(),
        runtime_variant_id: runtime_variant_id.map(ToString::to_string),
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

fn observation_from_source_row(
    item: &DependencyReadinessWorkItem,
    binding_id: DependencyBindingId,
    source_row: &RuntimeFeatureProviderSourceRow,
) -> DependencyInventoryObservationRow {
    match (source_row.state, source_row.freshness) {
        (_, DependencyInventoryObservationFreshness::Stale)
        | (DependencyProviderSourceState::Stale, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Unavailable,
            DependencyEnvironmentValidationState::Stale,
            DependencyPlanningDiagnosticCode::ArtifactStale,
            "Runtime-feature source facts are stale.",
            "dependency_environment.runtime_feature.source",
        ),
        (DependencyProviderSourceState::Ready, _) => row(
            binding_id,
            DependencyInventoryObservationState::Ready,
            DependencyEnvironmentValidationState::Valid,
            source_row.diagnostics.clone(),
        ),
        (DependencyProviderSourceState::Missing, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Missing,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::ArtifactMissing,
            "Runtime-feature source facts are missing.",
            "dependency_environment.runtime_feature.source",
        ),
        (DependencyProviderSourceState::Failed, _) => observation_with_diagnostic(
            item,
            binding_id,
            DependencyInventoryObservationState::Failed,
            DependencyEnvironmentValidationState::Valid,
            DependencyPlanningDiagnosticCode::RuntimeUnavailable,
            "Runtime-feature source reported a failure.",
            "dependency_environment.runtime_feature.source",
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
            "Runtime-feature source is not ready for the requested feature.",
            "dependency_environment.runtime_feature.source",
        ),
    }
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
