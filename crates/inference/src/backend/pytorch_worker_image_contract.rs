use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerError, PyTorchWorkerOperation,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use crate::backend::BackendError;
use crate::device_contracts::InferenceDeviceId;
use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::model_contracts::{
    DiffusersComponentRole, ImageGenerationFamilyLabel, PumasArtifactLoadPathKind,
    PumasArtifactLoadTarget, PumasModelRef,
};

#[allow(dead_code)]
pub(super) fn validate_generate_image_envelope(
    envelope: &PyTorchWorkerEnvelope<PyTorchGenerateImageRequest>,
) -> Result<(), BackendError> {
    if envelope.contract_version != PYTORCH_WORKER_CONTRACT_VERSION {
        return Err(BackendError::Config(format!(
            "Unsupported PyTorch worker generate_image envelope contract version {}",
            envelope.contract_version
        )));
    }
    if envelope.operation != PyTorchWorkerOperation::GenerateImage {
        return Err(BackendError::Config(format!(
            "Unexpected PyTorch worker operation {:?} for image generation",
            envelope.operation
        )));
    }
    validate_generate_image_request_payload(
        &envelope.payload,
        "PyTorch worker generate_image envelope",
    )
}

#[allow(dead_code)]
pub(super) fn validate_generate_image_batch_envelope(
    envelope: &PyTorchWorkerEnvelope<PyTorchGenerateImageBatchRequest>,
) -> Result<(), BackendError> {
    if envelope.contract_version != PYTORCH_WORKER_CONTRACT_VERSION {
        return Err(BackendError::Config(format!(
            "Unsupported PyTorch worker generate_image_batch envelope contract version {}",
            envelope.contract_version
        )));
    }
    if envelope.operation != PyTorchWorkerOperation::GenerateImageBatch {
        return Err(BackendError::Config(format!(
            "Unexpected PyTorch worker operation {:?} for image generation batch",
            envelope.operation
        )));
    }

    validate_stable_batch_id(
        &envelope.payload.batch_execution_id,
        "PyTorch worker generate_image_batch envelope requires a batch_execution_id",
    )?;
    validate_stable_batch_id(
        &envelope.payload.anchor_member_id,
        "PyTorch worker generate_image_batch envelope requires an anchor_member_id",
    )?;
    if envelope.payload.members.is_empty() {
        return Err(BackendError::Config(
            "PyTorch worker generate_image_batch envelope requires members".to_string(),
        ));
    }

    let mut member_ids = BTreeSet::new();
    for member in &envelope.payload.members {
        validate_stable_batch_id(
            &member.member_id,
            "PyTorch worker generate_image_batch envelope requires member_id",
        )?;
        if !member_ids.insert(member.member_id.as_str()) {
            return Err(BackendError::Config(format!(
                "PyTorch worker generate_image_batch envelope contains duplicate member_id {}",
                member.member_id
            )));
        }
        validate_generate_image_request_payload(
            &member.request,
            &format!(
                "PyTorch worker generate_image_batch envelope member {}",
                member.member_id
            ),
        )?;
    }

    if !member_ids.contains(envelope.payload.anchor_member_id.as_str()) {
        return Err(BackendError::Config(format!(
            "PyTorch worker generate_image_batch envelope anchor_member_id {} must reference a member",
            envelope.payload.anchor_member_id
        )));
    }

    Ok(())
}

fn validate_generate_image_request_payload(
    payload: &PyTorchGenerateImageRequest,
    context: &str,
) -> Result<(), BackendError> {
    if payload.pipeline_class.trim().is_empty() {
        return Err(BackendError::Config(format!(
            "{context} requires a pipeline_class"
        )));
    }
    if payload.prompt.trim().is_empty() {
        return Err(BackendError::Config(format!("{context} requires a prompt")));
    }
    if payload.required_components.is_empty() {
        return Err(BackendError::Config(format!(
            "{context} requires component roles"
        )));
    }
    if payload.artifact_load_target.load_path_kind != PumasArtifactLoadPathKind::Directory {
        return Err(BackendError::Config(format!(
            "{context} requires a directory artifact_load_target"
        )));
    }
    if payload
        .artifact_load_target
        .local_load_path
        .trim()
        .is_empty()
    {
        return Err(BackendError::Config(format!(
            "{context} requires artifact_load_target.local_load_path"
        )));
    }
    Ok(())
}

fn validate_stable_batch_id(value: &str, message: &str) -> Result<(), BackendError> {
    if value.trim().is_empty() {
        return Err(BackendError::Config(message.to_string()));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageBatchRequest {
    pub batch_execution_id: String,
    pub anchor_member_id: String,
    pub members: Vec<PyTorchGenerateImageBatchMemberRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageBatchMemberRequest {
    pub member_id: String,
    pub request: PyTorchGenerateImageRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageRequest {
    pub model_ref: PumasModelRef,
    pub artifact_load_target: PumasArtifactLoadTarget,
    pub family: ImageGenerationFamilyLabel,
    pub pipeline_class: String,
    pub required_components: Vec<DiffusersComponentRole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<InferenceDeviceId>,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_inference_steps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denoising_scheduler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_images_per_prompt: Option<u32>,
}

impl From<&ImageGenerationExecutionPlan> for PyTorchGenerateImageRequest {
    fn from(plan: &ImageGenerationExecutionPlan) -> Self {
        Self {
            model_ref: plan.model_ref.clone(),
            artifact_load_target: plan.artifact_load_target.clone(),
            family: plan.family,
            pipeline_class: plan.pipeline_class.clone(),
            required_components: plan.required_components.clone(),
            device: plan.selected_device_id.clone(),
            prompt: plan.prompt.clone(),
            negative_prompt: plan.negative_prompt.clone(),
            width: plan.width,
            height: plan.height,
            num_inference_steps: plan.num_inference_steps,
            guidance_scale: plan.guidance_scale,
            seed: plan.seed,
            denoising_scheduler: plan.denoising_scheduler.as_ref().map(ToString::to_string),
            num_images_per_prompt: plan.num_images_per_prompt,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageResult {
    pub images: Vec<PyTorchGeneratedImage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed_used: Option<u64>,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageBatchResult {
    pub batch_execution_id: String,
    pub members: Vec<PyTorchGenerateImageBatchMemberResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageBatchMemberResult {
    pub member_id: String,
    pub status: PyTorchGenerateImageBatchMemberStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<PyTorchGenerateImageResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<PyTorchWorkerError>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum PyTorchGenerateImageBatchMemberStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGeneratedImage {
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
}
