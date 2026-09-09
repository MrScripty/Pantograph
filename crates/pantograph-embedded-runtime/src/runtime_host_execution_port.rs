use std::sync::Arc;

use async_trait::async_trait;
use inference::{
    ImageGenerationBatchDiagnostic, ImageGenerationBatchDiagnosticCode,
    ImageGenerationBatchDiagnosticSeverity, ImageGenerationBatchExecutionMemberRequest,
    ImageGenerationBatchExecutionRequest, ImageGenerationBatchExecutionResponse,
    ImageGenerationBatchMemberExecutionState, ImageGenerationPlanningOutcome,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionMemberResponse,
    RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
    RuntimeHostBatchExecutionRequest, RuntimeHostBatchExecutionResponse,
    RuntimeHostBatchExecutionState, RuntimeHostBatchMemberFailurePolicy,
    RuntimeHostBatchMemberReservationDisposition, RuntimeHostBatchMemberReservationPolicy,
    RuntimeHostBatchMemberRetryDisposition, RuntimeHostExecutionCancellationHandle,
    RuntimeHostExecutionCancellationSnapshot, RuntimeHostExecutionCancellationState,
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionOutput,
    RuntimeHostExecutionOutputValue, RuntimeHostExecutionPort, RuntimeHostExecutionPortError,
    RuntimeHostExecutionRequest, RuntimeHostExecutionResponse, RuntimeHostExecutionState,
    ValidatedRuntimeHostBatchExecutionRequest, ValidatedRuntimeHostExecutionRequest,
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
};

use crate::runtime_host_image_execution::{
    project_runtime_host_image_generation, RuntimeHostImageGenerationProjectionError,
};
use crate::runtime_host_load_target::{
    RuntimeHostLoadTargetResolver, RuntimeHostPumasLoadTargetError,
    RuntimeHostPumasLoadTargetResolver,
};
use crate::runtime_host_media_artifact_sink::{
    RuntimeHostImageArtifactWriteRequest, RuntimeHostMediaArtifactSink,
    RuntimeHostMediaArtifactSinkError,
};
use crate::runtime_host_package_facts::{
    RuntimeHostPackageFactsResolver, RuntimeHostPumasPackageFactsError,
};
use crate::runtime_host_text_execution::{
    project_runtime_host_text_generation, text_from_inference_result,
    validate_runtime_host_text_generation_request, RuntimeHostTextGenerationProjectionError,
    TEXT_GENERATION_TASK,
};

const MISSING_LOAD_TARGET_RESOLVER_HINT: &str =
    "embedded_runtime_host_execution_port.missing_load_target_resolver";
const LOAD_TARGET_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.pumas_load_target_unavailable";
const MISSING_MEDIA_ARTIFACT_SINK_HINT: &str =
    "embedded_runtime_host_execution_port.missing_media_artifact_sink";
const MISSING_PACKAGE_FACTS_RESOLVER_HINT: &str =
    "embedded_runtime_host_execution_port.missing_package_facts_resolver";
const MISSING_INFERENCE_GATEWAY_HINT: &str =
    "embedded_runtime_host_execution_port.missing_inference_gateway";
const PACKAGE_FACTS_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.pumas_package_facts_unavailable";
const IMAGE_PROJECTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.image_projection_failed";
const TEXT_PROJECTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.text_projection_failed";
const GATEWAY_EXECUTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.gateway_execution_failed";
const TEXT_GATEWAY_EXECUTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.text_gateway_execution_failed";
const BATCH_COMPATIBILITY_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.batch_compatibility_failed";
const BATCH_PLANNING_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.batch_planning_failed";
const BATCH_GATEWAY_EXECUTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.batch_gateway_execution_failed";
const MEDIA_ARTIFACT_WRITE_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.media_artifact_write_failed";
const RUNTIME_EXECUTION_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.runtime_execution_unavailable";
const CANCELLATION_REQUESTED_HINT: &str =
    "embedded_runtime_host_execution_port.cancellation_requested";
const SHUTDOWN_REQUESTED_HINT: &str = "embedded_runtime_host_execution_port.shutdown_requested";
const UNKNOWN_CANCELLATION_STATE_HINT: &str =
    "embedded_runtime_host_execution_port.unknown_cancellation_state";

pub(crate) struct EmbeddedRuntimeHostExecutionPort {
    load_target_resolver: Option<Arc<dyn RuntimeHostLoadTargetResolver>>,
    media_artifact_sink: Option<Arc<dyn RuntimeHostMediaArtifactSink>>,
    package_facts_resolver: Option<Arc<dyn RuntimeHostPackageFactsResolver>>,
    gateway: Option<Arc<inference::InferenceGateway>>,
}

impl EmbeddedRuntimeHostExecutionPort {
    #[must_use]
    pub(crate) fn fail_closed() -> Self {
        Self {
            load_target_resolver: None,
            media_artifact_sink: None,
            package_facts_resolver: None,
            gateway: None,
        }
    }

    #[must_use]
    pub(crate) fn with_load_target_resolver(
        load_target_resolver: RuntimeHostPumasLoadTargetResolver,
    ) -> Self {
        Self {
            load_target_resolver: Some(Arc::new(load_target_resolver)),
            media_artifact_sink: None,
            package_facts_resolver: None,
            gateway: None,
        }
    }

    #[must_use]
    pub(crate) fn with_runtime_dependencies(
        load_target_resolver: Arc<dyn RuntimeHostLoadTargetResolver>,
        package_facts_resolver: Arc<dyn RuntimeHostPackageFactsResolver>,
        media_artifact_sink: Arc<dyn RuntimeHostMediaArtifactSink>,
        gateway: Arc<inference::InferenceGateway>,
    ) -> Self {
        Self {
            load_target_resolver: Some(load_target_resolver),
            media_artifact_sink: Some(media_artifact_sink),
            package_facts_resolver: Some(package_facts_resolver),
            gateway: Some(gateway),
        }
    }

    #[cfg(test)]
    fn with_load_target_resolver_only_for_test(
        load_target_resolver: Arc<dyn RuntimeHostLoadTargetResolver>,
    ) -> Self {
        Self {
            load_target_resolver: Some(load_target_resolver),
            media_artifact_sink: None,
            package_facts_resolver: None,
            gateway: None,
        }
    }
}

#[async_trait]
impl RuntimeHostExecutionPort for EmbeddedRuntimeHostExecutionPort {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        let validated_request =
            ValidatedRuntimeHostExecutionRequest::try_from(request).map_err(|error| {
                RuntimeHostExecutionPortError::ExecutionFailed {
                    message: format!("embedded runtime-host request failed validation: {error}"),
                }
            })?;

        if let Some(response) =
            cancellation_rejection_response(validated_request.as_ref(), &cancellation)?
        {
            return Ok(response);
        }

        if validated_request
            .as_ref()
            .handoff
            .task_intent
            .task_type
            .as_str()
            == TEXT_GENERATION_TASK
        {
            return self
                .execute_runtime_host_text_request(&validated_request, cancellation)
                .await;
        }

        let Some(load_target_resolver) = self.load_target_resolver.as_ref() else {
            return Ok(rejected_response(
                validated_request.as_ref(),
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired,
                "embedded runtime-host execution requires a Pumas load-target resolver",
                MISSING_LOAD_TARGET_RESOLVER_HINT,
            ));
        };

        match load_target_resolver.resolve(&validated_request).await {
            Ok(load_target) => {
                if let Some(response) =
                    cancellation_rejection_response(validated_request.as_ref(), &cancellation)?
                {
                    return Ok(response);
                }
                let Some(_media_artifact_sink) = self.media_artifact_sink.as_ref() else {
                    return Ok(rejected_response(
                        validated_request.as_ref(),
                        RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                        "embedded runtime-host execution requires a media artifact sink before generated media can be returned",
                        MISSING_MEDIA_ARTIFACT_SINK_HINT,
                    ));
                };
                let Some(package_facts_resolver) = self.package_facts_resolver.as_ref() else {
                    return Ok(rejected_response(
                        validated_request.as_ref(),
                        RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                        "embedded runtime-host execution requires a Pumas package-facts resolver",
                        MISSING_PACKAGE_FACTS_RESOLVER_HINT,
                    ));
                };
                let Some(gateway) = self.gateway.as_ref() else {
                    return Ok(rejected_response(
                        validated_request.as_ref(),
                        RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                        "embedded runtime-host execution requires an inference gateway",
                        MISSING_INFERENCE_GATEWAY_HINT,
                    ));
                };
                let package_facts = match package_facts_resolver.resolve(&validated_request).await {
                    Ok(package_facts) => package_facts,
                    Err(error) => {
                        return Ok(rejected_response(
                            validated_request.as_ref(),
                            RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                            &package_facts_error_message(error),
                            PACKAGE_FACTS_UNAVAILABLE_HINT,
                        ));
                    }
                };
                if let Some(response) =
                    cancellation_rejection_response(validated_request.as_ref(), &cancellation)?
                {
                    return Ok(response);
                }
                let projection = match project_runtime_host_image_generation(
                    &validated_request,
                    package_facts,
                    load_target,
                ) {
                    Ok(projection) => projection,
                    Err(error) => {
                        return Ok(rejected_response(
                            validated_request.as_ref(),
                            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                            &image_projection_error_message(error),
                            IMAGE_PROJECTION_FAILED_HINT,
                        ));
                    }
                };
                if let Some(response) =
                    cancellation_rejection_response(validated_request.as_ref(), &cancellation)?
                {
                    return Ok(response);
                }
                let inference_cancellation =
                    inference_cancellation_handle_from_runtime_host(cancellation.clone());
                let result = match gateway
                    .generate_image_from_planning_input_with_cancellation(
                        projection.planning_input(),
                        inference_cancellation,
                    )
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
                        if matches!(
                            error,
                            inference::GatewayError::Backend(inference::BackendError::Cancelled(_))
                        ) {
                            if let Some(response) = cancellation_rejection_response(
                                validated_request.as_ref(),
                                &cancellation,
                            )? {
                                return Ok(response);
                            }
                        }
                        return Ok(failed_response(
                            validated_request.as_ref(),
                            &gateway_error_message(error),
                            GATEWAY_EXECUTION_FAILED_HINT,
                        ));
                    }
                };
                match completed_image_response(
                    validated_request.as_ref(),
                    result,
                    _media_artifact_sink.as_ref(),
                ) {
                    Ok(response) => Ok(response),
                    Err(error) => Ok(failed_response(
                        validated_request.as_ref(),
                        &media_artifact_sink_error_message(error),
                        MEDIA_ARTIFACT_WRITE_FAILED_HINT,
                    )),
                }
            }
            Err(error) => Ok(rejected_response(
                validated_request.as_ref(),
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                &load_target_error_message(error),
                LOAD_TARGET_UNAVAILABLE_HINT,
            )),
        }
    }
}

impl EmbeddedRuntimeHostExecutionPort {
    async fn execute_runtime_host_text_request(
        &self,
        request: &ValidatedRuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        let request_ref = request.as_ref();
        if let Err(error) = validate_runtime_host_text_generation_request(request_ref) {
            return Ok(rejected_response(
                request_ref,
                RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                &text_projection_error_message(error),
                TEXT_PROJECTION_FAILED_HINT,
            ));
        }

        let Some(load_target_resolver) = self.load_target_resolver.as_ref() else {
            return Ok(rejected_response(
                request_ref,
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired,
                "embedded runtime-host text execution requires a Pumas load-target resolver",
                MISSING_LOAD_TARGET_RESOLVER_HINT,
            ));
        };
        let Some(package_facts_resolver) = self.package_facts_resolver.as_ref() else {
            return Ok(rejected_response(
                request_ref,
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host text execution requires a Pumas package-facts resolver",
                MISSING_PACKAGE_FACTS_RESOLVER_HINT,
            ));
        };
        let Some(gateway) = self.gateway.as_ref() else {
            return Ok(rejected_response(
                request_ref,
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host text execution requires an inference gateway",
                MISSING_INFERENCE_GATEWAY_HINT,
            ));
        };

        let load_target = match load_target_resolver.resolve(request).await {
            Ok(load_target) => load_target,
            Err(error) => {
                return Ok(rejected_response(
                    request_ref,
                    RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                    &load_target_error_message(error),
                    LOAD_TARGET_UNAVAILABLE_HINT,
                ));
            }
        };
        if let Some(response) = cancellation_rejection_response(request_ref, &cancellation)? {
            return Ok(response);
        }

        let package_facts = match package_facts_resolver.resolve(request).await {
            Ok(package_facts) => package_facts,
            Err(error) => {
                return Ok(rejected_response(
                    request_ref,
                    RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                    &package_facts_error_message(error),
                    PACKAGE_FACTS_UNAVAILABLE_HINT,
                ));
            }
        };
        if let Some(response) = cancellation_rejection_response(request_ref, &cancellation)? {
            return Ok(response);
        }

        let projection =
            match project_runtime_host_text_generation(request, package_facts, load_target) {
                Ok(projection) => projection,
                Err(error) => {
                    return Ok(rejected_response(
                        request_ref,
                        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        &text_projection_error_message(error),
                        TEXT_PROJECTION_FAILED_HINT,
                    ));
                }
            };
        if let Some(response) = cancellation_rejection_response(request_ref, &cancellation)? {
            return Ok(response);
        }

        let inference_cancellation =
            inference_cancellation_handle_from_runtime_host(cancellation.clone());
        let result = match gateway
            .execute_selected_text_with_cancellation(
                projection.request().clone(),
                projection.artifact_load_target().clone(),
                projection.backend_decision().clone(),
                inference_cancellation,
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                if matches!(
                    error,
                    inference::GatewayError::Backend(inference::BackendError::Cancelled(_))
                ) {
                    if let Some(response) =
                        cancellation_rejection_response(request_ref, &cancellation)?
                    {
                        return Ok(response);
                    }
                }
                return Ok(failed_response(
                    request_ref,
                    &text_gateway_error_message(error),
                    TEXT_GATEWAY_EXECUTION_FAILED_HINT,
                ));
            }
        };

        let text = match text_from_inference_result(result) {
            Ok(text) => text,
            Err(error) => {
                return Ok(failed_response(
                    request_ref,
                    &text_projection_error_message(error),
                    TEXT_PROJECTION_FAILED_HINT,
                ));
            }
        };

        Ok(completed_text_response(request_ref, text))
    }
}

impl EmbeddedRuntimeHostExecutionPort {
    async fn execute_runtime_host_text_batch_request(
        &self,
        request: &RuntimeHostBatchExecutionRequest,
        member_requests: Vec<ValidatedRuntimeHostExecutionRequest>,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        let validation_errors = member_requests
            .iter()
            .map(|member| {
                validate_runtime_host_text_generation_request(member.as_ref())
                    .err()
                    .map(|error| error.to_string())
            })
            .collect::<Vec<_>>();
        let mut members = Vec::with_capacity(member_requests.len());

        for (member_request, validation_error) in member_requests.into_iter().zip(validation_errors)
        {
            let member = request
                .members
                .iter()
                .find(|candidate| {
                    candidate.execution_request_id == member_request.as_ref().execution_request_id
                })
                .expect("validated batch member must match its source request");
            if let Some(error) = validation_error {
                members.push(batch_member_response(
                    member,
                    RuntimeHostBatchExecutionMemberState::Rejected,
                    Vec::new(),
                    vec![runtime_host_diagnostic(
                        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        &format!("embedded runtime-host text projection failed: {error}"),
                        TEXT_PROJECTION_FAILED_HINT,
                    )],
                ));
                continue;
            }

            if let Some(response) =
                text_batch_member_cancellation_response(member, request, &cancellation)?
            {
                members.push(response);
                continue;
            }

            let response = self
                .execute_runtime_host_text_request(&member_request, cancellation.clone())
                .await?;
            members.push(text_batch_member_response(member, response));
        }

        let state = runtime_host_batch_state_from_members(&members);
        let mut diagnostics = members
            .iter()
            .filter_map(|member| member.diagnostics.first().cloned())
            .collect::<Vec<_>>();
        if diagnostics.is_empty()
            && matches!(
                state,
                RuntimeHostBatchExecutionState::Rejected | RuntimeHostBatchExecutionState::Failed
            )
        {
            diagnostics.push(runtime_host_diagnostic(
                RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                "embedded runtime-host text batch execution ended without a completed member",
                TEXT_GATEWAY_EXECUTION_FAILED_HINT,
            ));
        }

        Ok(RuntimeHostBatchExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: request.batch_execution_request_id.clone(),
            state,
            members,
            diagnostics,
        })
    }
}

#[async_trait]
impl RuntimeHostBatchExecutionPort for EmbeddedRuntimeHostExecutionPort {
    async fn execute_runtime_host_batch_request(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        let validated_request = ValidatedRuntimeHostBatchExecutionRequest::try_from(request)
            .map_err(|error| RuntimeHostExecutionPortError::ExecutionFailed {
                message: format!("embedded runtime-host batch request failed validation: {error}"),
            })?;
        let request = validated_request.as_ref();

        if let Some(response) = batch_cancellation_rejection_response(request, &cancellation)? {
            return Ok(response);
        }

        let member_requests = validated_member_requests_from_batch(request)?;
        if let Some(message) = shared_batch_runtime_context_error(&member_requests) {
            return Ok(rejected_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                &message,
                BATCH_COMPATIBILITY_FAILED_HINT,
            ));
        }
        if member_requests.iter().all(|member| {
            member.as_ref().handoff.task_intent.task_type.as_str() == TEXT_GENERATION_TASK
        }) {
            return self
                .execute_runtime_host_text_batch_request(request, member_requests, cancellation)
                .await;
        }

        let Some(load_target_resolver) = self.load_target_resolver.as_ref() else {
            return Ok(rejected_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired,
                "embedded runtime-host batch execution requires a Pumas load-target resolver",
                MISSING_LOAD_TARGET_RESOLVER_HINT,
            ));
        };
        let Some(media_artifact_sink) = self.media_artifact_sink.as_ref() else {
            return Ok(rejected_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host batch execution requires a media artifact sink before generated media can be returned",
                MISSING_MEDIA_ARTIFACT_SINK_HINT,
            ));
        };
        let Some(package_facts_resolver) = self.package_facts_resolver.as_ref() else {
            return Ok(rejected_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host batch execution requires a Pumas package-facts resolver",
                MISSING_PACKAGE_FACTS_RESOLVER_HINT,
            ));
        };
        let Some(gateway) = self.gateway.as_ref() else {
            return Ok(rejected_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable,
                "embedded runtime-host batch execution requires an inference gateway",
                MISSING_INFERENCE_GATEWAY_HINT,
            ));
        };

        let anchor_member_request = anchor_member_request(request, &member_requests)?;

        let load_target = match load_target_resolver.resolve(anchor_member_request).await {
            Ok(load_target) => load_target,
            Err(error) => {
                return Ok(rejected_batch_response(
                    request,
                    RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                    &load_target_error_message(error),
                    LOAD_TARGET_UNAVAILABLE_HINT,
                ));
            }
        };
        if let Some(response) = batch_cancellation_rejection_response(request, &cancellation)? {
            return Ok(response);
        }

        let package_facts = match package_facts_resolver.resolve(anchor_member_request).await {
            Ok(package_facts) => package_facts,
            Err(error) => {
                return Ok(rejected_batch_response(
                    request,
                    RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable,
                    &package_facts_error_message(error),
                    PACKAGE_FACTS_UNAVAILABLE_HINT,
                ));
            }
        };
        if let Some(response) = batch_cancellation_rejection_response(request, &cancellation)? {
            return Ok(response);
        }

        let mut inference_members = Vec::with_capacity(member_requests.len());
        for member_request in &member_requests {
            let projection = match project_runtime_host_image_generation(
                member_request,
                package_facts.clone(),
                load_target.clone(),
            ) {
                Ok(projection) => projection,
                Err(error) => {
                    return Ok(rejected_batch_member_response(
                        request,
                        member_request.as_ref().execution_request_id.as_str(),
                        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        &image_projection_error_message(error),
                        IMAGE_PROJECTION_FAILED_HINT,
                    ));
                }
            };
            let plan = match inference::plan_image_generation_execution(projection.planning_input())
            {
                ImageGenerationPlanningOutcome::Planned { plan } => plan,
                ImageGenerationPlanningOutcome::Rejected { diagnostics } => {
                    return Ok(rejected_batch_member_response(
                        request,
                        member_request.as_ref().execution_request_id.as_str(),
                        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        &image_planning_diagnostics_message(&diagnostics),
                        BATCH_PLANNING_FAILED_HINT,
                    ));
                }
            };
            inference_members.push(ImageGenerationBatchExecutionMemberRequest {
                member_id: member_request.as_ref().execution_request_id.clone(),
                request: projection.request().clone(),
                plan,
            });
        }
        if let Some(response) = batch_cancellation_rejection_response(request, &cancellation)? {
            return Ok(response);
        }

        let inference_request = ImageGenerationBatchExecutionRequest {
            batch_execution_id: request.batch_execution_request_id.clone(),
            anchor_member_id: request.anchor_execution_request_id.clone(),
            members: inference_members,
        };
        let inference_cancellation =
            inference_cancellation_handle_from_runtime_host(cancellation.clone());
        let inference_response = match gateway
            .generate_image_batch_from_execution_request_with_cancellation(
                inference_request,
                inference_cancellation,
            )
            .await
        {
            Ok(response) => response,
            Err(error) => {
                if matches!(
                    error,
                    inference::GatewayError::Backend(inference::BackendError::Cancelled(_))
                ) {
                    if let Some(response) =
                        batch_cancellation_rejection_response(request, &cancellation)?
                    {
                        return Ok(response);
                    }
                }
                return Ok(failed_batch_response(
                    request,
                    &batch_gateway_error_message(error),
                    BATCH_GATEWAY_EXECUTION_FAILED_HINT,
                ));
            }
        };

        Ok(runtime_host_batch_response_from_inference(
            request,
            inference_response,
            media_artifact_sink.as_ref(),
        ))
    }
}

#[derive(Clone)]
struct RuntimeHostInferenceCancellationSignal {
    cancellation: RuntimeHostExecutionCancellationHandle,
}

impl inference::InferenceExecutionCancellationSignal for RuntimeHostInferenceCancellationSignal {
    fn snapshot(&self) -> inference::InferenceExecutionCancellationSnapshot {
        let snapshot = self.cancellation.snapshot();
        let reason = snapshot.reason;
        match snapshot.state {
            RuntimeHostExecutionCancellationState::Running => {
                inference::InferenceExecutionCancellationSnapshot::running()
            }
            RuntimeHostExecutionCancellationState::CancellationRequested => {
                inference::InferenceExecutionCancellationSnapshot::cancellation_requested(reason)
            }
            RuntimeHostExecutionCancellationState::ShutdownRequested => {
                inference::InferenceExecutionCancellationSnapshot::shutdown_requested(reason)
            }
            _ => inference::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "runtime-host cancellation signal entered an unknown state".to_string(),
            )),
        }
    }
}

fn inference_cancellation_handle_from_runtime_host(
    cancellation: RuntimeHostExecutionCancellationHandle,
) -> inference::InferenceExecutionCancellationHandle {
    inference::InferenceExecutionCancellationHandle::with_signal(Arc::new(
        RuntimeHostInferenceCancellationSignal { cancellation },
    ))
}

fn validated_member_requests_from_batch(
    request: &RuntimeHostBatchExecutionRequest,
) -> Result<Vec<ValidatedRuntimeHostExecutionRequest>, RuntimeHostExecutionPortError> {
    request
        .members
        .iter()
        .map(|member| {
            ValidatedRuntimeHostExecutionRequest::try_from(RuntimeHostExecutionRequest {
                contract_version: request.contract_version,
                execution_request_id: member.execution_request_id.clone(),
                cancellation_context: request.cancellation_context.clone(),
                handoff: member.handoff.clone(),
                materialized_inputs: member.materialized_inputs.clone(),
            })
            .map_err(|error| RuntimeHostExecutionPortError::ExecutionFailed {
                message: format!(
                    "embedded runtime-host batch member '{}' failed single-request validation: {error}",
                    member.execution_request_id
                ),
            })
        })
        .collect()
}

fn shared_batch_runtime_context_error(
    member_requests: &[ValidatedRuntimeHostExecutionRequest],
) -> Option<String> {
    let first = member_requests.first()?.as_ref();
    let first_decision = match first.handoff.dispatch_decision.as_ref() {
        Some(decision) => decision,
        None => {
            return Some(format!(
                "embedded runtime-host batch member '{}' is missing a scheduler dispatch decision",
                first.execution_request_id
            ));
        }
    };
    if !matches!(
        first_decision.task_intent.task_type.as_str(),
        "image_generation" | TEXT_GENERATION_TASK
    ) {
        return Some(format!(
            "embedded runtime-host batch task type '{}' is unsupported",
            first_decision.task_intent.task_type
        ));
    }

    for member_request in &member_requests[1..] {
        let member = member_request.as_ref();
        let Some(decision) = member.handoff.dispatch_decision.as_ref() else {
            return Some(format!(
                "embedded runtime-host batch member '{}' is missing a scheduler dispatch decision",
                member.execution_request_id
            ));
        };
        if decision.task_intent.task_type != first_decision.task_intent.task_type {
            return Some(format!(
                "embedded runtime-host batch members must share task type; member '{}' selected '{}'",
                member.execution_request_id, decision.task_intent.task_type
            ));
        }
        if decision.selected_model_ref != first_decision.selected_model_ref {
            return Some(format!(
                "embedded runtime-host batch members must share selected model ref; member '{}' selected '{}'",
                member.execution_request_id, decision.selected_model_ref.model_id
            ));
        }
        if decision.selected_runtime_id != first_decision.selected_runtime_id {
            return Some(format!(
                "embedded runtime-host batch members must share selected runtime; member '{}' selected '{}'",
                member.execution_request_id, decision.selected_runtime_id
            ));
        }
        if decision.selected_runtime_variant_id != first_decision.selected_runtime_variant_id {
            return Some(format!(
                "embedded runtime-host batch members must share selected runtime variant; member '{}' selected {:?}",
                member.execution_request_id, decision.selected_runtime_variant_id
            ));
        }
        if decision.selected_device_ids != first_decision.selected_device_ids {
            return Some(format!(
                "embedded runtime-host batch members must share selected device set; member '{}' selected {:?}",
                member.execution_request_id, decision.selected_device_ids
            ));
        }
    }

    None
}

fn anchor_member_request<'a>(
    request: &RuntimeHostBatchExecutionRequest,
    member_requests: &'a [ValidatedRuntimeHostExecutionRequest],
) -> Result<&'a ValidatedRuntimeHostExecutionRequest, RuntimeHostExecutionPortError> {
    member_requests
        .iter()
        .find(|member| member.as_ref().execution_request_id == request.anchor_execution_request_id)
        .ok_or_else(|| RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host batch anchor '{}' was not present after validation",
                request.anchor_execution_request_id
            ),
        })
}

fn batch_cancellation_rejection_response(
    request: &RuntimeHostBatchExecutionRequest,
    cancellation: &RuntimeHostExecutionCancellationHandle,
) -> Result<Option<RuntimeHostBatchExecutionResponse>, RuntimeHostExecutionPortError> {
    let snapshot = cancellation.snapshot();
    snapshot
        .validate()
        .map_err(|error| RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host batch cancellation snapshot failed validation: {error}"
            ),
        })?;
    if snapshot.cancellation_context_id != request.cancellation_context.cancellation_context_id {
        return Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host batch cancellation context mismatch: request '{}' but signal '{}'",
                request.cancellation_context.cancellation_context_id,
                snapshot.cancellation_context_id
            ),
        });
    }

    Ok(batch_cancellation_rejection_from_snapshot(
        request, &snapshot,
    ))
}

fn batch_cancellation_rejection_from_snapshot(
    request: &RuntimeHostBatchExecutionRequest,
    snapshot: &RuntimeHostExecutionCancellationSnapshot,
) -> Option<RuntimeHostBatchExecutionResponse> {
    let reason = snapshot.reason.as_deref().unwrap_or("no reason provided");
    match snapshot.state {
        RuntimeHostExecutionCancellationState::Running => None,
        RuntimeHostExecutionCancellationState::CancellationRequested => Some(
            cancelled_batch_response(
                request,
                RuntimeHostExecutionDiagnosticCode::CancellationRequested,
                &format!(
                    "embedded runtime-host batch execution cancelled before completion: {reason}"
                ),
                CANCELLATION_REQUESTED_HINT,
            ),
        ),
        RuntimeHostExecutionCancellationState::ShutdownRequested => Some(cancelled_batch_response(
            request,
            RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
            &format!(
                "embedded runtime-host batch execution stopped for workflow-service shutdown: {reason}"
            ),
            SHUTDOWN_REQUESTED_HINT,
        )),
        _ => Some(rejected_batch_response(
            request,
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "embedded runtime-host batch execution rejected an unknown cancellation state",
            UNKNOWN_CANCELLATION_STATE_HINT,
        )),
    }
}

fn rejected_batch_response(
    request: &RuntimeHostBatchExecutionRequest,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostBatchExecutionResponse {
    terminal_batch_response_for_all_members(
        request,
        RuntimeHostBatchExecutionState::Rejected,
        RuntimeHostBatchExecutionMemberState::Rejected,
        diagnostic_code,
        message,
        hint,
    )
}

fn rejected_batch_member_response(
    request: &RuntimeHostBatchExecutionRequest,
    rejected_execution_request_id: &str,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostBatchExecutionResponse {
    let top_diagnostic = runtime_host_diagnostic(diagnostic_code.clone(), message, hint);
    let members = request
        .members
        .iter()
        .map(|member| {
            let member_message = if member.execution_request_id == rejected_execution_request_id {
                message.to_string()
            } else {
                format!(
                    "embedded runtime-host batch execution rejected because member '{}' could not be planned",
                    rejected_execution_request_id
                )
            };
            batch_member_response(
                member,
                RuntimeHostBatchExecutionMemberState::Rejected,
                Vec::new(),
                vec![runtime_host_diagnostic(
                    diagnostic_code.clone(),
                    &member_message,
                    hint,
                )],
            )
        })
        .collect();

    RuntimeHostBatchExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: request.batch_execution_request_id.clone(),
        state: RuntimeHostBatchExecutionState::Rejected,
        members,
        diagnostics: vec![top_diagnostic],
    }
}

fn cancelled_batch_response(
    request: &RuntimeHostBatchExecutionRequest,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostBatchExecutionResponse {
    terminal_batch_response_for_all_members(
        request,
        RuntimeHostBatchExecutionState::Cancelled,
        RuntimeHostBatchExecutionMemberState::Cancelled,
        diagnostic_code,
        message,
        hint,
    )
}

fn failed_batch_response(
    request: &RuntimeHostBatchExecutionRequest,
    message: &str,
    hint: &str,
) -> RuntimeHostBatchExecutionResponse {
    terminal_batch_response_for_all_members(
        request,
        RuntimeHostBatchExecutionState::Failed,
        RuntimeHostBatchExecutionMemberState::Failed,
        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
        message,
        hint,
    )
}

fn terminal_batch_response_for_all_members(
    request: &RuntimeHostBatchExecutionRequest,
    batch_state: RuntimeHostBatchExecutionState,
    member_state: RuntimeHostBatchExecutionMemberState,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostBatchExecutionResponse {
    let diagnostic = runtime_host_diagnostic(diagnostic_code, message, hint);
    RuntimeHostBatchExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: request.batch_execution_request_id.clone(),
        state: batch_state,
        members: request
            .members
            .iter()
            .map(|member| {
                batch_member_response(
                    member,
                    member_state.clone(),
                    Vec::new(),
                    vec![diagnostic.clone()],
                )
            })
            .collect(),
        diagnostics: vec![diagnostic],
    }
}

fn runtime_host_batch_response_from_inference(
    request: &RuntimeHostBatchExecutionRequest,
    response: ImageGenerationBatchExecutionResponse,
    media_artifact_sink: &dyn RuntimeHostMediaArtifactSink,
) -> RuntimeHostBatchExecutionResponse {
    let mut members = Vec::with_capacity(request.members.len());
    let mut diagnostics = runtime_host_diagnostics_from_image_batch(&response.diagnostics);

    for member_request in &request.members {
        let Some(member_response) = response
            .members
            .iter()
            .find(|member| member.member_id == member_request.execution_request_id)
        else {
            let diagnostic = runtime_host_diagnostic(
                RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                &format!(
                    "embedded runtime-host batch gateway response omitted member '{}'",
                    member_request.execution_request_id
                ),
                BATCH_GATEWAY_EXECUTION_FAILED_HINT,
            );
            diagnostics.push(diagnostic.clone());
            members.push(batch_member_response(
                member_request,
                RuntimeHostBatchExecutionMemberState::Failed,
                Vec::new(),
                vec![diagnostic],
            ));
            continue;
        };

        match member_response.state {
            ImageGenerationBatchMemberExecutionState::Completed => {
                let Some(result) = member_response.result.clone() else {
                    let diagnostic = runtime_host_diagnostic(
                        RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                        "embedded runtime-host batch gateway completed a member without an image result",
                        BATCH_GATEWAY_EXECUTION_FAILED_HINT,
                    );
                    diagnostics.push(diagnostic.clone());
                    members.push(batch_member_response(
                        member_request,
                        RuntimeHostBatchExecutionMemberState::Failed,
                        Vec::new(),
                        vec![diagnostic],
                    ));
                    continue;
                };
                match completed_image_batch_member_response(
                    member_request,
                    result,
                    media_artifact_sink,
                ) {
                    Ok(member) => members.push(member),
                    Err(error) => {
                        let diagnostic = runtime_host_diagnostic(
                            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                            &media_artifact_sink_error_message(error),
                            MEDIA_ARTIFACT_WRITE_FAILED_HINT,
                        );
                        diagnostics.push(diagnostic.clone());
                        members.push(batch_member_response(
                            member_request,
                            RuntimeHostBatchExecutionMemberState::Failed,
                            Vec::new(),
                            vec![diagnostic],
                        ));
                    }
                }
            }
            ImageGenerationBatchMemberExecutionState::Rejected => {
                members.push(batch_member_response(
                    member_request,
                    RuntimeHostBatchExecutionMemberState::Rejected,
                    Vec::new(),
                    runtime_host_diagnostics_from_image_batch(&member_response.diagnostics),
                ));
            }
            ImageGenerationBatchMemberExecutionState::Failed => {
                members.push(batch_member_response(
                    member_request,
                    RuntimeHostBatchExecutionMemberState::Failed,
                    Vec::new(),
                    runtime_host_diagnostics_from_image_batch(&member_response.diagnostics),
                ));
            }
            ImageGenerationBatchMemberExecutionState::Cancelled => {
                members.push(batch_member_response(
                    member_request,
                    RuntimeHostBatchExecutionMemberState::Cancelled,
                    Vec::new(),
                    runtime_host_diagnostics_from_image_batch(&member_response.diagnostics),
                ));
            }
            ImageGenerationBatchMemberExecutionState::Accepted
            | ImageGenerationBatchMemberExecutionState::Running => {
                let diagnostic = runtime_host_diagnostic(
                    RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                    "embedded runtime-host batch gateway returned a non-terminal member state after awaited execution",
                    BATCH_GATEWAY_EXECUTION_FAILED_HINT,
                );
                diagnostics.push(diagnostic.clone());
                members.push(batch_member_response(
                    member_request,
                    RuntimeHostBatchExecutionMemberState::Failed,
                    Vec::new(),
                    vec![diagnostic],
                ));
            }
            _ => {
                let diagnostic = runtime_host_diagnostic(
                    RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
                    "embedded runtime-host batch gateway returned an unknown member state",
                    BATCH_GATEWAY_EXECUTION_FAILED_HINT,
                );
                diagnostics.push(diagnostic.clone());
                members.push(batch_member_response(
                    member_request,
                    RuntimeHostBatchExecutionMemberState::Failed,
                    Vec::new(),
                    vec![diagnostic],
                ));
            }
        }
    }

    let state = runtime_host_batch_state_from_members(&members);
    if matches!(
        state,
        RuntimeHostBatchExecutionState::Rejected | RuntimeHostBatchExecutionState::Failed
    ) && diagnostics.is_empty()
    {
        diagnostics.push(runtime_host_diagnostic(
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "embedded runtime-host batch execution ended without a completed member",
            BATCH_GATEWAY_EXECUTION_FAILED_HINT,
        ));
    }

    RuntimeHostBatchExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: request.batch_execution_request_id.clone(),
        state,
        members,
        diagnostics,
    }
}

fn completed_image_batch_member_response(
    member: &RuntimeHostBatchExecutionMemberRequest,
    result: inference::ImageGenerationResult,
    media_artifact_sink: &dyn RuntimeHostMediaArtifactSink,
) -> Result<RuntimeHostBatchExecutionMemberResponse, RuntimeHostMediaArtifactSinkError> {
    let dispatch_decision = member.handoff.dispatch_decision.as_ref();
    let model_id = dispatch_decision.map(|decision| decision.selected_model_ref.model_id.as_str());
    let runtime_id = dispatch_decision.map(|decision| decision.selected_runtime_id.as_str());
    let mut outputs = Vec::with_capacity(result.images.len());
    for (image_index, image) in result.images.iter().enumerate() {
        let artifact_ref =
            media_artifact_sink.write_image_output(RuntimeHostImageArtifactWriteRequest {
                workflow_run_id: member.handoff.workflow_run_id.as_str(),
                workflow_id: member.handoff.workflow_id.as_str(),
                node_id: member.handoff.node_id.as_str(),
                task_id: member.handoff.task_id.as_str(),
                port_id: "image",
                image_index,
                image,
                model_id,
                runtime_id,
            })?;
        outputs.push(RuntimeHostExecutionOutput {
            port_id: "image".to_string(),
            value: RuntimeHostExecutionOutputValue::MediaArtifactRef(artifact_ref),
        });
    }

    Ok(batch_member_response(
        member,
        RuntimeHostBatchExecutionMemberState::Completed,
        outputs,
        vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Info,
            code: RuntimeHostExecutionDiagnosticCode::ExecutionCompleted,
            message: "embedded runtime-host image batch member completed".to_string(),
            hint: None,
        }],
    ))
}

fn batch_member_response(
    member: &RuntimeHostBatchExecutionMemberRequest,
    state: RuntimeHostBatchExecutionMemberState,
    outputs: Vec<RuntimeHostExecutionOutput>,
    diagnostics: Vec<RuntimeHostExecutionDiagnostic>,
) -> RuntimeHostBatchExecutionMemberResponse {
    RuntimeHostBatchExecutionMemberResponse {
        execution_request_id: member.execution_request_id.clone(),
        assignment_id: member.assignment_id.clone(),
        workflow_id: member.handoff.workflow_id.clone(),
        workflow_run_id: member.handoff.workflow_run_id.clone(),
        node_id: member.handoff.node_id.clone(),
        task_id: member.handoff.task_id.clone(),
        retry_disposition: retry_disposition_for_member(member, &state),
        reservation_disposition: reservation_disposition_for_member(member),
        state,
        outputs,
        diagnostics,
        terminal_metadata: None,
    }
}

fn text_batch_member_response(
    member: &RuntimeHostBatchExecutionMemberRequest,
    response: RuntimeHostExecutionResponse,
) -> RuntimeHostBatchExecutionMemberResponse {
    let (state, outputs, mut diagnostics) = match response.state {
        RuntimeHostExecutionState::Completed => (
            RuntimeHostBatchExecutionMemberState::Completed,
            response.outputs,
            response.diagnostics,
        ),
        RuntimeHostExecutionState::Rejected
            if response.diagnostics.iter().any(|diagnostic| {
                matches!(
                    diagnostic.code,
                    RuntimeHostExecutionDiagnosticCode::CancellationRequested
                        | RuntimeHostExecutionDiagnosticCode::ShutdownRequested
                )
            }) =>
        {
            (
                RuntimeHostBatchExecutionMemberState::Cancelled,
                Vec::new(),
                response.diagnostics,
            )
        }
        RuntimeHostExecutionState::Rejected => (
            RuntimeHostBatchExecutionMemberState::Rejected,
            Vec::new(),
            response.diagnostics,
        ),
        RuntimeHostExecutionState::Failed => (
            RuntimeHostBatchExecutionMemberState::Failed,
            Vec::new(),
            response.diagnostics,
        ),
        RuntimeHostExecutionState::Accepted => (
            RuntimeHostBatchExecutionMemberState::Failed,
            Vec::new(),
            response.diagnostics,
        ),
        _ => (
            RuntimeHostBatchExecutionMemberState::Failed,
            Vec::new(),
            response.diagnostics,
        ),
    };
    if diagnostics.is_empty() {
        diagnostics.push(runtime_host_diagnostic(
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "embedded runtime-host text batch member returned no terminal diagnostic",
            TEXT_GATEWAY_EXECUTION_FAILED_HINT,
        ));
    }
    batch_member_response(member, state, outputs, diagnostics)
}

fn text_batch_member_cancellation_response(
    member: &RuntimeHostBatchExecutionMemberRequest,
    request: &RuntimeHostBatchExecutionRequest,
    cancellation: &RuntimeHostExecutionCancellationHandle,
) -> Result<Option<RuntimeHostBatchExecutionMemberResponse>, RuntimeHostExecutionPortError> {
    let snapshot = cancellation.snapshot();
    snapshot
        .validate()
        .map_err(|error| RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host batch cancellation snapshot failed validation: {error}"
            ),
        })?;
    if snapshot.cancellation_context_id != request.cancellation_context.cancellation_context_id {
        return Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host batch cancellation context mismatch: request '{}' but signal '{}'",
                request.cancellation_context.cancellation_context_id,
                snapshot.cancellation_context_id
            ),
        });
    }
    let reason = snapshot.reason.as_deref().unwrap_or("no reason provided");
    let (state, diagnostic_code, message, hint) = match snapshot.state {
        RuntimeHostExecutionCancellationState::Running => return Ok(None),
        RuntimeHostExecutionCancellationState::CancellationRequested => (
            RuntimeHostBatchExecutionMemberState::Cancelled,
            RuntimeHostExecutionDiagnosticCode::CancellationRequested,
            format!(
                "embedded runtime-host text batch member cancelled before completion: {reason}"
            ),
            CANCELLATION_REQUESTED_HINT,
        ),
        RuntimeHostExecutionCancellationState::ShutdownRequested => (
            RuntimeHostBatchExecutionMemberState::Cancelled,
            RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
            format!(
                "embedded runtime-host text batch member stopped for workflow-service shutdown: {reason}"
            ),
            SHUTDOWN_REQUESTED_HINT,
        ),
        _ => (
            RuntimeHostBatchExecutionMemberState::Failed,
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "embedded runtime-host text batch member observed an unknown cancellation state"
                .to_string(),
            UNKNOWN_CANCELLATION_STATE_HINT,
        ),
    };
    Ok(Some(batch_member_response(
        member,
        state,
        Vec::new(),
        vec![runtime_host_diagnostic(diagnostic_code, &message, hint)],
    )))
}

fn retry_disposition_for_member(
    member: &RuntimeHostBatchExecutionMemberRequest,
    state: &RuntimeHostBatchExecutionMemberState,
) -> RuntimeHostBatchMemberRetryDisposition {
    if matches!(state, RuntimeHostBatchExecutionMemberState::Completed) {
        return RuntimeHostBatchMemberRetryDisposition::NotRetryable;
    }
    match &member.failure_policy {
        RuntimeHostBatchMemberFailurePolicy::TerminalOnly => {
            RuntimeHostBatchMemberRetryDisposition::NotRetryable
        }
        RuntimeHostBatchMemberFailurePolicy::Retryable => {
            RuntimeHostBatchMemberRetryDisposition::Retryable
        }
        RuntimeHostBatchMemberFailurePolicy::Deferrable => {
            RuntimeHostBatchMemberRetryDisposition::Deferred
        }
        _ => RuntimeHostBatchMemberRetryDisposition::NotRetryable,
    }
}

fn reservation_disposition_for_member(
    member: &RuntimeHostBatchExecutionMemberRequest,
) -> RuntimeHostBatchMemberReservationDisposition {
    match &member.reservation_policy {
        RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal => {
            RuntimeHostBatchMemberReservationDisposition::Released
        }
        RuntimeHostBatchMemberReservationPolicy::RetainForRuntimeReuse => {
            RuntimeHostBatchMemberReservationDisposition::RetainedForRuntimeReuse
        }
        RuntimeHostBatchMemberReservationPolicy::DeferToScheduler => {
            RuntimeHostBatchMemberReservationDisposition::DeferredToScheduler
        }
        _ => RuntimeHostBatchMemberReservationDisposition::DeferredToScheduler,
    }
}

fn runtime_host_batch_state_from_members(
    members: &[RuntimeHostBatchExecutionMemberResponse],
) -> RuntimeHostBatchExecutionState {
    if members
        .iter()
        .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Completed)
    {
        return RuntimeHostBatchExecutionState::Completed;
    }
    if members
        .iter()
        .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Cancelled)
    {
        return RuntimeHostBatchExecutionState::Cancelled;
    }
    if members
        .iter()
        .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Rejected)
    {
        return RuntimeHostBatchExecutionState::Rejected;
    }
    if members
        .iter()
        .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Deferred)
    {
        return RuntimeHostBatchExecutionState::Deferred;
    }
    if members
        .iter()
        .any(|member| member.state == RuntimeHostBatchExecutionMemberState::Completed)
    {
        return RuntimeHostBatchExecutionState::PartiallyCompleted;
    }
    RuntimeHostBatchExecutionState::Failed
}

fn runtime_host_diagnostics_from_image_batch(
    diagnostics: &[ImageGenerationBatchDiagnostic],
) -> Vec<RuntimeHostExecutionDiagnostic> {
    diagnostics
        .iter()
        .map(runtime_host_diagnostic_from_image_batch)
        .collect()
}

fn runtime_host_diagnostic_from_image_batch(
    diagnostic: &ImageGenerationBatchDiagnostic,
) -> RuntimeHostExecutionDiagnostic {
    RuntimeHostExecutionDiagnostic {
        severity: match diagnostic.severity {
            ImageGenerationBatchDiagnosticSeverity::Info => {
                RuntimeHostExecutionDiagnosticSeverity::Info
            }
            ImageGenerationBatchDiagnosticSeverity::Warning => {
                RuntimeHostExecutionDiagnosticSeverity::Warning
            }
            ImageGenerationBatchDiagnosticSeverity::Error => {
                RuntimeHostExecutionDiagnosticSeverity::Error
            }
            _ => RuntimeHostExecutionDiagnosticSeverity::Error,
        },
        code: match diagnostic.code {
            ImageGenerationBatchDiagnosticCode::MemberCancelled => {
                RuntimeHostExecutionDiagnosticCode::CancellationRequested
            }
            ImageGenerationBatchDiagnosticCode::UnsupportedBatchExecution
            | ImageGenerationBatchDiagnosticCode::BatchPlanningRejected
            | ImageGenerationBatchDiagnosticCode::BatchExecutionRejected
            | ImageGenerationBatchDiagnosticCode::MemberPlanningRejected
            | ImageGenerationBatchDiagnosticCode::MemberExecutionFailed
            | ImageGenerationBatchDiagnosticCode::ContractViolation => {
                RuntimeHostExecutionDiagnosticCode::ExecutionFailed
            }
            _ => RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
        },
        message: format!(
            "image-generation batch diagnostic at {}: {}",
            diagnostic.field_path, diagnostic.message
        ),
        hint: Some(BATCH_GATEWAY_EXECUTION_FAILED_HINT.to_string()),
    }
}

fn runtime_host_diagnostic(
    code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostExecutionDiagnostic {
    RuntimeHostExecutionDiagnostic {
        severity: RuntimeHostExecutionDiagnosticSeverity::Error,
        code,
        message: message.to_string(),
        hint: Some(hint.to_string()),
    }
}

fn rejected_response(
    request: &RuntimeHostExecutionRequest,
    diagnostic_code: RuntimeHostExecutionDiagnosticCode,
    message: &str,
    hint: &str,
) -> RuntimeHostExecutionResponse {
    RuntimeHostExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        execution_request_id: request.execution_request_id.clone(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        state: RuntimeHostExecutionState::Rejected,
        outputs: Vec::new(),
        diagnostics: vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Error,
            code: diagnostic_code,
            message: message.to_string(),
            hint: Some(hint.to_string()),
        }],
        terminal_metadata: None,
    }
}

fn cancellation_rejection_response(
    request: &RuntimeHostExecutionRequest,
    cancellation: &RuntimeHostExecutionCancellationHandle,
) -> Result<Option<RuntimeHostExecutionResponse>, RuntimeHostExecutionPortError> {
    let snapshot = cancellation.snapshot();
    snapshot
        .validate()
        .map_err(|error| RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host cancellation snapshot failed validation: {error}"
            ),
        })?;
    if snapshot.cancellation_context_id != request.cancellation_context.cancellation_context_id {
        return Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: format!(
                "embedded runtime-host cancellation context mismatch: request '{}' but signal '{}'",
                request.cancellation_context.cancellation_context_id,
                snapshot.cancellation_context_id
            ),
        });
    }

    Ok(cancellation_rejection_from_snapshot(request, &snapshot))
}

fn cancellation_rejection_from_snapshot(
    request: &RuntimeHostExecutionRequest,
    snapshot: &RuntimeHostExecutionCancellationSnapshot,
) -> Option<RuntimeHostExecutionResponse> {
    let reason = snapshot.reason.as_deref().unwrap_or("no reason provided");
    match snapshot.state {
        RuntimeHostExecutionCancellationState::Running => None,
        RuntimeHostExecutionCancellationState::CancellationRequested => Some(rejected_response(
            request,
            RuntimeHostExecutionDiagnosticCode::CancellationRequested,
            &format!("embedded runtime-host execution cancelled before completion: {reason}"),
            CANCELLATION_REQUESTED_HINT,
        )),
        RuntimeHostExecutionCancellationState::ShutdownRequested => Some(rejected_response(
            request,
            RuntimeHostExecutionDiagnosticCode::ShutdownRequested,
            &format!(
                "embedded runtime-host execution stopped for workflow-service shutdown: {reason}"
            ),
            SHUTDOWN_REQUESTED_HINT,
        )),
        _ => Some(rejected_response(
            request,
            RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            "embedded runtime-host execution rejected an unknown cancellation state",
            UNKNOWN_CANCELLATION_STATE_HINT,
        )),
    }
}

fn failed_response(
    request: &RuntimeHostExecutionRequest,
    message: &str,
    hint: &str,
) -> RuntimeHostExecutionResponse {
    RuntimeHostExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        execution_request_id: request.execution_request_id.clone(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        state: RuntimeHostExecutionState::Failed,
        outputs: Vec::new(),
        diagnostics: vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Error,
            code: RuntimeHostExecutionDiagnosticCode::ExecutionFailed,
            message: message.to_string(),
            hint: Some(hint.to_string()),
        }],
        terminal_metadata: None,
    }
}

fn completed_text_response(
    request: &RuntimeHostExecutionRequest,
    text: String,
) -> RuntimeHostExecutionResponse {
    RuntimeHostExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        execution_request_id: request.execution_request_id.clone(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        state: RuntimeHostExecutionState::Completed,
        outputs: vec![RuntimeHostExecutionOutput {
            port_id: "text".to_string(),
            value: RuntimeHostExecutionOutputValue::String(text),
        }],
        diagnostics: vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Info,
            code: RuntimeHostExecutionDiagnosticCode::ExecutionCompleted,
            message: "embedded runtime-host text execution completed".to_string(),
            hint: None,
        }],
        terminal_metadata: None,
    }
}

fn completed_image_response(
    request: &RuntimeHostExecutionRequest,
    result: inference::ImageGenerationResult,
    media_artifact_sink: &dyn RuntimeHostMediaArtifactSink,
) -> Result<RuntimeHostExecutionResponse, RuntimeHostMediaArtifactSinkError> {
    let dispatch_decision = request.handoff.dispatch_decision.as_ref();
    let model_id = dispatch_decision.map(|decision| decision.selected_model_ref.model_id.as_str());
    let runtime_id = dispatch_decision.map(|decision| decision.selected_runtime_id.as_str());
    let mut outputs = Vec::with_capacity(result.images.len());
    for (image_index, image) in result.images.iter().enumerate() {
        let artifact_ref =
            media_artifact_sink.write_image_output(RuntimeHostImageArtifactWriteRequest {
                workflow_run_id: request.handoff.workflow_run_id.as_str(),
                workflow_id: request.handoff.workflow_id.as_str(),
                node_id: request.handoff.node_id.as_str(),
                task_id: request.handoff.task_id.as_str(),
                port_id: "image",
                image_index,
                image,
                model_id,
                runtime_id,
            })?;
        outputs.push(RuntimeHostExecutionOutput {
            port_id: "image".to_string(),
            value: RuntimeHostExecutionOutputValue::MediaArtifactRef(artifact_ref),
        });
    }

    Ok(RuntimeHostExecutionResponse {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        execution_request_id: request.execution_request_id.clone(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        state: RuntimeHostExecutionState::Completed,
        outputs,
        diagnostics: vec![RuntimeHostExecutionDiagnostic {
            severity: RuntimeHostExecutionDiagnosticSeverity::Info,
            code: RuntimeHostExecutionDiagnosticCode::ExecutionCompleted,
            message: "embedded runtime-host image execution completed".to_string(),
            hint: None,
        }],
        terminal_metadata: None,
    })
}

fn load_target_error_message(error: RuntimeHostPumasLoadTargetError) -> String {
    format!("embedded runtime-host Pumas load-target resolution failed: {error}")
}

fn package_facts_error_message(error: RuntimeHostPumasPackageFactsError) -> String {
    format!("embedded runtime-host Pumas package-facts resolution failed: {error}")
}

fn image_projection_error_message(error: RuntimeHostImageGenerationProjectionError) -> String {
    format!("embedded runtime-host image projection failed: {error}")
}

fn text_projection_error_message(error: RuntimeHostTextGenerationProjectionError) -> String {
    format!("embedded runtime-host text projection failed: {error}")
}

fn text_gateway_error_message(error: inference::GatewayError) -> String {
    format!("embedded runtime-host text gateway execution failed: {error}")
}

fn gateway_error_message(error: inference::GatewayError) -> String {
    match error {
        inference::GatewayError::ImageGenerationPlanning {
            diagnostic_count,
            diagnostics,
        } => {
            let details = diagnostics
                .iter()
                .take(4)
                .map(|diagnostic| format!("{:?}: {}", diagnostic.code, diagnostic.message))
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "embedded runtime-host image gateway execution failed: image generation planning failed with {diagnostic_count} diagnostic(s): {details}"
            )
        }
        other => format!("embedded runtime-host image gateway execution failed: {other}"),
    }
}

fn batch_gateway_error_message(error: inference::GatewayError) -> String {
    match error {
        inference::GatewayError::ImageGenerationBatchExecution {
            diagnostic_count,
            diagnostics,
        } => {
            let details = diagnostics
                .iter()
                .take(4)
                .map(|diagnostic| {
                    format!(
                        "{:?} at {}: {}",
                        diagnostic.code, diagnostic.field_path, diagnostic.message
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "embedded runtime-host image batch gateway execution failed with {diagnostic_count} diagnostic(s): {details}"
            )
        }
        other => {
            format!("embedded runtime-host image batch gateway execution failed: {other}")
        }
    }
}

fn image_planning_diagnostics_message(
    diagnostics: &[inference::ImageGenerationPlannerDiagnostic],
) -> String {
    let details = diagnostics
        .iter()
        .take(4)
        .map(|diagnostic| {
            format!(
                "{:?} at {}: {}",
                diagnostic.code, diagnostic.field_path, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "embedded runtime-host image batch planning failed with {} diagnostic(s): {details}",
        diagnostics.len()
    )
}

fn media_artifact_sink_error_message(error: RuntimeHostMediaArtifactSinkError) -> String {
    format!("embedded runtime-host media artifact write failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::process::ProcessSpawner;
    use inference::{
        BackendExecutionContext, ImageGenerationBatchExecutionResponse,
        ImageGenerationBatchExecutionState, ImageGenerationBatchMemberExecutionState,
        ImageGenerationExecutionPlan, ImageGenerationResult, RerankRequest, RerankResponse,
    };
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionCancellationSignal, RuntimeHostExecutionContractError,
    };
    use pantograph_workflow_service::{
        ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowArtifactWriter, WorkflowService,
    };
    use pumas_library::models::{
        AssetValidationState, BundleFormat, ImportState, ModelMetadata, PackageArtifactKind,
        PumasArtifactLoadPathKind, PumasArtifactLoadTarget, StorageKind,
    };
    use std::pin::Pin;
    use std::sync::Mutex;

    use crate::runtime_host_media_artifact_sink::{
        RuntimeHostImageArtifactWriteRequest, RuntimeHostMediaArtifactSinkError,
        WorkflowServiceRuntimeHostMediaArtifactSink,
    };
    use crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsResolver;

    #[derive(Debug)]
    struct FixtureCancellationSignal {
        snapshot: RuntimeHostExecutionCancellationSnapshot,
    }

    impl RuntimeHostExecutionCancellationSignal for FixtureCancellationSignal {
        fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
            self.snapshot.clone()
        }
    }

    #[tokio::test]
    async fn fail_closed_port_rejects_without_load_target_resolver() {
        let request = runtime_host_request_fixture();
        let cancellation = runtime_host_cancellation(&request);
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("missing resolver should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert_eq!(response.execution_request_id, "runtime-host.request.001");
        assert!(response.outputs.is_empty());
        assert_eq!(response.diagnostics.len(), 1);
        let diagnostic = &response.diagnostics[0];
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(MISSING_LOAD_TARGET_RESOLVER_HINT)
        );
    }

    #[tokio::test]
    async fn port_rejects_cancelled_request_before_runtime_dependencies() {
        let request = runtime_host_request_fixture();
        let cancellation = runtime_host_cancellation_with_state(
            &request,
            RuntimeHostExecutionCancellationState::CancellationRequested,
            Some("user cancelled task"),
        );
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("cancelled request should produce typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert_eq!(
            response.diagnostics[0].code,
            RuntimeHostExecutionDiagnosticCode::CancellationRequested
        );
        assert_eq!(
            response.diagnostics[0].hint.as_deref(),
            Some(CANCELLATION_REQUESTED_HINT)
        );
        assert!(response.diagnostics[0]
            .message
            .contains("user cancelled task"));
    }

    #[tokio::test]
    async fn port_rejects_shutdown_request_before_runtime_dependencies() {
        let request = runtime_host_request_fixture();
        let cancellation = runtime_host_cancellation_with_state(
            &request,
            RuntimeHostExecutionCancellationState::ShutdownRequested,
            Some("workflow shutdown"),
        );
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("shutdown request should produce typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert_eq!(
            response.diagnostics[0].code,
            RuntimeHostExecutionDiagnosticCode::ShutdownRequested
        );
        assert_eq!(
            response.diagnostics[0].hint.as_deref(),
            Some(SHUTDOWN_REQUESTED_HINT)
        );
        assert!(response.diagnostics[0]
            .message
            .contains("workflow shutdown"));
    }

    #[tokio::test]
    async fn port_rejects_mismatched_cancellation_context_as_port_error() {
        let request = runtime_host_request_fixture();
        let cancellation = RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(
            FixtureCancellationSignal {
                snapshot: RuntimeHostExecutionCancellationSnapshot {
                    cancellation_context_id: "runtime-host-cancellation.other".to_string(),
                    state: RuntimeHostExecutionCancellationState::Running,
                    reason: None,
                },
            },
        ));
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let error = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect_err("mismatched cancellation signal must fail the port");

        assert!(matches!(
            error,
            RuntimeHostExecutionPortError::ExecutionFailed { .. }
        ));
        assert!(error.to_string().contains("cancellation context mismatch"));
    }

    #[tokio::test]
    async fn port_rejects_invalid_requests_as_port_errors() {
        let mut request = runtime_host_request_fixture();
        request.execution_request_id.clear();
        let cancellation = runtime_host_cancellation(&request);
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let error = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect_err("invalid request should fail the port");

        assert!(matches!(
            error,
            RuntimeHostExecutionPortError::ExecutionFailed { .. }
        ));
        assert!(error
            .to_string()
            .contains("embedded runtime-host request failed validation"));
        assert!(error.to_string().contains(
            &RuntimeHostExecutionContractError::InvalidIdentifier {
                field: "execution_request_id"
            }
            .to_string()
        ));
    }

    #[tokio::test]
    async fn port_rejects_after_load_target_when_media_sink_is_missing() {
        let request = runtime_host_request_fixture();
        let cancellation = runtime_host_cancellation(&request);
        let port = EmbeddedRuntimeHostExecutionPort::with_load_target_resolver_only_for_test(
            Arc::new(ReadyLoadTargetResolver),
        );

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("missing media sink should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(MISSING_MEDIA_ARTIFACT_SINK_HINT)
        );
    }

    #[tokio::test]
    async fn port_rejects_after_load_target_when_package_facts_resolver_is_missing() {
        let request = runtime_host_request_fixture();
        let cancellation = runtime_host_cancellation(&request);
        let port = EmbeddedRuntimeHostExecutionPort {
            load_target_resolver: Some(Arc::new(ReadyLoadTargetResolver)),
            media_artifact_sink: Some(Arc::new(UnusedMediaArtifactSink)),
            package_facts_resolver: None,
            gateway: None,
        };

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("missing package resolver should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_eq!(
            diagnostic.code,
            RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable
        );
        assert_eq!(
            diagnostic.hint.as_deref(),
            Some(MISSING_PACKAGE_FACTS_RESOLVER_HINT)
        );
    }

    #[tokio::test]
    async fn port_completes_image_execution_with_sink_backed_media_ref() {
        let temp = tempfile::TempDir::new().expect("temp artifact dir");
        let artifact_writer = artifact_writer(&temp);
        let workflow_service = WorkflowService::new().with_artifact_writer(artifact_writer.clone());
        let mut request = runtime_host_request_fixture();
        request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("fixture has dispatch decision")
            .runtime_trait_settings
            .clear();
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(ReadyLoadTargetResolver),
            Arc::new(FixturePackageFactsResolver),
            Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                artifact_writer,
            )),
            Arc::new(inference::InferenceGateway::with_backend(
                Box::new(MockImageBackend),
                "PyTorch",
            )),
        );
        let cancellation = runtime_host_cancellation(&request);

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("image execution should complete");

        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{response:#?}"
        );
        assert_eq!(response.outputs.len(), 1);
        assert_eq!(
            response.diagnostics[0].code,
            RuntimeHostExecutionDiagnosticCode::ExecutionCompleted
        );
        let RuntimeHostExecutionOutputValue::MediaArtifactRef(artifact_ref) =
            &response.outputs[0].value
        else {
            panic!("image output should be a media artifact ref");
        };
        assert_eq!(response.outputs[0].port_id, "image");
        assert_eq!(artifact_ref.media_type.as_deref(), Some("image_png"));
        let body = workflow_service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: artifact_ref.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("image artifact body should be retained");
        assert_eq!(body.body, b"hello");
        assert_eq!(body.response.media_type, "image/png");
    }

    #[tokio::test]
    async fn port_completes_image_execution_with_pumas_resolvers_and_sink_backed_media_ref() {
        const MODEL_ID: &str = "diffusion/stable-diffusion/tiny-sd-runtime-host";
        const SELECTED_ARTIFACT_ID: &str = "diffusers-bundle";

        let temp = tempfile::TempDir::new().expect("temp artifact dir");
        let artifact_writer = artifact_writer(&temp);
        let workflow_service = WorkflowService::new().with_artifact_writer(artifact_writer.clone());
        let pumas_root = temp.path().join("pumas");
        std::fs::create_dir_all(&pumas_root).expect("pumas launcher root");
        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(pumas_root)
                .with_hf_client(false)
                .with_process_manager(false)
                .build()
                .await
                .expect("pumas api"),
        );
        seed_pumas_diffusers_model(&pumas_api, MODEL_ID, SELECTED_ARTIFACT_ID).await;
        pumas_api
            .resolve_model_package_facts(MODEL_ID)
            .await
            .expect("package facts should seed Pumas load-target cache");
        let mut request = runtime_host_request_fixture();
        set_request_model_ref(&mut request, MODEL_ID, SELECTED_ARTIFACT_ID);
        request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("fixture has dispatch decision")
            .runtime_trait_settings
            .clear();
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(RuntimeHostPumasLoadTargetResolver::new(pumas_api.clone())),
            Arc::new(RuntimeHostPumasPackageFactsResolver::new(pumas_api)),
            Arc::new(WorkflowServiceRuntimeHostMediaArtifactSink::new(
                artifact_writer,
            )),
            Arc::new(inference::InferenceGateway::with_backend(
                Box::new(MockImageBackend),
                "PyTorch",
            )),
        );
        let cancellation = runtime_host_cancellation(&request);

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("image execution should complete through production Pumas resolvers");

        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{response:#?}"
        );
        assert_eq!(response.outputs.len(), 1);
        let RuntimeHostExecutionOutputValue::MediaArtifactRef(artifact_ref) =
            &response.outputs[0].value
        else {
            panic!("image output should be a media artifact ref");
        };
        assert_eq!(response.outputs[0].port_id, "image");
        assert_eq!(artifact_ref.media_type.as_deref(), Some("image_png"));
        let body = workflow_service
            .read_artifact_body(ArtifactReadRequest {
                artifact_id: artifact_ref.artifact_id.clone(),
                byte_range_start: None,
                byte_range_end_exclusive: None,
            })
            .expect("image artifact body should be retained");
        assert_eq!(body.body, b"hello");
        assert_eq!(body.response.media_type, "image/png");
    }

    #[tokio::test]
    async fn batch_port_rejects_without_load_target_resolver() {
        let request = runtime_host_batch_request_fixture();
        let cancellation = runtime_host_batch_cancellation(&request);
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_batch_request(request, cancellation)
            .await
            .expect("missing resolver should be a typed rejected batch response");

        assert_eq!(response.state, RuntimeHostBatchExecutionState::Rejected);
        assert_eq!(
            response.batch_execution_request_id,
            "runtime-host.batch.001"
        );
        assert_eq!(response.members.len(), 2);
        assert!(response
            .members
            .iter()
            .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Rejected));
        assert_eq!(
            response.diagnostics[0].code,
            RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired
        );
        assert_eq!(
            response.diagnostics[0].hint.as_deref(),
            Some(MISSING_LOAD_TARGET_RESOLVER_HINT)
        );
    }

    #[tokio::test]
    async fn batch_port_executes_one_gateway_batch_and_writes_member_outputs() {
        let request = runtime_host_batch_request_fixture();
        let cancellation = runtime_host_batch_cancellation(&request);
        let media_sink = Arc::new(RecordingMediaArtifactSink::default());
        let backend = RecordingBatchImageBackend::default();
        let recorded_batches = backend.recorded_batches.clone();
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(ReadyLoadTargetResolver),
            Arc::new(FixturePackageFactsResolver),
            media_sink.clone(),
            Arc::new(inference::InferenceGateway::with_backend(
                Box::new(backend),
                "PyTorch",
            )),
        );

        let response = port
            .execute_runtime_host_batch_request(request, cancellation)
            .await
            .expect("batch execution should complete through gateway batch operation");

        assert_eq!(response.state, RuntimeHostBatchExecutionState::Completed);
        assert_eq!(response.members.len(), 2);
        assert!(response
            .members
            .iter()
            .all(|member| member.state == RuntimeHostBatchExecutionMemberState::Completed));
        assert!(response
            .members
            .iter()
            .all(|member| member.outputs.len() == 1));
        let recorded_batches = recorded_batches.lock().expect("recorded batches");
        assert_eq!(recorded_batches.len(), 1);
        assert_eq!(recorded_batches[0].members.len(), 2);
        assert_eq!(
            recorded_batches[0]
                .members
                .iter()
                .map(|member| member.request.prompt.as_str())
                .collect::<Vec<_>>(),
            vec![
                "a cinematic image of a red cube on a white table",
                "a cinematic image of a blue cube on a white table"
            ]
        );
        let writes = media_sink.writes.lock().expect("recorded image writes");
        assert_eq!(writes.len(), 2);
        assert_eq!(writes[0].image_data_base64, "aGVsbG8tMA==");
        assert_eq!(writes[1].image_data_base64, "aGVsbG8tMQ==");
    }

    #[tokio::test]
    async fn batch_port_rejects_incompatible_members_before_gateway_dispatch() {
        let mut request = runtime_host_batch_request_fixture();
        let incompatible_variant = "diffusers-pytorch.other"
            .parse()
            .expect("runtime variant id");
        request.members[1]
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("fixture has dispatch decision")
            .selected_runtime_variant_id = Some(incompatible_variant);
        let cancellation = runtime_host_batch_cancellation(&request);
        let backend = RecordingBatchImageBackend::default();
        let recorded_batches = backend.recorded_batches.clone();
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(ReadyLoadTargetResolver),
            Arc::new(FixturePackageFactsResolver),
            Arc::new(RecordingMediaArtifactSink::default()),
            Arc::new(inference::InferenceGateway::with_backend(
                Box::new(backend),
                "PyTorch",
            )),
        );

        let response = port
            .execute_runtime_host_batch_request(request, cancellation)
            .await
            .expect("incompatible batch should be a typed rejected response");

        assert_eq!(response.state, RuntimeHostBatchExecutionState::Rejected);
        assert!(response.diagnostics[0]
            .message
            .contains("must share selected runtime variant"));
        assert_eq!(
            response.diagnostics[0].hint.as_deref(),
            Some(BATCH_COMPATIBILITY_FAILED_HINT)
        );
        assert!(
            recorded_batches
                .lock()
                .expect("recorded batches")
                .is_empty(),
            "incompatible members must not reach gateway batch dispatch"
        );
    }

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture should deserialize")
    }

    fn runtime_host_batch_request_fixture() -> RuntimeHostBatchExecutionRequest {
        let mut first = runtime_host_request_fixture();
        clear_runtime_trait_settings(&mut first);
        let mut second = runtime_host_request_fixture();
        second.execution_request_id = "runtime-host.request.002".to_string();
        clear_runtime_trait_settings(&mut second);
        set_prompt_input(
            &mut second,
            "a cinematic image of a blue cube on a white table",
        );

        RuntimeHostBatchExecutionRequest {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: "runtime-host.batch.001".to_string(),
            anchor_execution_request_id: first.execution_request_id.clone(),
            cancellation_context:
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationContext::workflow_service(
                    "runtime-host.batch.001",
                ),
            members: vec![
                runtime_host_batch_member_from_request(first, "assignment.image.001"),
                runtime_host_batch_member_from_request(second, "assignment.image.002"),
            ],
        }
    }

    fn runtime_host_batch_member_from_request(
        request: RuntimeHostExecutionRequest,
        assignment_id: &str,
    ) -> RuntimeHostBatchExecutionMemberRequest {
        RuntimeHostBatchExecutionMemberRequest {
            execution_request_id: request.execution_request_id,
            assignment_id: assignment_id.to_string(),
            handoff: request.handoff,
            materialized_inputs: request.materialized_inputs,
            timeout_ms: None,
            failure_policy: RuntimeHostBatchMemberFailurePolicy::Retryable,
            reservation_policy: RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal,
        }
    }

    fn clear_runtime_trait_settings(request: &mut RuntimeHostExecutionRequest) {
        if let Some(dispatch_decision) = request.handoff.dispatch_decision.as_mut() {
            dispatch_decision.runtime_trait_settings.clear();
        }
    }

    fn set_prompt_input(request: &mut RuntimeHostExecutionRequest, prompt: &str) {
        let input = request
            .materialized_inputs
            .iter_mut()
            .find(|input| input.port_id == "prompt")
            .expect("fixture has prompt input");
        input.value = pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(
            prompt.to_string(),
        );
    }

    fn runtime_host_cancellation(
        request: &RuntimeHostExecutionRequest,
    ) -> RuntimeHostExecutionCancellationHandle {
        RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone())
    }

    fn runtime_host_batch_cancellation(
        request: &RuntimeHostBatchExecutionRequest,
    ) -> RuntimeHostExecutionCancellationHandle {
        RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone())
    }

    fn runtime_host_cancellation_with_state(
        request: &RuntimeHostExecutionRequest,
        state: RuntimeHostExecutionCancellationState,
        reason: Option<&str>,
    ) -> RuntimeHostExecutionCancellationHandle {
        RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(FixtureCancellationSignal {
            snapshot: RuntimeHostExecutionCancellationSnapshot {
                cancellation_context_id: request
                    .cancellation_context
                    .cancellation_context_id
                    .clone(),
                state,
                reason: reason.map(str::to_string),
            },
        }))
    }

    async fn seed_pumas_diffusers_model(
        pumas_api: &pumas_library::PumasApi,
        model_id: &str,
        selected_artifact_id: &str,
    ) {
        let library = pumas_api.model_library();
        let model_dir =
            library.build_model_path("diffusion", "stable-diffusion", "tiny-sd-runtime-host");
        create_diffusers_bundle(&model_dir);
        let metadata = ModelMetadata {
            schema_version: Some(2),
            model_id: Some(model_id.to_string()),
            family: Some("stable-diffusion".to_string()),
            model_type: Some("diffusion".to_string()),
            official_name: Some("tiny-sd-runtime-host".to_string()),
            cleaned_name: Some("tiny-sd-runtime-host".to_string()),
            storage_kind: Some(StorageKind::LibraryOwned),
            bundle_format: Some(BundleFormat::DiffusersDirectory),
            pipeline_class: Some("StableDiffusionPipeline".to_string()),
            import_state: Some(ImportState::Ready),
            validation_state: Some(AssetValidationState::Valid),
            task_type_primary: Some("text-to-image".to_string()),
            input_modalities: Some(vec!["text".to_string()]),
            output_modalities: Some(vec!["image".to_string()]),
            recommended_backend: Some("diffusers".to_string()),
            runtime_engine_hints: Some(vec!["diffusers".to_string(), "pytorch".to_string()]),
            selected_artifact_id: Some(selected_artifact_id.to_string()),
            ..Default::default()
        };
        library
            .save_metadata(&model_dir, &metadata)
            .await
            .expect("model metadata should save");
        library
            .index_model_dir(&model_dir)
            .await
            .expect("model metadata should index");
    }

    fn create_diffusers_bundle(model_dir: &std::path::Path) {
        std::fs::create_dir_all(model_dir.join("unet")).expect("unet dir");
        std::fs::create_dir_all(model_dir.join("vae")).expect("vae dir");
        std::fs::create_dir_all(model_dir.join("scheduler")).expect("scheduler dir");
        std::fs::create_dir_all(model_dir.join("text_encoder")).expect("text encoder dir");
        std::fs::create_dir_all(model_dir.join("tokenizer")).expect("tokenizer dir");
        write_min_safetensors(&model_dir.join("unet/diffusion_pytorch_model.safetensors"));
        write_min_safetensors(&model_dir.join("vae/diffusion_pytorch_model.safetensors"));
        write_min_safetensors(&model_dir.join("text_encoder/model.safetensors"));
        std::fs::write(
            model_dir.join("unet/config.json"),
            r#"{"model_type":"unet"}"#,
        )
        .expect("unet config fixture");
        std::fs::write(model_dir.join("vae/config.json"), r#"{"model_type":"vae"}"#)
            .expect("vae config fixture");
        std::fs::write(
            model_dir.join("text_encoder/config.json"),
            r#"{"model_type":"clip_text_model"}"#,
        )
        .expect("text encoder config fixture");
        std::fs::write(
            model_dir.join("scheduler/scheduler_config.json"),
            r#"{"scheduler":"euler"}"#,
        )
        .expect("scheduler fixture");
        std::fs::write(
            model_dir.join("tokenizer/tokenizer_config.json"),
            r#"{"model_type":"clip_tokenizer"}"#,
        )
        .expect("tokenizer config fixture");
        std::fs::write(
            model_dir.join("tokenizer/tokenizer.json"),
            r#"{"tokenizer":"tiny-sd-runtime-host"}"#,
        )
        .expect("tokenizer fixture");
        std::fs::write(
            model_dir.join("model_index.json"),
            r#"{
  "_class_name": "StableDiffusionPipeline",
  "scheduler": ["diffusers", "EulerDiscreteScheduler"],
  "unet": ["diffusers", "UNet2DConditionModel"],
  "vae": ["diffusers", "AutoencoderKL"],
  "text_encoder": ["transformers", "CLIPTextModel"],
  "tokenizer": ["transformers", "CLIPTokenizer"]
}"#,
        )
        .expect("model index fixture");
    }

    fn write_min_safetensors(path: &std::path::Path) {
        let header = b"{}";
        let header_size = header.len() as u64;
        let mut content = header_size.to_le_bytes().to_vec();
        content.extend_from_slice(header);
        content.extend_from_slice(&[0; 64]);
        std::fs::write(path, content).expect("minimal safetensors fixture");
    }

    fn set_request_model_ref(
        request: &mut RuntimeHostExecutionRequest,
        model_id: &str,
        selected_artifact_id: &str,
    ) {
        request.handoff.task_intent.model_ref.model_id = model_id.to_string();
        request.handoff.task_intent.model_ref.selected_artifact_id =
            Some(selected_artifact_id.to_string());
        request.handoff.task_intent.model_ref.selected_artifact_path = None;
        request
            .handoff
            .readiness_proof
            .preflight_result
            .identity_key
            .model_ref = request.handoff.task_intent.model_ref.clone();
        if let Some(dispatch_decision) = request.handoff.dispatch_decision.as_mut() {
            dispatch_decision.task_intent.model_ref.model_id = model_id.to_string();
            dispatch_decision.task_intent.model_ref.selected_artifact_id =
                Some(selected_artifact_id.to_string());
            dispatch_decision
                .task_intent
                .model_ref
                .selected_artifact_path = None;
            dispatch_decision.selected_model_ref.model_id = model_id.to_string();
            dispatch_decision.selected_model_ref.selected_artifact_id =
                Some(selected_artifact_id.to_string());
            dispatch_decision.selected_model_ref.selected_artifact_path = None;
            dispatch_decision
                .readiness_proof
                .preflight_result
                .identity_key
                .model_ref = dispatch_decision.task_intent.model_ref.clone();
        }
    }

    struct ReadyLoadTargetResolver;

    #[async_trait]
    impl RuntimeHostLoadTargetResolver for ReadyLoadTargetResolver {
        async fn resolve(
            &self,
            _request: &ValidatedRuntimeHostExecutionRequest,
        ) -> Result<PumasArtifactLoadTarget, RuntimeHostPumasLoadTargetError> {
            Ok(PumasArtifactLoadTarget {
                model_ref: pumas_library::models::PumasModelRef {
                    model_id: "pumas://models/juggernaut-xl-v10".to_string(),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: Some("juggernaut-xl-v10/diffusers".to_string()),
                    ..Default::default()
                },
                artifact_kind: PackageArtifactKind::DiffusersBundle,
                local_load_path: "/host-only/pumas/juggernaut-xl-v10".to_string(),
                load_path_kind: PumasArtifactLoadPathKind::Directory,
                library_root_id: Some("default".to_string()),
                storage_kind: StorageKind::LibraryOwned,
                validation_state: AssetValidationState::Valid,
                content_fingerprint: Some("sha256:abc".to_string()),
                package_facts_contract_version: Some(
                    pumas_library::models::PACKAGE_FACTS_CONTRACT_VERSION,
                ),
            })
        }
    }

    struct FixturePackageFactsResolver;

    #[async_trait]
    impl RuntimeHostPackageFactsResolver for FixturePackageFactsResolver {
        async fn resolve(
            &self,
            request: &ValidatedRuntimeHostExecutionRequest,
        ) -> Result<inference::ResolvedModelPackageFacts, RuntimeHostPumasPackageFactsError>
        {
            let selected_model_ref = request
                .as_ref()
                .handoff
                .dispatch_decision
                .as_ref()
                .expect("fixture has dispatch decision")
                .selected_model_ref
                .clone();
            let mut package_facts: inference::ResolvedModelPackageFacts =
                serde_json::from_str(include_str!(
                    "../../inference/tests/fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"
                ))
                .expect("image package facts fixture should decode");
            package_facts.model_ref = inference::PumasModelRef {
                model_id: selected_model_ref.model_id,
                revision: selected_model_ref.revision,
                selected_artifact_id: selected_model_ref.selected_artifact_id,
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            };
            Ok(package_facts)
        }
    }

    struct UnusedMediaArtifactSink;

    impl RuntimeHostMediaArtifactSink for UnusedMediaArtifactSink {
        fn write_image_output(
            &self,
            _request: RuntimeHostImageArtifactWriteRequest<'_>,
        ) -> Result<
            pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
            RuntimeHostMediaArtifactSinkError,
        > {
            panic!("media sink must not be called before runtime execution is wired")
        }
    }

    #[derive(Default)]
    struct RecordingMediaArtifactSink {
        writes: Mutex<Vec<RecordedImageWrite>>,
    }

    struct RecordedImageWrite {
        image_data_base64: String,
    }

    impl RuntimeHostMediaArtifactSink for RecordingMediaArtifactSink {
        fn write_image_output(
            &self,
            request: RuntimeHostImageArtifactWriteRequest<'_>,
        ) -> Result<
            pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
            RuntimeHostMediaArtifactSinkError,
        > {
            let mut writes = self.writes.lock().expect("record image write");
            let index = writes.len();
            writes.push(RecordedImageWrite {
                image_data_base64: request.image.data_base64.clone(),
            });
            Ok(
                pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef {
                    artifact_id: format!("runtime-host-batch-artifact.{index}"),
                    media_type: Some("image_png".to_string()),
                },
            )
        }
    }

    #[derive(Default)]
    struct RecordingBatchImageBackend {
        recorded_batches: Arc<Mutex<Vec<inference::ImageGenerationBatchExecutionRequest>>>,
    }

    #[async_trait]
    impl InferenceBackend for RecordingBatchImageBackend {
        fn name(&self) -> &'static str {
            "MockBatch"
        }

        fn description(&self) -> &'static str {
            "Mock image batch backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities {
                image_generation: true,
                image_generation_batch: true,
                ..BackendCapabilities::default()
            }
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_mock_batch_runtime".to_string()),
            })
        }

        async fn stop(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<
            Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }

        async fn generate_image_from_plan(
            &self,
            _plan: ImageGenerationExecutionPlan,
            _context: BackendExecutionContext,
        ) -> Result<ImageGenerationResult, BackendError> {
            panic!("batch runtime-host execution must not call single image generation")
        }

        async fn generate_image_batch_from_execution_request(
            &self,
            request: inference::ImageGenerationBatchExecutionRequest,
            _context: BackendExecutionContext,
        ) -> Result<ImageGenerationBatchExecutionResponse, BackendError> {
            self.recorded_batches
                .lock()
                .expect("record batch request")
                .push(request.clone());
            Ok(ImageGenerationBatchExecutionResponse {
                batch_execution_id: request.batch_execution_id,
                state: ImageGenerationBatchExecutionState::Completed,
                members: request
                    .members
                    .into_iter()
                    .enumerate()
                    .map(|(index, member)| {
                        let image_data_base64 = if index == 0 {
                            "aGVsbG8tMA=="
                        } else {
                            "aGVsbG8tMQ=="
                        };
                        inference::ImageGenerationBatchExecutionMemberResponse {
                            member_id: member.member_id,
                            state: ImageGenerationBatchMemberExecutionState::Completed,
                            result: Some(ImageGenerationResult {
                                images: vec![inference::EncodedImage {
                                    data_base64: image_data_base64.to_string(),
                                    mime_type: "image/png".to_string(),
                                    width: member.plan.width,
                                    height: member.plan.height,
                                }],
                                seed_used: member.plan.seed,
                                metadata: serde_json::Value::Null,
                            }),
                            diagnostics: Vec::new(),
                        }
                    })
                    .collect(),
                diagnostics: Vec::new(),
            })
        }
    }

    struct MockImageBackend;

    #[async_trait]
    impl InferenceBackend for MockImageBackend {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn description(&self) -> &'static str {
            "Mock image backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities {
                image_generation: true,
                ..BackendCapabilities::default()
            }
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
            })
        }

        async fn stop(&mut self) -> Result<(), BackendError> {
            Ok(())
        }

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<
            Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }

        async fn generate_image_from_plan(
            &self,
            plan: ImageGenerationExecutionPlan,
            _context: BackendExecutionContext,
        ) -> Result<ImageGenerationResult, BackendError> {
            Ok(ImageGenerationResult {
                images: vec![inference::EncodedImage {
                    data_base64: "aGVsbG8=".to_string(),
                    mime_type: "image/png".to_string(),
                    width: plan.width,
                    height: plan.height,
                }],
                seed_used: plan.seed,
                metadata: serde_json::Value::Null,
            })
        }
    }

    fn artifact_writer(temp: &tempfile::TempDir) -> WorkflowArtifactWriter {
        let artifact_store = ArtifactStore::open(temp.path().join("artifacts"), artifact_policy())
            .expect("open artifact store");
        WorkflowArtifactWriter::new(artifact_store)
    }

    fn artifact_policy() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "runtime-host-execution-port-test".to_string(),
            policy_version: 1,
            ttl_seconds: None,
            max_disk_bytes: None,
            max_memory_bytes: None,
            max_single_artifact_bytes: None,
            spill_threshold_bytes: None,
            delete_on_consume: false,
        }
    }
}
