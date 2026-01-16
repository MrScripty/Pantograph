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

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};

use crate::process::ProcessSpawner;

#[cfg(feature = "backend-llamacpp")]
pub use llamacpp::LlamaCppBackend;

#[cfg(feature = "backend-ollama")]
pub use ollama::OllamaBackend;

#[cfg(feature = "backend-candle")]
pub use candle::CandleBackend;

pub use registry::{BackendFactory, BackendRegistry};

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
    /// Supports embedding generation
    pub embeddings: bool,
    /// Has GPU acceleration available
    pub gpu: bool,
    /// Allows manual GPU device selection
    pub device_selection: bool,
    /// Supports streaming token output
    pub streaming: bool,
    /// Supports tool/function calling
    pub tool_calling: bool,
}

/// Backend information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend identifier (e.g., "llama.cpp", "Ollama", "Candle")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Backend capabilities
    pub capabilities: BackendCapabilities,
    /// Whether this backend is currently active
    pub active: bool,
    /// Whether this backend is available (dependencies met)
    pub available: bool,
    /// Reason if unavailable
    pub unavailable_reason: Option<String>,
    /// Whether this backend can be auto-installed (binaries can be downloaded)
    pub can_install: bool,
}

/// Configuration for starting a backend
#[derive(Debug, Clone, Default)]
pub struct BackendConfig {
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
    ) -> Result<(), BackendError>;

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
}
