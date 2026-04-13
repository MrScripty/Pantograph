//! llama.cpp backend implementation
//!
//! This backend wraps the LlamaServer sidecar management code,
//! providing the InferenceBackend trait interface.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::config::DeviceConfig;
use crate::process::ProcessSpawner;
use crate::server::LlamaServer;
use crate::types::{RerankRequest, RerankResponse, RerankResult};

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
                        if line.starts_with("data: ") {
                            let data = &line[6..];
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

            self.server
                .start_sidecar_embedding(spawner, &model_path.to_string_lossy(), &device_config)
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
            })
        } else if config.reranking_mode {
            let model_path = config.model_path.as_ref().ok_or_else(|| {
                BackendError::Config("model_path required for reranking mode".to_string())
            })?;

            self.server
                .start_sidecar_reranking(spawner, &model_path.to_string_lossy(), &device_config)
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

            self.server
                .start_sidecar_inference(
                    spawner,
                    &model_path.to_string_lossy(),
                    mmproj_path.as_deref(),
                    &device_config,
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
