use std::collections::HashSet;

use pantograph_runtime_registry::{
    select_runtime_technical_fit, RuntimeRegistrySnapshot, RuntimeTechnicalFitCandidate,
    RuntimeTechnicalFitCandidateSourceKind, RuntimeTechnicalFitCompatibilityIssue,
    RuntimeTechnicalFitCompatibilityReport, RuntimeTechnicalFitDecision, RuntimeTechnicalFitFactor,
    RuntimeTechnicalFitOverride, RuntimeTechnicalFitReason, RuntimeTechnicalFitReasonCode,
    RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitResourcePressure, RuntimeTechnicalFitSelectionMode,
    RuntimeTechnicalFitWarmupState,
};
use pantograph_workflow_service::{
    WorkflowHost, WorkflowRuntimeCapability, WorkflowRuntimeInstallState,
    WorkflowRuntimeSourceKind, WorkflowServiceError, WorkflowTechnicalFitCompatibilityIssue,
    WorkflowTechnicalFitCompatibilityReport, WorkflowTechnicalFitDecision,
    WorkflowTechnicalFitQueuePressure, WorkflowTechnicalFitReason, WorkflowTechnicalFitReasonCode,
    WorkflowTechnicalFitRequest, WorkflowTechnicalFitSelectionMode,
};
use workflow_nodes::setup::PumasSelectorAccess;

use crate::{workflow_runtime::unix_timestamp_ms, EmbeddedWorkflowHost};

const MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES: usize = 32;

pub fn build_runtime_technical_fit_request_with_package_facts(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> RuntimeTechnicalFitRequest {
    build_runtime_technical_fit_request_with_backend_package_facts(
        request,
        runtime_snapshot,
        runtime_capabilities,
        &[],
        package_facts,
    )
}

pub fn build_runtime_technical_fit_request_with_backend_package_facts(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
    available_backends: &[inference::BackendInfo],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> RuntimeTechnicalFitRequest {
    let mut runtime_request =
        build_runtime_technical_fit_request(request, runtime_snapshot, runtime_capabilities);
    let package_fact_candidates = if available_backends.is_empty() {
        runtime_candidates_from_pumas_package_facts(package_facts)
    } else {
        runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
            package_facts,
            available_backends,
        )
    };
    runtime_request.candidates.extend(package_fact_candidates);
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
    let available_backends = host.gateway.available_backends();
    let package_facts =
        resolve_required_model_package_facts(host, &request.runtime_requirements.required_models)
            .await;
    let runtime_request = if package_facts.is_empty() {
        build_runtime_technical_fit_request(request, runtime_snapshot, &runtime_capabilities)
    } else {
        build_runtime_technical_fit_request_with_backend_package_facts(
            request,
            runtime_snapshot,
            &runtime_capabilities,
            &available_backends,
            &package_facts,
        )
    };
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
        compatibility_report: decision
            .compatibility_report
            .as_ref()
            .map(project_compatibility_report),
        compatibility_issue_count: decision.compatibility_issue_count,
        compatibility_issues: decision
            .compatibility_issues
            .iter()
            .map(project_compatibility_issue)
            .collect(),
    }
    .normalized()
}

fn project_compatibility_report(
    report: &RuntimeTechnicalFitCompatibilityReport,
) -> WorkflowTechnicalFitCompatibilityReport {
    WorkflowTechnicalFitCompatibilityReport {
        status: report.status.clone(),
        compatible: report.compatible,
        task: report.task.clone(),
        model_source: report.model_source.clone(),
        preprocessing: report.preprocessing.clone(),
        postprocessing: report.postprocessing.clone(),
    }
}

fn project_compatibility_issue(
    issue: &RuntimeTechnicalFitCompatibilityIssue,
) -> WorkflowTechnicalFitCompatibilityIssue {
    WorkflowTechnicalFitCompatibilityIssue {
        kind: issue.kind.clone(),
        phase: issue.phase.clone(),
        message: issue.message.clone(),
        model_id: issue.model_id.clone(),
        path: issue.path.clone(),
    }
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
            compatibility_report: None,
            compatibility_issue_count: 0,
            compatibility_issues: Vec::new(),
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
                    compatibility_report: None,
                    compatibility_issue_count: 0,
                    compatibility_issues: Vec::new(),
                })
        })
        .collect()
}

pub fn runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
    package_facts: &[inference::ResolvedModelPackageFacts],
    available_backends: &[inference::BackendInfo],
) -> Vec<RuntimeTechnicalFitCandidate> {
    package_facts
        .iter()
        .filter_map(|facts| task_registry_entry_from_package_facts(facts).map(|task| (facts, task)))
        .flat_map(|(facts, task)| {
            available_backends.iter().map(move |backend| {
                let compatibility = backend.capabilities.check_model_compatibility(
                    Some(&backend.backend_key),
                    inference::BackendCompatibilityRequest::new(&task, facts),
                );
                RuntimeTechnicalFitCandidate {
                    candidate_id: format!("{}|{}", backend.backend_key, facts.model_ref.model_id),
                    runtime_id: None,
                    backend_key: Some(backend.backend_key.clone()),
                    model_id: Some(facts.model_ref.model_id.clone()),
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                    context_window_tokens: None,
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: compatibility.compatible,
                    compatibility_report: Some(runtime_compatibility_report(&compatibility)),
                    compatibility_issue_count: compatibility.issues.len().min(u32::MAX as usize)
                        as u32,
                    compatibility_issues: runtime_compatibility_issues(
                        &compatibility,
                        MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES,
                    ),
                }
            })
        })
        .collect()
}

fn runtime_compatibility_report(
    report: &inference::BackendCompatibilityReport,
) -> RuntimeTechnicalFitCompatibilityReport {
    let summary = report.to_inference_compatibility_report_summary();
    RuntimeTechnicalFitCompatibilityReport {
        status: summary.status,
        compatible: summary.compatible,
        task: summary.task,
        model_source: summary.model_source,
        preprocessing: summary.preprocessing,
        postprocessing: summary.postprocessing,
    }
}

fn runtime_compatibility_issues(
    report: &inference::BackendCompatibilityReport,
    limit: usize,
) -> Vec<RuntimeTechnicalFitCompatibilityIssue> {
    report
        .to_inference_compatibility_issue_summaries(limit)
        .into_iter()
        .map(|issue| RuntimeTechnicalFitCompatibilityIssue {
            kind: issue.kind,
            phase: inference_lifecycle_phase_key(&issue.phase).to_string(),
            message: issue.message,
            model_id: issue.model_id,
            path: issue.path,
        })
        .collect()
}

fn inference_lifecycle_phase_key(phase: &inference::InferenceLifecyclePhase) -> &'static str {
    match phase {
        inference::InferenceLifecyclePhase::ModelPackageResolution => "model_package_resolution",
        inference::InferenceLifecyclePhase::TaskValidation => "task_validation",
        inference::InferenceLifecyclePhase::Preprocessing => "preprocessing",
        inference::InferenceLifecyclePhase::BackendExecution => "backend_execution",
        inference::InferenceLifecyclePhase::Postprocessing => "postprocessing",
        inference::InferenceLifecyclePhase::ResultProjection => "result_projection",
    }
}

fn task_registry_entry_from_package_facts(
    facts: &inference::ResolvedModelPackageFacts,
) -> Option<inference::TaskRegistryEntry> {
    let labels = vec![
        facts.task.task_type_primary.as_deref(),
        facts.task.pipeline_tag.as_deref(),
    ];
    let task_id = labels
        .iter()
        .copied()
        .flatten()
        .find_map(inference_task_id_from_label)?;
    let result_family = task_result_family(&task_id).to_string();
    let registry_entry = inference::default_task_registry_entries()
        .into_iter()
        .find(|entry| entry.task_id == task_id);
    Some(inference::TaskRegistryEntry {
        task_id: task_id.clone(),
        aliases: labels
            .into_iter()
            .flatten()
            .map(str::to_string)
            .collect::<Vec<_>>(),
        task_family: registry_entry
            .as_ref()
            .map(|entry| entry.task_family.clone())
            .unwrap_or_default(),
        modality_signature: inference::TaskModalitySignature::new(
            facts
                .task
                .input_modalities
                .iter()
                .filter_map(|modality| inference_modality_from_label(modality))
                .collect(),
            facts
                .task
                .output_modalities
                .iter()
                .filter_map(|modality| inference_modality_from_label(modality))
                .collect(),
        ),
        result_family,
        execution_behavior: registry_entry
            .as_ref()
            .map(|entry| entry.execution_behavior.clone())
            .unwrap_or_default(),
        streaming_support: registry_entry
            .as_ref()
            .map(|entry| entry.streaming_support.clone())
            .unwrap_or_default(),
        support_tier: inference::SupportTier::Stable,
        required_components: registry_entry
            .as_ref()
            .map(|entry| entry.required_components.clone())
            .unwrap_or_default(),
        upstream_task_ids: registry_entry
            .map(|entry| entry.upstream_task_ids)
            .unwrap_or_default(),
    })
}

fn inference_task_id_from_label(label: &str) -> Option<inference::InferenceTaskId> {
    match normalize_package_label(label).as_str() {
        "text_generation" | "text2text_generation" => {
            Some(inference::InferenceTaskId::TextGeneration)
        }
        "chat_completion" | "conversational" => Some(inference::InferenceTaskId::ChatCompletion),
        "embedding" | "feature_extraction" | "sentence_similarity" => {
            Some(inference::InferenceTaskId::Embedding)
        }
        "rerank" | "text_reranking" => Some(inference::InferenceTaskId::Rerank),
        "image_generation" | "text_to_image" => Some(inference::InferenceTaskId::ImageGeneration),
        "image_understanding" | "image_to_text" | "visual_question_answering" => {
            Some(inference::InferenceTaskId::ImageUnderstanding)
        }
        "audio_transcription" | "automatic_speech_recognition" => {
            Some(inference::InferenceTaskId::AudioTranscription)
        }
        "video_understanding" => Some(inference::InferenceTaskId::VideoUnderstanding),
        "depth_estimation" | "depth_estimation_pipeline" => {
            Some(inference::InferenceTaskId::DepthEstimation)
        }
        "multimodal_generation" => Some(inference::InferenceTaskId::MultimodalGeneration),
        _ => None,
    }
}

fn inference_modality_from_label(label: &str) -> Option<inference::InferenceModality> {
    match normalize_package_label(label).as_str() {
        "text" => Some(inference::InferenceModality::Text),
        "image" => Some(inference::InferenceModality::Image),
        "audio" => Some(inference::InferenceModality::Audio),
        "video" => Some(inference::InferenceModality::Video),
        "embedding" => Some(inference::InferenceModality::Embedding),
        "tokens" => Some(inference::InferenceModality::Tokens),
        "json" => Some(inference::InferenceModality::Json),
        "point_cloud" => Some(inference::InferenceModality::PointCloud),
        "mesh" => Some(inference::InferenceModality::Mesh),
        "other" => Some(inference::InferenceModality::Other),
        _ => None,
    }
}

fn task_result_family(task_id: &inference::InferenceTaskId) -> &'static str {
    match task_id {
        inference::InferenceTaskId::Embedding => "embedding",
        inference::InferenceTaskId::Rerank => "ranking",
        inference::InferenceTaskId::ImageGeneration => "image",
        inference::InferenceTaskId::DepthEstimation => "depth",
        inference::InferenceTaskId::AudioTranscription => "text",
        inference::InferenceTaskId::VideoUnderstanding
        | inference::InferenceTaskId::ImageUnderstanding
        | inference::InferenceTaskId::MultimodalGeneration => "multimodal",
        inference::InferenceTaskId::TextGeneration | inference::InferenceTaskId::ChatCompletion => {
            "text"
        }
        inference::InferenceTaskId::Unknown => "unknown",
    }
}

fn normalize_package_label(label: &str) -> String {
    label.trim().to_ascii_lowercase().replace('-', "_")
}

async fn resolve_required_model_package_facts(
    host: &EmbeddedWorkflowHost,
    required_model_ids: &[String],
) -> Vec<inference::ResolvedModelPackageFacts> {
    let selector_access = host.pumas_selector_access().await;
    resolve_required_model_package_facts_from_selector_access(
        selector_access.as_deref(),
        required_model_ids,
    )
    .await
}

async fn resolve_required_model_package_facts_from_selector_access(
    selector_access: Option<&PumasSelectorAccess>,
    required_model_ids: &[String],
) -> Vec<inference::ResolvedModelPackageFacts> {
    match selector_access {
        Some(PumasSelectorAccess::Owner(api)) => {
            resolve_required_model_package_facts_from_api(Some(api.as_ref()), required_model_ids)
                .await
        }
        Some(PumasSelectorAccess::LocalClient(_)) => {
            log::warn!(
                "Pumas local-client selector access does not expose full package facts for technical-fit"
            );
            Vec::new()
        }
        Some(PumasSelectorAccess::ReadOnly(_)) => {
            log::warn!(
                "Pumas read-only selector access exposes package summaries, not full package facts for technical-fit"
            );
            Vec::new()
        }
        None => Vec::new(),
    }
}

async fn resolve_required_model_package_facts_from_api(
    api: Option<&pumas_library::PumasApi>,
    required_model_ids: &[String],
) -> Vec<inference::ResolvedModelPackageFacts> {
    let Some(api) = api else {
        return Vec::new();
    };

    let mut seen = HashSet::new();
    let mut resolved = Vec::new();
    for model_id in required_model_ids
        .iter()
        .map(|model_id| model_id.trim())
        .filter(|model_id| !model_id.is_empty())
    {
        if !seen.insert(model_id.to_string()) {
            continue;
        }

        match api.resolve_model_package_facts(model_id).await {
            Ok(facts) => match decode_inference_package_facts(&facts) {
                Ok(facts) => resolved.push(facts),
                Err(error) => {
                    log::warn!(
                        "Pumas package facts for '{}' did not match Pantograph inference contract: {}",
                        model_id,
                        error
                    );
                }
            },
            Err(error) => {
                log::warn!(
                    "Pumas package fact lookup failed during technical-fit for '{}': {}",
                    model_id,
                    error
                );
            }
        }
    }
    resolved
}

fn decode_inference_package_facts<T: serde::Serialize>(
    facts: &T,
) -> Result<inference::ResolvedModelPackageFacts, serde_json::Error> {
    serde_json::from_value(serde_json::to_value(facts)?)
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
    use std::sync::Arc;

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

    fn backend_info(
        backend_key: &str,
        artifact_kinds: Vec<inference::ModelArtifactKind>,
        backend_hints: Vec<inference::BackendHintLabel>,
    ) -> inference::BackendInfo {
        inference::BackendInfo {
            name: backend_key.to_string(),
            backend_key: backend_key.to_string(),
            description: "test backend".to_string(),
            capabilities: inference::BackendCapabilities {
                facts: inference::BackendCapabilityFacts {
                    tasks: vec![inference::BackendTaskCapability::stable(
                        inference::InferenceTaskId::TextGeneration,
                        vec![inference::InferenceModality::Text],
                        vec![inference::InferenceModality::Text],
                    )],
                    preprocessing: inference::BackendComponentCapability::RequiresPackageComponent,
                    postprocessing: inference::BackendComponentCapability::BackendManaged,
                    model_sources: inference::BackendModelSourceCapabilityFacts {
                        artifact_kinds,
                        backend_hints,
                        custom_code: inference::BackendFeatureSupport::Unsupported,
                    },
                    features: inference::BackendFeatureCapabilityFacts {
                        streaming: inference::BackendFeatureSupport::Supported,
                        device_selection: inference::BackendFeatureSupport::Supported,
                        external_connection: inference::BackendFeatureSupport::Unsupported,
                        kv_cache: inference::BackendFeatureSupport::Supported,
                    },
                },
                ..inference::BackendCapabilities::default()
            },
            default_start_mode: inference::backend::BackendDefaultStartMode::Inference,
            active: false,
            available: true,
            unavailable_reason: None,
            can_install: false,
            runtime_binary_id: None,
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
            selection_mode: RuntimeTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candidate-a".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: Some("model-a".to_string()),
            reasons: vec![RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::QueuePressure,
                Some("candidate-a"),
            )],
            compatibility_report: Some(RuntimeTechnicalFitCompatibilityReport {
                status: "rejected".to_string(),
                compatible: false,
                task: "supported".to_string(),
                model_source: "unsupported".to_string(),
                preprocessing: "supported".to_string(),
                postprocessing: "supported".to_string(),
            }),
            compatibility_issue_count: 1,
            compatibility_issues: vec![RuntimeTechnicalFitCompatibilityIssue {
                kind: "unsupported_model_artifact".to_string(),
                phase: "model_package_resolution".to_string(),
                message: "backend cannot load artifact".to_string(),
                model_id: Some("model-a".to_string()),
                path: Some("model.gguf".to_string()),
            }],
        };

        let projected = project_workflow_technical_fit_decision(&decision);

        assert_eq!(
            projected,
            WorkflowTechnicalFitDecision {
                selection_mode: WorkflowTechnicalFitSelectionMode::Automatic,
                selected_candidate_id: Some("candidate-a".to_string()),
                selected_runtime_id: Some("llama_cpp".to_string()),
                selected_backend_key: Some("llama_cpp".to_string()),
                selected_model_id: Some("model-a".to_string()),
                reasons: vec![WorkflowTechnicalFitReason {
                    code: WorkflowTechnicalFitReasonCode::QueuePressure,
                    candidate_id: Some("candidate-a".to_string()),
                }],
                compatibility_report: Some(WorkflowTechnicalFitCompatibilityReport {
                    status: "rejected".to_string(),
                    compatible: false,
                    task: "supported".to_string(),
                    model_source: "unsupported".to_string(),
                    preprocessing: "supported".to_string(),
                    postprocessing: "supported".to_string(),
                }),
                compatibility_issue_count: 1,
                compatibility_issues: vec![WorkflowTechnicalFitCompatibilityIssue {
                    kind: "unsupported_model_artifact".to_string(),
                    phase: "model_package_resolution".to_string(),
                    message: "backend cannot load artifact".to_string(),
                    model_id: Some("model-a".to_string()),
                    path: Some("model.gguf".to_string()),
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
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
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
    fn pumas_package_facts_candidates_use_backend_compatibility_reports() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
            ),
        )
        .expect("decode package facts fixture");
        let backends = vec![
            backend_info(
                "llama_cpp",
                vec![inference::ModelArtifactKind::Gguf],
                vec![inference::BackendHintLabel::LlamaCpp],
            ),
            backend_info(
                "pytorch",
                vec![inference::ModelArtifactKind::HfCompatibleDirectory],
                vec![inference::BackendHintLabel::Transformers],
            ),
        ];

        let candidates = runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
            &[package_facts],
            &backends,
        );

        let llama = candidates
            .iter()
            .find(|candidate| candidate.backend_key.as_deref() == Some("llama_cpp"))
            .expect("llama candidate");
        let pytorch = candidates
            .iter()
            .find(|candidate| candidate.backend_key.as_deref() == Some("pytorch"))
            .expect("pytorch candidate");

        assert!(llama.supports_runtime_requirements);
        assert_eq!(
            llama
                .compatibility_report
                .as_ref()
                .map(|report| report.status.as_str()),
            Some("accepted")
        );
        assert_eq!(llama.compatibility_issue_count, 0);
        assert!(!pytorch.supports_runtime_requirements);
        assert_eq!(
            pytorch
                .compatibility_report
                .as_ref()
                .map(|report| (report.status.as_str(), report.model_source.as_str())),
            Some(("rejected", "unsupported"))
        );
        assert!(pytorch
            .compatibility_issues
            .iter()
            .any(|issue| issue.kind == "unsupported_model_artifact"));
    }

    #[test]
    fn pumas_package_facts_candidates_reject_missing_package_components() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/missing_tokenizer_package_facts.json"
            ),
        )
        .expect("decode package facts fixture");
        let backends = vec![backend_info(
            "pytorch",
            vec![inference::ModelArtifactKind::HfCompatibleDirectory],
            vec![inference::BackendHintLabel::Transformers],
        )];

        let candidates = runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
            &[package_facts],
            &backends,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].backend_key.as_deref(), Some("pytorch"));
        assert!(!candidates[0].supports_runtime_requirements);
        assert_eq!(
            candidates[0]
                .compatibility_report
                .as_ref()
                .map(|report| report.preprocessing.as_str()),
            Some("unsupported")
        );
        assert!(candidates[0]
            .compatibility_issues
            .iter()
            .any(|issue| issue.kind == "missing_preprocessing_component"));
    }

    #[tokio::test]
    async fn required_model_package_facts_resolve_from_owner_selector_access() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let model_id = "llm/test/live-technical-fit-facts";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        std::fs::create_dir_all(&model_dir).expect("model dir");
        std::fs::write(
            model_dir.join("config.json"),
            r#"{"model_type":"llama","architectures":["LlamaForCausalLM"]}"#,
        )
        .expect("config");
        std::fs::write(model_dir.join("model.safetensors"), b"test").expect("weights");
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "model_id": model_id,
                "family": "test",
                "model_type": "llm",
                "official_name": "Live Technical Fit Facts",
                "cleaned_name": "live-technical-fit-facts",
                "files": [{"name": "model.safetensors"}],
                "runtime_engine_hints": ["transformers"]
            }))
            .expect("metadata json"),
        )
        .expect("metadata");
        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        let required_models = vec![model_id.to_string(), model_id.to_string(), " ".to_string()];
        let selector_access = PumasSelectorAccess::Owner(api);

        let facts = resolve_required_model_package_facts_from_selector_access(
            Some(&selector_access),
            &required_models,
        )
        .await;

        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].model_ref.model_id, model_id);
        assert_eq!(
            facts[0].artifact.artifact_kind,
            inference::ModelArtifactKind::HfCompatibleDirectory
        );
        assert_eq!(
            facts[0].backend_hints.accepted,
            vec![inference::BackendHintLabel::Transformers]
        );
    }

    #[tokio::test]
    async fn required_model_package_facts_do_not_promote_read_only_summaries() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models"))
            .expect("models dir");
        let api = pumas_library::PumasApi::builder(temp_dir.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .expect("pumas api");
        api.rebuild_model_index()
            .await
            .expect("model index rebuild");
        let read_only = pumas_library::PumasReadOnlyLibrary::open(
            temp_dir.path().join("shared-resources/models"),
        )
        .expect("read-only Pumas library");
        let selector_access = PumasSelectorAccess::ReadOnly(Arc::new(read_only));

        let facts = resolve_required_model_package_facts_from_selector_access(
            Some(&selector_access),
            &["llm/test/live-technical-fit-facts".to_string()],
        )
        .await;

        assert!(
            facts.is_empty(),
            "read-only package summaries must not be promoted to full technical-fit facts"
        );
    }

    #[test]
    fn inference_task_label_mapping_accepts_depth_estimation() {
        assert_eq!(
            inference_task_id_from_label("depth_estimation"),
            Some(inference::InferenceTaskId::DepthEstimation)
        );
        assert_eq!(
            inference_task_id_from_label("depth-estimation"),
            Some(inference::InferenceTaskId::DepthEstimation)
        );
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

    #[test]
    fn technical_fit_request_can_include_backend_checked_pumas_package_facts_candidates() {
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
        let backends = vec![
            backend_info(
                "llama_cpp",
                vec![inference::ModelArtifactKind::Gguf],
                vec![inference::BackendHintLabel::LlamaCpp],
            ),
            backend_info(
                "pytorch",
                vec![inference::ModelArtifactKind::HfCompatibleDirectory],
                vec![inference::BackendHintLabel::Transformers],
            ),
        ];

        let runtime_request = build_runtime_technical_fit_request_with_backend_package_facts(
            &workflow_request,
            None,
            &[],
            &backends,
            &[package_facts],
        );

        assert_eq!(runtime_request.candidates.len(), 2);
        let decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(
            decision.selected_model_id.as_deref(),
            Some("llm/llama/tiny-gguf")
        );
        assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(
            decision
                .compatibility_report
                .as_ref()
                .map(|report| report.status.as_str()),
            Some("accepted")
        );

        let workflow_decision = project_workflow_technical_fit_decision(&decision);
        assert_eq!(
            workflow_decision
                .compatibility_report
                .as_ref()
                .map(|report| report.status.as_str()),
            Some("accepted")
        );
    }
}
