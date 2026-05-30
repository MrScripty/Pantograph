use std::sync::Arc;

use pantograph_dependency_planning::PumasModelRef;
use workflow_nodes::setup::PumasSelectorAccess;

#[derive(Clone)]
pub(crate) struct PumasDispatchPackageFactsSource {
    selector_access: Option<Arc<PumasSelectorAccess>>,
}

impl PumasDispatchPackageFactsSource {
    pub(crate) fn new(selector_access: Option<Arc<PumasSelectorAccess>>) -> Self {
        Self { selector_access }
    }

    pub(crate) async fn collect(
        &self,
        model_ref: &PumasModelRef,
    ) -> PumasDispatchPackageFactsBridgeOutcome {
        resolve_pumas_dispatch_package_facts(self.selector_access.as_deref(), model_ref).await
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PumasDispatchPackageFactsProjection {
    pub model_ref: PumasModelRef,
    pub artifact_kind: inference::ModelArtifactKind,
    pub validation_state: inference::ModelValidationState,
    pub task: inference::TaskEvidence,
    pub backend_hints: inference::BackendHintFacts,
    pub requires_custom_code: bool,
    pub diffusers: Option<PumasDispatchDiffusersFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PumasDispatchDiffusersFacts {
    pub status: inference::PackageFactStatus,
    pub pipeline_class: Option<String>,
    pub diffusers_version: Option<String>,
    pub name_or_path: Option<String>,
    pub task: inference::TaskEvidence,
    pub family_evidence: Vec<PumasDispatchImageFamilyEvidence>,
    pub components: Vec<PumasDispatchDiffusersComponentFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PumasDispatchImageFamilyEvidence {
    pub family: inference::ImageGenerationFamilyLabel,
    pub source: inference::ImageGenerationFamilyEvidenceSource,
    pub value_source: inference::PackageFactValueSource,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PumasDispatchDiffusersComponentFacts {
    pub role: inference::DiffusersComponentRole,
    pub status: inference::PackageFactStatus,
    pub source_library: Option<String>,
    pub class_name: Option<String>,
    pub config_model_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PumasDispatchPackageFactsDiagnostic {
    pub code: PumasDispatchPackageFactsDiagnosticCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PumasDispatchPackageFactsDiagnosticCode {
    InvalidModelRef,
    PathCarryingModelRef,
    MissingSelectorAccess,
    UnsupportedSelectorAccessRole,
    PackageFactsLookupFailed,
    PackageFactsDecodeFailed,
    StalePackageFactsContract,
    SelectedArtifactMismatch,
    PathFactsStripped,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PumasDispatchPackageFactsBridgeOutcome {
    Projected {
        facts: PumasDispatchPackageFactsProjection,
        diagnostics: Vec<PumasDispatchPackageFactsDiagnostic>,
    },
    Unavailable {
        diagnostics: Vec<PumasDispatchPackageFactsDiagnostic>,
    },
}

impl PumasDispatchPackageFactsBridgeOutcome {
    pub(crate) fn diagnostics(&self) -> &[PumasDispatchPackageFactsDiagnostic] {
        match self {
            Self::Projected { diagnostics, .. } | Self::Unavailable { diagnostics } => diagnostics,
        }
    }
}

pub(crate) async fn resolve_pumas_dispatch_package_facts(
    selector_access: Option<&PumasSelectorAccess>,
    model_ref: &PumasModelRef,
) -> PumasDispatchPackageFactsBridgeOutcome {
    let mut diagnostics = validate_dispatch_model_ref(model_ref);
    if !diagnostics.is_empty() {
        return PumasDispatchPackageFactsBridgeOutcome::Unavailable { diagnostics };
    }

    let Some(selector_access) = selector_access else {
        return unavailable(
            PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess,
            format!(
                "Pumas owner API access is required to resolve full package facts for model '{}'",
                model_ref.model_id
            ),
        );
    };

    let api = match selector_access {
        PumasSelectorAccess::Owner(api) => api,
        PumasSelectorAccess::LocalClient(_) | PumasSelectorAccess::ReadOnly(_) => {
            return unavailable(
                PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole,
                format!(
                    "Pumas {} selector access does not provide full package facts for runtime dispatch",
                    selector_access.role_name()
                ),
            );
        }
    };

    let raw_facts = match api
        .resolve_model_package_facts(model_ref.model_id.as_str())
        .await
    {
        Ok(facts) => facts,
        Err(error) => {
            return unavailable(
                PumasDispatchPackageFactsDiagnosticCode::PackageFactsLookupFailed,
                format!(
                    "Pumas package facts lookup failed for model '{}': {error}",
                    model_ref.model_id
                ),
            );
        }
    };
    let facts = match decode_pumas_package_facts(raw_facts) {
        Ok(facts) => facts,
        Err(error) => {
            return unavailable(
                PumasDispatchPackageFactsDiagnosticCode::PackageFactsDecodeFailed,
                format!(
                    "Pumas package facts for model '{}' do not match the inference contract: {error}",
                    model_ref.model_id
                ),
            );
        }
    };
    if !facts.uses_current_contract() {
        return unavailable(
            PumasDispatchPackageFactsDiagnosticCode::StalePackageFactsContract,
            format!(
                "Pumas package facts for model '{}' use stale contract version {}",
                model_ref.model_id, facts.package_facts_contract_version
            ),
        );
    }
    if model_ref.selected_artifact_id.is_some()
        && facts.model_ref.selected_artifact_id != model_ref.selected_artifact_id
    {
        return unavailable(
            PumasDispatchPackageFactsDiagnosticCode::SelectedArtifactMismatch,
            format!(
                "Pumas package facts for model '{}' do not match selected artifact {:?}",
                model_ref.model_id, model_ref.selected_artifact_id
            ),
        );
    }
    if facts.model_ref.selected_artifact_path.is_some() {
        diagnostics.push(diagnostic(
            PumasDispatchPackageFactsDiagnosticCode::PathFactsStripped,
            format!(
                "Pumas package facts for model '{}' contained selected artifact path data that was stripped before dispatch projection",
                model_ref.model_id
            ),
        ));
    }

    PumasDispatchPackageFactsBridgeOutcome::Projected {
        facts: project_dispatch_package_facts(facts),
        diagnostics,
    }
}

fn validate_dispatch_model_ref(
    model_ref: &PumasModelRef,
) -> Vec<PumasDispatchPackageFactsDiagnostic> {
    if let Err(error) = model_ref.validate() {
        return vec![diagnostic(
            PumasDispatchPackageFactsDiagnosticCode::InvalidModelRef,
            format!("Pumas model ref is invalid: {error}"),
        )];
    }
    if model_ref.selected_artifact_path.is_some() {
        return vec![diagnostic(
            PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef,
            "Pumas model ref selected_artifact_path is not allowed for scheduler dispatch"
                .to_string(),
        )];
    }
    Vec::new()
}

fn project_dispatch_package_facts(
    facts: inference::ResolvedModelPackageFacts,
) -> PumasDispatchPackageFactsProjection {
    let mut model_ref = facts.model_ref;
    model_ref.selected_artifact_path = None;

    PumasDispatchPackageFactsProjection {
        model_ref,
        artifact_kind: facts.artifact.artifact_kind,
        validation_state: facts.artifact.validation_state,
        task: facts.task,
        backend_hints: facts.backend_hints,
        requires_custom_code: facts.custom_code.requires_custom_code,
        diffusers: facts.diffusers.map(project_diffusers_facts),
    }
}

fn project_diffusers_facts(
    facts: inference::DiffusersPackageEvidence,
) -> PumasDispatchDiffusersFacts {
    PumasDispatchDiffusersFacts {
        status: facts.status,
        pipeline_class: facts.pipeline_class,
        diffusers_version: facts.diffusers_version,
        name_or_path: facts.name_or_path,
        task: facts.task,
        family_evidence: facts
            .family_evidence
            .into_iter()
            .map(|evidence| PumasDispatchImageFamilyEvidence {
                family: evidence.family,
                source: evidence.source,
                value_source: evidence.value_source,
                message: evidence.message,
            })
            .collect(),
        components: facts
            .components
            .into_iter()
            .map(|component| PumasDispatchDiffusersComponentFacts {
                role: component.role,
                status: component.status,
                source_library: component.source_library,
                class_name: component.class_name,
                config_model_type: component.config_model_type,
            })
            .collect(),
    }
}

fn decode_pumas_package_facts(
    facts: pumas_library::models::ResolvedModelPackageFacts,
) -> Result<inference::ResolvedModelPackageFacts, serde_json::Error> {
    let mut value = serde_json::to_value(facts)?;
    if let Some(model_ref) = value
        .get_mut("model_ref")
        .and_then(serde_json::Value::as_object_mut)
    {
        model_ref.remove("model_ref_contract_version");
    }
    serde_json::from_value(value)
}

fn unavailable(
    code: PumasDispatchPackageFactsDiagnosticCode,
    message: String,
) -> PumasDispatchPackageFactsBridgeOutcome {
    PumasDispatchPackageFactsBridgeOutcome::Unavailable {
        diagnostics: vec![diagnostic(code, message)],
    }
}

fn diagnostic(
    code: PumasDispatchPackageFactsDiagnosticCode,
    message: String,
) -> PumasDispatchPackageFactsDiagnostic {
    PumasDispatchPackageFactsDiagnostic { code, message }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn owner_api_projects_diffusers_package_facts_without_paths() {
        let temp_dir = create_test_env();
        let model_id = "diffusion/imported/test-bundle";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_test_diffusers_bundle(&model_dir);
        write_imported_diffusion_metadata(&model_dir, model_id, &model_dir);
        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        api.rebuild_model_index()
            .await
            .expect("model index rebuild");
        let access = PumasSelectorAccess::Owner(api);

        let outcome = resolve_pumas_dispatch_package_facts(
            Some(&access),
            &PumasModelRef {
                model_id: model_id.to_string(),
                revision: None,
                selected_artifact_id: Some("diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
        )
        .await;

        let PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. } = &outcome else {
            panic!("owner API should project package facts: {outcome:?}");
        };
        assert_eq!(facts.model_ref.model_id, model_id);
        assert_eq!(
            facts.model_ref.selected_artifact_id.as_deref(),
            Some("diffusers")
        );
        assert_eq!(facts.model_ref.selected_artifact_path, None);
        assert_eq!(
            facts.artifact_kind,
            pantograph_dependency_planning::ModelArtifactKind::DiffusersBundle
        );
        assert!(facts
            .backend_hints
            .accepted
            .contains(&inference::BackendHintLabel::Diffusers));
        let diffusers = facts
            .diffusers
            .as_ref()
            .unwrap_or_else(|| panic!("diffusers facts missing: {facts:?}"));
        assert_eq!(
            diffusers.pipeline_class.as_deref(),
            Some("StableDiffusionPipeline")
        );
        assert!(diffusers.components.iter().any(|component| {
            component.role == inference::DiffusersComponentRole::Unet
                && component.class_name.as_deref() == Some("UNet2DConditionModel")
        }));
    }

    #[tokio::test]
    async fn source_preserves_owner_api_projected_facts() {
        let temp_dir = create_test_env();
        let model_id = "diffusion/imported/test-bundle";
        let model_dir = temp_dir
            .path()
            .join("shared-resources/models")
            .join(model_id);
        write_test_diffusers_bundle(&model_dir);
        write_imported_diffusion_metadata(&model_dir, model_id, &model_dir);
        let api = Arc::new(
            pumas_library::PumasApi::builder(temp_dir.path())
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        api.rebuild_model_index()
            .await
            .expect("model index rebuild");
        let source =
            PumasDispatchPackageFactsSource::new(Some(Arc::new(PumasSelectorAccess::Owner(api))));

        let outcome = source
            .collect(&model_ref(model_id, Some("diffusers")))
            .await;

        let PumasDispatchPackageFactsBridgeOutcome::Projected { facts, .. } = outcome else {
            panic!("owner API source should project package facts");
        };
        assert_eq!(facts.model_ref.model_id, model_id);
        assert_eq!(
            facts.model_ref.selected_artifact_id.as_deref(),
            Some("diffusers")
        );
        assert_eq!(facts.model_ref.selected_artifact_path, None);
    }

    #[tokio::test]
    async fn source_preserves_missing_selector_access_diagnostic() {
        let source = PumasDispatchPackageFactsSource::new(None);

        let outcome = source
            .collect(&model_ref("diffusion/imported/test-bundle", None))
            .await;

        assert!(matches!(
            outcome,
            PumasDispatchPackageFactsBridgeOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == PumasDispatchPackageFactsDiagnosticCode::MissingSelectorAccess
        }));
    }

    #[tokio::test]
    async fn read_only_selector_access_does_not_promote_summaries() {
        let temp_dir = create_test_env();
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
        .expect("read-only library");
        let access = PumasSelectorAccess::ReadOnly(Arc::new(read_only));

        let outcome = resolve_pumas_dispatch_package_facts(
            Some(&access),
            &PumasModelRef {
                model_id: "diffusion/imported/test-bundle".to_string(),
                revision: None,
                selected_artifact_id: None,
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
        )
        .await;

        assert!(matches!(
            outcome,
            PumasDispatchPackageFactsBridgeOutcome::Unavailable { .. }
        ));
        assert!(outcome.diagnostics().iter().any(|diagnostic| {
            diagnostic.code
                == PumasDispatchPackageFactsDiagnosticCode::UnsupportedSelectorAccessRole
        }));
    }

    #[tokio::test]
    async fn path_carrying_model_ref_is_rejected_before_pumas_lookup() {
        let outcome = resolve_pumas_dispatch_package_facts(
            None,
            &PumasModelRef {
                model_id: "diffusion/imported/test-bundle".to_string(),
                revision: None,
                selected_artifact_id: None,
                selected_artifact_path: Some("shared-resources/models/model".to_string()),
                migration_diagnostics: Vec::new(),
            },
        )
        .await;

        assert!(matches!(
            outcome,
            PumasDispatchPackageFactsBridgeOutcome::Unavailable { .. }
        ));
        assert_eq!(
            outcome.diagnostics()[0].code,
            PumasDispatchPackageFactsDiagnosticCode::PathCarryingModelRef
        );
    }

    fn create_test_env() -> tempfile::TempDir {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/metadata")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/cache")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("launcher-data/logs")).unwrap();
        std::fs::create_dir_all(temp_dir.path().join("shared-resources/models")).unwrap();
        temp_dir
    }

    fn model_ref(model_id: &str, selected_artifact_id: Option<&str>) -> PumasModelRef {
        PumasModelRef {
            model_id: model_id.to_string(),
            revision: None,
            selected_artifact_id: selected_artifact_id.map(str::to_string),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }
    }

    fn write_test_diffusers_bundle(root: &std::path::Path) {
        std::fs::create_dir_all(root.join("scheduler")).unwrap();
        std::fs::create_dir_all(root.join("text_encoder")).unwrap();
        std::fs::create_dir_all(root.join("tokenizer")).unwrap();
        std::fs::create_dir_all(root.join("unet")).unwrap();
        std::fs::create_dir_all(root.join("vae")).unwrap();
        std::fs::write(
            root.join("model_index.json"),
            serde_json::json!({
                "_class_name": "StableDiffusionPipeline",
                "_diffusers_version": "0.32.0",
                "_name_or_path": "synthetic/tiny-sd",
                "scheduler": ["diffusers", "EulerDiscreteScheduler"],
                "text_encoder": ["transformers", "CLIPTextModel"],
                "tokenizer": ["transformers", "CLIPTokenizer"],
                "unet": ["diffusers", "UNet2DConditionModel"],
                "vae": ["diffusers", "AutoencoderKL"]
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_imported_diffusion_metadata(
        model_dir: &std::path::Path,
        model_id: &str,
        entry_path: &std::path::Path,
    ) {
        std::fs::create_dir_all(model_dir).unwrap();
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": model_id,
                "family": "imported",
                "model_type": "diffusion",
                "official_name": "test-bundle",
                "cleaned_name": "test-bundle",
                "source_path": entry_path.display().to_string(),
                "entry_path": entry_path.display().to_string(),
                "storage_kind": "external_reference",
                "bundle_format": "diffusers_directory",
                "pipeline_class": "StableDiffusionPipeline",
                "selected_artifact_id": "diffusers",
                "import_state": "ready",
                "validation_state": "valid",
                "pipeline_tag": "text-to-image",
                "task_type_primary": "text-to-image",
                "input_modalities": ["text"],
                "output_modalities": ["image"],
                "task_classification_source": "external-diffusers-import",
                "task_classification_confidence": 1.0,
                "model_type_resolution_source": "external-diffusers-import",
                "model_type_resolution_confidence": 1.0,
                "recommended_backend": "diffusers",
                "runtime_engine_hints": ["diffusers", "pytorch"]
            })
            .to_string(),
        )
        .unwrap();
    }
}
