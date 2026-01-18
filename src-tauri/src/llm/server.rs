use std::fs;
use std::path::PathBuf;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

use crate::config::DeviceConfig;
use crate::constants::{defaults, device_types, hosts, ports, timeouts};

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
    /// Sidecar running in inference mode (VLM with vision support)
    SidecarInference {
        port: u16,
        model_path: String,
        mmproj_path: String,
    },
    /// Sidecar running in embedding mode (for RAG indexing)
    SidecarEmbedding {
        port: u16,
        model_path: String,
    },
}

pub struct LlamaServer {
    child: Option<CommandChild>,
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

    /// Start sidecar in inference mode (VLM with vision support)
    pub async fn start_sidecar_inference(
        &mut self,
        app: &AppHandle,
        model_path: &str,
        mmproj_path: &str,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = ports::SERVER;

        // Build arguments with device configuration
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();
        let context_size_str = defaults::CONTEXT_SIZE.to_string();

        // Build base args
        let mut args: Vec<String> = vec![
            "-m".to_string(), model_path.to_string(),
            "--mmproj".to_string(), mmproj_path.to_string(),
            "--port".to_string(), port_str,
            "-c".to_string(), context_size_str,
            "--host".to_string(), hosts::LOCAL.to_string(),
            "--jinja".to_string(),
            "-ngl".to_string(), gpu_layers_str,
        ];

        let pid_file = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?
            .join(SIDECAR_PID_FILE);
        args.push("--pid-file".to_string());
        args.push(pid_file.to_string_lossy().to_string());

        // Add device selection if not "auto"
        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting llama-server with device config: device={}, gpu_layers={}",
            device.device, device.gpu_layers
        );

        // The wrapper script handles LD_LIBRARY_PATH
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let sidecar = app
            .shell()
            .sidecar("llama-server-wrapper")
            .map_err(|e| format!("Failed to create sidecar: {}", e))?
            .args(&args_refs);

        let (mut rx, child) = sidecar
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.mode = ServerMode::SidecarInference {
            port,
            model_path: model_path.to_string(),
            mmproj_path: mmproj_path.to_string(),
        };

        self.wait_for_ready(&mut rx).await
    }

    /// Start sidecar in embedding mode (for RAG indexing)
    pub async fn start_sidecar_embedding(
        &mut self,
        app: &AppHandle,
        model_path: &str,
        device: &DeviceConfig,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port = ports::SERVER;

        // Build arguments with device configuration
        let gpu_layers_str = device.gpu_layers.to_string();
        let port_str = port.to_string();

        // Build base args
        let mut args: Vec<String> = vec![
            "-m".to_string(), model_path.to_string(),
            "--port".to_string(), port_str,
            "--host".to_string(), hosts::LOCAL.to_string(),
            "--embedding".to_string(),
            "-ngl".to_string(), gpu_layers_str,
        ];

        let pid_file = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?
            .join(SIDECAR_PID_FILE);
        args.push("--pid-file".to_string());
        args.push(pid_file.to_string_lossy().to_string());

        // Add device selection if not "auto"
        if device.device != device_types::AUTO {
            args.push("--device".to_string());
            args.push(device.device.clone());
        }

        log::info!(
            "Starting embedding server with device config: device={}, gpu_layers={}",
            device.device, device.gpu_layers
        );

        // The wrapper script handles LD_LIBRARY_PATH
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let sidecar = app
            .shell()
            .sidecar("llama-server-wrapper")
            .map_err(|e| format!("Failed to create sidecar: {}", e))?
            .args(&args_refs);

        let (mut rx, child) = sidecar
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

        self.child = Some(child);
        self.pid_file = Some(pid_file);
        self.mode = ServerMode::SidecarEmbedding {
            port,
            model_path: model_path.to_string(),
        };

        self.wait_for_ready(&mut rx).await
    }

    /// Verify the server is responding to HTTP requests
    async fn verify_http_ready(&self, timeout_ms: u64) -> Result<(), String> {
        let base_url = self.base_url()
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
                    log::debug!("Health check returned status {}, retrying...", resp.status());
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
        rx: &mut tokio::sync::mpsc::Receiver<tauri_plugin_shell::process::CommandEvent>,
    ) -> Result<(), String> {
        use tauri_plugin_shell::process::CommandEvent;

        let timeout = tokio::time::Duration::from_secs(timeouts::SERVER_STARTUP_SECS);
        let start = std::time::Instant::now();
        let mut oom_hint: Option<String> = None;

        while let Some(event) = rx.recv().await {
            if start.elapsed() > timeout {
                self.stop();
                return Err("Timeout waiting for llama-server to start".to_string());
            }

            match event {
                CommandEvent::Stdout(line) => {
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
                CommandEvent::Stderr(line) => {
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
                CommandEvent::Error(err) => {
                    self.stop();
                    if oom_hint.is_some() {
                        return Err(oom_error_message(oom_hint.as_deref()));
                    }
                    return Err(format!("llama-server error: {}", err));
                }
                CommandEvent::Terminated(status) => {
                    self.stop();
                    if oom_hint.is_some() {
                        return Err(oom_error_message(oom_hint.as_deref()));
                    }
                    return Err(format!("llama-server terminated unexpectedly: {:?}", status));
                }
                _ => {}
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
            ServerMode::SidecarInference { port, .. } => Some(format!("http://127.0.0.1:{}", port)),
            ServerMode::SidecarEmbedding { port, .. } => Some(format!("http://127.0.0.1:{}", port)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            // Get PID for graceful shutdown
            let pid = child.pid();

            #[cfg(unix)]
            {
                log::debug!("Stopping llama-server (PID: {})", pid);

                // 1. Try SIGTERM for graceful shutdown
                let pid_arg = pid.to_string();
                let _ = Command::new("kill")
                    .arg("-TERM")
                    .arg(&pid_arg)
                    .status();

                // 2. Wait for graceful exit (500ms)
                thread::sleep(Duration::from_millis(500));

                // 3. Check if still running
                let still_running = Command::new("kill")
                    .arg("-0")
                    .arg(&pid_arg)
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false);

                if still_running {
                    log::warn!("Process didn't exit gracefully, forcing kill");
                    // 4. Force kill with SIGKILL
                    let _ = Command::new("kill")
                        .arg("-KILL")
                        .arg(&pid_arg)
                        .status();

                    // 5. Final wait
                    thread::sleep(Duration::from_millis(200));
                }

                log::debug!("llama-server stopped");
            }

            #[cfg(not(unix))]
            {
                // Windows: use child.kill()
                let _ = child.kill();
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
