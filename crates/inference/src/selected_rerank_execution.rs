//! Validated scheduler selection and the separately approved executable target.
use std::path::Path;

use crate::backend::BackendError;
use crate::{
    BackendExecutionDecision, InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy,
    InferenceExecutionInput, InferenceExecutionRequest, InferenceTaskId, ModelArtifactKind,
    ModelStorageKind, ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef, ResolvedModelPackageFacts,
};

pub(crate) struct SelectedRerankLoad<'a> {
    pub(crate) package: &'a ResolvedModelPackageFacts,
    pub(crate) target: &'a PumasArtifactLoadTarget,
    pub(crate) device: &'a InferenceDeviceId,
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::Config(format!("selected rerank: {}", message.into()))
}

fn same_model(left: &PumasModelRef, right: &PumasModelRef) -> bool {
    left.model_id.trim_start_matches("pumas://models/")
        == right.model_id.trim_start_matches("pumas://models/")
        && left.selected_artifact_id == right.selected_artifact_id
        && left.selected_artifact_path == right.selected_artifact_path
}

impl<'a> SelectedRerankLoad<'a> {
    pub(crate) async fn validate(
        request: &'a InferenceExecutionRequest,
        target: &'a PumasArtifactLoadTarget,
        decision: &'a BackendExecutionDecision,
    ) -> Result<Self, BackendError> {
        request
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        if request.extra_options.as_object().is_some_and(|options| {
            [
                "model",
                "query",
                "documents",
                "top_n",
                "return_documents",
                "return_text",
            ]
            .iter()
            .any(|key| options.contains_key(*key))
        }) {
            return Err(invalid(
                "backend options cannot replace canonical rerank identity or inputs",
            ));
        }
        if request.generation_options.is_some() {
            return Err(invalid(
                "generation controls are unsupported for selected rerank",
            ));
        }
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
            if !same_model(reference, &target.model_ref) {
                return Err(invalid(
                    "request/package/target/scheduler model or artifact mismatch",
                ));
            }
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
        if request.task_id != InferenceTaskId::Rerank
            || decision.selected_task_id != Some(request.task_id.clone())
            || !matches!(request.input, InferenceExecutionInput::Rerank { .. })
        {
            return Err(invalid(
                "exact requested and selected canonical rerank task required",
            ));
        }
        let task = crate::resolve_task_registry_entry_from_evidence(&package.task)
            .map_err(|error| invalid(format!("invalid package task evidence: {error:?}")))?;
        if task.task_id != request.task_id {
            return Err(invalid(
                "package task must match the requested canonical rerank task",
            ));
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
            || target.artifact_kind != ModelArtifactKind::Gguf
            || target.storage_kind == ModelStorageKind::Unknown
            || target.storage_kind != package.artifact.storage_kind
        {
            return Err(invalid("a valid matching local GGUF artifact is required"));
        }
        if target.load_path_kind != PumasArtifactLoadPathKind::File
            || !Path::new(&target.local_load_path).is_absolute()
            || !tokio::fs::metadata(&target.local_load_path)
                .await
                .map_err(|error| invalid(format!("cannot inspect executable target: {error}")))?
                .is_file()
        {
            return Err(invalid(
                "Pumas executable target must be an existing absolute file",
            ));
        }
        if package.custom_code.requires_custom_code {
            return Err(invalid("custom model code is denied"));
        }
        let device = decision
            .selected_device_id
            .as_ref()
            .ok_or_else(|| invalid("concrete scheduler device is required"))?;
        if decision.selected_device_class != InferenceDeviceClass::Cpu || device.as_str() != "cpu" {
            return Err(invalid(
                "selected rerank currently supports explicit CPU only",
            ));
        }
        let runtime = decision.selected_runtime_variant_id.as_str();
        if !matches!(runtime, "llama_cpp.cpu" | "llamacpp.cpu") {
            return Err(invalid(
                "selected rerank requires the llama.cpp CPU variant",
            ));
        }
        let nested = &decision.device_decision;
        if crate::backend::canonical_backend_key(decision.selected_backend_id.as_str())
            != "llama_cpp"
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
pub(crate) mod tests {
    use super::*;
    pub(crate) fn fixture() -> (
        tempfile::TempDir,
        InferenceExecutionRequest,
        PumasArtifactLoadTarget,
        BackendExecutionDecision,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("synthetic.gguf");
        std::fs::write(&path, b"no model payload").unwrap();
        let mut package: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
            "../tests/fixtures/inference_package_facts/rerank_package_facts.json"
        ))
        .unwrap();
        package.model_ref.selected_artifact_path = None;
        package.model_ref.revision = Some("synthetic-r1".into());
        let model_ref = package.model_ref.clone();
        let target = PumasArtifactLoadTarget {
            model_ref: model_ref.clone(),
            artifact_kind: ModelArtifactKind::Gguf,
            local_load_path: path.to_str().unwrap().into(),
            load_path_kind: PumasArtifactLoadPathKind::File,
            library_root_id: Some("synthetic-root".into()),
            storage_kind: ModelStorageKind::LibraryOwned,
            validation_state: ModelValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(package.package_facts_contract_version),
        };
        let request = InferenceExecutionRequest {
            request_id: Some("rerank-execution".into()),
            task_id: InferenceTaskId::Rerank,
            model_ref: Some(model_ref.clone()),
            model_name: None,
            resolved_model_package_facts: Some(package),
            input: InferenceExecutionInput::Rerank {
                query: "search".into(),
                documents: vec!["a".into(), "b".into()],
                top_n: Some(1),
                return_documents: true,
            },
            generation_options: None,
            extra_options: serde_json::json!({}),
        };
        let variant = crate::RuntimeVariantId::parse("llama_cpp.cpu").unwrap();
        let device = InferenceDeviceId::parse("cpu").unwrap();
        let decision = BackendExecutionDecision {
            selected_backend_id: crate::BackendId::parse("llama_cpp").unwrap(),
            selected_runtime_variant_id: variant.clone(),
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device.clone()),
            device_decision: crate::DeviceResolutionDecision {
                policy: InferenceDevicePolicy::Auto,
                runtime_variant_id: variant,
                selected_device_class: InferenceDeviceClass::Cpu,
                selected_device_id: Some(device),
                diagnostics: vec![],
            },
            selected_task_id: Some(InferenceTaskId::Rerank),
            selected_model_ref: Some(model_ref),
            diagnostics: vec![],
            dependency_readiness: vec![],
            selection_policy_trace: None,
        };
        (directory, request, target, decision)
    }
}
