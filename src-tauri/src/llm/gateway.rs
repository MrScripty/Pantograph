//! Inference Gateway - Tauri wrapper around inference::InferenceGateway
//!
//! This module wraps the core `inference::InferenceGateway` and adds
//! Tauri-specific embedding server management for parallel embedding modes.
//! All backend lifecycle operations delegate to the crate gateway which
//! uses the `ProcessSpawner` abstraction.

use std::sync::Arc;

use tokio::sync::RwLock;

use inference::BackendConfig;
use inference::process::ProcessSpawner;

use super::embedding_server::EmbeddingServer;
use crate::config::{DeviceInfo, EmbeddingMemoryMode, ServerModeInfo};

/// Error types for gateway operations
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("{0}")]
    Inner(#[from] inference::GatewayError),

    #[error("Embedding server error: {0}")]
    EmbeddingServer(String),
}

/// Tauri inference gateway wrapping the core inference gateway.
///
/// Delegates all backend lifecycle operations to `inference::InferenceGateway`
/// and adds embedding server management for parallel embedding modes.
pub struct InferenceGateway {
    /// The core inference gateway (Arc-wrapped for sharing with CoreTaskExecutor)
    inner: Arc<inference::InferenceGateway>,
    /// Dedicated embedding server for parallel modes (CPU+GPU or GPU+GPU)
    embedding_server: Arc<RwLock<Option<EmbeddingServer>>>,
    /// Process spawner (shared with inner gateway and embedding server)
    spawner: Arc<dyn ProcessSpawner>,
}

impl InferenceGateway {
    /// Create a new gateway wrapping the core inference gateway.
    ///
    /// The `spawner` is injected into the core gateway for backend process
    /// management and stored for embedding server use.
    ///
    /// **Important**: Call `init()` after construction to complete async setup.
    pub fn new(spawner: Arc<dyn ProcessSpawner>) -> Self {
        let inner = Arc::new(inference::InferenceGateway::new());
        Self {
            inner,
            embedding_server: Arc::new(RwLock::new(None)),
            spawner,
        }
    }

    /// Complete async initialization (sets spawner on inner gateway).
    pub async fn init(&self) {
        self.inner.set_spawner(self.spawner.clone()).await;
    }

    /// Get an Arc clone of the inner crate gateway.
    ///
    /// Used to share the gateway with `CoreTaskExecutor` for inference
    /// node execution.
    pub fn inner_arc(&self) -> Arc<inference::InferenceGateway> {
        self.inner.clone()
    }

    // ─── LIFECYCLE METHODS ──────────────────────────────────────────

    /// Start the current backend with the given configuration.
    ///
    /// Delegates to the core gateway which uses the injected `ProcessSpawner`.
    pub async fn start(&self, config: &BackendConfig) -> Result<(), GatewayError> {
        self.inner.start(config).await.map_err(GatewayError::Inner)
    }

    /// Stop the current backend.
    pub async fn stop(&self) {
        self.inner.stop().await;
    }

    /// Stop the dedicated embedding server (if running).
    pub async fn stop_embedding_server(&self) {
        let mut guard = self.embedding_server.write().await;
        if let Some(ref mut server) = *guard {
            server.stop();
        }
        *guard = None;
    }

    /// Stop both the main backend and embedding server.
    pub async fn stop_all(&self) {
        self.stop().await;
        self.stop_embedding_server().await;
    }

    // ─── EMBEDDING SERVER MANAGEMENT ───────────────────────────────────

    /// Start the dedicated embedding server for parallel modes.
    ///
    /// This starts a separate llama.cpp instance for embedding operations,
    /// allowing vector search to work while the main LLM is loaded.
    pub async fn start_embedding_server(
        &self,
        model_path: &str,
        mode: EmbeddingMemoryMode,
        devices: &[DeviceInfo],
    ) -> Result<(), GatewayError> {
        // Sequential mode doesn't need a dedicated server
        if mode == EmbeddingMemoryMode::Sequential {
            log::info!("Sequential embedding mode: no dedicated server needed");
            return Ok(());
        }

        {
            let mut guard = self.embedding_server.write().await;
            if let Some(server) = guard.as_mut() {
                if server.matches_runtime(model_path, mode.clone()) {
                    server.mark_runtime_reused();
                    log::info!("Reusing dedicated embedding server");
                    return Ok(());
                }
            }
        }

        // Create or restart the dedicated embedding server with backend-owned
        // lifecycle tracking.
        let mut server = EmbeddingServer::new(mode.clone());
        server
            .start(model_path, &self.spawner, devices)
            .await
            .map_err(GatewayError::EmbeddingServer)?;
        let mut guard = self.embedding_server.write().await;
        *guard = Some(server);

        log::info!("Dedicated embedding server started");
        Ok(())
    }

    /// Get the URL of the embedding server (if available).
    ///
    /// Returns:
    /// - In parallel modes: URL of the dedicated embedding server
    /// - In sequential mode: None (use main gateway with mode switching)
    /// - If main backend is in embedding mode: main backend URL
    pub async fn embedding_url(&self) -> Option<String> {
        // First check dedicated embedding server
        {
            let server = self.embedding_server.read().await;
            if let Some(ref srv) = *server {
                if srv.is_ready() {
                    return Some(srv.base_url());
                }
            }
        }

        // Fall back to main server if in embedding mode
        if self.is_embedding_mode().await {
            return self.base_url().await;
        }

        None
    }

    /// Check if the embedding server is ready.
    pub async fn is_embedding_server_ready(&self) -> bool {
        let server = self.embedding_server.read().await;
        if let Some(ref srv) = *server {
            return srv.is_ready();
        }
        false
    }

    // ─── DELEGATED QUERY METHODS ──────────────────────────────────────

    /// Get the name of the currently active backend.
    pub async fn current_backend_name(&self) -> String {
        self.inner.current_backend_name().await
    }

    /// Switch to a different backend.
    pub async fn switch_backend(&self, name: &str) -> Result<(), GatewayError> {
        self.inner
            .switch_backend(name)
            .await
            .map_err(GatewayError::Inner)
    }

    /// List all available backends with their info.
    pub fn available_backends(&self) -> Vec<inference::BackendInfo> {
        self.inner.available_backends()
    }

    /// Check if the current backend is ready.
    pub async fn is_ready(&self) -> bool {
        self.inner.is_ready().await
    }

    /// Get the base URL of the current backend (if HTTP-based).
    pub async fn base_url(&self) -> Option<String> {
        self.inner.base_url().await
    }

    /// Get capabilities of the current backend.
    pub async fn capabilities(&self) -> inference::BackendCapabilities {
        self.inner.capabilities().await
    }

    /// Get the backend-owned runtime lifecycle snapshot.
    pub async fn runtime_lifecycle_snapshot(&self) -> inference::RuntimeLifecycleSnapshot {
        self.inner.runtime_lifecycle_snapshot().await
    }

    /// Get the backend-owned lifecycle snapshot for the dedicated embedding runtime.
    pub async fn embedding_runtime_lifecycle_snapshot(
        &self,
    ) -> Option<inference::RuntimeLifecycleSnapshot> {
        self.embedding_server
            .read()
            .await
            .as_ref()
            .map(EmbeddingServer::runtime_lifecycle_snapshot)
    }

    /// Check if currently in embedding mode.
    pub async fn is_embedding_mode(&self) -> bool {
        self.inner.is_embedding_mode().await
    }

    /// Check if currently in inference mode (ready and not embedding).
    pub async fn is_inference_mode(&self) -> bool {
        self.inner.is_inference_mode().await
    }

    /// Get the last inference config (for restoring after embedding mode).
    pub async fn last_inference_config(&self) -> Option<BackendConfig> {
        self.inner.last_inference_config().await
    }

    /// Get server mode info for the frontend.
    pub async fn mode_info(&self) -> ServerModeInfo {
        let info = self.inner.mode_info().await;
        // Convert from crate type to local config type
        ServerModeInfo {
            mode: info.mode,
            ready: info.ready,
            url: info.url,
            model_path: info.model_path,
            is_embedding_mode: info.is_embedding_mode,
        }
    }
}

/// Shared gateway type for Tauri state.
pub type SharedGateway = Arc<InferenceGateway>;

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use async_trait::async_trait;
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use tokio::sync::mpsc;

    use super::*;

    struct MockProcessHandle;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            11
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
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            Err("spawn should not be called in reuse-path tests".to_string())
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    #[tokio::test]
    async fn start_embedding_server_reuses_matching_runtime() {
        let gateway = InferenceGateway::new(Arc::new(MockProcessSpawner));

        let mut server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");

        {
            let mut guard = gateway.embedding_server.write().await;
            *guard = Some(server);
        }

        gateway
            .start_embedding_server("/models/embed.gguf", EmbeddingMemoryMode::CpuParallel, &[])
            .await
            .expect("matching runtime should be reused");

        let snapshot = gateway
            .embedding_runtime_lifecycle_snapshot()
            .await
            .expect("snapshot should exist");
        assert_eq!(snapshot.runtime_reused, Some(true));
        assert!(snapshot.active);
    }

    #[tokio::test]
    async fn embedding_runtime_lifecycle_snapshot_returns_none_without_server() {
        let gateway = InferenceGateway::new(Arc::new(MockProcessSpawner));

        assert_eq!(gateway.embedding_runtime_lifecycle_snapshot().await, None);
    }
}
