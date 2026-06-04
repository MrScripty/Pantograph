use pyo3::prelude::*;
use uuid::Uuid;

use super::pytorch_worker;
use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerFailure, PyTorchWorkerOperation, PyTorchWorkerResponse,
};
use super::pytorch_worker_image_contract::{
    validate_generate_image_envelope, PyTorchGenerateImageRequest, PyTorchGenerateImageResult,
};
use super::{task_join_error_message, PyTorchBackend};
use crate::backend::BackendError;
use crate::image_generation_planner::ImageGenerationExecutionPlan;
use crate::types::{EncodedImage, ImageGenerationResult};
use crate::{BackendExecutionContext, InferenceExecutionTelemetryRecorder};

impl PyTorchBackend {
    pub async fn generate_image_from_plan(
        &self,
        plan: ImageGenerationExecutionPlan,
        context: BackendExecutionContext,
    ) -> Result<ImageGenerationResult, BackendError> {
        if !self.ready {
            return Err(BackendError::NotReady);
        }
        reject_cancelled_image_generation(&context)?;

        let request_id = format!("pytorch-generate-image-{}", Uuid::new_v4().simple());
        let envelope = generate_image_envelope_from_plan(request_id.clone(), &plan)?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|error| {
            BackendError::Config(format!(
                "Failed to encode PyTorch worker generate_image envelope: {error}"
            ))
        })?;
        reject_cancelled_image_generation(&context)?;

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<ImageGenerationResult, BackendError> {
                let worker = pytorch_worker::worker_module(py).map_err(|error| {
                    image_worker_failure_from_message(
                        &request_id,
                        format!("Failed to get worker module: {error}"),
                    )
                })?;

                let response_json = worker
                    .call_method1("generate_image_from_envelope", (envelope_json,))
                    .map_err(|error| {
                        image_worker_failure_from_message(
                            &request_id,
                            format!("PyTorch worker generate_image envelope failed: {error}"),
                        )
                    })?
                    .extract::<String>()
                    .map_err(|error| {
                        image_worker_failure_from_message(
                            &request_id,
                            format!(
                                "PyTorch worker generate_image response was not JSON text: {error}"
                            ),
                        )
                    })?;
                image_generation_result_from_worker_response(
                    &request_id,
                    &response_json,
                    context.telemetry_recorder(),
                )
            })
        })
        .await
        .map_err(|error| BackendError::Inference(task_join_error_message(error)))?
    }
}

fn reject_cancelled_image_generation(
    context: &BackendExecutionContext,
) -> Result<(), BackendError> {
    context
        .cancellation_rejection_message("PyTorch image generation")
        .map(BackendError::Cancelled)
        .unwrap_or(Ok(()))
}

pub(super) fn generate_image_envelope_from_plan(
    request_id: impl Into<String>,
    plan: &ImageGenerationExecutionPlan,
) -> Result<PyTorchWorkerEnvelope<PyTorchGenerateImageRequest>, BackendError> {
    let envelope = PyTorchWorkerEnvelope::new(
        request_id,
        PyTorchWorkerOperation::GenerateImage,
        PyTorchGenerateImageRequest::from(plan),
    );
    validate_generate_image_envelope(&envelope)?;
    Ok(envelope)
}

pub(super) fn image_generation_result_from_worker_response(
    request_id: &str,
    response_json: &str,
    telemetry_recorder: &InferenceExecutionTelemetryRecorder,
) -> Result<ImageGenerationResult, BackendError> {
    let response: PyTorchWorkerResponse<PyTorchGenerateImageResult> =
        serde_json::from_str(response_json).map_err(|error| {
            image_worker_failure_from_message(
                request_id,
                format!("Failed to decode PyTorch worker generate_image response: {error}"),
            )
        })?;
    match response {
        PyTorchWorkerResponse::Ok(success) => {
            if success.request_id != request_id {
                return Err(image_worker_failure_from_message(
                    request_id,
                    format!(
                        "PyTorch worker generate_image response request_id mismatch: expected {request_id}, got {}",
                        success.request_id
                    ),
                ));
            }
            if success.result.images.is_empty() {
                return Err(image_worker_failure_from_message(
                    request_id,
                    "PyTorch worker generate_image response returned no images".to_string(),
                ));
            }
            record_worker_resource_observation(
                request_id,
                telemetry_recorder,
                success.resource_observation,
            );
            Ok(ImageGenerationResult {
                images: success
                    .result
                    .images
                    .into_iter()
                    .map(|image| EncodedImage {
                        data_base64: image.data_base64,
                        mime_type: image.mime_type,
                        width: image.width,
                        height: image.height,
                    })
                    .collect(),
                seed_used: success.result.seed_used,
                metadata: success.result.metadata,
            })
        }
        PyTorchWorkerResponse::Error(failure) => {
            image_worker_failure(request_id, telemetry_recorder, failure)
        }
    }
}

fn image_worker_failure(
    request_id: &str,
    telemetry_recorder: &InferenceExecutionTelemetryRecorder,
    failure: PyTorchWorkerFailure,
) -> Result<ImageGenerationResult, BackendError> {
    if failure.request_id != request_id {
        return Err(image_worker_failure_from_message(
            request_id,
            format!(
                "PyTorch worker generate_image response request_id mismatch: expected {request_id}, got {}",
                failure.request_id
            ),
        ));
    }
    record_worker_resource_observation(
        request_id,
        telemetry_recorder,
        failure.resource_observation.clone(),
    );
    Err(failure.into_backend_error())
}

fn record_worker_resource_observation(
    request_id: &str,
    telemetry_recorder: &InferenceExecutionTelemetryRecorder,
    resource_observation: Option<crate::InferenceExecutionResourceObservation>,
) {
    let Some(resource_observation) = resource_observation else {
        return;
    };
    if let Err(error) = telemetry_recorder.record_resource_observation(resource_observation) {
        log::warn!(
            "failed to record PyTorch worker image resource observation for {request_id}: {error}"
        );
    }
}

fn image_worker_failure_from_message(request_id: &str, message: String) -> BackendError {
    BackendError::Inference(format!(
        "PyTorch worker image generation failed for {request_id}: {message}"
    ))
}

#[cfg(test)]
#[path = "pytorch_image_generation_tests.rs"]
mod tests;
