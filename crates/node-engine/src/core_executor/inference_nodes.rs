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
        let expected_result_kind = expected_typed_result_kind(&request)?;
        let result = execute_typed_gateway(gw, request, extensions)
            .await
            .map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("Typed LLM inference failed: {error}"))
            })?;
        ensure_typed_result_kind(&result, expected_result_kind, "Typed LLM inference")?;
        let (response, usage, cache_handle_id, option_diagnostics) = match result {
            inference::InferenceExecutionResult::TextGeneration {
                text,
                usage,
                cache_handle_id,
                option_diagnostics,
                ..
            } => (text, usage, cache_handle_id, option_diagnostics),
            other => {
                return Err(NodeEngineError::ExecutionFailed(format!(
                    "Typed LLM inference returned unexpected result: {other:?}"
                )));
            }
        };

        let mut outputs = HashMap::new();
        outputs.insert("response".to_string(), serde_json::json!(response));
        outputs.insert("stream".to_string(), serde_json::Value::Null);
        if let Some(usage) = usage {
            outputs.insert(
                "usage".to_string(),
                serde_json::to_value(usage).unwrap_or(serde_json::Value::Null),
            );
        }
        if let Some(cache_handle_id) = cache_handle_id {
            outputs.insert(
                "kv_cache_out".to_string(),
                serde_json::json!({ "cache_id": cache_handle_id }),
            );
        }
        outputs.insert(
            "diagnostics".to_string(),
            serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
        );
        return Ok(outputs);
    }

    let (response, usage, cache_handle_id) = if let Some(sink) = event_sink {
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
        let mut usage = None;
        let mut cache_handle_id = None;
        while let Some(chunk_result) = token_stream.next().await {
            let chunk = chunk_result.map_err(|error| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {error}"))
            })?;
            if let Some(chunk_usage) = chunk.usage {
                usage = Some(chunk_usage);
            }
            if let Some(chunk_cache_handle_id) = chunk.cache_handle_id {
                cache_handle_id = Some(chunk_cache_handle_id);
            }
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

        (full_response, usage, cache_handle_id)
    } else {
        unreachable!("non-streaming typed inference returns before streaming request construction")
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response));
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    if let Some(usage) = usage {
        outputs.insert(
            "usage".to_string(),
            serde_json::to_value(usage).unwrap_or(serde_json::Value::Null),
        );
    }
    if let Some(cache_handle_id) = cache_handle_id {
        outputs.insert(
            "kv_cache_out".to_string(),
            serde_json::json!({ "cache_id": cache_handle_id }),
        );
    }
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

    let mut generation_options = read_optional_input_value(inputs, "generation_options")
        .map(normalize_generation_options_value)
        .map(serde_json::from_value::<inference::GenerationOptions>)
        .transpose()
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Invalid generation_options input: {error}"))
        })?;
    apply_graph_cache_generation_options(inputs, &mut generation_options);

    let resolved_model_package_facts = parse_resolved_model_package_facts(inputs)?;

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: text_generation_task_id(inputs)?,
        model_ref: parse_pumas_model_ref(inputs),
        model_name: read_optional_input_string_aliases(
            inputs,
            &["model_name", "modelName", "model", "model_id", "modelId"],
        ),
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        resolved_model_package_facts,
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
fn apply_graph_cache_generation_options(
    inputs: &HashMap<String, serde_json::Value>,
    generation_options: &mut Option<inference::GenerationOptions>,
) {
    if graph_cache_input_present(inputs) {
        generation_options
            .get_or_insert_with(inference::GenerationOptions::default)
            .cache
            .use_cache
            .get_or_insert(true);
    }

    if read_bool_with_task_options(
        inputs,
        &[
            "kv_cache_checkpoint_requested",
            "kvCacheCheckpointRequested",
        ],
        "kv_cache_checkpoint_requested",
    ) == Some(true)
    {
        generation_options
            .get_or_insert_with(inference::GenerationOptions::default)
            .cache
            .kv_cache_checkpoint_requested = Some(true);
    }
}

#[cfg(feature = "inference-nodes")]
fn graph_cache_input_present(inputs: &HashMap<String, serde_json::Value>) -> bool {
    read_optional_input_value(inputs, "kv_cache_in").is_some_and(|value| !value.is_null())
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn normalize_generation_options_value(
    mut value: serde_json::Value,
) -> serde_json::Value {
    let Some(options) = value.as_object_mut() else {
        return value;
    };

    if let Some(temperature) = options.get("temperature").cloned() {
        insert_nested_generation_option(options, "sampling", "temperature", temperature);
    }
    if let Some(max_new_tokens) = options.get("max_new_tokens").cloned() {
        insert_nested_generation_option(options, "length", "max_new_tokens", max_new_tokens);
    }
    value
}

#[cfg(feature = "inference-nodes")]
fn insert_nested_generation_option(
    options: &mut serde_json::Map<String, serde_json::Value>,
    group: &str,
    field: &str,
    value: serde_json::Value,
) {
    let group_value = options
        .entry(group.to_string())
        .or_insert_with(|| serde_json::json!({}));
    if let Some(group_options) = group_value.as_object_mut() {
        group_options.entry(field.to_string()).or_insert(value);
    }
}

#[cfg(feature = "inference-nodes")]
fn text_generation_task_id(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceTaskId> {
    let Some(task_label) =
        read_text_generation_task_label(inputs, &["task_kind", "taskKind", "task_id", "taskId"])?
    else {
        return Ok(inference::InferenceTaskId::TextGeneration);
    };

    let Some(entry) = inference::resolve_task_registry_entry(&task_label) else {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "Unsupported text generation task_kind '{task_label}'"
        )));
    };

    if task_entry_accepts_text_generation_input(&entry) {
        Ok(entry.task_id)
    } else {
        Err(NodeEngineError::ExecutionFailed(format!(
            "task_kind '{}' resolves to '{}' and cannot be executed by the text generation node",
            task_label,
            entry.task_id.canonical_label()
        )))
    }
}

#[cfg(feature = "inference-nodes")]
fn task_entry_accepts_text_generation_input(entry: &inference::TaskRegistryEntry) -> bool {
    entry.request_contract().is_some_and(|contract| {
        contract.execution_supported
            && contract.input_kind == inference::InferenceExecutionInputKind::TextGeneration
    })
}

#[cfg(feature = "inference-nodes")]
fn read_text_generation_task_label(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Result<Option<String>> {
    for alias in aliases {
        if let Some(value) = inputs
            .get(*alias)
            .or_else(|| inputs.get("_data").and_then(|data| data.get(*alias)))
        {
            return value
                .as_str()
                .map(|value| Some(value.to_string()))
                .ok_or_else(|| {
                    NodeEngineError::ExecutionFailed(format!(
                        "Invalid text generation task kind input '{alias}': expected string"
                    ))
                });
        }
    }

    Ok(None)
}

#[cfg(feature = "inference-nodes")]
fn parse_pumas_model_ref(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<inference::PumasModelRef> {
    inputs
        .get("pumas_model_ref")
        .or_else(|| inputs.get("model_ref"))
        .and_then(|value| serde_json::from_value(value.clone()).ok())
        .or_else(|| parse_resolved_model_source_ref(inputs))
}

#[cfg(feature = "inference-nodes")]
fn parse_resolved_model_source_ref(
    inputs: &HashMap<String, serde_json::Value>,
) -> Option<inference::PumasModelRef> {
    read_optional_input_value(inputs, "resolved_model_source")
        .and_then(|value| serde_json::from_value::<inference::ResolvedModelSource>(value).ok())
        .and_then(|source| source.model_ref)
}

#[cfg(feature = "inference-nodes")]
fn parse_resolved_model_package_facts(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Option<inference::ResolvedModelPackageFacts>> {
    let Some((key, value)) = read_optional_input_value(inputs, "resolved_model_package_facts")
        .map(|value| ("resolved_model_package_facts", value))
        .or_else(|| {
            read_optional_input_value(inputs, "model_package_facts")
                .map(|value| ("model_package_facts", value))
        })
    else {
        return Ok(None);
    };

    serde_json::from_value(value)
        .map(Some)
        .map_err(|error| NodeEngineError::ExecutionFailed(format!("Invalid {key} input: {error}")))
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
    let expected_result_kind = expected_typed_result_kind(&request)?;
    let model_name = request.model_name.clone();
    let start = std::time::Instant::now();
    let result = execute_typed_gateway(gw, request, extensions)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Typed embedding inference failed: {error}"))
        })?;
    ensure_typed_result_kind(&result, expected_result_kind, "Typed embedding inference")?;
    let (embeddings, usage, option_diagnostics) = match result {
        inference::InferenceExecutionResult::Embedding {
            embeddings,
            usage,
            option_diagnostics,
        } => (embeddings, usage, option_diagnostics),
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
    if let Some(usage) = usage {
        outputs.insert(
            "usage".to_string(),
            serde_json::to_value(usage).unwrap_or(serde_json::Value::Null),
        );
    }
    outputs.insert(
        "diagnostics".to_string(),
        serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
    );
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

    let resolved_model_package_facts = parse_resolved_model_package_facts(inputs)?;

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
        resolved_model_package_facts,
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
    let expected_result_kind = expected_typed_result_kind(&request)?;
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
    ensure_typed_result_kind(&result, expected_result_kind, "Typed rerank inference")?;
    let (response, option_diagnostics) = match result {
        inference::InferenceExecutionResult::Rerank {
            response,
            option_diagnostics,
        } => (response, option_diagnostics),
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
    outputs.insert(
        "diagnostics".to_string(),
        serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
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
    let top_n = read_positive_usize_with_task_options(inputs, &["top_n", "topN", "top_k", "topK"]);
    let return_documents = read_bool_with_task_options(
        inputs,
        &["return_documents", "returnDocuments"],
        "return_documents",
    )
    .unwrap_or(true);
    let model_ref = parse_pumas_model_ref(inputs);
    let model_name = read_rerank_model_name(inputs, model_ref.as_ref())?;
    let mut extra_settings = build_extra_settings(inputs);
    extra_settings.remove("gpu_layers");
    extra_settings.remove("context_length");

    let resolved_model_package_facts = parse_resolved_model_package_facts(inputs)?;

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: inference::InferenceTaskId::Rerank,
        model_ref,
        model_name,
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        resolved_model_package_facts,
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
pub(crate) async fn execute_audio_transcription_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;
    let mut request = build_audio_transcription_execution_request(inputs)?;
    assign_typed_request_id(&mut request, task_id, execution_id);
    let expected_result_kind = expected_typed_result_kind(&request)?;
    let result = execute_typed_gateway(gw, request, extensions)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Typed audio transcription failed: {error}"))
        })?;
    ensure_typed_result_kind(&result, expected_result_kind, "Typed audio transcription")?;
    let (transcription, option_diagnostics) = match result {
        inference::InferenceExecutionResult::AudioTranscription {
            result,
            option_diagnostics,
        } => (result, option_diagnostics),
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Typed audio transcription returned unexpected result: {other:?}"
            )));
        }
    };

    let mut outputs = HashMap::new();
    outputs.insert(
        "response".to_string(),
        serde_json::json!(transcription.text.clone()),
    );
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    outputs.insert("text".to_string(), serde_json::json!(transcription.text));
    outputs.insert(
        "language".to_string(),
        transcription
            .language
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "duration_seconds".to_string(),
        transcription
            .duration_seconds
            .map(|duration| serde_json::json!(duration))
            .unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "segments".to_string(),
        serde_json::to_value(transcription.segments).unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "metadata".to_string(),
        if transcription.metadata.is_null() {
            serde_json::Value::Null
        } else {
            transcription.metadata
        },
    );
    outputs.insert(
        "diagnostics".to_string(),
        serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
    );
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_image_generation_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    extensions: &ExecutorExtensions,
    task_id: &str,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;
    let mut request = build_image_generation_execution_request(inputs)?;
    assign_typed_request_id(&mut request, task_id, execution_id);
    let expected_result_kind = expected_typed_result_kind(&request)?;
    let result = execute_typed_gateway(gw, request, extensions)
        .await
        .map_err(|error| {
            NodeEngineError::ExecutionFailed(format!("Typed image generation failed: {error}"))
        })?;
    ensure_typed_result_kind(&result, expected_result_kind, "Typed image generation")?;
    let (image_result, option_diagnostics) = match result {
        inference::InferenceExecutionResult::ImageGeneration {
            result,
            option_diagnostics,
        } => (result, option_diagnostics),
        other => {
            return Err(NodeEngineError::ExecutionFailed(format!(
                "Typed image generation returned unexpected result: {other:?}"
            )));
        }
    };

    let mut outputs = HashMap::new();
    outputs.insert(
        "results".to_string(),
        serde_json::to_value(&image_result).unwrap_or(serde_json::Value::Null),
    );
    outputs.insert(
        "metadata".to_string(),
        serde_json::json!({
            "seed_used": image_result.seed_used,
            "image_count": image_result.images.len(),
            "backend_metadata": image_result.metadata,
        }),
    );
    outputs.insert(
        "diagnostics".to_string(),
        serde_json::to_value(option_diagnostics).unwrap_or(serde_json::Value::Null),
    );
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn build_image_generation_execution_request(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceExecutionRequest> {
    let prompt = inputs
        .get("prompt")
        .and_then(|value| value.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;
    if prompt.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Image generation prompt cannot be empty".to_string(),
        ));
    }

    let resolved_model_package_facts = parse_resolved_model_package_facts(inputs)?;
    let model_ref = parse_pumas_model_ref(inputs).or_else(|| {
        resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.clone())
    });
    let model = read_image_generation_model_name(inputs, model_ref.as_ref())?
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing image model input".to_string()))?;
    let mut extra_options = build_extra_settings(inputs);
    remove_image_generation_first_class_options(&mut extra_options);

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: inference::InferenceTaskId::ImageGeneration,
        model_ref,
        model_name: Some(model.clone()),
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        resolved_model_package_facts,
        input: inference::InferenceExecutionInput::ImageGeneration {
            request: inference::ImageGenerationRequest {
                model,
                prompt: prompt.to_string(),
                negative_prompt: read_string_with_task_options(
                    inputs,
                    &["negative_prompt", "negativePrompt"],
                ),
                width: read_positive_u32_with_task_options(inputs, &["width"]),
                height: read_positive_u32_with_task_options(inputs, &["height"]),
                num_inference_steps: read_positive_u32_with_task_options(
                    inputs,
                    &["num_inference_steps", "numInferenceSteps", "steps"],
                ),
                guidance_scale: read_positive_f32_with_task_options(
                    inputs,
                    &["guidance_scale", "guidanceScale", "cfg_scale", "cfgScale"],
                ),
                seed: read_u64_with_task_options(inputs, &["seed"]),
                scheduler: read_string_with_task_options(inputs, &["scheduler"]),
                num_images_per_prompt: read_positive_u32_with_task_options(
                    inputs,
                    &[
                        "num_images_per_prompt",
                        "numImagesPerPrompt",
                        "num_images",
                        "numImages",
                    ],
                ),
                init_image: None,
                mask_image: None,
                strength: read_positive_f32_with_task_options(inputs, &["strength"]),
                extra_options: serde_json::Value::Object(extra_options.into_iter().collect()),
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    })
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn build_audio_transcription_execution_request(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<inference::InferenceExecutionRequest> {
    let audio_value = read_optional_input_value(inputs, "audio")
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing audio input".to_string()))?;
    let (audio, audio_ref) = parse_audio_transcription_input(audio_value)?;
    let model_ref = parse_pumas_model_ref(inputs);
    let model_name = read_audio_model_name(inputs, model_ref.as_ref())?;
    let mut extra_settings = build_extra_settings(inputs);
    extra_settings.remove("audio");
    extra_settings.remove("audio_ref");
    extra_settings.remove("audioRef");
    extra_settings.remove("audio_base64");
    extra_settings.remove("audioBase64");
    extra_settings.remove("audio_data");
    extra_settings.remove("audioData");
    extra_settings.remove("data_base64");
    extra_settings.remove("dataBase64");

    Ok(inference::InferenceExecutionRequest {
        request_id: None,
        task_id: inference::InferenceTaskId::AudioTranscription,
        model_ref,
        model_name: model_name.clone(),
        runtime_hint: read_optional_input_string_aliases(inputs, &["runtime_hint", "runtimeHint"]),
        resolved_model_package_facts: parse_resolved_model_package_facts(inputs)?,
        input: inference::InferenceExecutionInput::AudioTranscription {
            request: inference::AudioTranscriptionRequest {
                model: model_name.unwrap_or_else(|| "default".to_string()),
                audio,
                audio_ref,
                language: read_optional_input_string_aliases(inputs, &["language", "lang"]),
                prompt: read_optional_input_string_aliases(inputs, &["prompt", "context"]),
                task: read_optional_input_string_aliases(inputs, &["asr_task", "asrTask", "task"]),
                chunk_length_s: read_positive_f32_aliases(
                    inputs,
                    &["chunk_length_s", "chunkLengthS", "chunk_length_seconds"],
                ),
                extra_options: serde_json::Value::Object(extra_settings.into_iter().collect()),
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    })
}

#[cfg(feature = "inference-nodes")]
fn parse_audio_transcription_input(
    value: serde_json::Value,
) -> Result<(Option<inference::EncodedAudio>, Option<String>)> {
    if let Some(audio) = value.as_str() {
        let audio = audio.trim();
        if audio.is_empty() {
            return Err(NodeEngineError::ExecutionFailed(
                "Audio input cannot be empty".to_string(),
            ));
        }
        if audio.starts_with("artifact://") || audio.starts_with("artifact-read://") {
            return Ok((None, Some(audio.to_string())));
        }
        return Ok((
            Some(inference::EncodedAudio {
                data_base64: audio.to_string(),
                mime_type: "audio/wav".to_string(),
                sample_rate_hz: None,
            }),
            None,
        ));
    }

    let Some(object) = value.as_object() else {
        return Err(NodeEngineError::ExecutionFailed(
            "Audio input must be a base64 string, artifact reference, or object".to_string(),
        ));
    };
    let audio_ref = read_string_aliases_from_object(object, &["audio_ref", "audioRef", "ref"])
        .filter(|value| !value.trim().is_empty());
    let data_base64 = read_string_aliases_from_object(
        object,
        &[
            "data_base64",
            "dataBase64",
            "audio_base64",
            "audioBase64",
            "audio_data",
            "audioData",
        ],
    )
    .filter(|value| !value.trim().is_empty());
    let audio = data_base64.map(|data_base64| inference::EncodedAudio {
        data_base64,
        mime_type: read_string_aliases_from_object(object, &["mime_type", "mimeType"])
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "audio/wav".to_string()),
        sample_rate_hz: read_u32_aliases_from_object(object, &["sample_rate_hz", "sampleRateHz"]),
    });

    if audio.is_none() && audio_ref.is_none() {
        return Err(NodeEngineError::ExecutionFailed(
            "Audio input object must include data_base64 or audio_ref".to_string(),
        ));
    }
    Ok((audio, audio_ref))
}

#[cfg(feature = "inference-nodes")]
fn read_audio_model_name(
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
        return Ok(Some(model_path));
    }

    Ok(model_ref.map(|model_ref| model_ref.model_id.clone()))
}

#[cfg(feature = "inference-nodes")]
fn read_image_generation_model_name(
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
        return Ok(Some(model_path));
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
            return Ok(Some(path));
        }
    }

    Ok(model_ref.map(|model_ref| model_ref.model_id.clone()))
}

#[cfg(feature = "inference-nodes")]
fn remove_image_generation_first_class_options(
    extra_options: &mut HashMap<String, serde_json::Value>,
) {
    for key in [
        "prompt",
        "negative_prompt",
        "negativePrompt",
        "width",
        "height",
        "num_inference_steps",
        "numInferenceSteps",
        "steps",
        "guidance_scale",
        "guidanceScale",
        "cfg_scale",
        "cfgScale",
        "seed",
        "scheduler",
        "num_images_per_prompt",
        "numImagesPerPrompt",
        "num_images",
        "numImages",
        "strength",
    ] {
        extra_options.remove(key);
    }
}

#[cfg(feature = "inference-nodes")]
fn read_string_aliases_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<String> {
    aliases
        .iter()
        .find_map(|alias| object.get(*alias).and_then(|value| value.as_str()))
        .map(str::to_string)
}

#[cfg(feature = "inference-nodes")]
fn read_u32_aliases_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<u32> {
    aliases.iter().find_map(|alias| {
        object.get(*alias).and_then(|value| {
            value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .or_else(|| value.as_i64().and_then(|value| u32::try_from(value).ok()))
        })
    })
}

#[cfg(feature = "inference-nodes")]
fn read_positive_f32_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<f32> {
    aliases.iter().find_map(|alias| {
        read_optional_input_value(inputs, alias).and_then(|value| {
            value
                .as_f64()
                .filter(|value| *value > 0.0 && value.is_finite())
                .map(|value| value as f32)
                .or_else(|| {
                    value
                        .as_i64()
                        .filter(|value| *value > 0)
                        .map(|value| value as f32)
                })
        })
    })
}

#[cfg(feature = "inference-nodes")]
fn read_positive_f32_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<f32> {
    read_positive_f32_aliases(inputs, aliases)
        .or_else(|| read_task_option_value_aliases(inputs, aliases).and_then(positive_f32_value))
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn expected_typed_result_kind(
    request: &inference::InferenceExecutionRequest,
) -> Result<inference::InferenceExecutionResultKind> {
    inference::resolve_task_registry_entry(request.task_id.canonical_label())
        .and_then(|entry| entry.request_contract())
        .map(|contract| contract.result_kind)
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(format!(
                "No typed result contract is registered for task '{}'",
                request.task_id.canonical_label()
            ))
        })
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn ensure_typed_result_kind(
    result: &inference::InferenceExecutionResult,
    expected: inference::InferenceExecutionResultKind,
    context: &str,
) -> Result<()> {
    let actual = result.result_kind();
    if actual == expected {
        Ok(())
    } else {
        Err(NodeEngineError::ExecutionFailed(format!(
            "{context} returned result kind '{}' but task contract expected '{}'",
            actual.canonical_label(),
            expected.canonical_label()
        )))
    }
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
fn read_positive_usize_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<usize> {
    read_positive_usize_aliases(inputs, aliases)
        .or_else(|| read_task_option_value_aliases(inputs, aliases).and_then(positive_usize_value))
}

#[cfg(feature = "inference-nodes")]
fn read_positive_u32_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<u32> {
    aliases
        .iter()
        .find_map(|alias| read_optional_input_value(inputs, alias).and_then(positive_u32_value))
        .or_else(|| read_task_option_value_aliases(inputs, aliases).and_then(positive_u32_value))
}

#[cfg(feature = "inference-nodes")]
fn read_u64_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<u64> {
    aliases
        .iter()
        .find_map(|alias| read_optional_input_value(inputs, alias).and_then(u64_value))
        .or_else(|| read_task_option_value_aliases(inputs, aliases).and_then(u64_value))
}

#[cfg(feature = "inference-nodes")]
fn read_string_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<String> {
    read_optional_input_string_aliases(inputs, aliases)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            read_task_option_value_aliases(inputs, aliases)
                .and_then(|value| value.as_str().map(str::to_string))
                .filter(|value| !value.trim().is_empty())
        })
}

#[cfg(feature = "inference-nodes")]
fn read_bool_with_task_options(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
    task_option_key: &str,
) -> Option<bool> {
    read_optional_input_bool_aliases(inputs, aliases)
        .or_else(|| read_task_option_value(inputs, task_option_key).and_then(bool_value))
}

#[cfg(feature = "inference-nodes")]
fn read_task_option_value_aliases(
    inputs: &HashMap<String, serde_json::Value>,
    aliases: &[&str],
) -> Option<serde_json::Value> {
    aliases
        .iter()
        .find_map(|alias| read_task_option_value(inputs, alias))
}

#[cfg(feature = "inference-nodes")]
fn read_task_option_value(
    inputs: &HashMap<String, serde_json::Value>,
    key: &str,
) -> Option<serde_json::Value> {
    read_optional_input_value(inputs, "task_options").and_then(|task_options| {
        task_options
            .as_object()
            .and_then(|options| options.get(key).cloned())
    })
}

#[cfg(feature = "inference-nodes")]
fn positive_usize_value(value: serde_json::Value) -> Option<usize> {
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
}

#[cfg(feature = "inference-nodes")]
fn positive_u32_value(value: serde_json::Value) -> Option<u32> {
    value
        .as_u64()
        .filter(|value| *value > 0)
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| {
            value
                .as_i64()
                .filter(|value| *value > 0)
                .and_then(|value| u32::try_from(value).ok())
        })
}

#[cfg(feature = "inference-nodes")]
fn positive_f32_value(value: serde_json::Value) -> Option<f32> {
    value
        .as_f64()
        .filter(|value| *value > 0.0 && value.is_finite())
        .map(|value| value as f32)
        .or_else(|| {
            value
                .as_i64()
                .filter(|value| *value > 0)
                .map(|value| value as f32)
        })
}

#[cfg(feature = "inference-nodes")]
fn u64_value(value: serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|value| u64::try_from(value).ok()))
}

#[cfg(feature = "inference-nodes")]
fn bool_value(value: serde_json::Value) -> Option<bool> {
    if let Some(boolean) = value.as_bool() {
        return Some(boolean);
    }
    value
        .as_str()
        .and_then(|s| match s.trim().to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" => Some(false),
            _ => None,
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
