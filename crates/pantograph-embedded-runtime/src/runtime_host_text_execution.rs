use inference::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, InferenceDevicePolicy, InferenceExecutionInput, InferenceExecutionRequest,
    InferenceExecutionResult, InferenceTaskId, ModelRefMigrationDiagnostic,
    PumasArtifactLoadTarget, PumasModelRef, ResolvedModelPackageFacts, RuntimeVariantId,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, RuntimeHostExecutionRequest,
    ValidatedRuntimeHostExecutionRequest,
};
use pantograph_scheduler::SchedulerDispatchDecision;
use thiserror::Error;

pub(crate) const TEXT_GENERATION_TASK: &str = "text_generation";
pub(crate) const PROMPT_PORT: &str = "prompt";
pub(crate) const MAX_TEXT_BYTES: usize = 1024;

/// Owned inputs for the canonical selected-text inference call.
#[derive(Debug)]
pub(crate) struct RuntimeHostTextGenerationProjection {
    request: InferenceExecutionRequest,
    artifact_load_target: PumasArtifactLoadTarget,
    backend_decision: BackendExecutionDecision,
}

impl RuntimeHostTextGenerationProjection {
    pub(crate) fn request(&self) -> &InferenceExecutionRequest {
        &self.request
    }

    pub(crate) fn artifact_load_target(&self) -> &PumasArtifactLoadTarget {
        &self.artifact_load_target
    }

    pub(crate) fn backend_decision(&self) -> &BackendExecutionDecision {
        &self.backend_decision
    }
}

/// Validate the host-owned text shape before resolving or loading runtime
/// dependencies. The inference gateway repeats its own typed validation after
/// the separately validated Pumas target and scheduler decision are attached.
pub(crate) fn validate_runtime_host_text_generation_request(
    request: &RuntimeHostExecutionRequest,
) -> Result<(), RuntimeHostTextGenerationProjectionError> {
    if request.handoff.task_intent.task_type.as_str() != TEXT_GENERATION_TASK {
        return Err(RuntimeHostTextGenerationProjectionError::UnsupportedTask {
            task_type: request.handoff.task_intent.task_type.as_str().to_string(),
        });
    }
    let dispatch_decision = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostTextGenerationProjectionError::MissingDispatchDecision)?;
    validate_supported_inputs(request)?;
    let prompt = required_prompt(request)?;
    if prompt.trim().is_empty() {
        return Err(RuntimeHostTextGenerationProjectionError::BlankPrompt);
    }
    if prompt.len() > MAX_TEXT_BYTES {
        return Err(RuntimeHostTextGenerationProjectionError::InputTooLong {
            bytes: prompt.len(),
        });
    }
    validate_selected_runtime_and_device(dispatch_decision)?;
    Ok(())
}

pub(crate) fn project_runtime_host_text_generation(
    request: &ValidatedRuntimeHostExecutionRequest,
    package_facts: ResolvedModelPackageFacts,
    load_target: pumas_library::models::PumasArtifactLoadTarget,
) -> Result<RuntimeHostTextGenerationProjection, RuntimeHostTextGenerationProjectionError> {
    let request = request.as_ref();
    validate_runtime_host_text_generation_request(request)?;
    let dispatch_decision = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostTextGenerationProjectionError::MissingDispatchDecision)?;
    let prompt = required_prompt(request)?.to_string();
    let backend_decision = text_backend_decision(dispatch_decision)?;
    let artifact_load_target =
        crate::runtime_host_image_execution::project_pumas_artifact_load_target(load_target);
    let inference_request = InferenceExecutionRequest {
        request_id: Some(request.execution_request_id.clone()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(project_model_ref(&dispatch_decision.selected_model_ref)),
        model_name: Some(dispatch_decision.selected_model_ref.model_id.clone()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some(prompt),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    Ok(RuntimeHostTextGenerationProjection {
        request: inference_request,
        artifact_load_target,
        backend_decision,
    })
}

pub(crate) fn text_from_inference_result(
    result: InferenceExecutionResult,
) -> Result<String, RuntimeHostTextGenerationProjectionError> {
    let InferenceExecutionResult::TextGeneration { text, .. } = result else {
        return Err(
            RuntimeHostTextGenerationProjectionError::UnexpectedInferenceResult {
                result_type: "non_text_generation".to_string(),
            },
        );
    };
    if text.len() > MAX_TEXT_BYTES {
        return Err(RuntimeHostTextGenerationProjectionError::OutputTooLong { bytes: text.len() });
    }
    Ok(text)
}

fn project_model_ref(model_ref: &pantograph_dependency_planning::PumasModelRef) -> PumasModelRef {
    PumasModelRef {
        model_id: model_ref.model_id.clone(),
        revision: model_ref.revision.clone(),
        selected_artifact_id: model_ref.selected_artifact_id.clone(),
        selected_artifact_path: model_ref.selected_artifact_path.clone(),
        migration_diagnostics: model_ref
            .migration_diagnostics
            .iter()
            .map(|diagnostic| ModelRefMigrationDiagnostic {
                code: diagnostic.code.clone(),
                message: diagnostic.message.clone(),
                input: diagnostic.input.clone(),
            })
            .collect(),
    }
}

fn text_backend_decision(
    decision: &SchedulerDispatchDecision,
) -> Result<BackendExecutionDecision, RuntimeHostTextGenerationProjectionError> {
    validate_selected_runtime_and_device(decision)?;
    let selected_runtime_variant_id = selected_runtime_variant_id(decision)?;
    let selected_device_id = selected_device_id(decision)?;
    let selected_device_class = device_class(selected_device_id.as_str())?;
    let device_decision = DeviceResolutionDecision {
        policy: InferenceDevicePolicy::Auto,
        runtime_variant_id: selected_runtime_variant_id.clone(),
        selected_device_class,
        selected_device_id: Some(selected_device_id.clone()),
        diagnostics: Vec::new(),
    };
    Ok(BackendExecutionDecision {
        selected_backend_id: BackendId::parse("pytorch").map_err(|error| {
            RuntimeHostTextGenerationProjectionError::InvalidBackendId {
                value: "pytorch".to_string(),
                message: error.to_string(),
            }
        })?,
        selected_runtime_variant_id,
        selected_device_class,
        selected_device_id: Some(selected_device_id),
        device_decision,
        selected_task_id: Some(InferenceTaskId::TextGeneration),
        selected_model_ref: Some(project_model_ref(&decision.selected_model_ref)),
        diagnostics: Vec::new(),
        dependency_readiness: Vec::new(),
        selection_policy_trace: None,
    })
}

fn validate_selected_runtime_and_device(
    decision: &SchedulerDispatchDecision,
) -> Result<(), RuntimeHostTextGenerationProjectionError> {
    let runtime_id = decision.selected_runtime_id.as_str();
    if !matches!(runtime_id, "pytorch" | "pytorch.transformers") {
        return Err(
            RuntimeHostTextGenerationProjectionError::UnsupportedRuntime {
                runtime_id: runtime_id.to_string(),
            },
        );
    }
    let _ = selected_runtime_variant_id(decision)?;
    let _ = selected_device_id(decision)?;
    Ok(())
}

fn selected_runtime_variant_id(
    decision: &SchedulerDispatchDecision,
) -> Result<RuntimeVariantId, RuntimeHostTextGenerationProjectionError> {
    let value = decision
        .selected_runtime_variant_id
        .as_ref()
        .ok_or(RuntimeHostTextGenerationProjectionError::MissingSelectedRuntimeVariant)?
        .to_string();
    let runtime_variant_id = RuntimeVariantId::parse(&value).map_err(|error| {
        RuntimeHostTextGenerationProjectionError::InvalidRuntimeVariantId {
            value: value.clone(),
            message: error.to_string(),
        }
    })?;
    if !matches!(
        runtime_variant_id.as_str(),
        "pytorch.cpu" | "pytorch.cuda" | "pytorch.mps"
    ) {
        return Err(
            RuntimeHostTextGenerationProjectionError::UnsupportedRuntimeVariant {
                runtime_id: runtime_variant_id.to_string(),
            },
        );
    }
    Ok(runtime_variant_id)
}

fn selected_device_id(
    decision: &SchedulerDispatchDecision,
) -> Result<InferenceDeviceId, RuntimeHostTextGenerationProjectionError> {
    let value = match decision.selected_device_ids.as_slice() {
        [] => return Err(RuntimeHostTextGenerationProjectionError::MissingSelectedDevice),
        [value] => value,
        values => {
            return Err(
                RuntimeHostTextGenerationProjectionError::AmbiguousSelectedDevices {
                    count: values.len(),
                },
            );
        }
    };
    InferenceDeviceId::parse(value.as_str()).map_err(|error| {
        RuntimeHostTextGenerationProjectionError::InvalidDeviceId {
            value: value.as_str().to_string(),
            message: error.to_string(),
        }
    })
}

fn device_class(
    device_id: &str,
) -> Result<InferenceDeviceClass, RuntimeHostTextGenerationProjectionError> {
    if device_id == "cpu" {
        return Ok(InferenceDeviceClass::Cpu);
    }
    if device_id
        .strip_prefix("cuda:")
        .is_some_and(|index| index.parse::<u32>().is_ok())
    {
        return Ok(InferenceDeviceClass::Cuda);
    }
    if device_id == "mps" {
        return Ok(InferenceDeviceClass::Mps);
    }
    Err(
        RuntimeHostTextGenerationProjectionError::UnsupportedDevice {
            device_id: device_id.to_string(),
        },
    )
}

fn required_prompt(
    request: &RuntimeHostExecutionRequest,
) -> Result<&str, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == PROMPT_PORT)
        .map(|input| match &input.value {
            RuntimeHostExecutionInputValue::String(value) => Ok(value.as_str()),
            _ => Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: PROMPT_PORT,
                expected: "string",
            }),
        })
        .unwrap_or(Err(
            RuntimeHostTextGenerationProjectionError::MissingRequiredInput {
                port_id: PROMPT_PORT,
            },
        ))
}

fn validate_supported_inputs(
    request: &RuntimeHostExecutionRequest,
) -> Result<(), RuntimeHostTextGenerationProjectionError> {
    for input in &request.materialized_inputs {
        if input.port_id != PROMPT_PORT {
            return Err(
                RuntimeHostTextGenerationProjectionError::UnsupportedInputPort {
                    port_id: input.port_id.clone(),
                },
            );
        }
    }
    Ok(())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum RuntimeHostTextGenerationProjectionError {
    #[error("runtime-host text execution supports text_generation only, got {task_type}")]
    UnsupportedTask { task_type: String },
    #[error("runtime-host text execution requires a scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("runtime-host text execution requires materialized input '{port_id}'")]
    MissingRequiredInput { port_id: &'static str },
    #[error("runtime-host text execution prompt must not be blank")]
    BlankPrompt,
    #[error("runtime-host text input '{port_id}' must be {expected}")]
    InvalidInputType {
        port_id: &'static str,
        expected: &'static str,
    },
    #[error("runtime-host text execution does not support materialized input '{port_id}'")]
    UnsupportedInputPort { port_id: String },
    #[error("runtime-host text input is {bytes} bytes; max is 1024")]
    InputTooLong { bytes: usize },
    #[error("runtime-host text execution requires a selected PyTorch runtime; got {runtime_id}")]
    UnsupportedRuntime { runtime_id: String },
    #[error("runtime-host text execution requires a selected runtime variant")]
    MissingSelectedRuntimeVariant,
    #[error("runtime-host text execution selected runtime variant '{runtime_id}' is unsupported")]
    UnsupportedRuntimeVariant { runtime_id: String },
    #[error("runtime-host text execution has no selected concrete device")]
    MissingSelectedDevice,
    #[error("runtime-host text execution has {count} selected devices; exactly one is required")]
    AmbiguousSelectedDevices { count: usize },
    #[error("runtime-host text selected device '{value}' is invalid: {message}")]
    InvalidDeviceId { value: String, message: String },
    #[error("runtime-host text selected device '{device_id}' is unsupported")]
    UnsupportedDevice { device_id: String },
    #[error("runtime-host text selected runtime variant '{value}' is invalid: {message}")]
    InvalidRuntimeVariantId { value: String, message: String },
    #[error("runtime-host text backend id '{value}' is invalid: {message}")]
    InvalidBackendId { value: String, message: String },
    #[error("runtime-host text execution output is {bytes} bytes; max is 1024")]
    OutputTooLong { bytes: usize },
    #[error("runtime-host text execution returned an unexpected result: {result_type}")]
    UnexpectedInferenceResult { result_type: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures_util::stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::process::ProcessSpawner;
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionInput, RuntimeHostExecutionOutputValue, RuntimeHostExecutionPort,
        RuntimeHostExecutionRequest, RuntimeHostExecutionState,
        ValidatedRuntimeHostExecutionRequest,
    };
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    #[test]
    fn projects_exact_prompt_and_scheduler_selection_without_rewriting_package_facts() {
        let request = text_request_fixture();
        let request = ValidatedRuntimeHostExecutionRequest::try_from(request)
            .expect("text request fixture should validate");
        let package_facts = text_package_facts(&request);
        let target_directory = tempfile::tempdir().expect("target directory");
        let target_path = target_directory
            .path()
            .to_str()
            .expect("target path")
            .to_string();
        let target = text_load_target(&package_facts, &target_directory);
        let projection = project_runtime_host_text_generation(&request, package_facts, target)
            .expect("text request should project");

        assert_eq!(
            projection.request().request_id.as_deref(),
            Some("runtime-host.request.001")
        );
        assert_eq!(
            projection.request().model_name.as_deref(),
            Some("pumas://models/juggernaut-xl-v10")
        );
        assert_eq!(
            projection
                .request()
                .resolved_model_package_facts
                .as_ref()
                .unwrap()
                .artifact
                .entry_path,
            "llm/example/tiny-transformers"
        );
        assert!(matches!(
            &projection.request().input,
            InferenceExecutionInput::TextGeneration { prompt: Some(prompt), system_prompt: None, messages, stream: false }
                if prompt == "exact prompt" && messages.is_empty()
        ));
        assert_eq!(
            projection.backend_decision().selected_backend_id.as_str(),
            "pytorch"
        );
        assert_eq!(
            projection
                .backend_decision()
                .selected_runtime_variant_id
                .as_str(),
            "pytorch.cpu"
        );
        assert_eq!(
            projection
                .backend_decision()
                .selected_device_id
                .as_ref()
                .map(InferenceDeviceId::as_str),
            Some("cpu")
        );
        assert_eq!(
            projection
                .backend_decision()
                .selected_model_ref
                .as_ref()
                .unwrap()
                .model_id,
            "pumas://models/juggernaut-xl-v10"
        );
        assert_eq!(
            projection.artifact_load_target().local_load_path,
            target_path
        );
    }

    #[test]
    fn accepts_1024_bytes_and_rejects_oversize_multibyte_prompt_without_truncation() {
        let mut request = text_request_fixture();
        let exact = "🦀".repeat(256);
        set_prompt(&mut request, exact.clone());
        validate_runtime_host_text_generation_request(&request)
            .expect("256 four-byte scalars should fit exactly");
        assert_eq!(required_prompt(&request).expect("prompt"), exact);

        set_prompt(&mut request, "🦀".repeat(257));
        let error = validate_runtime_host_text_generation_request(&request)
            .expect_err("the 1025th byte must be rejected");
        assert!(matches!(
            error,
            RuntimeHostTextGenerationProjectionError::InputTooLong { bytes: 1028 }
        ));
    }

    #[test]
    fn rejects_unsupported_inputs_before_dependency_resolution() {
        let mut request = text_request_fixture();
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: "system_prompt".to_string(),
            value: RuntimeHostExecutionInputValue::String("ignored".to_string()),
        });

        let error = validate_runtime_host_text_generation_request(&request)
            .expect_err("system prompt is not a host text input");
        assert!(matches!(
            error,
            RuntimeHostTextGenerationProjectionError::UnsupportedInputPort { port_id }
                if port_id == "system_prompt"
        ));
    }

    #[test]
    fn rejects_non_text_results_and_oversize_results_without_partial_output() {
        let error = text_from_inference_result(InferenceExecutionResult::TextGeneration {
            text: "🦀".repeat(257),
            usage: None,
            cache_handle_id: None,
            option_diagnostics: Vec::new(),
        })
        .expect_err("oversize output must fail before host output construction");
        assert!(matches!(
            error,
            RuntimeHostTextGenerationProjectionError::OutputTooLong { bytes: 1028 }
        ));

        let error = text_from_inference_result(InferenceExecutionResult::Embedding {
            embeddings: Vec::new(),
            usage: None,
            option_diagnostics: Vec::new(),
        })
        .expect_err("an image/text mismatch must not be projected as text");
        assert!(matches!(
            error,
            RuntimeHostTextGenerationProjectionError::UnexpectedInferenceResult { .. }
        ));
    }

    #[tokio::test]
    async fn executes_text_without_calling_an_image_sink() {
        let request = text_request_fixture();
        let validated_request = ValidatedRuntimeHostExecutionRequest::try_from(request.clone())
            .expect("text request fixture should validate");
        let package_facts = text_package_facts(&validated_request);
        let target_directory = tempfile::tempdir().expect("target directory");
        let target = text_load_target(&package_facts, &target_directory);
        let target_path = target.local_load_path.clone();
        let package_resolver = Arc::new(TextPackageFactsResolver { package_facts });
        let target_resolver = Arc::new(TextLoadTargetResolver { target });
        let backend = TextBackend::default();
        let backend_calls = Arc::clone(&backend.calls);
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            target_resolver,
            package_resolver,
            Arc::new(UnusedTextMediaSink),
            Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
        );
        let cancellation =
            pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                request.cancellation_context.clone(),
            );

        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .expect("selected text should produce a host response");

        assert_eq!(response.state, RuntimeHostExecutionState::Completed);
        assert_eq!(response.outputs.len(), 1);
        assert_eq!(response.outputs[0].port_id, "text");
        assert_eq!(
            response.outputs[0].value,
            RuntimeHostExecutionOutputValue::String("generated text".to_string())
        );
        assert!(backend_calls
            .lock()
            .expect("backend calls")
            .iter()
            .any(|call| call == &format!("load:{target_path}:cpu")));
    }

    fn text_request_fixture() -> RuntimeHostExecutionRequest {
        let mut request: RuntimeHostExecutionRequest = serde_json::from_str(include_str!(
            "../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json"
        ))
        .expect("runtime host fixture should decode");
        request
            .materialized_inputs
            .retain(|input| input.port_id == PROMPT_PORT);
        set_prompt(&mut request, "exact prompt");
        request.handoff.task_intent.task_type = TEXT_GENERATION_TASK.parse().expect("task type");
        request.handoff.task_intent.constraints.requested_runtime_id =
            Some("pytorch".parse().expect("runtime id"));
        request.handoff.task_intent.constraints.requested_device_id =
            Some("cpu".parse().expect("device id"));
        let identity_key = &mut request
            .handoff
            .readiness_proof
            .preflight_result
            .identity_key;
        identity_key.task_id = TEXT_GENERATION_TASK.parse().expect("task id");
        identity_key.scheduler_intent.requested_runtime_id =
            Some("pytorch".parse().expect("runtime id"));
        identity_key.scheduler_intent.requested_device_id = Some("cpu".parse().expect("device id"));
        let task_intent = request.handoff.task_intent.clone();
        let readiness_proof = request.handoff.readiness_proof.clone();
        let decision = request
            .handoff
            .dispatch_decision
            .as_mut()
            .expect("dispatch decision");
        decision.task_intent = task_intent;
        decision.selected_runtime_id = "pytorch".parse().expect("runtime id");
        decision.selected_runtime_variant_id = Some("pytorch.cpu".parse().expect("variant id"));
        decision.selected_device_ids = vec!["cpu".parse().expect("device id")];
        for reservation in &mut decision.reservations {
            reservation.device_id = "cpu".parse().expect("device id");
        }
        decision.readiness_proof = readiness_proof;
        request
    }

    fn set_prompt(request: &mut RuntimeHostExecutionRequest, prompt: impl Into<String>) {
        request.materialized_inputs[0].value =
            RuntimeHostExecutionInputValue::String(prompt.into());
    }

    fn text_package_facts(
        request: &ValidatedRuntimeHostExecutionRequest,
    ) -> ResolvedModelPackageFacts {
        let mut package_facts: ResolvedModelPackageFacts = serde_json::from_str(include_str!(
            "../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
        ))
        .expect("text package facts should decode");
        let selected_model_ref = request
            .as_ref()
            .handoff
            .dispatch_decision
            .as_ref()
            .expect("dispatch decision")
            .selected_model_ref
            .clone();
        package_facts.model_ref = PumasModelRef {
            model_id: selected_model_ref.model_id,
            revision: selected_model_ref.revision,
            selected_artifact_id: selected_model_ref.selected_artifact_id,
            selected_artifact_path: selected_model_ref.selected_artifact_path,
            migration_diagnostics: Vec::new(),
        };
        package_facts.custom_code.requires_custom_code = false;
        package_facts.custom_code.custom_code_sources.clear();
        package_facts.custom_code.auto_map_sources.clear();
        package_facts
    }

    fn text_load_target(
        package_facts: &ResolvedModelPackageFacts,
        target_directory: &tempfile::TempDir,
    ) -> pumas_library::models::PumasArtifactLoadTarget {
        pumas_library::models::PumasArtifactLoadTarget {
            model_ref: pumas_library::models::PumasModelRef {
                model_id: package_facts.model_ref.model_id.clone(),
                revision: package_facts.model_ref.revision.clone(),
                selected_artifact_id: package_facts.model_ref.selected_artifact_id.clone(),
                selected_artifact_path: package_facts.model_ref.selected_artifact_path.clone(),
                ..Default::default()
            },
            artifact_kind: pumas_library::models::PackageArtifactKind::HfCompatibleDirectory,
            local_load_path: target_directory
                .path()
                .to_str()
                .expect("target path")
                .to_string(),
            load_path_kind: pumas_library::models::PumasArtifactLoadPathKind::Directory,
            library_root_id: Some("text-test-root".to_string()),
            storage_kind: pumas_library::models::StorageKind::LibraryOwned,
            validation_state: pumas_library::models::AssetValidationState::Valid,
            content_fingerprint: None,
            package_facts_contract_version: Some(package_facts.package_facts_contract_version),
        }
    }

    struct TextLoadTargetResolver {
        target: pumas_library::models::PumasArtifactLoadTarget,
    }

    #[async_trait]
    impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for TextLoadTargetResolver {
        async fn resolve(
            &self,
            _request: &ValidatedRuntimeHostExecutionRequest,
        ) -> Result<
            pumas_library::models::PumasArtifactLoadTarget,
            crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
        > {
            Ok(self.target.clone())
        }
    }

    struct TextPackageFactsResolver {
        package_facts: ResolvedModelPackageFacts,
    }

    #[async_trait]
    impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver
        for TextPackageFactsResolver
    {
        async fn resolve(
            &self,
            _request: &ValidatedRuntimeHostExecutionRequest,
        ) -> Result<
            ResolvedModelPackageFacts,
            crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
        > {
            Ok(self.package_facts.clone())
        }
    }

    struct UnusedTextMediaSink;

    impl crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSink for UnusedTextMediaSink {
        fn write_image_output(
            &self,
            _request: crate::runtime_host_media_artifact_sink::RuntimeHostImageArtifactWriteRequest<
                '_,
            >,
        ) -> Result<
            pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
            crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSinkError,
        > {
            panic!("text execution must not call the image artifact sink");
        }
    }

    #[derive(Default)]
    struct TextBackend {
        calls: Arc<Mutex<Vec<String>>>,
        cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
        fail_completion: bool,
    }

    #[async_trait]
    impl InferenceBackend for TextBackend {
        fn name(&self) -> &'static str {
            "PyTorch"
        }

        fn description(&self) -> &'static str {
            "selected text test backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities {
                streaming: true,
                ..BackendCapabilities::default()
            }
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome::default())
        }

        async fn load_selected_text(
            &mut self,
            _request: &InferenceExecutionRequest,
            target: &PumasArtifactLoadTarget,
            decision: &BackendExecutionDecision,
        ) -> Result<BackendStartOutcome, BackendError> {
            self.calls.lock().expect("backend calls").push(format!(
                "load:{}:{}",
                target.local_load_path,
                decision
                    .selected_device_id
                    .as_ref()
                    .expect("selected device")
            ));
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("selected_text_test_loaded".to_string()),
            })
        }

        async fn finish_selected_text(&self, _cancel: bool) -> Result<(), BackendError> {
            if self.fail_completion {
                return Err(BackendError::Inference(
                    "selected text cleanup failed".into(),
                ));
            }
            self.calls
                .lock()
                .expect("backend calls")
                .push("finish".to_string());
            Ok(())
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
            request_json: String,
        ) -> Result<
            Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            let json: serde_json::Value = serde_json::from_str(&request_json).unwrap();
            let prompt = json["messages"][0]["content"][0]["text"].as_str().unwrap();
            if prompt == "cancel" {
                self.cancel
                    .as_ref()
                    .unwrap()
                    .store(true, std::sync::atomic::Ordering::SeqCst);
            }
            if prompt == "fail" {
                return Ok(Box::pin(stream::iter([
                    Ok(ChatChunk {
                        content: Some("must not escape".into()),
                        done: false,
                        usage: None,
                        cache_handle_id: None,
                    }),
                    Err(BackendError::Inference(
                        "selected text producer failed".into(),
                    )),
                ])));
            }
            let output = match prompt {
                "oversize" => "🦀".repeat(257),
                "exact-limit" => "🦀".repeat(256),
                _ => "generated text".into(),
            };
            Ok(Box::pin(stream::iter([
                Ok(ChatChunk {
                    content: Some(output),
                    done: false,
                    usage: None,
                    cache_handle_id: None,
                }),
                Ok(ChatChunk {
                    content: None,
                    done: true,
                    usage: None,
                    cache_handle_id: None,
                }),
            ])))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(
            &self,
            _request: inference::RerankRequest,
        ) -> Result<inference::RerankResponse, BackendError> {
            Ok(inference::RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }
    }
    fn text_batch(
        prompts: &[&str],
    ) -> pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest {
        use pantograph_runtime_host_contracts::*;
        let members = prompts
            .iter()
            .enumerate()
            .map(|(index, prompt)| {
                let mut request = text_request_fixture();
                set_prompt(&mut request, *prompt);
                request
                    .handoff
                    .dispatch_decision
                    .as_mut()
                    .unwrap()
                    .runtime_trait_settings
                    .clear();
                RuntimeHostBatchExecutionMemberRequest {
                    execution_request_id: format!("selected-text.request.{index}"),
                    assignment_id: format!("selected-text.assignment.{index}"),
                    handoff: request.handoff,
                    materialized_inputs: request.materialized_inputs,
                    timeout_ms: None,
                    failure_policy: RuntimeHostBatchMemberFailurePolicy::Retryable,
                    reservation_policy: RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal,
                }
            })
            .collect::<Vec<_>>();
        RuntimeHostBatchExecutionRequest {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            batch_execution_request_id: "selected-text.batch".into(),
            anchor_execution_request_id: members[0].execution_request_id.clone(),
            cancellation_context: RuntimeHostExecutionCancellationContext::workflow_service(
                "selected-text.batch",
            ),
            members,
        }
    }

    fn text_test_port(
        backend: TextBackend,
        directory: &tempfile::TempDir,
    ) -> crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort {
        let request =
            ValidatedRuntimeHostExecutionRequest::try_from(text_request_fixture()).unwrap();
        let package_facts = text_package_facts(&request);
        let target = text_load_target(&package_facts, directory);
        crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
            Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
        )
    }

    #[tokio::test]
    async fn text_batch_retains_only_complete_bounded_member_outputs() {
        use pantograph_runtime_host_contracts::*;
        let directory = tempfile::tempdir().unwrap();
        let port = text_test_port(TextBackend::default(), &directory);
        let request = text_batch(&["exact-limit", "oversize", "fail"]);
        let cancellation =
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
        let response = port
            .execute_runtime_host_batch_request(request, cancellation)
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostBatchExecutionState::PartiallyCompleted
        );
        assert_eq!(
            response.members[0].state,
            RuntimeHostBatchExecutionMemberState::Completed
        );
        assert_eq!(
            response.members[0].outputs[0].value,
            RuntimeHostExecutionOutputValue::String("🦀".repeat(256))
        );
        for member in &response.members[1..] {
            assert_eq!(member.state, RuntimeHostBatchExecutionMemberState::Failed);
            assert!(member.outputs.is_empty());
        }
        assert!(response.members[1]
            .diagnostics
            .iter()
            .any(|d| d.message.contains("1028")));
        assert!(response.members[2]
            .diagnostics
            .iter()
            .any(|d| d.message.contains("producer failed")));
        let _validated = ValidatedRuntimeHostBatchExecutionResponse::try_from(response).unwrap();
    }

    struct TextCancellationSignal {
        context_id: String,
        cancelled: Arc<std::sync::atomic::AtomicBool>,
    }
    impl pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSignal
        for TextCancellationSignal
    {
        fn snapshot(
            &self,
        ) -> pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
            use pantograph_runtime_host_contracts::*;
            RuntimeHostExecutionCancellationSnapshot {
                cancellation_context_id: self.context_id.clone(),
                state: if self.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                    RuntimeHostExecutionCancellationState::CancellationRequested
                } else {
                    RuntimeHostExecutionCancellationState::Running
                },
                reason: Some("text batch test".into()),
            }
        }
    }

    #[tokio::test]
    async fn text_batch_cancellation_preserves_cleanup_failure_and_cancels_unstarted_members() {
        use pantograph_runtime_host_contracts::*;
        for fail_completion in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(false));
            let calls = Arc::new(Mutex::new(Vec::new()));
            let backend = TextBackend {
                cancel: Some(cancelled.clone()),
                fail_completion,
                calls: calls.clone(),
            };
            let port = text_test_port(backend, &directory);
            let request = text_batch(&["cancel", "exact prompt"]);
            let cancellation = RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(
                TextCancellationSignal {
                    context_id: request.cancellation_context.cancellation_context_id.clone(),
                    cancelled,
                },
            ));
            let response = port
                .execute_runtime_host_batch_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(
                response.members[0].state,
                if fail_completion {
                    RuntimeHostBatchExecutionMemberState::Failed
                } else {
                    RuntimeHostBatchExecutionMemberState::Cancelled
                }
            );
            assert_eq!(
                response.members[1].state,
                RuntimeHostBatchExecutionMemberState::Cancelled
            );
            assert!(response
                .members
                .iter()
                .all(|member| member.outputs.is_empty()));
            assert_eq!(
                calls
                    .lock()
                    .unwrap()
                    .iter()
                    .filter(|call| call.starts_with("load:"))
                    .count(),
                1
            );
            if fail_completion {
                assert!(response.members[0]
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("cleanup failed")));
            }
            let _validated =
                ValidatedRuntimeHostBatchExecutionResponse::try_from(response).unwrap();
        }
    }
}
