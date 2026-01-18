//! llama.cpp backend implementation
//!
//! This backend wraps the existing LlamaServer sidecar management code,
//! providing the InferenceBackend trait interface without modifying
//! the original implementation.

use async_trait::async_trait;
use tauri::AppHandle;

use super::{BackendCapabilities, BackendConfig, BackendError, InferenceBackend};
use crate::config::DeviceConfig;
use crate::llm::server::LlamaServer;

/// llama.cpp backend using sidecar process management
///
/// This backend wraps the existing LlamaServer implementation,
/// which manages a llama-server binary as a sidecar process.
/// The sidecar exposes an OpenAI-compatible API that we forward
/// requests to.
pub struct LlamaCppBackend {
    /// The underlying server manager
    server: LlamaServer,
}

impl LlamaCppBackend {
    /// Create a new llama.cpp backend
    pub fn new() -> Self {
        Self {
            server: LlamaServer::new(),
        }
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: true,           // GGUF + mmproj support
            embeddings: true,       // Via --embedding mode
            gpu: true,              // CUDA, Vulkan, Metal
            device_selection: true, // Manual device choice
            streaming: true,        // SSE streaming
            tool_calling: true,     // Via OpenAI-compatible API
        }
    }
}

impl Default for LlamaCppBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for LlamaCppBackend {
    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(&mut self, config: &BackendConfig, app: &AppHandle) -> Result<(), BackendError> {
        // Build device config from BackendConfig
        let device_config = DeviceConfig {
            device: config.device.clone().unwrap_or_else(|| "auto".to_string()),
            gpu_layers: config.gpu_layers.unwrap_or(-1),
        };

        if config.embedding_mode {
            // Start in embedding mode
            let model_path = config
                .model_path
                .as_ref()
                .ok_or_else(|| BackendError::Config("model_path required for embedding mode".to_string()))?;

            self.server
                .start_sidecar_embedding(
                    app,
                    &model_path.to_string_lossy(),
                    &device_config,
                )
                .await
                .map_err(|e| {
                    if e.to_lowercase().contains("out of memory") || e.to_lowercase().contains("oom") {
                        BackendError::OutOfMemory(e)
                    } else {
                        BackendError::StartupFailed(e)
                    }
                })
        } else {
            // Start in inference mode (VLM with vision)
            let model_path = config
                .model_path
                .as_ref()
                .ok_or_else(|| BackendError::Config("model_path required".to_string()))?;

            let mmproj_path = config
                .mmproj_path
                .as_ref()
                .ok_or_else(|| BackendError::Config("mmproj_path required for inference mode".to_string()))?;

            self.server
                .start_sidecar_inference(
                    app,
                    &model_path.to_string_lossy(),
                    &mmproj_path.to_string_lossy(),
                    &device_config,
                )
                .await
                .map_err(|e| {
                    if e.to_lowercase().contains("out of memory") || e.to_lowercase().contains("oom") {
                        BackendError::OutOfMemory(e)
                    } else {
                        BackendError::StartupFailed(e)
                    }
                })
        }
    }

    fn stop(&mut self) {
        self.server.stop();
    }

    fn is_ready(&self) -> bool {
        self.server.is_ready()
    }

    fn base_url(&self) -> Option<String> {
        self.server.base_url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
