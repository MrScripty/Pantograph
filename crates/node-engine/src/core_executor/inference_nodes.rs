use std::collections::HashMap;
use std::sync::Arc;

use inference::{InferenceGateway, InferenceRequestLifecycleEventSink};

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::{extension_keys, ExecutorExtensions};
use crate::model_dependencies::ModelRefV2;

use super::{
    build_extra_settings, parse_reranker_documents_input, read_optional_input_bool_aliases,
    read_optional_input_string_aliases, read_optional_input_value,
};

#[cfg(feature = "inference-nodes")]
pub(crate) fn require_gateway(
    gateway: Option<&Arc<InferenceGateway>>,
) -> Result<&Arc<InferenceGateway>> {
    gateway.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "InferenceGateway not configured: requires host-specific executor".to_string(),
        )
    })
}

#[cfg(feature = "inference-nodes")]
async fn execute_typed_gateway(
    gateway: &InferenceGateway,
    request: inference::InferenceExecutionRequest,
    extensions: &ExecutorExtensions,
) -> std::result::Result<inference::InferenceExecutionResult, inference::GatewayError> {
    if let Some(sink) = inference_lifecycle_sink(extensions) {
        gateway.execute_typed_with_lifecycle(request, sink).await
    } else {
        gateway.execute_typed(request).await
    }
}

#[cfg(feature = "inference-nodes")]
fn inference_lifecycle_sink(
    extensions: &ExecutorExtensions,
) -> Option<Arc<dyn InferenceRequestLifecycleEventSink>> {
    extensions
        .get::<Arc<dyn InferenceRequestLifecycleEventSink>>(
            extension_keys::INFERENCE_LIFECYCLE_SINK,
        )
        .cloned()
}

#[cfg(feature = "inference-nodes")]
fn assign_typed_request_id(
    request: &mut inference::InferenceExecutionRequest,
    task_id: &str,
    execution_id: &str,
) {
    if request.request_id.is_none() {
        request.request_id = Some(inference_request_id(
            task_id,
            execution_id,
            request.task_id.canonical_label(),
        ));
    }
}

#[cfg(feature = "inference-nodes")]
fn inference_request_id(task_id: &str, execution_id: &str, task_label: &str) -> String {
    format!("{execution_id}:{task_id}:{task_label}")
}

/// Resolve a model path that may be a directory to the actual `.gguf` file inside.
///
/// pumas-library stores directory paths; llama.cpp needs the `.gguf` file.
#[cfg(feature = "inference-nodes")]
pub(crate) fn resolve_gguf_path(path: &str) -> Result<String> {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let gguf = std::fs::read_dir(p)
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Cannot read model directory '{}': {}",
                    path, e
                ))
            })?
            .filter_map(|entry| entry.ok())
            .find(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
            })
            .ok_or_else(|| {
                NodeEngineError::ExecutionFailed(format!(
                    "No .gguf file found in model directory '{}'",
                    path
                ))
            })?;
        Ok(gguf.path().to_string_lossy().into_owned())
    } else {
        Ok(path.to_string())
    }
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_llm_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    if event_sink.is_none() {
        let mut request = build_text_generation_execution_request(inputs)?;
        assign_typed_request_id(&mut request, task_id, execution_id);
        let result = execute_typed_gateway(gw, request, extensions)
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("Typed LLM inference failed: {error}"))
            })?;
        let (response, option_diagnostics) = match result {
            inference::InferenceExecutionResult::TextGeneration {
                text,
                option_diagnostics,
                ..
            } => (text, option_diagnostics),
            other => {
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Typed LLM inference returned unexpected result: {other:?}"
                )));
            }
        };

        let mut outputs = HashMap::new();
        outputs.insert("response".to_string(), serde_json::json!(response));
        outputs.insert("stream".to_string(), serde_json::Value::Null);
        outputs.insert(
            "diagnostics".to_string(),
            serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
        );
        return Ok(outputs);
    }

    let response = if let Some(sink) = event_sink {
        let mut request = build_text_generation_execution_request(inputs)?;
        assign_typed_request_id(&mut request, task_id, execution_id);
        if let inference::InferenceExecutionInput::TextGeneration { stream, .. } =
            &mut request.input
        {
            *stream = true;
        }
        let mut token_stream = if let Some(lifecycle_sink) = inference_lifecycle_sink(extensions) {
            gw.stream_typed_text_with_lifecycle(request, lifecycle_sink)
                .await
        } else {
            gw.stream_typed_text(request).await
        }
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("LLM request failed: {error}"))
        })?;

        let mut full_response = String::new();
        while let Some(chunk_result) = token_stream.next().await {
            let chunk = chunk_result.map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {error}"))
            })?;
            if let Some(token) = chunk.content.filter(|token| !token.is_empty()) {
                full_response.push_str(&token);
                let _ = sink.send(crate::WorkflowEvent::task_stream(
                    task_id,
                    execution_id,
                    "response",
                    serde_json::json!(token),
                ));
            }
            if chunk.done {
                break;
            }
        }

        full_response
    } else {
        unreachable!("non-streaming typed inference returns before streaming request construction")
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response));
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn build_text_generation_execution_request(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceExecutionRequest> {
    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;
    let system_prompt = inputs
        .get("system_prompt")
        .and_then(|p| p.as_str())
        .map(str::to_string);
    let extra_context = inputs.get("context").and_then(|c| c.as_str());
    let full_prompt = if let Some(ctx) = extra_context {
        format!("{prompt}\n\nContext:\n{ctx}")
    } else {
        prompt.to_string()
    };

    let generation_options = read_optional_input_value(inputs, "generation_options")
        .map(serde_json::from_value::<inference::GenerationOptions>)
        .transpose()
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Invalid generation_options input: {error}"))
        })?;

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: text_generation_task_id(inputs)?,
        model_ref: parse_pumas_model_ref(inputs),
        model_name: read_optional_input_string_aliases(
            inputs,
            &["model_name", "modelName", "model", "model_id", "modelId"],
        ),
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        input: inference::InferenceExecutionInput::TextGeneration {
            prompt: Some(full_prompt),
            system_prompt,
            messages: Vec::new(),
            stream: false,
        },
        generation_options,
        extra_options: serde_json::Value::Null,
    })
}

#[cfg(feature = "inference-nodes")]
fn text_generation_task_id(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceTaskId> {
    let Some(task_label) =
        read_optional_input_string_aliases(inputs, &["task_kind", "taskKind", "task_id", "taskId"])
    else {
        return Ok(inference::InferenceTaskId::TextGeneration);
    };

    let Some(entry) = inference::resolve_task_registry_entry(&task_label) else {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Unsupported text generation task_kind '{task_label}'"
        )));
    };

    match entry.task_id {
        inference::InferenceTaskId::TextGeneration | inference::InferenceTaskId::ChatCompletion => {
            Ok(entry.task_id)
        }
        task_id => Err(NodeEngineError::ExecutionFailed(format!(
            "task_kind '{}' resolves to '{}' and cannot be executed by the text generation node",
            task_label,
            task_id.canonical_label()
        ))),
    }
}

#[cfg(feature = "inference-nodes")]
fn parse_pumas_model_ref(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<inference::PumasModelRef> {
    inputs
        .get("pumas_model_ref")
        .or_else(|| inputs.get("model_ref"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_embedding_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;
    let mut request = build_embedding_execution_request(inputs)?;
    assign_typed_request_id(&mut request, task_id, execution_id);
    let model_name = request.model_name.clone();
    let start = std::time::Instant::now();
    let result = execute_typed_gateway(gw, request, extensions)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Typed embedding inference failed: {error}"))
        })?;
    let embeddings = match result {
        inference::InferenceExecutionResult::Embedding { embeddings, .. } => embeddings,
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Typed embedding inference returned unexpected result: {other:?}"
            )));
        }
    };
    let embedding = embeddings.first().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Typed embedding inference returned no vectors for input text".to_string(),
        )
    })?;
    if embedding.vector.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Typed embedding inference returned an empty vector".to_string(),
        ));
    }
    if embedding.vector.iter().any(|value| !value.is_finite()) {
        return Err(NodeEngineError::ExecutionFailed(
            "Typed embedding inference returned invalid vector values".to_string(),
        ));
    }

    let mut outputs = HashMap::new();
    outputs.insert("embedding".to_string(), serde_json::json!(embedding.vector));
    let emit_metadata =
        super::read_optional_input_bool_aliases(inputs, &["emit_metadata", "emitMetadata"])
            .unwrap_or(false);
    if emit_metadata {
        outputs.insert(
            "metadata".to_string(),
            serde_json::json!({
                "model": model_name.unwrap_or_else(|| "default".to_string()),
                "vector_length": embedding.vector.len(),
                "duration_ms": start.elapsed().as_millis(),
            }),
        );
    }

    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn build_embedding_execution_request(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceExecutionRequest> {
    let text = inputs
        .get("text")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing text input".to_string()))?;
    if text.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Embedding input text cannot be empty".to_string(),
        ));
    }

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: inference::InferenceTaskId::Embedding,
        model_ref: parse_pumas_model_ref(inputs),
        model_name: read_optional_input_string_aliases(
            inputs,
            &["model", "model_name", "modelName", "model_id", "modelId"],
        )
        .filter(|model| !model.trim().is_empty()),
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        input: inference::InferenceExecutionInput::Embedding {
            texts: vec![text.to_string()],
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    })
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_rerank_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;
    let mut request = build_rerank_execution_request(inputs)?;
    assign_typed_request_id(&mut request, task_id, execution_id);
    let output_model_ref = request.model_ref.clone();
    let output_model = request
        .model_name
        .clone()
        .or_else(|| {
            output_model_ref
                .as_ref()
                .map(|model_ref| model_ref.model_id.clone())
        })
        .unwrap_or_default();
    let result = execute_typed_gateway(gw, request, extensions)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Typed rerank inference failed: {error}"))
        })?;
    let response = match result {
        inference::InferenceExecutionResult::Rerank { response } => response,
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Typed rerank inference returned unexpected result: {other:?}"
            )));
        }
    };

    let scores = response
        .results
        .iter()
        .map(|result| serde_json::json!(result.score))
        .collect::<Vec<_>>();
    let top_document = response
        .results
        .first()
        .and_then(|result| result.document.clone());
    let top_score = response.results.first().map(|result| result.score);

    let mut outputs = HashMap::new();
    outputs.insert(
        "results".to_string(),
        serde_json::to_value(&response.results).unwrap_or(serde_json::Value::Null),
    );
    outputs.insert("scores".to_string(), serde_json::json!(scores));
    outputs.insert(
        "model_path".to_string(),
        serde_json::json!(output_model.clone()),
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "contractVersion": 2,
            "engine": "typed",
            "modelId": output_model,
            "modelPath": output_model,
            "pumasModelRef": output_model_ref,
            "taskTypePrimary": "reranking"
        }),
    );
    outputs.insert(
        "top_document".to_string(),
        top_document
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "top_score".to_string(),
        top_score
            .map(|value| serde_json::json!(value))
            .unwrap_or(serde_json::Value::Null),
    );
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn build_rerank_execution_request(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceExecutionRequest> {
    let query = inputs
        .get("query")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;
    if query.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker query cannot be empty".to_string(),
        ));
    }

    let documents = parse_reranker_documents_input(inputs)?;
    let top_n = read_positive_usize_aliases(inputs, &["top_n", "topN", "top_k", "topK"]);
    let return_documents =
        read_optional_input_bool_aliases(inputs, &["return_documents", "returnDocuments"])
            .unwrap_or(true);
    let model_ref = parse_pumas_model_ref(inputs);
    let model_name = read_rerank_model_name(inputs, model_ref.as_ref())?;
    let mut extra_settings = build_extra_settings(inputs);
    extra_settings.remove("gpu_layers");
    extra_settings.remove("context_length");

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: inference::InferenceTaskId::Rerank,
        model_ref,
        model_name,
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        input: inference::InferenceExecutionInput::Rerank {
            query: query.to_string(),
            documents,
            top_n,
            return_documents,
        },
        generation_options: None,
        extra_options: serde_json::Value::Object(extra_settings.into_iter().collect()),
    })
}

#[cfg(feature = "inference-nodes")]
fn read_positive_usize_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<usize> {
    aliases.iter().find_map(|alias| {
        inputs.get(*alias).and_then(|value| {
            value
                .as_u64()
                .filter(|value| *value > 0)
                .map(|value| value as usize)
                .or_else(|| {
                    value
                        .as_i64()
                        .filter(|value| *value > 0)
                        .map(|value| value as usize)
                })
        })
    })
}

#[cfg(feature = "inference-nodes")]
fn read_rerank_model_name(
    inputs: &HashMap<String, serde_json::Value>,
    model_ref: Option<&inference::PumasModelRef>,
) -> Result<Option<String>> {
    if let Some(model) = read_optional_input_string_aliases(
        inputs,
        &["model", "model_name", "modelName", "model_id", "modelId"],
    )
    .filter(|model| !model.trim().is_empty())
    {
        return Ok(Some(model));
    }

    if let Some(model_path) =
        read_optional_input_string_aliases(inputs, &["model_path", "modelPath"])
            .filter(|model_path| !model_path.trim().is_empty())
    {
        return resolve_gguf_path(&model_path).map(Some);
    }

    if let Some(model_ref_value) = inputs
        .get("pumas_model_ref")
        .or_else(|| inputs.get("model_ref"))
    {
        if let Some(path) = read_string_aliases_from_value(
            model_ref_value,
            &[
                "selected_artifact_path",
                "selectedArtifactPath",
                "model_path",
                "modelPath",
                "entry_path",
                "entryPath",
            ],
        )
        .filter(|path| !path.trim().is_empty())
        {
            return resolve_gguf_path(&path).map(Some);
        }
    }

    Ok(model_ref.map(|model_ref| model_ref.model_id.clone()))
}

#[cfg(feature = "inference-nodes")]
fn read_string_aliases_from_value(value: &serde_json::Value, aliases: &[&str]) -> Option<String> {
    aliases
        .iter()
        .find_map(|alias| value.get(*alias).and_then(|value| value.as_str()))
        .map(str::to_string)
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_vision_analysis(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let image_base64 = inputs
        .get("image")
        .and_then(|i| i.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing image input".to_string()))?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "Vision server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No vision server URL available".to_string())
    })?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&serde_json::json!({
            "model": "gpt-4-vision-preview",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:image/png;base64,{}", image_base64)
                        }
                    }
                ]
            }],
            "max_tokens": 4096
        }))
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Vision request failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Vision API error: {}",
            error_text
        )));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| {
        NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
    })?;

    let analysis = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    let mut outputs = HashMap::new();
    outputs.insert("analysis".to_string(), serde_json::json!(analysis));
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_unload_model(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let model_ref_value = inputs.get("model_ref").ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "Missing model_ref input. Connect an inference node's Model Reference output."
                .to_string(),
        )
    })?;
    let model_ref =
        ModelRefV2::validate_value(model_ref_value).map_err(NodeEngineError::ExecutionFailed)?;

    let engine = model_ref.engine.as_str();
    let model_id = model_ref.model_id.as_str();

    let trigger_value = inputs
        .get("trigger")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    log::info!(
        "UnloadModel: unloading '{}' from engine '{}'",
        model_id,
        engine
    );

    match engine {
        "llamacpp" => {
            let gw = require_gateway(gateway)?;
            gw.stop().await;
            log::info!(
                "UnloadModel: llama.cpp server stopped for model '{}'",
                model_id
            );
        }
        "ollama" => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Ollama model_ref for '{model_id}' cannot be unloaded because Ollama is no longer supported as a first-party Pantograph inference backend. Migrate this workflow to canonical llm-inference with a Pumas model reference and a supported runtime."
            )));
        }
        #[cfg(feature = "pytorch-nodes")]
        "pytorch" => {
            use pyo3::types::PyAnyMethods;
            // Unload via PyO3 in-process call to the Python worker
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_torch_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload PyTorch model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: PyTorch model '{}' unloaded", model_id);
        }
        #[cfg(feature = "audio-nodes")]
        "stable_audio" => {
            use pyo3::types::PyAnyMethods;
            let model_id_owned = model_id.to_string();
            tokio::task::spawn_blocking(move || {
                pyo3::Python::with_gil(|py| {
                    if let Ok(worker) = py.import("pantograph_audio_worker") {
                        let _ = worker.call_method0("unload_model");
                    }
                });
            })
            .await
            .map_err(|e| {
                NodeEngineError::ExecutionFailed(format!(
                    "Failed to unload audio model '{}': {}",
                    model_id_owned, e
                ))
            })?;
            log::info!("UnloadModel: audio model '{}' unloaded", model_id);
        }
        "onnx-runtime" | "onnxruntime" => {
            log::info!(
                "UnloadModel: onnx-runtime model '{}' does not keep a shared runtime session",
                model_id
            );
        }
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Unknown inference engine '{}'. Supported: llamacpp, pytorch, stable_audio, onnx-runtime",
                other
            )));
        }
    }

    let status_msg = format!("Model '{}' unloaded from {}", model_id, engine);

    let mut outputs = HashMap::new();
    outputs.insert("status".to_string(), serde_json::json!(status_msg));
    outputs.insert("trigger_passthrough".to_string(), trigger_value);
    Ok(outputs)
}
