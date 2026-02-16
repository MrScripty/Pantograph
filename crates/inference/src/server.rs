//! LlamaServer - sidecar process management for llama.cpp inference
//!
//! This module manages the lifecycle of llama-server processes, including:
//! - Starting servers in inference or embedding mode
//! - Monitoring process health
//! - Graceful shutdown

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::DeviceConfig;
use crate::constants::{defaults, device_types, hosts, ports, timeouts};
use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use crate::types::LLMStatus;

const SIDECAR_PID_FILE: &str = "llama-server.pid";

fn is_oom_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if lower.contains("out of memory") {
        return true;
    }
    if lower.contains("outofdevicememory") || lower.contains("erroroutofdevicememory") {
        return true;
    }
    if lower.contains("device memory allocation") {
        return true;
    }
    if lower.contains("failed to allocate")
        && (lower.contains("vulkan") || lower.contains("cuda"))
    {
        return true;
    }
    if lower.contains("ggml_gallocr") && lower.contains("failed to allocate") {
        return true;
    }
    if lower.contains("graph_reserve: failed to allocate") {
        return true;
    }
    false
}

fn oom_error_message(hint: Option<&str>) -> String {
    match hint {
        Some(line) if !line.is_empty() => format!("Out of GPU memory (OOM): {}", line),
        _ => "Out of GPU memory (OOM).".to_string(),
    }
}

/// Server operating mode
#[derive(Debug, Clone, PartialEq)]
pub enum ServerMode {
    /// No server running
    None,
    /// Connected to external server (remote API or local server like LM Studio)
    External { url: String },
    /// Sidecar running in inference mode (text LLM or VLM with optional vision)
    SidecarInference {
        port: u16,
        model_path: String,
        mmproj_path: Option<String>,
    },
    /// Sidecar running in embedding mode (for RAG indexing)
    SidecarEmbedding { port: u16, model_path: String },
}

/// Information about current server mode for frontend
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerModeInfo {
    /// Current mode type
    pub mode: String,
    /// Whether the server is ready
    pub ready: bool,
    /// URL if connected to external server
    pub url: Option<String>,
    /// Model path if using sidecar
    pub model_path: Option<String>,
    /// Whether in embedding mode (sidecar only)
    pub is_embedding_mode: bool,
}

/// Manages llama-server sidecar processes
pub struct LlamaServer {
    child: Option<Box<dyn ProcessHandle>>,
    mode: ServerMode,
    ready: bool,
    pid_file: Option<PathBuf>,
}

impl LlamaServer {
    pub fn new() -> Self {
        Self {
            child: None,
            mode: ServerMode::None,
            ready: false,
            pid_file: None,
        }
    }

    /// Cleanup any stale sidecar processes from previous runs
    pub fn cleanup_stale_sidecar(app_data_dir: &PathBuf) -> Result<(), String> {
        let pid_path = app_data_dir.join(SIDECAR_PID_FILE);
        if !pid_path.exists() {
            return Ok(());
        }

        let pid_raw = fs::read_to_string(&pid_path)
            .map_err(|e| format!("Failed to read sidecar pid file: {}", e))?;
        let pid_str = pid_raw.trim();
        let pid: i32 = match pid_str.parse() {
            Ok(pid) => pid,
            Err(_) => {
                let _ = fs::remove_file(&pid_path);
                return Ok(());
            }
        };

        #[cfg(unix)]
        {
            use std::process::Command;
            use std::thread;
            use std::time::Duration;

            let pid_arg = pid.to_string();
            let is_running = Command::new("kill")
                .arg("-0")
                .arg(&pid_arg)
                .status()
                .map(|status| status.success())
                .unwrap_or(false);

            if is_running {
                let _ = Command::new("kill").arg("-TERM").arg(&pid_arg).status();
                thread::sleep(Duration::from_millis(200));
                let still_running = Command::new("kill")
                    .arg("-0")
                    .arg(&pid_arg)
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false);
                if still_running {
                    let _ = Command::new("kill").arg("-KILL").arg(&pid_arg).status();
                }
            }
        }

        let _ = fs::remove_file(&pid_path);
        Ok(())
    }

    /// Connect to an external server
    pub async fn connect_external(&mut self, url: &str) -> Result<(), String> {
        // Stop any existing sidecar
        self.stop();

        // Validate the URL by making a simple request
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", url.trim_end_matches('/'));

        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                self.mode = ServerMode::External {
                    url: url.to_string(),
                };
                self.ready = true;
                Ok(())
            }
            Ok(resp) => {
                // Some servers don't have /health, try /v1/models instead
                let models_url = format!("{}/v1/models", url.trim_end_matches('/'));
                match client.get(&models_url).send().await {
                    Ok(resp2) if resp2.status().is_success() => {
                        self.mode = ServerMode::External {
                            url: url.to_string(),
                        };
                        self.ready = true;
                        Ok(())
                    }
                    _ => Err(format!("Server responded with status: {}", resp.status())),
                }
            }
            Err(e) => Err(format!("Failed to connect to server at {}: {}", url, e)),
        }
    }

    /// Start sidecar in inference mode (text LLM or VLM with optional vision)
    ///
    /// If `mmproj_path` is `Some`, the `--mmproj` flag is passed to enable
    /// vision/multimodal support. For text-only LLMs, pass `None`.
    pub async fn start_sidecar_inference(
        &mut self,
        spawner: Arc<dyn ProcessSpawner>,
        model_path: &str,
        mmproj_path: Option<&str>,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = ports::SERVER;

        // Build arguments with device configuration
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();
        let context_size_str = defaults::CONTEXT_SIZE.to_string();

        let app_data_dir = spawner.app_data_dir()?;
        let pid_file = app_data_dir.join(SIDECAR_PID_FILE);
        let pid_file_str = pid_file.to_string_lossy().to_string();

        // Build base args
        let mut args: Vec<String> = vec![
            "-m".to_string(),
            model_path.to_string(),
            "--port".to_string(),
            port_str,
            "-c".to_string(),
            context_size_str,
            "--host".to_string(),
            hosts::LOCAL.to_string(),
            "--jinja".to_string(),
            "-ngl".to_string(),
            gpu_layers_str,
            "--pid-file".to_string(),
            pid_file_str,
        ];

        // Add mmproj only for vision/multimodal models
        if let Some(mmproj) = mmproj_path {
            args.push("--mmproj".to_string());
            args.push(mmproj.to_string());
        }

        // Add device selection if not "auto"
        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting llama-server with device config: device={}, gpu_layers={}",
            device.device,
            device.gpu_layers
        );

        // Convert to &str for spawner
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let (rx, child) = spawner
            .spawn_sidecar("llama-server-wrapper", &args_refs)
            .await?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.mode = ServerMode::SidecarInference {
            port,
            model_path: model_path.to_string(),
            mmproj_path: mmproj_path.map(|s| s.to_string()),
        };

        self.wait_for_ready(rx).await
    }

    /// Start sidecar in embedding mode (for RAG indexing)
    pub async fn start_sidecar_embedding(
        &mut self,
        spawner: Arc<dyn ProcessSpawner>,
        model_path: &str,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = ports::SERVER;

        // Build arguments with device configuration
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();

        let app_data_dir = spawner.app_data_dir()?;
        let pid_file = app_data_dir.join(SIDECAR_PID_FILE);
        let pid_file_str = pid_file.to_string_lossy().to_string();

        // Build args
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
            "--pid-file".to_string(),
            pid_file_str,
        ];

        // Add device selection if not "auto"
        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting embedding server with device config: device={}, gpu_layers={}",
            device.device,
            device.gpu_layers
        );

        // Convert to &str for spawner
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let (rx, child) = spawner
            .spawn_sidecar("llama-server-wrapper", &args_refs)
            .await?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.mode = ServerMode::SidecarEmbedding {
            port,
            model_path: model_path.to_string(),
        };

        self.wait_for_ready(rx).await
    }

    /// Verify the server is responding to HTTP requests
    async fn verify_http_ready(&self, timeout_ms: u64) -> Result<(), String> {
        let base_url = self
            .base_url()
            .ok_or("No base URL available for health check")?;
        let health_url = format!("{}/health", base_url);

        let start = std::time::Instant::now();
        let mut delay = 100;

        log::debug!("Starting HTTP health verification for {}", health_url);

        while start.elapsed().as_millis() < timeout_ms as u128 {
            match reqwest::get(&health_url).await {
                Ok(resp) if resp.status().is_success() => {
                    log::info!("HTTP health check passed");
                    return Ok(());
                }
                Ok(resp) => {
                    log::debug!(
                        "Health check returned status {}, retrying...",
                        resp.status()
                    );
                }
                Err(e) => {
                    log::debug!("Health check failed: {}, retrying...", e);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
            delay = (delay * 2).min(2000); // Exponential backoff, max 2s
        }

        Err("Server stdout reported ready but HTTP health check failed within timeout".to_string())
    }

    /// Wait for sidecar to become ready
    async fn wait_for_ready(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<ProcessEvent>,
    ) -> Result<(), String> {
        let timeout = tokio::time::Duration::from_secs(timeouts::SERVER_STARTUP_SECS);
        let start = std::time::Instant::now();
        let mut oom_hint: Option<String> = None;

        while let Some(event) = rx.recv().await {
            if start.elapsed() > timeout {
                self.stop();
                return Err("Timeout waiting for llama-server to start".to_string());
            }

            match event {
                ProcessEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    let line_trimmed = line_str.trim();
                    // Only log important messages, skip verbose metadata
                    if !line_str.contains("llama_model_loader: - kv")
                        && !line_str.contains("llama_model_loader: - type")
                    {
                        log::info!("[llama-server] {}", line_str);
                    }
                    if is_oom_line(line_trimmed) {
                        oom_hint = Some(line_trimmed.to_string());
                    }
                    if (line_str.contains("server") && line_str.contains("listening"))
                        || line_str.contains("HTTP server listening")
                    {
                        log::debug!("Stdout reports server listening, verifying HTTP...");
                        // Verify HTTP is actually responding
                        match self.verify_http_ready(5000).await {
                            Ok(_) => {
                                self.ready = true;
                                break;
                            }
                            Err(e) => {
                                self.stop();
                                return Err(e);
                            }
                        }
                    }
                }
                ProcessEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    let line_trimmed = line_str.trim();
                    // Only log important messages, skip verbose metadata
                    if !line_str.contains("llama_model_loader: - kv")
                        && !line_str.contains("llama_model_loader: - type")
                    {
                        log::warn!("[llama-server stderr] {}", line_str);
                    }
                    if is_oom_line(line_trimmed) {
                        oom_hint = Some(line_trimmed.to_string());
                    }
                    if (line_str.contains("server") && line_str.contains("listening"))
                        || line_str.contains("HTTP server listening")
                    {
                        log::debug!("Stderr reports server listening, verifying HTTP...");
                        // Verify HTTP is actually responding
                        match self.verify_http_ready(5000).await {
                            Ok(_) => {
                                self.ready = true;
                                break;
                            }
                            Err(e) => {
                                self.stop();
                                return Err(e);
                            }
                        }
                    }
                }
                ProcessEvent::Error(err) => {
                    self.stop();
                    if oom_hint.is_some() {
                        return Err(oom_error_message(oom_hint.as_deref()));
                    }
                    return Err(format!("llama-server error: {}", err));
                }
                ProcessEvent::Terminated(status) => {
                    self.stop();
                    if oom_hint.is_some() {
                        return Err(oom_error_message(oom_hint.as_deref()));
                    }
                    return Err(format!(
                        "llama-server terminated unexpectedly: {:?}",
                        status
                    ));
                }
            }
        }

        if !self.ready {
            self.stop();
            return Err("llama-server failed to become ready".to_string());
        }

        Ok(())
    }

    pub fn base_url(&self) -> Option<String> {
        match &self.mode {
            ServerMode::None => None,
            ServerMode::External { url } => Some(url.trim_end_matches('/').to_string()),
            ServerMode::SidecarInference { port, .. } => Some(format!("http://127.0.0.1:{}", port)),
            ServerMode::SidecarEmbedding { port, .. } => Some(format!("http://127.0.0.1:{}", port)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Check if currently in external mode
    pub fn is_external(&self) -> bool {
        matches!(self.mode, ServerMode::External { .. })
    }

    /// Check if currently running a sidecar (either inference or embedding)
    pub fn is_sidecar(&self) -> bool {
        matches!(
            self.mode,
            ServerMode::SidecarInference { .. } | ServerMode::SidecarEmbedding { .. }
        )
    }

    /// Check if currently in embedding mode
    pub fn is_embedding_mode(&self) -> bool {
        matches!(self.mode, ServerMode::SidecarEmbedding { .. })
    }

    /// Check if currently in inference mode
    pub fn is_inference_mode(&self) -> bool {
        matches!(self.mode, ServerMode::SidecarInference { .. })
    }

    /// Get the current mode
    pub fn current_mode(&self) -> &ServerMode {
        &self.mode
    }

    pub fn status(&self) -> LLMStatus {
        LLMStatus {
            ready: self.ready,
            mode: match &self.mode {
                ServerMode::None => "none".to_string(),
                ServerMode::External { .. } => "external".to_string(),
                ServerMode::SidecarInference { .. } => "sidecar_inference".to_string(),
                ServerMode::SidecarEmbedding { .. } => "sidecar_embedding".to_string(),
            },
            url: self.base_url(),
        }
    }

    /// Get detailed server mode info for frontend
    pub fn mode_info(&self) -> ServerModeInfo {
        ServerModeInfo {
            mode: match &self.mode {
                ServerMode::None => "none".to_string(),
                ServerMode::External { .. } => "external".to_string(),
                ServerMode::SidecarInference { .. } => "sidecar_inference".to_string(),
                ServerMode::SidecarEmbedding { .. } => "sidecar_embedding".to_string(),
            },
            ready: self.ready,
            url: self.base_url(),
            model_path: match &self.mode {
                ServerMode::SidecarInference { model_path, .. } => Some(model_path.clone()),
                ServerMode::SidecarEmbedding { model_path, .. } => Some(model_path.clone()),
                _ => None,
            },
            is_embedding_mode: self.is_embedding_mode(),
        }
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            let pid = child.pid();
            log::debug!("Stopping llama-server (PID: {})", pid);

            // Kill the process
            if let Err(e) = child.kill() {
                log::warn!("Failed to kill llama-server: {}", e);
            }

            #[cfg(unix)]
            {
                use std::process::Command;
                use std::thread;
                use std::time::Duration;

                // Wait a bit for graceful exit
                thread::sleep(Duration::from_millis(500));

                // Check if still running and force kill if needed
                let pid_arg = pid.to_string();
                let still_running = Command::new("kill")
                    .arg("-0")
                    .arg(&pid_arg)
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false);

                if still_running {
                    log::warn!("Process didn't exit gracefully, forcing kill");
                    let _ = Command::new("kill").arg("-KILL").arg(&pid_arg).status();
                    thread::sleep(Duration::from_millis(200));
                }

                log::debug!("llama-server stopped");
            }
        }

        // Clean up PID file
        if let Some(pid_file) = self.pid_file.take() {
            let _ = fs::remove_file(pid_file);
        }

        self.mode = ServerMode::None;
        self.ready = false;
    }
}

impl Default for LlamaServer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LlamaServer {
    fn drop(&mut self) {
        self.stop();
    }
}

pub type SharedLlamaServer = Arc<RwLock<LlamaServer>>;
