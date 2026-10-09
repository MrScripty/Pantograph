//! Native CPU prerequisites owned by the compiled Candle adapter. Package
//! declarations are read from Pumas; readiness is supplied later by inventory.
//! This profile does not certify model bytes, residency, capacity or timing.

use pantograph_dependency_planning::*;
use pantograph_workflow_service::graph::InferenceInterfaceFactsProviderError;

use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsSource,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsOutcome, RuntimeDispatchCapabilityFactsSource,
};

pub(crate) const NATIVE_BINDINGS: [&str; 2] = [
    "candle.native_cpu.request_lifecycle.v1",
    "candle.native_cpu.device_selection.v1",
];

fn supported_request(request: &DependencyEnvironmentRequest) -> bool {
    let identity = &request.identity_key;
    let planning = &request.planning_request;
    let platform = DependencyPlanningPlatformContext::from_os_arch(
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
    .ok();
    let caller = &planning.caller_context;
    identity.task_id.as_str() == "embedding"
        && identity.task_type.is_none()
        && identity.expected_artifact_kind.is_none()
        && identity
            .scheduler_intent
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str())
            == Some("candle")
        && identity
            .scheduler_intent
            .requested_device_id
            .as_ref()
            .map(|id| id.as_str())
            == Some("cpu")
        && platform.is_some()
        && identity.platform_context == platform
        && identity.model_ref.model_id.len() <= 512
        && identity
            .model_ref
            .revision
            .as_ref()
            .is_none_or(|value| value.len() <= 512)
        && identity
            .model_ref
            .selected_artifact_id
            .as_ref()
            .is_none_or(|value| value.len() <= 512)
        && identity.model_ref.selected_artifact_path.is_none()
        && identity.model_ref.migration_diagnostics.is_empty()
        && identity.selected_binding_ids.len() <= 2
        && (identity.selected_binding_ids.is_empty()
            || (identity.selected_binding_ids.len() == 2
                && NATIVE_BINDINGS.iter().all(|id| {
                    identity
                        .selected_binding_ids
                        .iter()
                        .any(|binding| binding.as_str() == *id)
                })))
        && caller
            .source_node_type
            .as_ref()
            .is_none_or(|id| id.as_str().len() <= 128)
        && planning.dependency_override_patches.is_empty()
        && planning.trait_intents.is_empty()
        && [
            &caller.workflow_id,
            &caller.node_id,
            &caller.port_id,
            &caller.run_id,
        ]
        .iter()
        .all(|value| value.as_ref().is_none_or(|value| value.len() <= 512))
        && request
            .dependency_requirements_id
            .as_ref()
            .is_some_and(|id| id.as_str().len() <= 128)
        && request
            .environment_ref
            .as_ref()
            .is_none_or(|reference| reference == &native_environment_ref())
}

pub(crate) fn native_environment_ref() -> DependencyEnvironmentRef {
    DependencyEnvironmentRef {
        environment_id: DependencyEnvironmentId::parse(format!(
            "native:candle.cpu:{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
        .expect("native environment id"),
        manifest_id: Some(
            DependencyEnvironmentManifestId::parse("native-candle-cpu-embedding-v1")
                .expect("profile id"),
        ),
    }
}

fn native_requirements() -> Vec<DependencyRequirement> {
    ["request_lifecycle", "device_selection"].iter().map(|feature| {
        serde_json::from_value(serde_json::json!({"name": format!("candle_native_{feature}"), "kind": "runtime_feature",
            "runtime_feature": {"runtime_id": "candle", "feature_id": feature}})).expect("native prerequisite")
    }).collect()
}

fn native_bindings() -> Vec<DependencyRequirementBinding> {
    ["request_lifecycle", "device_selection"].iter().zip(NATIVE_BINDINGS).map(|(feature, binding)| {
        serde_json::from_value(serde_json::json!({"binding_id": binding, "requirement_name": format!("candle_native_{feature}"), "environment_kind": "runtime_feature",
            "runtime_feature": {"runtime_id": "candle", "feature_id": feature}})).expect("native prerequisite binding")
    }).collect()
}

pub(crate) fn is_native_payload(
    request: &DependencyEnvironmentRequest,
    payload: &pantograph_dependency_environment_service::DependencyRequirementsPayload,
) -> bool {
    supported_request(request)
        && payload.identity_key == request.identity_key
        && Some(&payload.dependency_requirements_id) == request.dependency_requirements_id.as_ref()
        && payload.selected_binding_ids.len() == 2
        && payload.bindings.len() == 2
        && payload.requirements.len() == 2
        && NATIVE_BINDINGS.iter().all(|binding| {
            payload
                .selected_binding_ids
                .iter()
                .any(|id| id.as_str() == *binding)
        })
        && payload.requirements == native_requirements()
        && payload.bindings == native_bindings()
}

pub(crate) async fn resolve_native_candle_requirements(
    packages: &PumasDispatchPackageFactsSource,
    capabilities: &RuntimeDispatchCapabilityFactsSource,
    validated: &ValidatedDependencyEnvironmentRequest,
) -> Result<Option<ValidatedDependencyEnvironmentResult>, InferenceInterfaceFactsProviderError> {
    let request = validated.as_request();
    if !matches!(request.action, DependencyEnvironmentAction::Resolve)
        || !supported_request(request)
    {
        return Ok(None);
    }
    if !capabilities.supports_candle_cpu_embeddings().await {
        return Ok(None);
    }
    let fail = |error: String| InferenceInterfaceFactsProviderError::Resolve(error);
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
        runtime.runtime_id == "candle"
            && runtime.runtime_family == "candle"
            && !matches!(
                runtime.status,
                pantograph_runtime_registry::RuntimeRegistryStatus::Failed
                    | pantograph_runtime_registry::RuntimeRegistryStatus::Unhealthy
                    | pantograph_runtime_registry::RuntimeRegistryStatus::Stopping
            )
            && runtime.automatic_device_candidates.iter().any(|candidate| {
                candidate.backend_key == "candle"
                    && candidate.runtime_variant_id.as_str() == "candle.cpu"
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
        || inference::resolve_task_registry_entry_from_evidence(&facts.task)
            .ok()
            .is_none_or(|entry| entry.task_id != inference::InferenceTaskId::Embedding)
    {
        return Ok(None);
    }
    let platform = request
        .identity_key
        .platform_context
        .as_ref()
        .expect("checked platform")
        .platform_key
        .as_str();
    if !packages
        .has_no_declared_dependencies(&request.identity_key.model_ref.model_id, platform)
        .await
        .map_err(fail)?
    {
        return Ok(None);
    }
    let selected = if request.identity_key.selected_binding_ids.is_empty() {
        NATIVE_BINDINGS
            .iter()
            .map(|id| DependencyBindingId::parse(id).expect("native binding"))
            .collect()
    } else {
        request.identity_key.selected_binding_ids.clone()
    };
    let result = DependencyEnvironmentResult {
        contract_version: 1,
        action: request.action,
        identity_key: request.identity_key.clone(),
        readiness_state: DependencyEnvironmentReadinessState::Resolved,
        install_state: DependencyEnvironmentInstallState::NotRequested,
        validation_state: DependencyEnvironmentValidationState::Valid,
        failure_state: None,
        dependency_requirements_id: request.dependency_requirements_id.clone(),
        environment_ref: request.environment_ref.clone(),
        requirements: native_requirements(),
        bindings: native_bindings(),
        selected_binding_ids: selected,
        binding_statuses: Vec::new(),
        operation: None,
        validation_errors: Vec::new(),
        diagnostics: Vec::new(),
    };
    ValidatedDependencyEnvironmentResult::try_from(result)
        .map(Some)
        .map_err(|error| fail(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_dependency_environment_service::DependencyRequirementsPayload;

    fn native_request() -> DependencyEnvironmentRequest {
        let planning = DependencyPlanningRequest {
            model_ref: PumasModelRef {
                model_id: "embedding/qualification/synthetic-bert-8".into(),
                revision: None,
                selected_artifact_id: Some("main".into()),
                selected_artifact_path: None,
                migration_diagnostics: vec![],
            },
            task_id: DependencyTaskId::parse("embedding").unwrap(),
            task_type: None,
            expected_artifact_kind: None,
            scheduler_intent: SchedulerIntent {
                requested_runtime_id: Some(RuntimeIntentId::parse("candle").unwrap()),
                requested_device_id: Some(DeviceIntentId::parse("cpu").unwrap()),
            },
            platform_context: Some(
                DependencyPlanningPlatformContext::from_os_arch(
                    std::env::consts::OS,
                    std::env::consts::ARCH,
                )
                .unwrap(),
            ),
            selected_binding_ids: vec![],
            dependency_override_patches: vec![],
            trait_intents: vec![],
            caller_context: Default::default(),
        };
        let proof = produce_dependency_requirements_proof(
            &ValidatedDependencyPlanningRequest::try_from(planning.clone()).unwrap(),
            None,
        )
        .unwrap();
        DependencyEnvironmentRequest {
            contract_version: 1,
            action: DependencyEnvironmentAction::Resolve,
            identity_key: proof.identity_key,
            planning_request: planning,
            dependency_requirements_id: Some(proof.dependency_requirements_id),
            environment_ref: None,
        }
    }

    fn payload(request: &DependencyEnvironmentRequest) -> DependencyRequirementsPayload {
        DependencyRequirementsPayload::new(
            request.dependency_requirements_id.clone().unwrap(),
            request.identity_key.clone(),
            native_requirements(),
            native_bindings(),
            NATIVE_BINDINGS
                .iter()
                .map(|id| DependencyBindingId::parse(id).unwrap())
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn native_environment_requires_exact_cpu_identity_and_profile() {
        let request = native_request();
        assert!(supported_request(&request));
        let mut payload = payload(&request);
        assert!(is_native_payload(&request, &payload));
        payload.bindings[0]
            .runtime_feature
            .as_mut()
            .unwrap()
            .runtime_id = Some(RuntimeSourceId::parse("pytorch").unwrap());
        assert!(!is_native_payload(&request, &payload));
        let mut request = request;
        request.identity_key.scheduler_intent.requested_device_id =
            Some(DeviceIntentId::parse("cuda:0").unwrap());
        assert!(!is_native_payload(&request, &payload));
    }

    #[tokio::test]
    async fn native_resolver_refuses_paths_large_input_and_unknown_authored_choices() {
        let mut request = native_request();
        request.identity_key.model_ref.model_id = "x".repeat(513);
        assert!(!supported_request(&request));
        let mut request = native_request();
        request.identity_key.model_ref.selected_artifact_path = Some("/tmp/model".into());
        assert!(!supported_request(&request));
        let mut request = native_request();
        request.identity_key.selected_binding_ids =
            vec![DependencyBindingId::parse("python.foreign").unwrap()];
        assert!(!supported_request(&request));
        let request = native_request();
        let unsupported = RuntimeDispatchCapabilityFactsSource::new(std::sync::Arc::new(
            pantograph_runtime_registry::RuntimeRegistry::new(),
        ));
        assert!(!unsupported.supports_candle_cpu_embeddings().await);
        assert_ne!(
            native_environment_ref().environment_id.as_str(),
            "python:default-host"
        );
        assert!(supported_request(&request));
    }
}
