//! Validation for scheduler-selected embedding execution.
//!
//! The runtime host owns the scheduler handoff and Pumas target. This module
//! keeps the executable-target and identity checks together so the gateway can
//! start the selected Pantograph embedding backend without importing Pumas
//! provider behavior or inferring a model path from metadata.

use std::path::Path;

use crate::backend::BackendError;
use crate::{
    BackendExecutionDecision, InferenceDeviceClass, InferenceDevicePolicy, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceTaskId, ModelArtifactKind, ModelStorageKind,
    ModelValidationState, PumasArtifactEntryPath, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef,
};

pub(crate) struct SelectedEmbeddingLoad<'a> {
    pub(crate) target: &'a PumasArtifactLoadTarget,
    pub(crate) device: &'a crate::InferenceDeviceId,
}

fn invalid(message: impl Into<String>) -> BackendError {
    BackendError::Config(format!("selected embedding: {}", message.into()))
}

fn same_model(left: &PumasModelRef, right: &PumasModelRef) -> bool {
    left.model_id.trim_start_matches("pumas://models/")
        == right.model_id.trim_start_matches("pumas://models/")
        && left.revision == right.revision
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
            || target.artifact_kind != ModelArtifactKind::Gguf
            || target.storage_kind == ModelStorageKind::Unknown
            || target.storage_kind != package.artifact.storage_kind
        {
            return Err(invalid(
                "a valid matching local GGUF embedding artifact is required",
            ));
        }
        if target.load_path_kind != PumasArtifactLoadPathKind::File
            || !Path::new(&target.local_load_path).is_absolute()
            || !tokio::fs::metadata(&target.local_load_path)
                .await
                .map_err(|error| invalid(format!("cannot inspect executable target: {error}")))?
                .is_file()
        {
            return Err(invalid(
                "Pumas executable embedding target must be an existing absolute file",
            ));
        }
        if package.custom_code.requires_custom_code {
            return Err(invalid("custom embedding code is denied"));
        }
        if decision.device_decision.runtime_variant_id != decision.selected_runtime_variant_id
            || decision.device_decision.selected_device_class != decision.selected_device_class
            || decision.device_decision.selected_device_id != decision.selected_device_id
        {
            return Err(invalid(
                "scheduler device decision does not match selected runtime/device",
            ));
        }
        let device = decision
            .selected_device_id
            .as_ref()
            .ok_or_else(|| invalid("concrete scheduler device is required"))?;
        let expected_runtime = match decision.selected_device_class {
            InferenceDeviceClass::Cpu if device.as_str() == "cpu" => "llama_cpp.cpu",
            InferenceDeviceClass::Cuda
                if device
                    .as_str()
                    .strip_prefix("cuda:")
                    .is_some_and(|index| index.parse::<u32>().is_ok()) =>
            {
                "llama_cpp.cuda"
            }
            InferenceDeviceClass::Metal if device.as_str() == "mps" => "llama_cpp.metal",
            _ => return Err(invalid("unsupported or ambiguous selected device")),
        };
        let nested = &decision.device_decision;
        if decision.selected_backend_id.as_str() != "llama_cpp"
            || decision.selected_runtime_variant_id.as_str() != expected_runtime
        {
            return Err(invalid(
                "scheduler embedding selection must use a matching llama_cpp runtime",
            ));
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
        Ok(Self { target, device })
    }
}
