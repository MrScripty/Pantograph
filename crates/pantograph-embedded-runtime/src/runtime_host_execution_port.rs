use std::sync::Arc;

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionDiagnostic, RuntimeHostExecutionDiagnosticCode,
    RuntimeHostExecutionDiagnosticSeverity, RuntimeHostExecutionOutput,
    RuntimeHostExecutionOutputValue, RuntimeHostExecutionPort, RuntimeHostExecutionPortError,
    RuntimeHostExecutionRequest, RuntimeHostExecutionResponse, RuntimeHostExecutionState,
    ValidatedRuntimeHostExecutionRequest, RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
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
const GATEWAY_EXECUTION_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.gateway_execution_failed";
const MEDIA_ARTIFACT_WRITE_FAILED_HINT: &str =
    "embedded_runtime_host_execution_port.media_artifact_write_failed";
const RUNTIME_EXECUTION_UNAVAILABLE_HINT: &str =
    "embedded_runtime_host_execution_port.runtime_execution_unavailable";

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
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        let validated_request =
            ValidatedRuntimeHostExecutionRequest::try_from(request).map_err(|error| {
                RuntimeHostExecutionPortError::ExecutionFailed {
                    message: format!("embedded runtime-host request failed validation: {error}"),
                }
            })?;

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
                let result = match gateway
                    .generate_image_from_planning_input(projection.planning_input())
                    .await
                {
                    Ok(result) => result,
                    Err(error) => {
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
        BackendExecutionContext, ImageGenerationExecutionPlan, ImageGenerationResult,
        RerankRequest, RerankResponse,
    };
    use pantograph_runtime_host_contracts::RuntimeHostExecutionContractError;
    use pantograph_workflow_service::{
        ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowArtifactWriter, WorkflowService,
    };
    use pumas_library::models::{
        AssetValidationState, BundleFormat, ImportState, ModelMetadata, PackageArtifactKind,
        PumasArtifactLoadPathKind, PumasArtifactLoadTarget, StorageKind,
    };
    use std::pin::Pin;

    use crate::runtime_host_media_artifact_sink::{
        RuntimeHostImageArtifactWriteRequest, RuntimeHostMediaArtifactSinkError,
        WorkflowServiceRuntimeHostMediaArtifactSink,
    };
    use crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsResolver;

    #[tokio::test]
    async fn fail_closed_port_rejects_without_load_target_resolver() {
        let request = runtime_host_request_fixture();
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let response = port
            .execute_runtime_host_request(request)
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
    async fn port_rejects_invalid_requests_as_port_errors() {
        let mut request = runtime_host_request_fixture();
        request.execution_request_id.clear();
        let port = EmbeddedRuntimeHostExecutionPort::fail_closed();

        let error = port
            .execute_runtime_host_request(request)
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
        let port = EmbeddedRuntimeHostExecutionPort::with_load_target_resolver_only_for_test(
            Arc::new(ReadyLoadTargetResolver),
        );

        let response = port
            .execute_runtime_host_request(request)
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
        let port = EmbeddedRuntimeHostExecutionPort {
            load_target_resolver: Some(Arc::new(ReadyLoadTargetResolver)),
            media_artifact_sink: Some(Arc::new(UnusedMediaArtifactSink)),
            package_facts_resolver: None,
            gateway: None,
        };

        let response = port
            .execute_runtime_host_request(request)
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

        let response = port
            .execute_runtime_host_request(request)
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

        let response = port
            .execute_runtime_host_request(request)
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

    fn runtime_host_request_fixture() -> RuntimeHostExecutionRequest {
        serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host request fixture should deserialize")
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
                package_facts_contract_version: Some(2),
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

        fn stop(&mut self) {}

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
