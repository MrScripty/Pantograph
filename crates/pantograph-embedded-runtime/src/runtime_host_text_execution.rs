use inference::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, GenerationOptions,
    InferenceDeviceClass, InferenceDeviceId, InferenceDevicePolicy, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceExecutionResult, InferenceTaskId, LengthGenerationOptions,
    ModelRefMigrationDiagnostic, PumasArtifactLoadTarget, PumasModelRef, ResolvedModelPackageFacts,
    RuntimeVariantId, SamplingGenerationOptions, StoppingGenerationOptions,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, RuntimeHostExecutionRequest,
    ValidatedRuntimeHostExecutionRequest,
};
use pantograph_scheduler::SchedulerDispatchDecision;
use thiserror::Error;

pub(crate) const TEXT_GENERATION_TASK: &str = "text_generation";
pub(crate) const PROMPT_PORT: &str = "prompt";
pub(crate) const MAX_NEW_TOKENS_PORT: &str = "max_new_tokens";
pub(crate) const SYSTEM_PROMPT_PORT: &str = "system_prompt";
pub(crate) const TOP_K_PORT: &str = "top_k";
pub(crate) const TEMPERATURE_PORT: &str = "temperature";
pub(crate) const TOP_P_PORT: &str = "top_p";
pub(crate) const REPETITION_PENALTY_PORT: &str = "repetition_penalty";
pub(crate) const MIN_NEW_TOKENS_PORT: &str = "min_new_tokens";
pub(crate) const SEED_PORT: &str = "seed";
pub(crate) const STOP_PORT: &str = "stop";
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
    validate_min_new_tokens_budget(request)?;
    optional_top_k(request)?;
    optional_temperature(request)?;
    optional_top_p(request)?;
    optional_repetition_penalty(request)?;
    optional_seed(request)?;
    optional_stop(request)?;
    optional_system_prompt(request)?;
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
            system_prompt: optional_system_prompt(request)?.map(str::to_owned),
            messages: Vec::new(),
            stream: false,
        },
        generation_options: optional_generation_options(request)?,
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

pub(crate) fn project_model_ref(
    model_ref: &pantograph_dependency_planning::PumasModelRef,
) -> PumasModelRef {
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
        if ![
            PROMPT_PORT,
            MAX_NEW_TOKENS_PORT,
            SYSTEM_PROMPT_PORT,
            TOP_K_PORT,
            TEMPERATURE_PORT,
            TOP_P_PORT,
            REPETITION_PENALTY_PORT,
            MIN_NEW_TOKENS_PORT,
            SEED_PORT,
            STOP_PORT,
        ]
        .contains(&input.port_id.as_str())
        {
            return Err(
                RuntimeHostTextGenerationProjectionError::UnsupportedInputPort {
                    port_id: input.port_id.clone(),
                },
            );
        }
    }
    Ok(())
}

fn optional_max_new_tokens(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<u32>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == MAX_NEW_TOKENS_PORT)
        .map(|input| {
            let value = match input.value {
                RuntimeHostExecutionInputValue::U64(value) => u32::try_from(value).ok(),
                RuntimeHostExecutionInputValue::I64(value) => u32::try_from(value).ok(),
                _ => {
                    return Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                        port_id: MAX_NEW_TOKENS_PORT,
                        expected: "integer",
                    });
                }
            };
            value
                .filter(|value| *value > 0)
                .ok_or(RuntimeHostTextGenerationProjectionError::InvalidMaxNewTokens)
        })
        .transpose()
}

fn optional_top_k(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<u32>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == TOP_K_PORT)
        .map(|input| {
            match input.value {
                RuntimeHostExecutionInputValue::U64(value) => u32::try_from(value).ok(),
                RuntimeHostExecutionInputValue::I64(value) => u32::try_from(value).ok(),
                _ => {
                    return Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                        port_id: TOP_K_PORT,
                        expected: "integer",
                    });
                }
            }
            .ok_or(RuntimeHostTextGenerationProjectionError::InvalidTopK)
        })
        .transpose()
}

fn optional_min_new_tokens(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<u32>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == MIN_NEW_TOKENS_PORT)
        .map(|input| {
            match input.value {
                RuntimeHostExecutionInputValue::U64(value) => u32::try_from(value).ok(),
                RuntimeHostExecutionInputValue::I64(value) => u32::try_from(value).ok(),
                _ => {
                    return Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                        port_id: MIN_NEW_TOKENS_PORT,
                        expected: "integer",
                    })
                }
            }
            .ok_or(RuntimeHostTextGenerationProjectionError::InvalidMinNewTokens)
        })
        .transpose()
}

fn validate_min_new_tokens_budget(
    request: &RuntimeHostExecutionRequest,
) -> Result<(), RuntimeHostTextGenerationProjectionError> {
    let maximum = optional_max_new_tokens(request)?
        .unwrap_or(inference::constants::pytorch::DEFAULT_MAX_NEW_TOKENS);
    if let Some(minimum) = optional_min_new_tokens(request)? {
        if minimum > maximum {
            return Err(
                RuntimeHostTextGenerationProjectionError::MinNewTokensExceedsBudget {
                    minimum,
                    maximum,
                },
            );
        }
    }
    Ok(())
}

fn optional_temperature(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<f32>, RuntimeHostTextGenerationProjectionError> {
    optional_sampling_number(
        request,
        TEMPERATURE_PORT,
        0.0..=f32::MAX,
        RuntimeHostTextGenerationProjectionError::InvalidTemperature,
    )
}

fn optional_top_p(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<f32>, RuntimeHostTextGenerationProjectionError> {
    optional_sampling_number(
        request,
        TOP_P_PORT,
        0.0..=1.0,
        RuntimeHostTextGenerationProjectionError::InvalidTopP,
    )
}

fn optional_repetition_penalty(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<f32>, RuntimeHostTextGenerationProjectionError> {
    optional_sampling_number(
        request,
        REPETITION_PENALTY_PORT,
        f32::from_bits(1)..=f32::MAX,
        RuntimeHostTextGenerationProjectionError::InvalidRepetitionPenalty,
    )
}

fn optional_sampling_number(
    request: &RuntimeHostExecutionRequest,
    port_id: &'static str,
    range: std::ops::RangeInclusive<f32>,
    invalid: RuntimeHostTextGenerationProjectionError,
) -> Result<Option<f32>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == port_id)
        .map(|input| {
            if !matches!(
                input.value,
                RuntimeHostExecutionInputValue::F64(_)
                    | RuntimeHostExecutionInputValue::I64(_)
                    | RuntimeHostExecutionInputValue::U64(_)
            ) {
                return Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                    port_id,
                    expected: "finite number",
                });
            }
            input
                .value
                .try_as_f32()
                .ok()
                .filter(|value| range.contains(value))
                .ok_or(invalid)
        })
        .transpose()
}

fn optional_seed(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<u64>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == SEED_PORT)
        .map(|input| match input.value {
            RuntimeHostExecutionInputValue::U64(value) => Ok(value),
            RuntimeHostExecutionInputValue::I64(value) => u64::try_from(value)
                .map_err(|_| RuntimeHostTextGenerationProjectionError::InvalidSeed),
            _ => Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: SEED_PORT,
                expected: "non-negative integer",
            }),
        })
        .transpose()
}

fn optional_stop(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<&str>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == STOP_PORT)
        .map(|input| match &input.value {
            RuntimeHostExecutionInputValue::String(value) if value.is_empty() => {
                Err(RuntimeHostTextGenerationProjectionError::EmptyStopString)
            }
            RuntimeHostExecutionInputValue::String(value) if value.len() > MAX_TEXT_BYTES => {
                Err(RuntimeHostTextGenerationProjectionError::InputTooLong { bytes: value.len() })
            }
            RuntimeHostExecutionInputValue::String(value) => Ok(value.as_str()),
            _ => Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: STOP_PORT,
                expected: "string",
            }),
        })
        .transpose()
}

fn optional_generation_options(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<GenerationOptions>, RuntimeHostTextGenerationProjectionError> {
    let max_new_tokens = optional_max_new_tokens(request)?;
    let min_new_tokens = optional_min_new_tokens(request)?;
    let top_k = optional_top_k(request)?;
    let temperature = optional_temperature(request)?;
    let top_p = optional_top_p(request)?;
    let repetition_penalty = optional_repetition_penalty(request)?;
    let seed = optional_seed(request)?;
    let stop = optional_stop(request)?;
    if max_new_tokens.is_none()
        && min_new_tokens.is_none()
        && top_k.is_none()
        && temperature.is_none()
        && top_p.is_none()
        && repetition_penalty.is_none()
        && seed.is_none()
        && stop.is_none()
    {
        return Ok(None);
    }
    Ok(Some(GenerationOptions {
        length: LengthGenerationOptions {
            max_new_tokens,
            min_new_tokens,
            ..LengthGenerationOptions::default()
        },
        sampling: SamplingGenerationOptions {
            top_k,
            temperature,
            top_p,
            repetition_penalty,
            seed,
        },
        stopping: StoppingGenerationOptions {
            stop_strings: stop.map(|value| vec![value.to_owned()]).unwrap_or_default(),
            ..Default::default()
        },
        ..GenerationOptions::default()
    }))
}

fn optional_system_prompt(
    request: &RuntimeHostExecutionRequest,
) -> Result<Option<&str>, RuntimeHostTextGenerationProjectionError> {
    request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == SYSTEM_PROMPT_PORT)
        .map(|input| match &input.value {
            RuntimeHostExecutionInputValue::String(value) if value.len() <= MAX_TEXT_BYTES => {
                Ok(value.as_str())
            }
            RuntimeHostExecutionInputValue::String(value) => {
                Err(RuntimeHostTextGenerationProjectionError::InputTooLong { bytes: value.len() })
            }
            _ => Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: SYSTEM_PROMPT_PORT,
                expected: "string",
            }),
        })
        .transpose()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum RuntimeHostTextGenerationProjectionError {
    #[error("runtime-host text execution supports text_generation only, got {task_type}")]
    UnsupportedTask { task_type: String },
    #[error("runtime-host text execution requires a scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("runtime-host text execution requires materialized input '{port_id}'")]
    MissingRequiredInput { port_id: &'static str },
    #[error("runtime-host text input 'max_new_tokens' must be between 1 and 4294967295")]
    InvalidMaxNewTokens,
    #[error("runtime-host text input 'min_new_tokens' must be between 0 and 4294967295")]
    InvalidMinNewTokens,
    #[error(
        "runtime-host text min_new_tokens {minimum} exceeds effective max_new_tokens {maximum}"
    )]
    MinNewTokensExceedsBudget { minimum: u32, maximum: u32 },
    #[error("runtime-host text input 'top_k' must be between 0 and 4294967295")]
    InvalidTopK,
    #[error("runtime-host text input 'seed' must be a non-negative u64 integer")]
    InvalidSeed,
    #[error("runtime-host text input 'stop' must not be empty")]
    EmptyStopString,
    #[error("runtime-host text input 'temperature' must be a nonnegative finite number representable by the generation f32 contract without losing authored precision")]
    InvalidTemperature,
    #[error("runtime-host text input 'top_p' must be between 0 and 1, retaining authored precision in the generation f32 contract")]
    InvalidTopP,
    #[error("runtime-host text input 'repetition_penalty' must be positive and finite, retaining authored precision in the generation f32 contract")]
    InvalidRepetitionPenalty,
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
        assert!(projection.request().generation_options.is_none());
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
    fn token_limit_preserves_integer_boundaries_and_rejects_invalid_values() {
        for value in [1, 128, u64::from(u32::MAX)] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: MAX_NEW_TOKENS_PORT.to_string(),
                value: RuntimeHostExecutionInputValue::U64(value),
            });
            validate_runtime_host_text_generation_request(&request).expect("valid limit");
            assert_eq!(
                optional_max_new_tokens(&request).unwrap(),
                Some(value as u32)
            );
        }
        for value in [
            RuntimeHostExecutionInputValue::U64(0),
            RuntimeHostExecutionInputValue::U64(u64::from(u32::MAX) + 1),
            RuntimeHostExecutionInputValue::I64(-1),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: MAX_NEW_TOKENS_PORT.to_string(),
                value,
            });
            assert_eq!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidMaxNewTokens),
            );
        }
        let mut request = text_request_fixture();
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: MAX_NEW_TOKENS_PORT.to_string(),
            value: RuntimeHostExecutionInputValue::String("128".to_string()),
        });
        assert!(matches!(
            validate_runtime_host_text_generation_request(&request),
            Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: MAX_NEW_TOKENS_PORT,
                ..
            }),
        ));
    }

    #[test]
    fn top_p_accepts_unit_interval_and_rejects_wrong_type_range_or_precision() {
        let number = |value| {
            RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(value).unwrap())
        };
        assert_eq!(optional_top_p(&text_request_fixture()).unwrap(), None);
        for (value, expected) in [
            (number(0.0), 0.0),
            (number(0.7), 0.7),
            (number(1.0), 1.0),
            (number(f64::from(f32::from_bits(1))), f32::from_bits(1)),
            (RuntimeHostExecutionInputValue::I64(0), 0.0),
            (RuntimeHostExecutionInputValue::U64(1), 1.0),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_P_PORT.into(),
                value,
            });
            validate_runtime_host_text_generation_request(&request).unwrap();
            assert_eq!(optional_top_p(&request).unwrap(), Some(expected));
            assert_eq!(
                optional_generation_options(&request)
                    .unwrap()
                    .unwrap()
                    .sampling
                    .top_p,
                Some(expected)
            );
        }
        for value in [
            number(-0.1),
            number(1.1),
            number(1.0000000000000002),
            number(f64::MAX),
            number(1e-100),
            number(0.7000000000000001),
            RuntimeHostExecutionInputValue::I64(-1),
            RuntimeHostExecutionInputValue::U64(2),
            RuntimeHostExecutionInputValue::U64(u64::MAX),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_P_PORT.into(),
                value,
            });
            assert_eq!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidTopP)
            );
        }
        for value in [
            RuntimeHostExecutionInputValue::String("0.7".into()),
            RuntimeHostExecutionInputValue::Bool(true),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_P_PORT.into(),
                value,
            });
            assert!(matches!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                    port_id: TOP_P_PORT,
                    expected: "finite number"
                })
            ));
        }
    }

    #[test]
    fn temperature_preserves_zero_decimals_and_rejects_precision_loss_and_integer_coercion() {
        let number = |value| {
            RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(value).unwrap())
        };
        let request = text_request_fixture();
        assert_eq!(optional_temperature(&request).unwrap(), None);
        for (value, expected) in [
            (number(0.0), 0.0),
            (number(0.7), 0.7),
            (number(0.001), 0.001),
            (number(2.0), 2.0),
            (number(f64::from(f32::MAX)), f32::MAX),
            (number(f64::from(f32::MIN_POSITIVE)), f32::MIN_POSITIVE),
            (number(f64::from(f32::from_bits(1))), f32::from_bits(1)),
            (RuntimeHostExecutionInputValue::I64(0), 0.0),
            (RuntimeHostExecutionInputValue::U64(2), 2.0),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TEMPERATURE_PORT.into(),
                value,
            });
            validate_runtime_host_text_generation_request(&request).unwrap();
            assert_eq!(optional_temperature(&request).unwrap(), Some(expected));
            assert_eq!(
                optional_generation_options(&request)
                    .unwrap()
                    .unwrap()
                    .sampling
                    .temperature,
                Some(expected)
            );
        }
        for value in [
            number(-0.1),
            number(f64::MAX),
            number(1e-100),
            number(0.7000000000000001),
            RuntimeHostExecutionInputValue::I64(-1),
            RuntimeHostExecutionInputValue::U64(16_777_217),
            RuntimeHostExecutionInputValue::U64((1_u64 << 53) + 1),
            RuntimeHostExecutionInputValue::U64(u64::MAX),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TEMPERATURE_PORT.into(),
                value,
            });
            assert_eq!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidTemperature)
            );
        }
        for port_id in [TOP_K_PORT, MAX_NEW_TOKENS_PORT] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: port_id.into(),
                value: number(1.5),
            });
            assert!(matches!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                    expected: "integer",
                    ..
                })
            ));
        }
    }

    #[test]
    fn top_k_accepts_zero_and_u32_boundaries_and_rejects_invalid_values() {
        let request = text_request_fixture();
        assert_eq!(optional_top_k(&request).unwrap(), None);
        for value in [
            RuntimeHostExecutionInputValue::U64(0),
            RuntimeHostExecutionInputValue::U64(40),
            RuntimeHostExecutionInputValue::U64(u64::from(u32::MAX)),
            RuntimeHostExecutionInputValue::I64(0),
            RuntimeHostExecutionInputValue::I64(40),
            RuntimeHostExecutionInputValue::I64(i64::from(u32::MAX)),
        ] {
            let expected = match value {
                RuntimeHostExecutionInputValue::U64(value) => value as u32,
                RuntimeHostExecutionInputValue::I64(value) => value as u32,
                _ => unreachable!(),
            };
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_K_PORT.into(),
                value,
            });
            validate_runtime_host_text_generation_request(&request).unwrap();
            assert_eq!(optional_top_k(&request).unwrap(), Some(expected));
        }
        for value in [
            RuntimeHostExecutionInputValue::I64(-1),
            RuntimeHostExecutionInputValue::U64(u64::from(u32::MAX) + 1),
            RuntimeHostExecutionInputValue::U64(u64::MAX),
            RuntimeHostExecutionInputValue::I64(i64::MAX),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_K_PORT.into(),
                value,
            });
            assert_eq!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidTopK)
            );
        }
        for value in [
            RuntimeHostExecutionInputValue::String("40".into()),
            RuntimeHostExecutionInputValue::Bool(true),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_K_PORT.into(),
                value,
            });
            assert_eq!(
                validate_runtime_host_text_generation_request(&request),
                Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                    port_id: TOP_K_PORT,
                    expected: "integer",
                })
            );
        }
    }

    #[tokio::test]
    async fn invalid_top_k_fails_host_execution_before_recording_gateway_calls() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                "top_k' must be between 0",
            ),
            (
                RuntimeHostExecutionInputValue::U64(u64::from(u32::MAX) + 1),
                "top_k' must be between 0",
            ),
            (
                RuntimeHostExecutionInputValue::String("40".into()),
                "top_k' must be integer",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "top_k' must be integer",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_K_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }),
                Arc::new(TextPackageFactsResolver { package_facts }),
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
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn invalid_temperature_fails_host_execution_before_recording_gateway_calls() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                "temperature' must be a nonnegative finite number",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(f64::MAX).unwrap(),
                ),
                "temperature' must be a nonnegative finite number",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1e-100).unwrap()),
                "temperature' must be a nonnegative finite number",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(0.7000000000000001).unwrap(),
                ),
                "temperature' must be a nonnegative finite number",
            ),
            (
                RuntimeHostExecutionInputValue::String("40".into()),
                "temperature' must be finite number",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "temperature' must be finite number",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TEMPERATURE_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }),
                Arc::new(TextPackageFactsResolver { package_facts }),
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
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn invalid_top_p_fails_host_execution_before_recording_gateway_calls() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                "top_p' must be between 0 and 1",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(f64::MAX).unwrap(),
                ),
                "top_p' must be between 0 and 1",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1e-100).unwrap()),
                "top_p' must be between 0 and 1",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(0.7000000000000001).unwrap(),
                ),
                "top_p' must be between 0 and 1",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1.1).unwrap()),
                "top_p' must be between 0 and 1",
            ),
            (
                RuntimeHostExecutionInputValue::String("40".into()),
                "top_p' must be finite number",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "top_p' must be finite number",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TOP_P_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }),
                Arc::new(TextPackageFactsResolver { package_facts }),
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
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn invalid_repetition_penalty_fails_host_execution_before_recording_gateway_calls() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                "repetition_penalty' must be positive and finite",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(f64::MAX).unwrap(),
                ),
                "repetition_penalty' must be positive and finite",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1e-100).unwrap()),
                "repetition_penalty' must be positive and finite",
            ),
            (
                RuntimeHostExecutionInputValue::F64(
                    serde_json::Number::from_f64(0.7000000000000001).unwrap(),
                ),
                "repetition_penalty' must be positive and finite",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(0.0).unwrap()),
                "repetition_penalty' must be positive and finite",
            ),
            (
                RuntimeHostExecutionInputValue::String("40".into()),
                "repetition_penalty' must be finite number",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "repetition_penalty' must be finite number",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: REPETITION_PENALTY_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }),
                Arc::new(TextPackageFactsResolver { package_facts }),
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
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn min_new_tokens_reaches_actual_host_gateway_with_zero_omission_and_budget_boundary() {
        for (minimum, maximum) in [
            (None, None),
            (Some(0), None),
            (Some(3), Some(3)),
            (
                Some(inference::constants::pytorch::DEFAULT_MAX_NEW_TOKENS),
                None,
            ),
            (Some(u32::MAX), Some(u32::MAX)),
        ] {
            let mut request = text_request_fixture();
            for (port_id, value) in [
                (MIN_NEW_TOKENS_PORT, minimum),
                (MAX_NEW_TOKENS_PORT, maximum),
            ] {
                if let Some(value) = value {
                    request.materialized_inputs.push(RuntimeHostExecutionInput {
                        port_id: port_id.into(),
                        value: RuntimeHostExecutionInputValue::U64(u64::from(value)),
                    });
                }
            }
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TEMPERATURE_PORT.into(),
                value: RuntimeHostExecutionInputValue::I64(0),
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let projection = project_runtime_host_text_generation(
                &validated,
                package_facts.clone(),
                target.clone(),
            )
            .unwrap();
            assert_eq!(
                projection
                    .request()
                    .generation_options
                    .as_ref()
                    .unwrap()
                    .length
                    .min_new_tokens,
                minimum
            );
            let backend = TextBackend::default();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
            );
            let cancellation =
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                );
            let response = port
                .execute_runtime_host_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Completed);
            let recorded = recorded.lock().unwrap();
            assert_eq!(recorded.len(), 1);
            assert_eq!(
                recorded[0].get("min_new_tokens"),
                minimum.map(serde_json::Value::from).as_ref()
            );
            assert_eq!(
                recorded[0].get("max_tokens"),
                maximum.map(serde_json::Value::from).as_ref()
            );
            assert_eq!(recorded[0]["temperature"], serde_json::json!(0.0));
        }
    }

    #[tokio::test]
    async fn min_new_tokens_invalid_shape_or_budget_refuses_host_before_backend_effects() {
        for (value, maximum, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                None,
                "must be between 0",
            ),
            (
                RuntimeHostExecutionInputValue::U64(u64::from(u32::MAX) + 1),
                None,
                "must be between 0",
            ),
            (
                RuntimeHostExecutionInputValue::String("3".into()),
                None,
                "must be integer",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                None,
                "must be integer",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(3.0).unwrap()),
                None,
                "must be integer",
            ),
            (
                RuntimeHostExecutionInputValue::U64(513),
                None,
                "exceeds effective max_new_tokens 512",
            ),
            (
                RuntimeHostExecutionInputValue::I64(9),
                Some(8),
                "exceeds effective max_new_tokens 8",
            ),
            (
                RuntimeHostExecutionInputValue::U64(0),
                Some(0),
                "must be between 1",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: MIN_NEW_TOKENS_PORT.into(),
                value,
            });
            if let Some(maximum) = maximum {
                request.materialized_inputs.push(RuntimeHostExecutionInput {
                    port_id: MAX_NEW_TOKENS_PORT.into(),
                    value: RuntimeHostExecutionInputValue::U64(maximum),
                });
            }
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
            );
            let cancellation =
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                );
            let response = port
                .execute_runtime_host_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn temperature_reaches_recording_gateway_with_optional_existing_controls() {
        for temperature in [None, Some(0.0), Some(0.7), Some(2.0), Some(f32::MAX)] {
            for companion_controls in [false, true] {
                let mut request = text_request_fixture();
                if let Some(value) = temperature {
                    request.materialized_inputs.push(RuntimeHostExecutionInput {
                        port_id: TEMPERATURE_PORT.into(),
                        value: RuntimeHostExecutionInputValue::F64(
                            serde_json::Number::from_f64(f64::from(value)).unwrap(),
                        ),
                    });
                }
                if companion_controls {
                    request.materialized_inputs.extend([
                        RuntimeHostExecutionInput {
                            port_id: TOP_K_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(0),
                        },
                        RuntimeHostExecutionInput {
                            port_id: MAX_NEW_TOKENS_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(128),
                        },
                        RuntimeHostExecutionInput {
                            port_id: SYSTEM_PROMPT_PORT.into(),
                            value: RuntimeHostExecutionInputValue::String(
                                "  image prompt\n".into(),
                            ),
                        },
                    ]);
                }
                let validated =
                    ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
                let package_facts = text_package_facts(&validated);
                let directory = tempfile::tempdir().unwrap();
                let target = text_load_target(&package_facts, &directory);
                let projection = project_runtime_host_text_generation(
                    &validated,
                    package_facts.clone(),
                    target.clone(),
                )
                .unwrap();
                let options = projection.request().generation_options.as_ref();
                assert_eq!(
                    options.is_some(),
                    temperature.is_some() || companion_controls
                );
                assert_eq!(
                    options.and_then(|options| options.sampling.temperature),
                    temperature
                );
                let backend = TextBackend::default();
                let recorded = backend.requests.clone();
                let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                    Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                    Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
                );
                let cancellation = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
                let response = port
                    .execute_runtime_host_request(request, cancellation)
                    .await
                    .unwrap();
                assert_eq!(response.state, RuntimeHostExecutionState::Completed);
                let recorded = recorded.lock().unwrap();
                assert_eq!(recorded.len(), 1);
                let json = &recorded[0];
                assert_eq!(
                    json.get("temperature"),
                    temperature
                        .map(|value| serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&value).unwrap()
                        )
                        .unwrap())
                        .as_ref()
                );
                assert_eq!(
                    json.get("top_k"),
                    companion_controls.then_some(serde_json::json!(0)).as_ref()
                );
                assert_eq!(
                    json.get("max_tokens"),
                    companion_controls
                        .then_some(serde_json::json!(128))
                        .as_ref()
                );
                if companion_controls {
                    assert_eq!(
                        json["messages"][0]["content"][0]["text"],
                        "  image prompt\n"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn top_p_reaches_recording_gateway_with_optional_existing_controls() {
        for top_p in [None, Some(0.0), Some(0.7), Some(1.0)] {
            for companion_controls in [false, true] {
                let mut request = text_request_fixture();
                if let Some(value) = top_p {
                    request.materialized_inputs.push(RuntimeHostExecutionInput {
                        port_id: TOP_P_PORT.into(),
                        value: RuntimeHostExecutionInputValue::F64(
                            serde_json::Number::from_f64(f64::from(value)).unwrap(),
                        ),
                    });
                }
                if companion_controls {
                    request.materialized_inputs.extend([
                        RuntimeHostExecutionInput {
                            port_id: TEMPERATURE_PORT.into(),
                            value: RuntimeHostExecutionInputValue::F64(
                                serde_json::Number::from_f64(0.0).unwrap(),
                            ),
                        },
                        RuntimeHostExecutionInput {
                            port_id: TOP_K_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(0),
                        },
                        RuntimeHostExecutionInput {
                            port_id: MAX_NEW_TOKENS_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(128),
                        },
                        RuntimeHostExecutionInput {
                            port_id: SYSTEM_PROMPT_PORT.into(),
                            value: RuntimeHostExecutionInputValue::String(
                                "  image prompt\n".into(),
                            ),
                        },
                    ]);
                }
                let validated =
                    ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
                let package_facts = text_package_facts(&validated);
                let directory = tempfile::tempdir().unwrap();
                let target = text_load_target(&package_facts, &directory);
                let projection = project_runtime_host_text_generation(
                    &validated,
                    package_facts.clone(),
                    target.clone(),
                )
                .unwrap();
                let options = projection.request().generation_options.as_ref();
                assert_eq!(options.is_some(), top_p.is_some() || companion_controls);
                assert_eq!(options.and_then(|options| options.sampling.top_p), top_p);
                assert_eq!(
                    options.and_then(|options| options.sampling.temperature),
                    companion_controls.then_some(0.0)
                );
                let backend = TextBackend::default();
                let recorded = backend.requests.clone();
                let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                    Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                    Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
                );
                let cancellation = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
                let response = port
                    .execute_runtime_host_request(request, cancellation)
                    .await
                    .unwrap();
                assert_eq!(response.state, RuntimeHostExecutionState::Completed);
                let recorded = recorded.lock().unwrap();
                assert_eq!(recorded.len(), 1);
                let json = &recorded[0];
                assert_eq!(
                    json.get("top_p"),
                    top_p
                        .map(|value| serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&value).unwrap()
                        )
                        .unwrap())
                        .as_ref()
                );
                assert_eq!(
                    json.get("temperature"),
                    companion_controls
                        .then_some(serde_json::json!(0.0))
                        .as_ref()
                );
                assert_eq!(
                    json.get("top_k"),
                    companion_controls.then_some(serde_json::json!(0)).as_ref()
                );
                assert_eq!(
                    json.get("max_tokens"),
                    companion_controls
                        .then_some(serde_json::json!(128))
                        .as_ref()
                );
                if companion_controls {
                    assert_eq!(
                        json["messages"][0]["content"][0]["text"],
                        "  image prompt\n"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn repetition_penalty_reaches_recording_gateway_with_optional_existing_controls() {
        for repetition_penalty in [None, Some(0.5), Some(1.0), Some(1.2), Some(f32::MAX)] {
            for companion_controls in [false, true] {
                let mut request = text_request_fixture();
                if let Some(value) = repetition_penalty {
                    request.materialized_inputs.push(RuntimeHostExecutionInput {
                        port_id: REPETITION_PENALTY_PORT.into(),
                        value: RuntimeHostExecutionInputValue::F64(
                            serde_json::Number::from_f64(f64::from(value)).unwrap(),
                        ),
                    });
                }
                if companion_controls {
                    request.materialized_inputs.extend([
                        RuntimeHostExecutionInput {
                            port_id: TEMPERATURE_PORT.into(),
                            value: RuntimeHostExecutionInputValue::F64(
                                serde_json::Number::from_f64(0.0).unwrap(),
                            ),
                        },
                        RuntimeHostExecutionInput {
                            port_id: TOP_K_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(0),
                        },
                        RuntimeHostExecutionInput {
                            port_id: MAX_NEW_TOKENS_PORT.into(),
                            value: RuntimeHostExecutionInputValue::U64(128),
                        },
                        RuntimeHostExecutionInput {
                            port_id: SYSTEM_PROMPT_PORT.into(),
                            value: RuntimeHostExecutionInputValue::String(
                                "  image prompt\n".into(),
                            ),
                        },
                    ]);
                }
                let validated =
                    ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
                let package_facts = text_package_facts(&validated);
                let directory = tempfile::tempdir().unwrap();
                let target = text_load_target(&package_facts, &directory);
                let projection = project_runtime_host_text_generation(
                    &validated,
                    package_facts.clone(),
                    target.clone(),
                )
                .unwrap();
                let options = projection.request().generation_options.as_ref();
                assert_eq!(
                    options.is_some(),
                    repetition_penalty.is_some() || companion_controls
                );
                assert_eq!(
                    options.and_then(|options| options.sampling.repetition_penalty),
                    repetition_penalty
                );
                assert_eq!(
                    options.and_then(|options| options.sampling.temperature),
                    companion_controls.then_some(0.0)
                );
                let backend = TextBackend::default();
                let recorded = backend.requests.clone();
                let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                    Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                    Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
                );
                let cancellation = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
                let response = port
                    .execute_runtime_host_request(request, cancellation)
                    .await
                    .unwrap();
                assert_eq!(response.state, RuntimeHostExecutionState::Completed);
                let recorded = recorded.lock().unwrap();
                assert_eq!(recorded.len(), 1);
                let json = &recorded[0];
                assert_eq!(
                    json.get("repetition_penalty"),
                    repetition_penalty
                        .map(|value| serde_json::from_str::<serde_json::Value>(
                            &serde_json::to_string(&value).unwrap()
                        )
                        .unwrap())
                        .as_ref()
                );
                assert_eq!(
                    json.get("temperature"),
                    companion_controls
                        .then_some(serde_json::json!(0.0))
                        .as_ref()
                );
                assert_eq!(
                    json.get("top_k"),
                    companion_controls.then_some(serde_json::json!(0)).as_ref()
                );
                assert_eq!(
                    json.get("max_tokens"),
                    companion_controls
                        .then_some(serde_json::json!(128))
                        .as_ref()
                );
                if companion_controls {
                    assert_eq!(
                        json["messages"][0]["content"][0]["text"],
                        "  image prompt\n"
                    );
                }
            }
        }
    }

    #[tokio::test]
    async fn top_k_reaches_recording_gateway_alone_and_with_limit_and_system_prompt() {
        for top_k in [None, Some(0), Some(40), Some(u32::MAX)] {
            for max_new_tokens in [None, Some(128)] {
                for system_prompt in [None, Some("  image prompt instructions\n")] {
                    let mut request = text_request_fixture();
                    for (port_id, value) in
                        [(TOP_K_PORT, top_k), (MAX_NEW_TOKENS_PORT, max_new_tokens)]
                    {
                        if let Some(value) = value {
                            request.materialized_inputs.push(RuntimeHostExecutionInput {
                                port_id: port_id.into(),
                                value: RuntimeHostExecutionInputValue::U64(u64::from(value)),
                            });
                        }
                    }
                    if let Some(system_prompt) = system_prompt {
                        request.materialized_inputs.push(RuntimeHostExecutionInput {
                            port_id: SYSTEM_PROMPT_PORT.into(),
                            value: RuntimeHostExecutionInputValue::String(system_prompt.into()),
                        });
                    }
                    let validated =
                        ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
                    let package_facts = text_package_facts(&validated);
                    let directory = tempfile::tempdir().unwrap();
                    let target = text_load_target(&package_facts, &directory);
                    let projection = project_runtime_host_text_generation(
                        &validated,
                        package_facts.clone(),
                        target.clone(),
                    )
                    .unwrap();
                    let options = projection.request().generation_options.as_ref();
                    assert_eq!(
                        options.is_some(),
                        top_k.is_some() || max_new_tokens.is_some()
                    );
                    assert_eq!(options.and_then(|options| options.sampling.top_k), top_k);
                    assert_eq!(
                        options.and_then(|options| options.length.max_new_tokens),
                        max_new_tokens
                    );
                    let backend = TextBackend::default();
                    let recorded = backend.requests.clone();
                    let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                        Arc::new(TextLoadTargetResolver { target }),
                        Arc::new(TextPackageFactsResolver { package_facts }),
                        Arc::new(UnusedTextMediaSink),
                        Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
                    );
                    let cancellation = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone());
                    let response = port
                        .execute_runtime_host_request(request, cancellation)
                        .await
                        .unwrap();
                    assert_eq!(response.state, RuntimeHostExecutionState::Completed);
                    let recorded = recorded.lock().unwrap();
                    assert_eq!(recorded.len(), 1);
                    let json = &recorded[0];
                    assert_eq!(
                        json.get("top_k"),
                        top_k.map(serde_json::Value::from).as_ref()
                    );
                    assert_eq!(
                        json.get("max_tokens"),
                        max_new_tokens.map(serde_json::Value::from).as_ref()
                    );
                    let messages = json["messages"].as_array().unwrap();
                    assert_eq!(messages.len(), if system_prompt.is_some() { 2 } else { 1 });
                    if let Some(system_prompt) = system_prompt {
                        assert_eq!(messages[0]["role"], "system");
                        assert_eq!(messages[0]["content"][0]["text"], system_prompt);
                    }
                    assert_eq!(messages.last().unwrap()["role"], "user");
                    assert_eq!(
                        messages.last().unwrap()["content"][0]["text"],
                        "exact prompt"
                    );
                }
            }
        }
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
    fn optional_system_prompt_preserves_exact_strings_and_existing_byte_bounds() {
        let mut request = text_request_fixture();
        assert_eq!(optional_system_prompt(&request).unwrap(), None);
        for text in [
            String::new(),
            "  image prompt instructions\n".into(),
            "🦀".repeat(256),
        ] {
            request
                .materialized_inputs
                .retain(|input| input.port_id != SYSTEM_PROMPT_PORT);
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: SYSTEM_PROMPT_PORT.into(),
                value: RuntimeHostExecutionInputValue::String(text.clone()),
            });
            validate_runtime_host_text_generation_request(&request).unwrap();
            assert_eq!(
                optional_system_prompt(&request).unwrap(),
                Some(text.as_str())
            );
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package, &directory);
            let projection =
                project_runtime_host_text_generation(&validated, package, target).unwrap();
            assert!(matches!(&projection.request().input,
                InferenceExecutionInput::TextGeneration { system_prompt: Some(actual), .. }
                    if actual == &text));
        }
        let input = request
            .materialized_inputs
            .iter_mut()
            .find(|input| input.port_id == SYSTEM_PROMPT_PORT)
            .unwrap();
        input.value = RuntimeHostExecutionInputValue::String("🦀".repeat(257));
        assert_eq!(
            validate_runtime_host_text_generation_request(&request),
            Err(RuntimeHostTextGenerationProjectionError::InputTooLong { bytes: 1028 })
        );
        request
            .materialized_inputs
            .iter_mut()
            .find(|input| input.port_id == SYSTEM_PROMPT_PORT)
            .unwrap()
            .value = RuntimeHostExecutionInputValue::U64(1);
        assert_eq!(
            validate_runtime_host_text_generation_request(&request),
            Err(RuntimeHostTextGenerationProjectionError::InvalidInputType {
                port_id: SYSTEM_PROMPT_PORT,
                expected: "string",
            })
        );
    }

    #[test]
    fn rejects_unsupported_inputs_before_dependency_resolution() {
        let mut request = text_request_fixture();
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: "messages".to_string(),
            value: RuntimeHostExecutionInputValue::String("ignored".to_string()),
        });

        let error = validate_runtime_host_text_generation_request(&request)
            .expect_err("arbitrary message arrays are not host text inputs");
        assert!(matches!(
            error,
            RuntimeHostTextGenerationProjectionError::UnsupportedInputPort { port_id }
                if port_id == "messages"
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
    async fn executes_text_with_system_prompt_and_token_limit_without_calling_an_image_sink() {
        let mut request = text_request_fixture();
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: MAX_NEW_TOKENS_PORT.to_string(),
            value: RuntimeHostExecutionInputValue::I64(128),
        });
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: SYSTEM_PROMPT_PORT.into(),
            value: RuntimeHostExecutionInputValue::String(
                "  Return an image prompt only.\n".into(),
            ),
        });
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
        assert!(backend_calls
            .lock()
            .expect("backend calls")
            .iter()
            .any(|call| call == "max_tokens:128"));
        assert!(backend_calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call == "system:  Return an image prompt only.\n"));
    }

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn unconstrained_owned_cpu_candidate_reaches_selected_text_host_with_full_peak_lease() {
        exercise_owned_cpu_selected_text_host(false, false, false).await;
    }

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn host_ram_ceiling_gates_actual_cpu_selection_and_preserves_execution_custody() {
        exercise_owned_cpu_selected_text_host(true, false, false).await;
    }

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn host_ram_shrink_does_not_starve_unrelated_vram_owner_publication() {
        exercise_owned_cpu_selected_text_host(true, true, false).await;
    }

    #[cfg(feature = "backend-pytorch")]
    #[tokio::test]
    async fn service_timing_capture_reaches_actual_selected_text_host_with_peak_custody() {
        exercise_owned_cpu_selected_text_host(false, false, true).await;
    }

    #[cfg(feature = "backend-pytorch")]
    async fn exercise_owned_cpu_selected_text_host(
        clamp_capacity: bool,
        unrelated_vram_owner: bool,
        record_timing: bool,
    ) {
        use crate::pumas_dispatch_package_facts::{
            PumasDispatchPackageFactsBridgeOutcome, PumasDispatchPackageFactsProjection,
        };
        use crate::runtime_dispatch_candidate_provider::EmbeddedRuntimeDispatchCandidateProvider;
        use crate::runtime_dispatch_capability_facts::RuntimeDispatchCapabilityFactsSource;
        use crate::runtime_dispatch_load_target_facts::{
            RuntimeDispatchLoadTargetFact, RuntimeDispatchLoadTargetFactsOutcome,
            RuntimeDispatchLoadTargetFactsProjection,
        };
        use crate::runtime_dispatch_resource_facts::RuntimeDispatchResourceFactsSource;
        use crate::runtime_dispatch_source_snapshot::EmbeddedRuntimeDispatchCandidateSourceSnapshot;
        use pantograph_runtime_registry::{
            RuntimeAdmissionBudget, RuntimeAdmissionResourceBudget, RuntimeDispatchIdentity,
            RuntimeRegistration, RuntimeRegistry, RuntimeTransition,
        };
        use pantograph_scheduler::{
            SchedulerDispatchSelectionRequest, SchedulerTaskExecutionIntent, SchedulerTaskState,
            SchedulerTaskStateRecord, ValidatedSchedulerDispatchSelectionRequest,
        };
        use pantograph_workflow_service::workflow::WorkflowRuntimeDispatchCandidateProvider;
        use pantograph_workflow_service::{
            WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
        };

        const PEAK_BYTES: u64 = 3 * 1024 * 1024;
        let mut request = text_request_fixture();
        let timing_request_id = if record_timing {
            request.execution_request_id = format!(
                "{}host-padded-timing-id{}",
                " ".repeat(512 * 1024),
                " ".repeat(512 * 1024)
            );
            Some(request.execution_request_id.clone())
        } else {
            None
        };
        request.handoff.task_intent.constraints.requested_device_id = None;
        request.handoff.task_intent.estimate_hints =
            vec![pantograph_scheduler::SchedulerEstimateHint {
                kind: pantograph_scheduler::SchedulerEstimateHintKind::PeakRamBytes,
                value: PEAK_BYTES,
            }];
        request
            .handoff
            .readiness_proof
            .preflight_result
            .identity_key
            .scheduler_intent
            .requested_device_id = None;
        let intent = request.handoff.task_intent.clone();
        let proof = request.handoff.readiness_proof.clone();
        // Keep the reusable package fixture valid while preparing a new selection.
        let previous = request.handoff.dispatch_decision.as_mut().unwrap();
        previous.task_intent = intent.clone();
        previous.readiness_proof = proof.clone();
        let validated = ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
        let package = text_package_facts(&validated);
        let directory = tempfile::tempdir().unwrap();
        let mut target = text_load_target(&package, &directory);
        let mut backend = TextBackend::default();
        if record_timing {
            target.content_fingerprint = Some("controlled-host-content-v1".into());
            backend.timing_facts = Some(inference::RuntimeServiceTimingOwnerFacts {
                implementation_fingerprint: "controlled-host-implementation-v1".into(),
                effective_configuration_fingerprint: "controlled-host-config-v1".into(),
                physical_device_fingerprint: "controlled-host-device-v1".into(),
                device_id: "cpu".parse().unwrap(),
            });
        }
        let timing_rows = Arc::new(Mutex::new(Vec::new()));
        struct HostTimingRecorder(Arc<Mutex<Vec<inference::RuntimeServiceTimingAttempt>>>);
        impl inference::RuntimeServiceTimingRecorder for HostTimingRecorder {
            fn try_record(&self, attempt: inference::RuntimeServiceTimingAttempt) -> bool {
                let Ok(mut rows) = self.0.try_lock() else {
                    return false;
                };
                if rows.len() == 4 {
                    return false;
                }
                rows.push(attempt);
                true
            }
        }
        let calls = backend.calls.clone();
        let mut gateway = inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch");
        if record_timing {
            gateway = gateway.with_runtime_service_timing_recorder(Arc::new(HostTimingRecorder(
                timing_rows.clone(),
            )));
        }
        let gateway = Arc::new(gateway);
        let registry = Arc::new(RuntimeRegistry::new());
        registry.register_runtime(
            RuntimeRegistration::new("pytorch", "PyTorch")
                .with_backend_keys(vec!["pytorch".into(), "transformers".into()])
                .with_dispatch_identity(
                    RuntimeDispatchIdentity::new(
                        "transformers",
                        "runtime.transformers.pytorch.shared",
                    )
                    .unwrap(),
                )
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(vec![
                    RuntimeAdmissionResourceBudget::ram_bytes(Some(8 * 1024 * 1024)),
                ])),
        );
        registry
            .transition_runtime(
                "pytorch",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("runtime.pytorch.cpu.001".into()),
                },
            )
            .unwrap();
        #[derive(Debug)]
        struct ControlledCapacity(std::sync::atomic::AtomicU64);
        impl pantograph_runtime_registry::RuntimeHostRamCapacitySource for ControlledCapacity {
            fn capacity_ceiling_bytes(&self) -> Option<u64> {
                Some(self.0.load(std::sync::atomic::Ordering::SeqCst))
            }
        }
        let capacity = Arc::new(ControlledCapacity(std::sync::atomic::AtomicU64::new(
            PEAK_BYTES - 1,
        )));
        if clamp_capacity {
            registry
                .configure_resource_domain(pantograph_runtime_registry::RuntimeResourceDomain {
                    domain_id: "host.ram".into(),
                    total_bytes: 8 * 1024 * 1024,
                    safety_margin_bytes: 0,
                    bindings: vec![pantograph_runtime_registry::RuntimeResourceDomainBinding {
                        runtime_id: "pytorch".into(),
                        resource_kind:
                            pantograph_runtime_registry::RuntimeAdmissionResourceKind::RamBytes,
                    }],
                })
                .unwrap();
            registry.bind_host_ram_capacity_source(capacity.clone());
        }
        let capabilities = RuntimeDispatchCapabilityFactsSource::new(registry.clone())
            .with_gateway(gateway.clone())
            .collect();
        assert!(
            registry.snapshot().reservations.is_empty(),
            "capability reads acquire no custody"
        );
        let snapshot = EmbeddedRuntimeDispatchCandidateSourceSnapshot {
            runtime_capability_facts: Some(capabilities),
            pumas_package_facts: Some(PumasDispatchPackageFactsBridgeOutcome::Projected {
                facts: Box::new(PumasDispatchPackageFactsProjection {
                    model_ref: intent.model_ref.clone(),
                    artifact_kind: package.artifact.artifact_kind.clone(),
                    validation_state: package.artifact.validation_state.clone(),
                    task: package.task.clone(),
                    backend_hints: package.backend_hints.clone(),
                    requires_custom_code: false,
                    logical_size: inference::PackageLogicalSizeFacts {
                        total_size_bytes: Some(4096),
                        value_source: inference::PackageFactValueSource::FilesystemMetadata,
                        files: Vec::new(),
                        diagnostics: Vec::new(),
                    },
                    diffusers: None,
                }),
                diagnostics: Vec::new(),
            }),
            pumas_load_target_facts: Some(RuntimeDispatchLoadTargetFactsOutcome::Projected {
                facts: RuntimeDispatchLoadTargetFactsProjection {
                    load_targets: vec![RuntimeDispatchLoadTargetFact {
                        runtime_family: "transformers".into(),
                        resolved_load_target: target.local_load_path.clone(),
                        model_ref: intent.model_ref.clone(),
                        artifact_kind: "HfCompatibleDirectory".into(),
                        load_path_kind: "Directory".into(),
                        library_root_id: target.library_root_id.clone(),
                        storage_kind: "LibraryOwned".into(),
                        validation_state: "Valid".into(),
                        content_fingerprint: None,
                        package_facts_contract_version: Some(
                            package.package_facts_contract_version,
                        ),
                    }],
                },
                diagnostics: Vec::new(),
            }),
            ..Default::default()
        };
        let provider = EmbeddedRuntimeDispatchCandidateProvider::with_source_snapshot(snapshot)
            .with_resource_facts_source(RuntimeDispatchResourceFactsSource::new(registry.clone()));
        let task = WorkflowSchedulerTask {
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            node_type: "inference".into(),
            execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            dependency_task_ids: Vec::new(),
            input_bindings: Vec::new(),
            schedulable_intent: Some(intent.clone()),
            schedulable_intent_template: None,
            non_runtime_task_template: None,
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
            diagnostics: Vec::new(),
        };
        let ready = SchedulerTaskStateRecord {
            contract_version: 1,
            workflow_id: intent.workflow_id.clone(),
            workflow_run_id: intent.workflow_run_id.clone(),
            node_id: intent.node_id.clone(),
            task_id: intent.task_id.clone(),
            state: SchedulerTaskState::Ready {
                execution_intent: SchedulerTaskExecutionIntent::runtime(intent.clone()),
            },
            state_version: 1,
            last_transition_id: "transition.ready.cpu".parse().unwrap(),
        };
        if clamp_capacity {
            let blocked = provider
                .runtime_dispatch_candidates(&task, &ready, &proof)
                .unwrap();
            assert!(blocked.candidates.is_empty(), "{:?}", blocked.diagnostics);
            assert!(registry.snapshot().reservations.is_empty());
            assert!(
                calls.lock().unwrap().is_empty(),
                "insufficient RAM cannot load a backend"
            );
            capacity
                .0
                .store(PEAK_BYTES, std::sync::atomic::Ordering::SeqCst);
        }
        let result = provider
            .runtime_dispatch_candidates(&task, &ready, &proof)
            .unwrap();
        assert_eq!(result.candidates.len(), 1, "{:?}", result.diagnostics);
        let candidate = &result.candidates[0];
        assert_eq!(candidate.selected_device_ids[0].as_str(), "cpu");
        assert_eq!(
            candidate
                .selected_runtime_variant_id
                .as_ref()
                .unwrap()
                .as_str(),
            "pytorch.cpu"
        );
        assert_eq!(
            candidate
                .reservations
                .iter()
                .map(|claim| claim.reserved_bytes)
                .sum::<u64>(),
            PEAK_BYTES
        );
        assert_eq!(registry.snapshot().reservations.len(), 1);
        let selected_request = ValidatedSchedulerDispatchSelectionRequest::try_from(
            SchedulerDispatchSelectionRequest {
                contract_version: 1,
                task_intent: intent,
                readiness_proof: proof.clone(),
                environment_ref: proof.preflight_result.environment_ref.clone().unwrap(),
                candidates: result.candidates.clone(),
                diagnostics: result.diagnostics.clone(),
            },
        )
        .unwrap();
        request.handoff.dispatch_decision =
            pantograph_scheduler::select_scheduler_dispatch(selected_request)
                .unwrap()
                .into_inner()
                .dispatch_decision;
        assert!(request.handoff.dispatch_decision.is_some());
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts: package }), Arc::new(UnusedTextMediaSink), gateway);
        let cancellation =
            pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                request.cancellation_context.clone(),
            );
        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .unwrap();
        assert_eq!(
            response.state,
            RuntimeHostExecutionState::Completed,
            "{:?}",
            response.diagnostics
        );
        assert_eq!(
            response.outputs[0].value,
            RuntimeHostExecutionOutputValue::String("generated text".into())
        );
        assert!(calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call.starts_with("load:") && call.ends_with(":cpu")));
        assert_eq!(
            registry.snapshot().reservations.len(),
            1,
            "execution cannot silently drop provisional peak coverage"
        );
        if record_timing {
            let rows = timing_rows.lock().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                &response.execution_request_id,
                timing_request_id.as_ref().unwrap()
            );
            assert_eq!(
                rows[0].execution_request_id_digest.as_ref().unwrap().len(),
                64
            );
            assert!(serde_json::to_string(&rows[0]).unwrap().len() < 4096);
            let inference::RuntimeServiceTimingIdentity::Exact { profile } = &rows[0].identity
            else {
                panic!("{:?}", rows[0].identity);
            };
            assert_eq!(
                rows[0].outcome,
                inference::RuntimeServiceTimingOutcome::Completed
            );
            for phase in [
                inference::RuntimeServiceTimingPhase::GatewayCustodyWait,
                inference::RuntimeServiceTimingPhase::SelectedModelLoad,
                inference::RuntimeServiceTimingPhase::TextExecution,
                inference::RuntimeServiceTimingPhase::WorkerCleanup,
            ] {
                assert!(rows[0].observed_completed_ns(profile, phase).is_some());
            }
            assert_eq!(
                registry.snapshot().runtimes[0].active_reservation_claims[0].claims[0].bytes,
                PEAK_BYTES
            );
        }
        if clamp_capacity {
            capacity.0.store(1, std::sync::atomic::Ordering::SeqCst);
            if unrelated_vram_owner {
                registry.register_runtime(RuntimeRegistration::new("candle", "Candle"));
                registry.configure_resource_domain(pantograph_runtime_registry::RuntimeResourceDomain {
                    domain_id: "controlled.candle.vram".into(), total_bytes: 100, safety_margin_bytes: 0,
                    bindings: vec![pantograph_runtime_registry::RuntimeResourceDomainBinding {
                        runtime_id: "candle".into(), resource_kind: pantograph_runtime_registry::RuntimeAdmissionResourceKind::VramBytes,
                    }],
                }).unwrap();
                registry.observe_runtime(pantograph_runtime_registry::RuntimeObservation {
                    runtime_id: "candle".into(),
                    display_name: "Candle".into(),
                    backend_keys: vec!["candle".into()],
                    model_id: Some("controlled-candle-model".into()),
                    runtime_instance_id: Some("controlled-candle-generation".into()),
                    status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
                    last_error: None,
                });
                registry.declare_model_residency_resources("candle", "controlled-candle-model", "controlled-candle-generation", pantograph_runtime_registry::RuntimeReservationRequirements::from_claims(vec![pantograph_runtime_registry::RuntimeReservationResourceClaim::vram_bytes(10)])).unwrap();
                let vram_request = pantograph_runtime_registry::RuntimeReservationRequest {
                    runtime_id: "candle".into(), workflow_id: "controlled-vram".into(), reservation_owner_id: Some("controlled-candle-task".into()),
                    usage_profile: None, model_id: None, pin_runtime: false, retention_hint: pantograph_runtime_registry::RuntimeRetentionHint::Ephemeral,
                    requirements: Some(pantograph_runtime_registry::RuntimeReservationRequirements::from_claims(vec![pantograph_runtime_registry::RuntimeReservationResourceClaim::vram_bytes(20)])),
                };
                let observation = registry.evaluate_reservation(vram_request.clone()).unwrap();
                let pool = &observation.observation().resource_domains[0];
                assert_eq!(
                    (
                        pool.resident_bytes,
                        pool.reserved_bytes,
                        pool.available_bytes
                    ),
                    (10, 10, 90)
                );
                let vram_lease = registry.acquire_reservation(vram_request).unwrap();
                assert_eq!(registry.snapshot().reservations.len(), 2);
                registry
                    .release_reservation(vram_lease.reservation_id)
                    .unwrap();
            }
            let probe = pantograph_runtime_registry::RuntimeReservationRequest {
                runtime_id: "pytorch".into(),
                workflow_id: "probe".into(),
                reservation_owner_id: Some("another-task".into()),
                usage_profile: None,
                model_id: None,
                pin_runtime: false,
                retention_hint: pantograph_runtime_registry::RuntimeRetentionHint::Ephemeral,
                requirements: Some(
                    pantograph_runtime_registry::RuntimeReservationRequirements::from_claims(vec![
                        pantograph_runtime_registry::RuntimeReservationResourceClaim::ram_bytes(1),
                    ]),
                ),
            };
            assert!(registry.evaluate_reservation(probe).is_err());
            assert_eq!(
                registry
                    .snapshot()
                    .runtimes
                    .iter()
                    .find(|runtime| runtime.runtime_id == "pytorch")
                    .unwrap()
                    .active_reservation_claims[0]
                    .claims[0]
                    .bytes,
                PEAK_BYTES
            );
        }
        drop(result);
        assert!(
            registry.snapshot().reservations.is_empty(),
            "untransferred test custody rolls back"
        );
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
        requests: Arc<Mutex<Vec<serde_json::Value>>>,
        cancel: Option<Arc<std::sync::atomic::AtomicBool>>,
        fail_completion: bool,
        timing_facts: Option<inference::RuntimeServiceTimingOwnerFacts>,
    }

    #[async_trait]
    impl InferenceBackend for TextBackend {
        fn runtime_service_timing_owner_facts(
            &self,
        ) -> Option<inference::RuntimeServiceTimingOwnerFacts> {
            self.timing_facts.clone()
        }
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
            self.requests.lock().unwrap().push(json.clone());
            if let Some(max_tokens) = json.get("max_tokens") {
                self.calls
                    .lock()
                    .expect("backend calls")
                    .push(format!("max_tokens:{max_tokens}"));
            }
            let messages = json["messages"].as_array().unwrap();
            if let Some(system) = messages.iter().find(|message| message["role"] == "system") {
                self.calls.lock().unwrap().push(format!(
                    "system:{}",
                    system["content"][0]["text"].as_str().unwrap()
                ));
            }
            let prompt = messages
                .iter()
                .find(|message| message["role"] == "user")
                .unwrap()["content"][0]["text"]
                .as_str()
                .unwrap();
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
                ..Default::default()
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
    #[test]
    fn seed_accepts_nonnegative_signed_inputs_without_inventing_an_omitted_seed() {
        assert!(optional_generation_options(&text_request_fixture())
            .unwrap()
            .is_none());
        for value in [0, i64::MAX] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: SEED_PORT.into(),
                value: RuntimeHostExecutionInputValue::I64(value),
            });
            assert_eq!(
                optional_generation_options(&request)
                    .unwrap()
                    .unwrap()
                    .sampling
                    .seed,
                Some(value as u64)
            );
        }
    }

    #[tokio::test]
    async fn seed_reaches_actual_host_gateway_with_u64_boundaries_and_omission() {
        for seed in [None, Some(0), Some(42), Some(u64::MAX)] {
            let mut request = text_request_fixture();
            if let Some(value) = seed {
                request.materialized_inputs.push(RuntimeHostExecutionInput {
                    port_id: SEED_PORT.into(),
                    value: RuntimeHostExecutionInputValue::U64(value),
                });
            }
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: TEMPERATURE_PORT.into(),
                value: RuntimeHostExecutionInputValue::I64(0),
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let projection = project_runtime_host_text_generation(
                &validated,
                package_facts.clone(),
                target.clone(),
            )
            .unwrap();
            assert_eq!(
                projection
                    .request()
                    .generation_options
                    .as_ref()
                    .unwrap()
                    .sampling
                    .seed,
                seed
            );
            let backend = TextBackend::default();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
            );
            let cancellation =
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                );
            let response = port
                .execute_runtime_host_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Completed);
            let recorded = recorded.lock().unwrap();
            assert_eq!(recorded.len(), 1);
            assert_eq!(
                recorded[0].get("seed"),
                seed.map(serde_json::Value::from).as_ref()
            );
            assert_eq!(recorded[0]["temperature"], serde_json::json!(0.0));
        }
    }

    #[tokio::test]
    async fn invalid_seed_fails_host_before_backend_effects() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::I64(-1),
                "seed' must be a non-negative u64 integer",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1.0).unwrap()),
                "seed' must be non-negative integer",
            ),
            (
                RuntimeHostExecutionInputValue::String("42".into()),
                "seed' must be non-negative integer",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "seed' must be non-negative integer",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: SEED_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }),
                Arc::new(TextPackageFactsResolver { package_facts }),
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
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }
    #[tokio::test]
    async fn stop_string_reaches_actual_host_gateway_exactly_with_omission_and_byte_boundary() {
        for stop in [
            None,
            Some("終わり🛑".to_owned()),
            Some("  END\n".to_owned()),
            Some(" ".to_owned()),
            Some("é".repeat(MAX_TEXT_BYTES / 2)),
        ] {
            let mut request = text_request_fixture();
            if let Some(value) = &stop {
                request.materialized_inputs.push(RuntimeHostExecutionInput {
                    port_id: STOP_PORT.into(),
                    value: RuntimeHostExecutionInputValue::String(value.clone()),
                });
            }
            // Exercise existing request JSON save/load without a new wire type.
            let request: RuntimeHostExecutionRequest =
                serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let projection = project_runtime_host_text_generation(
                &validated,
                package_facts.clone(),
                target.clone(),
            )
            .unwrap();
            assert_eq!(
                projection.request().generation_options.is_some(),
                stop.is_some()
            );
            assert_eq!(
                projection
                    .request()
                    .generation_options
                    .as_ref()
                    .map(|options| &options.stopping.stop_strings),
                stop.as_ref().map(|value| vec![value.clone()]).as_ref()
            );
            let backend = TextBackend::default();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
            );
            let cancellation =
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                );
            let response = port
                .execute_runtime_host_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Completed);
            let recorded = recorded.lock().unwrap();
            assert_eq!(recorded.len(), 1);
            assert_eq!(
                recorded[0].get("stop"),
                stop.as_ref()
                    .map(|value| serde_json::json!([value]))
                    .as_ref()
            );
        }
    }

    #[tokio::test]
    async fn invalid_stop_string_fails_host_before_backend_effects() {
        for (value, expected) in [
            (
                RuntimeHostExecutionInputValue::String(String::new()),
                "stop' must not be empty",
            ),
            (
                RuntimeHostExecutionInputValue::U64(42),
                "stop' must be string",
            ),
            (
                RuntimeHostExecutionInputValue::Bool(true),
                "stop' must be string",
            ),
            (
                RuntimeHostExecutionInputValue::F64(serde_json::Number::from_f64(1.0).unwrap()),
                "stop' must be string",
            ),
        ] {
            let mut request = text_request_fixture();
            request.materialized_inputs.push(RuntimeHostExecutionInput {
                port_id: STOP_PORT.into(),
                value,
            });
            let validated =
                ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
            let package_facts = text_package_facts(&validated);
            let directory = tempfile::tempdir().unwrap();
            let target = text_load_target(&package_facts, &directory);
            let backend = TextBackend::default();
            let calls = backend.calls.clone();
            let recorded = backend.requests.clone();
            let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
                Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
            );
            let cancellation =
                pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                );
            let response = port
                .execute_runtime_host_request(request, cancellation)
                .await
                .unwrap();
            assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(
                response
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected)),
                "{:?}",
                response.diagnostics
            );
            assert!(calls.lock().unwrap().is_empty());
            assert!(recorded.lock().unwrap().is_empty());
        }
    }
    #[tokio::test]
    async fn oversized_stop_string_fails_generic_host_contract_before_backend_effects() {
        let mut request = text_request_fixture();
        let valid_baseline =
            ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap();
        let package_facts = text_package_facts(&valid_baseline);
        let directory = tempfile::tempdir().unwrap();
        let target = text_load_target(&package_facts, &directory);
        request.materialized_inputs.push(RuntimeHostExecutionInput {
            port_id: STOP_PORT.into(),
            value: RuntimeHostExecutionInputValue::String("é".repeat(MAX_TEXT_BYTES / 2 + 1)),
        });
        // The generic materialized-input contract rejects oversized strings
        // before the task-specific text projection can run.
        assert_eq!(
            ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap_err(),
            pantograph_runtime_host_contracts::RuntimeHostExecutionContractError::FieldTooLong {
                field: "input.string",
                max_len: MAX_TEXT_BYTES,
            }
        );
        let backend = TextBackend::default();
        let calls = backend.calls.clone();
        let recorded = backend.requests.clone();
        let port = crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(TextLoadTargetResolver { target }), Arc::new(TextPackageFactsResolver { package_facts }),
            Arc::new(UnusedTextMediaSink), Arc::new(inference::InferenceGateway::with_backend(Box::new(backend), "PyTorch")),
        );
        let cancellation =
            pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                request.cancellation_context.clone(),
            );
        let error = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("input.string"), "{error}");
        assert!(error.to_string().contains("max 1024 bytes"), "{error}");
        assert!(calls.lock().unwrap().is_empty());
        assert!(recorded.lock().unwrap().is_empty());
    }
}
