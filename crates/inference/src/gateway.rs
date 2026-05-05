//! Inference Gateway - Single entry point for all inference operations
//!
//! The gateway abstracts over supported inference backends such as llama.cpp,
//! Candle, PyTorch, and external-compatible runtimes. It manages backend
//! lifecycle, switching, and forwards requests to the active backend.

use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{Stream, StreamExt};
use pantograph_runtime_identity::canonical_runtime_id;
use tokio::sync::RwLock;

use crate::backend::{
    canonical_backend_key, BackendCapabilities, BackendCompatibilityOptions,
    BackendCompatibilityRequest, BackendConfig, BackendDefaultStartMode, BackendError, BackendInfo,
    BackendRegistry, ChatChunk, EmbeddingResult, InferenceBackend,
};
use crate::config::EmbeddingMemoryMode;
use crate::constants::device_types;
use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::model_contracts::{
    resolve_task_registry_entry, GenerationOptions, InferenceLifecyclePhase, ModelArtifactKind,
    OptionCompatibilityDiagnostic, OptionSupportState,
};
use crate::process::ProcessSpawner;
use crate::types::{
    AudioTranscriptionRequest, AudioTranscriptionResult, ChatMessage, ChatRequest, ContentPart,
    ImageGenerationRequest, ImageGenerationResult, InferenceCompatibilityIssueSummary,
    InferenceCompatibilityReportSummary, InferenceEmbeddingResult, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceExecutionRequestValidationError, InferenceExecutionResult,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceUsage, RerankRequest, RerankResponse,
    RuntimeLifecycleSnapshot, ServerModeInfo,
};

const MAX_LIFECYCLE_COMPATIBILITY_ISSUES: usize = 32;

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

    #[error("Invalid typed inference request: {0}")]
    Validation(#[from] InferenceExecutionRequestValidationError),
}

/// Host-supplied inputs for starting the active backend in inference mode.
#[derive(Debug, Clone, Default)]
pub struct InferenceStartRequest {
    pub external_url: Option<String>,
    pub file_model_path: Option<PathBuf>,
    pub mmproj_path: Option<PathBuf>,
    pub device: Option<String>,
    pub gpu_layers: Option<i32>,
}

/// Host-supplied inputs for starting the active backend in embedding mode.
#[derive(Debug, Clone, Default)]
pub struct EmbeddingStartRequest {
    pub gguf_model_path: Option<PathBuf>,
    pub candle_model_path: Option<PathBuf>,
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

fn selected_device_id_from_config(config: Option<&BackendConfig>) -> Option<String> {
    let device = config?.device.as_deref()?.trim();
    if device.is_empty() || device.eq_ignore_ascii_case(device_types::AUTO) {
        None
    } else {
        Some(device.to_string())
    }
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
    /// (e.g., llama.cpp).
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
            "Ollama" => Err(unsupported_ollama_gateway_error()),
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
            "Ollama" => Err(unsupported_ollama_gateway_error()),
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
                "PyTorch does not support embedding mode. Use llama.cpp or Candle for embeddings."
                    .to_string(),
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

    /// Describe the currently active backend instance.
    pub async fn current_backend_info(&self) -> BackendInfo {
        let selected_name = self.current_backend_name().await;
        let selected_backend_key = canonical_backend_key(&selected_name);
        let registry_info = self
            .registry
            .list()
            .into_iter()
            .find(|info| canonical_backend_key(&info.backend_key) == selected_backend_key);
        let backend = self.backend.read().await;

        BackendInfo {
            name: selected_name,
            backend_key: selected_backend_key,
            description: backend.description().to_string(),
            capabilities: backend.capabilities(),
            default_start_mode: registry_info
                .as_ref()
                .map(|info| info.default_start_mode)
                .unwrap_or(BackendDefaultStartMode::Inference),
            active: true,
            available: true,
            unavailable_reason: None,
            can_install: registry_info.as_ref().is_some_and(|info| info.can_install),
            runtime_binary_id: registry_info.and_then(|info| info.runtime_binary_id),
        }
    }

    // ─── LIFECYCLE METHODS ──────────────────────────────────────────

    /// Start the current backend with the given configuration
    pub async fn start(&self, config: &BackendConfig) -> Result<(), GatewayError> {
        // Get the spawner
        let spawner = {
            let guard = self.spawner.read().await;
            guard.clone().ok_or(GatewayError::NoSpawner)?
        };
        let previous_last_inference_config = self.last_inference_config.read().await.clone();

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
                {
                    let mut current_runtime_config = self.current_runtime_config.write().await;
                    *current_runtime_config = None;
                }
                {
                    let mut mode = self.embedding_mode.write().await;
                    *mode = false;
                }
                {
                    let mut mode = self.reranking_mode.write().await;
                    *mode = false;
                }
                {
                    let mut mode = self.external_mode.write().await;
                    *mode = false;
                }
                {
                    let mut last_config = self.last_inference_config.write().await;
                    *last_config = previous_last_inference_config;
                }
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
        if lifecycle.last_error.is_none() {
            lifecycle.lifecycle_decision_reason = Some("runtime_stopped".to_string());
        }
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
        let (active_model_target, active_resolved_device) = {
            let current_runtime_config = self.current_runtime_config.read().await;
            (
                current_runtime_config
                    .as_ref()
                    .and_then(config_model_target),
                selected_device_id_from_config(current_runtime_config.as_ref()),
            )
        };
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
            active_resolved_device,
            embedding_model_target: None,
            embedding_resolved_device: None,
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

    /// Stream chat completion responses and emit request-scoped lifecycle facts.
    pub async fn chat_completion_stream_with_lifecycle(
        &self,
        request_json: String,
        request_id: Option<String>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, GatewayError>
    {
        self.chat_completion_stream_with_lifecycle_for_task(
            request_json,
            request_id,
            None,
            lifecycle_sink,
            None,
            None,
            Vec::new(),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn chat_completion_stream_with_lifecycle_for_task(
        &self,
        request_json: String,
        request_id: Option<String>,
        task_id: Option<String>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
        model_id_override: Option<String>,
        compatibility_report: Option<InferenceCompatibilityReportSummary>,
        compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, GatewayError>
    {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let model_id = model_id_override.or_else(|| chat_request_model_id(&request_json));

        record_inference_lifecycle_event(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        match self.chat_completion_stream(request_json).await {
            Ok(stream) => Ok(Box::pin(LifecycleStream::new(
                stream,
                lifecycle_sink,
                request_id,
                task_id,
                backend_key,
                runtime_id,
                runtime_instance_id,
                selected_device_id,
                model_id,
                compatibility_report,
                compatibility_issues,
            ))),
            Err(error) => {
                record_inference_lifecycle_phase_event_with_diagnostics(
                    lifecycle_sink.as_ref(),
                    InferenceLifecyclePhase::BackendExecution,
                    request_id.clone(),
                    task_id.clone(),
                    backend_key.clone(),
                    runtime_id.clone(),
                    runtime_instance_id.clone(),
                    selected_device_id.clone(),
                    model_id.clone(),
                    InferenceRequestLifecycleEventKind::Failed,
                    Some(error.to_string()),
                    Vec::new(),
                    compatibility_report,
                    compatibility_issues,
                );
                record_inference_lifecycle_event(
                    lifecycle_sink.as_ref(),
                    request_id,
                    task_id,
                    backend_key,
                    runtime_id,
                    runtime_instance_id,
                    selected_device_id,
                    model_id,
                    InferenceRequestLifecycleEventKind::CleanupCompleted,
                    None,
                );
                Err(error)
            }
        }
    }

    /// Stream a typed text/chat generation request.
    ///
    /// This keeps OpenAI-compatible transport JSON inside the gateway adapter
    /// while callers use the canonical typed request contract.
    pub async fn stream_typed_text(
        &self,
        request: InferenceExecutionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, GatewayError>
    {
        request.validate()?;
        let request_json = typed_text_generation_stream_request_json(request)?;
        self.chat_completion_stream(request_json).await
    }

    /// Stream a typed text/chat generation request and emit lifecycle facts.
    pub async fn stream_typed_text_with_lifecycle(
        &self,
        request: InferenceExecutionRequest,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, GatewayError>
    {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let request_id = request.request_id.clone();
        let model_id = typed_request_lifecycle_model_id(&request);
        let task_id = Some(request.task_id.canonical_label().to_string());
        record_model_package_resolution_lifecycle_if_present(
            lifecycle_sink.as_ref(),
            &request,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
        );
        record_inference_lifecycle_phase_event(
            lifecycle_sink.as_ref(),
            InferenceLifecyclePhase::TaskValidation,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        let validation_result = request.validate().map_err(GatewayError::Validation);
        if let Err(error) = validation_result {
            let result = Err(error);
            record_non_streaming_lifecycle_phase_result(
                lifecycle_sink.as_ref(),
                InferenceLifecyclePhase::TaskValidation,
                request_id,
                task_id,
                backend_key,
                runtime_id,
                runtime_instance_id,
                selected_device_id,
                model_id,
                &result,
            );
            return result;
        }

        let compatibility_diagnostics = self
            .compatibility_diagnostics_for_request(&request, backend_key.as_deref())
            .await;
        let mut validation_option_diagnostics =
            compatibility_diagnostics.option_diagnostics.clone();
        validation_option_diagnostics.extend(typed_request_option_diagnostics(
            &request,
            backend_key.as_deref(),
        ));
        dedupe_option_diagnostics(&mut validation_option_diagnostics);
        record_non_streaming_lifecycle_phase_result_with_diagnostics(
            lifecycle_sink.as_ref(),
            InferenceLifecyclePhase::TaskValidation,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            &Ok(()),
            validation_option_diagnostics,
            compatibility_diagnostics.compatibility_report.clone(),
            compatibility_diagnostics.compatibility_issues.clone(),
        );
        let request_json = typed_text_generation_stream_request_json(request)?;
        self.chat_completion_stream_with_lifecycle_for_task(
            request_json,
            request_id,
            task_id,
            lifecycle_sink,
            model_id,
            compatibility_diagnostics.compatibility_report,
            compatibility_diagnostics.compatibility_issues,
        )
        .await
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

    /// Generate embeddings and emit request-scoped lifecycle facts.
    pub async fn embeddings_with_lifecycle(
        &self,
        texts: Vec<String>,
        model: &str,
        request_id: Option<String>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<Vec<EmbeddingResult>, GatewayError> {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let model_id = non_empty_model_id(model);
        record_inference_lifecycle_event(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            Some("embedding".to_string()),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        let result = self.embeddings(texts, model).await;
        record_non_streaming_lifecycle_result(
            lifecycle_sink.as_ref(),
            request_id,
            Some("embedding".to_string()),
            backend_key,
            runtime_id,
            runtime_instance_id,
            selected_device_id,
            model_id,
            &result,
        );
        result
    }

    /// Rank documents through the active backend.
    pub async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, GatewayError> {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard.rerank(request).await.map_err(GatewayError::Backend)
    }

    /// Rank documents and emit request-scoped lifecycle facts.
    pub async fn rerank_with_lifecycle(
        &self,
        request: RerankRequest,
        request_id: Option<String>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<RerankResponse, GatewayError> {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let model_id = non_empty_model_id(&request.model);
        record_inference_lifecycle_event(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            Some("rerank".to_string()),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        let result = self.rerank(request).await;
        record_non_streaming_lifecycle_result(
            lifecycle_sink.as_ref(),
            request_id,
            Some("rerank".to_string()),
            backend_key,
            runtime_id,
            runtime_instance_id,
            selected_device_id,
            model_id,
            &result,
        );
        result
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

    /// Transcribe audio through the active backend.
    pub async fn transcribe_audio(
        &self,
        request: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResult, GatewayError> {
        let guard = self.backend.read().await;
        if !guard.is_ready() {
            return Err(GatewayError::Backend(BackendError::NotReady));
        }
        guard
            .transcribe_audio(request)
            .await
            .map_err(GatewayError::Backend)
    }

    /// Generate one or more images and emit request-scoped lifecycle facts.
    pub async fn generate_image_with_lifecycle(
        &self,
        request: ImageGenerationRequest,
        request_id: Option<String>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<ImageGenerationResult, GatewayError> {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let model_id = non_empty_model_id(&request.model);
        record_inference_lifecycle_event(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            Some("image_generation".to_string()),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        let result = self.generate_image(request).await;
        record_non_streaming_lifecycle_result(
            lifecycle_sink.as_ref(),
            request_id,
            Some("image_generation".to_string()),
            backend_key,
            runtime_id,
            runtime_instance_id,
            selected_device_id,
            model_id,
            &result,
        );
        result
    }

    /// Execute a typed inference request through the current backend facade.
    ///
    /// This adapter validates canonical typed request semantics and then
    /// bridges into the existing backend methods. Backend-specific transports
    /// such as OpenAI-compatible JSON remain at this gateway/backend edge.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::Validation`] when the typed request is malformed
    /// and [`GatewayError::Backend`] when the active backend rejects or fails
    /// execution.
    pub async fn execute_typed(
        &self,
        request: InferenceExecutionRequest,
    ) -> Result<InferenceExecutionResult, GatewayError> {
        request.validate()?;
        self.execute_typed_validated(request).await
    }

    /// Execute a typed request and emit request-scoped lifecycle facts.
    ///
    /// Task validation and backend execution are recorded as separate lifecycle
    /// phases so ledger consumers can distinguish malformed typed requests from
    /// backend/runtime failures without importing diagnostics-ledger here.
    pub async fn execute_typed_with_lifecycle(
        &self,
        request: InferenceExecutionRequest,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    ) -> Result<InferenceExecutionResult, GatewayError> {
        let (backend_key, runtime_id, runtime_instance_id, selected_device_id) =
            self.lifecycle_event_context().await;
        let request_id = request.request_id.clone();
        let model_id = typed_request_lifecycle_model_id(&request);
        let task_id = Some(request.task_id.canonical_label().to_string());
        let emit_typed_boundary_lifecycle = typed_request_has_boundary_lifecycle(&request);
        record_model_package_resolution_lifecycle_if_present(
            lifecycle_sink.as_ref(),
            &request,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
        );
        record_inference_lifecycle_phase_event(
            lifecycle_sink.as_ref(),
            InferenceLifecyclePhase::TaskValidation,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );

        let validation_result = request.validate().map_err(GatewayError::Validation);
        if let Err(error) = validation_result {
            let result = Err(error);
            record_non_streaming_lifecycle_phase_result(
                lifecycle_sink.as_ref(),
                InferenceLifecyclePhase::TaskValidation,
                request_id,
                task_id,
                backend_key,
                runtime_id,
                runtime_instance_id,
                selected_device_id,
                model_id,
                &result,
            );
            return result;
        }

        let compatibility_diagnostics = self
            .compatibility_diagnostics_for_request(&request, backend_key.as_deref())
            .await;
        let request_option_diagnostics =
            typed_request_option_diagnostics(&request, backend_key.as_deref());
        let mut validation_option_diagnostics =
            compatibility_diagnostics.option_diagnostics.clone();
        validation_option_diagnostics.extend(request_option_diagnostics.clone());
        dedupe_option_diagnostics(&mut validation_option_diagnostics);
        record_non_streaming_lifecycle_phase_result_with_diagnostics(
            lifecycle_sink.as_ref(),
            InferenceLifecyclePhase::TaskValidation,
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            &Ok(()),
            validation_option_diagnostics,
            compatibility_diagnostics.compatibility_report.clone(),
            compatibility_diagnostics.compatibility_issues.clone(),
        );
        if emit_typed_boundary_lifecycle {
            record_successful_non_streaming_lifecycle_phase(
                lifecycle_sink.as_ref(),
                InferenceLifecyclePhase::Preprocessing,
                request_id.clone(),
                task_id.clone(),
                backend_key.clone(),
                runtime_id.clone(),
                runtime_instance_id.clone(),
                selected_device_id.clone(),
                model_id.clone(),
            );
        }
        record_inference_lifecycle_event(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Started,
            None,
        );
        let result = self.execute_typed_validated(request).await;
        let mut option_diagnostics = option_diagnostics_from_execution_result(&result);
        option_diagnostics.extend(request_option_diagnostics);
        dedupe_option_diagnostics(&mut option_diagnostics);
        record_typed_lifecycle_result_with_option_diagnostics(
            lifecycle_sink.as_ref(),
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            &result,
            option_diagnostics,
            compatibility_diagnostics.compatibility_report,
            compatibility_diagnostics.compatibility_issues,
        );
        if emit_typed_boundary_lifecycle && result.is_ok() {
            record_successful_non_streaming_lifecycle_phase(
                lifecycle_sink.as_ref(),
                InferenceLifecyclePhase::Postprocessing,
                request_id.clone(),
                task_id.clone(),
                backend_key.clone(),
                runtime_id.clone(),
                runtime_instance_id.clone(),
                selected_device_id.clone(),
                model_id.clone(),
            );
            record_successful_non_streaming_lifecycle_phase(
                lifecycle_sink.as_ref(),
                InferenceLifecyclePhase::ResultProjection,
                request_id,
                task_id,
                backend_key,
                runtime_id,
                runtime_instance_id,
                selected_device_id,
                model_id,
            );
        }
        result
    }

    async fn execute_typed_validated(
        &self,
        request: InferenceExecutionRequest,
    ) -> Result<InferenceExecutionResult, GatewayError> {
        let model = typed_request_model_name(&request);
        let backend_key = canonical_backend_key(&self.current_backend_name().await);
        let request_option_diagnostics =
            typed_request_option_diagnostics(&request, Some(&backend_key));
        let task_id = request.task_id.clone();

        match request.input {
            InferenceExecutionInput::TextGeneration {
                prompt,
                system_prompt,
                messages,
                stream,
            } => {
                let chat_request = typed_text_generation_to_chat_request(
                    model,
                    prompt,
                    system_prompt,
                    messages,
                    stream,
                    request.generation_options.as_ref(),
                );
                let request_json = serde_json::to_string(&chat_request).map_err(|error| {
                    GatewayError::Backend(BackendError::Inference(format!(
                        "Failed to encode typed chat request: {error}"
                    )))
                })?;
                let mut stream = self.chat_completion_stream(request_json).await?;
                let mut text = String::new();
                let mut usage = None;
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(GatewayError::Backend)?;
                    if let Some(chunk_usage) = chunk.usage.clone() {
                        usage = Some(chunk_usage);
                    }
                    if let Some(content) = chunk.content {
                        text.push_str(&content);
                    }
                    if chunk.done {
                        break;
                    }
                }
                Ok(InferenceExecutionResult::TextGeneration {
                    text,
                    usage,
                    cache_handle_id: None,
                    option_diagnostics: request_option_diagnostics,
                })
            }
            InferenceExecutionInput::Embedding { texts } => {
                let embeddings: Vec<InferenceEmbeddingResult> = self
                    .embeddings(texts, &model)
                    .await?
                    .into_iter()
                    .enumerate()
                    .map(|(index, embedding)| InferenceEmbeddingResult {
                        vector: embedding.vector,
                        token_count: Some(embedding.token_count),
                        index: Some(index),
                    })
                    .collect();
                let usage = embedding_usage_from_results(&embeddings);
                Ok(InferenceExecutionResult::Embedding {
                    embeddings,
                    usage,
                    option_diagnostics: request_option_diagnostics,
                })
            }
            InferenceExecutionInput::Rerank {
                query,
                documents,
                top_n,
                return_documents,
            } => {
                let response = self
                    .rerank(RerankRequest {
                        model,
                        query,
                        documents,
                        top_n,
                        return_documents,
                        extra_options: request.extra_options,
                    })
                    .await?;
                Ok(InferenceExecutionResult::Rerank {
                    response,
                    option_diagnostics: request_option_diagnostics,
                })
            }
            InferenceExecutionInput::ImageGeneration { request } => {
                let mut option_diagnostics =
                    typed_image_generation_option_diagnostics(&request, Some(&backend_key));
                option_diagnostics.extend(extra_option_diagnostics(
                    &request.extra_options,
                    Some(&backend_key),
                    "image.extra_options",
                ));
                dedupe_option_diagnostics(&mut option_diagnostics);
                let result = self.generate_image(request).await?;
                Ok(InferenceExecutionResult::ImageGeneration {
                    result,
                    option_diagnostics,
                })
            }
            InferenceExecutionInput::AudioTranscription { request } => {
                let mut option_diagnostics =
                    typed_audio_transcription_option_diagnostics(&request, Some(&backend_key));
                option_diagnostics.extend(extra_option_diagnostics(
                    &request.extra_options,
                    Some(&backend_key),
                    "audio_transcription.extra_options",
                ));
                dedupe_option_diagnostics(&mut option_diagnostics);
                let result = self.transcribe_audio(request).await?;
                Ok(InferenceExecutionResult::AudioTranscription {
                    result,
                    option_diagnostics,
                })
            }
            InferenceExecutionInput::ImageUnderstanding { .. }
            | InferenceExecutionInput::VideoUnderstanding { .. }
            | InferenceExecutionInput::MultimodalGeneration { .. } => {
                Err(GatewayError::Validation(
                    InferenceExecutionRequestValidationError::UnsupportedTask { task_id },
                ))
            }
        }
    }

    // ─── LEGACY COMPATIBILITY ───────────────────────────────────────

    /// Get a reference to the underlying backend for legacy code
    ///
    /// This is a temporary method for gradual migration. New code should
    /// use the gateway methods directly.
    pub fn backend(&self) -> Arc<RwLock<Box<dyn InferenceBackend>>> {
        self.backend.clone()
    }

    async fn lifecycle_event_context(
        &self,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) {
        let runtime_snapshot = self.runtime_lifecycle_snapshot().await;
        let runtime_id = runtime_snapshot.runtime_id.clone();
        let runtime_instance_id = runtime_snapshot.runtime_instance_id.clone();
        let backend_key = Some(canonical_backend_key(&self.current_backend_name().await));
        let current_runtime_config = self.current_runtime_config.read().await;
        let selected_device_id = selected_device_id_from_config(current_runtime_config.as_ref());
        (
            backend_key,
            runtime_id,
            runtime_instance_id,
            selected_device_id,
        )
    }

    async fn compatibility_diagnostics_for_request(
        &self,
        request: &InferenceExecutionRequest,
        backend_key: Option<&str>,
    ) -> InferenceLifecycleCompatibilityDiagnostics {
        let Some(package_facts) = request.resolved_model_package_facts.as_ref() else {
            return InferenceLifecycleCompatibilityDiagnostics::default();
        };
        let Some(task) = resolve_task_registry_entry(request.task_id.canonical_label()) else {
            return InferenceLifecycleCompatibilityDiagnostics::default();
        };

        let capabilities = self.backend.read().await.capabilities();
        let compatibility_options = BackendCompatibilityOptions {
            streaming: typed_request_is_streaming(request),
            cache: request
                .generation_options
                .as_ref()
                .map(|options| options.cache.clone())
                .unwrap_or_default(),
            ..BackendCompatibilityOptions::default()
        };
        let report = capabilities.check_model_compatibility(
            backend_key,
            BackendCompatibilityRequest::new(&task, package_facts)
                .with_options(compatibility_options),
        );

        InferenceLifecycleCompatibilityDiagnostics {
            option_diagnostics: report.option_diagnostics.clone(),
            compatibility_report: Some(report.to_inference_compatibility_report_summary()),
            compatibility_issues: report
                .to_inference_compatibility_issue_summaries(MAX_LIFECYCLE_COMPATIBILITY_ISSUES),
        }
    }
}

#[derive(Debug, Default)]
struct InferenceLifecycleCompatibilityDiagnostics {
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
}

fn typed_request_is_streaming(request: &InferenceExecutionRequest) -> bool {
    matches!(
        &request.input,
        InferenceExecutionInput::TextGeneration { stream: true, .. }
    )
}

fn typed_request_has_boundary_lifecycle(request: &InferenceExecutionRequest) -> bool {
    matches!(
        &request.input,
        InferenceExecutionInput::TextGeneration { .. }
            | InferenceExecutionInput::Embedding { .. }
            | InferenceExecutionInput::Rerank { .. }
            | InferenceExecutionInput::ImageGeneration { .. }
            | InferenceExecutionInput::AudioTranscription { .. }
    )
}

struct LifecycleStream {
    inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
    lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    usage: Option<InferenceUsage>,
    finished: bool,
}

impl LifecycleStream {
    #[allow(clippy::too_many_arguments)]
    fn new(
        inner: Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
        lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink>,
        request_id: Option<String>,
        task_id: Option<String>,
        backend_key: Option<String>,
        runtime_id: Option<String>,
        runtime_instance_id: Option<String>,
        selected_device_id: Option<String>,
        model_id: Option<String>,
        compatibility_report: Option<InferenceCompatibilityReportSummary>,
        compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    ) -> Self {
        Self {
            inner,
            lifecycle_sink,
            request_id,
            task_id,
            backend_key,
            runtime_id,
            runtime_instance_id,
            selected_device_id,
            model_id,
            compatibility_report,
            compatibility_issues,
            usage: None,
            finished: false,
        }
    }

    fn record(&self, kind: InferenceRequestLifecycleEventKind, detail: Option<String>) {
        record_inference_lifecycle_event(
            self.lifecycle_sink.as_ref(),
            self.request_id.clone(),
            self.task_id.clone(),
            self.backend_key.clone(),
            self.runtime_id.clone(),
            self.runtime_instance_id.clone(),
            self.selected_device_id.clone(),
            self.model_id.clone(),
            kind,
            detail,
        );
    }

    fn record_terminal(&self, kind: InferenceRequestLifecycleEventKind, detail: Option<String>) {
        record_inference_lifecycle_phase_event_with_references(
            self.lifecycle_sink.as_ref(),
            InferenceLifecyclePhase::BackendExecution,
            self.request_id.clone(),
            self.task_id.clone(),
            self.backend_key.clone(),
            self.runtime_id.clone(),
            self.runtime_instance_id.clone(),
            self.selected_device_id.clone(),
            self.model_id.clone(),
            kind,
            detail,
            Vec::new(),
            self.compatibility_report.clone(),
            self.compatibility_issues.clone(),
            self.usage.clone(),
            None,
            None,
        );
    }

    fn finish(&mut self, kind: InferenceRequestLifecycleEventKind, detail: Option<String>) {
        if self.finished {
            return;
        }

        self.record_terminal(kind, detail);
        self.record(InferenceRequestLifecycleEventKind::CleanupCompleted, None);
        self.finished = true;
    }
}

impl Stream for LifecycleStream {
    type Item = Result<ChatChunk, BackendError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                if let Some(usage) = chunk.usage.clone() {
                    self.usage = Some(usage);
                }
                if chunk.done {
                    self.finish(InferenceRequestLifecycleEventKind::Completed, None);
                }
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(error))) => {
                let detail = error.to_string();
                self.finish(InferenceRequestLifecycleEventKind::Failed, Some(detail));
                Poll::Ready(Some(Err(error)))
            }
            Poll::Ready(None) => {
                self.finish(InferenceRequestLifecycleEventKind::Completed, None);
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for LifecycleStream {
    fn drop(&mut self) {
        if !self.finished {
            self.finish(
                InferenceRequestLifecycleEventKind::Cancelled,
                Some("stream dropped before completion".to_string()),
            );
        }
    }
}

fn typed_request_model_name(request: &InferenceExecutionRequest) -> String {
    request
        .model_name
        .clone()
        .or_else(|| {
            request
                .model_ref
                .as_ref()
                .map(|model_ref| model_ref.model_id.clone())
        })
        .unwrap_or_default()
}

fn typed_request_lifecycle_model_id(request: &InferenceExecutionRequest) -> Option<String> {
    request
        .resolved_model_package_facts
        .as_ref()
        .map(|facts| facts.model_ref.model_id.clone())
        .or_else(|| {
            request
                .model_ref
                .as_ref()
                .map(|model_ref| model_ref.model_id.clone())
        })
        .or_else(|| request.model_name.clone())
        .and_then(|value| non_empty_model_id(&value))
}

fn chat_request_model_id(request_json: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(request_json)
        .ok()
        .and_then(|value| {
            value
                .get("model")
                .and_then(serde_json::Value::as_str)
                .and_then(non_empty_model_id)
        })
}

fn non_empty_model_id(model: &str) -> Option<String> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn typed_text_generation_to_chat_request(
    model: String,
    prompt: Option<String>,
    system_prompt: Option<String>,
    messages: Vec<ChatMessage>,
    stream: bool,
    generation_options: Option<&GenerationOptions>,
) -> ChatRequest {
    let messages = if messages.is_empty() {
        let mut converted = Vec::new();
        if let Some(system_prompt) = system_prompt {
            converted.push(ChatMessage {
                role: "system".to_string(),
                content: vec![ContentPart::Text {
                    text: system_prompt,
                }],
            });
        }
        if let Some(prompt) = prompt {
            converted.push(ChatMessage {
                role: "user".to_string(),
                content: vec![ContentPart::Text { text: prompt }],
            });
        }
        converted
    } else {
        messages
    };

    ChatRequest {
        model,
        messages,
        stream,
        max_tokens: generation_options.and_then(|options| options.length.max_new_tokens),
        temperature: generation_options.and_then(|options| options.sampling.temperature),
        top_p: generation_options.and_then(|options| options.sampling.top_p),
        top_k: generation_options.and_then(|options| options.sampling.top_k),
    }
}

fn typed_text_generation_option_diagnostics(
    generation_options: Option<&GenerationOptions>,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let Some(options) = generation_options else {
        return Vec::new();
    };

    let mut diagnostics = Vec::new();
    let mut mapped_paths = Vec::new();
    push_chat_option_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        "length.max_new_tokens",
        options.length.max_new_tokens.is_some(),
        "mapped to chat max_tokens",
    );
    push_chat_option_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        "sampling.temperature",
        options.sampling.temperature.is_some(),
        "mapped to chat temperature",
    );
    push_chat_option_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        "sampling.top_p",
        options.sampling.top_p.is_some(),
        "mapped to chat top_p",
    );
    push_chat_option_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        "sampling.top_k",
        options.sampling.top_k.is_some(),
        "mapped to chat top_k",
    );
    push_chat_cache_use_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        options.cache.use_cache,
    );
    push_chat_cache_checkpoint_diagnostic(
        &mut diagnostics,
        &mut mapped_paths,
        backend_key,
        options.cache.kv_cache_checkpoint_requested,
    );

    for path in options.requested_option_paths() {
        if mapped_paths.iter().any(|mapped| mapped == &path) {
            continue;
        }
        diagnostics.push(OptionCompatibilityDiagnostic {
            option_path: path,
            state: OptionSupportState::Unsupported,
            backend_key: backend_key.map(ToOwned::to_owned),
            message: Some(
                "typed text gateway does not map this option through the chat boundary".to_string(),
            ),
        });
    }

    diagnostics
}

fn typed_request_option_diagnostics(
    request: &InferenceExecutionRequest,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    match &request.input {
        InferenceExecutionInput::TextGeneration { .. } => typed_text_generation_option_diagnostics(
            request.generation_options.as_ref(),
            backend_key,
        ),
        _ => typed_non_generation_option_diagnostics(request, backend_key),
    }
}

fn push_chat_cache_use_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    mapped_paths: &mut Vec<&'static str>,
    backend_key: Option<&str>,
    requested: Option<bool>,
) {
    if requested.is_none() {
        return;
    }

    mapped_paths.push("cache.use_cache");
    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: "cache.use_cache".to_string(),
        state: OptionSupportState::RequiresBackendSupport,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(
            "cache reuse is resolved by Pantograph runtime/KV policy outside the chat request"
                .to_string(),
        ),
    });
}

fn push_chat_cache_checkpoint_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    mapped_paths: &mut Vec<&'static str>,
    backend_key: Option<&str>,
    requested: Option<bool>,
) {
    let Some(requested) = requested else {
        return;
    };

    mapped_paths.push("cache.kv_cache_checkpoint_requested");
    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: "cache.kv_cache_checkpoint_requested".to_string(),
        state: if requested {
            OptionSupportState::Mapped
        } else {
            OptionSupportState::Honored
        },
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(if requested {
            "handled by Pantograph KV-cache publication outside the chat request".to_string()
        } else {
            "no KV-cache checkpoint requested".to_string()
        }),
    });
}

fn typed_non_generation_option_diagnostics(
    request: &InferenceExecutionRequest,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let mut diagnostics = match &request.input {
        InferenceExecutionInput::Embedding { .. } => Vec::new(),
        InferenceExecutionInput::Rerank {
            top_n,
            return_documents,
            ..
        } => {
            let mut diagnostics = Vec::new();
            if top_n.is_some() {
                diagnostics.push(OptionCompatibilityDiagnostic {
                    option_path: "rerank.top_n".to_string(),
                    state: OptionSupportState::Honored,
                    backend_key: backend_key.map(ToOwned::to_owned),
                    message: Some("typed rerank gateway forwards top_n".to_string()),
                });
            }
            if !return_documents {
                diagnostics.push(OptionCompatibilityDiagnostic {
                    option_path: "rerank.return_documents".to_string(),
                    state: OptionSupportState::Honored,
                    backend_key: backend_key.map(ToOwned::to_owned),
                    message: Some("typed rerank gateway forwards return_documents".to_string()),
                });
            }
            diagnostics
        }
        InferenceExecutionInput::ImageGeneration { request } => {
            let mut diagnostics = typed_image_generation_option_diagnostics(request, backend_key);
            diagnostics.extend(extra_option_diagnostics(
                &request.extra_options,
                backend_key,
                "image.extra_options",
            ));
            diagnostics
        }
        InferenceExecutionInput::AudioTranscription { request } => {
            let mut diagnostics =
                typed_audio_transcription_option_diagnostics(request, backend_key);
            diagnostics.extend(extra_option_diagnostics(
                &request.extra_options,
                backend_key,
                "audio_transcription.extra_options",
            ));
            diagnostics
        }
        InferenceExecutionInput::ImageUnderstanding { request } => extra_option_diagnostics(
            &request.extra_options,
            backend_key,
            "image_understanding.extra_options",
        ),
        InferenceExecutionInput::VideoUnderstanding { request } => extra_option_diagnostics(
            &request.extra_options,
            backend_key,
            "video_understanding.extra_options",
        ),
        InferenceExecutionInput::MultimodalGeneration { request } => extra_option_diagnostics(
            &request.extra_options,
            backend_key,
            "multimodal_generation.extra_options",
        ),
        InferenceExecutionInput::TextGeneration { .. } => Vec::new(),
    };

    diagnostics.extend(extra_option_diagnostics(
        &request.extra_options,
        backend_key,
        "extra_options",
    ));
    diagnostics
}

fn typed_audio_transcription_option_diagnostics(
    request: &AudioTranscriptionRequest,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let mut diagnostics = Vec::new();
    push_audio_transcription_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "audio_transcription.language",
        request
            .language
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        "typed audio gateway forwards language hints",
    );
    push_audio_transcription_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "audio_transcription.prompt",
        request
            .prompt
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        "typed audio gateway forwards prompt hints without diagnostics payload values",
    );
    push_audio_transcription_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "audio_transcription.task",
        request
            .task
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty()),
        "typed audio gateway forwards task hints",
    );
    push_audio_transcription_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "audio_transcription.chunk_length_s",
        request.chunk_length_s.is_some(),
        "typed audio gateway forwards chunk length hints",
    );
    diagnostics
}

fn push_audio_transcription_option_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    backend_key: Option<&str>,
    option_path: &str,
    requested: bool,
    message: &str,
) {
    if !requested {
        return;
    }

    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: option_path.to_string(),
        state: OptionSupportState::Honored,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(message.to_string()),
    });
}

fn typed_image_generation_option_diagnostics(
    request: &ImageGenerationRequest,
    backend_key: Option<&str>,
) -> Vec<OptionCompatibilityDiagnostic> {
    let mut diagnostics = Vec::new();
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.negative_prompt",
        request.negative_prompt.is_some(),
        "typed image gateway forwards negative_prompt",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.width",
        request.width.is_some(),
        "typed image gateway forwards width",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.height",
        request.height.is_some(),
        "typed image gateway forwards height",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.num_inference_steps",
        request.num_inference_steps.is_some(),
        "typed image gateway forwards num_inference_steps",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.guidance_scale",
        request.guidance_scale.is_some(),
        "typed image gateway forwards guidance_scale",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.seed",
        request.seed.is_some(),
        "typed image gateway forwards seed",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.scheduler",
        request.scheduler.is_some(),
        "typed image gateway forwards scheduler",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.num_images_per_prompt",
        request.num_images_per_prompt.is_some(),
        "typed image gateway forwards num_images_per_prompt",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.init_image",
        request.init_image.is_some(),
        "typed image gateway forwards init_image presence",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.mask_image",
        request.mask_image.is_some(),
        "typed image gateway forwards mask_image presence",
    );
    push_image_option_diagnostic(
        &mut diagnostics,
        backend_key,
        "image.strength",
        request.strength.is_some(),
        "typed image gateway forwards strength",
    );
    diagnostics
}

fn push_image_option_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    backend_key: Option<&str>,
    option_path: &'static str,
    requested: bool,
    message: &'static str,
) {
    if !requested {
        return;
    }
    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: option_path.to_string(),
        state: OptionSupportState::Honored,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(message.to_string()),
    });
}

fn extra_option_diagnostics(
    value: &serde_json::Value,
    backend_key: Option<&str>,
    root_path: &str,
) -> Vec<OptionCompatibilityDiagnostic> {
    match value {
        serde_json::Value::Object(options) => {
            let mut diagnostics = options
                .keys()
                .filter(|key| !key.trim().is_empty())
                .map(|key| OptionCompatibilityDiagnostic {
                    option_path: format!("{root_path}.{key}"),
                    state: OptionSupportState::Mapped,
                    backend_key: backend_key.map(ToOwned::to_owned),
                    message: Some(
                        "typed gateway forwards backend-specific extra option by key".to_string(),
                    ),
                })
                .collect::<Vec<_>>();
            diagnostics.sort_by(|left, right| left.option_path.cmp(&right.option_path));
            diagnostics
        }
        serde_json::Value::Null => Vec::new(),
        _ => vec![OptionCompatibilityDiagnostic {
            option_path: root_path.to_string(),
            state: OptionSupportState::Unsupported,
            backend_key: backend_key.map(ToOwned::to_owned),
            message: Some("typed gateway extra_options must be an object".to_string()),
        }],
    }
}

fn push_chat_option_diagnostic(
    diagnostics: &mut Vec<OptionCompatibilityDiagnostic>,
    mapped_paths: &mut Vec<&'static str>,
    backend_key: Option<&str>,
    option_path: &'static str,
    requested: bool,
    message: &'static str,
) {
    if !requested {
        return;
    }
    mapped_paths.push(option_path);
    diagnostics.push(OptionCompatibilityDiagnostic {
        option_path: option_path.to_string(),
        state: OptionSupportState::Mapped,
        backend_key: backend_key.map(ToOwned::to_owned),
        message: Some(message.to_string()),
    });
}

fn typed_text_generation_stream_request_json(
    request: InferenceExecutionRequest,
) -> Result<String, GatewayError> {
    let model = typed_request_model_name(&request);
    match request.input {
        InferenceExecutionInput::TextGeneration {
            prompt,
            system_prompt,
            messages,
            ..
        } => {
            let chat_request = typed_text_generation_to_chat_request(
                model,
                prompt,
                system_prompt,
                messages,
                true,
                request.generation_options.as_ref(),
            );
            serde_json::to_string(&chat_request).map_err(|error| {
                GatewayError::Backend(BackendError::Inference(format!(
                    "Failed to encode typed streaming chat request: {error}"
                )))
            })
        }
        other => Err(GatewayError::Validation(
            InferenceExecutionRequestValidationError::TaskInputMismatch {
                task_id: request.task_id,
                input_type: other.input_type_label(),
            },
        )),
    }
}

fn record_inference_lifecycle_event(
    sink: &dyn InferenceRequestLifecycleEventSink,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    kind: InferenceRequestLifecycleEventKind,
    detail: Option<String>,
) {
    record_inference_lifecycle_phase_event(
        sink,
        InferenceLifecyclePhase::BackendExecution,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        kind,
        detail,
    );
}

fn record_inference_lifecycle_phase_event(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    kind: InferenceRequestLifecycleEventKind,
    detail: Option<String>,
) {
    record_inference_lifecycle_phase_event_with_option_diagnostics(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        kind,
        detail,
        Vec::new(),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_model_package_resolution_lifecycle_if_present(
    sink: &dyn InferenceRequestLifecycleEventSink,
    request: &InferenceExecutionRequest,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
) {
    let Some(package_facts) = request.resolved_model_package_facts.as_ref() else {
        return;
    };
    let resolved_artifact_kind =
        Some(model_artifact_kind_label(&package_facts.artifact.artifact_kind).to_string());

    record_inference_lifecycle_phase_event_with_references(
        sink,
        InferenceLifecyclePhase::ModelPackageResolution,
        request_id.clone(),
        task_id.clone(),
        backend_key.clone(),
        runtime_id.clone(),
        runtime_instance_id.clone(),
        selected_device_id.clone(),
        model_id.clone(),
        InferenceRequestLifecycleEventKind::Started,
        None,
        Vec::new(),
        None,
        Vec::new(),
        None,
        None,
        resolved_artifact_kind.clone(),
    );
    record_non_streaming_lifecycle_phase_result_with_references(
        sink,
        InferenceLifecyclePhase::ModelPackageResolution,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        &Ok::<(), GatewayError>(()),
        Vec::new(),
        None,
        Vec::new(),
        None,
        None,
        resolved_artifact_kind,
    );
}

fn model_artifact_kind_label(kind: &ModelArtifactKind) -> &'static str {
    match kind {
        ModelArtifactKind::Gguf => "gguf",
        ModelArtifactKind::HfCompatibleDirectory => "hf_compatible_directory",
        ModelArtifactKind::Safetensors => "safetensors",
        ModelArtifactKind::DiffusersBundle => "diffusers_bundle",
        ModelArtifactKind::Onnx => "onnx",
        ModelArtifactKind::Adapter => "adapter",
        ModelArtifactKind::Shard => "shard",
        ModelArtifactKind::Unknown => "unknown",
    }
}

#[allow(clippy::too_many_arguments)]
fn record_inference_lifecycle_phase_event_with_option_diagnostics(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    kind: InferenceRequestLifecycleEventKind,
    detail: Option<String>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
) {
    record_inference_lifecycle_phase_event_with_diagnostics(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        kind,
        detail,
        option_diagnostics,
        None,
        Vec::new(),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_inference_lifecycle_phase_event_with_diagnostics(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    kind: InferenceRequestLifecycleEventKind,
    detail: Option<String>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
) {
    record_inference_lifecycle_phase_event_with_references(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        kind,
        detail,
        option_diagnostics,
        compatibility_report,
        compatibility_issues,
        None,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn record_inference_lifecycle_phase_event_with_references(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    kind: InferenceRequestLifecycleEventKind,
    detail: Option<String>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    usage: Option<InferenceUsage>,
    cache_handle_id: Option<String>,
    resolved_artifact_kind: Option<String>,
) {
    sink.record(InferenceRequestLifecycleEvent {
        request_id,
        phase,
        kind,
        occurred_at_ms: unix_timestamp_ms(),
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        selected_network_node_id: None,
        model_id,
        resolved_artifact_kind,
        usage,
        cache_handle_id,
        detail,
        canonical_error_event_id: None,
        compatibility_report,
        compatibility_issues,
        option_diagnostics,
    });
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_result<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
) {
    record_non_streaming_lifecycle_result_with_option_diagnostics(
        sink,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        Vec::new(),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_typed_lifecycle_result_with_option_diagnostics(
    sink: &dyn InferenceRequestLifecycleEventSink,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<InferenceExecutionResult, GatewayError>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
) {
    record_non_streaming_lifecycle_phase_result_with_references(
        sink,
        InferenceLifecyclePhase::BackendExecution,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        option_diagnostics,
        compatibility_report,
        compatibility_issues,
        usage_from_execution_result(result),
        cache_handle_from_execution_result(result),
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_result_with_option_diagnostics<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
) {
    record_non_streaming_lifecycle_phase_result_with_option_diagnostics(
        sink,
        InferenceLifecyclePhase::BackendExecution,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        option_diagnostics,
    );
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_phase_result<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
) {
    record_non_streaming_lifecycle_phase_result_with_option_diagnostics(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        Vec::new(),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_successful_non_streaming_lifecycle_phase(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
) {
    record_inference_lifecycle_phase_event(
        sink,
        phase.clone(),
        request_id.clone(),
        task_id.clone(),
        backend_key.clone(),
        runtime_id.clone(),
        runtime_instance_id.clone(),
        selected_device_id.clone(),
        model_id.clone(),
        InferenceRequestLifecycleEventKind::Started,
        None,
    );
    let result: Result<(), GatewayError> = Ok(());
    record_non_streaming_lifecycle_phase_result(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        &result,
    );
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_phase_result_with_option_diagnostics<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
) {
    record_non_streaming_lifecycle_phase_result_with_diagnostics(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        option_diagnostics,
        None,
        Vec::new(),
    );
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_phase_result_with_diagnostics<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
) {
    record_non_streaming_lifecycle_phase_result_with_references(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        result,
        option_diagnostics,
        compatibility_report,
        compatibility_issues,
        None,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn record_non_streaming_lifecycle_phase_result_with_references<T>(
    sink: &dyn InferenceRequestLifecycleEventSink,
    phase: InferenceLifecyclePhase,
    request_id: Option<String>,
    task_id: Option<String>,
    backend_key: Option<String>,
    runtime_id: Option<String>,
    runtime_instance_id: Option<String>,
    selected_device_id: Option<String>,
    model_id: Option<String>,
    result: &Result<T, GatewayError>,
    option_diagnostics: Vec<OptionCompatibilityDiagnostic>,
    compatibility_report: Option<InferenceCompatibilityReportSummary>,
    compatibility_issues: Vec<InferenceCompatibilityIssueSummary>,
    usage: Option<InferenceUsage>,
    cache_handle_id: Option<String>,
    resolved_artifact_kind: Option<String>,
) {
    match result {
        Ok(_) => record_inference_lifecycle_phase_event_with_references(
            sink,
            phase.clone(),
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Completed,
            None,
            option_diagnostics,
            compatibility_report,
            compatibility_issues,
            usage,
            cache_handle_id,
            resolved_artifact_kind.clone(),
        ),
        Err(error) => record_inference_lifecycle_phase_event(
            sink,
            phase.clone(),
            request_id.clone(),
            task_id.clone(),
            backend_key.clone(),
            runtime_id.clone(),
            runtime_instance_id.clone(),
            selected_device_id.clone(),
            model_id.clone(),
            InferenceRequestLifecycleEventKind::Failed,
            Some(error.to_string()),
        ),
    }

    record_inference_lifecycle_phase_event(
        sink,
        phase,
        request_id,
        task_id,
        backend_key,
        runtime_id,
        runtime_instance_id,
        selected_device_id,
        model_id,
        InferenceRequestLifecycleEventKind::CleanupCompleted,
        None,
    );
}

fn option_diagnostics_from_execution_result(
    result: &Result<InferenceExecutionResult, GatewayError>,
) -> Vec<OptionCompatibilityDiagnostic> {
    match result {
        Ok(result) => result.option_diagnostics().to_vec(),
        _ => Vec::new(),
    }
}

fn usage_from_execution_result(
    result: &Result<InferenceExecutionResult, GatewayError>,
) -> Option<InferenceUsage> {
    match result {
        Ok(result) => result.usage().cloned(),
        _ => None,
    }
}

fn embedding_usage_from_results(results: &[InferenceEmbeddingResult]) -> Option<InferenceUsage> {
    let mut saw_count = false;
    let mut total = 0u64;

    for result in results {
        if let Some(token_count) = result.token_count {
            saw_count = true;
            total = total.saturating_add(token_count as u64);
        }
    }

    if !saw_count {
        return None;
    }

    let total = total.min(u64::from(u32::MAX)) as u32;
    Some(InferenceUsage {
        prompt_tokens: Some(total),
        completion_tokens: None,
        total_tokens: Some(total),
    })
}

fn cache_handle_from_execution_result(
    result: &Result<InferenceExecutionResult, GatewayError>,
) -> Option<String> {
    match result {
        Ok(result) => result.cache_handle_id().map(ToOwned::to_owned),
        _ => None,
    }
}

fn dedupe_option_diagnostics(diagnostics: &mut Vec<OptionCompatibilityDiagnostic>) {
    let mut seen = Vec::new();
    diagnostics.retain(|diagnostic| {
        let key = (
            diagnostic.option_path.clone(),
            diagnostic.state,
            diagnostic.backend_key.clone(),
        );
        if seen.iter().any(|existing| existing == &key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

fn unix_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn unsupported_ollama_gateway_error() -> GatewayError {
    GatewayError::Backend(BackendError::Config(
        "Ollama is no longer supported as a first-party Pantograph inference backend. Use a Pumas model reference with a supported runtime such as llama.cpp, PyTorch/Transformers, or Candle.".to_string(),
    ))
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
