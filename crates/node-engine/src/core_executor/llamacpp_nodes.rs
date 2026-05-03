use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use inference::InferenceGateway;

use crate::error::{NodeEngineError, Result};
use crate::events::EventSink;
use crate::extensions::ExecutorExtensions;
use crate::model_dependencies::ModelRefV2;

use super::{
    build_extra_settings, build_model_ref_v2, infer_task_type_primary, kv_cache,
    normalize_generation_options_value, require_gateway, resolve_gguf_path,
};

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
    let generation_parameters = llama_cpp_request_generation_parameters(inputs)?;

    // Read model-specific inference settings
    let extra_settings = build_extra_settings(inputs);

    // Ensure the gateway is running the model requested by this node. A ready
    // llama.cpp gateway may still be serving a previous workflow's model.
    if !llamacpp_gateway_matches_requested_model(gw, &model_path).await {
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
        "n_predict": generation_parameters.max_tokens,
        "temperature": generation_parameters.temperature,
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
    let task_type_primary = infer_task_type_primary("llm-inference", inputs);
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

#[derive(Debug, PartialEq)]
struct LlamaCppRequestGenerationParameters {
    max_tokens: i64,
    temperature: f64,
}

fn llama_cpp_request_generation_parameters(
    inputs: &HashMap<String, serde_json::Value>,
) -> Result<LlamaCppRequestGenerationParameters> {
    let mut parameters = LlamaCppRequestGenerationParameters {
        max_tokens: inputs
            .get("max_tokens")
            .and_then(|m| m.as_i64())
            .unwrap_or(512),
        temperature: inputs
            .get("temperature")
            .and_then(|t| t.as_f64())
            .unwrap_or(0.7),
    };

    let Some(options_value) = inputs.get("generation_options") else {
        return Ok(parameters);
    };
    let options = serde_json::from_value::<inference::GenerationOptions>(
        normalize_generation_options_value(options_value.clone()),
    )
    .map_err(|error| {
        NodeEngineError::ExecutionFailed(format!("Invalid generation_options input: {error}"))
    })?;

    if let Some(max_new_tokens) = options.length.max_new_tokens {
        parameters.max_tokens = i64::from(max_new_tokens);
    }
    if let Some(temperature) = options.sampling.temperature {
        parameters.temperature = f64::from(temperature);
    }

    Ok(parameters)
}

async fn llamacpp_gateway_matches_requested_model(gw: &InferenceGateway, model_path: &str) -> bool {
    if !gw.is_ready().await || gw.is_embedding_mode().await || gw.is_reranking_mode().await {
        return false;
    }

    let Some(config) = gw.restart_runtime_config().await else {
        return false;
    };
    if config.external_url.is_some() {
        return true;
    }
    let Some(active_model_path) = config.model_path.as_deref() else {
        return false;
    };

    paths_refer_to_same_file(active_model_path, Path::new(model_path))
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (std::fs::canonicalize(left), std::fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// Parse a llama.cpp `/completion` SSE data line into a content token.
///
/// llama.cpp streams `data: {"content": "token", ...}` per line.
fn parse_llamacpp_sse_content(line: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use futures_util::{stream, Stream};
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{InferenceGateway, RerankRequest, RerankResponse};
    use std::pin::Pin;
    use std::sync::Arc;

    struct MockProcessHandle;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            1
        }

        fn kill(&self) -> std::result::Result<(), String> {
            Ok(())
        }
    }

    struct MockProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> std::result::Result<
            (
                tokio::sync::mpsc::Receiver<ProcessEvent>,
                Box<dyn ProcessHandle>,
            ),
            String,
        > {
            let (_tx, rx) = tokio::sync::mpsc::channel(1);
            Ok((rx, Box::new(MockProcessHandle)))
        }

        fn app_data_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }

        fn binaries_dir(&self) -> std::result::Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }
    }

    struct MockReadyBackend {
        ready: bool,
    }

    #[async_trait]
    impl InferenceBackend for MockReadyBackend {
        fn name(&self) -> &'static str {
            "llama.cpp"
        }

        fn description(&self) -> &'static str {
            "Mock llama.cpp backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities {
                streaming: true,
                external_connection: true,
                ..BackendCapabilities::default()
            }
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> std::result::Result<BackendStartOutcome, BackendError> {
            self.ready = true;
            Ok(BackendStartOutcome::default())
        }

        fn stop(&mut self) {
            self.ready = false;
        }

        fn is_ready(&self) -> bool {
            self.ready
        }

        async fn health_check(&self) -> bool {
            self.ready
        }

        fn base_url(&self) -> Option<String> {
            Some("http://127.0.0.1:8080".to_string())
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> std::result::Result<
            Pin<Box<dyn Stream<Item = std::result::Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(
            &self,
            _request: RerankRequest,
        ) -> std::result::Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }
    }

    fn unique_model_path(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "pantograph-node-engine-{name}-{}.gguf",
            std::process::id()
        ));
        std::fs::write(&path, b"gguf").expect("write mock model");
        path
    }

    #[test]
    fn generation_parameters_prefer_canonical_generation_options() {
        let mut inputs = HashMap::new();
        inputs.insert("max_tokens".to_string(), serde_json::json!(12));
        inputs.insert("temperature".to_string(), serde_json::json!(1.2));
        inputs.insert(
            "generation_options".to_string(),
            serde_json::json!({
                "length": {"max_new_tokens": 34},
                "sampling": {"temperature": 0.3}
            }),
        );

        let parameters = llama_cpp_request_generation_parameters(&inputs)
            .expect("canonical generation options should parse");

        assert_eq!(parameters.max_tokens, 34);
        assert!((parameters.temperature - 0.3).abs() < 0.000_001);
    }

    #[test]
    fn generation_parameters_accept_legacy_flat_generation_options() {
        let mut inputs = HashMap::new();
        inputs.insert(
            "generation_options".to_string(),
            serde_json::json!({
                "max_new_tokens": 55,
                "temperature": 0.45
            }),
        );

        let parameters = llama_cpp_request_generation_parameters(&inputs)
            .expect("flat generation options should normalize");

        assert_eq!(parameters.max_tokens, 55);
        assert!((parameters.temperature - 0.45).abs() < 0.000_001);
    }

    #[test]
    fn generation_parameters_keep_top_level_fallbacks() {
        let mut inputs = HashMap::new();
        inputs.insert("max_tokens".to_string(), serde_json::json!(21));
        inputs.insert("temperature".to_string(), serde_json::json!(0.9));

        let parameters = llama_cpp_request_generation_parameters(&inputs)
            .expect("top-level fallback generation parameters should parse");

        assert_eq!(parameters.max_tokens, 21);
        assert!((parameters.temperature - 0.9).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn gateway_match_requires_active_model_path() {
        let model_a = unique_model_path("a");
        let model_b = unique_model_path("b");
        let gateway = InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        );
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&BackendConfig {
                model_path: Some(model_a.clone()),
                ..BackendConfig::default()
            })
            .await
            .expect("start mock backend");

        assert!(
            llamacpp_gateway_matches_requested_model(&gateway, &model_a.to_string_lossy()).await
        );
        assert!(
            !llamacpp_gateway_matches_requested_model(&gateway, &model_b.to_string_lossy()).await
        );

        let _ = std::fs::remove_file(model_a);
        let _ = std::fs::remove_file(model_b);
    }

    #[tokio::test]
    async fn gateway_match_rejects_embedding_runtime() {
        let model = unique_model_path("embedding");
        let gateway = InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        );
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&BackendConfig {
                model_path: Some(model.clone()),
                embedding_mode: true,
                ..BackendConfig::default()
            })
            .await
            .expect("start mock backend");

        assert!(
            !llamacpp_gateway_matches_requested_model(&gateway, &model.to_string_lossy()).await
        );

        let _ = std::fs::remove_file(model);
    }
}
