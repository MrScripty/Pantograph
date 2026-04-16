//! Inference Gateway - Tauri wrapper around inference::InferenceGateway
//!
//! This module wraps the core `inference::InferenceGateway` and composes the
//! inference-owned dedicated embedding runtime for parallel embedding modes.
//! All runtime lifecycle operations delegate to backend-owned Rust types.

use std::sync::Arc;

use tokio::sync::RwLock;

use inference::config::DeviceInfo as InferenceDeviceInfo;
use inference::process::ProcessSpawner;
#[cfg(test)]
use inference::LlamaCppEmbeddingRuntime;
use inference::{
    BackendConfig, DedicatedEmbeddingRuntimeManager, EmbeddingMemoryMode as InferenceEmbeddingMode,
    EmbeddingRuntimePreparation, EmbeddingStartRequest, GatewayError as InferenceGatewayError,
    InferenceStartRequest,
};

use crate::config::{DeviceInfo, EmbeddingMemoryMode, ServerModeInfo};

/// Error types for gateway operations
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("{0}")]
    Inner(#[from] inference::GatewayError),

    #[error("Embedding runtime error: {0}")]
    EmbeddingRuntime(String),
}

/// Tauri inference gateway wrapping the core inference gateway.
///
/// Delegates all backend lifecycle operations to `inference::InferenceGateway`
/// and adds embedding server management for parallel embedding modes.
pub struct InferenceGateway {
    /// The core inference gateway (Arc-wrapped for sharing with CoreTaskExecutor)
    inner: Arc<inference::InferenceGateway>,
    /// Dedicated embedding runtime for parallel modes (CPU+GPU or GPU+GPU)
    embedding_runtime: Arc<RwLock<DedicatedEmbeddingRuntimeManager>>,
    /// Process spawner (shared with inner gateway and embedding runtime)
    spawner: Arc<dyn ProcessSpawner>,
}

fn to_inference_embedding_mode(mode: &EmbeddingMemoryMode) -> InferenceEmbeddingMode {
    match mode {
        EmbeddingMemoryMode::CpuParallel => InferenceEmbeddingMode::CpuParallel,
        EmbeddingMemoryMode::GpuParallel => InferenceEmbeddingMode::GpuParallel,
        EmbeddingMemoryMode::Sequential => InferenceEmbeddingMode::Sequential,
    }
}

fn to_inference_devices(devices: &[DeviceInfo]) -> Vec<InferenceDeviceInfo> {
    devices
        .iter()
        .map(|device| InferenceDeviceInfo {
            id: device.id.clone(),
            name: device.name.clone(),
            total_vram_mb: device.total_vram_mb,
            free_vram_mb: device.free_vram_mb,
        })
        .collect()
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
            embedding_runtime: Arc::new(RwLock::new(DedicatedEmbeddingRuntimeManager::new())),
            spawner,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_backend(
        backend: Box<dyn inference::InferenceBackend>,
        name: &str,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Self {
        let inner = Arc::new(inference::InferenceGateway::with_backend(backend, name));
        Self {
            inner,
            embedding_runtime: Arc::new(RwLock::new(DedicatedEmbeddingRuntimeManager::new())),
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

    /// Stop the dedicated embedding runtime (if running).
    pub async fn stop_embedding_server(&self) {
        self.embedding_runtime.write().await.stop();
    }

    /// Stop both the main backend and embedding runtime.
    pub async fn stop_all(&self) {
        self.stop().await;
        self.stop_embedding_server().await;
    }

    // ─── EMBEDDING RUNTIME MANAGEMENT ──────────────────────────────────

    /// Start the dedicated embedding runtime for parallel modes.
    ///
    /// This starts a separate llama.cpp instance for embedding operations,
    /// allowing vector search to work while the main LLM is loaded.
    pub async fn start_embedding_server(
        &self,
        model_path: &str,
        mode: EmbeddingMemoryMode,
        devices: &[DeviceInfo],
    ) -> Result<(), GatewayError> {
        if mode == EmbeddingMemoryMode::Sequential {
            log::info!("Sequential embedding mode: no dedicated embedding runtime needed");
            return Ok(());
        }

        let inference_mode = to_inference_embedding_mode(&mode);
        let inference_devices = to_inference_devices(devices);
        self.embedding_runtime
            .write()
            .await
            .ensure_runtime(
                model_path,
                inference_mode,
                &self.spawner,
                &inference_devices,
            )
            .await
            .map_err(GatewayError::EmbeddingRuntime)
    }

    /// Get the URL of the dedicated embedding runtime (if available).
    ///
    /// Returns:
    /// - In parallel modes: URL of the dedicated embedding server
    /// - In sequential mode: None (use main gateway with mode switching)
    /// - If main backend is in embedding mode: main backend URL
    pub async fn embedding_url(&self) -> Option<String> {
        if let Some(url) = self.embedding_runtime.read().await.base_url() {
            return Some(url);
        }

        if self.is_embedding_mode().await {
            return self.base_url().await;
        }

        None
    }

    /// Get the URL of the dedicated embedding runtime only.
    pub async fn dedicated_embedding_base_url(&self) -> Option<String> {
        self.embedding_runtime.read().await.base_url()
    }

    /// Check if the dedicated embedding runtime is ready.
    pub async fn is_embedding_server_ready(&self) -> bool {
        self.embedding_runtime.read().await.is_ready()
    }

    // ─── DELEGATED QUERY METHODS ──────────────────────────────────────

    /// Get the name of the currently active backend.
    pub async fn current_backend_name(&self) -> String {
        self.inner.current_backend_name().await
    }

    /// Build backend-owned startup config for the active inference runtime.
    pub async fn build_inference_start_config(
        &self,
        request: InferenceStartRequest,
    ) -> Result<BackendConfig, InferenceGatewayError> {
        self.inner.build_inference_start_config(request).await
    }

    /// Build backend-owned startup config for the active embedding runtime.
    pub async fn build_embedding_start_config(
        &self,
        request: EmbeddingStartRequest,
    ) -> Result<BackendConfig, InferenceGatewayError> {
        self.inner.build_embedding_start_config(request).await
    }

    /// Start the active backend in embedding mode and capture restore context.
    pub async fn prepare_embedding_runtime(
        &self,
        request: EmbeddingStartRequest,
    ) -> Result<EmbeddingRuntimePreparation, InferenceGatewayError> {
        self.inner.prepare_embedding_runtime(request).await
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
        self.embedding_runtime
            .read()
            .await
            .runtime_lifecycle_snapshot()
    }

    #[cfg(test)]
    pub(crate) async fn set_test_embedding_server(&self, server: LlamaCppEmbeddingRuntime) {
        self.embedding_runtime
            .write()
            .await
            .set_test_runtime(server);
    }

    /// Check if currently in embedding mode.
    pub async fn is_embedding_mode(&self) -> bool {
        self.inner.is_embedding_mode().await
    }

    /// Restore the last non-embedding inference runtime when available.
    pub async fn restore_inference_runtime(
        &self,
        restore_config: Option<BackendConfig>,
    ) -> Result<(), InferenceGatewayError> {
        self.inner.restore_inference_runtime(restore_config).await
    }

    /// Get the saved restart config for the active runtime before teardown.
    pub async fn restart_runtime_config(&self) -> Option<BackendConfig> {
        self.inner.restart_runtime_config().await
    }

    /// Get server mode info for the frontend.
    pub async fn mode_info(&self) -> ServerModeInfo {
        let mut mode_info = self.inner.mode_info().await;
        let embedding_runtime = self.embedding_runtime.read().await;
        mode_info.embedding_runtime = embedding_runtime.runtime_lifecycle_snapshot();
        mode_info.embedding_model_target = embedding_runtime.model_target();
        mode_info
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

        let mut server = LlamaCppEmbeddingRuntime::new(InferenceEmbeddingMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        gateway.set_test_embedding_server(server).await;

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

    #[tokio::test]
    async fn mode_info_includes_embedding_runtime_snapshot() {
        let gateway = InferenceGateway::new(Arc::new(MockProcessSpawner));
        gateway.init().await;

        let mut server = LlamaCppEmbeddingRuntime::new(InferenceEmbeddingMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
        server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-1".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(25),
            warmup_duration_ms: Some(15),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        });
        gateway.set_test_embedding_server(server).await;

        let mode = gateway.mode_info().await;

        assert!(mode.active_runtime.is_some());
        assert_eq!(
            mode.embedding_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_id.as_deref()),
            Some("llama.cpp.embedding")
        );
        assert_eq!(
            mode.embedding_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_reused),
            Some(false)
        );
        assert_eq!(
            mode.embedding_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_instance_id.as_deref()),
            Some("llama-cpp-embedding-1")
        );
        assert_eq!(
            mode.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
    }
}
