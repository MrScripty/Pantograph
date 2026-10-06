//! Validated scheduler selection and the separately approved executable target.
use std::path::Path;

use crate::backend::BackendError;
use crate::{
    BackendExecutionDecision, InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy,
    InferenceExecutionInput, InferenceExecutionRequest, InferenceTaskId, ModelArtifactKind,
    ModelStorageKind, ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef, ResolvedModelPackageFacts,
};

pub(crate) struct SelectedEmbeddingLoad<'a> {
    pub(crate) package: &'a ResolvedModelPackageFacts,
    pub(crate) model_directory: &'a Path,
    pub(crate) device: &'a InferenceDeviceId,
}

#[cfg(all(test, feature = "backend-candle"))]
pub(crate) fn fixture(
    width: usize,
) -> (
    tempfile::TempDir,
    InferenceExecutionRequest,
    PumasArtifactLoadTarget,
    BackendExecutionDecision,
) {
    fn copy(source: &Path, target: &Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target.join(entry.file_name()));
            } else {
                std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
            }
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("tests/fixtures/candle_bert/bert-{width}"));
    copy(&source, directory.path());
    // Git does not retain the standard Normalize module's empty directory.
    std::fs::create_dir_all(directory.path().join("2_Normalize")).unwrap();
    let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
        "../tests/fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"
    ))
    .unwrap();
    package.model_ref.model_id = format!("embedding/test/synthetic-bert-{width}");
    package.model_ref.revision = Some(format!("untrained-seed-{}", 171 + width));
    let target = PumasArtifactLoadTarget {
        model_ref: package.model_ref.clone(),
        artifact_kind: package.artifact.artifact_kind.clone(),
        local_load_path: directory.path().to_str().unwrap().into(),
        load_path_kind: PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("synthetic-test-root".into()),
        storage_kind: package.artifact.storage_kind.clone(),
        validation_state: ModelValidationState::Valid,
        content_fingerprint: None,
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    let device = InferenceDeviceId::parse("cpu").unwrap();
    let runtime = crate::RuntimeVariantId::parse("candle.cpu").unwrap();
    let decision = BackendExecutionDecision {
        selected_backend_id: crate::BackendId::parse("candle").unwrap(),
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
        selected_task_id: Some(InferenceTaskId::Embedding),
        selected_model_ref: Some(package.model_ref.clone()),
        diagnostics: vec![],
        dependency_readiness: vec![],
        selection_policy_trace: None,
    };
    let request = InferenceExecutionRequest {
        request_id: Some("selected-embedding-test".into()),
        task_id: InferenceTaskId::Embedding,
        model_ref: Some(package.model_ref.clone()),
        model_name: None,
        resolved_model_package_facts: Some(package),
        input: InferenceExecutionInput::Embedding {
            texts: vec![
                "hello world".into(),
                "hello".into(),
                "different world".into(),
            ],
        },
        generation_options: None,
        extra_options: serde_json::json!({}),
    };
    (directory, request, target, decision)
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::Config(format!("selected embedding: {}", message.into()))
}

fn same_model_identity(left: &PumasModelRef, right: &PumasModelRef) -> bool {
    left.model_id.trim_start_matches("pumas://models/")
        == right.model_id.trim_start_matches("pumas://models/")
        && left.selected_artifact_id == right.selected_artifact_id
        && left.selected_artifact_path == right.selected_artifact_path
}

impl<'a> SelectedEmbeddingLoad<'a> {
    pub(crate) async fn validate(
        request: &'a InferenceExecutionRequest,
        target: &'a PumasArtifactLoadTarget,
        decision: &'a BackendExecutionDecision,
    ) -> Result<Self, BackendError> {
        request
            .validate()
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
        for reference in [model, selected, &package.model_ref] {
            reference
                .validate()
                .map_err(|error| invalid(error.to_string()))?;
            if !same_model_identity(reference, &target.model_ref) {
                return Err(invalid(
                    "request/package/target/scheduler model or artifact mismatch",
                ));
            }
        }
        if request.model_name.as_deref().is_some_and(|name| {
            !name.is_empty()
                && name.strip_prefix("pumas://models/").unwrap_or(name)
                    != target
                        .model_ref
                        .model_id
                        .strip_prefix("pumas://models/")
                        .unwrap_or(&target.model_ref.model_id)
        }) {
            return Err(invalid(
                "request model name conflicts with selected model identity",
            ));
        }
        // Request and scheduler revisions constrain both producer observations.
        // An omitted request revision permits additional target evidence, but
        // never supplies a missing explicitly requested package/target revision.
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
        if request.task_id != InferenceTaskId::Embedding
            || decision.selected_task_id != Some(InferenceTaskId::Embedding)
            || !matches!(request.input, InferenceExecutionInput::Embedding { .. })
        {
            return Err(invalid("explicit embedding task required"));
        }
        let task = crate::resolve_task_registry_entry_from_evidence(&package.task)
            .map_err(|error| invalid(format!("invalid package task evidence: {error:?}")))?;
        if task.task_id != InferenceTaskId::Embedding {
            return Err(invalid("package task is not embedding"));
        }
        if !package.uses_current_contract()
            || target.package_facts_contract_version != Some(package.package_facts_contract_version)
        {
            return Err(invalid(
                "current package and target contract versions are required",
            ));
        }
        let _entry_path = PumasArtifactEntryPath::parse(&package.artifact.entry_path)
            .map_err(|error| invalid(error.to_string()))?;
        if package.artifact.validation_state != ModelValidationState::Valid
            || target.validation_state != ModelValidationState::Valid
            || !package.artifact.validation_errors.is_empty()
            || target.artifact_kind != package.artifact.artifact_kind
            || !matches!(
                target.artifact_kind,
                ModelArtifactKind::HfCompatibleDirectory
            )
            || target.storage_kind == ModelStorageKind::Unknown
            || target.storage_kind != package.artifact.storage_kind
        {
            return Err(invalid(
                "a valid matching local Transformers artifact is required",
            ));
        }
        let path = Path::new(&target.local_load_path);
        if target.load_path_kind != PumasArtifactLoadPathKind::Directory || !path.is_absolute() {
            return Err(invalid(
                "Pumas executable target must declare an absolute Transformers directory",
            ));
        }
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|error| invalid(format!("cannot inspect executable target: {error}")))?;
        let model_directory = if metadata.is_dir() {
            path
        } else if metadata.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some("model.safetensors")
            && package
                .artifact
                .selected_files
                .iter()
                .any(|name| name == "model.safetensors")
            && package
                .components
                .iter()
                .find(|component| {
                    component.kind == crate::ProcessorComponentKind::Weights
                        && component.status == crate::PackageFactStatus::Present
                })
                .and_then(|component| component.relative_path.as_deref())
                == Some("model.safetensors")
        {
            // Current Pumas identifies an HF directory artifact by its selected
            // primary weight entry. Load sibling components from that entry's
            // parent, without replacing its identity or choosing another weight.
            let directory = path
                .parent()
                .ok_or_else(|| invalid("weight target has no parent"))?;
            let canonical_directory = tokio::fs::canonicalize(directory)
                .await
                .map_err(|error| invalid(format!("cannot inspect weight parent: {error}")))?;
            let canonical_weight = tokio::fs::canonicalize(path)
                .await
                .map_err(|error| invalid(format!("cannot inspect selected weight: {error}")))?;
            if canonical_weight.parent() != Some(canonical_directory.as_path()) {
                return Err(invalid(
                    "selected weight resolves outside its Transformers directory",
                ));
            }
            directory
        } else {
            return Err(invalid("Pumas target must be an existing Transformers directory or its selected model.safetensors entry"));
        };
        if package.custom_code.requires_custom_code {
            return Err(invalid("custom Transformers code is denied"));
        }
        let device = decision
            .selected_device_id
            .as_ref()
            .ok_or_else(|| invalid("concrete scheduler device is required"))?;
        let runtime = match decision.selected_device_class {
            InferenceDeviceClass::Cpu if device.as_str() == "cpu" => "candle.cpu",
            _ => return Err(invalid("only a concrete CPU device is supported")),
        };
        let nested = &decision.device_decision;
        if decision.selected_backend_id.as_str() != "candle"
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
            model_directory,
            device,
        })
    }
}

#[cfg(all(test, feature = "backend-candle"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn selected_weight_entry_preserves_artifact_runtime_and_path_guards() {
        for rejection in [
            "none",
            "file_kind",
            "relative",
            "missing",
            "other_weight",
            "unselected",
            "component",
            "model",
            "artifact",
            "runtime",
            "device",
            "custom_code",
        ] {
            let (directory, mut request, mut target, mut decision) = fixture(8);
            target.local_load_path = directory
                .path()
                .join("model.safetensors")
                .to_str()
                .unwrap()
                .into();
            match rejection {
                "none" => {}
                "file_kind" => target.load_path_kind = PumasArtifactLoadPathKind::File,
                "relative" => target.local_load_path = "model.safetensors".into(),
                "missing" => std::fs::remove_file(&target.local_load_path).unwrap(),
                "other_weight" => {
                    let path = directory.path().join("other.safetensors");
                    std::fs::copy(&target.local_load_path, &path).unwrap();
                    target.local_load_path = path.to_str().unwrap().into();
                }
                "unselected" => request
                    .resolved_model_package_facts
                    .as_mut()
                    .unwrap()
                    .artifact
                    .selected_files
                    .clear(),
                "component" => {
                    request
                        .resolved_model_package_facts
                        .as_mut()
                        .unwrap()
                        .components
                        .iter_mut()
                        .find(|component| component.kind == crate::ProcessorComponentKind::Weights)
                        .unwrap()
                        .relative_path = Some("other.safetensors".into())
                }
                "model" => target.model_ref.model_id = "another/model".into(),
                "artifact" => target.model_ref.selected_artifact_id = Some("other".into()),
                "runtime" => {
                    decision.selected_runtime_variant_id =
                        crate::RuntimeVariantId::parse("candle.cuda").unwrap()
                }
                "device" => {
                    decision.selected_device_id = Some(InferenceDeviceId::parse("cuda:0").unwrap())
                }
                "custom_code" => {
                    request
                        .resolved_model_package_facts
                        .as_mut()
                        .unwrap()
                        .custom_code
                        .requires_custom_code = true
                }
                _ => unreachable!(),
            }
            let result = SelectedEmbeddingLoad::validate(&request, &target, &decision).await;
            assert_eq!(result.is_ok(), rejection == "none", "{rejection}");
            if let Ok(selected) = result {
                assert_eq!(selected.model_directory, directory.path());
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn selected_weight_entry_cannot_resolve_outside_component_directory() {
        let (directory, request, target, decision) = fixture(8);
        let outside = tempfile::tempdir().unwrap();
        let weights = directory.path().join("model.safetensors");
        let other = outside.path().join("model.safetensors");
        std::fs::rename(&weights, &other).unwrap();
        std::os::unix::fs::symlink(&other, &weights).unwrap();
        let mut target = target;
        target.local_load_path = weights.to_str().unwrap().into();
        let error = SelectedEmbeddingLoad::validate(&request, &target, &decision)
            .await
            .err()
            .unwrap();
        assert!(error
            .to_string()
            .contains("selected weight resolves outside"));
    }

    #[tokio::test]
    async fn optional_request_revisions_preserve_required_producer_evidence() {
        for (requested, scheduled, package_revision, target_revision, valid) in [
            (None, None, None, None, true),
            (None, None, None, Some("a"), true),
            (None, None, Some("a"), Some("a"), true),
            (Some("a"), Some("a"), Some("a"), Some("a"), true),
            (None, Some("a"), Some("a"), Some("a"), true),
            (Some("a"), None, Some("a"), Some("a"), true),
            (Some("a"), Some("a"), None, Some("a"), false),
            (Some("a"), Some("a"), Some("a"), None, false),
            (Some("a"), Some("a"), Some("b"), Some("a"), false),
            (Some("a"), Some("a"), Some("a"), Some("b"), false),
            (Some("a"), Some("b"), Some("a"), Some("a"), false),
            (None, Some("a"), None, Some("a"), false),
            (None, None, Some("a"), None, false),
            (None, None, Some("a"), Some("b"), false),
        ] {
            let (_directory, mut request, mut target, mut decision) = fixture(8);
            request.model_ref.as_mut().unwrap().revision = requested.map(str::to_owned);
            decision.selected_model_ref.as_mut().unwrap().revision = scheduled.map(str::to_owned);
            request
                .resolved_model_package_facts
                .as_mut()
                .unwrap()
                .model_ref
                .revision = package_revision.map(str::to_owned);
            target.model_ref.revision = target_revision.map(str::to_owned);
            let result = SelectedEmbeddingLoad::validate(&request, &target, &decision).await;
            assert_eq!(result.is_ok(), valid,
                "request={requested:?}, scheduler={scheduled:?}, package={package_revision:?}, target={target_revision:?}");
        }
    }
}
