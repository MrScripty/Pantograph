use pantograph_runtime_registry::{
    RuntimeTechnicalFitCandidate, RuntimeTechnicalFitCandidateSourceKind,
    RuntimeTechnicalFitDeviceDiagnostic, RuntimeTechnicalFitDeviceDiagnosticCode,
    RuntimeTechnicalFitDeviceDiagnosticSeverity, RuntimeTechnicalFitResourceEstimate,
};
use pantograph_workflow_service::WorkflowRuntimeCapability;

use super::{
    pumas_candidate_id, runtime_capability_for_backend, runtime_capability_is_ready,
    runtime_capability_residency_state, runtime_capability_variant_fact_entries,
    runtime_capability_warmup_state, runtime_compatibility_issues, runtime_compatibility_report,
    MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES,
};

#[derive(Debug, Clone)]
pub(crate) struct ExecutionEvidenceTechnicalFitReport<'a> {
    pub(crate) task_id: inference::InferenceTaskId,
    pub(crate) model_id: &'a str,
    pub(crate) report: &'a inference::ExecutionEvidenceReport,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutionEvidenceTechnicalFitAdapterInput<'a> {
    pub(crate) reports: &'a [ExecutionEvidenceTechnicalFitReport<'a>],
    pub(crate) runtime_capabilities: &'a [WorkflowRuntimeCapability],
    pub(crate) resource_estimate: Option<RuntimeTechnicalFitResourceEstimate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExecutionEvidenceTechnicalFitAdapterOutput {
    pub(crate) candidates: Vec<RuntimeTechnicalFitCandidate>,
    pub(crate) diagnostics: Vec<RuntimeTechnicalFitDeviceDiagnostic>,
}

pub(crate) fn adapt_execution_evidence_to_technical_fit(
    input: ExecutionEvidenceTechnicalFitAdapterInput<'_>,
) -> ExecutionEvidenceTechnicalFitAdapterOutput {
    let mut output = ExecutionEvidenceTechnicalFitAdapterOutput::default();

    for report in input.reports {
        let candidate_start = output.candidates.len();
        let requested_runtime_key = requested_runtime_key(report.report);

        output
            .diagnostics
            .extend(report.report.diagnostics.iter().map(|diagnostic| {
                map_execution_evidence_diagnostic(
                    report.task_id.clone(),
                    report.model_id,
                    diagnostic,
                    input.runtime_capabilities,
                )
            }));

        for candidate in &report.report.candidates {
            let Some(capability) =
                runtime_capability_for_backend(input.runtime_capabilities, &candidate.backend_key)
            else {
                output
                    .diagnostics
                    .push(missing_runtime_capability_diagnostic(
                        report.task_id.clone(),
                        &candidate.backend_key,
                        &candidate.model_id,
                        requested_runtime_key.as_deref(),
                    ));
                continue;
            };

            output.candidates.extend(
                runtime_capability_variant_fact_entries(capability)
                    .into_iter()
                    .map(|variant_facts| {
                        let compatibility_report = Some(runtime_compatibility_report(
                            &candidate.compatibility_report,
                        ));
                        let compatibility_issue_count = candidate
                            .compatibility_report
                            .issues
                            .len()
                            .min(u32::MAX as usize)
                            as u32;
                        let compatibility_issues = runtime_compatibility_issues(
                            &candidate.compatibility_report,
                            MAX_RUNTIME_TECHNICAL_FIT_COMPATIBILITY_ISSUES,
                        );
                        let runtime_id = Some(capability.runtime_id.clone());
                        let runtime_ready = runtime_capability_is_ready(capability);
                        let variant_ready = variant_facts.available
                            && variant_facts.runtime_variant_id.is_some()
                            && variant_facts.device_class.is_some();
                        RuntimeTechnicalFitCandidate {
                            candidate_id: pumas_candidate_id(
                                &candidate.backend_key,
                                &candidate.model_id,
                                runtime_id.as_deref(),
                                variant_facts.runtime_variant_id.as_deref(),
                            ),
                            runtime_id,
                            runtime_variant_id: variant_facts.runtime_variant_id,
                            backend_key: Some(candidate.backend_key.clone()),
                            model_id: Some(candidate.model_id.clone()),
                            device_class: variant_facts.device_class,
                            selected_device_id: None,
                            resource_estimate: input.resource_estimate.clone(),
                            observed_throughput_hint: None,
                            device_diagnostics: variant_facts.device_diagnostics,
                            dependency_readiness: Vec::new(),
                            source_kind: RuntimeTechnicalFitCandidateSourceKind::PumasPackageFacts,
                            context_window_tokens: None,
                            residency_state: Some(runtime_capability_residency_state(capability)),
                            warmup_state: runtime_capability_warmup_state(capability),
                            supports_runtime_requirements: candidate
                                .compatibility_report
                                .compatible
                                && runtime_ready
                                && variant_ready,
                            compatibility_report,
                            compatibility_issue_count,
                            compatibility_issues,
                        }
                    }),
            );
        }

        if output.candidates.len() == candidate_start {
            output.diagnostics.push(no_accepted_candidate_diagnostic(
                report.task_id.clone(),
                report.model_id,
                requested_runtime_key.as_deref(),
            ));
        }
    }

    output
}

fn map_execution_evidence_diagnostic(
    task_id: inference::InferenceTaskId,
    model_id: &str,
    diagnostic: &inference::ExecutionEvidenceDiagnostic,
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> RuntimeTechnicalFitDeviceDiagnostic {
    let runtime_id = diagnostic.backend_key.as_ref().and_then(|backend_key| {
        runtime_capability_for_backend(runtime_capabilities, backend_key)
            .map(|capability| capability.runtime_id.clone())
    });

    RuntimeTechnicalFitDeviceDiagnostic {
        code: map_execution_evidence_diagnostic_code(diagnostic.code),
        severity: map_execution_evidence_diagnostic_severity(diagnostic.severity),
        message: diagnostic.message.clone(),
        task_id: Some(task_id.canonical_label().to_string()),
        runtime_id,
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: diagnostic.backend_key.clone(),
        model_id: Some(model_id.to_string()),
        evidence_key: Some(execution_evidence_key(diagnostic.code).to_string()),
        requested_runtime_key: diagnostic.requested_runtime_key.clone(),
    }
}

fn map_execution_evidence_diagnostic_code(
    code: inference::ExecutionEvidenceDiagnosticCode,
) -> RuntimeTechnicalFitDeviceDiagnosticCode {
    match code {
        inference::ExecutionEvidenceDiagnosticCode::UnsupportedTask => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceUnsupportedTask
        }
        inference::ExecutionEvidenceDiagnosticCode::BackendUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendUnavailable
        }
        inference::ExecutionEvidenceDiagnosticCode::MissingRuntimeCapability => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability
        }
        inference::ExecutionEvidenceDiagnosticCode::RequiredPackageEvidenceUnavailable => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceRequiredPackageUnavailable
        }
        inference::ExecutionEvidenceDiagnosticCode::BackendCompatibilityRejected => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendCompatibilityRejected
        }
        inference::ExecutionEvidenceDiagnosticCode::GraphRuntimeRequirementUnsatisfied => {
            RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceGraphRuntimeUnsatisfied
        }
    }
}

fn map_execution_evidence_diagnostic_severity(
    severity: inference::ExecutionEvidenceDiagnosticSeverity,
) -> RuntimeTechnicalFitDeviceDiagnosticSeverity {
    match severity {
        inference::ExecutionEvidenceDiagnosticSeverity::Info => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
        }
        inference::ExecutionEvidenceDiagnosticSeverity::Warning => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Warning
        }
        inference::ExecutionEvidenceDiagnosticSeverity::Error => {
            RuntimeTechnicalFitDeviceDiagnosticSeverity::Error
        }
    }
}

fn execution_evidence_key(code: inference::ExecutionEvidenceDiagnosticCode) -> &'static str {
    match code {
        inference::ExecutionEvidenceDiagnosticCode::UnsupportedTask => "task_registry",
        inference::ExecutionEvidenceDiagnosticCode::BackendUnavailable => "backend_capability",
        inference::ExecutionEvidenceDiagnosticCode::MissingRuntimeCapability => {
            "runtime_capability"
        }
        inference::ExecutionEvidenceDiagnosticCode::RequiredPackageEvidenceUnavailable => {
            "package_facts"
        }
        inference::ExecutionEvidenceDiagnosticCode::BackendCompatibilityRejected => {
            "compatibility_report"
        }
        inference::ExecutionEvidenceDiagnosticCode::GraphRuntimeRequirementUnsatisfied => {
            "graph_runtime_requirement"
        }
    }
}

fn requested_runtime_key(report: &inference::ExecutionEvidenceReport) -> Option<String> {
    report
        .diagnostics
        .iter()
        .find_map(|diagnostic| diagnostic.requested_runtime_key.clone())
        .or_else(|| {
            report.records.iter().find_map(|record| {
                (record.role == inference::ExecutionEvidenceRole::GraphRuntimeConstraint
                    && record.key == "runtime")
                    .then(|| record.value.clone())
            })
        })
}

fn missing_runtime_capability_diagnostic(
    task_id: inference::InferenceTaskId,
    backend_key: &str,
    model_id: &str,
    requested_runtime_key: Option<&str>,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: format!(
            "execution evidence selected backend '{}' for model '{}' but no matching runtime capability facts are available",
            backend_key, model_id
        ),
        task_id: Some(task_id.canonical_label().to_string()),
        runtime_id: None,
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: Some(backend_key.to_string()),
        model_id: Some(model_id.to_string()),
        evidence_key: Some("runtime_capability".to_string()),
        requested_runtime_key: requested_runtime_key.map(str::to_string),
    }
}

fn no_accepted_candidate_diagnostic(
    task_id: inference::InferenceTaskId,
    model_id: &str,
    requested_runtime_key: Option<&str>,
) -> RuntimeTechnicalFitDeviceDiagnostic {
    RuntimeTechnicalFitDeviceDiagnostic {
        code: RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate,
        severity: RuntimeTechnicalFitDeviceDiagnosticSeverity::Error,
        message: format!(
            "execution evidence produced no accepted technical-fit candidate for model '{}'",
            model_id
        ),
        task_id: Some(task_id.canonical_label().to_string()),
        runtime_id: None,
        device_class: None,
        device_id: None,
        runtime_variant_id: None,
        backend_key: None,
        model_id: Some(model_id.to_string()),
        evidence_key: Some("execution_evidence".to_string()),
        requested_runtime_key: requested_runtime_key.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_runtime_registry::{
        RuntimeTechnicalFitDeviceClass, RuntimeTechnicalFitResidencyState,
        RuntimeTechnicalFitWarmupState,
    };
    use pantograph_workflow_service::{
        WorkflowBackendCapabilityFacts, WorkflowInferenceDeviceClass, WorkflowRuntimeInstallState,
        WorkflowRuntimeReadinessState, WorkflowRuntimeSourceKind, WorkflowRuntimeVariantCapability,
    };

    fn fixture(raw: &str) -> inference::ResolvedModelPackageFacts {
        serde_json::from_str(raw).expect("fixture should decode")
    }

    fn diffusers_package() -> inference::ResolvedModelPackageFacts {
        fixture(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
        ))
    }

    fn available_variant(runtime_variant_id: &str) -> inference::RuntimeVariantCapability {
        inference::RuntimeVariantCapability {
            runtime_variant_id: inference::RuntimeVariantId::parse(runtime_variant_id)
                .expect("test runtime variant should parse"),
            device_class: inference::InferenceDeviceClass::Cuda,
            available: true,
            diagnostics: Vec::new(),
        }
    }

    fn pytorch_diffusers_backend() -> inference::BackendInfo {
        inference::BackendInfo {
            name: "pytorch".to_string(),
            backend_key: "pytorch".to_string(),
            description: "pytorch test backend".to_string(),
            capabilities: inference::BackendCapabilities {
                image_generation: true,
                facts: inference::BackendCapabilityFacts {
                    tasks: vec![inference::BackendTaskCapability {
                        task_id: inference::InferenceTaskId::ImageGeneration,
                        support_tier: inference::SupportTier::Experimental,
                        modality_signature: inference::TaskModalitySignature::new(
                            vec![inference::InferenceModality::Text],
                            vec![inference::InferenceModality::Image],
                        ),
                    }],
                    preprocessing: inference::BackendComponentCapability::RequiresPackageComponent,
                    postprocessing: inference::BackendComponentCapability::BackendManaged,
                    model_sources: inference::BackendModelSourceCapabilityFacts {
                        artifact_kinds: vec![inference::ModelArtifactKind::DiffusersBundle],
                        backend_hints: vec![inference::BackendHintLabel::Diffusers],
                        custom_code: inference::BackendFeatureSupport::Unsupported,
                    },
                    features: inference::BackendFeatureCapabilityFacts::default(),
                    runtime_variants: vec![available_variant("pytorch.cuda")],
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

    fn pytorch_runtime_capability() -> WorkflowRuntimeCapability {
        WorkflowRuntimeCapability {
            runtime_id: "pytorch".to_string(),
            display_name: "PyTorch".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Managed,
            selected: false,
            readiness_state: Some(WorkflowRuntimeReadinessState::Ready),
            selected_version: None,
            supports_external_connection: false,
            backend_capability_facts: Some(WorkflowBackendCapabilityFacts {
                tasks: Vec::new(),
                runtime_variants: vec![WorkflowRuntimeVariantCapability {
                    runtime_variant_id: "pytorch.cuda".to_string(),
                    device_class: WorkflowInferenceDeviceClass::Cuda,
                    available: true,
                    diagnostics: Vec::new(),
                }],
                preprocessing: Default::default(),
                postprocessing: Default::default(),
                model_sources: Default::default(),
                features: Default::default(),
                request_lifecycle: Default::default(),
            }),
            backend_keys: vec!["pytorch".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }
    }

    fn report_for_package(
        package: &inference::ResolvedModelPackageFacts,
        backends: &[inference::BackendInfo],
        graph_runtime_requirement: Option<&inference::GraphRuntimeRequirement>,
    ) -> inference::ExecutionEvidenceReport {
        inference::normalize_execution_evidence(inference::ExecutionEvidenceRequest {
            task_id: inference::InferenceTaskId::ImageGeneration,
            package_facts: package,
            backends,
            graph_runtime_requirement,
        })
    }

    #[test]
    fn adapter_projects_pytorch_diffusers_evidence_candidate() {
        let package = diffusers_package();
        let backends = vec![pytorch_diffusers_backend()];
        let report = report_for_package(&package, &backends, None);
        let runtime_capabilities = vec![pytorch_runtime_capability()];
        let reports = vec![ExecutionEvidenceTechnicalFitReport {
            task_id: inference::InferenceTaskId::ImageGeneration,
            model_id: &package.model_ref.model_id,
            report: &report,
        }];

        let output =
            adapt_execution_evidence_to_technical_fit(ExecutionEvidenceTechnicalFitAdapterInput {
                reports: &reports,
                runtime_capabilities: &runtime_capabilities,
                resource_estimate: Some(RuntimeTechnicalFitResourceEstimate {
                    estimated_peak_vram_mb: Some(4096),
                    estimated_peak_ram_mb: Some(8192),
                    estimated_min_vram_mb: Some(2048),
                    estimated_min_ram_mb: Some(4096),
                }),
            });

        assert!(output.diagnostics.is_empty());
        assert_eq!(output.candidates.len(), 1);
        let candidate = &output.candidates[0];
        assert_eq!(candidate.backend_key.as_deref(), Some("pytorch"));
        assert_eq!(candidate.runtime_id.as_deref(), Some("pytorch"));
        assert_eq!(
            candidate.runtime_variant_id.as_deref(),
            Some("pytorch.cuda")
        );
        assert_eq!(
            candidate.model_id.as_deref(),
            Some("image/stable-diffusion/tiny-sd")
        );
        assert_eq!(
            candidate.device_class,
            Some(RuntimeTechnicalFitDeviceClass::Cuda)
        );
        assert_eq!(
            candidate.residency_state,
            Some(RuntimeTechnicalFitResidencyState::Loaded)
        );
        assert_eq!(
            candidate.warmup_state,
            Some(RuntimeTechnicalFitWarmupState::Warm)
        );
        assert!(candidate.supports_runtime_requirements);
        assert_eq!(
            candidate
                .compatibility_report
                .as_ref()
                .map(|report| report.status.as_str()),
            Some("accepted")
        );
        assert_ne!(candidate.backend_key.as_deref(), Some("diffusers"));
    }

    #[test]
    fn adapter_maps_explicit_diffusers_runtime_without_aliasing_to_pytorch() {
        let package = diffusers_package();
        let requirement =
            inference::GraphRuntimeRequirement::parse("diffusers").expect("requirement parses");
        let backends = vec![pytorch_diffusers_backend()];
        let report = report_for_package(&package, &backends, Some(&requirement));
        let runtime_capabilities = vec![pytorch_runtime_capability()];
        let reports = vec![ExecutionEvidenceTechnicalFitReport {
            task_id: inference::InferenceTaskId::ImageGeneration,
            model_id: &package.model_ref.model_id,
            report: &report,
        }];

        let output =
            adapt_execution_evidence_to_technical_fit(ExecutionEvidenceTechnicalFitAdapterInput {
                reports: &reports,
                runtime_capabilities: &runtime_capabilities,
                resource_estimate: None,
            });

        assert!(output.candidates.is_empty());
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceGraphRuntimeUnsatisfied
                && diagnostic.requested_runtime_key.as_deref() == Some("diffusers")
        }));
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate
                && diagnostic.requested_runtime_key.as_deref() == Some("diffusers")
        }));
        assert!(!output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.backend_key.as_deref() == Some("diffusers")));
    }

    #[test]
    fn adapter_fails_validated_backend_without_runtime_capability() {
        let package = diffusers_package();
        let backends = vec![pytorch_diffusers_backend()];
        let report = report_for_package(&package, &backends, None);
        let reports = vec![ExecutionEvidenceTechnicalFitReport {
            task_id: inference::InferenceTaskId::ImageGeneration,
            model_id: &package.model_ref.model_id,
            report: &report,
        }];

        let output =
            adapt_execution_evidence_to_technical_fit(ExecutionEvidenceTechnicalFitAdapterInput {
                reports: &reports,
                runtime_capabilities: &[],
                resource_estimate: None,
            });

        assert!(output.candidates.is_empty());
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code
                == RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability
                && diagnostic.backend_key.as_deref() == Some("pytorch")
                && diagnostic.model_id.as_deref() == Some("image/stable-diffusion/tiny-sd")
        }));
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate
                && diagnostic.evidence_key.as_deref() == Some("execution_evidence")
        }));
    }

    #[test]
    fn adapter_maps_each_execution_evidence_diagnostic_code() {
        let report = inference::ExecutionEvidenceReport {
            candidates: Vec::new(),
            records: Vec::new(),
            diagnostics: vec![
                evidence_diagnostic(inference::ExecutionEvidenceDiagnosticCode::UnsupportedTask),
                evidence_diagnostic(inference::ExecutionEvidenceDiagnosticCode::BackendUnavailable),
                evidence_diagnostic(
                    inference::ExecutionEvidenceDiagnosticCode::MissingRuntimeCapability,
                ),
                evidence_diagnostic(
                    inference::ExecutionEvidenceDiagnosticCode::RequiredPackageEvidenceUnavailable,
                ),
                evidence_diagnostic(
                    inference::ExecutionEvidenceDiagnosticCode::BackendCompatibilityRejected,
                ),
                evidence_diagnostic(
                    inference::ExecutionEvidenceDiagnosticCode::GraphRuntimeRequirementUnsatisfied,
                ),
            ],
        };
        let runtime_capabilities = vec![pytorch_runtime_capability()];
        let reports = vec![ExecutionEvidenceTechnicalFitReport {
            task_id: inference::InferenceTaskId::ImageGeneration,
            model_id: "image/stable-diffusion/tiny-sd",
            report: &report,
        }];

        let output =
            adapt_execution_evidence_to_technical_fit(ExecutionEvidenceTechnicalFitAdapterInput {
                reports: &reports,
                runtime_capabilities: &runtime_capabilities,
                resource_estimate: None,
            });

        let codes = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        assert!(codes.contains(&RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceUnsupportedTask));
        assert!(
            codes.contains(&RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendUnavailable)
        );
        assert!(codes
            .contains(&RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceMissingRuntimeCapability));
        assert!(codes.contains(
            &RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceRequiredPackageUnavailable
        ));
        assert!(codes.contains(
            &RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceBackendCompatibilityRejected
        ));
        assert!(codes
            .contains(&RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceGraphRuntimeUnsatisfied));
        assert!(
            codes.contains(&RuntimeTechnicalFitDeviceDiagnosticCode::EvidenceNoAcceptedCandidate)
        );
    }

    fn evidence_diagnostic(
        code: inference::ExecutionEvidenceDiagnosticCode,
    ) -> inference::ExecutionEvidenceDiagnostic {
        inference::ExecutionEvidenceDiagnostic {
            code,
            severity: inference::ExecutionEvidenceDiagnosticSeverity::Error,
            message: "evidence diagnostic".to_string(),
            backend_key: Some("pytorch".to_string()),
            requested_runtime_key: Some("pytorch".to_string()),
        }
    }
}
