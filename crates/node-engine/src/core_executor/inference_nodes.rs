use std::collections::HashMap;
use std::sync::Arc;

use inference::InferenceGateway;

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::model_dependencies::ModelRefV2;

use super::build_extra_settings;

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
