//! Local CPU text prerequisites. Pumas declarations and inventory observations
//! remain separate authorities; Resolve never supplies a Ready observation.

use inference::backend::pytorch::TEXT_GENERATION_PACKAGES;
use pantograph_dependency_environment_service::DependencyRequirementsPayload;
use pantograph_dependency_planning::*;
use pantograph_workflow_service::graph::InferenceInterfaceFactsProviderError;

use crate::inference_dependency_requirements::supported_profile_request;
use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsSource,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsOutcome, RuntimeDispatchCapabilityFactsSource,
};

const PROFILE: &str = "pytorch.embedded.cpu.text.v1";

fn binding_names() -> Vec<String> {
    TEXT_GENERATION_PACKAGES
        .iter()
        .map(|name| format!("{PROFILE}.{name}"))
        .collect()
}

pub(crate) fn environment_ref() -> DependencyEnvironmentRef {
    DependencyEnvironmentRef {
        environment_id: DependencyEnvironmentId::parse("python:pytorch-embedded")
            .expect("embedded interpreter identity"),
        manifest_id: Some(DependencyEnvironmentManifestId::parse(PROFILE).expect("text profile")),
    }
}

fn supported_request(request: &DependencyEnvironmentRequest) -> bool {
    supported_profile_request(
        request,
        "text_generation",
        "pytorch",
        &binding_names(),
        &environment_ref(),
    )
}

fn requirements() -> Vec<DependencyRequirement> {
    TEXT_GENERATION_PACKAGES
        .iter()
        .map(|name| {
            serde_json::from_value(serde_json::json!({
                "name": name, "kind": "python_package"
            }))
            .expect("fixed text prerequisite")
        })
        .collect()
}

fn bindings() -> Vec<DependencyRequirementBinding> {
    TEXT_GENERATION_PACKAGES
        .iter()
        .zip(binding_names())
        .map(|(name, id)| {
            serde_json::from_value(serde_json::json!({
                "binding_id": id, "requirement_name": name,
                "environment_kind": "python", "profile_id": PROFILE
            }))
            .expect("fixed text prerequisite binding")
        })
        .collect()
}

/// Only the exact owned profile may observe the embedded interpreter. An
/// altered payload must not silently fall through to a host-process probe.
pub(crate) fn is_text_payload(
    request: &DependencyEnvironmentRequest,
    payload: &DependencyRequirementsPayload,
) -> bool {
    supported_request(request)
        && payload.identity_key == request.identity_key
        && Some(&payload.dependency_requirements_id) == request.dependency_requirements_id.as_ref()
        && payload.requirements == requirements()
        && payload.bindings == bindings()
        && payload.selected_binding_ids.len() == TEXT_GENERATION_PACKAGES.len()
        && binding_names().iter().all(|id| {
            payload
                .selected_binding_ids
                .iter()
                .any(|binding| binding.as_str() == id)
        })
}

/// Reserve the owned environment/profile so malformed text payloads cannot be
/// interpreted as a request to inspect another Python installation.
pub(crate) fn targets_text_environment(
    request: &DependencyEnvironmentRequest,
    payload: &DependencyRequirementsPayload,
) -> bool {
    request
        .environment_ref
        .as_ref()
        .is_some_and(|environment| environment.environment_id == environment_ref().environment_id)
        || payload.bindings.iter().any(|binding| {
            binding
                .profile_id
                .as_ref()
                .is_some_and(|profile| profile.as_str() == PROFILE)
                || binding.binding_id.as_str().starts_with(PROFILE)
        })
}

pub(crate) async fn resolve_text_requirements(
    packages: &PumasDispatchPackageFactsSource,
    capabilities: &RuntimeDispatchCapabilityFactsSource,
    validated: &ValidatedDependencyEnvironmentRequest,
) -> Result<Option<ValidatedDependencyEnvironmentResult>, InferenceInterfaceFactsProviderError> {
    let request = validated.as_request();
    if request.action != DependencyEnvironmentAction::Resolve || !supported_request(request) {
        return Ok(None);
    }
    let fail = InferenceInterfaceFactsProviderError::Resolve;
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(request.planning_request.clone())
            .map_err(|error| fail(error.to_string()))?,
        None,
    )
    .map_err(|error| fail(error.to_string()))?;
    if Some(&proof.dependency_requirements_id) != request.dependency_requirements_id.as_ref()
        || proof.identity_key != request.identity_key
    {
        return Ok(None);
    }
    let RuntimeDispatchCapabilityFactsOutcome::Projected { facts, .. } =
        capabilities.collect().await
    else {
        return Ok(None);
    };
    if !facts.runtimes.iter().any(|runtime| {
        runtime.runtime_id == "pytorch"
            && runtime.runtime_family == "pytorch"
            && runtime
                .backend_capabilities
                .supports_task(inference::InferenceTaskId::TextGeneration)
            && !matches!(
                runtime.status,
                pantograph_runtime_registry::RuntimeRegistryStatus::Failed
                    | pantograph_runtime_registry::RuntimeRegistryStatus::Unhealthy
                    | pantograph_runtime_registry::RuntimeRegistryStatus::Stopping
            )
            && runtime.automatic_device_candidates.iter().any(|candidate| {
                candidate.backend_key == "pytorch"
                    && candidate.runtime_variant_id.as_str() == "pytorch.cpu"
                    && candidate.device_id.as_str() == "cpu"
            })
    }) {
        return Ok(None);
    }
    let PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. } =
        packages.collect(&request.identity_key.model_ref).await
    else {
        return Ok(None);
    };
    if facts.validation_state != inference::ModelValidationState::Valid
        || facts.requires_custom_code
        || !facts
            .backend_hints
            .accepted
            .contains(&inference::BackendHintLabel::Transformers)
        || !matches!(
            facts.artifact_kind,
            inference::ModelArtifactKind::HfCompatibleDirectory
                | inference::ModelArtifactKind::Safetensors
        )
        || inference::resolve_task_registry_entry_from_evidence(&facts.task)
            .ok()
            .is_none_or(|entry| entry.task_id != inference::InferenceTaskId::TextGeneration)
    {
        return Ok(None);
    }
    let platform = request
        .identity_key
        .platform_context
        .as_ref()
        .expect("checked host platform");
    if !packages
        .has_no_declared_dependencies(
            &request.identity_key.model_ref.model_id,
            platform.platform_key.as_str(),
            "pytorch",
        )
        .await
        .map_err(fail)?
    {
        return Ok(None);
    }
    let selected_binding_ids = if request.identity_key.selected_binding_ids.is_empty() {
        binding_names()
            .iter()
            .map(|id| DependencyBindingId::parse(id).expect("text binding"))
            .collect()
    } else {
        request.identity_key.selected_binding_ids.clone()
    };
    ValidatedDependencyEnvironmentResult::try_from(DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Resolved,
        install_state: DependencyEnvironmentInstallState::NotRequested,
        validation_state: DependencyEnvironmentValidationState::Valid,
        failure_state: None,
        dependency_requirements_id: request.dependency_requirements_id.clone(),
        environment_ref: Some(environment_ref()),
        requirements: requirements(),
        bindings: bindings(),
        selected_binding_ids,
        binding_statuses: Vec::new(),
        operation: None,
        validation_errors: Vec::new(),
        diagnostics: Vec::new(),
    })
    .map(Some)
    .map_err(|error| fail(error.to_string()))
}

#[cfg(test)]
#[path = "inference_text_dependency_requirements_tests.rs"]
mod tests;
