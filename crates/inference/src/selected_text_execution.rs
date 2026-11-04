//! Validated scheduler selection and the separately approved executable target.
use std::path::Path;

use crate::backend::BackendError;
use crate::{
    BackendExecutionDecision, InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy,
    InferenceExecutionInput, InferenceExecutionRequest, InferenceTaskId, ModelArtifactKind,
    ModelStorageKind, ModelValidationState, PumasArtifactLoadPathKind, PumasArtifactLoadTarget,
    PumasModelRef, ResolvedModelPackageFacts,
};

pub(crate) struct SelectedTextLoad<'a> {
    pub(crate) package: &'a ResolvedModelPackageFacts,
    pub(crate) target: &'a PumasArtifactLoadTarget,
    pub(crate) device: &'a InferenceDeviceId,
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::Config(format!("selected text: {}", message.into()))
}

fn same_model(left: &PumasModelRef, right: &PumasModelRef) -> bool {
    left.model_id.trim_start_matches("pumas://models/")
        == right.model_id.trim_start_matches("pumas://models/")
        && revisions_compatible(left.revision.as_deref(), right.revision.as_deref())
        && left.selected_artifact_id == right.selected_artifact_id
        && artifact_paths_compatible(
            left.selected_artifact_path.as_deref(),
            right.selected_artifact_path.as_deref(),
        )
}

fn revisions_compatible(left: Option<&str>, right: Option<&str>) -> bool {
    left.is_none() || right.is_none() || left == right
}

fn revisions_agree(references: &[&PumasModelRef]) -> bool {
    let mut observed = None;
    for reference in references {
        if let Some(revision) = reference.revision.as_deref() {
            if observed.is_some_and(|previous| previous != revision) {
                return false;
            }
            observed = Some(revision);
        }
    }
    true
}

fn artifact_paths_agree(references: &[&PumasModelRef]) -> bool {
    let mut observed = None;
    for reference in references {
        if let Some(path) = reference.selected_artifact_path.as_deref() {
            if observed.is_some_and(|previous| previous != path) {
                return false;
            }
            observed = Some(path);
        }
    }
    true
}

fn artifact_paths_compatible(left: Option<&str>, right: Option<&str>) -> bool {
    left.is_none() || right.is_none() || left == right
}

fn producer_and_target_paths_agree(
    package: &ResolvedModelPackageFacts,
    target: &PumasArtifactLoadTarget,
) -> bool {
    let package_path = package.artifact.entry_path.trim();
    let target_path = target.local_load_path.trim();
    !is_absolute_local_path(package_path)
        || !is_absolute_local_path(target_path)
        || package_path == target_path
}

fn is_absolute_local_path(path: &str) -> bool {
    Path::new(path).is_absolute()
        || (path.len() >= 3
            && path.as_bytes()[0].is_ascii_alphabetic()
            && path.as_bytes()[1] == b':'
            && matches!(path.as_bytes()[2], b'/' | b'\\'))
}

impl<'a> SelectedTextLoad<'a> {
    pub(crate) async fn validate(
        request: &'a InferenceExecutionRequest,
        target: &'a PumasArtifactLoadTarget,
        decision: &'a BackendExecutionDecision,
    ) -> Result<Self, BackendError> {
        request
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        request
            .validate_text_min_new_tokens_budget(crate::constants::pytorch::DEFAULT_MAX_NEW_TOKENS)
            .map_err(|error| invalid(error.to_string()))?;
        if request
            .request_id
            .as_deref()
            .is_none_or(|id| id.trim().is_empty())
        {
            return Err(invalid("execution request id is required"));
        }
        target
            .validate_for_handoff()
            .map_err(|error| invalid(error.to_string()))?;
        let package = request
            .resolved_model_package_facts
            .as_ref()
            .ok_or_else(|| invalid("resolved package facts are required"))?;
        let model = request
            .model_ref
            .as_ref()
            .ok_or_else(|| invalid("request model identity is required"))?;
        let selected = decision
            .selected_model_ref
            .as_ref()
            .ok_or_else(|| invalid("scheduler model identity is required"))?;
        let references = [model, selected, &package.model_ref];
        if !revisions_agree(&references) {
            return Err(invalid("request/package/scheduler revisions disagree"));
        }
        if !artifact_paths_agree(&references) {
            return Err(invalid("request/package/scheduler artifact paths disagree"));
        }
        for reference in references {
            reference
                .validate()
                .map_err(|error| invalid(error.to_string()))?;
            if !same_model(reference, &target.model_ref) {
                return Err(invalid(
                    "request/package/target/scheduler model or artifact mismatch",
                ));
            }
        }
        if model.selected_artifact_id.is_none()
            || selected.selected_artifact_id.is_none()
            || package.model_ref.selected_artifact_id.is_none()
            || target.model_ref.selected_artifact_id.is_none()
        {
            return Err(invalid("producer-selected artifact identity is required"));
        }
        if model
            .revision
            .as_ref()
            .is_some_and(|revision| selected.revision.as_ref() != Some(revision))
        {
            return Err(invalid(
                "selected revision must preserve the explicit requested revision",
            ));
        }
        for requested in [model, selected] {
            if let Some(revision) = &requested.revision {
                if package.model_ref.revision.as_ref() != Some(revision)
                    || target.model_ref.revision.as_ref() != Some(revision)
                {
                    return Err(invalid(
                        "explicit requested revision is missing or mismatched",
                    ));
                }
            }
        }
        if package
            .model_ref
            .revision
            .as_ref()
            .is_some_and(|revision| target.model_ref.revision.as_ref() != Some(revision))
        {
            return Err(invalid("target must preserve the known package revision"));
        }
        if !matches!(
            request.task_id,
            InferenceTaskId::TextGeneration | InferenceTaskId::ChatCompletion
        ) || decision.selected_task_id != Some(request.task_id.clone())
            || !matches!(
                request.input,
                InferenceExecutionInput::TextGeneration { .. }
            )
        {
            return Err(invalid(
                "exact requested and selected canonical text task required",
            ));
        }
        let task = crate::resolve_task_registry_entry_from_evidence(&package.task)
            .map_err(|error| invalid(format!("invalid package task evidence: {error:?}")))?;
        if task.task_id != request.task_id {
            return Err(invalid(
                "package task must match the requested canonical text task",
            ));
        }
        if !package.uses_current_contract()
            || target.package_facts_contract_version != Some(package.package_facts_contract_version)
        {
            return Err(invalid(
                "current package and target contract versions are required",
            ));
        }
        if !producer_and_target_paths_agree(package, target) {
            return Err(invalid(
                "Pumas package facts and artifact load target must refer to the same executable artifact path",
            ));
        }
        if package.artifact.validation_state != ModelValidationState::Valid
            || target.validation_state != ModelValidationState::Valid
            || !package.artifact.validation_errors.is_empty()
            || target.artifact_kind != package.artifact.artifact_kind
            || !matches!(
                target.artifact_kind,
                ModelArtifactKind::HfCompatibleDirectory | ModelArtifactKind::Safetensors
            )
            || target.storage_kind == ModelStorageKind::Unknown
            || target.storage_kind != package.artifact.storage_kind
        {
            return Err(invalid(
                "a valid matching local Transformers artifact is required",
            ));
        }
        if target.load_path_kind != PumasArtifactLoadPathKind::Directory
            || !Path::new(&target.local_load_path).is_absolute()
            || !tokio::fs::metadata(&target.local_load_path)
                .await
                .map_err(|error| invalid(format!("cannot inspect executable target: {error}")))?
                .is_dir()
        {
            return Err(invalid(
                "Pumas executable target must be an existing absolute directory",
            ));
        }
        if package.custom_code.requires_custom_code {
            return Err(invalid("custom Transformers code is denied"));
        }
        let device = decision
            .selected_device_id
            .as_ref()
            .ok_or_else(|| invalid("concrete scheduler device is required"))?;
        let runtime = match decision.selected_device_class {
            InferenceDeviceClass::Cpu if device.as_str() == "cpu" => "pytorch.cpu",
            InferenceDeviceClass::Mps if device.as_str() == "mps" => "pytorch.mps",
            InferenceDeviceClass::Cuda
                if device
                    .as_str()
                    .strip_prefix("cuda:")
                    .is_some_and(|index| index.parse::<u32>().is_ok()) =>
            {
                "pytorch.cuda"
            }
            _ => return Err(invalid("unsupported or ambiguous selected device")),
        };
        let nested = &decision.device_decision;
        if decision.selected_backend_id.as_str() != "pytorch"
            || decision.selected_runtime_variant_id.as_str() != runtime
            || nested.runtime_variant_id != decision.selected_runtime_variant_id
            || nested.selected_device_class != decision.selected_device_class
            || nested.selected_device_id != decision.selected_device_id
        {
            return Err(invalid("scheduler runtime/device decisions disagree"));
        }
        if let InferenceDevicePolicy::Explicit {
            device_class,
            device_id,
        } = &nested.policy
        {
            if *device_class != decision.selected_device_class
                || device_id.as_ref().is_some_and(|id| id != device)
            {
                return Err(invalid(
                    "selected device conflicts with explicit device policy",
                ));
            }
        }
        Ok(Self {
            package,
            target,
            device,
        })
    }
}

#[cfg(test)]
pub(crate) fn fixture() -> (
    tempfile::TempDir,
    InferenceExecutionRequest,
    PumasArtifactLoadTarget,
    BackendExecutionDecision,
) {
    let directory = tempfile::tempdir().unwrap();
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    )).unwrap();
    package.custom_code.requires_custom_code = false;
    package.custom_code.custom_code_sources.clear();
    package.custom_code.auto_map_sources.clear();
    let target = PumasArtifactLoadTarget {
        model_ref: package.model_ref.clone(),
        artifact_kind: package.artifact.artifact_kind.clone(),
        local_load_path: directory.path().to_str().unwrap().into(),
        load_path_kind: PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("test-root".into()),
        storage_kind: package.artifact.storage_kind.clone(),
        validation_state: ModelValidationState::Valid,
        verification_source_fingerprint: None,
        verification_observed_from_cache_at: None,
        content_fingerprint: None,
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    let device = InferenceDeviceId::parse("cpu").unwrap();
    let runtime = crate::RuntimeVariantId::parse("pytorch.cpu").unwrap();
    let decision = BackendExecutionDecision {
        selected_backend_id: crate::BackendId::parse("pytorch").unwrap(),
        selected_runtime_variant_id: runtime.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(device.clone()),
        device_decision: crate::DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id: runtime,
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device),
            diagnostics: vec![],
        },
        selected_task_id: Some(InferenceTaskId::TextGeneration),
        selected_model_ref: Some(package.model_ref.clone()),
        diagnostics: vec![],
        dependency_readiness: vec![],
        selection_policy_trace: None,
    };
    let request = InferenceExecutionRequest {
        request_id: Some("selected-text-test".into()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package.model_ref.clone()),
        model_name: None,
        resolved_model_package_facts: Some(package),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("  exact prompt\n".into()),
            system_prompt: None,
            messages: vec![],
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };
    (directory, request, target, decision)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_missing_package_revision_when_target_observes_one() {
        assert!(revisions_compatible(None, Some("main")));
        assert!(!revisions_compatible(Some("main"), Some("other")));
    }

    #[tokio::test]
    async fn rejects_conflicting_supplied_revisions_when_target_omits_one() {
        let (_directory, mut request, mut target, mut decision) = fixture();
        request.model_ref.as_mut().expect("request model").revision = Some("main".into());
        request
            .resolved_model_package_facts
            .as_mut()
            .expect("package facts")
            .model_ref
            .revision = Some("other".into());
        decision
            .selected_model_ref
            .as_mut()
            .expect("scheduler model")
            .revision = Some("main".into());
        target.model_ref.revision = None;

        let result = SelectedTextLoad::validate(&request, &target, &decision).await;
        assert!(matches!(
            result,
            Err(BackendError::Config(message))
                if message.contains("request/package/scheduler revisions disagree")
        ));
    }

    #[tokio::test]
    async fn rejects_execution_without_producer_selected_artifact_identity() {
        let (_directory, request, mut target, mut decision) = fixture();
        let mut request = request;
        request
            .model_ref
            .as_mut()
            .expect("fixture has request model identity")
            .selected_artifact_id = None;
        request
            .resolved_model_package_facts
            .as_mut()
            .expect("fixture has package facts")
            .model_ref
            .selected_artifact_id = None;
        target.model_ref.selected_artifact_id = None;
        decision
            .selected_model_ref
            .as_mut()
            .expect("fixture has scheduler model identity")
            .selected_artifact_id = None;

        let error = match SelectedTextLoad::validate(&request, &target, &decision).await {
            Ok(_) => panic!("selected text must require producer artifact identity"),
            Err(error) => error,
        };

        assert!(error
            .to_string()
            .contains("producer-selected artifact identity is required"));
    }

    #[test]
    fn supplied_artifact_paths_must_agree_without_a_target_path() {
        let request = PumasModelRef {
            model_id: "image/model".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: Some("image/a".to_string()),
            migration_diagnostics: Vec::new(),
        };
        let package = PumasModelRef {
            selected_artifact_path: Some("image/b".to_string()),
            ..request.clone()
        };

        assert!(!artifact_paths_agree(&[&request, &package]));
    }

    #[tokio::test]
    async fn canonical_text_tasks_require_exact_request_selection_and_package_agreement() {
        for requested in [
            InferenceTaskId::TextGeneration,
            InferenceTaskId::ChatCompletion,
        ] {
            for selected in [
                InferenceTaskId::TextGeneration,
                InferenceTaskId::ChatCompletion,
            ] {
                for packaged in [
                    InferenceTaskId::TextGeneration,
                    InferenceTaskId::ChatCompletion,
                ] {
                    let (_directory, mut request, target, mut decision) = fixture();
                    request.task_id = requested.clone();
                    decision.selected_task_id = Some(selected.clone());
                    let task = &mut request.resolved_model_package_facts.as_mut().unwrap().task;
                    task.pipeline_tag = None;
                    task.task_type_primary = Some(packaged.canonical_label().to_string());
                    assert_eq!(
                        SelectedTextLoad::validate(&request, &target, &decision)
                            .await
                            .is_ok(),
                        requested == selected && selected == packaged,
                        "request={requested:?}, selected={selected:?}, package={packaged:?}"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn canonical_text_revisions_preserve_explicit_constraints_and_allow_omitted_refinement() {
        for (requested, selected, package, target_revision, valid) in [
            (Some("a"), Some("a"), Some("a"), Some("a"), true),
            (None, Some("a"), Some("a"), Some("a"), true),
            (None, None, Some("a"), Some("a"), true),
            (Some("b"), Some("a"), Some("a"), Some("a"), false),
            (Some("a"), None, Some("a"), Some("a"), false),
            (Some("a"), Some("a"), None, Some("a"), false),
            (Some("a"), Some("a"), Some("a"), None, false),
            (None, Some("a"), Some("a"), Some("b"), false),
            (None, None, Some("a"), None, false),
        ] {
            let (_directory, mut request, mut target, mut decision) = fixture();
            request.model_ref.as_mut().unwrap().revision = requested.map(str::to_owned);
            decision.selected_model_ref.as_mut().unwrap().revision = selected.map(str::to_owned);
            request
                .resolved_model_package_facts
                .as_mut()
                .unwrap()
                .model_ref
                .revision = package.map(str::to_owned);
            target.model_ref.revision = target_revision.map(str::to_owned);
            assert_eq!(SelectedTextLoad::validate(&request, &target, &decision).await.is_ok(), valid,
                "request={requested:?}, selected={selected:?}, package={package:?}, target={target_revision:?}");
        }
    }
}
