use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use inference::InferenceGateway;

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::model_dependencies::ModelRefV2;

use super::{
    build_extra_settings, build_model_ref_v2, canonical_backend_key, infer_task_type_primary,
    kv_cache, read_optional_input_bool_aliases, read_optional_input_string_aliases,
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
pub(crate) fn parse_reranker_documents(value: &serde_json::Value) -> Result<Vec<String>> {
    let items = if let Some(items) = value.as_array() {
        items
    } else {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input must be a JSON array".to_string(),
        ));
    };

    let mut documents = Vec::with_capacity(items.len());
    for item in items {
        if let Some(text) = item.as_str() {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        if let Some(text) = item
            .get("text")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("content").and_then(|v| v.as_str()))
            .or_else(|| item.get("document").and_then(|v| v.as_str()))
        {
            if !text.trim().is_empty() {
                documents.push(text.to_string());
            }
            continue;
        }
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents must be strings or objects with text/content/document fields"
                .to_string(),
        ));
    }

    if documents.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker documents input cannot be empty".to_string(),
        ));
    }

    Ok(documents)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn parse_reranker_documents_input(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<Vec<String>> {
    if let Some(value) = inputs.get("documents") {
        return parse_reranker_documents(value);
    }

    if let Some(raw) = inputs
        .get("documents_json")
        .and_then(|value| value.as_str())
    {
        let parsed: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Reranker documents_json must be valid JSON: {}",
                e
            ))
        })?;
        return parse_reranker_documents(&parsed);
    }

    Err(NodeEngineError::ExecutionFailed(
        "Missing documents input".to_string(),
    ))
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_llamacpp_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
    resolved_model_ref: Option<ModelRefV2>,
    extensions: &ExecutorExtensions,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;

    let model_path = resolve_gguf_path(model_path_raw)?;
    let system_prompt = inputs.get("system_prompt").and_then(|s| s.as_str());
    let temperature = inputs
        .get("temperature")
        .and_then(|t| t.as_f64())
        .unwrap_or(0.7);
    let max_tokens = inputs
        .get("max_tokens")
        .and_then(|m| m.as_i64())
        .unwrap_or(512);

    // Read model-specific inference settings
    let extra_settings = build_extra_settings(inputs);

    // Ensure gateway is ready — start if needed
    if !gw.is_ready().await {
        let mut config = inference::BackendConfig {
            model_path: Some(PathBuf::from(&model_path)),
            device: Some("auto".to_string()),
            gpu_layers: Some(-1),
            embedding_mode: false,
            ..Default::default()
        };

        // Apply model-specific settings to backend config
        if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
            config.gpu_layers = Some(v as i32);
        }
        if let Some(v) = extra_settings
            .get("context_length")
            .and_then(|v| v.as_i64())
        {
            config.context_size = Some(v as u32);
        }

        log::info!(
            "LlamaCppInference: starting server with model '{}'",
            model_path
        );
        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start llama.cpp server: {}", e))
        })?;

        // Wait for readiness with timeout
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for llama.cpp server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        log::info!("LlamaCppInference: server is ready");
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "llama.cpp server started but no URL available".to_string(),
        )
    })?;

    let full_prompt = if let Some(sys) = system_prompt {
        format!("{}\n\n{}", sys, prompt)
    } else {
        prompt.to_string()
    };

    let restored_kv_slot = kv_cache::restore_llamacpp_input_handle(
        inputs,
        gw,
        extensions,
        task_id,
        execution_id,
        event_sink,
    )
    .await?;
    let streaming = event_sink.is_some();
    let mut request_body = serde_json::json!({
        "prompt": full_prompt,
        "n_predict": max_tokens,
        "temperature": temperature,
        "stop": ["</s>", "<|im_end|>", "<|end|>"],
        "stream": streaming
    });
    if restored_kv_slot {
        request_body["id_slot"] = serde_json::json!(0);
        request_body["cache_prompt"] = serde_json::json!(true);
    }

    let client = reqwest::Client::new();
    let url = format!("{}/completion", base_url);

    log::debug!(
        "LlamaCppInference: sending request to {} (stream={})",
        url,
        streaming
    );

    let http_response = client
        .post(&url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!(
                "Failed to connect to llama.cpp server at {}: {}",
                url, e
            ))
        })?;

    if !http_response.status().is_success() {
        let status = http_response.status();
        let error_body = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "llama.cpp API error ({}): {}",
            status, error_body
        )));
    }

    let response_text = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete lines from buffer
            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_llamacpp_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        // Process any remaining data in buffer
        let line = buffer.trim().to_string();
        if let Some(token) = parse_llamacpp_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let response_json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse llama.cpp response: {}", e))
        })?;
        response_json["content"].as_str().unwrap_or("").to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response_text));
    outputs.insert("model_path".to_string(), serde_json::json!(model_path));
    let task_type_primary = infer_task_type_primary("llamacpp-inference", inputs);
    let model_ref = build_model_ref_v2(
        resolved_model_ref,
        "llamacpp",
        &model_path,
        &model_path,
        &task_type_primary,
        inputs,
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::to_value(model_ref).unwrap_or_else(|_| {
            serde_json::json!({
                "contractVersion": 2,
                "engine": "llamacpp",
                "modelId": model_path,
                "modelPath": model_path,
                "taskTypePrimary": task_type_primary,
            })
        }),
    );
    let kv_cache_output = match kv_cache::capture_llamacpp_output_handle(
        task_id,
        execution_id,
        gw,
        extensions,
        event_sink,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            log::warn!(
                "LlamaCppInference: failed to capture KV cache output for '{}': {}",
                task_id,
                error
            );
            serde_json::Value::Null
        }
    };
    outputs.insert("kv_cache_out".to_string(), kv_cache_output);
    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_reranker(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let query = inputs
        .get("query")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing query input".to_string()))?;
    if query.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Reranker query cannot be empty".to_string(),
        ));
    }

    let documents = parse_reranker_documents_input(inputs)?;

    let model_path_raw = inputs
        .get("model_path")
        .and_then(|m| m.as_str())
        .ok_or_else(|| {
            NodeEngineError::ExecutionFailed(
                "Missing model_path input. Connect a Puma-Lib node.".to_string(),
            )
        })?;
    let model_path = resolve_gguf_path(model_path_raw)?;

    let top_k = inputs
        .get("top_k")
        .and_then(|value| value.as_u64().map(|v| v as usize))
        .or_else(|| {
            inputs
                .get("top_k")
                .and_then(|value| value.as_i64())
                .filter(|v| *v > 0)
                .map(|v| v as usize)
        });
    let return_documents =
        read_optional_input_bool_aliases(inputs, &["return_documents", "returnDocuments"])
            .unwrap_or(true);

    let mut extra_settings = build_extra_settings(inputs);
    let mut config = inference::BackendConfig {
        model_path: Some(PathBuf::from(&model_path)),
        device: Some("auto".to_string()),
        gpu_layers: Some(-1),
        reranking_mode: true,
        ..Default::default()
    };

    if let Some(v) = extra_settings.get("gpu_layers").and_then(|v| v.as_i64()) {
        config.gpu_layers = Some(v as i32);
    }
    if let Some(v) = extra_settings
        .get("context_length")
        .and_then(|v| v.as_i64())
    {
        config.context_size = Some(v as u32);
    }
    extra_settings.remove("gpu_layers");
    extra_settings.remove("context_length");

    if !gw.is_ready().await || !gw.is_reranking_mode().await {
        if gw.is_ready().await {
            gw.stop().await;
        }

        gw.start(&config).await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to start reranking server: {}", e))
        })?;

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !gw.is_ready().await {
            if std::time::Instant::now() > deadline {
                return Err(NodeEngineError::ExecutionFailed(
                    "Timeout waiting for reranking server to start".to_string(),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }

    let response = gw
        .rerank(inference::RerankRequest {
            model: model_path.clone(),
            query: query.to_string(),
            documents,
            top_n: top_k,
            return_documents,
            extra_options: serde_json::Value::Object(extra_settings.into_iter().collect()),
        })
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("Reranker request failed: {}", e)))?;

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
        serde_json::json!(model_path.clone()),
    );
    outputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "contractVersion": 2,
            "engine": "llamacpp",
            "modelId": model_path,
            "modelPath": model_path,
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
pub(crate) async fn execute_embedding(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<HashMap<String, serde_json::Value>> {
    let gw = require_gateway(gateway)?;

    let text = inputs
        .get("text")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing text input".to_string()))?;
    if text.trim().is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "Embedding input text cannot be empty".to_string(),
        ));
    }

    let backend_name = gw.current_backend_name().await;
    if !is_llamacpp_backend_name(&backend_name) {
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LlamaCpp Embedding blocked execution: active backend '{}' is not supported",
            backend_name
        )));
    }

    let model = read_optional_input_string_aliases(
        inputs,
        &["model", "model_name", "modelName", "model_id", "modelId"],
    )
    .filter(|s| !s.trim().is_empty())
    .unwrap_or_else(|| "default".to_string());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is not ready. Start llama.cpp in embedding mode (`--embeddings`) first".to_string(),
        ));
    }
    if !gw.is_embedding_mode().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: backend is running in inference mode. Restart with `--embeddings`".to_string(),
        ));
    }
    let capabilities = gw.capabilities().await;
    if !capabilities.embeddings {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding blocked execution: active backend does not support embeddings"
                .to_string(),
        ));
    }

    let emit_metadata =
        read_optional_input_bool_aliases(inputs, &["emit_metadata", "emitMetadata"])
            .unwrap_or(false);

    let start = std::time::Instant::now();
    let results = gw
        .embeddings(vec![text.to_string()], &model)
        .await
        .map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("LlamaCpp Embedding request failed: {}", e))
        })?;
    let embedding = results.first().ok_or_else(|| {
        NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned no vectors for input text".to_string(),
        )
    })?;
    if embedding.vector.is_empty() {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned an empty vector".to_string(),
        ));
    }
    if embedding.vector.iter().any(|v| !v.is_finite()) {
        return Err(NodeEngineError::ExecutionFailed(
            "LlamaCpp Embedding returned invalid vector values".to_string(),
        ));
    }

    let mut outputs = HashMap::new();
    outputs.insert("embedding".to_string(), serde_json::json!(embedding.vector));
    if emit_metadata {
        outputs.insert(
            "metadata".to_string(),
            serde_json::json!({
                "backend": "llamacpp",
                "model": model,
                "vector_length": embedding.vector.len(),
                "duration_ms": start.elapsed().as_millis(),
            }),
        );
    }

    Ok(outputs)
}

#[cfg(feature = "inference-nodes")]
pub(crate) fn is_llamacpp_backend_name(backend_name: &str) -> bool {
    canonical_backend_key(Some(backend_name)).as_deref() == Some("llamacpp")
}

/// Parse a llama.cpp `/completion` SSE data line into a content token.
///
/// llama.cpp streams `data: {"content": "token", ...}` per line.
#[cfg(feature = "inference-nodes")]
pub(crate) fn parse_llamacpp_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("content")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Parse an OpenAI-compatible `/v1/chat/completions` SSE data line into a content token.
///
/// Streams `data: {"choices": [{"delta": {"content": "token"}}]}` per line.
#[cfg(feature = "inference-nodes")]
pub(crate) fn parse_openai_sse_content(line: &str) -> Option<String> {
    let data = line.strip_prefix("data: ")?;
    if data == "[DONE]" {
        return None;
    }
    let json: serde_json::Value = serde_json::from_str(data).ok()?;
    json.get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("delta"))
        .and_then(|d| d.get("content"))
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

#[cfg(feature = "inference-nodes")]
pub(crate) async fn execute_llm_inference(
    gateway: Option<&Arc<InferenceGateway>>,
    inputs: &HashMap<String, serde_json::Value>,
    task_id: &str,
    event_sink: Option<&Arc<dyn EventSink>>,
    execution_id: &str,
) -> Result<HashMap<String, serde_json::Value>> {
    use futures_util::StreamExt;

    let gw = require_gateway(gateway)?;

    let prompt = inputs
        .get("prompt")
        .and_then(|p| p.as_str())
        .ok_or_else(|| NodeEngineError::ExecutionFailed("Missing prompt input".to_string()))?;

    let system_prompt = inputs.get("system_prompt").and_then(|p| p.as_str());
    let extra_context = inputs.get("context").and_then(|c| c.as_str());

    if !gw.is_ready().await {
        return Err(NodeEngineError::ExecutionFailed(
            "LLM server is not ready".to_string(),
        ));
    }

    let base_url = gw.base_url().await.ok_or_else(|| {
        NodeEngineError::ExecutionFailed("No LLM server URL available".to_string())
    })?;

    let full_prompt = if let Some(ctx) = extra_context {
        format!("{}\n\nContext:\n{}", prompt, ctx)
    } else {
        prompt.to_string()
    };

    let mut messages = Vec::new();
    if let Some(sys) = system_prompt {
        messages.push(serde_json::json!({"role": "system", "content": sys}));
    }
    messages.push(serde_json::json!({"role": "user", "content": full_prompt}));

    let streaming = event_sink.is_some();
    let mut request_body = serde_json::json!({
        "model": "gpt-4",
        "messages": messages,
        "stream": streaming
    });

    // Forward model-specific inference settings into the request body
    let extra_settings = build_extra_settings(inputs);
    for (key, value) in &extra_settings {
        request_body[key] = value.clone();
    }

    let client = reqwest::Client::new();
    let http_response = client
        .post(format!("{}/v1/chat/completions", base_url))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| NodeEngineError::ExecutionFailed(format!("LLM request failed: {}", e)))?;

    if !http_response.status().is_success() {
        let error = http_response.text().await.unwrap_or_default();
        return Err(NodeEngineError::ExecutionFailed(format!(
            "LLM error: {}",
            error
        )));
    }

    let response = if let Some(sink) = event_sink {
        // Streaming path: parse SSE and emit per-token events
        let mut full_response = String::new();
        let mut byte_stream = http_response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let chunk = chunk_result.map_err(|e| {
                NodeEngineError::ExecutionFailed(format!("Stream read error: {}", e))
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_pos) = buffer.find('\n') {
                let line = buffer[..newline_pos].trim().to_string();
                buffer = buffer[newline_pos + 1..].to_string();

                if let Some(token) = parse_openai_sse_content(&line) {
                    full_response.push_str(&token);
                    let _ = sink.send(crate::WorkflowEvent::task_stream(
                        task_id,
                        execution_id,
                        "response",
                        serde_json::json!(token),
                    ));
                }
            }
        }
        let line = buffer.trim().to_string();
        if let Some(token) = parse_openai_sse_content(&line) {
            full_response.push_str(&token);
            let _ = sink.send(crate::WorkflowEvent::task_stream(
                task_id,
                execution_id,
                "response",
                serde_json::json!(token),
            ));
        }

        full_response
    } else {
        // Non-streaming path: collect entire response
        let json: serde_json::Value = http_response.json().await.map_err(|e| {
            NodeEngineError::ExecutionFailed(format!("Failed to parse response: {}", e))
        })?;
        json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    };

    let mut outputs = HashMap::new();
    outputs.insert("response".to_string(), serde_json::json!(response));
    outputs.insert("stream".to_string(), serde_json::Value::Null);
    Ok(outputs)
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
            let client = reqwest::Client::new();
            let url = "http://localhost:11434/api/generate";
            let request_body = serde_json::json!({
                "model": model_id,
                "keep_alive": 0
            });

            match client.post(url).json(&request_body).send().await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!(
                        "UnloadModel: Ollama model '{}' unloaded from VRAM",
                        model_id
                    );
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    log::warn!(
                        "UnloadModel: Ollama unload returned {} for model '{}': {}",
                        status,
                        model_id,
                        body
                    );
                }
                Err(e) => {
                    return Err(NodeEngineError::ExecutionFailed(format!(
                        "Failed to connect to Ollama server to unload model '{}': {}",
                        model_id, e
                    )));
                }
            }
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
                "Unknown inference engine '{}'. Supported: llamacpp, ollama, pytorch, stable_audio, onnx-runtime",
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
