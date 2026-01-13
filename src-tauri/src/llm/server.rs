use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;
use tokio::sync::RwLock;

use super::types::LLMStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMode {
    None,
    External { url: String },
    Sidecar { port: u16 },
}

pub struct LlamaServer {
    child: Option<CommandChild>,
    mode: ServerMode,
    ready: bool,
}

impl LlamaServer {
    pub fn new() -> Self {
        Self {
            child: None,
            mode: ServerMode::None,
            ready: false,
        }
    }

    pub async fn connect_external(&mut self, url: &str) -> Result<(), String> {
        // Stop any existing sidecar
        self.stop();

        // Validate the URL by making a simple request
        let client = reqwest::Client::new();
        let health_url = format!("{}/health", url.trim_end_matches('/'));

        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => {
                self.mode = ServerMode::External { url: url.to_string() };
                self.ready = true;
                Ok(())
            }
            Ok(resp) => {
                // Some servers don't have /health, try /v1/models instead
                let models_url = format!("{}/v1/models", url.trim_end_matches('/'));
                match client.get(&models_url).send().await {
                    Ok(resp2) if resp2.status().is_success() => {
                        self.mode = ServerMode::External { url: url.to_string() };
                        self.ready = true;
                        Ok(())
                    }
                    _ => Err(format!("Server responded with status: {}", resp.status())),
                }
            }
            Err(e) => Err(format!("Failed to connect to server at {}: {}", url, e)),
        }
    }

    pub async fn start_sidecar(
        &mut self,
        app: &AppHandle,
        model_path: &str,
        mmproj_path: &str,
    ) -> Result<(), String> {
        // Stop any existing connection
        self.stop();

        let port: u16 = 8080;

        let sidecar = app
            .shell()
            .sidecar("llama-server")
            .map_err(|e| format!("Failed to create sidecar: {}", e))?
            .args([
                "-m",
                model_path,
                "--mmproj",
                mmproj_path,
                "--port",
                &port.to_string(),
                "-c",
                "8192",
                "--host",
                "127.0.0.1",
                "--jinja",
            ]);

        let (mut rx, child) = sidecar
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;

        self.child = Some(child);
        self.mode = ServerMode::Sidecar { port };

        // Wait for server ready signal with timeout
        let timeout = tokio::time::Duration::from_secs(120);
        let start = std::time::Instant::now();

        use tauri_plugin_shell::process::CommandEvent;
        while let Some(event) = rx.recv().await {
            if start.elapsed() > timeout {
                self.stop();
                return Err("Timeout waiting for llama-server to start".to_string());
            }

            match event {
                CommandEvent::Stdout(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    println!("[llama-server] {}", line_str);
                    if line_str.contains("server listening")
                        || line_str.contains("HTTP server listening")
                    {
                        self.ready = true;
                        break;
                    }
                }
                CommandEvent::Stderr(line) => {
                    let line_str = String::from_utf8_lossy(&line);
                    eprintln!("[llama-server stderr] {}", line_str);
                    if line_str.contains("server listening")
                        || line_str.contains("HTTP server listening")
                    {
                        self.ready = true;
                        break;
                    }
                }
                CommandEvent::Error(err) => {
                    self.stop();
                    return Err(format!("llama-server error: {}", err));
                }
                CommandEvent::Terminated(status) => {
                    self.stop();
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
            ServerMode::External { url } => Some(url.trim_end_matches('/').to_string()),
            ServerMode::Sidecar { port } => Some(format!("http://127.0.0.1:{}", port)),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn status(&self) -> LLMStatus {
        LLMStatus {
            ready: self.ready,
            mode: match &self.mode {
                ServerMode::None => "none".to_string(),
                ServerMode::External { .. } => "external".to_string(),
                ServerMode::Sidecar { .. } => "sidecar".to_string(),
            },
            url: self.base_url(),
        }
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            let _ = child.kill();
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
