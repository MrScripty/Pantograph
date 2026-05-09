use std::collections::HashMap;
use std::sync::Arc;

use futures_util::StreamExt;

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::model_dependencies::ModelRefV2;

use super::{build_extra_settings, build_model_ref_v2, infer_task_type_primary, kv_cache};

// ---------------------------------------------------------------------------
// PyTorch handlers (behind pytorch-nodes feature)
// ---------------------------------------------------------------------------

const TEXT_GENERATION_STREAM_PORT: &str = "response";

async fn pytorch_model_needs_load(model_path: &str) -> Result<bool> {
    match inference::backend::pytorch::active_loaded_model_info().await {
        Ok(info) => Ok(info.model_path != model_path),
        Err(inference::backend::BackendError::NotRunning(_)) => Ok(true),
        Err(error) => Err(NodeEngineError::ExecutionFailed(format!(
            "PyTorch loaded-model lookup failed: {}",
            error
        ))),
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct PyTorchTypedGenerationSettings {
    pub(crate) max_tokens: Option<i64>,
    pub(crate) temperature: Option<f64>,
    pub(crate) top_p: Option<f64>,
    pub(crate) top_k: Option<u32>,
}

pub(crate) fn pytorch_typed_generation_settings(
    extra_settings: &HashMap<String, serde_json::Value>,
) -> Result<PyTorchTypedGenerationSettings> {
    let mut settings = PyTorchTypedGenerationSettings::default();
    for (key, value) in extra_settings {
        match key.as_str() {
            "max_new_tokens" | "max_tokens" => {
                let value = value.as_u64().and_then(|value| i64::try_from(value).ok());
                let Some(value) = value else {
                    return Err(NodeEngineError::ExecutionFailed(
                        "PyTorch max_new_tokens must be a non-negative integer within i64 range"
                            .to_string(),
                    ));
                };
                if let Some(existing) = settings.max_tokens {
                    if existing != value {
                        return Err(NodeEngineError::ExecutionFailed(
                            "PyTorch max_new_tokens and max_tokens settings conflict".to_string(),
                        ));
                    }
                }
                settings.max_tokens = Some(value);
            }
            "temperature" => {
                let Some(value) = value.as_f64() else {
                    return Err(NodeEngineError::ExecutionFailed(
                        "PyTorch temperature must be numeric".to_string(),
                    ));
                };
                if value < 0.0 {
                    return Err(NodeEngineError::ExecutionFailed(
                        "PyTorch temperature must be non-negative".to_string(),
                    ));
                }
                settings.temperature = Some(value);
            }
            "top_k" => {
                let value = value.as_u64().and_then(|value| u32::try_from(value).ok());
                let Some(value) = value else {
                    return Err(NodeEngineError::ExecutionFailed(
                        "PyTorch top_k must be a non-negative integer within u32 range".to_string(),
                    ));
                };
                settings.top_k = Some(value);
            }
            "top_p" => {
                let Some(value) = value.as_f64() else {
                    return Err(NodeEngineError::ExecutionFailed(
                        "PyTorch top_p must be numeric".to_string(),
                    ));
                };
                settings.top_p = Some(value);
            }
            unsupported => {
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Unsupported PyTorch generation setting '{}'. Add it to typed GenerationOptions before using it.",
                    unsupported
                )));
            }
        }
    }
    Ok(settings)
}

pub(crate) async fn execute_pytorch_inference(
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    // Detect if the prompt input is a masked prompt JSON object
    let masked_prompt_json = inputs
        .get("prompt")
        .filter(|p| p.get("type").and_then(|t| t.as_str()) == Some("masked_prompt"))
        .map(|p| serde_json::to_string(p).unwrap_or_default());

    let prompt = if let Some(p_str) = inputs.get("prompt").and_then(|p| p.as_str()) {
        p_str.to_string()
    } else if let Some(p_obj) = inputs.get("prompt") {
        // For masked prompt objects, concatenate all segment texts as the plain prompt
        if let Some(segments) = p_obj.get("segments").and_then(|s| s.as_array()) {
            segments
                .iter()
                .filter_map(|seg| seg.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("")
        } else {
            return Err(NodeEngineError::ExecutionFailed(
                "Missing prompt input: not a string or masked prompt".to_string(),
            ));
        }
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Missing prompt input".to_string(),
        ));
    };

    let model_path = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?
        .to_string();

    let system_prompt = inputs
        .get("system_prompt")
        .and_then(|s| s.as_str())
        .map(|s| s.to_string());
    let extra_settings = build_extra_settings(inputs);
    let generation_settings = pytorch_typed_generation_settings(&extra_settings)?;
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .or(generation_settings.temperature)
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .or(generation_settings.max_tokens)
        .unwrap_or(512);
    let device = inputs
        .get("device")
        .and_then(|d| d.as_str())
        .unwrap_or("auto")
        .to_string();
    let model_type = inputs
        .get("model_type")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string());

    let model_name = std::path::Path::new(&model_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("pytorch-model")
        .to_string();

    // Phase 1: Check if model is already loaded, load if needed
    if pytorch_model_needs_load(&model_path).await? {
        log::info!("PyTorchInference: loading model from '{}'", model_path);
        inference::backend::pytorch::PyTorchBackend::new()
            .load_model(&model_path, &device, model_type.as_deref())
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch model load failed: {}", error))
            })?;
        log::info!("PyTorchInference: model loaded successfully");
    }

    let _restored_kv_cache = kv_cache::restore_pytorch_input_handle(
        inputs,
        extensions,
        task_id,
        execution_id,
        event_sink,
    )
    .await?;

    let top_p = inputs
        .get("top_p")
        .and_then(|v| v.as_f64())
        .or(generation_settings.top_p)
        .unwrap_or(0.95);
    let top_k = generation_settings.top_k;

    // Phase 2: Generate through the inference-owned PyTorch worker envelope.
    let response_text = if let Some(sink) = event_sink {
        let mut stream = inference::backend::pytorch::PyTorchBackend::new()
            .generate_stream_with_top_k(
                prompt.clone(),
                system_prompt.clone(),
                max_tokens,
                temperature,
                top_p,
                top_k,
                masked_prompt_json.clone(),
            );
        let mut full_response = String::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch generation error: {}", error))
            })?;
            if let Some(text) = chunk.content.filter(|text| !text.is_empty()) {
                full_response.push_str(&text);
                let _ = sink.send(crate::WorkflowEvent::task_stream(
                    task_id,
                    execution_id,
                    TEXT_GENERATION_STREAM_PORT,
                    serde_json::json!({"mode": "append", "text": text}),
                ));
            }
        }
        full_response
    } else {
        inference::backend::pytorch::PyTorchBackend::new()
            .generate_with_top_k(
                prompt.clone(),
                system_prompt.clone(),
                max_tokens,
                temperature,
                top_p,
                top_k,
                masked_prompt_json.clone(),
            )
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("PyTorch generation error: {}", error))
            })?
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    let task_type_primary = infer_task_type_primary("llm-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "pytorch",
        &model_name,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "pytorch",
                "modelId": model_name,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    let kv_cache_output = match kv_cache::capture_pytorch_output_handle(
        task_id,
        execution_id,
        extensions,
        event_sink,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "PyTorchInference: failed to capture KV cache output for '{}': {}",
                task_id,
                error
            );
            serde_json::Value::Null
        }
    };
    outputs.insert("kv_cache_out".to_string(), kv_cache_output);
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pytorch_text_generation_stream_port_uses_canonical_response_output() {
        assert_eq!(TEXT_GENERATION_STREAM_PORT, "response");
    }
}
