//! Ollama backend implementation
//!
//! This backend integrates with the Ollama daemon for inference.
//! It supports automatic model management and the Ollama model registry.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};

use super::{
    BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use crate::types::{RerankRequest, RerankResponse};

/// Ollama backend using the Ollama daemon
///
/// This backend communicates with an Ollama server via HTTP.
/// If Ollama is not running, it can optionally start it automatically.
pub struct OllamaBackend {
    /// HTTP client for API requests
    http_client: reqwest::Client,
    /// Base URL of the Ollama server
    base_url: Option<String>,
    /// Whether the backend is ready
    ready: bool,
    /// Process spawner (stored after start)
    spawner: Option<Arc<dyn ProcessSpawner>>,
    /// Managed Ollama child process started by this backend, if any.
    managed_child: Option<Box<dyn ProcessHandle>>,
}

impl OllamaBackend {
    const DEFAULT_BASE_URL: &'static str = "http://127.0.0.1:11434";

    /// Create a new Ollama backend
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url: None,
            ready: false,
            spawner: None,
            managed_child: None,
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: true, // Ollama supports multimodal models
            image_generation: false,
            embeddings: true, // Via embedding API
            reranking: false,
            gpu: true,               // Ollama handles GPU automatically
            device_selection: false, // Ollama manages devices internally
            streaming: true,         // SSE streaming
            tool_calling: true,      // Via OpenAI-compatible API
        }
    }

    /// Check if Ollama is available on the system
    pub fn check_availability() -> (bool, Option<String>) {
        if which::which("ollama").is_ok() {
            (true, None)
        } else {
            (
                false,
                Some(
                    "Ollama not found in PATH. Install it or download it through Pantograph."
                        .to_string(),
                ),
            )
        }
    }

    /// Check if auto-installation is supported
    pub fn can_auto_install() -> bool {
        cfg!(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64")
        ))
    }

    async fn tags_ready(&self, base_url: &str) -> bool {
        let health_url = format!("{}/api/tags", base_url);
        match self.http_client.get(&health_url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    async fn wait_for_ready(&self, base_url: &str, timeout_ms: u64) -> Result<(), BackendError> {
        let started = std::time::Instant::now();
        while started.elapsed().as_millis() < timeout_ms as u128 {
            if self.tags_ready(base_url).await {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }

        Err(BackendError::StartupFailed(format!(
            "Ollama server did not become ready within {}ms",
            timeout_ms
        )))
    }

    fn drain_process_events(mut rx: tokio::sync::mpsc::Receiver<ProcessEvent>) {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    ProcessEvent::Stdout(line) => {
                        log::debug!("[ollama] {}", String::from_utf8_lossy(&line));
                    }
                    ProcessEvent::Stderr(line) => {
                        log::debug!("[ollama stderr] {}", String::from_utf8_lossy(&line));
                    }
                    ProcessEvent::Terminated(code) => {
                        log::warn!("Managed Ollama process terminated: {:?}", code);
                        break;
                    }
                    ProcessEvent::Error(error) => {
                        log::warn!("Managed Ollama process error: {}", error);
                        break;
                    }
                }
            }
        });
    }

    /// Parse SSE stream into ChatChunk stream
    fn parse_sse_stream(
        response: reqwest::Response,
    ) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>> {
        let stream = response.bytes_stream().map(|result| {
            match result {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);

                    // Parse SSE format
                    for line in text.lines() {
                        if line.starts_with("data: ") {
                            let data = &line[6..];
                            if data == "[DONE]" {
                                return Ok(ChatChunk {
                                    content: None,
                                    done: true,
                                });
                            }

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

impl Default for OllamaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for OllamaBackend {
    fn name(&self) -> &'static str {
        "Ollama"
    }

    fn description(&self) -> &'static str {
        "Ollama daemon with automatic model management. Models are pulled from the Ollama registry."
    }

    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.stop();
        self.spawner = Some(spawner.clone());
        let base_url = Self::DEFAULT_BASE_URL.to_string();

        if self.tags_ready(&base_url).await {
            self.base_url = Some(base_url);
            self.ready = true;
            log::info!("Connected to existing Ollama server");
            return Ok(BackendStartOutcome {
                runtime_reused: Some(true),
            });
        }

        let (rx, child) = spawner
            .spawn_sidecar("ollama", &["serve"])
            .await
            .map_err(BackendError::StartupFailed)?;
        Self::drain_process_events(rx);
        self.managed_child = Some(child);

        if let Err(error) = self.wait_for_ready(&base_url, 30_000).await {
            if let Some(child) = self.managed_child.take() {
                let _ = child.kill();
            }
            return Err(error);
        }

        self.base_url = Some(base_url);
        self.ready = true;
        log::info!("Started managed Ollama server");
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
        })
    }

    fn stop(&mut self) {
        if let Some(child) = self.managed_child.take() {
            let _ = child.kill();
        }
        self.base_url = None;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        if let Some(ref base_url) = self.base_url {
            self.tags_ready(base_url).await
        } else {
            false
        }
    }

    fn base_url(&self) -> Option<String> {
        self.base_url.clone()
    }

    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        let base_url = self.base_url.as_ref().ok_or(BackendError::NotReady)?;

        // Use OpenAI-compatible endpoint
        let url = format!("{}/v1/chat/completions", base_url);

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
        let base_url = self.base_url.as_ref().ok_or(BackendError::NotReady)?;

        // Use OpenAI-compatible embeddings endpoint
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
                token_count: 0,
            });
        }

        Ok(results)
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Err(BackendError::Inference(
            "Reranking not supported by Ollama backend".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_name() {
        let backend = OllamaBackend::new();
        assert_eq!(backend.name(), "Ollama");
    }

    #[test]
    fn test_capabilities() {
        let caps = OllamaBackend::static_capabilities();
        assert!(caps.vision);
        assert!(caps.embeddings);
        assert!(caps.gpu);
        assert!(!caps.device_selection); // Ollama manages devices internally
        assert!(caps.streaming);
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = OllamaBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none());
    }
}
