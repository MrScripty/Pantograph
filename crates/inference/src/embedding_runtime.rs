//! Dedicated llama.cpp embedding runtime management.
//!
//! Owns the lifecycle, reuse checks, and runtime metrics for the dedicated
//! embedding sidecar used in parallel embedding modes.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{DeviceConfig, DeviceInfo, EmbeddingMemoryMode};
use crate::constants::{device_types, hosts};
use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use crate::RuntimeLifecycleSnapshot;

/// Default port for the embedding server (separate from main LLM on 8080)
const EMBEDDING_SERVER_PORT: u16 = 8081;

/// Minimum VRAM (MB) needed for embedding model (Qwen3-Embedding-0.6B ≈ 600MB)
const EMBEDDING_MODEL_VRAM_MB: u64 = 800;

/// PID file name for the embedding server
const EMBEDDING_PID_FILE: &str = "embedding-server.pid";

/// Canonical runtime identifier for the dedicated embedding sidecar.
const EMBEDDING_RUNTIME_ID: &str = "llama.cpp.embedding";

/// Dedicated embedding runtime that can run alongside the main LLM runtime.
pub struct LlamaCppEmbeddingRuntime {
    child: Option<Box<dyn ProcessHandle>>,
    port: u16,
    mode: EmbeddingMemoryMode,
    ready: bool,
    pid_file: Option<PathBuf>,
    model_path: Option<String>,
    runtime_lifecycle: RuntimeLifecycleSnapshot,
    runtime_instance_sequence: u64,
}

/// Backend-owned coordinator for the optional dedicated embedding runtime.
///
/// Hosts may compose this manager, but the reuse/start/stop logic stays in the
/// inference crate rather than being reimplemented in adapters.
#[derive(Default)]
pub struct DedicatedEmbeddingRuntimeManager {
    runtime: Option<LlamaCppEmbeddingRuntime>,
}

impl LlamaCppEmbeddingRuntime {
    /// Create a new embedding runtime manager.
    pub fn new(mode: EmbeddingMemoryMode) -> Self {
        Self {
            child: None,
            port: EMBEDDING_SERVER_PORT,
            mode,
            ready: false,
            pid_file: None,
            model_path: None,
            runtime_lifecycle: RuntimeLifecycleSnapshot {
                runtime_id: Some(EMBEDDING_RUNTIME_ID.to_string()),
                ..RuntimeLifecycleSnapshot::default()
            },
            runtime_instance_sequence: 0,
        }
    }

    /// Check if there's enough free VRAM for the embedding model.
    pub fn check_vram_available(devices: &[DeviceInfo]) -> bool {
        devices
            .iter()
            .any(|device| device.id != "none" && device.free_vram_mb >= EMBEDDING_MODEL_VRAM_MB)
    }

    /// Start the embedding runtime based on memory mode.
    pub async fn start(
        &mut self,
        model_path: &str,
        spawner: &Arc<dyn ProcessSpawner>,
        devices: &[DeviceInfo],
    ) -> Result<(), String> {
        if self.mode == EmbeddingMemoryMode::Sequential {
            log::info!("Sequential mode: no dedicated embedding runtime needed");
            return Ok(());
        }

        let warmup_started_at_ms = unix_timestamp_ms();
        self.mark_start_attempt(warmup_started_at_ms);

        self.stop();

        let device_config = match self.mode {
            EmbeddingMemoryMode::CpuParallel => {
                log::info!("Starting embedding runtime on CPU (RAM)");
                DeviceConfig {
                    device: "none".to_string(),
                    gpu_layers: 0,
                }
            }
            EmbeddingMemoryMode::GpuParallel => {
                if !Self::check_vram_available(devices) {
                    return Err(format!(
                        "Insufficient VRAM for both models. Need at least {}MB free. Use 'CPU + GPU' mode instead.",
                        EMBEDDING_MODEL_VRAM_MB
                    ));
                }
                log::info!("Starting embedding runtime on GPU (VRAM)");
                DeviceConfig {
                    device: device_types::AUTO.to_string(),
                    gpu_layers: -1,
                }
            }
            EmbeddingMemoryMode::Sequential => return Ok(()),
        };

        match self.start_server(model_path, spawner, &device_config).await {
            Ok(()) => {
                self.mark_start_success(warmup_started_at_ms);
                Ok(())
            }
            Err(error) => {
                self.mark_start_failure(warmup_started_at_ms, error.clone());
                Err(error)
            }
        }
    }

    async fn start_server(
        &mut self,
        model_path: &str,
        spawner: &Arc<dyn ProcessSpawner>,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        let port_str = self.port.to_string();
        let gpu_layers_str = device.gpu_layers.to_string();

        let mut args: Vec<String> = vec![
            "-m".to_string(),
            model_path.to_string(),
            "--port".to_string(),
            port_str,
            "--host".to_string(),
            hosts::LOCAL.to_string(),
            "--embedding".to_string(),
            "-ngl".to_string(),
            gpu_layers_str,
        ];

        let pid_file = spawner
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?
            .join(EMBEDDING_PID_FILE);
        args.push("--pid-file".to_string());
        args.push(pid_file.to_string_lossy().to_string());

        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting embedding runtime on port {} with device={}, gpu_layers={}",
            self.port,
            device.device,
            device.gpu_layers
        );

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (mut rx, child) = spawner
            .spawn_sidecar("llama-server-wrapper", &args_refs)
            .await
            .map_err(|e| format!("Failed to spawn embedding runtime: {}", e))?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.model_path = Some(model_path.to_string());

        self.wait_for_ready(&mut rx).await?;
        self.ready = true;
        Ok(())
    }

    fn is_server_listening(line: &str) -> bool {
        (line.contains("server") && line.contains("listening"))
            || line.contains("HTTP server listening")
    }

    async fn verify_http_ready(&self, timeout_ms: u64) -> Result<(), String> {
        let health_url = format!("{}/health", self.base_url());
        let start = std::time::Instant::now();

        while start.elapsed().as_millis() < timeout_ms as u128 {
            match reqwest::get(&health_url).await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!("Embedding runtime HTTP verified on port {}", self.port);
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }

        Err(format!(
            "Embedding runtime HTTP not responding after {}ms",
            timeout_ms
        ))
    }

    async fn wait_for_ready(
        &self,
        rx: &mut tokio::sync::mpsc::Receiver<ProcessEvent>,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let timeout_ms = 60000;

        while start.elapsed().as_millis() < timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await {
                Ok(Some(event)) => match event {
                    ProcessEvent::Stdout(line) => {
                        let line_str = String::from_utf8_lossy(&line);
                        if !line_str.contains("llama_model_loader: - kv")
                            && !line_str.contains("llama_model_loader: - type")
                        {
                            log::debug!("[embedding-runtime] {}", line_str);
                        }

                        if Self::is_server_listening(&line_str) {
                            log::debug!(
                                "Stdout reports embedding runtime listening, verifying HTTP..."
                            );
                            return self.verify_http_ready(5000).await;
                        }
                    }
                    ProcessEvent::Stderr(line) => {
                        let line_str = String::from_utf8_lossy(&line);
                        if !line_str.contains("llama_model_loader: - kv")
                            && !line_str.contains("llama_model_loader: - type")
                        {
                            log::debug!("[embedding-runtime stderr] {}", line_str);
                        }

                        if line_str.to_lowercase().contains("out of memory") {
                            return Err("Embedding runtime: Out of memory".to_string());
                        }

                        if Self::is_server_listening(&line_str) {
                            log::debug!(
                                "Stderr reports embedding runtime listening, verifying HTTP..."
                            );
                            return self.verify_http_ready(5000).await;
                        }
                    }
                    ProcessEvent::Terminated(code) => {
                        return Err(format!(
                            "Embedding runtime terminated unexpectedly with code: {:?}",
                            code
                        ));
                    }
                    ProcessEvent::Error(err) => {
                        return Err(format!("Embedding runtime error: {}", err));
                    }
                },
                Ok(None) => {
                    return Err("Embedding runtime process ended without ready signal".to_string());
                }
                Err(_) => continue,
            }
        }

        Err(format!(
            "Embedding runtime failed to start within {} seconds",
            timeout_ms / 1000
        ))
    }

    /// Get the base URL of the embedding runtime.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", hosts::LOCAL, self.port)
    }

    /// Check if the runtime is ready.
    pub fn is_ready(&self) -> bool {
        self.ready && self.child.is_some()
    }

    /// Return the backend-owned lifecycle snapshot for the embedding runtime.
    pub fn runtime_lifecycle_snapshot(&self) -> RuntimeLifecycleSnapshot {
        self.runtime_lifecycle.clone()
    }

    /// Return the current model target for the embedding runtime, when known.
    pub fn model_target(&self) -> Option<String> {
        self.model_path.clone()
    }

    /// Return whether the active embedding runtime can satisfy the request.
    pub fn matches_runtime(&self, model_path: &str, mode: EmbeddingMemoryMode) -> bool {
        self.is_ready() && self.mode == mode && self.model_path.as_deref() == Some(model_path)
    }

    /// Mark the current embedding runtime as reused by a later request.
    pub fn mark_runtime_reused(&mut self) {
        self.runtime_lifecycle.runtime_reused = Some(true);
        self.runtime_lifecycle.active = self.is_ready();
        self.runtime_lifecycle.last_error = None;
        self.refresh_lifecycle_decision_reason();
    }

    #[doc(hidden)]
    pub fn set_test_ready_state(&mut self, child: Box<dyn ProcessHandle>, model_path: &str) {
        self.child = Some(child);
        self.ready = true;
        self.model_path = Some(model_path.to_string());
    }

    #[doc(hidden)]
    pub fn set_test_runtime_lifecycle_snapshot(&mut self, snapshot: RuntimeLifecycleSnapshot) {
        self.runtime_lifecycle = snapshot;
    }

    /// Stop the embedding runtime.
    pub fn stop(&mut self) {
        if let Some(ref child) = self.child {
            log::info!("Stopping embedding runtime");
            let _ = child.kill();
        }
        self.child = None;
        self.ready = false;
        self.model_path = None;
        self.runtime_lifecycle.active = false;

        if let Some(ref pid_file) = self.pid_file {
            let _ = std::fs::remove_file(pid_file);
        }
        self.pid_file = None;
    }

    fn mark_start_attempt(&mut self, warmup_started_at_ms: u64) {
        self.runtime_lifecycle.runtime_id = Some(EMBEDDING_RUNTIME_ID.to_string());
        self.runtime_lifecycle.runtime_instance_id = None;
        self.runtime_lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
        self.runtime_lifecycle.warmup_completed_at_ms = None;
        self.runtime_lifecycle.warmup_duration_ms = None;
        self.runtime_lifecycle.runtime_reused = Some(false);
        self.runtime_lifecycle.lifecycle_decision_reason = None;
        self.runtime_lifecycle.active = false;
        self.runtime_lifecycle.last_error = None;
        self.refresh_lifecycle_decision_reason();
    }

    fn mark_start_success(&mut self, warmup_started_at_ms: u64) {
        let warmup_completed_at_ms = unix_timestamp_ms();
        self.runtime_instance_sequence = self.runtime_instance_sequence.saturating_add(1);
        self.runtime_lifecycle.runtime_id = Some(EMBEDDING_RUNTIME_ID.to_string());
        self.runtime_lifecycle.runtime_instance_id = Some(format!(
            "llama-cpp-embedding-{}",
            self.runtime_instance_sequence
        ));
        self.runtime_lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
        self.runtime_lifecycle.warmup_completed_at_ms = Some(warmup_completed_at_ms);
        self.runtime_lifecycle.warmup_duration_ms =
            Some(warmup_completed_at_ms.saturating_sub(warmup_started_at_ms));
        self.runtime_lifecycle.runtime_reused = Some(false);
        self.runtime_lifecycle.active = true;
        self.runtime_lifecycle.last_error = None;
        self.refresh_lifecycle_decision_reason();
    }

    fn mark_start_failure(&mut self, warmup_started_at_ms: u64, error: String) {
        let warmup_completed_at_ms = unix_timestamp_ms();
        self.runtime_lifecycle.runtime_id = Some(EMBEDDING_RUNTIME_ID.to_string());
        self.runtime_lifecycle.warmup_started_at_ms = Some(warmup_started_at_ms);
        self.runtime_lifecycle.warmup_completed_at_ms = Some(warmup_completed_at_ms);
        self.runtime_lifecycle.warmup_duration_ms =
            Some(warmup_completed_at_ms.saturating_sub(warmup_started_at_ms));
        self.runtime_lifecycle.runtime_reused = Some(false);
        self.runtime_lifecycle.active = false;
        self.runtime_lifecycle.last_error = Some(error);
        self.refresh_lifecycle_decision_reason();
    }

    fn refresh_lifecycle_decision_reason(&mut self) {
        self.runtime_lifecycle.lifecycle_decision_reason = self
            .runtime_lifecycle
            .normalized_lifecycle_decision_reason();
    }
}

impl DedicatedEmbeddingRuntimeManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn ensure_runtime(
        &mut self,
        model_path: &str,
        mode: EmbeddingMemoryMode,
        spawner: &Arc<dyn ProcessSpawner>,
        devices: &[DeviceInfo],
    ) -> Result<(), String> {
        if mode == EmbeddingMemoryMode::Sequential {
            log::info!("Sequential embedding mode: no dedicated embedding runtime needed");
            return Ok(());
        }

        if let Some(runtime) = self.runtime.as_mut() {
            if runtime.matches_runtime(model_path, mode.clone()) {
                runtime.mark_runtime_reused();
                log::info!("Reusing dedicated embedding runtime");
                return Ok(());
            }
        }

        let mut runtime = LlamaCppEmbeddingRuntime::new(mode);
        runtime.start(model_path, spawner, devices).await?;
        self.runtime = Some(runtime);
        log::info!("Dedicated embedding runtime started");
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(runtime) = self.runtime.as_mut() {
            runtime.stop();
        }
        self.runtime = None;
    }

    pub fn base_url(&self) -> Option<String> {
        self.runtime
            .as_ref()
            .filter(|runtime| runtime.is_ready())
            .map(LlamaCppEmbeddingRuntime::base_url)
    }

    pub fn is_ready(&self) -> bool {
        self.runtime
            .as_ref()
            .map(LlamaCppEmbeddingRuntime::is_ready)
            .unwrap_or(false)
    }

    pub fn runtime_lifecycle_snapshot(&self) -> Option<RuntimeLifecycleSnapshot> {
        self.runtime
            .as_ref()
            .map(LlamaCppEmbeddingRuntime::runtime_lifecycle_snapshot)
    }

    pub fn model_target(&self) -> Option<String> {
        self.runtime
            .as_ref()
            .and_then(LlamaCppEmbeddingRuntime::model_target)
    }

    #[doc(hidden)]
    pub fn set_test_runtime(&mut self, runtime: LlamaCppEmbeddingRuntime) {
        self.runtime = Some(runtime);
    }
}

impl Drop for LlamaCppEmbeddingRuntime {
    fn drop(&mut self) {
        self.stop();
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    struct MockProcessHandle;
    struct MockProcessSpawner;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            7
        }

        fn kill(&self) -> Result<(), String> {
            Ok(())
        }
    }

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

    #[test]
    fn test_check_vram_available() {
        let devices_with_vram = vec![DeviceInfo {
            id: "Vulkan0".to_string(),
            name: "Test GPU".to_string(),
            total_vram_mb: 8000,
            free_vram_mb: 4000,
        }];
        assert!(LlamaCppEmbeddingRuntime::check_vram_available(
            &devices_with_vram
        ));

        let devices_low_vram = vec![DeviceInfo {
            id: "Vulkan0".to_string(),
            name: "Test GPU".to_string(),
            total_vram_mb: 8000,
            free_vram_mb: 500,
        }];
        assert!(!LlamaCppEmbeddingRuntime::check_vram_available(
            &devices_low_vram
        ));

        let devices_cpu_only = vec![DeviceInfo {
            id: "none".to_string(),
            name: "CPU".to_string(),
            total_vram_mb: 0,
            free_vram_mb: 0,
        }];
        assert!(!LlamaCppEmbeddingRuntime::check_vram_available(
            &devices_cpu_only
        ));
    }

    #[test]
    fn test_base_url() {
        let runtime = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        assert_eq!(runtime.base_url(), "http://127.0.0.1:8081");
    }

    #[test]
    fn test_runtime_lifecycle_snapshot_tracks_start_success() {
        let mut runtime = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        runtime.mark_start_success(100);

        let snapshot = runtime.runtime_lifecycle_snapshot();
        assert_eq!(snapshot.runtime_id.as_deref(), Some(EMBEDDING_RUNTIME_ID));
        assert_eq!(
            snapshot.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-1")
        );
        assert_eq!(snapshot.runtime_reused, Some(false));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
        assert!(snapshot.active);
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn test_runtime_lifecycle_snapshot_tracks_start_failure() {
        let mut runtime = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        runtime.mark_start_failure(200, "boom".to_string());

        let snapshot = runtime.runtime_lifecycle_snapshot();
        assert_eq!(snapshot.runtime_id.as_deref(), Some(EMBEDDING_RUNTIME_ID));
        assert_eq!(snapshot.runtime_reused, Some(false));
        assert_eq!(
            snapshot.lifecycle_decision_reason.as_deref(),
            Some("runtime_start_failed")
        );
        assert!(!snapshot.active);
        assert_eq!(snapshot.last_error.as_deref(), Some("boom"));
    }

    #[test]
    fn test_matches_runtime_requires_ready_model_and_mode() {
        let mut runtime = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        runtime.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");

        assert!(runtime.matches_runtime("/models/embed.gguf", EmbeddingMemoryMode::CpuParallel));
        assert!(!runtime.matches_runtime("/models/other.gguf", EmbeddingMemoryMode::CpuParallel));
        assert!(!runtime.matches_runtime("/models/embed.gguf", EmbeddingMemoryMode::GpuParallel));
    }

    #[tokio::test]
    async fn dedicated_embedding_runtime_manager_reuses_matching_runtime() {
        let mut runtime = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
        runtime.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");

        let mut manager = DedicatedEmbeddingRuntimeManager::new();
        manager.set_test_runtime(runtime);
        let spawner: Arc<dyn ProcessSpawner> = Arc::new(MockProcessSpawner);

        manager
            .ensure_runtime(
                "/models/embed.gguf",
                EmbeddingMemoryMode::CpuParallel,
                &spawner,
                &[],
            )
            .await
            .expect("matching runtime should be reused");

        let snapshot = manager
            .runtime_lifecycle_snapshot()
            .expect("snapshot should exist");
        assert_eq!(snapshot.runtime_reused, Some(true));
        assert!(snapshot.active);
    }
}
