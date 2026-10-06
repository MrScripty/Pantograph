//! Single-text embedding projection for the scheduler-owned host boundary.

use inference::{
    BackendExecutionDecision, BackendId, DeviceResolutionDecision, InferenceDeviceClass,
    InferenceDeviceId, InferenceDevicePolicy, InferenceExecutionInput, InferenceExecutionRequest,
    InferenceExecutionResult, InferenceTaskId, PumasArtifactLoadTarget, ResolvedModelPackageFacts,
    RuntimeVariantId,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, RuntimeHostExecutionOutput, RuntimeHostExecutionOutputValue,
    RuntimeHostExecutionRequest, ValidatedRuntimeHostExecutionRequest,
};
use thiserror::Error;

pub(crate) const EMBEDDING_TASK: &str = "embedding";

pub(crate) struct RuntimeHostEmbeddingProjection {
    pub request: InferenceExecutionRequest,
    pub target: PumasArtifactLoadTarget,
    pub decision: BackendExecutionDecision,
}

pub(crate) fn validate_runtime_host_embedding_request(
    request: &RuntimeHostExecutionRequest,
) -> Result<&str, RuntimeHostEmbeddingProjectionError> {
    if request.handoff.task_intent.task_type.as_str() != EMBEDDING_TASK {
        return Err(RuntimeHostEmbeddingProjectionError::UnsupportedTask);
    }
    let [input] = request.materialized_inputs.as_slice() else {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidInput);
    };
    let RuntimeHostExecutionInputValue::String(text) = &input.value else {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidInput);
    };
    if input.port_id != "text" || text.trim().is_empty() || text.len() > 1024 {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidInput);
    }
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?;
    if request
        .handoff
        .task_intent
        .model_ref
        .revision
        .as_ref()
        .is_some_and(|revision| selected.selected_model_ref.revision.as_ref() != Some(revision))
    {
        return Err(RuntimeHostEmbeddingProjectionError::RequestedRevisionMismatch);
    }
    if selected.selected_runtime_id.as_str() != "candle"
        || selected
            .selected_runtime_variant_id
            .as_ref()
            .map(|id| id.as_str())
            != Some("candle.cpu")
        || selected.selected_device_ids.len() != 1
        || selected.selected_device_ids[0].as_str() != "cpu"
        || !selected.task_intent.trait_settings.is_empty()
    {
        return Err(RuntimeHostEmbeddingProjectionError::UnsupportedSelection);
    }
    Ok(text)
}

pub(crate) fn project_runtime_host_embedding(
    request: &ValidatedRuntimeHostExecutionRequest,
    package: ResolvedModelPackageFacts,
    target: PumasArtifactLoadTarget,
) -> Result<RuntimeHostEmbeddingProjection, RuntimeHostEmbeddingProjectionError> {
    let request = request.as_ref();
    let text = validate_runtime_host_embedding_request(request)?.to_string();
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?;
    let model_ref =
        super::runtime_host_text_execution::project_model_ref(&selected.selected_model_ref);
    // Preserve the original request's revision constraint (including omission)
    // instead of replacing it with owner refinement before gateway validation.
    let mut requested_model_ref = model_ref.clone();
    requested_model_ref.revision = request.handoff.task_intent.model_ref.revision.clone();
    let device = InferenceDeviceId::parse("cpu")
        .map_err(|_| RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?;
    let runtime = RuntimeVariantId::parse("candle.cpu")
        .map_err(|_| RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?;
    let decision = BackendExecutionDecision {
        selected_backend_id: BackendId::parse("candle")
            .map_err(|_| RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?,
        selected_runtime_variant_id: runtime.clone(),
        selected_device_class: InferenceDeviceClass::Cpu,
        selected_device_id: Some(device.clone()),
        device_decision: DeviceResolutionDecision {
            policy: InferenceDevicePolicy::Auto,
            runtime_variant_id: runtime,
            selected_device_class: InferenceDeviceClass::Cpu,
            selected_device_id: Some(device),
            diagnostics: Vec::new(),
        },
        selected_task_id: Some(InferenceTaskId::Embedding),
        selected_model_ref: Some(model_ref.clone()),
        diagnostics: Vec::new(),
        dependency_readiness: Vec::new(),
        selection_policy_trace: None,
    };
    Ok(RuntimeHostEmbeddingProjection {
        request: InferenceExecutionRequest {
            request_id: Some(request.execution_request_id.clone()),
            task_id: InferenceTaskId::Embedding,
            model_ref: Some(requested_model_ref),
            model_name: None,
            resolved_model_package_facts: Some(package),
            input: InferenceExecutionInput::Embedding { texts: vec![text] },
            generation_options: None,
            extra_options: serde_json::Value::Null,
        },
        target,
        decision,
    })
}

pub(crate) fn embedding_outputs(
    request: &RuntimeHostExecutionRequest,
    result: InferenceExecutionResult,
) -> Result<Vec<RuntimeHostExecutionOutput>, RuntimeHostEmbeddingProjectionError> {
    let InferenceExecutionResult::Embedding {
        embeddings, usage, ..
    } = result
    else {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidResult);
    };
    let [embedding] = embeddings.as_slice() else {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidResult);
    };
    if embedding.index != Some(0)
        || embedding.token_count.is_none()
        || embedding.vector.is_empty()
        || embedding.vector.len() > 4096
        || embedding.vector.iter().any(|value| !value.is_finite())
    {
        return Err(RuntimeHostEmbeddingProjectionError::InvalidResult);
    }
    let selected = request
        .handoff
        .dispatch_decision
        .as_ref()
        .ok_or(RuntimeHostEmbeddingProjectionError::UnsupportedSelection)?;
    let mut outputs = vec![
        RuntimeHostExecutionOutput {
            port_id: "embedding".into(),
            value: RuntimeHostExecutionOutputValue::Json(serde_json::json!(embedding.vector)),
        },
        RuntimeHostExecutionOutput {
            port_id: "metadata".into(),
            value: RuntimeHostExecutionOutputValue::Json(serde_json::json!({
                "model_ref": selected.selected_model_ref,
                "vector_length": embedding.vector.len(),
                "index": embedding.index,
                "token_count": embedding.token_count,
                "runtime_variant_id": selected.selected_runtime_variant_id,
                "device_ids": selected.selected_device_ids,
            })),
        },
    ];
    if let Some(usage) = usage {
        outputs.push(RuntimeHostExecutionOutput {
            port_id: "usage".into(),
            value: RuntimeHostExecutionOutputValue::Json(
                serde_json::to_value(usage)
                    .map_err(|_| RuntimeHostEmbeddingProjectionError::InvalidResult)?,
            ),
        });
    }
    Ok(outputs)
}

#[derive(Debug, Error)]
pub(crate) enum RuntimeHostEmbeddingProjectionError {
    #[error("runtime-host embedding requires the embedding task")]
    UnsupportedTask,
    #[error("runtime-host embedding requires one nonblank text input of at most 1024 bytes")]
    InvalidInput,
    #[error("runtime-host embedding requires scheduler-selected candle/candle.cpu on cpu")]
    UnsupportedSelection,
    #[error(
        "runtime-host embedding selected revision must preserve the explicit requested revision"
    )]
    RequestedRevisionMismatch,
    #[error("runtime-host embedding requires one indexed finite vector of 1 to 4096 values with token usage")]
    InvalidResult,
}

#[cfg(all(test, feature = "backend-candle"))]
#[path = "runtime_host_embedding_execution_tests.rs"]
pub(crate) mod tests;
