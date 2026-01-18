//! Pluggable inference backend abstraction
//!
//! This module provides a trait-based abstraction for different inference engines
//! (llama.cpp, Ollama, Candle, external APIs). All backends implement the same
//! interface, allowing runtime switching between engines.

pub mod llamacpp;
#[cfg(feature = "backend-ollama")]
pub mod ollama;
#[cfg(feature = "backend-candle")]
pub mod candle;
pub mod registry;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

pub use llamacpp::LlamaCppBackend;
#[cfg(feature = "backend-ollama")]
pub use ollama::OllamaBackend;
#[cfg(feature = "backend-candle")]
pub use candle::CandleBackend;
pub use registry::BackendRegistry;

/// Error types for backend operations
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
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
    /// Device configuration
    pub device: Option<String>,
    /// Number of GPU layers (-1 for all)
    pub gpu_layers: Option<i32>,
    /// Embedding mode
    pub embedding_mode: bool,
}

/// The core trait that all inference backends must implement.
///
/// Backends can be HTTP-based (llama.cpp, Ollama, External) or in-process (Candle).
/// All use a common interface that application code can call without knowing
/// which backend is active.
#[async_trait]
pub trait InferenceBackend: Send + Sync {
    /// What this backend supports
    fn capabilities(&self) -> BackendCapabilities;

    /// Initialize and start the backend with given configuration
    async fn start(&mut self, config: &BackendConfig, app: &AppHandle) -> Result<(), BackendError>;

    /// Stop the backend and cleanup resources
    fn stop(&mut self);

    /// Is the backend ready to accept requests?
    fn is_ready(&self) -> bool;

    /// Get the base URL for this backend (if HTTP-based)
    /// Returns None for in-process backends like Candle
    fn base_url(&self) -> Option<String>;
}
