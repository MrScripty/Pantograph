use pantograph_runtime_registry::{
    select_runtime_technical_fit, RuntimeRegistrySnapshot, RuntimeTechnicalFitCandidate,
    RuntimeTechnicalFitCandidateSourceKind, RuntimeTechnicalFitDecision, RuntimeTechnicalFitFactor,
    RuntimeTechnicalFitOverride, RuntimeTechnicalFitReason, RuntimeTechnicalFitReasonCode,
    RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitResourcePressure, RuntimeTechnicalFitSelectionMode,
    RuntimeTechnicalFitWarmupState,
};
use pantograph_workflow_service::{
    WorkflowHost, WorkflowRuntimeCapability, WorkflowRuntimeInstallState,
    WorkflowRuntimeSourceKind, WorkflowServiceError, WorkflowTechnicalFitDecision,
    WorkflowTechnicalFitQueuePressure, WorkflowTechnicalFitReason, WorkflowTechnicalFitReasonCode,
    WorkflowTechnicalFitRequest, WorkflowTechnicalFitSelectionMode,
};

use crate::{workflow_runtime::unix_timestamp_ms, EmbeddedWorkflowHost};

pub fn build_runtime_technical_fit_request_with_package_facts(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> RuntimeTechnicalFitRequest {
    let mut runtime_request =
        build_runtime_technical_fit_request(request, runtime_snapshot, runtime_capabilities);
    runtime_request
        .candidates
        .extend(runtime_candidates_from_pumas_package_facts(package_facts));
    runtime_request.normalized()
}

pub(crate) async fn workflow_technical_fit_decision(
    host: &EmbeddedWorkflowHost,
    request: &WorkflowTechnicalFitRequest,
) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
    let runtime_capabilities = host.runtime_capabilities().await?;
    let runtime_snapshot = host
        .runtime_registry
        .as_ref()
        .map(|registry| registry.snapshot());
    let runtime_request =
        build_runtime_technical_fit_request(request, runtime_snapshot, &runtime_capabilities);
    let decision = select_runtime_technical_fit(&runtime_request);
    Ok(Some(project_workflow_technical_fit_decision(&decision)))
}

pub fn build_runtime_technical_fit_request(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> RuntimeTechnicalFitRequest {
    RuntimeTechnicalFitRequest {
        runtime_snapshot: runtime_snapshot.unwrap_or_else(empty_runtime_snapshot),
        workflow_id: Some(request.workflow_id.clone()),
        required_model_ids: request.runtime_requirements.required_models.clone(),
        required_backend_keys: request.runtime_requirements.required_backends.clone(),
        required_extensions: request.runtime_requirements.required_extensions.clone(),
        required_context_window_tokens: None,
        override_selection: request
            .override_selection
            .as_ref()
            .and_then(project_override),
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: runtime_capability_candidates(runtime_capabilities),
        resource_pressure: project_resource_pressure(
            request.queue_pressure.as_ref(),
            request.runtime_requirements.estimated_peak_vram_mb,
            request.runtime_requirements.estimated_peak_ram_mb,
        ),
    }
    .normalized()
}

pub fn project_workflow_technical_fit_decision(
    decision: &RuntimeTechnicalFitDecision,
) -> WorkflowTechnicalFitDecision {
    WorkflowTechnicalFitDecision {
        selection_mode: project_selection_mode(decision.selection_mode),
        selected_candidate_id: decision.selected_candidate_id.clone(),
        selected_runtime_id: decision.selected_runtime_id.clone(),
        selected_backend_key: decision.selected_backend_key.clone(),
        selected_model_id: decision.selected_model_id.clone(),
        reasons: decision
            .reasons
            .iter()
            .map(project_reason)
            .collect::<Vec<_>>(),
    }
    .normalized()
}

fn runtime_capability_candidates(
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> Vec<RuntimeTechnicalFitCandidate> {
    runtime_capabilities
        .iter()
        .map(|capability| RuntimeTechnicalFitCandidate {
            candidate_id: capability
                .backend_keys
                .first()
                .cloned()
                .unwrap_or_else(|| capability.runtime_id.clone()),
            runtime_id: Some(capability.runtime_id.clone()),
            backend_key: capability
                .backend_keys
                .first()
                .cloned()
                .or_else(|| Some(capability.runtime_id.clone())),
            model_id: None,
            source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFallback,
            context_window_tokens: None,
            residency_state: Some(runtime_capability_residency_state(capability)),
            warmup_state: runtime_capability_warmup_state(capability),
            supports_runtime_requirements: runtime_capability_is_ready(capability),
        })
        .collect()
}

pub fn runtime_candidates_from_pumas_package_facts(
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> Vec<RuntimeTechnicalFitCandidate> {
    package_facts
        .iter()
        .flat_map(|facts| {
            facts
                .backend_hints
                .accepted
                .iter()
                .filter_map(|hint| pumas_backend_hint_label_to_backend_key(*hint))
                .map(|backend_key| RuntimeTechnicalFitCandidate {
                    candidate_id: format!("{}|{}", backend_key, facts.model_ref.model_id),
                    runtime_id: None,
                    backend_key: Some(backend_key),
                    model_id: Some(facts.model_ref.model_id.clone()),
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                    context_window_tokens: None,
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: matches!(
                        facts.artifact.validation_state,
                        inference::ModelValidationState::Valid
                    ),
                })
        })
        .collect()
}

fn pumas_backend_hint_label_to_backend_key(label: inference::BackendHintLabel) -> Option<String> {
    match label {
        inference::BackendHintLabel::Transformers => Some("pytorch".to_string()),
        inference::BackendHintLabel::LlamaCpp => Some("llama_cpp".to_string()),
        inference::BackendHintLabel::Vllm => Some("vllm".to_string()),
        inference::BackendHintLabel::Mlx => Some("mlx".to_string()),
        inference::BackendHintLabel::Candle => Some("candle".to_string()),
        inference::BackendHintLabel::Diffusers => Some("diffusers".to_string()),
        inference::BackendHintLabel::OnnxRuntime => Some("onnx_runtime".to_string()),
    }
}

fn project_override(
    override_selection: &pantograph_workflow_service::WorkflowTechnicalFitOverride,
) -> Option<RuntimeTechnicalFitOverride> {
    RuntimeTechnicalFitOverride {
        model_id: override_selection.model_id.clone(),
        backend_key: override_selection.backend_key.clone(),
    }
    .normalized()
}

fn project_resource_pressure(
    queue_pressure: Option<&WorkflowTechnicalFitQueuePressure>,
    estimated_peak_vram_mb: Option<u64>,
    estimated_peak_ram_mb: Option<u64>,
) -> Option<RuntimeTechnicalFitResourcePressure> {
    let pressure = RuntimeTechnicalFitResourcePressure {
        queued_run_count: queue_pressure.and_then(|pressure| pressure.total_queued_run_count),
        loaded_runtime_count: queue_pressure.and_then(|pressure| pressure.loaded_runtime_count),
        loaded_runtime_capacity: queue_pressure
            .and_then(|pressure| pressure.loaded_runtime_capacity),
        estimated_peak_vram_mb,
        estimated_peak_ram_mb,
    };

    if pressure.queued_run_count.is_none()
        && pressure.loaded_runtime_count.is_none()
        && pressure.loaded_runtime_capacity.is_none()
        && pressure.estimated_peak_vram_mb.is_none()
        && pressure.estimated_peak_ram_mb.is_none()
    {
        None
    } else {
        Some(pressure)
    }
}

fn project_selection_mode(
    selection_mode: RuntimeTechnicalFitSelectionMode,
) -> WorkflowTechnicalFitSelectionMode {
    match selection_mode {
        RuntimeTechnicalFitSelectionMode::Automatic => WorkflowTechnicalFitSelectionMode::Automatic,
        RuntimeTechnicalFitSelectionMode::ExplicitOverride => {
            WorkflowTechnicalFitSelectionMode::ExplicitOverride
        }
        RuntimeTechnicalFitSelectionMode::ConservativeFallback => {
            WorkflowTechnicalFitSelectionMode::ConservativeFallback
        }
    }
}

fn project_reason(reason: &RuntimeTechnicalFitReason) -> WorkflowTechnicalFitReason {
    WorkflowTechnicalFitReason::new(
        project_reason_code(reason.code),
        reason.candidate_id.as_deref(),
    )
}

fn project_reason_code(
    reason_code: RuntimeTechnicalFitReasonCode,
) -> WorkflowTechnicalFitReasonCode {
    match reason_code {
        RuntimeTechnicalFitReasonCode::ExplicitModelOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitModelOverride
        }
        RuntimeTechnicalFitReasonCode::ExplicitBackendOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
        }
        RuntimeTechnicalFitReasonCode::RequiredContextLength => {
            WorkflowTechnicalFitReasonCode::RequiredContextLength
        }
        RuntimeTechnicalFitReasonCode::RuntimeRequirements => {
            WorkflowTechnicalFitReasonCode::RuntimeRequirements
        }
        RuntimeTechnicalFitReasonCode::ResidencyReuse => {
            WorkflowTechnicalFitReasonCode::ResidencyReuse
        }
        RuntimeTechnicalFitReasonCode::WarmupCost => WorkflowTechnicalFitReasonCode::WarmupCost,
        RuntimeTechnicalFitReasonCode::BudgetPressure => {
            WorkflowTechnicalFitReasonCode::BudgetPressure
        }
        RuntimeTechnicalFitReasonCode::QueuePressure => {
            WorkflowTechnicalFitReasonCode::QueuePressure
        }
        RuntimeTechnicalFitReasonCode::MissingCandidateData => {
            WorkflowTechnicalFitReasonCode::MissingCandidateData
        }
        RuntimeTechnicalFitReasonCode::MissingRuntimeState => {
            WorkflowTechnicalFitReasonCode::MissingRuntimeState
        }
        RuntimeTechnicalFitReasonCode::DeterministicTieBreak => {
            WorkflowTechnicalFitReasonCode::DeterministicTieBreak
        }
        RuntimeTechnicalFitReasonCode::ConservativeFallback => {
            WorkflowTechnicalFitReasonCode::ConservativeFallback
        }
    }
}

fn runtime_capability_residency_state(
    capability: &WorkflowRuntimeCapability,
) -> RuntimeTechnicalFitResidencyState {
    if capability.available && capability.selected {
        RuntimeTechnicalFitResidencyState::Active
    } else if capability.available {
        RuntimeTechnicalFitResidencyState::Loaded
    } else {
        RuntimeTechnicalFitResidencyState::Unloaded
    }
}

fn runtime_capability_warmup_state(
    capability: &WorkflowRuntimeCapability,
) -> Option<RuntimeTechnicalFitWarmupState> {
    if capability.available && capability.selected {
        Some(RuntimeTechnicalFitWarmupState::Ready)
    } else if capability.available {
        Some(RuntimeTechnicalFitWarmupState::Warm)
    } else {
        None
    }
}

fn runtime_capability_is_ready(capability: &WorkflowRuntimeCapability) -> bool {
    capability.available
        && capability.configured
        && matches!(
            capability.install_state,
            WorkflowRuntimeInstallState::Installed | WorkflowRuntimeInstallState::SystemProvided
        )
        && matches!(
            capability.source_kind,
            WorkflowRuntimeSourceKind::Managed
                | WorkflowRuntimeSourceKind::System
                | WorkflowRuntimeSourceKind::Host
        )
}

fn empty_runtime_snapshot() -> RuntimeRegistrySnapshot {
    RuntimeRegistrySnapshot {
        generated_at_ms: unix_timestamp_ms(),
        runtimes: Vec::new(),
        reservations: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_workflow_service::{
        build_workflow_technical_fit_request, WorkflowRuntimeReadinessState,
        WorkflowRuntimeRequirements,
    };

    fn runtime_capability() -> WorkflowRuntimeCapability {
        WorkflowRuntimeCapability {
            runtime_id: "llama.cpp".to_string(),
            display_name: "llama.cpp".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Managed,
            selected: true,
            readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
            selected_version: None,
            supports_external_connection: false,
            backend_capability_facts: None,
            backend_keys: vec!["llama_cpp".to_string(), "llama.cpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }
    }

    #[test]
    fn runtime_request_projection_maps_service_request_into_registry_contract() {
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["llama.cpp".to_string()],
                required_extensions: vec!["kv_cache".to_string()],
            },
            Some(pantograph_workflow_service::WorkflowTechnicalFitOverride {
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama.cpp".to_string()),
            }),
            Some("session-a"),
            Some("interactive"),
            Some(WorkflowTechnicalFitQueuePressure {
                current_session_queue_depth: Some(1),
                total_queued_run_count: Some(3),
                loaded_runtime_count: Some(1),
                loaded_runtime_capacity: Some(4),
            }),
        );

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &[runtime_capability()]);

        assert_eq!(runtime_request.workflow_id.as_deref(), Some("workflow-a"));
        assert_eq!(runtime_request.required_model_ids, vec!["model-a"]);
        assert_eq!(runtime_request.required_backend_keys, vec!["llama_cpp"]);
        assert_eq!(runtime_request.required_extensions, vec!["kv_cache"]);
        assert_eq!(
            runtime_request.override_selection,
            Some(RuntimeTechnicalFitOverride {
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            })
        );
        assert_eq!(runtime_request.candidates.len(), 1);
        assert_eq!(runtime_request.candidates[0].candidate_id, "llama_cpp");
        assert_eq!(
            runtime_request.candidates[0].residency_state,
            Some(RuntimeTechnicalFitResidencyState::Active)
        );
        assert_eq!(
            runtime_request.resource_pressure,
            Some(RuntimeTechnicalFitResourcePressure {
                queued_run_count: Some(3),
                loaded_runtime_count: Some(1),
                loaded_runtime_capacity: Some(4),
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
            })
        );
    }

    #[test]
    fn workflow_decision_projection_preserves_reason_codes() {
        let decision = RuntimeTechnicalFitDecision {
            selection_mode: RuntimeTechnicalFitSelectionMode::ConservativeFallback,
            selected_candidate_id: Some("candidate-a".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: Some("model-a".to_string()),
            reasons: vec![RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::QueuePressure,
                Some("candidate-a"),
            )],
        };

        let projected = project_workflow_technical_fit_decision(&decision);

        assert_eq!(
            projected,
            WorkflowTechnicalFitDecision {
                selection_mode: WorkflowTechnicalFitSelectionMode::ConservativeFallback,
                selected_candidate_id: Some("candidate-a".to_string()),
                selected_runtime_id: Some("llama_cpp".to_string()),
                selected_backend_key: Some("llama_cpp".to_string()),
                selected_model_id: Some("model-a".to_string()),
                reasons: vec![WorkflowTechnicalFitReason {
                    code: WorkflowTechnicalFitReasonCode::QueuePressure,
                    candidate_id: Some("candidate-a".to_string()),
                }],
            }
        );
    }

    #[test]
    fn runtime_selector_decision_projects_back_into_workflow_contracts() {
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
                estimation_confidence: "high".to_string(),
                required_models: Vec::new(),
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            Some(pantograph_workflow_service::WorkflowTechnicalFitOverride {
                model_id: None,
                backend_key: Some("llama.cpp".to_string()),
            }),
            None,
            None,
            None,
        );

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &[runtime_capability()]);
        let registry_decision = select_runtime_technical_fit(&runtime_request);
        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);

        assert_eq!(
            workflow_decision,
            WorkflowTechnicalFitDecision {
                selection_mode: WorkflowTechnicalFitSelectionMode::ExplicitOverride,
                selected_candidate_id: Some("llama_cpp".to_string()),
                selected_runtime_id: Some("llama_cpp".to_string()),
                selected_backend_key: Some("llama_cpp".to_string()),
                selected_model_id: None,
                reasons: vec![WorkflowTechnicalFitReason {
                    code: WorkflowTechnicalFitReasonCode::ExplicitBackendOverride,
                    candidate_id: Some("llama_cpp".to_string()),
                }],
            }
        );
    }

    #[test]
    fn pumas_package_facts_project_to_advisory_runtime_candidates() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
            ),
        )
        .expect("decode package facts fixture");

        let candidates = runtime_candidates_from_pumas_package_facts(&[package_facts]);

        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].source_kind,
            RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts
        );
        assert_eq!(candidates[0].backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(
            candidates[0].model_id.as_deref(),
            Some("llm/llama/tiny-gguf")
        );
        assert_eq!(candidates[0].runtime_id, None);
        assert_eq!(candidates[0].residency_state, None);
        assert_eq!(candidates[0].warmup_state, None);
        assert!(candidates[0].supports_runtime_requirements);
    }

    #[test]
    fn pumas_package_facts_do_not_project_remote_discovery_hints() {
        let remote_search_hint: serde_json::Value = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/remote_search_mlx_vllm_hint.json"
            ),
        )
        .expect("decode remote search fixture");

        assert!(remote_search_hint
            .get("package_facts_contract_version")
            .is_none());
        assert!(
            serde_json::from_value::<inference::ResolvedModelPackageFacts>(remote_search_hint)
                .is_err()
        );
        assert!(runtime_candidates_from_pumas_package_facts(&[]).is_empty());
    }

    #[test]
    fn technical_fit_request_can_include_pumas_package_facts_candidates() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
            ),
        )
        .expect("decode package facts fixture");
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "fixture".to_string(),
                required_models: vec!["llm/llama/tiny-gguf".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            None,
            None,
            None,
            None,
        );

        let runtime_request = build_runtime_technical_fit_request_with_package_facts(
            &workflow_request,
            None,
            &[],
            &[package_facts],
        );

        assert_eq!(runtime_request.candidates.len(), 1);
        assert_eq!(
            runtime_request.candidates[0].source_kind,
            RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts
        );

        let decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(
            decision.selected_model_id.as_deref(),
            Some("llm/llama/tiny-gguf")
        );
        assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
    }
}
