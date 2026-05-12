use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerOperation, PYTORCH_WORKER_CONTRACT_VERSION,
};
use crate::backend::BackendError;
use crate::device_contracts::InferenceDeviceId;
use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::model_contracts::{DiffusersComponentRole, ImageGenerationFamilyLabel, PumasModelRef};

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
    if envelope.payload.artifact_entry_path.trim().is_empty() {
        return Err(BackendError::Config(
            "PyTorch worker generate_image envelope requires an artifact_entry_path".to_string(),
        ));
    }
    if envelope.payload.pipeline_class.trim().is_empty() {
        return Err(BackendError::Config(
            "PyTorch worker generate_image envelope requires a pipeline_class".to_string(),
        ));
    }
    if envelope.payload.prompt.trim().is_empty() {
        return Err(BackendError::Config(
            "PyTorch worker generate_image envelope requires a prompt".to_string(),
        ));
    }
    if envelope.payload.required_components.is_empty() {
        return Err(BackendError::Config(
            "PyTorch worker generate_image envelope requires component roles".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(super) struct PyTorchGenerateImageRequest {
    pub model_ref: PumasModelRef,
    pub artifact_entry_path: String,
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
    pub scheduler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub num_images_per_prompt: Option<u32>,
}

impl From<&ImageGenerationExecutionPlan> for PyTorchGenerateImageRequest {
    fn from(plan: &ImageGenerationExecutionPlan) -> Self {
        Self {
            model_ref: plan.model_ref.clone(),
            artifact_entry_path: plan.artifact_entry_path.clone(),
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
            scheduler: plan.scheduler.clone(),
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
