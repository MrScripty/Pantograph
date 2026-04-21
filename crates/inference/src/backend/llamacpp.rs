//! llama.cpp backend implementation
//!
//! This backend wraps the LlamaServer sidecar management code,
//! providing the InferenceBackend trait interface.

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::config::DeviceConfig;
use crate::constants::defaults;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::process::ProcessSpawner;
use crate::server::{LlamaServer, ServerMode};
use crate::types::{RerankRequest, RerankResponse, RerankResult};
use pantograph_runtime_identity::{canonical_runtime_backend_key, canonical_runtime_id};

/// llama.cpp backend using sidecar process management
///
/// This backend wraps the LlamaServer implementation,
/// which manages a llama-server binary as a sidecar process.
/// The sidecar exposes an OpenAI-compatible API that we forward
/// requests to.
pub struct LlamaCppBackend {
    /// The underlying server manager
    server: LlamaServer,
    /// HTTP client for API requests
    http_client: reqwest::Client,
    /// Process spawner (stored after start)
    spawner: Option<Arc<dyn ProcessSpawner>>,
}

impl LlamaCppBackend {
    /// Create a new llama.cpp backend
    pub fn new() -> Self {
        Self {
            server: LlamaServer::new(),
            http_client: reqwest::Client::new(),
            spawner: None,
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: true, // GGUF + mmproj support
            image_generation: false,
            embeddings: true,       // Via --embedding mode
            reranking: true,        // Via --reranking mode
            gpu: true,              // CUDA, Vulkan, Metal
            device_selection: true, // Manual device choice
            streaming: true,        // SSE streaming
            tool_calling: true,     // Via OpenAI-compatible API
            external_connection: true,
        }
    }

    fn normalize_rerank_results(
        json: serde_json::Value,
        documents: &[String],
        return_documents: bool,
    ) -> Result<RerankResponse, BackendError> {
        let (items, metadata) = if let Some(results) = json
            .get("results")
            .and_then(|value| value.as_array())
            .cloned()
        {
            let mut metadata = json;
            if let Some(object) = metadata.as_object_mut() {
                object.remove("results");
            }
            (results, metadata)
        } else if let Some(results) = json.as_array() {
            (results.clone(), serde_json::Value::Null)
        } else {
            return Err(BackendError::Inference(
                "Invalid rerank response format".to_string(),
            ));
        };

        let mut normalized = Vec::with_capacity(items.len());
        for item in items {
            let index =
                item.get("index").and_then(|v| v.as_u64()).ok_or_else(|| {
                    BackendError::Inference("Missing rerank result index".to_string())
                })? as usize;
            let score = item
                .get("score")
                .or_else(|| item.get("relevance_score"))
                .and_then(|v| v.as_f64())
                .ok_or_else(|| BackendError::Inference("Missing rerank score".to_string()))?
                as f32;
            let document = if return_documents {
                item.get("document")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| documents.get(index).cloned())
            } else {
                None
            };
            normalized.push(RerankResult {
                index,
                score,
                document,
            });
        }

        Ok(RerankResponse {
            results: normalized,
            metadata,
        })
    }

    async fn post_rerank_request(
        &self,
        url: &str,
        request: &serde_json::Value,
        documents: &[String],
        return_documents: bool,
    ) -> Result<RerankResponse, BackendError> {
        let response = self
            .http_client
            .post(url)
            .json(request)
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::Inference(format!(
                "Rerank API error {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendError::Inference(format!("Failed to parse response: {}", e)))?;
        Self::normalize_rerank_results(json, documents, return_documents)
    }

    /// Parse SSE stream into ChatChunk stream
    fn parse_sse_stream(
        response: reqwest::Response,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
        let stream = response.bytes_stream().map(|result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Parse SSE format: "data: {...}\n\n"
                    for line in text.lines() {
                        if let Some(data) = line.strip_prefix("data: ") {
                            if data == "[DONE]" {
                                return Ok(ChatChunk {
                                    content: None,
                                    done: true,
                                });
                            }

                            // Parse JSON and extract content
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                if let Some(content) = json
                                    .get("choices")
                                    .and_then(|c| c.get(0))
                                    .and_then(|c| c.get("delta"))
                                    .and_then(|d| d.get("content"))
                                    .and_then(|c| c.as_str())
                                {
                                    return Ok(ChatChunk {
                                        content: Some(content.to_string()),
                                        done: false,
                                    });
                                }
                            }
                        }
                    }

                    // No content in this chunk
                    Ok(ChatChunk {
                        content: None,
                        done: false,
                    })
                }
                Err(e) => Err(BackendError::Http(e)),
            }
        });

        Box::pin(stream)
    }

    fn kv_cache_runtime_fingerprint_for_mode(
        mode: &ServerMode,
        active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        let (model_path, mmproj_path, device) = match mode {
            ServerMode::SidecarInference {
                model_path,
                mmproj_path,
                device,
                ..
            } => (model_path.as_str(), mmproj_path.as_deref(), device),
            ServerMode::External { .. } => {
                return Err(BackendError::Inference(
                    "KV cache reuse is not supported for external llama.cpp runtimes".to_string(),
                ));
            }
            _ => {
                return Err(BackendError::Inference(
                    "KV cache reuse requires llama.cpp inference mode".to_string(),
                ));
            }
        };

        let context_size = active_config
            .and_then(|config| config.context_size)
            .unwrap_or(defaults::CONTEXT_SIZE);

        Ok(KvCacheRuntimeFingerprint {
            runtime_id: canonical_runtime_id("llama.cpp"),
            backend_key: canonical_runtime_backend_key("llama.cpp"),
            tokenizer_fingerprint: format!(
                "llamacpp:{}:{}:{}:{}",
                model_path, device.device, device.gpu_layers, context_size
            ),
            prompt_format_fingerprint: Some(if mmproj_path.is_some() {
                "llamacpp_completion_multimodal".to_string()
            } else {
                "llamacpp_completion".to_string()
            }),
            runtime_build_fingerprint: Some(format!("ctx-{}", context_size)),
        })
    }

    fn kv_cache_model_fingerprint_for_mode(
        mode: &ServerMode,
        active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        let (model_path, mmproj_path, device) = match mode {
            ServerMode::SidecarInference {
                model_path,
                mmproj_path,
                device,
                ..
            } => (model_path.as_str(), mmproj_path.as_deref(), device),
            ServerMode::External { .. } => {
                return Err(BackendError::Inference(
                    "KV cache model fingerprint is not supported for external llama.cpp runtimes"
                        .to_string(),
                ));
            }
            _ => {
                return Err(BackendError::Inference(
                    "KV cache model fingerprint requires llama.cpp inference mode".to_string(),
                ));
            }
        };

        let context_size = active_config
            .and_then(|config| config.context_size)
            .unwrap_or(defaults::CONTEXT_SIZE);

        Ok(ModelFingerprint {
            model_id: model_path.to_string(),
            config_hash: format!(
                "llamacpp:{}:{}:{}:{}:{}",
                model_path,
                mmproj_path.unwrap_or("none"),
                device.device,
                device.gpu_layers,
                context_size
            ),
        })
    }
}

impl Default for LlamaCppBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for LlamaCppBackend {
    fn name(&self) -> &'static str {
        "llama.cpp"
    }

    fn description(&self) -> &'static str {
        "Local llama.cpp server with GGUF model support. Supports CUDA, Vulkan, and Metal GPU backends."
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(
        &mut self,
        config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        // Store spawner for later use
        self.spawner = Some(spawner.clone());

        if let Some(external_url) = config.external_url.as_deref() {
            if config.embedding_mode || config.reranking_mode {
                return Err(BackendError::Config(
                    "external_url is only supported for inference mode".to_string(),
                ));
            }

            if self.server.matches_external_runtime(external_url) {
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.server
                .connect_external(external_url)
                .await
                .map_err(BackendError::StartupFailed)?;
            return Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            });
        }

        // Build device config from BackendConfig
        let device_config = DeviceConfig {
            device: config.device.clone().unwrap_or_else(|| "auto".to_string()),
            gpu_layers: config.gpu_layers.unwrap_or(-1),
        };

        if config.embedding_mode {
            // Start in embedding mode
            let model_path = config.model_path.as_ref().ok_or_else(|| {
                BackendError::Config("model_path required for embedding mode".to_string())
            })?;

            if self.server.matches_embedding_runtime(
                &model_path.to_string_lossy(),
                &device_config,
                config.port_override,
            ) {
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.server
                .start_sidecar_embedding(
                    spawner,
                    &model_path.to_string_lossy(),
                    &device_config,
                    config.port_override,
                )
                .await
                .map_err(|e| {
                    if e.to_lowercase().contains("out of memory")
                        || e.to_lowercase().contains("oom")
                    {
                        BackendError::OutOfMemory(e)
                    } else {
                        BackendError::StartupFailed(e)
                    }
                })?;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            })
        } else if config.reranking_mode {
            let model_path = config.model_path.as_ref().ok_or_else(|| {
                BackendError::Config("model_path required for reranking mode".to_string())
            })?;

            if self.server.matches_reranking_runtime(
                &model_path.to_string_lossy(),
                &device_config,
                config.port_override,
            ) {
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.server
                .start_sidecar_reranking(
                    spawner,
                    &model_path.to_string_lossy(),
                    &device_config,
                    config.port_override,
                )
                .await
                .map_err(|e| {
                    if e.to_lowercase().contains("out of memory")
                        || e.to_lowercase().contains("oom")
                    {
                        BackendError::OutOfMemory(e)
                    } else {
                        BackendError::StartupFailed(e)
                    }
                })?;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            })
        } else {
            // Start in inference mode (text LLM or VLM with optional vision)
            let model_path = config
                .model_path
                .as_ref()
                .ok_or_else(|| BackendError::Config("model_path required".to_string()))?;

            // mmproj_path is optional — only needed for vision/multimodal models
            let mmproj_path = config
                .mmproj_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            if self.server.matches_inference_runtime(
                &model_path.to_string_lossy(),
                mmproj_path.as_deref(),
                &device_config,
                config.port_override,
            ) {
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(true),
                    lifecycle_decision_reason: Some("runtime_reused".to_string()),
                });
            }

            self.server
                .start_sidecar_inference(
                    spawner,
                    &model_path.to_string_lossy(),
                    mmproj_path.as_deref(),
                    &device_config,
                    config.port_override,
                )
                .await
                .map_err(|e| {
                    if e.to_lowercase().contains("out of memory")
                        || e.to_lowercase().contains("oom")
                    {
                        BackendError::OutOfMemory(e)
                    } else {
                        BackendError::StartupFailed(e)
                    }
                })?;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            })
        }
    }

    fn stop(&mut self) {
        self.server.stop();
    }

    fn is_ready(&self) -> bool {
        self.server.is_ready()
    }

    async fn health_check(&self) -> bool {
        if let Some(base_url) = self.base_url() {
            let health_url = format!("{}/health", base_url);
            match self.http_client.get(&health_url).send().await {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }

    fn base_url(&self) -> Option<String> {
        self.server.base_url()
    }

    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        let base_url = self.base_url().ok_or_else(|| BackendError::NotReady)?;

        let url = format!("{}/v1/chat/completions", base_url);

        // Parse and ensure stream is enabled
        let mut request: serde_json::Value = serde_json::from_str(&request_json)
            .map_err(|e| BackendError::Inference(format!("Invalid request JSON: {}", e)))?;

        request["stream"] = serde_json::json!(true);

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::Inference(format!(
                "API error {}: {}",
                status, body
            )));
        }

        Ok(Self::parse_sse_stream(response))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        let base_url = self.base_url().ok_or_else(|| BackendError::NotReady)?;

        let url = format!("{}/v1/embeddings", base_url);

        let request = serde_json::json!({
            "input": texts,
            "model": model,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::Inference(format!(
                "Embedding API error {}: {}",
                status, body
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| BackendError::Inference(format!("Failed to parse response: {}", e)))?;

        // Parse OpenAI embedding response format
        let data = json.get("data").and_then(|d| d.as_array()).ok_or_else(|| {
            BackendError::Inference("Invalid embedding response format".to_string())
        })?;

        let mut results = Vec::new();
        for item in data {
            let embedding = item
                .get("embedding")
                .and_then(|e| e.as_array())
                .ok_or_else(|| BackendError::Inference("Missing embedding vector".to_string()))?;

            let vector: Vec<f32> = embedding
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();

            results.push(EmbeddingResult {
                vector,
                token_count: 0, // llama.cpp doesn't return token count
            });
        }

        Ok(results)
    }

    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, BackendError> {
        let base_url = self.base_url().ok_or(BackendError::NotReady)?;
        let mut body = serde_json::json!({
            "model": request.model,
            "query": request.query,
            "documents": request.documents,
            "top_n": request.top_n,
            "return_documents": request.return_documents,
            "return_text": request.return_documents,
        });

        if let Some(options) = request.extra_options.as_object() {
            for (key, value) in options {
                body[key] = value.clone();
            }
        }

        let documents = body
            .get("documents")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let preferred_url = format!("{}/v1/rerank", base_url);
        match self
            .post_rerank_request(&preferred_url, &body, &documents, request.return_documents)
            .await
        {
            Ok(response) => Ok(response),
            Err(BackendError::Inference(message))
                if message.contains(" 404 ")
                    || message.contains(" 405 ")
                    || message.contains("Not Found") =>
            {
                let fallback_url = format!("{}/reranking", base_url);
                self.post_rerank_request(&fallback_url, &body, &documents, request.return_documents)
                    .await
            }
            Err(error) => Err(error),
        }
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Self::kv_cache_runtime_fingerprint_for_mode(self.server.current_mode(), active_config)
    }

    async fn kv_cache_model_fingerprint(
        &self,
        active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Self::kv_cache_model_fingerprint_for_mode(self.server.current_mode(), active_config)
    }

    async fn save_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        let filename = path.to_str().ok_or_else(|| {
            BackendError::Config("KV cache slot path must be valid UTF-8".to_string())
        })?;
        self.server
            .save_slot(slot_id, filename)
            .await
            .map_err(BackendError::Inference)
    }

    async fn restore_kv_cache_slot(&self, slot_id: u32, path: &Path) -> Result<(), BackendError> {
        let filename = path.to_str().ok_or_else(|| {
            BackendError::Config("KV cache slot path must be valid UTF-8".to_string())
        })?;
        self.server
            .restore_slot(slot_id, filename)
            .await
            .map_err(BackendError::Inference)
    }

    async fn clear_kv_cache_slot(&self, slot_id: u32) -> Result<(), BackendError> {
        self.server
            .erase_slot(slot_id)
            .await
            .map_err(BackendError::Inference)
    }

    async fn truncate_kv_cache_data(
        &self,
        _data: &[u8],
        _token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        Err(BackendError::Inference(
            "KV cache truncation is not supported for llama.cpp slot snapshots".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::sync::mpsc;

    use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use crate::server::ServerMode;

    struct NoopProcessSpawner;

    #[async_trait]
    impl ProcessSpawner for NoopProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn_sidecar should not be called for reuse checks".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(std::env::temp_dir())
        }
    }

    #[test]
    fn test_backend_name() {
        let backend = LlamaCppBackend::new();
        assert_eq!(backend.name(), "llama.cpp");
    }

    #[test]
    fn test_capabilities() {
        let caps = LlamaCppBackend::static_capabilities();
        assert!(caps.vision);
        assert!(caps.embeddings);
        assert!(caps.gpu);
        assert!(caps.device_selection);
        assert!(caps.streaming);
        assert!(caps.tool_calling);
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = LlamaCppBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none());
    }

    #[test]
    fn kv_cache_runtime_fingerprint_reflects_active_inference_mode() {
        let mode = ServerMode::SidecarInference {
            port: 8080,
            model_path: "/models/main.gguf".to_string(),
            mmproj_path: None,
            device: DeviceConfig {
                device: "Vulkan0".to_string(),
                gpu_layers: 40,
            },
        };
        let config = BackendConfig {
            context_size: Some(8192),
            ..BackendConfig::default()
        };

        let fingerprint =
            LlamaCppBackend::kv_cache_runtime_fingerprint_for_mode(&mode, Some(&config))
                .expect("llama.cpp inference mode should support KV fingerprints");

        assert_eq!(fingerprint.runtime_id, "llama_cpp");
        assert_eq!(fingerprint.backend_key, "llama_cpp");
        assert!(
            fingerprint
                .tokenizer_fingerprint
                .contains("/models/main.gguf")
        );
        assert_eq!(
            fingerprint.prompt_format_fingerprint.as_deref(),
            Some("llamacpp_completion")
        );
        assert_eq!(
            fingerprint.runtime_build_fingerprint.as_deref(),
            Some("ctx-8192")
        );
    }

    #[test]
    fn kv_cache_model_fingerprint_reflects_context_and_device() {
        let mode = ServerMode::SidecarInference {
            port: 8080,
            model_path: "/models/main.gguf".to_string(),
            mmproj_path: Some("/models/vision.mmproj".to_string()),
            device: DeviceConfig {
                device: "auto".to_string(),
                gpu_layers: -1,
            },
        };

        let fingerprint = LlamaCppBackend::kv_cache_model_fingerprint_for_mode(&mode, None)
            .expect("llama.cpp inference mode should support model fingerprints");

        assert_eq!(fingerprint.model_id, "/models/main.gguf");
        assert!(fingerprint.config_hash.contains("/models/vision.mmproj"));
        assert!(fingerprint.config_hash.contains("auto"));
        assert!(
            fingerprint
                .config_hash
                .contains(&defaults::CONTEXT_SIZE.to_string())
        );
    }

    #[tokio::test]
    async fn test_reuses_matching_inference_runtime() {
        let mut backend = LlamaCppBackend::new();
        backend.server.set_test_runtime_state(
            ServerMode::SidecarInference {
                port: 11434,
                model_path: "/models/main.gguf".to_string(),
                mmproj_path: Some("/models/vision.mmproj".to_string()),
                device: DeviceConfig {
                    device: "Vulkan0".to_string(),
                    gpu_layers: 40,
                },
            },
            true,
        );

        let outcome = backend
            .start(
                &BackendConfig {
                    model_path: Some(PathBuf::from("/models/main.gguf")),
                    mmproj_path: Some(PathBuf::from("/models/vision.mmproj")),
                    device: Some("Vulkan0".to_string()),
                    gpu_layers: Some(40),
                    ..BackendConfig::default()
                },
                Arc::new(NoopProcessSpawner),
            )
            .await
            .expect("matching runtime should be reused");

        assert_eq!(outcome.runtime_reused, Some(true));
        assert_eq!(
            outcome.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
    }

    #[tokio::test]
    async fn test_does_not_reuse_inference_runtime_when_device_differs() {
        let mut backend = LlamaCppBackend::new();
        backend.server.set_test_runtime_state(
            ServerMode::SidecarInference {
                port: 11434,
                model_path: "/models/main.gguf".to_string(),
                mmproj_path: None,
                device: DeviceConfig {
                    device: "Vulkan0".to_string(),
                    gpu_layers: 40,
                },
            },
            true,
        );

        let error = backend
            .start(
                &BackendConfig {
                    model_path: Some(PathBuf::from("/models/main.gguf")),
                    device: Some("Vulkan1".to_string()),
                    gpu_layers: Some(40),
                    ..BackendConfig::default()
                },
                Arc::new(NoopProcessSpawner),
            )
            .await
            .expect_err("mismatched runtime should not be reused");

        assert!(
            matches!(error, BackendError::StartupFailed(ref message) if message.contains("spawn_sidecar")),
            "unexpected error: {error:?}"
        );
    }

    #[tokio::test]
    async fn test_does_not_reuse_inference_runtime_when_port_differs() {
        let mut backend = LlamaCppBackend::new();
        backend.server.set_test_runtime_state(
            ServerMode::SidecarInference {
                port: 11434,
                model_path: "/models/main.gguf".to_string(),
                mmproj_path: None,
                device: DeviceConfig {
                    device: "Vulkan0".to_string(),
                    gpu_layers: 40,
                },
            },
            true,
        );

        let error = backend
            .start(
                &BackendConfig {
                    model_path: Some(PathBuf::from("/models/main.gguf")),
                    device: Some("Vulkan0".to_string()),
                    gpu_layers: Some(40),
                    port_override: Some(18080),
                    ..BackendConfig::default()
                },
                Arc::new(NoopProcessSpawner),
            )
            .await
            .expect_err("mismatched port should not be reused");

        assert!(
            matches!(error, BackendError::StartupFailed(ref message) if message.contains("spawn_sidecar")),
            "unexpected error: {error:?}"
        );
    }

    #[tokio::test]
    async fn test_reuses_matching_external_runtime() {
        let mut backend = LlamaCppBackend::new();
        backend.server.set_test_runtime_state(
            ServerMode::External {
                url: "http://127.0.0.1:1234".to_string(),
            },
            true,
        );

        let outcome = backend
            .start(
                &BackendConfig {
                    external_url: Some("http://127.0.0.1:1234/".to_string()),
                    ..BackendConfig::default()
                },
                Arc::new(NoopProcessSpawner),
            )
            .await
            .expect("matching external runtime should be reused");

        assert_eq!(outcome.runtime_reused, Some(true));
        assert_eq!(
            outcome.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
    }

    #[tokio::test]
    async fn test_rejects_external_runtime_for_embedding_mode() {
        let mut backend = LlamaCppBackend::new();

        let error = backend
            .start(
                &BackendConfig {
                    external_url: Some("http://127.0.0.1:1234".to_string()),
                    embedding_mode: true,
                    ..BackendConfig::default()
                },
                Arc::new(NoopProcessSpawner),
            )
            .await
            .expect_err("external runtime should be rejected for embedding mode");

        assert!(
            matches!(error, BackendError::Config(ref message) if message.contains("inference mode")),
            "unexpected error: {error:?}"
        );
    }
}
