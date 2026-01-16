//! Inference Gateway - Single entry point for all inference operations
//!
//! The gateway abstracts over different inference backends (llama.cpp, Ollama, Candle)
//! providing a unified interface for the rest of the application. It manages backend
//! lifecycle, switching, and forwards requests to the active backend.

use std::pin::Pin;
use std::sync::Arc;

use futures_util::Stream;
use tokio::sync::RwLock;

use crate::backend::{
    BackendCapabilities, BackendConfig, BackendError, BackendInfo, BackendRegistry, ChatChunk,
    EmbeddingResult, InferenceBackend,
};
use crate::config::EmbeddingMemoryMode;
use crate::process::ProcessSpawner;
use crate::types::ServerModeInfo;

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
    /// Last used inference config (for mode switching)
    last_inference_config: Arc<RwLock<Option<BackendConfig>>>,
    /// Current embedding memory mode
    embedding_memory_mode: Arc<RwLock<EmbeddingMemoryMode>>,
    /// Process spawner for starting backends
    spawner: Arc<RwLock<Option<Arc<dyn ProcessSpawner>>>>,
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
            last_inference_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new gateway with a specific backend
    pub fn with_backend(backend: Box<dyn InferenceBackend>, name: &str) -> Self {
        Self {
            backend: Arc::new(RwLock::new(backend)),
            registry: BackendRegistry::new(),
            current_backend_name: Arc::new(RwLock::new(name.to_string())),
            embedding_mode: Arc::new(RwLock::new(false)),
            last_inference_config: Arc::new(RwLock::new(None)),
            embedding_memory_mode: Arc::new(RwLock::new(EmbeddingMemoryMode::default())),
            spawner: Arc::new(RwLock::new(None)),
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

        // Store inference config for mode restoration
        if !config.embedding_mode {
            let mut last_config = self.last_inference_config.write().await;
            *last_config = Some(config.clone());
        }

        let mut guard = self.backend.write().await;
        guard
            .start(config, spawner)
            .await
            .map_err(GatewayError::Backend)
    }

    /// Stop the current backend
    pub async fn stop(&self) {
        let mut guard = self.backend.write().await;
        guard.stop();
        // Reset embedding mode
        let mut mode = self.embedding_mode.write().await;
        *mode = false;
    }

    /// Check if currently in embedding mode
    pub async fn is_embedding_mode(&self) -> bool {
        *self.embedding_mode.read().await
    }

    /// Check if currently in inference mode (ready and not embedding)
    pub async fn is_inference_mode(&self) -> bool {
        self.is_ready().await && !self.is_embedding_mode().await
    }

    /// Get the last inference config (for restoring after embedding mode)
    pub async fn last_inference_config(&self) -> Option<BackendConfig> {
        self.last_inference_config.read().await.clone()
    }

    /// Get server mode info (for legacy compatibility)
    pub async fn mode_info(&self) -> ServerModeInfo {
        let ready = self.is_ready().await;
        let is_embedding = self.is_embedding_mode().await;
        let url = self.base_url().await;

        ServerModeInfo {
            mode: if !ready {
                "none".to_string()
            } else if is_embedding {
                "sidecar_embedding".to_string()
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

    // ─── LEGACY COMPATIBILITY ───────────────────────────────────────

    /// Get a reference to the underlying backend for legacy code
    ///
    /// This is a temporary method for gradual migration. New code should
    /// use the gateway methods directly.
    pub fn backend(&self) -> Arc<RwLock<Box<dyn InferenceBackend>>> {
        self.backend.clone()
    }
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
}
