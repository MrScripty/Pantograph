//! Inference Gateway - Single entry point for all inference operations
//!
//! The gateway abstracts over different inference backends (llama.cpp, Ollama, Candle)
//! providing a unified interface for the rest of the application. It manages backend
//! lifecycle, switching, and forwards requests to the active backend.

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendInfo, BackendRegistry, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::config::EmbeddingMemoryMode;
use crate::process::ProcessSpawner;
use crate::types::{
    ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse, ServerModeInfo,
};

#[cfg(feature = "backend-llamacpp")]
use crate::backend::LlamaCppBackend;

/// Error types for gateway operations
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("Backend error: {0}")]
    Backend(#[from] BackendError),

    #[error("No backend active")]
    NoBackend,

    #[error("Backend switch failed: {0}")]
    SwitchFailed(String),

    #[error("No process spawner configured")]
    NoSpawner,
}

/// Host-supplied inputs for starting the active backend in inference mode.
#[derive(Debug, Clone, Default)]
pub struct InferenceStartRequest {
    pub external_url: Option<String>,
    pub file_model_path: Option<PathBuf>,
    pub mmproj_path: Option<PathBuf>,
    pub ollama_model_name: Option<String>,
    pub device: Option<String>,
    pub gpu_layers: Option<i32>,
}

/// Host-supplied inputs for starting the active backend in embedding mode.
#[derive(Debug, Clone, Default)]
pub struct EmbeddingStartRequest {
    pub gguf_model_path: Option<PathBuf>,
    pub candle_model_path: Option<PathBuf>,
    pub ollama_model_name: Option<String>,
    pub device: Option<String>,
    pub gpu_layers: Option<i32>,
}

/// Snapshot of the active runtime lifecycle owned by the inference gateway.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeLifecycleSnapshot {
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub warmup_started_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_completed_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_duration_ms: Option<u64>,
    #[serde(default)]
    pub runtime_reused: Option<bool>,
    #[serde(default)]
    pub lifecycle_decision_reason: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub last_error: Option<String>,
}

/// The single entry point for ALL inference operations.
///
/// Application code should only interact with InferenceGateway, never
/// with backends directly. The gateway handles backend lifecycle and
/// forwards requests to the active backend.
pub struct InferenceGateway {
    /// The currently active backend
    backend: Arc<RwLock<Box<dyn InferenceBackend>>>,
    /// Registry of available backends
    registry: BackendRegistry,
    /// Name of the current backend
    current_backend_name: Arc<RwLock<String>>,
    /// Whether running in embedding mode (for legacy compatibility)
    embedding_mode: Arc<RwLock<bool>>,
    /// Whether running in reranking mode
    reranking_mode: Arc<RwLock<bool>>,
    /// Whether the active runtime is an external host connection.
    external_mode: Arc<RwLock<bool>>,
    /// Last used inference config (for mode switching)
    last_inference_config: Arc<RwLock<Option<BackendConfig>>>,
    /// Current embedding memory mode
    embedding_memory_mode: Arc<RwLock<EmbeddingMemoryMode>>,
    /// Process spawner for starting backends
    spawner: Arc<RwLock<Option<Arc<dyn ProcessSpawner>>>>,
    /// Backend-owned lifecycle snapshot for the active runtime instance.
    runtime_lifecycle: Arc<RwLock<RuntimeLifecycleSnapshot>>,
    /// Monotonic instance counter for runtime instance IDs.
    runtime_instance_sequence: Arc<AtomicU64>,
}

impl InferenceGateway {
    /// Create a new gateway with llama.cpp as the default backend
    #[cfg(feature = "backend-llamacpp")]
    pub fn new() -> Self {
        Self {
            backend: Arc::new(RwLock::new(Box::new(LlamaCppBackend::new()))),
            registry: BackendRegistry::new(),
            current_backend_name: Arc::new(RwLock::new("llama.cpp".to_string())),
            embedding_mode: Arc::new(RwLock::new(false)),
            reranking_mode: Arc::new(RwLock::new(false)),
            external_mode: Arc::new(RwLock::new(false)),
            last_inference_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
            runtime_lifecycle: Arc::new(RwLock::new(RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                ..RuntimeLifecycleSnapshot::default()
            })),
            runtime_instance_sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Create a new gateway with a specific backend
    pub fn with_backend(backend: Box<dyn InferenceBackend>, name: &str) -> Self {
        Self {
            backend: Arc::new(RwLock::new(backend)),
            registry: BackendRegistry::new(),
            current_backend_name: Arc::new(RwLock::new(name.to_string())),
            embedding_mode: Arc::new(RwLock::new(false)),
            reranking_mode: Arc::new(RwLock::new(false)),
            external_mode: Arc::new(RwLock::new(false)),
            last_inference_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
            runtime_lifecycle: Arc::new(RwLock::new(RuntimeLifecycleSnapshot {
                runtime_id: Some(name.to_string()),
                ..RuntimeLifecycleSnapshot::default()
            })),
            runtime_instance_sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Set the process spawner
    ///
    /// This must be called before starting any backend that requires process spawning
    /// (e.g., llama.cpp, Ollama).
    pub async fn set_spawner(&self, spawner: Arc<dyn ProcessSpawner>) {
        let mut guard = self.spawner.write().await;
        *guard = Some(spawner);
    }

    /// Get the registry for backend information
    pub fn registry(&self) -> &BackendRegistry {
        &self.registry
    }

    /// Get the name of the currently active backend
    pub async fn current_backend_name(&self) -> String {
        self.current_backend_name.read().await.clone()
    }

    /// Build backend-owned startup config for inference mode using the active backend.
    pub async fn build_inference_start_config(
        &self,
        request: InferenceStartRequest,
    ) -> Result<BackendConfig, GatewayError> {
        let backend_name = self.current_backend_name().await;
        if let Some(external_url) = request.external_url {
            if backend_name != "llama.cpp" {
                return Err(GatewayError::Backend(BackendError::Config(format!(
                    "External server attachment is only supported for llama.cpp, but active backend is '{}'",
                    backend_name
                ))));
            }

            return Ok(BackendConfig {
                external_url: Some(external_url),
                embedding_mode: false,
                ..BackendConfig::default()
            });
        }

        match backend_name.as_str() {
            "Ollama" => {
                let model_name = request.ollama_model_name.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "Ollama VLM model not configured. Set a model like 'llava:13b' or 'qwen2-vl:7b' in Model Configuration.".to_string(),
                    ))
                })?;
                Ok(BackendConfig {
                    model_name: Some(model_name),
                    embedding_mode: false,
                    ..BackendConfig::default()
                })
            }
            _ => {
                let model_path = request.file_model_path.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "VLM model path not configured".to_string(),
                    ))
                })?;
                let mmproj_path = request.mmproj_path.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "VLM mmproj path not configured".to_string(),
                    ))
                })?;

                Ok(BackendConfig {
                    model_path: Some(model_path),
                    mmproj_path: Some(mmproj_path),
                    device: request.device,
                    gpu_layers: request.gpu_layers,
                    embedding_mode: false,
                    ..BackendConfig::default()
                })
            }
        }
    }

    /// Build backend-owned startup config for embedding mode using the active backend.
    pub async fn build_embedding_start_config(
        &self,
        request: EmbeddingStartRequest,
    ) -> Result<BackendConfig, GatewayError> {
        let backend_name = self.current_backend_name().await;
        match backend_name.as_str() {
            "Ollama" => {
                let model_name = request
                    .ollama_model_name
                    .unwrap_or_else(|| "nomic-embed-text".to_string());
                Ok(BackendConfig {
                    model_name: Some(model_name),
                    embedding_mode: true,
                    ..BackendConfig::default()
                })
            }
            "Candle" => {
                let model_path = request.candle_model_path.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "Candle embedding model path not configured. Download a SafeTensors model from HuggingFace (e.g., BAAI/bge-small-en-v1.5) and set the path in Settings.".to_string(),
                    ))
                })?;
                Ok(BackendConfig {
                    model_path: Some(model_path),
                    embedding_mode: true,
                    ..BackendConfig::default()
                })
            }
            _ => {
                let model_path = request.gguf_model_path.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "Embedding model path not configured".to_string(),
                    ))
                })?;
                Ok(BackendConfig {
                    model_path: Some(model_path),
                    device: request.device,
                    gpu_layers: request.gpu_layers,
                    embedding_mode: true,
                    ..BackendConfig::default()
                })
            }
        }
    }

    /// Switch to a different backend
    ///
    /// This stops the current backend and creates a new instance
    /// of the specified backend. The backend is not started - call
    /// `start()` after switching to initialize it.
    pub async fn switch_backend(&self, name: &str) -> Result<(), GatewayError> {
        // Create new backend first to validate the name
        let new_backend = self
            .registry
            .create(name)
            .map_err(|e| GatewayError::SwitchFailed(e.to_string()))?;

        // Stop current backend
        {
            let mut guard = self.backend.write().await;
            guard.stop();
            *guard = new_backend;
        }

        // Update current backend name
        {
            let mut name_guard = self.current_backend_name.write().await;
            *name_guard = name.to_string();
        }
        {
            let mut lifecycle = self.runtime_lifecycle.write().await;
            *lifecycle = RuntimeLifecycleSnapshot {
                runtime_id: Some(name.to_string()),
                ..RuntimeLifecycleSnapshot::default()
            };
        }
        {
            let mut mode = self.external_mode.write().await;
            *mode = false;
        }

        log::info!("Switched to backend: {}", name);
        Ok(())
    }

    /// List all available backends with their info
    pub fn available_backends(&self) -> Vec<BackendInfo> {
        self.registry.list()
    }

    // ─── LIFECYCLE METHODS ──────────────────────────────────────────

    /// Start the current backend with the given configuration
    pub async fn start(&self, config: &BackendConfig) -> Result<(), GatewayError> {
        // Get the spawner
        let spawner = {
            let guard = self.spawner.read().await;
            guard.clone().ok_or(GatewayError::NoSpawner)?
        };

        // Track embedding mode
        {
            let mut mode = self.embedding_mode.write().await;
            *mode = config.embedding_mode;
        }
        {
            let mut mode = self.reranking_mode.write().await;
            *mode = config.reranking_mode;
        }
        {
            let mut mode = self.external_mode.write().await;
            *mode = config.external_url.is_some();
        }

        // Store inference config for mode restoration
        if !config.embedding_mode && !config.reranking_mode {
            let mut last_config = self.last_inference_config.write().await;
            *last_config = Some(config.clone());
        }

        let runtime_id = self.current_backend_name().await;
        let warmup_started_at_ms = unix_timestamp_ms();
        let previous_runtime_instance_id = {
            let lifecycle = self.runtime_lifecycle.read().await;
            if lifecycle.active && lifecycle.runtime_id.as_deref() == Some(runtime_id.as_str()) {
                lifecycle.runtime_instance_id.clone()
            } else {
                None
            }
        };
        {
            let mut lifecycle = self.runtime_lifecycle.write().await;
            lifecycle.runtime_id = Some(runtime_id.clone());
            lifecycle.runtime_instance_id = None;
            lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
            lifecycle.warmup_completed_at_ms = None;
            lifecycle.warmup_duration_ms = None;
            lifecycle.runtime_reused = None;
            lifecycle.lifecycle_decision_reason = None;
            lifecycle.active = false;
            lifecycle.last_error = None;
        }

        let start_result = {
            let mut guard = self.backend.write().await;
            guard.start(config, spawner).await
        };

        match start_result {
            Ok(start_outcome) => {
                let warmup_completed_at_ms = unix_timestamp_ms();
                let runtime_reused = start_outcome
                    .runtime_reused
                    .unwrap_or(previous_runtime_instance_id.is_some());
                let runtime_instance_id = if runtime_reused {
                    previous_runtime_instance_id.unwrap_or_else(|| {
                        format!(
                            "{}-{}",
                            runtime_id.replace([' ', '.'], "-"),
                            self.runtime_instance_sequence
                                .fetch_add(1, Ordering::Relaxed)
                                + 1
                        )
                    })
                } else {
                    format!(
                        "{}-{}",
                        runtime_id.replace([' ', '.'], "-"),
                        self.runtime_instance_sequence
                            .fetch_add(1, Ordering::Relaxed)
                            + 1
                    )
                };
                let mut lifecycle = self.runtime_lifecycle.write().await;
                lifecycle.runtime_id = Some(runtime_id);
                lifecycle.runtime_instance_id = Some(runtime_instance_id);
                lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
                lifecycle.warmup_completed_at_ms = Some(warmup_completed_at_ms);
                lifecycle.warmup_duration_ms =
                    Some(warmup_completed_at_ms.saturating_sub(warmup_started_at_ms));
                lifecycle.runtime_reused = Some(runtime_reused);
                lifecycle.lifecycle_decision_reason =
                    Some(start_outcome.lifecycle_decision_reason.unwrap_or_else(|| {
                        if runtime_reused {
                            "runtime_reused".to_string()
                        } else {
                            "runtime_ready".to_string()
                        }
                    }));
                lifecycle.active = true;
                lifecycle.last_error = None;
                Ok(())
            }
            Err(error) => {
                let completed_at_ms = unix_timestamp_ms();
                let mut lifecycle = self.runtime_lifecycle.write().await;
                lifecycle.runtime_id = Some(runtime_id);
                lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
                lifecycle.warmup_completed_at_ms = Some(completed_at_ms);
                lifecycle.warmup_duration_ms =
                    Some(completed_at_ms.saturating_sub(warmup_started_at_ms));
                lifecycle.active = false;
                lifecycle.lifecycle_decision_reason = Some("runtime_start_failed".to_string());
                lifecycle.last_error = Some(error.to_string());
                Err(GatewayError::Backend(error))
            }
        }
    }

    /// Stop the current backend
    pub async fn stop(&self) {
        let mut guard = self.backend.write().await;
        guard.stop();
        // Reset embedding mode
        let mut mode = self.embedding_mode.write().await;
        *mode = false;
        let mut reranking_mode = self.reranking_mode.write().await;
        *reranking_mode = false;
        let mut external_mode = self.external_mode.write().await;
        *external_mode = false;
        let mut lifecycle = self.runtime_lifecycle.write().await;
        lifecycle.active = false;
    }

    /// Check if currently in embedding mode
    pub async fn is_embedding_mode(&self) -> bool {
        *self.embedding_mode.read().await
    }

    /// Check if currently in inference mode (ready and not embedding)
    pub async fn is_inference_mode(&self) -> bool {
        self.is_ready().await && !self.is_embedding_mode().await && !self.is_reranking_mode().await
    }

    /// Check if currently in reranking mode
    pub async fn is_reranking_mode(&self) -> bool {
        *self.reranking_mode.read().await
    }

    /// Check if currently connected to an external runtime host.
    pub async fn is_external_mode(&self) -> bool {
        *self.external_mode.read().await
    }

    /// Get the last inference config (for restoring after embedding mode)
    pub async fn last_inference_config(&self) -> Option<BackendConfig> {
        self.last_inference_config.read().await.clone()
    }

    /// Get server mode info (for legacy compatibility)
    pub async fn mode_info(&self) -> ServerModeInfo {
        let backend_name = self.current_backend_name().await;
        let ready = self.is_ready().await;
        let is_embedding = self.is_embedding_mode().await;
        let is_reranking = self.is_reranking_mode().await;
        let is_external = self.is_external_mode().await;
        let url = self.base_url().await;

        ServerModeInfo {
            backend_name: Some(backend_name),
            mode: if !ready {
                "none".to_string()
            } else if is_external {
                "external".to_string()
            } else if is_embedding {
                "sidecar_embedding".to_string()
            } else if is_reranking {
                "sidecar_reranking".to_string()
            } else {
                "sidecar_inference".to_string()
            },
            ready,
            url,
            model_path: None,
            is_embedding_mode: is_embedding,
        }
    }

    /// Check if the current backend is ready
    pub async fn is_ready(&self) -> bool {
        let guard = self.backend.read().await;
        guard.is_ready()
    }

    /// Health check the current backend
    pub async fn health_check(&self) -> bool {
        let guard = self.backend.read().await;
        guard.health_check().await
    }

    /// Get the base URL of the current backend (if HTTP-based)
    pub async fn base_url(&self) -> Option<String> {
        let guard = self.backend.read().await;
        guard.base_url()
    }

    /// Get capabilities of the current backend
    pub async fn capabilities(&self) -> BackendCapabilities {
        let guard = self.backend.read().await;
        guard.capabilities()
    }

    /// Get the backend-owned runtime lifecycle snapshot.
    pub async fn runtime_lifecycle_snapshot(&self) -> RuntimeLifecycleSnapshot {
        self.runtime_lifecycle.read().await.clone()
    }

    /// Get the current embedding memory mode
    pub async fn embedding_memory_mode(&self) -> EmbeddingMemoryMode {
        self.embedding_memory_mode.read().await.clone()
    }

    /// Set the embedding memory mode
    pub async fn set_embedding_memory_mode(&self, mode: EmbeddingMemoryMode) {
        let mut guard = self.embedding_memory_mode.write().await;
        *guard = mode;
    }

    // ─── INFERENCE METHODS ──────────────────────────────────────────

    /// Stream chat completion responses
    ///
    /// Takes a JSON-serialized OpenAI-compatible request and returns
    /// a stream of response chunks.
    pub async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, GatewayError>
    {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard
            .chat_completion_stream(request_json)
            .await
            .map_err(GatewayError::Backend)
    }

    /// Generate embeddings for the given texts
    pub async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, GatewayError> {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard
            .embeddings(texts, model)
            .await
            .map_err(GatewayError::Backend)
    }

    /// Rank documents through the active backend.
    pub async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, GatewayError> {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard.rerank(request).await.map_err(GatewayError::Backend)
    }

    /// Generate one or more images through the active backend.
    pub async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, GatewayError> {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard
            .generate_image(request)
            .await
            .map_err(GatewayError::Backend)
    }

    // ─── LEGACY COMPATIBILITY ───────────────────────────────────────

    /// Get a reference to the underlying backend for legacy code
    ///
    /// This is a temporary method for gradual migration. New code should
    /// use the gateway methods directly.
    pub fn backend(&self) -> Arc<RwLock<Box<dyn InferenceBackend>>> {
        self.backend.clone()
    }
}

fn unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(feature = "backend-llamacpp")]
impl Default for InferenceGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared gateway type for application state
pub type SharedGateway = Arc<InferenceGateway>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures_util::stream;
    use tokio::sync::mpsc;

    use crate::backend::BackendStartOutcome;

    struct MockImageBackend;
    struct MockReusedBackend;

    struct MockProcessHandle;

    impl crate::process::ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            1
        }

        fn kill(&self) -> Result<(), String> {
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
        ) -> Result<
            (
                mpsc::Receiver<crate::process::ProcessEvent>,
                Box<dyn crate::process::ProcessHandle>,
            ),
            String,
        > {
            let (_tx, rx) = mpsc::channel(1);
            Ok((rx, Box::new(MockProcessHandle)))
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    #[async_trait]
    impl InferenceBackend for MockImageBackend {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn description(&self) -> &'static str {
            "Mock image backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities {
                image_generation: true,
                ..BackendCapabilities::default()
            }
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
            })
        }

        fn stop(&mut self) {}

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
        {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }

        async fn generate_image(
            &self,
            request: ImageGenerationRequest,
        ) -> Result<ImageGenerationResult, BackendError> {
            Ok(ImageGenerationResult {
                images: vec![crate::types::EncodedImage {
                    data_base64: request.prompt,
                    mime_type: "image/png".to_string(),
                    width: Some(512),
                    height: Some(512),
                }],
                seed_used: Some(7),
                metadata: serde_json::Value::Null,
            })
        }
    }

    #[async_trait]
    impl InferenceBackend for MockReusedBackend {
        fn name(&self) -> &'static str {
            "MockReused"
        }

        fn description(&self) -> &'static str {
            "Mock reused backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            Ok(BackendStartOutcome {
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("reused_mock_runtime".to_string()),
            })
        }

        fn stop(&mut self) {}

        fn is_ready(&self) -> bool {
            true
        }

        async fn health_check(&self) -> bool {
            true
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
        {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }
    }

    #[cfg(feature = "backend-llamacpp")]
    #[test]
    fn test_gateway_creation() {
        let gateway = InferenceGateway::new();
        // Registry should have at least llama.cpp
        assert!(!gateway.registry.list().is_empty());
    }

    #[cfg(feature = "backend-llamacpp")]
    #[tokio::test]
    async fn test_initial_backend_is_llamacpp() {
        let gateway = InferenceGateway::new();
        let name = gateway.current_backend_name().await;
        assert_eq!(name, "llama.cpp");
    }

    #[cfg(feature = "backend-llamacpp")]
    #[tokio::test]
    async fn test_not_ready_initially() {
        let gateway = InferenceGateway::new();
        assert!(!gateway.is_ready().await);
    }

    #[tokio::test]
    async fn test_generate_image_forwards_to_active_backend() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
        let result = gateway
            .generate_image(ImageGenerationRequest {
                model: "mock".to_string(),
                prompt: "paper lantern".to_string(),
                negative_prompt: None,
                width: Some(512),
                height: Some(512),
                num_inference_steps: Some(20),
                guidance_scale: Some(4.0),
                seed: Some(7),
                scheduler: None,
                num_images_per_prompt: Some(1),
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::Value::Null,
            })
            .await
            .unwrap();

        assert_eq!(result.seed_used, Some(7));
        assert_eq!(result.images.len(), 1);
        assert_eq!(result.images[0].data_base64, "paper lantern");
    }

    #[tokio::test]
    async fn test_rerank_forwards_to_active_backend() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
        let result = gateway
            .rerank(RerankRequest {
                model: "mock".to_string(),
                query: "alpha".to_string(),
                documents: vec!["a".to_string()],
                top_n: Some(1),
                return_documents: true,
                extra_options: serde_json::Value::Null,
            })
            .await
            .expect("rerank should forward");
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn test_runtime_lifecycle_snapshot_tracks_start_and_stop() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let started = gateway.runtime_lifecycle_snapshot().await;
        assert_eq!(started.runtime_id.as_deref(), Some("mock"));
        assert!(started.runtime_instance_id.is_some());
        assert!(started.warmup_started_at_ms.is_some());
        assert!(started.warmup_completed_at_ms.is_some());
        assert!(started.warmup_duration_ms.is_some());
        assert_eq!(started.runtime_reused, Some(false));
        assert_eq!(
            started.lifecycle_decision_reason.as_deref(),
            Some("started_mock_runtime")
        );
        assert!(started.active);
        assert!(started.last_error.is_none());

        gateway.stop().await;

        let stopped = gateway.runtime_lifecycle_snapshot().await;
        assert_eq!(stopped.runtime_id.as_deref(), Some("mock"));
        assert!(!stopped.active);
    }

    #[tokio::test]
    async fn test_runtime_lifecycle_snapshot_preserves_instance_id_for_reused_runtime() {
        let gateway = InferenceGateway::with_backend(Box::new(MockReusedBackend), "mock");
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");
        let first = gateway.runtime_lifecycle_snapshot().await;

        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should reuse");
        let second = gateway.runtime_lifecycle_snapshot().await;

        assert_eq!(first.runtime_id.as_deref(), Some("mock"));
        assert_eq!(second.runtime_id.as_deref(), Some("mock"));
        assert_eq!(second.runtime_reused, Some(true));
        assert_eq!(second.runtime_instance_id, first.runtime_instance_id);
        assert_eq!(
            second.lifecycle_decision_reason.as_deref(),
            Some("reused_mock_runtime")
        );
    }

    #[tokio::test]
    async fn test_mode_info_reports_external_runtime_from_start_config() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

        gateway
            .start(&BackendConfig {
                external_url: Some("http://127.0.0.1:1234".to_string()),
                ..BackendConfig::default()
            })
            .await
            .expect("gateway should start");

        let mode = gateway.mode_info().await;
        assert_eq!(mode.backend_name.as_deref(), Some("mock"));
        assert_eq!(mode.mode, "external");
        assert!(!mode.is_embedding_mode);
    }

    #[tokio::test]
    async fn test_build_inference_start_config_for_ollama_uses_model_name() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Ollama");

        let config = gateway
            .build_inference_start_config(InferenceStartRequest {
                external_url: None,
                ollama_model_name: Some("llava:13b".to_string()),
                ..InferenceStartRequest::default()
            })
            .await
            .expect("config should build");

        assert_eq!(config.model_name.as_deref(), Some("llava:13b"));
        assert_eq!(config.model_path, None);
        assert!(!config.embedding_mode);
    }

    #[tokio::test]
    async fn test_build_inference_start_config_for_external_llamacpp_uses_external_url() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "llama.cpp");

        let config = gateway
            .build_inference_start_config(InferenceStartRequest {
                external_url: Some("http://127.0.0.1:1234".to_string()),
                ..InferenceStartRequest::default()
            })
            .await
            .expect("config should build");

        assert_eq!(
            config.external_url.as_deref(),
            Some("http://127.0.0.1:1234")
        );
        assert_eq!(config.model_path, None);
        assert!(!config.embedding_mode);
    }

    #[tokio::test]
    async fn test_build_embedding_start_config_for_candle_uses_candle_model_path() {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Candle");

        let config = gateway
            .build_embedding_start_config(EmbeddingStartRequest {
                candle_model_path: Some(PathBuf::from("/models/bge-small-en-v1.5")),
                ..EmbeddingStartRequest::default()
            })
            .await
            .expect("config should build");

        assert_eq!(
            config
                .model_path
                .as_ref()
                .map(|path| path.to_string_lossy().to_string()),
            Some("/models/bge-small-en-v1.5".to_string())
        );
        assert!(config.embedding_mode);
    }
}
