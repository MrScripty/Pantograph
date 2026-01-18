//! Ollama backend implementation
//!
//! This backend connects to an Ollama daemon. If the daemon is not running,
//! it will automatically start it. Ollama exposes an OpenAI-compatible API
//! at `/v1/`, so we can forward requests directly without translation.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use super::{BackendCapabilities, BackendConfig, BackendError, InferenceBackend};

/// Default Ollama daemon URL
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";

/// Ollama model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
}

/// Ollama tags response (list of models)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

/// Ollama backend connecting to the Ollama daemon
///
/// Ollama can run as a system service or be started on-demand by this backend.
/// It exposes an OpenAI-compatible API at `/v1/` endpoints.
pub struct OllamaBackend {
    /// Base URL for Ollama daemon (default: http://localhost:11434)
    base_url: String,
    /// HTTP client for API requests
    http_client: reqwest::Client,
    /// Currently selected model name (e.g., "llava:13b")
    model_name: Option<String>,
    /// Whether the backend is ready (daemon is running and model is available)
    ready: bool,
    /// Daemon process if we started it (None if using external daemon)
    daemon_process: Option<Child>,
}

impl OllamaBackend {
    /// Create a new Ollama backend with default URL
    pub fn new() -> Self {
        Self {
            base_url: DEFAULT_OLLAMA_URL.to_string(),
            http_client: reqwest::Client::new(),
            model_name: None,
            ready: false,
            daemon_process: None,
        }
    }

    /// Check if Ollama is available for this backend
    ///
    /// Returns (available, reason_if_not) tuple.
    /// This is called by the registry to populate BackendInfo.available.
    pub fn check_availability() -> (bool, Option<String>) {
        // Check system PATH first
        if which::which("ollama").is_ok() {
            return (true, None);
        }

        // Check managed binaries directory
        if Self::find_managed_ollama().is_some() {
            return (true, None);
        }

        (
            false,
            Some("Ollama not installed".to_string()),
        )
    }

    /// Check if Ollama can be auto-installed on this platform
    ///
    /// Currently only Linux x86_64 is supported for auto-installation.
    pub fn can_auto_install() -> bool {
        cfg!(target_os = "linux") && cfg!(target_arch = "x86_64")
    }

    /// Find Ollama binary in our managed binaries directory
    fn find_managed_ollama() -> Option<PathBuf> {
        let mut candidates: Vec<PathBuf> = vec![];

        // App data directory (where downloads go to avoid triggering recompilation)
        // On Linux: ~/.local/share/com.pantograph.app/binaries/ollama
        #[cfg(target_os = "linux")]
        if let Some(data_dir) = dirs::data_dir() {
            candidates.push(data_dir.join("com.pantograph.app/binaries/ollama"));
        }

        // Dev mode: src-tauri/binaries
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("binaries/ollama"));
        }

        // Dev mode: exe in target/debug, binaries in src-tauri/binaries
        if let Ok(exe) = std::env::current_exe() {
            if let Some(p) = exe.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                candidates.push(p.join("binaries/ollama"));
            }
        }

        // Production: binaries next to exe
        if let Ok(exe) = std::env::current_exe() {
            if let Some(p) = exe.parent() {
                candidates.push(p.join("binaries/ollama"));
            }
        }

        candidates.into_iter().find(|p| p.exists())
    }

    /// Find the best Ollama binary to use (managed or system)
    fn find_ollama_binary() -> Option<PathBuf> {
        // Prefer our managed version
        if let Some(managed) = Self::find_managed_ollama() {
            return Some(managed);
        }

        // Fall back to system PATH
        which::which("ollama").ok()
    }

    /// Start the Ollama daemon if not already running
    async fn ensure_daemon_running(&mut self) -> Result<(), BackendError> {
        // First check if daemon is already running (external or from us)
        if self.check_daemon().await.is_ok() {
            log::info!("Ollama daemon already running");
            return Ok(());
        }

        // Find Ollama binary (prefer managed, then system PATH)
        let ollama_bin = Self::find_ollama_binary().ok_or_else(|| {
            BackendError::Config(
                "Ollama not found. Install from https://ollama.ai/download".to_string(),
            )
        })?;

        log::info!("Starting Ollama daemon from: {:?}", ollama_bin);

        // Try to start ollama serve
        let child = Command::new(&ollama_bin)
            .arg("serve")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                BackendError::StartupFailed(format!(
                    "Failed to start Ollama daemon from {:?}. Error: {}",
                    ollama_bin, e
                ))
            })?;

        self.daemon_process = Some(child);

        // Wait for daemon to be ready (poll with timeout)
        for i in 0..30 {
            // 30 second timeout
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            if self.check_daemon().await.is_ok() {
                log::info!("Ollama daemon started successfully after {}s", i + 1);
                return Ok(());
            }
        }

        // Failed to start - clean up
        if let Some(mut child) = self.daemon_process.take() {
            let _ = child.kill();
        }

        Err(BackendError::StartupFailed(
            "Ollama daemon failed to start within 30 seconds".to_string(),
        ))
    }

    /// Get static capabilities (for registry info before instantiation)
    pub fn static_capabilities() -> BackendCapabilities {
        BackendCapabilities {
            vision: true,            // llava and other vision models
            embeddings: true,        // nomic-embed-text and others
            gpu: true,               // Auto-managed by Ollama
            device_selection: false, // Ollama manages GPU automatically
            streaming: true,         // SSE streaming supported
            tool_calling: true,      // Via OpenAI-compatible API
        }
    }

    /// Check if Ollama daemon is running
    async fn check_daemon(&self) -> Result<(), BackendError> {
        let url = format!("{}/api/tags", self.base_url);

        self.http_client
            .get(&url)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|_| {
                BackendError::NotRunning(
                    "Ollama daemon not running. Start with: ollama serve".to_string(),
                )
            })?;

        Ok(())
    }

    /// List available models from Ollama
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, BackendError> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            return Err(BackendError::Inference(format!(
                "Failed to list models: {}",
                response.status()
            )));
        }

        let tags: OllamaTagsResponse = response
            .json()
            .await
            .map_err(|e| BackendError::Inference(format!("Failed to parse models: {}", e)))?;

        Ok(tags.models)
    }

    /// Check if a specific model is available
    async fn check_model_available(&self, model_name: &str) -> Result<bool, BackendError> {
        let models = self.list_models().await?;

        // Check for exact match or prefix match (e.g., "llava" matches "llava:13b")
        let available = models.iter().any(|m| {
            m.name == model_name || m.name.starts_with(&format!("{}:", model_name))
        });

        Ok(available)
    }

    /// Pull a model if not available (blocking operation)
    async fn ensure_model(&self, model_name: &str) -> Result<(), BackendError> {
        if self.check_model_available(model_name).await? {
            log::info!("Ollama model '{}' is available", model_name);
            return Ok(());
        }

        log::info!("Ollama model '{}' not found, attempting to pull...", model_name);

        let url = format!("{}/api/pull", self.base_url);
        let request = serde_json::json!({
            "name": model_name,
            "stream": false,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(600)) // 10 min timeout for large models
            .send()
            .await
            .map_err(BackendError::Http)?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(BackendError::StartupFailed(format!(
                "Failed to pull model '{}': {}",
                model_name, body
            )));
        }

        log::info!("Successfully pulled Ollama model '{}'", model_name);
        Ok(())
    }
}

impl Default for OllamaBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InferenceBackend for OllamaBackend {
    fn capabilities(&self) -> BackendCapabilities {
        Self::static_capabilities()
    }

    async fn start(&mut self, config: &BackendConfig, _app: &AppHandle) -> Result<(), BackendError> {
        // 1. Ensure Ollama daemon is running (start if needed)
        self.ensure_daemon_running().await?;

        // 2. Get model name from config, with smart defaults for embedding mode
        let model_name = if let Some(name) = config.model_name.as_ref() {
            name.clone()
        } else if config.embedding_mode {
            // Use a sensible default for embedding mode
            log::info!("No model_name specified for Ollama embedding mode, using default 'nomic-embed-text'");
            "nomic-embed-text".to_string()
        } else {
            return Err(BackendError::Config(
                "model_name required for Ollama inference (e.g., 'llava:13b', 'gemma3:12b'). \
                 Configure Ollama models in the backend settings.".to_string(),
            ));
        };

        // 3. Ensure model is available (pull if needed)
        self.ensure_model(&model_name).await?;

        // 4. Store model name and mark ready
        self.model_name = Some(model_name.clone());
        self.ready = true;

        log::info!("Ollama backend started with model: {}", model_name);
        Ok(())
    }

    fn stop(&mut self) {
        // Kill daemon if we started it
        if let Some(mut child) = self.daemon_process.take() {
            log::info!("Stopping Ollama daemon that we started...");
            let _ = child.kill();
            let _ = child.wait(); // Reap the process
        }

        self.model_name = None;
        self.ready = false;
        log::info!("Ollama backend stopped");
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    fn base_url(&self) -> Option<String> {
        if self.ready {
            Some(self.base_url.clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capabilities() {
        let caps = OllamaBackend::static_capabilities();
        assert!(caps.vision);
        assert!(caps.embeddings);
        assert!(caps.gpu);
        assert!(!caps.device_selection); // Ollama manages GPU automatically
        assert!(caps.streaming);
        assert!(caps.tool_calling);
    }

    #[test]
    fn test_not_ready_initially() {
        let backend = OllamaBackend::new();
        assert!(!backend.is_ready());
        assert!(backend.base_url().is_none());
    }
}
