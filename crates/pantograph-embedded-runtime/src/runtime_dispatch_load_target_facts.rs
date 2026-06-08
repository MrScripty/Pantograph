use std::collections::BTreeSet;
use std::sync::Arc;

use pantograph_dependency_planning::PumasModelRef;
use pumas_library::models::{
    PumasArtifactConsumer, PumasArtifactLoadTarget, PumasArtifactLoadTargetDiagnostic,
    PumasArtifactLoadTargetResolutionMode, ResolveModelArtifactLoadTargetRequest,
    ResolveModelArtifactLoadTargetResponse,
};
use workflow_nodes::setup::PumasSelectorAccess;

const PANTOGRAPH_RUNTIME_DISPATCH_CONSUMER: &str = "pantograph-embedded-runtime-dispatch";
const MAX_DIAGNOSTICS: usize = 4;

#[derive(Clone)]
pub(crate) struct RuntimeDispatchLoadTargetFactsSource {
    selector_access: Option<Arc<PumasSelectorAccess>>,
}

impl RuntimeDispatchLoadTargetFactsSource {
    pub(crate) fn new(selector_access: Option<Arc<PumasSelectorAccess>>) -> Self {
        Self { selector_access }
    }

    pub(crate) async fn collect(
        &self,
        model_ref: &PumasModelRef,
        runtime_families: Vec<String>,
        task_kind: Option<String>,
    ) -> RuntimeDispatchLoadTargetFactsOutcome {
        resolve_runtime_dispatch_load_target_facts(
            self.selector_access.as_deref(),
            model_ref,
            runtime_families,
            task_kind,
        )
        .await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchLoadTargetFactsProjection {
    pub load_targets: Vec<RuntimeDispatchLoadTargetFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchLoadTargetFact {
    pub runtime_family: String,
    pub resolved_load_target: String,
    pub model_ref: PumasModelRef,
    pub artifact_kind: String,
    pub load_path_kind: String,
    pub library_root_id: Option<String>,
    pub storage_kind: String,
    pub validation_state: String,
    pub content_fingerprint: Option<String>,
    pub package_facts_contract_version: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RuntimeDispatchLoadTargetFactsDiagnostic {
    pub code: RuntimeDispatchLoadTargetFactsDiagnosticCode,
    pub runtime_family: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchLoadTargetFactsDiagnosticCode {
    MissingSelectorAccess,
    UnsupportedSelectorAccessRole,
    MissingRuntimeFamily,
    LoadTargetLookupFailed,
    LoadTargetUnavailable,
    ReadyResponseMissingTarget,
    EmptyLoadTargetPath,
    PathFactsStripped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RuntimeDispatchLoadTargetFactsOutcome {
    Projected {
        facts: RuntimeDispatchLoadTargetFactsProjection,
        diagnostics: Vec<RuntimeDispatchLoadTargetFactsDiagnostic>,
    },
    Unavailable {
        diagnostics: Vec<RuntimeDispatchLoadTargetFactsDiagnostic>,
    },
}

impl RuntimeDispatchLoadTargetFactsOutcome {
    pub(crate) fn diagnostics(&self) -> &[RuntimeDispatchLoadTargetFactsDiagnostic] {
        match self {
            Self::Projected { diagnostics, .. } | Self::Unavailable { diagnostics } => diagnostics,
        }
    }
}

async fn resolve_runtime_dispatch_load_target_facts(
    selector_access: Option<&PumasSelectorAccess>,
    model_ref: &PumasModelRef,
    runtime_families: Vec<String>,
    task_kind: Option<String>,
) -> RuntimeDispatchLoadTargetFactsOutcome {
    let runtime_families = normalized_runtime_families(runtime_families);
    if runtime_families.is_empty() {
        return unavailable(vec![diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingRuntimeFamily,
            None,
            "runtime dispatch load-target facts require at least one runtime family",
        )]);
    }

    let Some(selector_access) = selector_access else {
        return unavailable(vec![diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingSelectorAccess,
            None,
            "Pumas owner API access is required to resolve runtime dispatch load-target facts",
        )]);
    };
    let PumasSelectorAccess::Owner(api) = selector_access else {
        return unavailable(vec![diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::UnsupportedSelectorAccessRole,
            None,
            format!(
                "Pumas {} selector access does not provide owner-fresh load-target facts for runtime dispatch",
                selector_access.role_name()
            ),
        )]);
    };

    let mut facts = Vec::new();
    let mut diagnostics = Vec::new();
    for runtime_family in runtime_families {
        let request = build_runtime_dispatch_load_target_request(
            model_ref,
            &runtime_family,
            task_kind.clone(),
        );
        match api.resolve_model_artifact_load_target(request).await {
            Ok(response) => match project_ready_load_target(response, &runtime_family) {
                Ok((fact, mut fact_diagnostics)) => {
                    facts.push(fact);
                    diagnostics.append(&mut fact_diagnostics);
                }
                Err(diagnostic) => diagnostics.push(diagnostic),
            },
            Err(error) => diagnostics.push(diagnostic(
                RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetLookupFailed,
                Some(runtime_family),
                format!("Pumas load-target lookup failed for runtime dispatch: {error}"),
            )),
        }
    }

    if facts.is_empty() {
        RuntimeDispatchLoadTargetFactsOutcome::Unavailable { diagnostics }
    } else {
        RuntimeDispatchLoadTargetFactsOutcome::Projected {
            facts: RuntimeDispatchLoadTargetFactsProjection {
                load_targets: facts,
            },
            diagnostics,
        }
    }
}

fn normalized_runtime_families(runtime_families: Vec<String>) -> Vec<String> {
    runtime_families
        .into_iter()
        .map(|runtime_family| runtime_family.trim().to_string())
        .filter(|runtime_family| !runtime_family.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn build_runtime_dispatch_load_target_request(
    model_ref: &PumasModelRef,
    runtime_family: &str,
    task_kind: Option<String>,
) -> ResolveModelArtifactLoadTargetRequest {
    ResolveModelArtifactLoadTargetRequest {
        model_ref: pumas_model_ref(model_ref),
        expected_artifact_kind: None,
        caller_observed_entry_path: None,
        caller_observed_package_facts_contract_version: None,
        resolution_mode: PumasArtifactLoadTargetResolutionMode::OwnerFresh,
        consumer: PumasArtifactConsumer {
            consumer_name: PANTOGRAPH_RUNTIME_DISPATCH_CONSUMER.to_string(),
            task_kind,
            runtime_family: Some(runtime_family.to_string()),
        },
    }
}

fn pumas_model_ref(model_ref: &PumasModelRef) -> pumas_library::models::PumasModelRef {
    pumas_library::models::PumasModelRef {
        model_id: model_ref.model_id.clone(),
        revision: model_ref.revision.clone(),
        selected_artifact_id: model_ref.selected_artifact_id.clone(),
        selected_artifact_path: None,
        migration_diagnostics: model_ref
            .migration_diagnostics
            .iter()
            .map(
                |diagnostic| pumas_library::models::ModelRefMigrationDiagnostic {
                    code: diagnostic.code.clone(),
                    message: diagnostic.message.clone(),
                    input: diagnostic.input.clone(),
                },
            )
            .collect(),
        ..Default::default()
    }
}

fn project_ready_load_target(
    response: ResolveModelArtifactLoadTargetResponse,
    runtime_family: &str,
) -> Result<
    (
        RuntimeDispatchLoadTargetFact,
        Vec<RuntimeDispatchLoadTargetFactsDiagnostic>,
    ),
    RuntimeDispatchLoadTargetFactsDiagnostic,
> {
    if !response.is_ready() {
        return Err(diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetUnavailable,
            Some(runtime_family.to_string()),
            format!(
                "Pumas load target is unavailable for runtime dispatch: artifact_state={:?} entry_path_state={:?} diagnostics={}: {:?}",
                response.artifact_state,
                response.entry_path_state,
                response.diagnostics.len(),
                compact_diagnostics(&response.diagnostics)
            ),
        ));
    }
    let target = response.target.ok_or_else(|| {
        diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::ReadyResponseMissingTarget,
            Some(runtime_family.to_string()),
            "ready Pumas load-target response did not include a target",
        )
    })?;
    project_load_target(target, runtime_family)
}

fn project_load_target(
    target: PumasArtifactLoadTarget,
    runtime_family: &str,
) -> Result<
    (
        RuntimeDispatchLoadTargetFact,
        Vec<RuntimeDispatchLoadTargetFactsDiagnostic>,
    ),
    RuntimeDispatchLoadTargetFactsDiagnostic,
> {
    if target.local_load_path.trim().is_empty() {
        return Err(diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::EmptyLoadTargetPath,
            Some(runtime_family.to_string()),
            "Pumas load target must include a non-empty local load path before path stripping",
        ));
    }

    let mut diagnostics = Vec::new();
    let mut model_ref = pantograph_model_ref(target.model_ref);
    if model_ref.selected_artifact_path.take().is_some() {
        diagnostics.push(diagnostic(
            RuntimeDispatchLoadTargetFactsDiagnosticCode::PathFactsStripped,
            Some(runtime_family.to_string()),
            "Pumas load target contained selected artifact path data that was stripped before dispatch projection",
        ));
    }
    let artifact_kind = format!("{:?}", target.artifact_kind);
    let load_path_kind = format!("{:?}", target.load_path_kind);
    let storage_kind = format!("{:?}", target.storage_kind);
    let validation_state = format!("{:?}", target.validation_state);
    let resolved_load_target = dispatch_safe_load_target_id(
        &model_ref,
        &artifact_kind,
        &load_path_kind,
        &storage_kind,
        target.content_fingerprint.as_deref(),
    );
    Ok((
        RuntimeDispatchLoadTargetFact {
            runtime_family: runtime_family.to_string(),
            resolved_load_target,
            model_ref,
            artifact_kind,
            load_path_kind,
            library_root_id: target.library_root_id,
            storage_kind,
            validation_state,
            content_fingerprint: target.content_fingerprint,
            package_facts_contract_version: target.package_facts_contract_version,
        },
        diagnostics,
    ))
}

fn pantograph_model_ref(model_ref: pumas_library::models::PumasModelRef) -> PumasModelRef {
    PumasModelRef {
        model_id: model_ref.model_id,
        revision: model_ref.revision,
        selected_artifact_id: model_ref.selected_artifact_id,
        selected_artifact_path: model_ref.selected_artifact_path,
        migration_diagnostics: model_ref
            .migration_diagnostics
            .into_iter()
            .map(
                |diagnostic| pantograph_dependency_planning::ModelRefMigrationDiagnostic {
                    code: diagnostic.code,
                    message: diagnostic.message,
                    input: diagnostic.input,
                },
            )
            .collect(),
    }
}

fn dispatch_safe_load_target_id(
    model_ref: &PumasModelRef,
    artifact_kind: &str,
    load_path_kind: &str,
    storage_kind: &str,
    content_fingerprint: Option<&str>,
) -> String {
    format!(
        "pumas:{}:{}:{}:{}:{}",
        model_ref.model_id,
        model_ref
            .selected_artifact_id
            .as_deref()
            .unwrap_or("selected-artifact"),
        artifact_kind,
        load_path_kind,
        content_fingerprint.unwrap_or(storage_kind)
    )
}

fn compact_diagnostics(diagnostics: &[PumasArtifactLoadTargetDiagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .take(MAX_DIAGNOSTICS)
        .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
        .collect()
}

fn unavailable(
    diagnostics: Vec<RuntimeDispatchLoadTargetFactsDiagnostic>,
) -> RuntimeDispatchLoadTargetFactsOutcome {
    RuntimeDispatchLoadTargetFactsOutcome::Unavailable { diagnostics }
}

fn diagnostic(
    code: RuntimeDispatchLoadTargetFactsDiagnosticCode,
    runtime_family: Option<String>,
    message: impl Into<String>,
) -> RuntimeDispatchLoadTargetFactsDiagnostic {
    RuntimeDispatchLoadTargetFactsDiagnostic {
        code,
        runtime_family,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use pumas_library::models::{
        AssetValidationState, ModelArtifactState, ModelEntryPathState, PackageArtifactKind,
        PumasArtifactLoadPathKind, StorageKind, PACKAGE_FACTS_CONTRACT_VERSION,
    };

    use super::*;

    #[tokio::test]
    async fn source_requires_owner_access() {
        let source = RuntimeDispatchLoadTargetFactsSource::new(None);

        let outcome = source
            .collect(
                &model_ref(),
                vec!["diffusers".to_string()],
                Some("image_generation".to_string()),
            )
            .await;

        assert!(matches!(
            outcome,
            RuntimeDispatchLoadTargetFactsOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == RuntimeDispatchLoadTargetFactsDiagnosticCode::MissingSelectorAccess
        }));
    }

    #[test]
    fn request_uses_path_free_model_ref_and_registry_runtime_family() {
        let mut model_ref = model_ref();
        model_ref.selected_artifact_path = Some("/host/private/model".to_string());

        let request = build_runtime_dispatch_load_target_request(
            &model_ref,
            "diffusers",
            Some("image_generation".to_string()),
        );

        assert_eq!(request.model_ref.model_id, "pumas.model.sdxl");
        assert_eq!(
            request.model_ref.selected_artifact_id.as_deref(),
            Some("diffusers")
        );
        assert_eq!(request.model_ref.selected_artifact_path, None);
        assert_eq!(request.caller_observed_entry_path, None);
        assert_eq!(
            request.consumer.runtime_family.as_deref(),
            Some("diffusers")
        );
        assert_eq!(
            request.consumer.task_kind.as_deref(),
            Some("image_generation")
        );
    }

    #[test]
    fn projection_strips_host_paths_from_ready_load_target() {
        let response = ResolveModelArtifactLoadTargetResponse {
            artifact_state: ModelArtifactState::Ready,
            entry_path_state: ModelEntryPathState::Ready,
            target: Some(load_target()),
            diagnostics: Vec::new(),
        };

        let (fact, diagnostics) =
            project_ready_load_target(response, "diffusers").expect("ready load target");

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == RuntimeDispatchLoadTargetFactsDiagnosticCode::PathFactsStripped
                && diagnostic.runtime_family.as_deref() == Some("diffusers")
        }));
        assert_eq!(fact.runtime_family, "diffusers");
        assert_eq!(fact.model_ref.selected_artifact_path, None);
        assert!(!fact.resolved_load_target.contains("/host/private"));
        assert_eq!(fact.content_fingerprint.as_deref(), Some("sha256:abc"));
    }

    #[test]
    fn unavailable_response_is_typed_by_runtime_family() {
        let response = ResolveModelArtifactLoadTargetResponse {
            artifact_state: ModelArtifactState::Missing,
            entry_path_state: ModelEntryPathState::Missing,
            target: None,
            diagnostics: Vec::new(),
        };

        let diagnostic = project_ready_load_target(response, "diffusers")
            .expect_err("unavailable load target must fail");

        assert_eq!(
            diagnostic.code,
            RuntimeDispatchLoadTargetFactsDiagnosticCode::LoadTargetUnavailable
        );
        assert_eq!(diagnostic.runtime_family.as_deref(), Some("diffusers"));
    }

    fn model_ref() -> PumasModelRef {
        PumasModelRef {
            model_id: "pumas.model.sdxl".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("diffusers".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }

    fn load_target() -> PumasArtifactLoadTarget {
        PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: "pumas.model.sdxl".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("diffusers".to_string()),
                selected_artifact_path: Some("/host/private/sdxl".to_string()),
                ..Default::default()
            },
            artifact_kind: PackageArtifactKind::DiffusersBundle,
            local_load_path: "/host/private/sdxl".to_string(),
            load_path_kind: PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("default".to_string()),
            storage_kind: StorageKind::LibraryOwned,
            validation_state: AssetValidationState::Valid,
            content_fingerprint: Some("sha256:abc".to_string()),
            package_facts_contract_version: Some(PACKAGE_FACTS_CONTRACT_VERSION),
        }
    }
}
