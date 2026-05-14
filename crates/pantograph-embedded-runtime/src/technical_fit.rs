use std::collections::HashSet;

use pantograph_runtime_identity::canonical_runtime_backend_key;
use pantograph_runtime_registry::{
    select_runtime_technical_fit, RuntimeRegistrySnapshot, RuntimeTechnicalFitCandidate,
    RuntimeTechnicalFitCandidateSourceKind, RuntimeTechnicalFitCompatibilityIssue,
    RuntimeTechnicalFitCompatibilityReport, RuntimeTechnicalFitDecision,
    RuntimeTechnicalFitDecisionCode, RuntimeTechnicalFitDeviceClass,
    RuntimeTechnicalFitDeviceDiagnostic, RuntimeTechnicalFitDeviceDiagnosticCode,
    RuntimeTechnicalFitDeviceDiagnosticSeverity, RuntimeTechnicalFitDevicePolicy,
    RuntimeTechnicalFitFactor, RuntimeTechnicalFitHistoryThresholdState,
    RuntimeTechnicalFitObservedThroughputHint, RuntimeTechnicalFitOverride,
    RuntimeTechnicalFitPolicyPhase, RuntimeTechnicalFitReason, RuntimeTechnicalFitReasonCode,
    RuntimeTechnicalFitRequest, RuntimeTechnicalFitResidencyState,
    RuntimeTechnicalFitResourceEstimate, RuntimeTechnicalFitResourcePressure,
    RuntimeTechnicalFitSelectionMode, RuntimeTechnicalFitSelectionPolicyTrace,
    RuntimeTechnicalFitWarmupState,
};
use pantograph_workflow_service::{
    WorkflowDeviceResolutionDiagnostic, WorkflowDeviceResolutionDiagnosticCode,
    WorkflowDeviceResolutionDiagnosticSeverity, WorkflowHost, WorkflowInferenceDeviceClass,
    WorkflowRuntimeCapability, WorkflowRuntimeInstallState, WorkflowRuntimeSourceKind,
    WorkflowRuntimeVariantCapability, WorkflowServiceError, WorkflowTechnicalFitCompatibilityIssue,
    WorkflowTechnicalFitCompatibilityReport, WorkflowTechnicalFitDecision,
    WorkflowTechnicalFitDecisionCode, WorkflowTechnicalFitDeviceClass,
    WorkflowTechnicalFitDeviceDiagnostic, WorkflowTechnicalFitDeviceDiagnosticCode,
    WorkflowTechnicalFitDeviceDiagnosticSeverity, WorkflowTechnicalFitDevicePolicy,
    WorkflowTechnicalFitHistoryThresholdState, WorkflowTechnicalFitObservedThroughputHint,
    WorkflowTechnicalFitPolicyPhase, WorkflowTechnicalFitQueuePressure, WorkflowTechnicalFitReason,
    WorkflowTechnicalFitReasonCode, WorkflowTechnicalFitRequest,
    WorkflowTechnicalFitResourceEstimate, WorkflowTechnicalFitSelectionMode,
    WorkflowTechnicalFitSelectionPolicyTrace,
};
use workflow_nodes::setup::PumasSelectorAccess;

use crate::{workflow_runtime::unix_timestamp_ms, EmbeddedWorkflowHost};

const MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES: usize = 32;
const MAX_RUNTIME_TECHNICAL_FIT_CANDIDATES: usize = 512;

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
        runtime_candidates_from_pumas_package_facts_with_runtime_capabilities(
            package_facts,
            runtime_capabilities,
            runtime_requirements_resource_estimate(&request.runtime_requirements),
        )
    } else {
        runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
            package_facts,
            available_backends,
            runtime_capabilities,
            runtime_requirements_resource_estimate(&request.runtime_requirements),
        )
    };
    runtime_request.candidates.extend(package_fact_candidates);
    runtime_request_with_candidate_cap(runtime_request)
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
    let runtime_request = build_runtime_technical_fit_request_for_resolved_package_facts(
        request,
        runtime_snapshot,
        &runtime_capabilities,
        &available_backends,
        &package_facts,
    );
    let decision = select_runtime_technical_fit(&runtime_request);
    Ok(Some(project_workflow_technical_fit_decision(&decision)))
}

fn build_runtime_technical_fit_request_for_resolved_package_facts(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
    available_backends: &[inference::BackendInfo],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> RuntimeTechnicalFitRequest {
    let missing_package_fact_candidates = missing_required_model_package_fact_candidates(
        &request.runtime_requirements.required_models,
        package_facts,
    );
    if !missing_package_fact_candidates.is_empty() {
        let mut runtime_request =
            build_runtime_technical_fit_request(request, runtime_snapshot, &[]);
        runtime_request.candidates = missing_package_fact_candidates;
        return runtime_request_with_candidate_cap(runtime_request);
    }

    if package_facts.is_empty() {
        return build_runtime_technical_fit_request(
            request,
            runtime_snapshot,
            runtime_capabilities,
        );
    }

    build_runtime_technical_fit_request_with_backend_package_facts(
        request,
        runtime_snapshot,
        runtime_capabilities,
        available_backends,
        package_facts,
    )
}

pub fn build_runtime_technical_fit_request(
    request: &WorkflowTechnicalFitRequest,
    runtime_snapshot: Option<RuntimeRegistrySnapshot>,
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> RuntimeTechnicalFitRequest {
    runtime_request_with_candidate_cap(RuntimeTechnicalFitRequest {
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
        device_policy: request.device_policy.as_ref().map(project_device_policy),
        legal_factors: RuntimeTechnicalFitFactor::all().to_vec(),
        candidates: runtime_capability_candidates(
            runtime_capabilities,
            runtime_requirements_resource_estimate(&request.runtime_requirements),
        ),
        resource_pressure: project_resource_pressure(
            request.queue_pressure.as_ref(),
            request.runtime_requirements.estimated_peak_vram_mb,
            request.runtime_requirements.estimated_peak_ram_mb,
        ),
    })
}

fn runtime_request_with_candidate_cap(
    mut request: RuntimeTechnicalFitRequest,
) -> RuntimeTechnicalFitRequest {
    if request.candidates.len() <= MAX_RUNTIME_TECHNICAL_FIT_CANDIDATES {
        return request.normalized();
    }

    let candidate_count = request.candidates.len();
    request.candidates = vec![candidate_set_overflow_candidate(candidate_count)];
    request.normalized()
}

pub fn project_workflow_technical_fit_decision(
    decision: &RuntimeTechnicalFitDecision,
) -> WorkflowTechnicalFitDecision {
    WorkflowTechnicalFitDecision {
        selection_mode: project_selection_mode(decision.selection_mode),
        selected_candidate_id: decision.selected_candidate_id.clone(),
        selected_runtime_id: decision.selected_runtime_id.clone(),
        selected_runtime_variant_id: decision.selected_runtime_variant_id.clone(),
        selected_backend_key: decision.selected_backend_key.clone(),
        selected_model_id: decision.selected_model_id.clone(),
        selected_device_class: decision
            .selected_device_class
            .map(project_runtime_device_class),
        selected_device_id: decision.selected_device_id.clone(),
        resource_estimate: decision
            .resource_estimate
            .as_ref()
            .map(project_resource_estimate),
        observed_throughput_hint: decision
            .observed_throughput_hint
            .as_ref()
            .map(project_observed_throughput_hint),
        device_diagnostics: decision
            .device_diagnostics
            .iter()
            .map(project_device_diagnostic)
            .collect(),
        reasons: decision
            .reasons
            .iter()
            .map(project_reason)
            .collect::<Vec<_>>(),
        selection_policy_trace: decision
            .selection_policy_trace
            .as_ref()
            .map(project_selection_policy_trace),
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

fn project_selection_policy_trace(
    trace: &RuntimeTechnicalFitSelectionPolicyTrace,
) -> WorkflowTechnicalFitSelectionPolicyTrace {
    WorkflowTechnicalFitSelectionPolicyTrace {
        policy_version: trace.policy_version,
        policy_phase: trace.policy_phase.map(project_policy_phase),
        decision_code: trace.decision_code.map(project_decision_code),
        history_threshold_state: trace
            .history_threshold_state
            .map(project_history_threshold_state),
        candidate_set_summary: trace.candidate_set_summary.as_ref().map(|summary| {
            pantograph_workflow_service::WorkflowTechnicalFitCandidateSetSummary {
                total_candidate_count: summary.total_candidate_count,
                eligible_candidate_count: summary.eligible_candidate_count,
                rejected_candidate_count: summary.rejected_candidate_count,
                eligible_candidate_ids: summary.eligible_candidate_ids.clone(),
            }
        }),
        ranking_reason: trace.ranking_reason.clone(),
        exploration_reason: trace.exploration_reason.clone(),
        seed_basis: trace.seed_basis.clone(),
    }
    .normalized()
}

fn project_policy_phase(phase: RuntimeTechnicalFitPolicyPhase) -> WorkflowTechnicalFitPolicyPhase {
    match phase {
        RuntimeTechnicalFitPolicyPhase::CandidateRanking => {
            WorkflowTechnicalFitPolicyPhase::CandidateRanking
        }
    }
}

fn project_decision_code(
    code: RuntimeTechnicalFitDecisionCode,
) -> WorkflowTechnicalFitDecisionCode {
    match code {
        RuntimeTechnicalFitDecisionCode::SelectedCandidate => {
            WorkflowTechnicalFitDecisionCode::SelectedCandidate
        }
    }
}

fn project_history_threshold_state(
    state: RuntimeTechnicalFitHistoryThresholdState,
) -> WorkflowTechnicalFitHistoryThresholdState {
    match state {
        RuntimeTechnicalFitHistoryThresholdState::NotEvaluated => {
            WorkflowTechnicalFitHistoryThresholdState::NotEvaluated
        }
    }
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
    resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
) -> Vec<RuntimeTechnicalFitCandidate> {
    runtime_capabilities
        .iter()
        .flat_map(|capability| {
            let resource_estimate = resource_estimate.clone();
            let backend_key = capability
                .backend_keys
                .first()
                .cloned()
                .or_else(|| Some(capability.runtime_id.clone()));
            let variant_entries = runtime_capability_variant_fact_entries(capability);
            let multi_variant = variant_entries.len() > 1;
            variant_entries
                .into_iter()
                .map(move |runtime_variant_facts| {
                    let supports_runtime_requirements = runtime_capability_is_ready(capability)
                        && runtime_capability_variant_is_ready(&runtime_variant_facts);
                    RuntimeTechnicalFitCandidate {
                        candidate_id: runtime_capability_candidate_id(
                            capability,
                            backend_key.as_deref(),
                            runtime_variant_facts.runtime_variant_id.as_deref(),
                            multi_variant,
                        ),
                        runtime_id: Some(capability.runtime_id.clone()),
                        runtime_variant_id: runtime_variant_facts.runtime_variant_id.clone(),
                        backend_key: backend_key.clone(),
                        model_id: None,
                        device_class: runtime_variant_facts.device_class,
                        selected_device_id: None,
                        resource_estimate: resource_estimate.clone(),
                        observed_throughput_hint: None,
                        device_diagnostics: runtime_variant_facts.device_diagnostics,
                        source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
                        context_window_tokens: None,
                        residency_state: Some(runtime_capability_residency_state(capability)),
                        warmup_state: runtime_capability_warmup_state(capability),
                        supports_runtime_requirements,
                        compatibility_report: None,
                        compatibility_issue_count: 0,
                        compatibility_issues: Vec::new(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn runtime_capability_candidate_id(
    capability: &WorkflowRuntimeCapability,
    backend_key: Option<&str>,
    runtime_variant_id: Option<&str>,
    multi_variant: bool,
) -> String {
    let base = backend_key.unwrap_or(capability.runtime_id.as_str());
    if multi_variant {
        if let Some(runtime_variant_id) = runtime_variant_id {
            return format!("{}|{}|{}", base, capability.runtime_id, runtime_variant_id);
        }
    }
    base.to_string()
}

fn runtime_capability_variant_is_ready(variant_facts: &RuntimeCapabilityVariantFacts) -> bool {
    variant_facts.available
        && variant_facts.runtime_variant_id.is_some()
        && variant_facts.device_class.is_some()
}

struct RuntimeCapabilityVariantFacts {
    runtime_variant_id: Option<String>,
    device_class: Option<RuntimeTechnicalFitDeviceClass>,
    available: bool,
    device_diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
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
                    runtime_variant_id: None,
                    backend_key: Some(backend_key.clone()),
                    model_id: Some(facts.model_ref.model_id.clone()),
                    device_class: None,
                    selected_device_id: None,
                    resource_estimate: None,
                    observed_throughput_hint: None,
                    device_diagnostics: vec![missing_runtime_capability_diagnostic(
                        &backend_key,
                        &facts.model_ref.model_id,
                    )],
                    source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                    context_window_tokens: None,
                    residency_state: None,
                    warmup_state: None,
                    supports_runtime_requirements: false,
                    compatibility_report: None,
                    compatibility_issue_count: 0,
                    compatibility_issues: Vec::new(),
                })
        })
        .collect()
}

pub fn runtime_candidates_from_pumas_package_facts_with_runtime_capabilities(
    package_facts: &[inference::ResolvedModelPackageFacts],
    runtime_capabilities: &[WorkflowRuntimeCapability],
    resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
) -> Vec<RuntimeTechnicalFitCandidate> {
    package_facts
        .iter()
        .flat_map(|facts| {
            facts
                .backend_hints
                .accepted
                .iter()
                .filter_map(|hint| pumas_backend_hint_label_to_backend_key(*hint))
                .flat_map(|backend_key| {
                    runtime_capability_for_backend(runtime_capabilities, &backend_key)
                        .map(|capability| {
                            runtime_capability_variant_fact_entries(capability)
                                .into_iter()
                                .map(|variant_facts| {
                                    pumas_runtime_candidate_from_parts(
                                        facts,
                                        &backend_key,
                                        None,
                                        Some(capability),
                                        variant_facts,
                                        resource_estimate.clone(),
                                        matches!(
                                            facts.artifact.validation_state,
                                            inference::ModelValidationState::Valid
                                        ),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_else(|| {
                            vec![pumas_runtime_candidate_without_capability(
                                facts,
                                &backend_key,
                                None,
                            )]
                        })
                })
        })
        .collect()
}

pub fn runtime_candidates_from_pumas_package_facts_with_backend_capabilities(
    package_facts: &[inference::ResolvedModelPackageFacts],
    available_backends: &[inference::BackendInfo],
    runtime_capabilities: &[WorkflowRuntimeCapability],
    resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
) -> Vec<RuntimeTechnicalFitCandidate> {
    package_facts
        .iter()
        .filter_map(|facts| task_registry_entry_from_package_facts(facts).map(|task| (facts, task)))
        .flat_map(|(facts, task)| {
            let resource_estimate = resource_estimate.clone();
            available_backends.iter().flat_map(move |backend| {
                let compatibility = backend.capabilities.check_model_compatibility(
                    Some(&backend.backend_key),
                    inference::BackendCompatibilityRequest::new(&task, facts),
                );
                let compatibility_report = Some(runtime_compatibility_report(&compatibility));
                let compatibility_issue_count =
                    compatibility.issues.len().min(u32::MAX as usize) as u32;
                let compatibility_issues = runtime_compatibility_issues(
                    &compatibility,
                    MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES,
                );
                runtime_capability_for_backend(runtime_capabilities, &backend.backend_key)
                    .map(|capability| {
                        runtime_capability_variant_fact_entries(capability)
                            .into_iter()
                            .map(|variant_facts| {
                                pumas_runtime_candidate_from_parts(
                                    facts,
                                    &backend.backend_key,
                                    Some((
                                        compatibility_report.clone(),
                                        compatibility_issue_count,
                                        compatibility_issues.clone(),
                                    )),
                                    Some(capability),
                                    variant_facts,
                                    resource_estimate.clone(),
                                    compatibility.compatible,
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| {
                        vec![pumas_runtime_candidate_without_capability(
                            facts,
                            &backend.backend_key,
                            Some((
                                compatibility_report,
                                compatibility_issue_count,
                                compatibility_issues,
                            )),
                        )]
                    })
            })
        })
        .collect()
}

fn pumas_runtime_candidate_from_parts(
    facts: &inference::ResolvedModelPackageFacts,
    backend_key: &str,
    compatibility: Option<(
        Option<RuntimeTechnicalFitCompatibilityReport>,
        u32,
        Vec<RuntimeTechnicalFitCompatibilityIssue>,
    )>,
    capability: Option<&WorkflowRuntimeCapability>,
    variant_facts: RuntimeCapabilityVariantFacts,
    resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
    supports_model: bool,
) -> RuntimeTechnicalFitCandidate {
    let (compatibility_report, compatibility_issue_count, compatibility_issues) =
        compatibility.unwrap_or((None, 0, Vec::new()));
    let runtime_id = capability.map(|capability| capability.runtime_id.clone());
    let runtime_ready = capability.is_some_and(runtime_capability_is_ready);
    let variant_ready = variant_facts.available
        && variant_facts.runtime_variant_id.is_some()
        && variant_facts.device_class.is_some();
    let candidate_id = pumas_candidate_id(
        backend_key,
        &facts.model_ref.model_id,
        runtime_id.as_deref(),
        variant_facts.runtime_variant_id.as_deref(),
    );

    RuntimeTechnicalFitCandidate {
        candidate_id,
        runtime_id,
        runtime_variant_id: variant_facts.runtime_variant_id,
        backend_key: Some(backend_key.to_string()),
        model_id: Some(facts.model_ref.model_id.clone()),
        device_class: variant_facts.device_class,
        selected_device_id: None,
        resource_estimate,
        observed_throughput_hint: None,
        device_diagnostics: variant_facts.device_diagnostics,
        source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
        context_window_tokens: None,
        residency_state: capability.map(runtime_capability_residency_state),
        warmup_state: capability.and_then(runtime_capability_warmup_state),
        supports_runtime_requirements: supports_model && runtime_ready && variant_ready,
        compatibility_report,
        compatibility_issue_count,
        compatibility_issues,
    }
}

fn pumas_runtime_candidate_without_capability(
    facts: &inference::ResolvedModelPackageFacts,
    backend_key: &str,
    compatibility: Option<(
        Option<RuntimeTechnicalFitCompatibilityReport>,
        u32,
        Vec<RuntimeTechnicalFitCompatibilityIssue>,
    )>,
) -> RuntimeTechnicalFitCandidate {
    let (compatibility_report, compatibility_issue_count, compatibility_issues) =
        compatibility.unwrap_or((None, 0, Vec::new()));
    let compatibility_rejected = compatibility_report
        .as_ref()
        .is_some_and(|report| !report.compatible)
        || compatibility_issue_count > 0
        || !compatibility_issues.is_empty();
    RuntimeTechnicalFitCandidate {
        candidate_id: pumas_candidate_id(backend_key, &facts.model_ref.model_id, None, None),
        runtime_id: None,
        runtime_variant_id: None,
        backend_key: Some(backend_key.to_string()),
        model_id: Some(facts.model_ref.model_id.clone()),
        device_class: None,
        selected_device_id: None,
        resource_estimate: None,
        observed_throughput_hint: None,
        device_diagnostics: if compatibility_rejected {
            Vec::new()
        } else {
            vec![missing_runtime_capability_diagnostic(
                backend_key,
                &facts.model_ref.model_id,
            )]
        },
        source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
        context_window_tokens: None,
        residency_state: None,
        warmup_state: None,
        supports_runtime_requirements: false,
        compatibility_report,
        compatibility_issue_count,
        compatibility_issues,
    }
}

fn missing_required_model_package_fact_candidates(
    required_model_ids: &[String],
    package_facts: &[inference::ResolvedModelPackageFacts],
) -> Vec<RuntimeTechnicalFitCandidate> {
    let resolved_model_ids = package_facts
        .iter()
        .map(|facts| facts.model_ref.model_id.trim().to_string())
        .filter(|model_id| !model_id.is_empty())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    required_model_ids
        .iter()
        .map(|model_id| model_id.trim())
        .filter(|model_id| !model_id.is_empty())
        .filter(|model_id| seen.insert((*model_id).to_string()))
        .filter(|model_id| !resolved_model_ids.contains(*model_id))
        .map(missing_model_package_facts_candidate)
        .collect()
}

fn missing_model_package_facts_candidate(model_id: &str) -> RuntimeTechnicalFitCandidate {
    RuntimeTechnicalFitCandidate {
        candidate_id: format!("missing_model_package_facts|{}", model_id),
        runtime_id: None,
        runtime_variant_id: None,
        backend_key: None,
        model_id: Some(model_id.to_string()),
        device_class: None,
        selected_device_id: None,
        resource_estimate: None,
        observed_throughput_hint: None,
        device_diagnostics: vec![missing_model_package_facts_diagnostic(model_id)],
        source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
        context_window_tokens: None,
        residency_state: None,
        warmup_state: None,
        supports_runtime_requirements: false,
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    }
}

fn missing_model_package_facts_diagnostic(model_id: &str) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: format!(
            "required model '{}' did not resolve to Pumas package facts for technical-fit planning",
            model_id
        ),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }
}

fn candidate_set_overflow_candidate(candidate_count: usize) -> RuntimeTechnicalFitCandidate {
    RuntimeTechnicalFitCandidate {
        candidate_id: "candidate_set_overflow".to_string(),
        runtime_id: None,
        runtime_variant_id: None,
        backend_key: None,
        model_id: None,
        device_class: None,
        selected_device_id: None,
        resource_estimate: None,
        observed_throughput_hint: None,
        device_diagnostics: vec![candidate_set_overflow_diagnostic(candidate_count)],
        source_kind: RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts,
        context_window_tokens: None,
        residency_state: None,
        warmup_state: None,
        supports_runtime_requirements: false,
        compatibility_report: None,
        compatibility_issue_count: 0,
        compatibility_issues: Vec::new(),
    }
}

fn candidate_set_overflow_diagnostic(
    candidate_count: usize,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: format!(
            "technical-fit candidate synthesis produced {} candidates, exceeding the documented cap of {}",
            candidate_count, MAX_RUNTIME_TECHNICAL_FIT_CANDIDATES
        ),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
    }
}

fn pumas_candidate_id(
    backend_key: &str,
    model_id: &str,
    runtime_id: Option<&str>,
    runtime_variant_id: Option<&str>,
) -> String {
    let mut parts = vec![backend_key.to_string(), model_id.to_string()];
    if let Some(runtime_id) = runtime_id {
        parts.push(runtime_id.to_string());
    }
    if let Some(runtime_variant_id) = runtime_variant_id {
        parts.push(runtime_variant_id.to_string());
    }
    parts.join("|")
}

fn runtime_capability_for_backend<'a>(
    runtime_capabilities: &'a [WorkflowRuntimeCapability],
    backend_key: &str,
) -> Option<&'a WorkflowRuntimeCapability> {
    let normalized_backend_key = canonical_runtime_backend_key(backend_key);
    runtime_capabilities.iter().find(|capability| {
        canonical_runtime_backend_key(&capability.runtime_id) == normalized_backend_key
            || capability
                .backend_keys
                .iter()
                .any(|candidate| canonical_runtime_backend_key(candidate) == normalized_backend_key)
    })
}

fn runtime_capability_variant_fact_entries(
    capability: &WorkflowRuntimeCapability,
) -> Vec<RuntimeCapabilityVariantFacts> {
    let variants = capability
        .backend_capability_facts
        .as_ref()
        .map(|facts| facts.runtime_variants.as_slice())
        .unwrap_or_default();

    if variants.is_empty() {
        return vec![RuntimeCapabilityVariantFacts {
            runtime_variant_id: None,
            device_class: None,
            available: false,
            device_diagnostics: vec![missing_runtime_variant_diagnostic(capability)],
        }];
    }

    variants
        .iter()
        .map(|variant| runtime_capability_variant_facts_from_variant(capability, variant))
        .collect()
}

fn runtime_capability_variant_facts_from_variant(
    capability: &WorkflowRuntimeCapability,
    variant: &WorkflowRuntimeVariantCapability,
) -> RuntimeCapabilityVariantFacts {
    let mut device_diagnostics = variant
        .diagnostics
        .iter()
        .map(project_workflow_device_diagnostic)
        .collect::<Vec<_>>();
    let device_class = project_workflow_runtime_variant_device_class(variant.device_class);
    if device_class.is_none() {
        device_diagnostics.push(RuntimeTechnicalFitDeviceDiagnostic {
            code: RuntimeTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass,
            severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
            message: "runtime variant reported an unsupported device class".to_string(),
            device_class: None,
            device_id: None,
            runtime_variant_id: Some(variant.runtime_variant_id.clone()),
            backend_key: capability.backend_keys.first().cloned(),
        });
    }

    RuntimeCapabilityVariantFacts {
        runtime_variant_id: Some(variant.runtime_variant_id.clone()),
        device_class,
        available: variant.available,
        device_diagnostics,
    }
}

fn missing_runtime_capability_diagnostic(
    backend_key: &str,
    model_id: &str,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: format!(
            "no runtime capability facts are available for backend '{}' while planning model '{}'",
            backend_key, model_id
        ),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: Some(backend_key.to_string()),
    }
}

fn missing_runtime_variant_diagnostic(
    capability: &WorkflowRuntimeCapability,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: "runtime capability did not report a runtime variant".to_string(),
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: capability.backend_keys.first().cloned(),
    }
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
        runtime_id: override_selection.runtime_id.clone(),
        runtime_variant_id: override_selection.runtime_variant_id.clone(),
        model_id: override_selection.model_id.clone(),
        backend_key: override_selection.backend_key.clone(),
    }
    .normalized()
}

fn project_device_policy(
    device_policy: &WorkflowTechnicalFitDevicePolicy,
) -> RuntimeTechnicalFitDevicePolicy {
    match device_policy {
        WorkflowTechnicalFitDevicePolicy::Auto => RuntimeTechnicalFitDevicePolicy::Auto,
        WorkflowTechnicalFitDevicePolicy::Explicit {
            device_class,
            device_id,
        } => RuntimeTechnicalFitDevicePolicy::Explicit {
            device_class: project_device_class(*device_class),
            device_id: device_id.clone(),
        },
    }
    .normalized()
}

fn project_device_class(
    device_class: WorkflowTechnicalFitDeviceClass,
) -> RuntimeTechnicalFitDeviceClass {
    match device_class {
        WorkflowTechnicalFitDeviceClass::Cpu => RuntimeTechnicalFitDeviceClass::Cpu,
        WorkflowTechnicalFitDeviceClass::Cuda => RuntimeTechnicalFitDeviceClass::Cuda,
        WorkflowTechnicalFitDeviceClass::Metal => RuntimeTechnicalFitDeviceClass::Metal,
        WorkflowTechnicalFitDeviceClass::Mps => RuntimeTechnicalFitDeviceClass::Mps,
    }
}

fn project_runtime_device_class(
    device_class: RuntimeTechnicalFitDeviceClass,
) -> WorkflowTechnicalFitDeviceClass {
    match device_class {
        RuntimeTechnicalFitDeviceClass::Cpu => WorkflowTechnicalFitDeviceClass::Cpu,
        RuntimeTechnicalFitDeviceClass::Cuda => WorkflowTechnicalFitDeviceClass::Cuda,
        RuntimeTechnicalFitDeviceClass::Metal => WorkflowTechnicalFitDeviceClass::Metal,
        RuntimeTechnicalFitDeviceClass::Mps => WorkflowTechnicalFitDeviceClass::Mps,
    }
}

fn project_workflow_runtime_variant_device_class(
    device_class: WorkflowInferenceDeviceClass,
) -> Option<RuntimeTechnicalFitDeviceClass> {
    match device_class {
        WorkflowInferenceDeviceClass::Cpu => Some(RuntimeTechnicalFitDeviceClass::Cpu),
        WorkflowInferenceDeviceClass::Cuda => Some(RuntimeTechnicalFitDeviceClass::Cuda),
        WorkflowInferenceDeviceClass::Metal => Some(RuntimeTechnicalFitDeviceClass::Metal),
        WorkflowInferenceDeviceClass::Mps => Some(RuntimeTechnicalFitDeviceClass::Mps),
        WorkflowInferenceDeviceClass::Unknown => None,
    }
}

fn project_resource_estimate(
    estimate: &RuntimeTechnicalFitResourceEstimate,
) -> WorkflowTechnicalFitResourceEstimate {
    WorkflowTechnicalFitResourceEstimate {
        estimated_peak_vram_mb: estimate.estimated_peak_vram_mb,
        estimated_peak_ram_mb: estimate.estimated_peak_ram_mb,
        estimated_min_vram_mb: estimate.estimated_min_vram_mb,
        estimated_min_ram_mb: estimate.estimated_min_ram_mb,
    }
}

fn project_observed_throughput_hint(
    hint: &RuntimeTechnicalFitObservedThroughputHint,
) -> WorkflowTechnicalFitObservedThroughputHint {
    WorkflowTechnicalFitObservedThroughputHint {
        tokens_per_second_milli: hint.tokens_per_second_milli,
        images_per_second_milli: hint.images_per_second_milli,
        sample_count: hint.sample_count,
    }
}

fn project_device_diagnostic(
    diagnostic: &RuntimeTechnicalFitDeviceDiagnostic,
) -> WorkflowTechnicalFitDeviceDiagnostic {
    WorkflowTechnicalFitDeviceDiagnostic {
        code: project_device_diagnostic_code(diagnostic.code),
        severity: project_device_diagnostic_severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        device_class: diagnostic.device_class.map(project_runtime_device_class),
        device_id: diagnostic.device_id.clone(),
        runtime_variant_id: diagnostic.runtime_variant_id.clone(),
        backend_key: diagnostic.backend_key.clone(),
    }
}

fn project_device_diagnostic_code(
    code: RuntimeTechnicalFitDeviceDiagnosticCode,
) -> WorkflowTechnicalFitDeviceDiagnosticCode {
    match code {
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDeviceId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidDeviceId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::InvalidBackendId => {
            WorkflowTechnicalFitDeviceDiagnosticCode::InvalidBackendId
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable => {
            WorkflowTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate => {
            WorkflowTechnicalFitDeviceDiagnosticCode::NoValidCandidate
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution => {
            WorkflowTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible => {
            WorkflowTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass => {
            WorkflowTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant => {
            WorkflowTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts => {
            WorkflowTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow => {
            WorkflowTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow
        }
        RuntimeTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected => {
            WorkflowTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected
        }
    }
}

fn project_device_diagnostic_severity(
    severity: RuntimeTechnicalFitDeviceDiagnosticSeverity,
) -> WorkflowTechnicalFitDeviceDiagnosticSeverity {
    match severity {
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Advisory => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Advisory
        }
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Warning
        }
        RuntimeTechnicalFitDeviceDiagnosticSeverity::Error => {
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
        }
    }
}

fn project_workflow_device_diagnostic(
    diagnostic: &WorkflowDeviceResolutionDiagnostic,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: project_workflow_device_diagnostic_code(diagnostic.code),
        severity: project_workflow_device_diagnostic_severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        device_class: diagnostic
            .device_class
            .and_then(project_workflow_runtime_variant_device_class),
        device_id: diagnostic.device_id.clone(),
        runtime_variant_id: diagnostic.runtime_variant_id.clone(),
        backend_key: diagnostic.backend_id.clone(),
    }
}

fn project_workflow_device_diagnostic_code(
    code: WorkflowDeviceResolutionDiagnosticCode,
) -> RuntimeTechnicalFitDeviceDiagnosticCode {
    match code {
        WorkflowDeviceResolutionDiagnosticCode::Unknown
        | WorkflowDeviceResolutionDiagnosticCode::NoValidCandidate => {
            RuntimeTechnicalFitDeviceDiagnosticCode::NoValidCandidate
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidDevicePolicy => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDevicePolicy
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidDeviceId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidDeviceId
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidRuntimeVariantId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidRuntimeVariantId
        }
        WorkflowDeviceResolutionDiagnosticCode::InvalidBackendId => {
            RuntimeTechnicalFitDeviceDiagnosticCode::InvalidBackendId
        }
        WorkflowDeviceResolutionDiagnosticCode::CandidateUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        }
        WorkflowDeviceResolutionDiagnosticCode::ExplicitDeviceUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::ExplicitDeviceUnavailable
        }
        WorkflowDeviceResolutionDiagnosticCode::AmbiguousAutoResolution => {
            RuntimeTechnicalFitDeviceDiagnosticCode::AmbiguousAutoResolution
        }
        WorkflowDeviceResolutionDiagnosticCode::BackendIncompatible => {
            RuntimeTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        }
        WorkflowDeviceResolutionDiagnosticCode::UnsupportedDeviceClass => {
            RuntimeTechnicalFitDeviceDiagnosticCode::UnsupportedDeviceClass
        }
        WorkflowDeviceResolutionDiagnosticCode::MissingRuntimeVariant => {
            RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant
        }
        WorkflowDeviceResolutionDiagnosticCode::LegacyDeviceRejected => {
            RuntimeTechnicalFitDeviceDiagnosticCode::LegacyDeviceRejected
        }
    }
}

fn project_workflow_device_diagnostic_severity(
    severity: WorkflowDeviceResolutionDiagnosticSeverity,
) -> RuntimeTechnicalFitDeviceDiagnosticSeverity {
    match severity {
        WorkflowDeviceResolutionDiagnosticSeverity::Unknown
        | WorkflowDeviceResolutionDiagnosticSeverity::Error => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
        }
        WorkflowDeviceResolutionDiagnosticSeverity::Advisory => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Advisory
        }
        WorkflowDeviceResolutionDiagnosticSeverity::Warning => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning
        }
    }
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

fn runtime_requirements_resource_estimate(
    requirements: &pantograph_workflow_service::WorkflowRuntimeRequirements,
) -> Option<RuntimeTechnicalFitResourceEstimate> {
    RuntimeTechnicalFitResourceEstimate {
        estimated_peak_vram_mb: requirements.estimated_peak_vram_mb,
        estimated_peak_ram_mb: requirements.estimated_peak_ram_mb,
        estimated_min_vram_mb: requirements.estimated_min_vram_mb,
        estimated_min_ram_mb: requirements.estimated_min_ram_mb,
    }
    .normalized()
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
        RuntimeTechnicalFitReasonCode::ExplicitRuntimeOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitRuntimeOverride
        }
        RuntimeTechnicalFitReasonCode::ExplicitRuntimeVariantOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitRuntimeVariantOverride
        }
        RuntimeTechnicalFitReasonCode::ExplicitModelOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitModelOverride
        }
        RuntimeTechnicalFitReasonCode::ExplicitBackendOverride => {
            WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
        }
        RuntimeTechnicalFitReasonCode::AutomaticRanking => {
            WorkflowTechnicalFitReasonCode::AutomaticRanking
        }
        RuntimeTechnicalFitReasonCode::ControlledExploration => {
            WorkflowTechnicalFitReasonCode::ControlledExploration
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
        build_workflow_technical_fit_request, WorkflowBackendCapabilityFacts,
        WorkflowRuntimeReadinessState, WorkflowRuntimeRequirements,
        WorkflowRuntimeVariantCapability,
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
            backend_capability_facts: Some(WorkflowBackendCapabilityFacts {
                tasks: Vec::new(),
                runtime_variants: vec![WorkflowRuntimeVariantCapability {
                    runtime_variant_id: "llama_cpp.cuda".to_string(),
                    device_class: WorkflowInferenceDeviceClass::Cuda,
                    available: true,
                    diagnostics: vec![WorkflowDeviceResolutionDiagnostic {
                        code: WorkflowDeviceResolutionDiagnosticCode::CandidateUnavailable,
                        severity: WorkflowDeviceResolutionDiagnosticSeverity::Warning,
                        message: "cuda runtime warmup pending".to_string(),
                        device_class: Some(WorkflowInferenceDeviceClass::Cuda),
                        device_id: Some("cuda:0".to_string()),
                        runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                        backend_id: Some("llama_cpp".to_string()),
                    }],
                }],
                preprocessing: Default::default(),
                postprocessing: Default::default(),
                model_sources: Default::default(),
                features: Default::default(),
                request_lifecycle: Default::default(),
            }),
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
                    runtime_variants: Vec::new(),
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
        let mut workflow_request = build_workflow_technical_fit_request(
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
                runtime_id: Some("llama.cpp".to_string()),
                runtime_variant_id: Some("llama_cpp.cuda".to_string()),
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
        workflow_request.device_policy = Some(WorkflowTechnicalFitDevicePolicy::Explicit {
            device_class: WorkflowTechnicalFitDeviceClass::Cuda,
            device_id: Some("cuda:0".to_string()),
        });

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &[runtime_capability()]);

        assert_eq!(runtime_request.workflow_id.as_deref(), Some("workflow-a"));
        assert_eq!(runtime_request.required_model_ids, vec!["model-a"]);
        assert_eq!(runtime_request.required_backend_keys, vec!["llama_cpp"]);
        assert_eq!(runtime_request.required_extensions, vec!["kv_cache"]);
        assert_eq!(
            runtime_request.override_selection,
            Some(RuntimeTechnicalFitOverride {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                model_id: Some("model-a".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            })
        );
        assert_eq!(
            runtime_request.device_policy,
            Some(RuntimeTechnicalFitDevicePolicy::Explicit {
                device_class: RuntimeTechnicalFitDeviceClass::Cuda,
                device_id: Some("cuda:0".to_string()),
            })
        );
        assert_eq!(runtime_request.candidates.len(), 1);
        assert_eq!(runtime_request.candidates[0].candidate_id, "llama_cpp");
        assert_eq!(
            runtime_request.candidates[0].runtime_variant_id.as_deref(),
            Some("llama_cpp.cuda")
        );
        assert_eq!(
            runtime_request.candidates[0].device_class,
            Some(RuntimeTechnicalFitDeviceClass::Cuda)
        );
        assert_eq!(
            runtime_request.candidates[0]
                .resource_estimate
                .as_ref()
                .and_then(|estimate| estimate.estimated_peak_vram_mb),
            Some(4096)
        );
        assert_eq!(
            runtime_request.candidates[0].device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        );
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
    fn runtime_request_projection_emits_all_runtime_variant_candidates() {
        let mut capability = runtime_capability();
        let backend_facts = capability
            .backend_capability_facts
            .as_mut()
            .expect("test capability should include backend facts");
        backend_facts.runtime_variants = vec![
            WorkflowRuntimeVariantCapability {
                runtime_variant_id: "llama_cpp.cpu".to_string(),
                device_class: WorkflowInferenceDeviceClass::Cpu,
                available: true,
                diagnostics: Vec::new(),
            },
            WorkflowRuntimeVariantCapability {
                runtime_variant_id: "llama_cpp.cuda".to_string(),
                device_class: WorkflowInferenceDeviceClass::Cuda,
                available: false,
                diagnostics: vec![WorkflowDeviceResolutionDiagnostic {
                    code: WorkflowDeviceResolutionDiagnosticCode::CandidateUnavailable,
                    severity: WorkflowDeviceResolutionDiagnosticSeverity::Error,
                    message: "cuda runtime is unavailable".to_string(),
                    device_class: Some(WorkflowInferenceDeviceClass::Cuda),
                    device_id: Some("cuda:0".to_string()),
                    runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                    backend_id: Some("llama_cpp".to_string()),
                }],
            },
        ];
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
            None,
            Some("session-a"),
            Some("interactive"),
            None,
        );

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &[capability]);

        assert_eq!(runtime_request.candidates.len(), 2);
        assert_eq!(
            runtime_request
                .candidates
                .iter()
                .map(|candidate| (
                    candidate.candidate_id.as_str(),
                    candidate.runtime_variant_id.as_deref(),
                    candidate.device_class,
                    candidate.supports_runtime_requirements,
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "llama_cpp|llama.cpp|llama_cpp.cpu",
                    Some("llama_cpp.cpu"),
                    Some(RuntimeTechnicalFitDeviceClass::Cpu),
                    true,
                ),
                (
                    "llama_cpp|llama.cpp|llama_cpp.cuda",
                    Some("llama_cpp.cuda"),
                    Some(RuntimeTechnicalFitDeviceClass::Cuda),
                    false,
                ),
            ]
        );
        assert_eq!(
            runtime_request.candidates[1].device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
        );
    }

    #[test]
    fn runtime_request_projection_rejects_candidate_set_overflow() {
        let capabilities = (0..=MAX_RUNTIME_TECHNICAL_FIT_CANDIDATES)
            .map(|index| {
                let mut capability = runtime_capability();
                capability.runtime_id = format!("runtime-{index}");
                capability.backend_keys = vec![format!("backend-{index}")];
                let backend_facts = capability
                    .backend_capability_facts
                    .as_mut()
                    .expect("test capability should include backend facts");
                backend_facts.runtime_variants = vec![WorkflowRuntimeVariantCapability {
                    runtime_variant_id: format!("runtime-{index}.cpu"),
                    device_class: WorkflowInferenceDeviceClass::Cpu,
                    available: true,
                    diagnostics: Vec::new(),
                }];
                capability
            })
            .collect::<Vec<_>>();
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
                estimation_confidence: "high".to_string(),
                required_models: Vec::new(),
                required_backends: Vec::new(),
                required_extensions: Vec::new(),
            },
            None,
            Some("session-a"),
            Some("interactive"),
            None,
        );

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &capabilities);

        assert_eq!(runtime_request.candidates.len(), 1);
        assert_eq!(
            runtime_request.candidates[0].candidate_id,
            "candidate_set_overflow"
        );
        assert_eq!(
            runtime_request.candidates[0].device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow
        );

        let registry_decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(registry_decision.selected_candidate_id, None);
        assert_eq!(
            registry_decision.device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow
        );

        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);
        assert_eq!(
            workflow_decision.device_diagnostics[0].code,
            WorkflowTechnicalFitDeviceDiagnosticCode::CandidateSetOverflow
        );
    }

    #[test]
    fn workflow_decision_projection_preserves_reason_codes() {
        let decision = RuntimeTechnicalFitDecision {
            selection_mode: RuntimeTechnicalFitSelectionMode::Automatic,
            selected_candidate_id: Some("candidate-a".to_string()),
            selected_runtime_id: Some("llama_cpp".to_string()),
            selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
            selected_backend_key: Some("llama_cpp".to_string()),
            selected_model_id: Some("model-a".to_string()),
            selected_device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
            selected_device_id: Some("cuda:0".to_string()),
            resource_estimate: Some(RuntimeTechnicalFitResourceEstimate {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
            }),
            observed_throughput_hint: Some(RuntimeTechnicalFitObservedThroughputHint {
                tokens_per_second_milli: None,
                images_per_second_milli: Some(125),
                sample_count: Some(3),
            }),
            device_diagnostics: vec![RuntimeTechnicalFitDeviceDiagnostic {
                code: RuntimeTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
                severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning,
                message: "cuda runtime warmup pending".to_string(),
                device_class: Some(RuntimeTechnicalFitDeviceClass::Cuda),
                device_id: Some("cuda:0".to_string()),
                runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                backend_key: Some("llama_cpp".to_string()),
            }],
            reasons: vec![RuntimeTechnicalFitReason::new(
                RuntimeTechnicalFitReasonCode::QueuePressure,
                Some("candidate-a"),
            )],
            selection_policy_trace: Some(RuntimeTechnicalFitSelectionPolicyTrace {
                policy_version: 1,
                policy_phase: Some(RuntimeTechnicalFitPolicyPhase::CandidateRanking),
                decision_code: Some(RuntimeTechnicalFitDecisionCode::SelectedCandidate),
                history_threshold_state: Some(
                    RuntimeTechnicalFitHistoryThresholdState::NotEvaluated,
                ),
                candidate_set_summary: Some(
                    pantograph_runtime_registry::RuntimeTechnicalFitCandidateSetSummary {
                        total_candidate_count: 2,
                        eligible_candidate_count: 1,
                        rejected_candidate_count: 1,
                        eligible_candidate_ids: vec!["candidate-a".to_string()],
                    },
                ),
                ranking_reason: Some("queue_pressure".to_string()),
                exploration_reason: None,
                seed_basis: Some("workflow-a:node-a".to_string()),
            }),
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
                selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                selected_backend_key: Some("llama_cpp".to_string()),
                selected_model_id: Some("model-a".to_string()),
                selected_device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
                selected_device_id: Some("cuda:0".to_string()),
                resource_estimate: Some(WorkflowTechnicalFitResourceEstimate {
                    estimated_peak_vram_mb: Some(4096),
                    estimated_peak_ram_mb: Some(8192),
                    estimated_min_vram_mb: Some(2048),
                    estimated_min_ram_mb: Some(4096),
                }),
                observed_throughput_hint: Some(WorkflowTechnicalFitObservedThroughputHint {
                    tokens_per_second_milli: None,
                    images_per_second_milli: Some(125),
                    sample_count: Some(3),
                }),
                device_diagnostics: vec![WorkflowTechnicalFitDeviceDiagnostic {
                    code: WorkflowTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
                    severity: WorkflowTechnicalFitDeviceDiagnosticSeverity::Warning,
                    message: "cuda runtime warmup pending".to_string(),
                    device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
                    device_id: Some("cuda:0".to_string()),
                    runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                }],
                reasons: vec![WorkflowTechnicalFitReason {
                    code: WorkflowTechnicalFitReasonCode::QueuePressure,
                    candidate_id: Some("candidate-a".to_string()),
                }],
                selection_policy_trace: Some(WorkflowTechnicalFitSelectionPolicyTrace {
                    policy_version: 1,
                    policy_phase: Some(WorkflowTechnicalFitPolicyPhase::CandidateRanking),
                    decision_code: Some(WorkflowTechnicalFitDecisionCode::SelectedCandidate),
                    history_threshold_state: Some(
                        WorkflowTechnicalFitHistoryThresholdState::NotEvaluated,
                    ),
                    candidate_set_summary: Some(
                        pantograph_workflow_service::WorkflowTechnicalFitCandidateSetSummary {
                            total_candidate_count: 2,
                            eligible_candidate_count: 1,
                            rejected_candidate_count: 1,
                            eligible_candidate_ids: vec!["candidate-a".to_string()],
                        },
                    ),
                    ranking_reason: Some("queue_pressure".to_string()),
                    exploration_reason: None,
                    seed_basis: Some("workflow-a:node-a".to_string()),
                }),
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
                runtime_id: None,
                runtime_variant_id: None,
                model_id: None,
                backend_key: Some("llama.cpp".to_string()),
            }),
            None,
            None,
            None,
        );

        let runtime_request =
            build_runtime_technical_fit_request(&workflow_request, None, &[runtime_capability()]);
        assert_eq!(
            runtime_request.candidates[0].source_kind,
            RuntimeTechnicalFitCandidateSourceKind::RuntimeCapabilityFacts
        );

        let registry_decision = select_runtime_technical_fit(&runtime_request);
        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);

        assert_eq!(
            workflow_decision,
            WorkflowTechnicalFitDecision {
                selection_mode: WorkflowTechnicalFitSelectionMode::ExplicitOverride,
                selected_candidate_id: Some("llama_cpp".to_string()),
                selected_runtime_id: Some("llama_cpp".to_string()),
                selected_runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                selected_backend_key: Some("llama_cpp".to_string()),
                selected_model_id: None,
                selected_device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
                selected_device_id: None,
                resource_estimate: Some(WorkflowTechnicalFitResourceEstimate {
                    estimated_peak_vram_mb: Some(4096),
                    estimated_peak_ram_mb: Some(8192),
                    estimated_min_vram_mb: Some(2048),
                    estimated_min_ram_mb: Some(4096),
                }),
                observed_throughput_hint: None,
                device_diagnostics: vec![WorkflowTechnicalFitDeviceDiagnostic {
                    code: WorkflowTechnicalFitDeviceDiagnosticCode::CandidateUnavailable,
                    severity: WorkflowTechnicalFitDeviceDiagnosticSeverity::Warning,
                    message: "cuda runtime warmup pending".to_string(),
                    device_class: Some(WorkflowTechnicalFitDeviceClass::Cuda),
                    device_id: Some("cuda:0".to_string()),
                    runtime_variant_id: Some("llama_cpp.cuda".to_string()),
                    backend_key: Some("llama_cpp".to_string()),
                }],
                reasons: vec![WorkflowTechnicalFitReason {
                    code: WorkflowTechnicalFitReasonCode::ExplicitBackendOverride,
                    candidate_id: Some("llama_cpp".to_string()),
                }],
                selection_policy_trace: None,
                compatibility_report: None,
                compatibility_issue_count: 0,
                compatibility_issues: Vec::new(),
            }
        );
    }

    #[test]
    fn roadmap_backend_overrides_reject_without_fallback_selection() {
        for (backend_key, expected_device_class, expected_variant, message) in [
            (
                "mlx",
                WorkflowTechnicalFitDeviceClass::Metal,
                "mlx.metal",
                "MLX",
            ),
            (
                "vllm",
                WorkflowTechnicalFitDeviceClass::Cpu,
                "vllm.cpu",
                "vLLM",
            ),
        ] {
            let workflow_request = build_workflow_technical_fit_request(
                "workflow-a",
                &WorkflowRuntimeRequirements::default(),
                Some(pantograph_workflow_service::WorkflowTechnicalFitOverride {
                    runtime_id: None,
                    runtime_variant_id: None,
                    model_id: None,
                    backend_key: Some(backend_key.to_string()),
                }),
                None,
                None,
                None,
            );

            let runtime_request = build_runtime_technical_fit_request(
                &workflow_request,
                None,
                &crate::runtime_capabilities::roadmap_runtime_capabilities("llama_cpp"),
            );
            let registry_decision = select_runtime_technical_fit(&runtime_request);
            let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);

            assert_eq!(
                workflow_decision.selection_mode,
                WorkflowTechnicalFitSelectionMode::ExplicitOverride
            );
            assert_eq!(workflow_decision.selected_candidate_id, None);
            assert_eq!(workflow_decision.selected_runtime_id, None);
            assert_eq!(workflow_decision.selected_backend_key, None);
            assert!(workflow_decision.reasons.iter().any(|reason| {
                reason.code == WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
                    && reason.candidate_id.is_none()
            }));
            assert_eq!(workflow_decision.device_diagnostics.len(), 1);
            let diagnostic = &workflow_decision.device_diagnostics[0];
            assert_eq!(
                diagnostic.code,
                WorkflowTechnicalFitDeviceDiagnosticCode::CandidateUnavailable
            );
            assert_eq!(
                diagnostic.severity,
                WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
            );
            assert_eq!(diagnostic.device_class, Some(expected_device_class));
            assert_eq!(
                diagnostic.runtime_variant_id.as_deref(),
                Some(expected_variant)
            );
            assert_eq!(diagnostic.backend_key.as_deref(), Some(backend_key));
            assert!(diagnostic.message.contains(message));
        }
    }

    #[test]
    fn pumas_package_facts_without_runtime_capability_are_not_selectable() {
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
        assert!(!candidates[0].supports_runtime_requirements);
        assert!(candidates[0].device_diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeTechnicalFitDeviceDiagnosticCode::MissingRuntimeVariant
                && diagnostic.severity == RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
        }));
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
            &[runtime_capability()],
            None,
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
        assert_eq!(llama.runtime_id.as_deref(), Some("llama.cpp"));
        assert_eq!(llama.runtime_variant_id.as_deref(), Some("llama_cpp.cuda"));
        assert_eq!(
            llama.device_class,
            Some(RuntimeTechnicalFitDeviceClass::Cuda)
        );
        assert_eq!(
            llama.residency_state,
            Some(RuntimeTechnicalFitResidencyState::Active)
        );
        assert_eq!(
            llama.warmup_state,
            Some(RuntimeTechnicalFitWarmupState::Ready)
        );
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
            &[],
            None,
        );

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].backend_key.as_deref(), Some("pytorch"));
        assert!(!candidates[0].supports_runtime_requirements);
        assert!(candidates[0].device_diagnostics.is_empty());
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
            &[runtime_capability()],
            &[package_facts],
        );

        assert_eq!(runtime_request.candidates.len(), 2);
        assert!(runtime_request.candidates.iter().any(|candidate| {
            candidate.source_kind == RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts
                && candidate.model_id.as_deref() == Some("llm/llama/tiny-gguf")
                && candidate.runtime_variant_id.as_deref() == Some("llama_cpp.cuda")
        }));

        let decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(
            decision.selected_model_id.as_deref(),
            Some("llm/llama/tiny-gguf")
        );
        assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(decision.selected_runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            decision.selected_runtime_variant_id.as_deref(),
            Some("llama_cpp.cuda")
        );
        assert_eq!(
            decision.selected_device_class,
            Some(RuntimeTechnicalFitDeviceClass::Cuda)
        );
    }

    #[test]
    fn missing_required_package_facts_block_capability_only_selection() {
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(4096),
                estimated_peak_ram_mb: Some(8192),
                estimated_min_vram_mb: Some(2048),
                estimated_min_ram_mb: Some(4096),
                estimation_confidence: "high".to_string(),
                required_models: vec!["llm/llama/missing-facts".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            },
            None,
            Some("session-a"),
            Some("interactive"),
            None,
        );

        let runtime_request = build_runtime_technical_fit_request_for_resolved_package_facts(
            &workflow_request,
            None,
            &[runtime_capability()],
            &[],
            &[],
        );

        assert_eq!(runtime_request.candidates.len(), 1);
        assert_eq!(
            runtime_request.candidates[0].candidate_id,
            "missing_model_package_facts|llm/llama/missing-facts"
        );
        assert_eq!(
            runtime_request.candidates[0].device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts
        );

        let registry_decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(registry_decision.selected_candidate_id, None);
        assert_eq!(
            registry_decision.device_diagnostics[0].code,
            RuntimeTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts
        );

        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);
        assert_eq!(
            workflow_decision.device_diagnostics[0].code,
            WorkflowTechnicalFitDeviceDiagnosticCode::MissingModelPackageFacts
        );
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
            &[runtime_capability()],
            &backends,
            &[package_facts],
        );

        assert_eq!(runtime_request.candidates.len(), 3);
        let decision = select_runtime_technical_fit(&runtime_request);
        assert_eq!(
            decision.selected_model_id.as_deref(),
            Some("llm/llama/tiny-gguf")
        );
        assert_eq!(decision.selected_backend_key.as_deref(), Some("llama_cpp"));
        assert_eq!(decision.selected_runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            decision.selected_runtime_variant_id.as_deref(),
            Some("llama_cpp.cuda")
        );
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

    #[test]
    fn candle_image_generation_override_rejects_backend_incompatibility_without_selection() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
            ),
        )
        .expect("decode image generation package facts fixture");
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "fixture".to_string(),
                required_models: vec!["image/stable-diffusion/tiny-sd".to_string()],
                required_backends: vec!["candle".to_string()],
                required_extensions: Vec::new(),
            },
            Some(pantograph_workflow_service::WorkflowTechnicalFitOverride {
                runtime_id: None,
                runtime_variant_id: None,
                model_id: None,
                backend_key: Some("candle".to_string()),
            }),
            None,
            None,
            None,
        );
        let backends = vec![backend_info(
            "candle",
            vec![inference::ModelArtifactKind::DiffusersBundle],
            vec![inference::BackendHintLabel::Candle],
        )];

        let runtime_request = build_runtime_technical_fit_request_with_backend_package_facts(
            &workflow_request,
            None,
            &[],
            &backends,
            &[package_facts],
        );

        let registry_decision = select_runtime_technical_fit(&runtime_request);
        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);

        assert_eq!(
            workflow_decision.selection_mode,
            WorkflowTechnicalFitSelectionMode::ExplicitOverride
        );
        assert_eq!(workflow_decision.selected_candidate_id, None);
        assert_eq!(workflow_decision.selected_backend_key, None);
        assert!(workflow_decision.reasons.iter().any(|reason| {
            reason.code == WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
                && reason.candidate_id.is_none()
        }));
        assert_eq!(workflow_decision.device_diagnostics.len(), 1);
        let diagnostic = &workflow_decision.device_diagnostics[0];
        assert_eq!(
            diagnostic.code,
            WorkflowTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        );
        assert_eq!(
            diagnostic.severity,
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
        );
        assert_eq!(diagnostic.backend_key.as_deref(), Some("candle"));
        assert!(diagnostic.message.contains("ImageGeneration"));
    }

    #[test]
    fn vllm_image_generation_override_rejects_unsupported_package_without_selection() {
        let package_facts: inference::ResolvedModelPackageFacts = serde_json::from_str(
            include_str!(
                "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
            ),
        )
        .expect("decode image generation package facts fixture");
        let workflow_request = build_workflow_technical_fit_request(
            "workflow-a",
            &WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "fixture".to_string(),
                required_models: vec!["image/stable-diffusion/tiny-sd".to_string()],
                required_backends: vec!["vllm".to_string()],
                required_extensions: Vec::new(),
            },
            Some(pantograph_workflow_service::WorkflowTechnicalFitOverride {
                runtime_id: None,
                runtime_variant_id: None,
                model_id: None,
                backend_key: Some("vllm".to_string()),
            }),
            None,
            None,
            None,
        );
        let backends = vec![backend_info(
            "vllm",
            vec![inference::ModelArtifactKind::HfCompatibleDirectory],
            vec![inference::BackendHintLabel::Vllm],
        )];

        let runtime_request = build_runtime_technical_fit_request_with_backend_package_facts(
            &workflow_request,
            None,
            &[],
            &backends,
            &[package_facts],
        );

        let registry_decision = select_runtime_technical_fit(&runtime_request);
        let workflow_decision = project_workflow_technical_fit_decision(&registry_decision);

        assert_eq!(
            workflow_decision.selection_mode,
            WorkflowTechnicalFitSelectionMode::ExplicitOverride
        );
        assert_eq!(workflow_decision.selected_candidate_id, None);
        assert_eq!(workflow_decision.selected_backend_key, None);
        assert!(workflow_decision.reasons.iter().any(|reason| {
            reason.code == WorkflowTechnicalFitReasonCode::ExplicitBackendOverride
                && reason.candidate_id.is_none()
        }));
        assert_eq!(workflow_decision.device_diagnostics.len(), 1);
        let diagnostic = &workflow_decision.device_diagnostics[0];
        assert_eq!(
            diagnostic.code,
            WorkflowTechnicalFitDeviceDiagnosticCode::BackendIncompatible
        );
        assert_eq!(
            diagnostic.severity,
            WorkflowTechnicalFitDeviceDiagnosticSeverity::Error
        );
        assert_eq!(diagnostic.backend_key.as_deref(), Some("vllm"));
        assert!(diagnostic.message.contains("ImageGeneration"));
    }
}
