use pyo3::prelude::*;
use uuid::Uuid;

use super::pytorch_worker;
use super::pytorch_worker_contract::{
    PyTorchWorkerEnvelope, PyTorchWorkerFailure, PyTorchWorkerOperation, PyTorchWorkerResponse,
};
use super::pytorch_worker_image_contract::{
    validate_generate_image_batch_envelope, validate_generate_image_envelope,
    PyTorchGenerateImageBatchMemberRequest, PyTorchGenerateImageBatchMemberResult,
    PyTorchGenerateImageBatchMemberStatus, PyTorchGenerateImageBatchRequest,
    PyTorchGenerateImageBatchResult, PyTorchGenerateImageRequest, PyTorchGenerateImageResult,
};
use super::{task_join_error_message, PyTorchBackend};
use crate::backend::BackendError;
use crate::image_generation_batch::{
    ImageGenerationBatchDiagnostic, ImageGenerationBatchDiagnosticCode,
    ImageGenerationBatchDiagnosticSeverity, ImageGenerationBatchExecutionMemberResponse,
    ImageGenerationBatchExecutionRequest, ImageGenerationBatchExecutionResponse,
    ImageGenerationBatchExecutionState, ImageGenerationBatchMemberExecutionState,
};
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

impl PyTorchBackend {
    pub async fn generate_image_batch_from_execution_request(
        &self,
        request: ImageGenerationBatchExecutionRequest,
        context: BackendExecutionContext,
    ) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
        if !self.ready {
            return Err(BackendError::NotReady);
        }
        reject_cancelled_image_generation(&context)?;
        if let Some(response) = reject_incompatible_pytorch_batch(&request) {
            return Ok(response);
        }

        let request_id = format!("pytorch-generate-image-batch-{}", Uuid::new_v4().simple());
        let envelope =
            generate_image_batch_envelope_from_execution_request(request_id.clone(), &request)?;
        let envelope_json = serde_json::to_string(&envelope).map_err(|error| {
            BackendError::Config(format!(
                "Failed to encode PyTorch worker generate_image_batch envelope: {error}"
            ))
        })?;
        reject_cancelled_image_generation(&context)?;

        let batch_execution_id = request.batch_execution_id.clone();
        tokio::task::spawn_blocking(move || {
            Python::with_gil(
                |py| -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
                    let worker = pytorch_worker::worker_module(py).map_err(|error| {
                        image_worker_failure_from_message(
                            &request_id,
                            format!("Failed to get worker module: {error}"),
                        )
                    })?;

                    let response_json = worker
                        .call_method1("generate_image_batch_from_envelope", (envelope_json,))
                        .map_err(|error| {
                            image_worker_failure_from_message(
                                &request_id,
                                format!(
                                    "PyTorch worker generate_image_batch envelope failed: {error}"
                                ),
                            )
                        })?
                        .extract::<String>()
                        .map_err(|error| {
                            image_worker_failure_from_message(
                                &request_id,
                                format!(
                                    "PyTorch worker generate_image_batch response was not JSON text: {error}"
                                ),
                            )
                        })?;
                    image_generation_batch_response_from_worker_response(
                        &request_id,
                        &batch_execution_id,
                        &response_json,
                        context.telemetry_recorder(),
                    )
                },
            )
        })
        .await
        .map_err(|error| BackendError::Inference(task_join_error_message(error)))?
    }
}

fn reject_cancelled_image_generation(
    context: &BackendExecutionContext,
) -> Result<(), BackendError> {
    match context.cancellation_rejection_message("PyTorch image generation") {
        Some(message) => Err(BackendError::Cancelled(message)),
        None => Ok(()),
    }
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

pub(super) fn generate_image_batch_envelope_from_execution_request(
    request_id: impl Into<String>,
    request: &ImageGenerationBatchExecutionRequest,
) -> Result<PyTorchWorkerEnvelope<PyTorchGenerateImageBatchRequest>, BackendError> {
    let envelope = PyTorchWorkerEnvelope::new(
        request_id,
        PyTorchWorkerOperation::GenerateImageBatch,
        PyTorchGenerateImageBatchRequest {
            batch_execution_id: request.batch_execution_id.clone(),
            anchor_member_id: request.anchor_member_id.clone(),
            members: request
                .members
                .iter()
                .map(|member| PyTorchGenerateImageBatchMemberRequest {
                    member_id: member.member_id.clone(),
                    request: PyTorchGenerateImageRequest::from(&member.plan),
                })
                .collect(),
        },
    );
    validate_generate_image_batch_envelope(&envelope)?;
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

pub(super) fn image_generation_batch_response_from_worker_response(
    request_id: &str,
    batch_execution_id: &str,
    response_json: &str,
    telemetry_recorder: &InferenceExecutionTelemetryRecorder,
) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
    let response: PyTorchWorkerResponse<PyTorchGenerateImageBatchResult> =
        serde_json::from_str(response_json).map_err(|error| {
            image_worker_failure_from_message(
                request_id,
                format!("Failed to decode PyTorch worker generate_image_batch response: {error}"),
            )
        })?;
    match response {
        PyTorchWorkerResponse::Ok(success) => {
            if success.request_id != request_id {
                return Err(image_worker_failure_from_message(
                    request_id,
                    format!(
                        "PyTorch worker generate_image_batch response request_id mismatch: expected {request_id}, got {}",
                        success.request_id
                    ),
                ));
            }
            if success.result.batch_execution_id != batch_execution_id {
                return Err(image_worker_failure_from_message(
                    request_id,
                    format!(
                        "PyTorch worker generate_image_batch response batch_execution_id mismatch: expected {batch_execution_id}, got {}",
                        success.result.batch_execution_id
                    ),
                ));
            }
            record_worker_resource_observation(
                request_id,
                telemetry_recorder,
                success.resource_observation,
            );
            Ok(batch_response_from_worker_result(success.result)?)
        }
        PyTorchWorkerResponse::Error(failure) => {
            batch_worker_failure(request_id, telemetry_recorder, failure)
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

fn batch_worker_failure(
    request_id: &str,
    telemetry_recorder: &InferenceExecutionTelemetryRecorder,
    failure: PyTorchWorkerFailure,
) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
    if failure.request_id != request_id {
        return Err(image_worker_failure_from_message(
            request_id,
            format!(
                "PyTorch worker generate_image_batch response request_id mismatch: expected {request_id}, got {}",
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

fn batch_response_from_worker_result(
    result: PyTorchGenerateImageBatchResult,
) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
    let members: Vec<_> = result
        .members
        .into_iter()
        .map(batch_member_response_from_worker_result)
        .collect::<Result<_, _>>()?;
    let state = batch_state_from_members(&members);
    Ok(ImageGenerationBatchExecutionResponse {
        batch_execution_id: result.batch_execution_id,
        state,
        members,
        diagnostics: Vec::new(),
    })
}

fn batch_member_response_from_worker_result(
    member: PyTorchGenerateImageBatchMemberResult,
) -> Result<ImageGenerationBatchExecutionMemberResponse, BackendError> {
    match member.status {
        PyTorchGenerateImageBatchMemberStatus::Succeeded => {
            let result = member.result.ok_or_else(|| {
                image_worker_failure_from_message(
                    &member.member_id,
                    "PyTorch worker generate_image_batch member succeeded without a result"
                        .to_string(),
                )
            })?;
            Ok(ImageGenerationBatchExecutionMemberResponse {
                member_id: member.member_id,
                state: ImageGenerationBatchMemberExecutionState::Completed,
                result: Some(ImageGenerationResult {
                    images: result
                        .images
                        .into_iter()
                        .map(|image| EncodedImage {
                            data_base64: image.data_base64,
                            mime_type: image.mime_type,
                            width: image.width,
                            height: image.height,
                        })
                        .collect(),
                    seed_used: result.seed_used,
                    metadata: result.metadata,
                }),
                diagnostics: Vec::new(),
            })
        }
        PyTorchGenerateImageBatchMemberStatus::Failed => Ok(failed_batch_member_response(
            member.member_id,
            ImageGenerationBatchMemberExecutionState::Failed,
            ImageGenerationBatchDiagnosticCode::MemberExecutionFailed,
            member.error,
        )),
        PyTorchGenerateImageBatchMemberStatus::Cancelled => Ok(failed_batch_member_response(
            member.member_id,
            ImageGenerationBatchMemberExecutionState::Cancelled,
            ImageGenerationBatchDiagnosticCode::MemberCancelled,
            member.error,
        )),
    }
}

fn failed_batch_member_response(
    member_id: String,
    state: ImageGenerationBatchMemberExecutionState,
    code: ImageGenerationBatchDiagnosticCode,
    error: Option<super::pytorch_worker_contract::PyTorchWorkerError>,
) -> ImageGenerationBatchExecutionMemberResponse {
    let message = error
        .map(|error| match error.canonical_code {
            Some(code) => format!("{code}: {}", error.message),
            None => error.message,
        })
        .unwrap_or_else(|| "PyTorch worker image batch member failed".to_string());
    ImageGenerationBatchExecutionMemberResponse {
        member_id: member_id.clone(),
        state,
        result: None,
        diagnostics: vec![ImageGenerationBatchDiagnostic {
            code,
            severity: ImageGenerationBatchDiagnosticSeverity::Error,
            member_id: Some(member_id),
            field_path: "worker.members".to_string(),
            message,
        }],
    }
}

fn batch_state_from_members(
    members: &[ImageGenerationBatchExecutionMemberResponse],
) -> ImageGenerationBatchExecutionState {
    let completed = members
        .iter()
        .filter(|member| member.state == ImageGenerationBatchMemberExecutionState::Completed)
        .count();
    if completed == members.len() {
        return ImageGenerationBatchExecutionState::Completed;
    }
    if completed > 0 {
        return ImageGenerationBatchExecutionState::PartiallyCompleted;
    }
    if members
        .iter()
        .all(|member| member.state == ImageGenerationBatchMemberExecutionState::Cancelled)
    {
        return ImageGenerationBatchExecutionState::Cancelled;
    }
    if members
        .iter()
        .all(|member| member.state == ImageGenerationBatchMemberExecutionState::Rejected)
    {
        return ImageGenerationBatchExecutionState::Rejected;
    }
    ImageGenerationBatchExecutionState::Failed
}

fn reject_incompatible_pytorch_batch(
    request: &ImageGenerationBatchExecutionRequest,
) -> Option<ImageGenerationBatchExecutionResponse> {
    let message = pytorch_batch_compatibility_error(request)?;
    let members = request
        .members
        .iter()
        .map(|member| ImageGenerationBatchExecutionMemberResponse {
            member_id: member.member_id.clone(),
            state: ImageGenerationBatchMemberExecutionState::Rejected,
            result: None,
            diagnostics: vec![ImageGenerationBatchDiagnostic {
                code: ImageGenerationBatchDiagnosticCode::BatchExecutionRejected,
                severity: ImageGenerationBatchDiagnosticSeverity::Error,
                member_id: Some(member.member_id.clone()),
                field_path: "members.plan".to_string(),
                message: message.clone(),
            }],
        })
        .collect();
    Some(ImageGenerationBatchExecutionResponse {
        batch_execution_id: request.batch_execution_id.clone(),
        state: ImageGenerationBatchExecutionState::Rejected,
        members,
        diagnostics: vec![ImageGenerationBatchDiagnostic {
            code: ImageGenerationBatchDiagnosticCode::BatchExecutionRejected,
            severity: ImageGenerationBatchDiagnosticSeverity::Error,
            member_id: None,
            field_path: "members".to_string(),
            message,
        }],
    })
}

fn pytorch_batch_compatibility_error(
    request: &ImageGenerationBatchExecutionRequest,
) -> Option<String> {
    let anchor = request.members.first()?.plan.clone();
    if request
        .members
        .iter()
        .any(|member| member.plan.num_images_per_prompt.unwrap_or(1) != 1)
    {
        return Some(
            "PyTorch image batch execution currently supports exactly one image per member"
                .to_string(),
        );
    }
    let has_seed = request
        .members
        .iter()
        .any(|member| member.plan.seed.is_some());
    let has_unseeded = request
        .members
        .iter()
        .any(|member| member.plan.seed.is_none());
    if has_seed && has_unseeded {
        return Some(
            "PyTorch image batch execution requires all members to either provide seeds or omit seeds"
                .to_string(),
        );
    }
    if request
        .members
        .iter()
        .any(|member| member.plan.denoising_scheduler.is_some())
    {
        return Some(
            "PyTorch image batch execution does not support explicit denoising_scheduler changes"
                .to_string(),
        );
    }
    for member in request.members.iter().skip(1) {
        let plan = &member.plan;
        if plan.model_ref != anchor.model_ref {
            return Some("PyTorch image batch members must use the same model_ref".to_string());
        }
        if plan.artifact_load_target != anchor.artifact_load_target {
            return Some(
                "PyTorch image batch members must use the same artifact_load_target".to_string(),
            );
        }
        if plan.family != anchor.family || plan.pipeline_class != anchor.pipeline_class {
            return Some(
                "PyTorch image batch members must use the same image family and pipeline"
                    .to_string(),
            );
        }
        if plan.required_components != anchor.required_components {
            return Some(
                "PyTorch image batch members must use the same required component roles"
                    .to_string(),
            );
        }
        if plan.selected_device_id != anchor.selected_device_id {
            return Some(
                "PyTorch image batch members must use the same selected device".to_string(),
            );
        }
        if plan.width != anchor.width
            || plan.height != anchor.height
            || plan.num_inference_steps != anchor.num_inference_steps
            || plan.guidance_scale != anchor.guidance_scale
            || plan.denoising_scheduler != anchor.denoising_scheduler
        {
            return Some(
                "PyTorch image batch members must use the same dimensions and generation settings"
                    .to_string(),
            );
        }
    }
    None
}

#[cfg(test)]
#[path = "pytorch_image_generation_tests.rs"]
mod tests;
