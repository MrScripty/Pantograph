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
use pantograph_runtime_identity::canonical_runtime_id;
use tokio::sync::RwLock;

use crate::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendInfo, BackendRegistry, ChatChunk,
    EmbeddingResult, InferenceBackend, canonical_backend_key,
};
use crate::config::EmbeddingMemoryMode;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::process::ProcessSpawner;
use crate::types::{
    ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse,
    RuntimeLifecycleSnapshot, ServerModeInfo,
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

/// Result of switching the active backend into embedding mode.
#[derive(Debug, Clone, Default)]
pub struct EmbeddingRuntimePreparation {
    pub backend_name: String,
    pub restore_config: Option<BackendConfig>,
    pub base_url: Option<String>,
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
    /// Current config for the actively loaded runtime.
    current_runtime_config: Arc<RwLock<Option<BackendConfig>>>,
    /// Current embedding memory mode
    embedding_memory_mode: Arc<RwLock<EmbeddingMemoryMode>>,
    /// Process spawner for starting backends
    spawner: Arc<RwLock<Option<Arc<dyn ProcessSpawner>>>>,
    /// Backend-owned lifecycle snapshot for the active runtime instance.
    runtime_lifecycle: Arc<RwLock<RuntimeLifecycleSnapshot>>,
    /// Monotonic instance counter for runtime instance IDs.
    runtime_instance_sequence: Arc<AtomicU64>,
}

fn config_model_target(config: &BackendConfig) -> Option<String> {
    config
        .model_path
        .as_ref()
        .map(|path| path.display().to_string())
        .or_else(|| config.model_name.clone())
        .or_else(|| config.model_id.clone())
}

fn runtime_id_for_backend_name(backend_name: &str) -> String {
    canonical_runtime_id(backend_name)
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
            current_runtime_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
            runtime_lifecycle: Arc::new(RwLock::new(RuntimeLifecycleSnapshot {
                runtime_id: Some(runtime_id_for_backend_name("llama.cpp")),
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
            current_runtime_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
            runtime_lifecycle: Arc::new(RwLock::new(RuntimeLifecycleSnapshot {
                runtime_id: Some(runtime_id_for_backend_name(name)),
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

    pub async fn kv_cache_runtime_fingerprint(
        &self,
    ) -> Result<KvCacheRuntimeFingerprint, GatewayError> {
        let active_config = self.current_runtime_config.read().await.clone();
        self.backend
            .read()
            .await
            .kv_cache_runtime_fingerprint(active_config.as_ref())
            .await
            .map_err(GatewayError::Backend)
    }

    pub async fn kv_cache_model_fingerprint(&self) -> Result<ModelFingerprint, GatewayError> {
        let active_config = self.current_runtime_config.read().await.clone();
        self.backend
            .read()
            .await
            .kv_cache_model_fingerprint(active_config.as_ref())
            .await
            .map_err(GatewayError::Backend)
    }

    pub async fn save_kv_cache_slot(
        &self,
        slot_id: u32,
        path: &std::path::Path,
    ) -> Result<(), GatewayError> {
        self.backend
            .read()
            .await
            .save_kv_cache_slot(slot_id, path)
            .await
            .map_err(GatewayError::Backend)
    }

    pub async fn restore_kv_cache_slot(
        &self,
        slot_id: u32,
        path: &std::path::Path,
    ) -> Result<(), GatewayError> {
        self.backend
            .read()
            .await
            .restore_kv_cache_slot(slot_id, path)
            .await
            .map_err(GatewayError::Backend)
    }

    pub async fn clear_kv_cache_slot(&self, slot_id: u32) -> Result<(), GatewayError> {
        self.backend
            .read()
            .await
            .clear_kv_cache_slot(slot_id)
            .await
            .map_err(GatewayError::Backend)
    }

    pub async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
    ) -> Result<Vec<u8>, GatewayError> {
        let active_config = self.current_runtime_config.read().await.clone();
        self.backend
            .read()
            .await
            .truncate_kv_cache_data(data, token_position, active_config.as_ref())
            .await
            .map_err(GatewayError::Backend)
    }

    /// Build backend-owned startup config for inference mode using the active backend.
    pub async fn build_inference_start_config(
        &self,
        request: InferenceStartRequest,
    ) -> Result<BackendConfig, GatewayError> {
        let backend_name = self.current_backend_name().await;
        if let Some(external_url) = request.external_url {
            let supports_external_connection =
                self.backend.read().await.capabilities().external_connection;
            if !supports_external_connection {
                return Err(GatewayError::Backend(BackendError::Config(format!(
                    "External server attachment is not supported for active backend '{}'",
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
            "PyTorch" => {
                let model_path = request.file_model_path.ok_or_else(|| {
                    GatewayError::Backend(BackendError::Config(
                        "PyTorch model path not configured. Set a local model directory in Model Configuration.".to_string(),
                    ))
                })?;

                Ok(BackendConfig {
                    model_path: Some(model_path),
                    device: request.device,
                    embedding_mode: false,
                    ..BackendConfig::default()
                })
            }
            "Candle" => Err(GatewayError::Backend(BackendError::Config(
                "Candle does not support inference mode. Use embedding mode with a SafeTensors embedding model instead.".to_string(),
            ))),
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
            "PyTorch" => Err(GatewayError::Backend(BackendError::Config(
                "PyTorch does not support embedding mode. Use llama.cpp, Ollama, or Candle for embeddings.".to_string(),
            ))),
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

    /// Start the active backend in embedding mode and capture restore context.
    pub async fn prepare_embedding_runtime(
        &self,
        request: EmbeddingStartRequest,
    ) -> Result<EmbeddingRuntimePreparation, GatewayError> {
        let backend_name = self.current_backend_name().await;
        if self.is_ready().await && self.is_embedding_mode().await {
            return Ok(EmbeddingRuntimePreparation {
                backend_name,
                restore_config: None,
                base_url: self.base_url().await,
            });
        }

        let restore_config = if self.is_ready().await && !self.is_embedding_mode().await {
            self.last_inference_config().await
        } else {
            None
        };
        let config = self.build_embedding_start_config(request).await?;
        self.start(&config).await?;

        Ok(EmbeddingRuntimePreparation {
            backend_name,
            restore_config,
            base_url: self.base_url().await,
        })
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
        let canonical_backend_name = new_backend.name().to_string();

        // Stop current backend
        {
            let mut guard = self.backend.write().await;
            guard.stop();
            *guard = new_backend;
        }

        // Update current backend name
        {
            let mut name_guard = self.current_backend_name.write().await;
            *name_guard = canonical_backend_name.clone();
        }
        {
            let mut lifecycle = self.runtime_lifecycle.write().await;
            *lifecycle = RuntimeLifecycleSnapshot {
                runtime_id: Some(runtime_id_for_backend_name(&canonical_backend_name)),
                ..RuntimeLifecycleSnapshot::default()
            };
        }
        {
            let mut mode = self.external_mode.write().await;
            *mode = false;
        }

        log::info!(
            "Switched to backend '{}' (requested as '{}')",
            canonical_backend_name,
            name
        );
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

        let runtime_id = runtime_id_for_backend_name(&self.current_backend_name().await);
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
                let mut current_runtime_config = self.current_runtime_config.write().await;
                *current_runtime_config = Some(config.clone());
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
                lifecycle.active = true;
                lifecycle.last_error = None;
                lifecycle.lifecycle_decision_reason = start_outcome.lifecycle_decision_reason;
                lifecycle.lifecycle_decision_reason =
                    lifecycle.normalized_lifecycle_decision_reason();
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
                lifecycle.last_error = Some(error.to_string());
                lifecycle.lifecycle_decision_reason =
                    lifecycle.normalized_lifecycle_decision_reason();
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
        let mut current_runtime_config = self.current_runtime_config.write().await;
        *current_runtime_config = None;
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

    /// Get the saved config for the currently active runtime, if any.
    ///
    /// Recovery flows should read this before stopping the runtime when they
    /// intend to restart the exact active mode.
    pub async fn restart_runtime_config(&self) -> Option<BackendConfig> {
        self.current_runtime_config.read().await.clone()
    }

    /// Restore the last non-embedding inference runtime when available.
    pub async fn restore_inference_runtime(
        &self,
        restore_config: Option<BackendConfig>,
    ) -> Result<(), GatewayError> {
        if let Some(config) = restore_config {
            self.start(&config).await?;
        }
        Ok(())
    }

    /// Get server mode info (for legacy compatibility)
    pub async fn mode_info(&self) -> ServerModeInfo {
        let backend_name = self.current_backend_name().await;
        let ready = self.is_ready().await;
        let is_embedding = self.is_embedding_mode().await;
        let is_reranking = self.is_reranking_mode().await;
        let is_external = self.is_external_mode().await;
        let url = self.base_url().await;
        let active_model_target = self
            .current_runtime_config
            .read()
            .await
            .as_ref()
            .and_then(config_model_target);
        let backend_key = canonical_backend_key(&backend_name);

        ServerModeInfo {
            backend_name: Some(backend_name),
            backend_key: Some(backend_key),
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
            active_model_target,
            embedding_model_target: None,
            active_runtime: Some(self.runtime_lifecycle_snapshot().await),
            embedding_runtime: None,
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
#[path = "gateway_tests.rs"]
mod tests;
