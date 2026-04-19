//! Pluggable inference backend abstraction
//!
//! This module provides a trait-based abstraction for different inference engines
//! (llama.cpp, Ollama, Candle, external APIs). All backends implement the same
//! interface, allowing runtime switching between engines.

pub mod registry;

#[cfg(feature = "backend-llamacpp")]
pub mod llamacpp;

#[cfg(feature = "backend-ollama")]
pub mod ollama;

#[cfg(feature = "backend-candle")]
pub mod candle;

#[cfg(feature = "backend-pytorch")]
pub mod pytorch;

use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use crate::managed_runtime::ManagedBinaryId;
use crate::process::ProcessSpawner;
use crate::types::{ImageGenerationRequest, ImageGenerationResult, RerankRequest, RerankResponse};

#[cfg(feature = "backend-llamacpp")]
pub use llamacpp::LlamaCppBackend;

#[cfg(feature = "backend-ollama")]
pub use ollama::OllamaBackend;

#[cfg(feature = "backend-candle")]
pub use candle::CandleBackend;

#[cfg(feature = "backend-pytorch")]
pub use pytorch::PyTorchBackend;

pub use registry::{canonical_backend_key, BackendFactory, BackendRegistry};

/// Error types for backend operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Backend not ready")]
    NotReady,

    #[error("Backend not running: {0}")]
    NotRunning(String),

    #[error("Startup failed: {0}")]
    StartupFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Inference error: {0}")]
    Inference(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Capabilities that a backend may or may not support
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendCapabilities {
    /// Supports vision/multimodal models (image + text)
    pub vision: bool,
    /// Supports image generation / diffusion requests
    pub image_generation: bool,
    /// Supports embedding generation
    pub embeddings: bool,
    /// Supports document reranking
    pub reranking: bool,
    /// Has GPU acceleration available
    pub gpu: bool,
    /// Allows manual GPU device selection
    pub device_selection: bool,
    /// Supports streaming token output
    pub streaming: bool,
    /// Supports tool/function calling
    pub tool_calling: bool,
    /// Supports attaching to an already-running external inference host.
    pub external_connection: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendDefaultStartMode {
    Inference,
    Embedding,
}

/// Backend information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend identifier (e.g., "llama.cpp", "Ollama", "Candle")
    pub name: String,
    /// Stable backend key for contracts and selection state.
    pub backend_key: String,
    /// Human-readable description
    pub description: String,
    /// Backend capabilities
    pub capabilities: BackendCapabilities,
    /// Backend-owned recommended mode to start when the host selects this backend.
    pub default_start_mode: BackendDefaultStartMode,
    /// Whether this backend is currently active
    pub active: bool,
    /// Whether this backend is available (dependencies met)
    pub available: bool,
    /// Reason if unavailable
    pub unavailable_reason: Option<String>,
    /// Whether this backend can be auto-installed (binaries can be downloaded)
    pub can_install: bool,
    /// Managed runtime backing this backend, when applicable.
    #[serde(default)]
    pub runtime_binary_id: Option<ManagedBinaryId>,
}

/// Configuration for starting a backend
#[derive(Debug, Clone, Default)]
pub struct BackendConfig {
    /// External OpenAI-compatible base URL (for remote or already-running hosts)
    pub external_url: Option<String>,
    /// Optional host-selected port for managed HTTP sidecars.
    ///
    /// This remains backend-owned transport config rather than host-local
    /// recovery policy so restart flows can preserve the requested port
    /// through the normal backend start contract.
    pub port_override: Option<u16>,
    /// Model file path (for llama.cpp GGUF files)
    pub model_path: Option<std::path::PathBuf>,
    /// Vision projection file path (for llama.cpp mmproj)
    pub mmproj_path: Option<std::path::PathBuf>,
    /// Model name (for Ollama, e.g., "llava:13b")
    pub model_name: Option<String>,
    /// HuggingFace model ID (for Candle)
    pub model_id: Option<String>,
    /// Device configuration
    pub device: Option<String>,
    /// Number of GPU layers (-1 for all)
    pub gpu_layers: Option<i32>,
    /// Context size
    pub context_size: Option<u32>,
    /// Embedding mode
    pub embedding_mode: bool,
    /// Reranking mode
    pub reranking_mode: bool,
    /// Model type hint for PyTorch backend (dllm, sherry, text-generation).
    /// If None, auto-detected from config.json.
    pub model_type: Option<String>,
}

/// Backend-owned outcome for a successful runtime start request.
#[derive(Debug, Clone, Default)]
pub struct BackendStartOutcome {
    /// Whether the backend attached to an already-running runtime instead of
    /// launching a fresh one.
    pub runtime_reused: Option<bool>,
    /// Structured reason describing the lifecycle decision taken by the backend.
    pub lifecycle_decision_reason: Option<String>,
}

/// A streaming chunk from chat completion
#[derive(Debug, Clone, Serialize)]
pub struct ChatChunk {
    /// Text content of this chunk
    pub content: Option<String>,
    /// Whether this is the final chunk
    pub done: bool,
}

/// Embedding result
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingResult {
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Number of tokens in the input
    pub token_count: usize,
}

/// Re-export diffusion request/result types from the shared `types` module so
/// backend consumers can reach them from the backend facade.
pub type ImageRequest = ImageGenerationRequest;
pub type ImageResult = ImageGenerationResult;

/// The core trait that all inference backends must implement.
///
/// Backends can be HTTP-based (llama.cpp, Ollama, External) or in-process (Candle).
/// All use a common interface that application code can call without knowing
/// which backend is active.
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    // ─── IDENTITY ───────────────────────────────────────────────────

    /// Human-readable name for UI display
    fn name(&self) -> &'static str;

    /// Description of this backend
    fn description(&self) -> &'static str;

    /// What this backend supports
    fn capabilities(&self) -> BackendCapabilities;

    // ─── LIFECYCLE ──────────────────────────────────────────────────

    /// Initialize and start the backend with given configuration
    ///
    /// # Arguments
    /// * `config` - Backend configuration (model paths, device settings, etc.)
    /// * `spawner` - Process spawner for launching sidecar processes
    async fn start(
        &mut self,
        config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError>;

    /// Stop the backend and cleanup resources
    fn stop(&mut self);

    /// Is the backend ready to accept requests?
    fn is_ready(&self) -> bool;

    /// Health check - verify the backend is responding
    async fn health_check(&self) -> bool;

    /// Get the base URL for this backend (if HTTP-based)
    /// Returns None for in-process backends like Candle
    fn base_url(&self) -> Option<String>;

    // ─── INFERENCE ──────────────────────────────────────────────────

    /// Stream chat completion responses
    ///
    /// Takes a JSON-serialized OpenAI-compatible chat completion request
    /// and returns a stream of response chunks.
    async fn chat_completion_stream(
        &self,
        request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>;

    /// Generate embeddings for the given texts
    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError>;

    /// Rank candidate documents against a query.
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, BackendError>;

    /// Generate one or more images from a diffusion-capable backend.
    async fn generate_image(
        &self,
        _request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Err(BackendError::Inference(
            "Image generation not supported by this backend".to_string(),
        ))
    }

    /// Describe the active runtime semantics that govern whether one KV artifact
    /// may be reused by this backend.
    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Err(BackendError::Inference(
            "KV cache runtime fingerprint not supported by this backend".to_string(),
        ))
    }

    /// Describe the active model configuration for KV-compatibility checks.
    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Err(BackendError::Inference(
            "KV cache model fingerprint not supported by this backend".to_string(),
        ))
    }

    /// Persist the active runtime slot state into a backend-owned file.
    async fn save_kv_cache_slot(&self, _slot_id: u32, _path: &Path) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot save not supported by this backend".to_string(),
        ))
    }

    /// Restore a backend-owned file into a live runtime slot.
    async fn restore_kv_cache_slot(&self, _slot_id: u32, _path: &Path) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot restore not supported by this backend".to_string(),
        ))
    }

    /// Clear the active runtime slot state after a restore, failure, or reset.
    async fn clear_kv_cache_slot(&self, _slot_id: u32) -> Result<(), BackendError> {
        Err(BackendError::Inference(
            "KV cache slot clear not supported by this backend".to_string(),
        ))
    }

    /// Truncate a backend-owned KV artifact to the requested token position.
    async fn truncate_kv_cache_data(
        &self,
        _data: &[u8],
        _token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        Err(BackendError::Inference(
            "KV cache truncation not supported by this backend".to_string(),
        ))
    }
}
