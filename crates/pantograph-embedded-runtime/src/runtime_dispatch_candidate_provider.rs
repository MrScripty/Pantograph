use pantograph_dependency_planning::{DependencyReadinessProofEnvelope, PumasModelRef};
use pantograph_scheduler::{
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionDiagnosticCode,
    SchedulerDispatchSelectionDiagnosticSeverity, SchedulerTaskStateRecord,
};
use pantograph_workflow_service::workflow::{
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchCandidateProviderError,
    WorkflowRuntimeDispatchCandidateSet,
};
use pantograph_workflow_service::WorkflowSchedulerTask;

const MISSING_PUMAS_PACKAGE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_pumas_package_facts";
const MISSING_RUNTIME_CAPABILITY_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_capability_facts";
const MISSING_RUNTIME_RESOURCE_FACTS_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.missing_runtime_resource_facts";
const PATH_CARRYING_MODEL_REF_HINT: &str =
    "embedded_runtime_dispatch_candidate_provider.path_carrying_model_ref";

#[derive(Debug, Default, Clone)]
pub(crate) struct EmbeddedRuntimeDispatchCandidateProvider;

impl EmbeddedRuntimeDispatchCandidateProvider {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self
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
                &readiness_proof.preflight_result.identity_key.model_ref,
            ),
        })
    }
}

fn fail_closed_diagnostics(model_ref: &PumasModelRef) -> Vec<SchedulerDispatchSelectionDiagnostic> {
    let mut diagnostics = Vec::new();
    if model_ref.selected_artifact_path.is_some() {
        diagnostics.push(provider_diagnostic(
            SchedulerDispatchSelectionDiagnosticCode::InvalidCandidateEvidence,
            "runtime dispatch candidate provider rejected a path-carrying Pumas model ref",
            PATH_CARRYING_MODEL_REF_HINT,
        ));
        return diagnostics;
    }

    diagnostics.push(provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        "runtime dispatch candidate provider has no staged Pumas package facts",
        MISSING_PUMAS_PACKAGE_FACTS_HINT,
    ));
    diagnostics.push(provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        "runtime dispatch candidate provider has no staged runtime capability facts",
        MISSING_RUNTIME_CAPABILITY_FACTS_HINT,
    ));
    diagnostics.push(provider_diagnostic(
        SchedulerDispatchSelectionDiagnosticCode::NoCandidates,
        "runtime dispatch candidate provider has no staged runtime resource facts",
        MISSING_RUNTIME_RESOURCE_FACTS_HINT,
    ));
    diagnostics
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

    #[test]
    fn fail_closed_provider_reports_missing_source_facts() {
        let diagnostics = fail_closed_diagnostics(&path_free_model_ref());

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
        let diagnostics = fail_closed_diagnostics(&path_carrying_model_ref());

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
}
