//! LlamaServer - sidecar process management for llama.cpp inference
//!
//! This module manages the lifecycle of llama-server processes, including:
//! - Starting servers in inference or embedding mode
//! - Monitoring process health
//! - Graceful shutdown

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sysinfo::{Pid, ProcessesToUpdate, Signal, System};
use tokio::sync::RwLock;

use crate::config::DeviceConfig;
use crate::constants::{defaults, device_types, hosts, ports, timeouts};
use crate::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use crate::types::ServerModeInfo;

const SIDECAR_PID_FILE: &str = "llama-server.pid";
const KV_SLOT_SAVE_DIR: &str = "llama-kv-slots";

#[derive(Debug, Deserialize)]
struct SidecarPidRecord {
    pid: i32,
}

fn normalize_server_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

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
    if lower.contains("failed to allocate") && (lower.contains("vulkan") || lower.contains("cuda"))
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

fn kv_slot_save_dir(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join(KV_SLOT_SAVE_DIR)
}

fn parse_sidecar_pid(raw: &str) -> Option<i32> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(pid) = value.parse::<i32>() {
        return Some(pid);
    }

    serde_json::from_str::<SidecarPidRecord>(value)
        .ok()
        .map(|record| record.pid)
}

#[derive(Debug, Clone, Copy)]
enum SlotAction {
    Save,
    Restore,
    Erase,
}

impl SlotAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Save => "save",
            Self::Restore => "restore",
            Self::Erase => "erase",
        }
    }
}

fn terminate_pid(pid: i32) -> bool {
    if pid <= 0 {
        return false;
    }

    let mut system = System::new();
    let target_pid = Pid::from_u32(pid as u32);
    system.refresh_processes(ProcessesToUpdate::Some(&[target_pid]), true);

    let Some(process) = system.process(target_pid) else {
        return false;
    };

    if process.kill_with(Signal::Term).unwrap_or(false) {
        return true;
    }

    process.kill()
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
        device: DeviceConfig,
    },
    /// Sidecar running in embedding mode (for RAG indexing)
    SidecarEmbedding {
        port: u16,
        model_path: String,
        device: DeviceConfig,
    },
    /// Sidecar running in reranking mode
    SidecarReranking {
        port: u16,
        model_path: String,
        device: DeviceConfig,
    },
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
    pub fn cleanup_stale_sidecar(app_data_dir: &Path) -> Result<(), String> {
        let pid_path = app_data_dir.join(SIDECAR_PID_FILE);
        if !pid_path.exists() {
            return Ok(());
        }

        let pid_raw = fs::read_to_string(&pid_path)
            .map_err(|e| format!("Failed to read sidecar pid file: {}", e))?;
        let pid = match parse_sidecar_pid(&pid_raw) {
            Some(pid) => pid,
            None => {
                let _ = fs::remove_file(&pid_path);
                return Ok(());
            }
        };

        if terminate_pid(pid) {
            log::info!("Terminated stale sidecar process (PID: {})", pid);
        } else {
            log::debug!("No running stale sidecar process found for PID {}", pid);
        }

        let _ = fs::remove_file(&pid_path);
        Ok(())
    }

    /// Connect to an external server
    pub async fn connect_external(&mut self, url: &str) -> Result<(), String> {
        // Stop any existing sidecar
        self.stop();
        let normalized_url = normalize_server_url(url);

        // Validate the URL by making a simple request
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", normalized_url);

        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                self.mode = ServerMode::External {
                    url: normalized_url,
                };
                self.ready = true;
                Ok(())
            }
            Ok(resp) => {
                // Some servers don't have /health, try /v1/models instead
                let models_url = format!("{}/v1/models", normalize_server_url(url));
                match client.get(&models_url).send().await {
                    Ok(resp2) if resp2.status().is_success() => {
                        self.mode = ServerMode::External {
                            url: normalize_server_url(url),
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
        port_override: Option<u16>,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = port_override.unwrap_or(ports::SERVER);

        // Build arguments with device configuration
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();
        let context_size_str = defaults::CONTEXT_SIZE.to_string();

        let app_data_dir = spawner.app_data_dir()?;
        let pid_file = app_data_dir.join(SIDECAR_PID_FILE);
        let pid_file_str = pid_file.to_string_lossy().to_string();
        let slot_save_dir = kv_slot_save_dir(&app_data_dir);
        fs::create_dir_all(&slot_save_dir)
            .map_err(|e| format!("Failed to create llama.cpp KV slot directory: {}", e))?;
        let slot_save_dir_str = slot_save_dir.to_string_lossy().to_string();

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
            "--slots".to_string(),
            "--slot-save-path".to_string(),
            slot_save_dir_str,
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
            device: device.clone(),
        };

        self.wait_for_ready(rx).await
    }

    /// Start sidecar in embedding mode (for RAG indexing)
    pub async fn start_sidecar_embedding(
        &mut self,
        spawner: Arc<dyn ProcessSpawner>,
        model_path: &str,
        device: &DeviceConfig,
        port_override: Option<u16>,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = port_override.unwrap_or(ports::SERVER);

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
            device: device.clone(),
        };

        self.wait_for_ready(rx).await
    }

    /// Start sidecar in reranking mode.
    pub async fn start_sidecar_reranking(
        &mut self,
        spawner: Arc<dyn ProcessSpawner>,
        model_path: &str,
        device: &DeviceConfig,
        port_override: Option<u16>,
    ) -> Result<(), String> {
        self.stop();

        let port = port_override.unwrap_or(ports::SERVER);
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();

        let app_data_dir = spawner.app_data_dir()?;
        let pid_file = app_data_dir.join(SIDECAR_PID_FILE);
        let pid_file_str = pid_file.to_string_lossy().to_string();

        let mut args: Vec<String> = vec![
            "-m".to_string(),
            model_path.to_string(),
            "--port".to_string(),
            port_str,
            "--host".to_string(),
            hosts::LOCAL.to_string(),
            "--reranking".to_string(),
            "-ngl".to_string(),
            gpu_layers_str,
            "--pid-file".to_string(),
            pid_file_str,
        ];

        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting reranking server with device config: device={}, gpu_layers={}",
            device.device,
            device.gpu_layers
        );

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (rx, child) = spawner
            .spawn_sidecar("llama-server-wrapper", &args_refs)
            .await?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.mode = ServerMode::SidecarReranking {
            port,
            model_path: model_path.to_string(),
            device: device.clone(),
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
            ServerMode::SidecarReranking { port, .. } => Some(format!("http://127.0.0.1:{}", port)),
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
            ServerMode::SidecarInference { .. }
                | ServerMode::SidecarEmbedding { .. }
                | ServerMode::SidecarReranking { .. }
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

    /// Check if currently in reranking mode
    pub fn is_reranking_mode(&self) -> bool {
        matches!(self.mode, ServerMode::SidecarReranking { .. })
    }

    pub fn matches_inference_runtime(
        &self,
        model_path: &str,
        mmproj_path: Option<&str>,
        device: &DeviceConfig,
        port_override: Option<u16>,
    ) -> bool {
        let expected_port = port_override.unwrap_or(ports::SERVER);
        self.ready
            && matches!(
                &self.mode,
                ServerMode::SidecarInference {
                    port: active_port,
                    model_path: active_model_path,
                    mmproj_path: active_mmproj_path,
                    device: active_device,
                    ..
                } if active_model_path == model_path
                    && active_mmproj_path.as_deref() == mmproj_path
                    && active_device == device
                    && *active_port == expected_port
            )
    }

    pub fn matches_embedding_runtime(
        &self,
        model_path: &str,
        device: &DeviceConfig,
        port_override: Option<u16>,
    ) -> bool {
        let expected_port = port_override.unwrap_or(ports::SERVER);
        self.ready
            && matches!(
                &self.mode,
                ServerMode::SidecarEmbedding {
                    port: active_port,
                    model_path: active_model_path,
                    device: active_device,
                    ..
                } if active_model_path == model_path
                    && active_device == device
                    && *active_port == expected_port
            )
    }

    pub fn matches_reranking_runtime(
        &self,
        model_path: &str,
        device: &DeviceConfig,
        port_override: Option<u16>,
    ) -> bool {
        let expected_port = port_override.unwrap_or(ports::SERVER);
        self.ready
            && matches!(
                &self.mode,
                ServerMode::SidecarReranking {
                    port: active_port,
                    model_path: active_model_path,
                    device: active_device,
                    ..
                } if active_model_path == model_path
                    && active_device == device
                    && *active_port == expected_port
            )
    }

    pub fn matches_external_runtime(&self, url: &str) -> bool {
        let normalized_url = normalize_server_url(url);
        self.ready
            && matches!(
                &self.mode,
                ServerMode::External { url: active_url } if active_url == &normalized_url
            )
    }

    /// Get the current mode
    pub fn current_mode(&self) -> &ServerMode {
        &self.mode
    }

    async fn execute_slot_action(
        &self,
        slot_id: u32,
        action: SlotAction,
        filename: Option<&str>,
    ) -> Result<(), String> {
        if !self.ready {
            return Err("llama-server is not ready".to_string());
        }

        let base_url = self
            .base_url()
            .ok_or_else(|| "llama-server has no base URL".to_string())?;
        let client = reqwest::Client::new();
        let url = format!("{}/slots/{}?action={}", base_url, slot_id, action.as_str());

        let request = if let Some(filename) = filename {
            client
                .post(&url)
                .json(&serde_json::json!({ "filename": filename }))
        } else {
            client.post(&url)
        };

        let response = request
            .send()
            .await
            .map_err(|e| format!("llama.cpp slot {} request failed: {}", action.as_str(), e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!(
                "llama.cpp slot {} failed with status {}: {}",
                action.as_str(),
                status,
                body
            ));
        }

        Ok(())
    }

    pub async fn save_slot(&self, slot_id: u32, filename: &str) -> Result<(), String> {
        self.execute_slot_action(slot_id, SlotAction::Save, Some(filename))
            .await
    }

    pub async fn restore_slot(&self, slot_id: u32, filename: &str) -> Result<(), String> {
        self.execute_slot_action(slot_id, SlotAction::Restore, Some(filename))
            .await
    }

    pub async fn erase_slot(&self, slot_id: u32) -> Result<(), String> {
        self.execute_slot_action(slot_id, SlotAction::Erase, None)
            .await
    }

    #[cfg(test)]
    pub(crate) fn set_test_runtime_state(&mut self, mode: ServerMode, ready: bool) {
        self.mode = mode;
        self.ready = ready;
    }

    /// Get detailed server mode info for frontend
    pub fn mode_info(&self) -> ServerModeInfo {
        ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: match &self.mode {
                ServerMode::None => "none".to_string(),
                ServerMode::External { .. } => "external".to_string(),
                ServerMode::SidecarInference { .. } => "sidecar_inference".to_string(),
                ServerMode::SidecarEmbedding { .. } => "sidecar_embedding".to_string(),
                ServerMode::SidecarReranking { .. } => "sidecar_reranking".to_string(),
            },
            ready: self.ready,
            url: self.base_url(),
            model_path: match &self.mode {
                ServerMode::SidecarInference { model_path, .. } => Some(model_path.clone()),
                ServerMode::SidecarEmbedding { model_path, .. } => Some(model_path.clone()),
                ServerMode::SidecarReranking { model_path, .. } => Some(model_path.clone()),
                _ => None,
            },
            is_embedding_mode: self.is_embedding_mode(),
            active_model_target: None,
            embedding_model_target: None,
            active_runtime: None,
            embedding_runtime: None,
        }
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            let pid = child.pid();
            log::debug!("Stopping llama-server (PID: {})", pid);

            if let Err(e) = child.kill() {
                log::warn!("Failed to kill llama-server: {}", e);
            }
            log::debug!("llama-server stop signal sent");
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

#[cfg(test)]
#[path = "server_tests.rs"]
mod tests;
