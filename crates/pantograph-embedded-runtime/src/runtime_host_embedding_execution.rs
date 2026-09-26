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

pub(crate) const EMBEDDING_TASK: &str = "embedding";
pub(crate) const EMBEDDING_TEXT_PORT: &str = "text";
pub(crate) const MAX_EMBEDDING_TEXT_BYTES: usize = 1024;

/// Owned inputs for the canonical scheduler-selected embedding call.
#[derive(Debug)]
pub(crate) struct RuntimeHostEmbeddingGenerationProjection {
    request: InferenceExecutionRequest,
    artifact_load_target: PumasArtifactLoadTarget,
    backend_decision: BackendExecutionDecision,
}

impl RuntimeHostEmbeddingGenerationProjection {
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

pub(crate) fn validate_runtime_host_embedding_generation_request(
    request: &RuntimeHostExecutionRequest,
) -> Result<(), RuntimeHostEmbeddingGenerationProjectionError> {
    if request.handoff.task_intent.task_type.as_str() != EMBEDDING_TASK {
        return Err(
            RuntimeHostEmbeddingGenerationProjectionError::UnsupportedTask {
                task_type: request.handoff.task_intent.task_type.as_str().to_string(),
            },
        );
    }
    let dispatch_decision = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostEmbeddingGenerationProjectionError::MissingDispatchDecision)?;
    let text = required_embedding_text(request)?;
    if text.trim().is_empty() {
        return Err(RuntimeHostEmbeddingGenerationProjectionError::BlankText);
    }
    if text.len() > MAX_EMBEDDING_TEXT_BYTES {
        return Err(
            RuntimeHostEmbeddingGenerationProjectionError::InputTooLong { bytes: text.len() },
        );
    }
    validate_selected_runtime_and_device(dispatch_decision)?;
    Ok(())
}

pub(crate) fn project_runtime_host_embedding_generation(
    request: &ValidatedRuntimeHostExecutionRequest,
    package_facts: ResolvedModelPackageFacts,
    load_target: pumas_library::models::PumasArtifactLoadTarget,
) -> Result<RuntimeHostEmbeddingGenerationProjection, RuntimeHostEmbeddingGenerationProjectionError>
{
    let request_ref = request.as_ref();
    validate_runtime_host_embedding_generation_request(request_ref)?;
    let dispatch_decision = request_ref
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostEmbeddingGenerationProjectionError::MissingDispatchDecision)?;
    let text = required_embedding_text(request_ref)?.to_string();
    let backend_decision = embedding_backend_decision(dispatch_decision)?;
    let artifact_load_target =
        crate::runtime_host_image_execution::project_pumas_artifact_load_target(load_target);
    let model_ref = project_model_ref(&dispatch_decision.selected_model_ref);
    let inference_request = InferenceExecutionRequest {
        request_id: Some(request_ref.execution_request_id.clone()),
        task_id: InferenceTaskId::Embedding,
        model_ref: Some(model_ref.clone()),
        model_name: Some(model_ref.model_id.clone()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::Embedding { texts: vec![text] },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    Ok(RuntimeHostEmbeddingGenerationProjection {
        request: inference_request,
        artifact_load_target,
        backend_decision,
    })
}

pub(crate) fn embedding_vector_from_inference_result(
    result: InferenceExecutionResult,
) -> Result<Vec<f32>, RuntimeHostEmbeddingGenerationProjectionError> {
    let InferenceExecutionResult::Embedding { embeddings, .. } = result else {
        return Err(
            RuntimeHostEmbeddingGenerationProjectionError::UnexpectedInferenceResult {
                result_type: "non_embedding".to_string(),
            },
        );
    };
    let [embedding] = embeddings.as_slice() else {
        return if embeddings.is_empty() {
            Err(RuntimeHostEmbeddingGenerationProjectionError::EmptyEmbeddingResult)
        } else {
            Err(
                RuntimeHostEmbeddingGenerationProjectionError::UnexpectedEmbeddingCount {
                    count: embeddings.len(),
                },
            )
        };
    };
    if embedding.vector.is_empty() {
        return Err(RuntimeHostEmbeddingGenerationProjectionError::EmptyEmbeddingResult);
    }
    if embedding.vector.iter().any(|value| !value.is_finite()) {
        return Err(RuntimeHostEmbeddingGenerationProjectionError::InvalidEmbeddingValues);
    }
    Ok(embedding.vector.clone())
}

fn required_embedding_text(
    request: &RuntimeHostExecutionRequest,
) -> Result<&str, RuntimeHostEmbeddingGenerationProjectionError> {
    let input = request
        .materialized_inputs
        .iter()
        .find(|input| input.port_id == EMBEDDING_TEXT_PORT)
        .ok_or(RuntimeHostEmbeddingGenerationProjectionError::MissingText)?;
    match &input.value {
        RuntimeHostExecutionInputValue::String(value) => Ok(value.as_str()),
        _ => Err(RuntimeHostEmbeddingGenerationProjectionError::InvalidTextInput),
    }
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

fn embedding_backend_decision(
    decision: &SchedulerDispatchDecision,
) -> Result<BackendExecutionDecision, RuntimeHostEmbeddingGenerationProjectionError> {
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
        selected_backend_id: BackendId::parse("llama_cpp").map_err(|error| {
            RuntimeHostEmbeddingGenerationProjectionError::InvalidBackendId {
                value: "llama_cpp".to_string(),
                message: error.to_string(),
            }
        })?,
        selected_runtime_variant_id,
        selected_device_class,
        selected_device_id: Some(selected_device_id),
        device_decision,
        selected_task_id: Some(InferenceTaskId::Embedding),
        selected_model_ref: Some(project_model_ref(&decision.selected_model_ref)),
        diagnostics: Vec::new(),
        dependency_readiness: Vec::new(),
        selection_policy_trace: None,
    })
}

fn validate_selected_runtime_and_device(
    decision: &SchedulerDispatchDecision,
) -> Result<(), RuntimeHostEmbeddingGenerationProjectionError> {
    if decision.selected_runtime_id.as_str() != "llama.cpp.embedding"
        && decision.selected_runtime_id.as_str() != "llama_cpp.embedding"
    {
        return Err(
            RuntimeHostEmbeddingGenerationProjectionError::UnsupportedRuntime {
                runtime_id: decision.selected_runtime_id.to_string(),
            },
        );
    }
    let _ = selected_runtime_variant_id(decision)?;
    let _ = selected_device_id(decision)?;
    Ok(())
}

fn selected_runtime_variant_id(
    decision: &SchedulerDispatchDecision,
) -> Result<RuntimeVariantId, RuntimeHostEmbeddingGenerationProjectionError> {
    let value = decision
        .selected_runtime_variant_id
        .as_ref()
        .ok_or(RuntimeHostEmbeddingGenerationProjectionError::MissingSelectedRuntimeVariant)?
        .to_string();
    let runtime_variant_id = RuntimeVariantId::parse(&value).map_err(|error| {
        RuntimeHostEmbeddingGenerationProjectionError::InvalidRuntimeVariantId {
            value: value.clone(),
            message: error.to_string(),
        }
    })?;
    if !matches!(
        runtime_variant_id.as_str(),
        "llama_cpp.cpu" | "llama_cpp.cuda" | "llama_cpp.metal"
    ) {
        return Err(
            RuntimeHostEmbeddingGenerationProjectionError::UnsupportedRuntimeVariant {
                runtime_id: runtime_variant_id.to_string(),
            },
        );
    }
    Ok(runtime_variant_id)
}

fn selected_device_id(
    decision: &SchedulerDispatchDecision,
) -> Result<InferenceDeviceId, RuntimeHostEmbeddingGenerationProjectionError> {
    let value = match decision.selected_device_ids.as_slice() {
        [] => return Err(RuntimeHostEmbeddingGenerationProjectionError::MissingSelectedDevice),
        [value] => value,
        values => {
            return Err(
                RuntimeHostEmbeddingGenerationProjectionError::AmbiguousSelectedDevices {
                    count: values.len(),
                },
            );
        }
    };
    InferenceDeviceId::parse(value.as_str()).map_err(|error| {
        RuntimeHostEmbeddingGenerationProjectionError::InvalidDeviceId {
            value: value.as_str().to_string(),
            message: error.to_string(),
        }
    })
}

fn device_class(
    device_id: &str,
) -> Result<InferenceDeviceClass, RuntimeHostEmbeddingGenerationProjectionError> {
    if device_id == "cpu" {
        return Ok(InferenceDeviceClass::Cpu);
    }
    if device_id.starts_with("cuda:") {
        return Ok(InferenceDeviceClass::Cuda);
    }
    if device_id == "mps" {
        return Ok(InferenceDeviceClass::Metal);
    }
    Err(
        RuntimeHostEmbeddingGenerationProjectionError::UnsupportedDevice {
            device_id: device_id.to_string(),
        },
    )
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeHostEmbeddingGenerationProjectionError {
    #[error("runtime-host embedding task '{task_type}' is unsupported")]
    UnsupportedTask { task_type: String },
    #[error("runtime-host embedding execution requires a scheduler dispatch decision")]
    MissingDispatchDecision,
    #[error("runtime-host embedding execution is missing its text input")]
    MissingText,
    #[error("runtime-host embedding text input must be a string")]
    InvalidTextInput,
    #[error("runtime-host embedding text input is blank")]
    BlankText,
    #[error("runtime-host embedding text input is {bytes} bytes; max is 1024")]
    InputTooLong { bytes: usize },
    #[error("runtime-host embedding execution requires a selected runtime variant")]
    MissingSelectedRuntimeVariant,
    #[error("runtime-host embedding runtime '{runtime_id}' is unsupported")]
    UnsupportedRuntime { runtime_id: String },
    #[error("runtime-host embedding runtime variant '{runtime_id}' is unsupported")]
    UnsupportedRuntimeVariant { runtime_id: String },
    #[error("runtime-host embedding execution has no selected concrete device")]
    MissingSelectedDevice,
    #[error(
        "runtime-host embedding execution has {count} selected devices; exactly one is required"
    )]
    AmbiguousSelectedDevices { count: usize },
    #[error("runtime-host embedding selected device '{value}' is invalid: {message}")]
    InvalidDeviceId { value: String, message: String },
    #[error("runtime-host embedding selected device '{device_id}' is unsupported")]
    UnsupportedDevice { device_id: String },
    #[error("runtime-host embedding runtime variant '{value}' is invalid: {message}")]
    InvalidRuntimeVariantId { value: String, message: String },
    #[error("runtime-host embedding backend id '{value}' is invalid: {message}")]
    InvalidBackendId { value: String, message: String },
    #[error("runtime-host embedding returned no vectors")]
    EmptyEmbeddingResult,
    #[error("runtime-host embedding returned {count} vectors; exactly one is required")]
    UnexpectedEmbeddingCount { count: usize },
    #[error("runtime-host embedding returned non-finite vector values")]
    InvalidEmbeddingValues,
    #[error("runtime-host embedding execution returned an unexpected result: {result_type}")]
    UnexpectedInferenceResult { result_type: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_only_finite_nonempty_embedding_vectors() {
        let result = InferenceExecutionResult::Embedding {
            embeddings: vec![inference::InferenceEmbeddingResult {
                vector: vec![0.25, -1.5, 3.0],
                token_count: Some(3),
                index: Some(0),
            }],
            usage: None,
            option_diagnostics: Vec::new(),
        };

        assert_eq!(
            embedding_vector_from_inference_result(result).expect("finite vector"),
            vec![0.25, -1.5, 3.0]
        );
    }

    #[test]
    fn rejects_empty_and_non_finite_embedding_vectors() {
        let empty = InferenceExecutionResult::Embedding {
            embeddings: Vec::new(),
            usage: None,
            option_diagnostics: Vec::new(),
        };
        assert!(matches!(
            embedding_vector_from_inference_result(empty),
            Err(RuntimeHostEmbeddingGenerationProjectionError::EmptyEmbeddingResult)
        ));

        let non_finite = InferenceExecutionResult::Embedding {
            embeddings: vec![inference::InferenceEmbeddingResult {
                vector: vec![f32::NAN],
                token_count: None,
                index: Some(0),
            }],
            usage: None,
            option_diagnostics: Vec::new(),
        };
        assert!(matches!(
            embedding_vector_from_inference_result(non_finite),
            Err(RuntimeHostEmbeddingGenerationProjectionError::InvalidEmbeddingValues)
        ));

        let extra = InferenceExecutionResult::Embedding {
            embeddings: vec![
                inference::InferenceEmbeddingResult {
                    vector: vec![1.0],
                    token_count: None,
                    index: Some(0),
                },
                inference::InferenceEmbeddingResult {
                    vector: vec![2.0],
                    token_count: None,
                    index: Some(1),
                },
            ],
            usage: None,
            option_diagnostics: Vec::new(),
        };
        assert!(matches!(
            embedding_vector_from_inference_result(extra),
            Err(
                RuntimeHostEmbeddingGenerationProjectionError::UnexpectedEmbeddingCount {
                    count: 2
                }
            )
        ));
    }
}
