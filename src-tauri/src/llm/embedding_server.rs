//! Dedicated embedding server manager
//!
//! Manages a separate llama.cpp server instance for embedding operations,
//! allowing it to run in parallel with the main LLM server.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use inference::RuntimeLifecycleSnapshot;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};

use crate::config::{DeviceConfig, DeviceInfo, EmbeddingMemoryMode};
use crate::constants::{device_types, hosts};

/// Default port for the embedding server (separate from main LLM on 8080)
const EMBEDDING_SERVER_PORT: u16 = 8081;

/// Minimum VRAM (MB) needed for embedding model (Qwen3-Embedding-0.6B ≈ 600MB)
const EMBEDDING_MODEL_VRAM_MB: u64 = 800;

/// PID file name for the embedding server
const EMBEDDING_PID_FILE: &str = "embedding-server.pid";
/// Canonical runtime identifier for the dedicated embedding sidecar.
const EMBEDDING_RUNTIME_ID: &str = "llama.cpp.embedding";

/// Dedicated embedding server that can run alongside the main LLM
pub struct EmbeddingServer {
    child: Option<Box<dyn ProcessHandle>>,
    port: u16,
    mode: EmbeddingMemoryMode,
    ready: bool,
    pid_file: Option<PathBuf>,
    model_path: Option<String>,
    runtime_lifecycle: RuntimeLifecycleSnapshot,
    runtime_instance_sequence: u64,
}

impl EmbeddingServer {
    /// Create a new embedding server manager
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

    /// Check if there's enough free VRAM for the embedding model
    pub fn check_vram_available(devices: &[DeviceInfo]) -> bool {
        // Find a GPU device (not "none"/CPU) with sufficient free VRAM
        devices
            .iter()
            .any(|device| device.id != "none" && device.free_vram_mb >= EMBEDDING_MODEL_VRAM_MB)
    }

    /// Start the embedding server based on memory mode
    ///
    /// # Arguments
    /// * `model_path` - Path to the embedding model GGUF file
    /// * `spawner` - Process spawner for launching sidecar
    /// * `devices` - Available device info (for VRAM checking in GpuParallel mode)
    ///
    /// # Returns
    /// * `Ok(())` if server started successfully
    /// * `Err` if failed (insufficient VRAM, spawn error, etc.)
    pub async fn start(
        &mut self,
        model_path: &str,
        spawner: &Arc<dyn ProcessSpawner>,
        devices: &[DeviceInfo],
    ) -> Result<(), String> {
        // Sequential mode doesn't use a dedicated server
        if self.mode == EmbeddingMemoryMode::Sequential {
            log::info!("Sequential mode: no dedicated embedding server needed");
            return Ok(());
        }

        let warmup_started_at_ms = unix_timestamp_ms();
        self.mark_start_attempt(warmup_started_at_ms);

        // Stop any existing server
        self.stop();

        // Configure device based on memory mode
        let device_config = match self.mode {
            EmbeddingMemoryMode::CpuParallel => {
                log::info!("Starting embedding server on CPU (RAM)");
                DeviceConfig {
                    device: "none".to_string(),
                    gpu_layers: 0,
                }
            }
            EmbeddingMemoryMode::GpuParallel => {
                // Check VRAM before attempting GPU load
                if !Self::check_vram_available(devices) {
                    return Err(format!(
                        "Insufficient VRAM for both models. Need at least {}MB free. Use 'CPU + GPU' mode instead.",
                        EMBEDDING_MODEL_VRAM_MB
                    ));
                }
                log::info!("Starting embedding server on GPU (VRAM)");
                DeviceConfig {
                    device: device_types::AUTO.to_string(),
                    gpu_layers: -1, // All layers on GPU
                }
            }
            EmbeddingMemoryMode::Sequential => {
                // Already handled above
                return Ok(());
            }
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

    /// Internal method to start the llama.cpp embedding server
    async fn start_server(
        &mut self,
        model_path: &str,
        spawner: &Arc<dyn ProcessSpawner>,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        let port_str = self.port.to_string();
        let gpu_layers_str = device.gpu_layers.to_string();

        // Build arguments
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

        // Add PID file
        let pid_file = spawner
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?
            .join(EMBEDDING_PID_FILE);
        args.push("--pid-file".to_string());
        args.push(pid_file.to_string_lossy().to_string());

        // Add device selection if not "auto"
        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting embedding server on port {} with device={}, gpu_layers={}",
            self.port,
            device.device,
            device.gpu_layers
        );

        // Spawn the sidecar via ProcessSpawner
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (mut rx, child) = spawner
            .spawn_sidecar("llama-server-wrapper", &args_refs)
            .await
            .map_err(|e| format!("Failed to spawn embedding server: {}", e))?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.model_path = Some(model_path.to_string());

        // Wait for server to be ready
        self.wait_for_ready(&mut rx).await?;

        self.ready = true;
        Ok(())
    }

    /// Check if a log line indicates the server is ready
    fn is_server_listening(line: &str) -> bool {
        (line.contains("server") && line.contains("listening"))
            || line.contains("HTTP server listening")
    }

    /// Verify the server is actually responding to HTTP requests
    async fn verify_http_ready(&self, timeout_ms: u64) -> Result<(), String> {
        let health_url = format!("{}/health", self.base_url());
        let start = std::time::Instant::now();

        while start.elapsed().as_millis() < timeout_ms as u128 {
            match reqwest::get(&health_url).await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!("Embedding server HTTP verified on port {}", self.port);
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }

        Err(format!(
            "Embedding server HTTP not responding after {}ms",
            timeout_ms
        ))
    }

    /// Wait for the server to signal it's ready
    async fn wait_for_ready(
        &self,
        rx: &mut tokio::sync::mpsc::Receiver<ProcessEvent>,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();
        let timeout_ms = 60000; // 60 second timeout

        while start.elapsed().as_millis() < timeout_ms {
            match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await {
                Ok(Some(event)) => {
                    match event {
                        ProcessEvent::Stdout(line) => {
                            let line_str = String::from_utf8_lossy(&line);
                            // Skip verbose model loading lines
                            if !line_str.contains("llama_model_loader: - kv")
                                && !line_str.contains("llama_model_loader: - type")
                            {
                                log::debug!("[embedding-server] {}", line_str);
                            }

                            // Check for ready signal (same pattern as main server)
                            if Self::is_server_listening(&line_str) {
                                log::debug!(
                                    "Stdout reports embedding server listening, verifying HTTP..."
                                );
                                return self.verify_http_ready(5000).await;
                            }
                        }
                        ProcessEvent::Stderr(line) => {
                            let line_str = String::from_utf8_lossy(&line);
                            // Skip verbose model loading lines
                            if !line_str.contains("llama_model_loader: - kv")
                                && !line_str.contains("llama_model_loader: - type")
                            {
                                log::debug!("[embedding-server stderr] {}", line_str);
                            }

                            // Check for OOM
                            if line_str.to_lowercase().contains("out of memory") {
                                return Err("Embedding server: Out of memory".to_string());
                            }

                            // Also check stderr for ready signal (llama.cpp may output there)
                            if Self::is_server_listening(&line_str) {
                                log::debug!(
                                    "Stderr reports embedding server listening, verifying HTTP..."
                                );
                                return self.verify_http_ready(5000).await;
                            }
                        }
                        ProcessEvent::Terminated(code) => {
                            return Err(format!(
                                "Embedding server terminated unexpectedly with code: {:?}",
                                code
                            ));
                        }
                        ProcessEvent::Error(err) => {
                            return Err(format!("Embedding server error: {}", err));
                        }
                    }
                }
                Ok(None) => {
                    return Err("Embedding server process ended without ready signal".to_string());
                }
                Err(_) => {
                    // Timeout on this iteration, continue waiting
                    continue;
                }
            }
        }

        Err(format!(
            "Embedding server failed to start within {} seconds",
            timeout_ms / 1000
        ))
    }

    /// Get the base URL of the embedding server
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", hosts::LOCAL, self.port)
    }

    /// Check if the server is ready
    pub fn is_ready(&self) -> bool {
        self.ready && self.child.is_some()
    }

    /// Return the backend-owned lifecycle snapshot for the dedicated embedding runtime.
    pub fn runtime_lifecycle_snapshot(&self) -> RuntimeLifecycleSnapshot {
        self.runtime_lifecycle.clone()
    }

    /// Return whether the active embedding runtime can satisfy the requested configuration.
    pub fn matches_runtime(&self, model_path: &str, mode: EmbeddingMemoryMode) -> bool {
        self.is_ready() && self.mode == mode && self.model_path.as_deref() == Some(model_path)
    }

    /// Mark the current embedding runtime as reused by a later request.
    pub fn mark_runtime_reused(&mut self) {
        self.runtime_lifecycle.runtime_reused = Some(true);
        self.runtime_lifecycle.active = self.is_ready();
        self.runtime_lifecycle.last_error = None;
    }

    #[cfg(test)]
    pub(crate) fn set_test_ready_state(&mut self, child: Box<dyn ProcessHandle>, model_path: &str) {
        self.child = Some(child);
        self.ready = true;
        self.model_path = Some(model_path.to_string());
    }

    /// Stop the embedding server
    pub fn stop(&mut self) {
        if let Some(ref child) = self.child {
            log::info!("Stopping embedding server");
            let _ = child.kill();
        }
        self.child = None;
        self.ready = false;
        self.model_path = None;
        self.runtime_lifecycle.active = false;

        // Clean up PID file
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
        self.runtime_lifecycle.active = false;
        self.runtime_lifecycle.last_error = None;
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
    }
}

impl Drop for EmbeddingServer {
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
    use inference::process::ProcessHandle;

    struct MockProcessHandle;

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            7
        }

        fn kill(&self) -> Result<(), String> {
            Ok(())
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
        assert!(EmbeddingServer::check_vram_available(&devices_with_vram));

        let devices_low_vram = vec![DeviceInfo {
            id: "Vulkan0".to_string(),
            name: "Test GPU".to_string(),
            total_vram_mb: 8000,
            free_vram_mb: 500, // Below threshold
        }];
        assert!(!EmbeddingServer::check_vram_available(&devices_low_vram));

        let devices_cpu_only = vec![DeviceInfo {
            id: "none".to_string(),
            name: "CPU".to_string(),
            total_vram_mb: 0,
            free_vram_mb: 0,
        }];
        assert!(!EmbeddingServer::check_vram_available(&devices_cpu_only));
    }

    #[test]
    fn test_base_url() {
        let server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);
        assert_eq!(server.base_url(), "http://127.0.0.1:8081");
    }

    #[test]
    fn test_runtime_lifecycle_snapshot_tracks_start_success() {
        let mut server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);

        server.mark_start_attempt(100);
        server.mark_start_success(100);

        let snapshot = server.runtime_lifecycle_snapshot();
        assert_eq!(snapshot.runtime_id.as_deref(), Some(EMBEDDING_RUNTIME_ID));
        assert_eq!(
            snapshot.runtime_instance_id.as_deref(),
            Some("llama-cpp-embedding-1")
        );
        assert_eq!(snapshot.warmup_started_at_ms, Some(100));
        assert!(snapshot.warmup_completed_at_ms.is_some());
        assert!(snapshot.warmup_duration_ms.is_some());
        assert_eq!(snapshot.runtime_reused, Some(false));
        assert!(snapshot.active);
        assert!(snapshot.last_error.is_none());
    }

    #[test]
    fn test_runtime_lifecycle_snapshot_tracks_start_failure() {
        let mut server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);

        server.mark_start_attempt(200);
        server.mark_start_failure(200, "spawn failed".to_string());

        let snapshot = server.runtime_lifecycle_snapshot();
        assert_eq!(snapshot.runtime_id.as_deref(), Some(EMBEDDING_RUNTIME_ID));
        assert_eq!(snapshot.warmup_started_at_ms, Some(200));
        assert!(snapshot.warmup_completed_at_ms.is_some());
        assert!(snapshot.warmup_duration_ms.is_some());
        assert_eq!(snapshot.runtime_reused, Some(false));
        assert!(!snapshot.active);
        assert_eq!(snapshot.last_error.as_deref(), Some("spawn failed"));
    }

    #[test]
    fn test_matches_runtime_requires_ready_model_and_mode() {
        let mut server = EmbeddingServer::new(EmbeddingMemoryMode::CpuParallel);
        server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");

        assert!(server.matches_runtime("/models/embed.gguf", EmbeddingMemoryMode::CpuParallel));
        assert!(!server.matches_runtime("/models/other.gguf", EmbeddingMemoryMode::CpuParallel));
        assert!(!server.matches_runtime("/models/embed.gguf", EmbeddingMemoryMode::GpuParallel));
    }
}
