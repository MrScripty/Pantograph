use std::collections::BTreeSet;

use pantograph_dependency_planning::{
    DependencyReadinessProofEnvelope, PumasModelRef, RuntimeIntentId,
};
use pantograph_scheduler::{
    SchedulerDispatchCandidateId, SchedulerDispatchSelectionDiagnostic,
    SchedulerDispatchSelectionDiagnosticCode, SchedulerDispatchSelectionDiagnosticSeverity,
    SchedulerTaskStateRecord,
};
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchCandidateProviderError,
    WorkflowRuntimeDispatchCandidateSet,
};
use pantograph_workflow_service::WorkflowSchedulerTask;

use crate::pumas_dispatch_package_facts::{
    PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsDiagnostic,
    PumasDispatchPackageFactsDiagnosticCode, PumasDispatchPackageFactsProjection,
};
use crate::runtime_dispatch_capability_facts::{
    RuntimeDispatchCapabilityFactsDiagnostic, RuntimeDispatchCapabilityFactsOutcome,
    RuntimeDispatchCapabilityFactsProjection, RuntimeDispatchRuntimeCapabilityFacts,
};

const MISSING_PUMAS_PACKAGE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_pumas_package_facts";
const MISSING_RUNTIME_CAPABILITY_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_capability_facts";
const MISSING_RUNTIME_RESOURCE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_resource_facts";
const PATH_CARRYING_MODEL_REF_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.path_carrying_model_ref";
const INCOMPATIBLE_RUNTIME_BACKEND_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.incompatible_runtime_backend";

#[derive(Debug, Default, Clone)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateSourceSnapshot {
    pub(crate) pumas_package_facts: Option<PumasDispatchPackageFactsBridgeOutcome>,
    pub(crate) runtime_capability_facts: Option<RuntimeDispatchCapabilityFactsOutcome>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateProvider {
    source_snapshot: EmbeddedRuntimeDispatchCandidateSourceSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EmbeddedRuntimeDispatchCandidateDraft {
    candidate_id: SchedulerDispatchCandidateId,
    selected_runtime_id: RuntimeIntentId,
    selected_backend_key: String,
    selected_model_ref: PumasModelRef,
}

impl EmbeddedRuntimeDispatchCandidateProvider {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub(crate) fn with_source_snapshot(
        source_snapshot: EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    ) -> Self {
        Self { source_snapshot }
    }
}

impl WorkflowRuntimeDispatchCandidateProvider for EmbeddedRuntimeDispatchCandidateProvider {
    fn runtime_dispatch_candidates(
        &self,
        _task: &WorkflowSchedulerTask,
        _ready_record: &SchedulerTaskStateRecord,
        readiness_proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        Ok(WorkflowRuntimeDispatchCandidateSet {
            candidates: Vec::new(),
            diagnostics: fail_closed_diagnostics(
                &self.source_snapshot,
                &readiness_proof.preflight_result.identity_key.model_ref,
            ),
        })
    }
}

fn fail_closed_diagnostics(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
    model_ref: &PumasModelRef,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    let mut diagnostics = Vec::new();
    if model_ref.selected_artifact_path.is_some() {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider rejected a path-carrying Pumas model ref",
            PATH_CARRYING_MODEL_REF_HINT,
        ));
        return diagnostics;
    }

    diagnostics.extend(pumas_package_diagnostics(
        source_snapshot.pumas_package_facts.as_ref(),
    ));
    diagnostics.extend(runtime_capability_diagnostics(
        source_snapshot.runtime_capability_facts.as_ref(),
    ));
    let (_candidate_drafts, draft_diagnostics) = candidate_drafts(source_snapshot);
    diagnostics.extend(draft_diagnostics);
    diagnostics.push(provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        "runtime dispatch candidate provider has no staged runtime resource facts",
        MISSING_RUNTIME_RESOURCE_FACTS_HINT,
    ));
    diagnostics
}

fn pumas_package_diagnostics(
    outcome: Option<&PumasDispatchPackageFactsBridgeOutcome>,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    match outcome {
        Some(PumasDispatchPackageFactsBridgeOutcome::Projected { diagnostics, .. }) => diagnostics
            .iter()
            .map(pumas_package_source_diagnostic)
            .collect(),
        Some(PumasDispatchPackageFactsBridgeOutcome::Unavailable { diagnostics }) => diagnostics
            .iter()
            .map(pumas_package_source_diagnostic)
            .collect(),
        None => vec![provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider has no staged Pumas package facts",
            MISSING_PUMAS_PACKAGE_FACTS_HINT,
        )],
    }
}

fn pumas_package_source_diagnostic(
    diagnostic: &PumasDispatchPackageFactsDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    provider_diagnostic(
        pumas_package_diagnostic_code(diagnostic.code),
        &diagnostic.message,
        pumas_package_diagnostic_hint(diagnostic.code),
    )
}

fn pumas_package_diagnostic_code(
    code: PumasDispatchPackageFactsDiagnosticCode,
) -> SchedulerDispatchSelectionDiagnosticCode {
    match code {
        PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef
        | PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef => {
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        }
        PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess
        | PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole
        | PumasDispatchPackageFactsDiagnosticCode::PackageFactsLookupFailed
        | PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed
        | PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract
        | PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch
        | PumasDispatchPackageFactsDiagnosticCode::PathFactsStripped => {
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates
        }
    }
}

fn pumas_package_diagnostic_hint(code: PumasDispatchPackageFactsDiagnosticCode) -> &'static str {
    match code {
        PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef => {
            "embedded_runtime_dispatch_candidate_provider.pumas.invalid_model_ref"
        }
        PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef => {
            "embedded_runtime_dispatch_candidate_provider.pumas.path_carrying_model_ref"
        }
        PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess => {
            "embedded_runtime_dispatch_candidate_provider.pumas.missing_selector_access"
        }
        PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole => {
            "embedded_runtime_dispatch_candidate_provider.pumas.unsupported_selector_access_role"
        }
        PumasDispatchPackageFactsDiagnosticCode::PackageFactsLookupFailed => {
            "embedded_runtime_dispatch_candidate_provider.pumas.package_facts_lookup_failed"
        }
        PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed => {
            "embedded_runtime_dispatch_candidate_provider.pumas.package_facts_decode_failed"
        }
        PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract => {
            "embedded_runtime_dispatch_candidate_provider.pumas.stale_package_facts_contract"
        }
        PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch => {
            "embedded_runtime_dispatch_candidate_provider.pumas.selected_artifact_mismatch"
        }
        PumasDispatchPackageFactsDiagnosticCode::PathFactsStripped => {
            "embedded_runtime_dispatch_candidate_provider.pumas.path_facts_stripped"
        }
    }
}

fn runtime_capability_diagnostics(
    outcome: Option<&RuntimeDispatchCapabilityFactsOutcome>,
) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    match outcome {
        Some(RuntimeDispatchCapabilityFactsOutcome::Projected { diagnostics, .. }) => diagnostics
            .iter()
            .map(runtime_capability_source_diagnostic)
            .collect(),
        Some(RuntimeDispatchCapabilityFactsOutcome::Unavailable { diagnostics }) => diagnostics
            .iter()
            .map(runtime_capability_source_diagnostic)
            .collect(),
        None => vec![provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
            "runtime dispatch candidate provider has no staged runtime capability facts",
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT,
        )],
    }
}

fn runtime_capability_source_diagnostic(
    diagnostic: &RuntimeDispatchCapabilityFactsDiagnostic,
) -> SchedulerDispatchSelectionDiagnostic {
    provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        &diagnostic.message,
        "embedded_runtime_dispatch_candidate_provider.runtime_capability.source_diagnostic",
    )
}

fn candidate_drafts(
    source_snapshot: &EmbeddedRuntimeDispatchCandidateSourceSnapshot,
) -> (
    Vec<EmbeddedRuntimeDispatchCandidateDraft>,
    Vec<SchedulerDispatchSelectionDiagnostic>,
) {
    let Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
        facts: package_facts,
        ..
    }) = source_snapshot.pumas_package_facts.as_ref()
    else {
        return (Vec::new(), Vec::new());
    };
    let Some(RuntimeDispatchCapabilityFactsOutcome::Projected {
        facts: capability_facts,
        ..
    }) = source_snapshot.runtime_capability_facts.as_ref()
    else {
        return (Vec::new(), Vec::new());
    };

    candidate_drafts_from_projected_facts(package_facts, capability_facts)
}

fn candidate_drafts_from_projected_facts(
    package_facts: &PumasDispatchPackageFactsProjection,
    capability_facts: &RuntimeDispatchCapabilityFactsProjection,
) -> (
    Vec<EmbeddedRuntimeDispatchCandidateDraft>,
    Vec<SchedulerDispatchSelectionDiagnostic>,
) {
    let backend_hint_keys = backend_hint_keys(&package_facts.backend_hints);
    if backend_hint_keys.is_empty() {
        return (
            Vec::new(),
            vec![provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
                "Pumas package facts contain no accepted backend hints for runtime dispatch",
                INCOMPATIBLE_RUNTIME_BACKEND_HINT,
            )],
        );
    }

    let drafts = capability_facts
        .runtimes
        .iter()
        .flat_map(|runtime| runtime_candidate_drafts(package_facts, runtime, &backend_hint_keys))
        .collect::<Vec<_>>();

    if drafts.is_empty() {
        return (
            drafts,
            vec![provider_diagnostic(
                SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement,
                "no runtime registry capability facts match the Pumas package backend hints",
                INCOMPATIBLE_RUNTIME_BACKEND_HINT,
            )],
        );
    }

    (drafts, Vec::new())
}

fn runtime_candidate_drafts(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
    backend_hint_keys: &BTreeSet<String>,
) -> Vec<EmbeddedRuntimeDispatchCandidateDraft> {
    runtime
        .backend_keys
        .iter()
        .filter_map(|backend_key| {
            let normalized_backend_key = normalize_backend_key(backend_key);
            backend_hint_keys
                .contains(&normalized_backend_key)
                .then(|| runtime_candidate_draft(package_facts, runtime, normalized_backend_key))
        })
        .collect()
}

fn runtime_candidate_draft(
    package_facts: &PumasDispatchPackageFactsProjection,
    runtime: &RuntimeDispatchRuntimeCapabilityFacts,
    selected_backend_key: String,
) -> EmbeddedRuntimeDispatchCandidateDraft {
    EmbeddedRuntimeDispatchCandidateDraft {
        candidate_id: SchedulerDispatchCandidateId::parse(format!(
            "runtime.{}",
            runtime.runtime_id
        ))
        .expect("runtime registry ids should be scheduler-safe"),
        selected_runtime_id: RuntimeIntentId::parse(&runtime.runtime_id)
            .expect("runtime registry ids should be runtime-intent safe"),
        selected_backend_key,
        selected_model_ref: package_facts.model_ref.clone(),
    }
}

fn backend_hint_keys(backend_hints: &inference::BackendHintFacts) -> BTreeSet<String> {
    backend_hints
        .accepted
        .iter()
        .map(|hint| normalize_backend_key(backend_hint_label_key(*hint)))
        .collect()
}

fn backend_hint_label_key(label: inference::BackendHintLabel) -> &'static str {
    match label {
        inference::BackendHintLabel::Transformers => "transformers",
        inference::BackendHintLabel::LlamaCpp => "llama.cpp",
        inference::BackendHintLabel::Vllm => "vllm",
        inference::BackendHintLabel::Mlx => "mlx",
        inference::BackendHintLabel::Candle => "candle",
        inference::BackendHintLabel::Diffusers => "diffusers",
        inference::BackendHintLabel::OnnxRuntime => "onnxruntime",
    }
}

fn normalize_backend_key(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn provider_diagnostic(
    code: SchedulerDispatchSelectionDiagnosticCode,
    message: &str,
    hint: &str,
) -> SchedulerDispatchSelectionDiagnostic {
    SchedulerDispatchSelectionDiagnostic {
        severity: SchedulerDispatchSelectionDiagnosticSeverity::Error,
        code,
        message: message.to_string(),
        candidate_id: None,
        hint: Some(hint.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_dispatch_capability_facts::{
        RuntimeDispatchCapabilityFactsProjection, RuntimeDispatchRuntimeCapabilityFacts,
    };

    #[test]
    fn fail_closed_provider_reports_missing_source_facts() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot::default(),
            &path_free_model_ref(),
        );

        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics
            .iter()
            .all(|diagnostic| diagnostic.candidate_id.is_none()));
        assert!(has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(&diagnostics, MISSING_RUNTIME_RESOURCE_FACTS_HINT));
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.code == SchedulerDispatchSelectionDiagnosticCode::NoCandidates
                && diagnostic.severity == SchedulerDispatchSelectionDiagnosticSeverity::Error
        }));
    }

    #[test]
    fn fail_closed_provider_rejects_path_carrying_model_refs() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot::default(),
            &path_carrying_model_ref(),
        );

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(
            diagnostic.code,
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(PATH_CARRYING_MODEL_REF_HINT)
        );
    }

    #[test]
    fn fail_closed_provider_projects_staged_source_diagnostics() {
        let diagnostics = fail_closed_diagnostics(
            &EmbeddedRuntimeDispatchCandidateSourceSnapshot {
                pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Unavailable {
                    diagnostics: vec![PumasDispatchPackageFactsDiagnostic {
                        code: PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess,
                        message: "Pumas owner access is unavailable".to_string(),
                    }],
                }),
                runtime_capability_facts: Some(RuntimeDispatchCapabilityFactsOutcome::Unavailable {
                    diagnostics: vec![RuntimeDispatchCapabilityFactsDiagnostic {
                        code: crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsDiagnosticCode::NoRegisteredRuntimes,
                        runtime_id: None,
                        message: "runtime registry has no runtimes".to_string(),
                    }],
                }),
            },
            &path_free_model_ref(),
        );

        assert_eq!(diagnostics.len(), 3);
        assert!(!has_hint(&diagnostics, MISSING_PUMAS_PACKAGE_FACTS_HINT));
        assert!(!has_hint(
            &diagnostics,
            MISSING_RUNTIME_CAPABILITY_FACTS_HINT
        ));
        assert!(has_hint(&diagnostics, MISSING_RUNTIME_RESOURCE_FACTS_HINT));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "Pumas owner access is unavailable"));
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message == "runtime registry has no runtimes"));
    }

    #[test]
    fn candidate_drafts_match_pumas_backend_hints_to_runtime_capabilities() {
        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
            &runtime_capability_facts(vec![runtime_capability("pytorch", vec!["diffusers"])]),
        );

        assert!(diagnostics.is_empty());
        assert_eq!(drafts.len(), 1);
        let draft = &drafts[0];
        assert_eq!(draft.candidate_id.as_str(), "runtime.pytorch");
        assert_eq!(draft.selected_runtime_id.as_str(), "pytorch");
        assert_eq!(draft.selected_backend_key, "diffusers");
        assert_eq!(draft.selected_model_ref.model_id, "pumas.model.sdxl");
        assert_eq!(draft.selected_model_ref.selected_artifact_path, None);
    }

    #[test]
    fn candidate_drafts_report_backend_hint_mismatch() {
        let (drafts, diagnostics) = candidate_drafts_from_projected_facts(
            &pumas_package_facts(vec![inference::BackendHintLabel::Diffusers]),
            &runtime_capability_facts(vec![runtime_capability("llama.cpp", vec!["llama.cpp"])]),
        );

        assert!(drafts.is_empty());
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(
            diagnostic.code,
            SchedulerDispatchSelectionDiagnosticCode::IncompatibleRuntimeRequirement
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(INCOMPATIBLE_RUNTIME_BACKEND_HINT)
        );
    }

    fn has_hint(diagnostics: &[SchedulerDispatchSelectionDiagnostic], hint: &str) -> bool {
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.hint.as_deref() == Some(hint))
    }

    fn path_free_model_ref() -> PumasModelRef {
        PumasModelRef {
            model_id: "pumas.model.sdxl".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }

    fn path_carrying_model_ref() -> PumasModelRef {
        PumasModelRef {
            selected_artifact_path: Some("/models/sdxl".to_string()),
            ..path_free_model_ref()
        }
    }

    fn pumas_package_facts(
        accepted_backend_hints: Vec<inference::BackendHintLabel>,
    ) -> PumasDispatchPackageFactsProjection {
        PumasDispatchPackageFactsProjection {
            model_ref: path_free_model_ref(),
            artifact_kind: inference::ModelArtifactKind::DiffusersBundle,
            validation_state: inference::ModelValidationState::Valid,
            task: inference::TaskEvidence {
                pipeline_tag: Some("text-to-image".to_string()),
                task_type_primary: Some("image-generation".to_string()),
                input_modalities: vec!["text".to_string()],
                output_modalities: vec!["image".to_string()],
            },
            backend_hints: inference::BackendHintFacts {
                accepted: accepted_backend_hints,
                raw: Vec::new(),
                unsupported: Vec::new(),
            },
            requires_custom_code: false,
            diffusers: None,
        }
    }

    fn runtime_capability_facts(
        runtimes: Vec<RuntimeDispatchRuntimeCapabilityFacts>,
    ) -> RuntimeDispatchCapabilityFactsProjection {
        RuntimeDispatchCapabilityFactsProjection {
            generated_at_ms: 1,
            runtimes,
        }
    }

    fn runtime_capability(
        runtime_id: &str,
        backend_keys: Vec<&str>,
    ) -> RuntimeDispatchRuntimeCapabilityFacts {
        RuntimeDispatchRuntimeCapabilityFacts {
            runtime_id: runtime_id.to_string(),
            backend_keys: backend_keys.into_iter().map(str::to_string).collect(),
            status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some(format!("{runtime_id}.instance")),
            loaded_model_ids: Vec::new(),
            active_reservation_ids: Vec::new(),
            has_admission_budget: true,
        }
    }
}
