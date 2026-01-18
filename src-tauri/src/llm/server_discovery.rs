//! Server discovery and takeover
//!
//! Finds existing Pantograph servers and enables connecting to them
//! instead of spawning new processes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

use crate::llm::port_manager::{check_port_available, get_process_info};

/// Enhanced PID file with full server metadata
const SERVER_REGISTRY_FILE: &str = "pantograph-server.json";

/// Server operating mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegisteredServerMode {
    Inference,
    Embedding,
}

/// Full server registration with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRegistration {
    /// Process ID
    pub pid: u32,
    /// Port the server is listening on
    pub port: u16,
    /// Server mode (inference or embedding)
    pub mode: RegisteredServerMode,
    /// Path to the model file
    pub model_path: String,
    /// Path to mmproj file (for inference mode)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mmproj_path: Option<String>,
    /// When the server was started
    pub started_at: DateTime<Utc>,
    /// Unique instance ID (to distinguish between runs)
    pub instance_id: String,
    /// Device configuration used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// GPU layers configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_layers: Option<i32>,
}

impl ServerRegistration {
    /// Create a new registration for an inference server
    pub fn new_inference(
        pid: u32,
        port: u16,
        model_path: String,
        mmproj_path: String,
        device: Option<String>,
        gpu_layers: Option<i32>,
    ) -> Self {
        Self {
            pid,
            port,
            mode: RegisteredServerMode::Inference,
            model_path,
            mmproj_path: Some(mmproj_path),
            started_at: Utc::now(),
            instance_id: Uuid::new_v4().to_string(),
            device,
            gpu_layers,
        }
    }

    /// Create a new registration for an embedding server
    pub fn new_embedding(
        pid: u32,
        port: u16,
        model_path: String,
        device: Option<String>,
        gpu_layers: Option<i32>,
    ) -> Self {
        Self {
            pid,
            port,
            mode: RegisteredServerMode::Embedding,
            model_path,
            mmproj_path: None,
            started_at: Utc::now(),
            instance_id: Uuid::new_v4().to_string(),
            device,
            gpu_layers,
        }
    }
}

/// Result of server discovery
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveryResult {
    /// Found registrations that are still valid
    pub active_servers: Vec<ServerRegistration>,
    /// Registrations that were stale (process dead)
    pub stale_registrations: Vec<ServerRegistration>,
}

/// Server discovery manager
pub struct ServerDiscovery {
    app_data_dir: PathBuf,
}

impl ServerDiscovery {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { app_data_dir }
    }

    /// Get the path to the server registry file
    fn registry_path(&self) -> PathBuf {
        self.app_data_dir.join(SERVER_REGISTRY_FILE)
    }

    /// Register a running server
    pub fn register(&self, registration: &ServerRegistration) -> Result<(), String> {
        let path = self.registry_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create registry directory: {}", e))?;
        }

        let json = serde_json::to_string_pretty(registration)
            .map_err(|e| format!("Failed to serialize registration: {}", e))?;

        fs::write(&path, json).map_err(|e| format!("Failed to write registry: {}", e))?;

        log::info!(
            "Registered server: pid={}, port={}, mode={:?}",
            registration.pid,
            registration.port,
            registration.mode
        );

        Ok(())
    }

    /// Unregister a server (on clean shutdown)
    pub fn unregister(&self) -> Result<(), String> {
        let path = self.registry_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove registry: {}", e))?;
            log::info!("Unregistered server");
        }
        Ok(())
    }

    /// Find existing servers from the registry
    pub fn discover(&self) -> DiscoveryResult {
        let mut result = DiscoveryResult {
            active_servers: Vec::new(),
            stale_registrations: Vec::new(),
        };

        let path = self.registry_path();
        if !path.exists() {
            return result;
        }

        // Read and parse the registry
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Failed to read server registry: {}", e);
                return result;
            }
        };

        let registration: ServerRegistration = match serde_json::from_str(&contents) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("Failed to parse server registry: {}", e);
                // Remove corrupted registry
                let _ = fs::remove_file(&path);
                return result;
            }
        };

        // Verify the process is still running
        if self.is_process_running(registration.pid) {
            // Additionally verify it's still listening on the expected port
            let port_status = check_port_available(registration.port);
            if !port_status.available {
                // Port is in use - check if it's our process
                if let Some(ref proc_info) = port_status.blocking_process {
                    if proc_info.pid == registration.pid {
                        result.active_servers.push(registration);
                        return result;
                    }
                }
            }
            // Process running but port mismatch - stale
            result.stale_registrations.push(registration);
        } else {
            result.stale_registrations.push(registration);
        }

        result
    }

    /// Verify a registered server is still responsive
    pub async fn verify_server(&self, registration: &ServerRegistration) -> bool {
        let url = format!("http://127.0.0.1:{}/health", registration.port);

        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Check if a process is running
    #[cfg(unix)]
    fn is_process_running(&self, pid: u32) -> bool {
        use std::process::Command;

        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    fn is_process_running(&self, _pid: u32) -> bool {
        // On non-Unix, assume process is running if we have a registration
        // We'll verify with HTTP health check anyway
        true
    }

    /// Clean up stale registrations
    pub fn cleanup_stale(&self) -> Result<(), String> {
        let result = self.discover();

        if !result.stale_registrations.is_empty() {
            log::info!(
                "Cleaning up {} stale server registrations",
                result.stale_registrations.len()
            );
            self.unregister()?;
        }

        Ok(())
    }

    /// Try to connect to an existing server
    ///
    /// Returns the registration if successful, None if no valid server found
    pub async fn try_connect_existing(&self) -> Option<ServerRegistration> {
        let result = self.discover();

        for registration in result.active_servers {
            if self.verify_server(&registration).await {
                log::info!(
                    "Found existing server: pid={}, port={}, instance={}",
                    registration.pid,
                    registration.port,
                    registration.instance_id
                );
                return Some(registration);
            }
        }

        // Clean up stale registrations
        if let Err(e) = self.cleanup_stale() {
            log::warn!("Failed to cleanup stale registrations: {}", e);
        }

        None
    }

    /// Check if the current registration matches what we want
    pub fn matches_config(
        &self,
        registration: &ServerRegistration,
        mode: RegisteredServerMode,
        model_path: &str,
        mmproj_path: Option<&str>,
    ) -> bool {
        if registration.mode != mode {
            return false;
        }

        if registration.model_path != model_path {
            return false;
        }

        // For inference mode, check mmproj path
        if mode == RegisteredServerMode::Inference {
            match (registration.mmproj_path.as_deref(), mmproj_path) {
                (Some(reg_mmproj), Some(req_mmproj)) => {
                    if reg_mmproj != req_mmproj {
                        return false;
                    }
                }
                (None, Some(_)) | (Some(_), None) => return false,
                (None, None) => {}
            }
        }

        true
    }
}

/// Get process info for display
pub fn get_server_info_display(registration: &ServerRegistration) -> String {
    let mode_str = match registration.mode {
        RegisteredServerMode::Inference => "Inference",
        RegisteredServerMode::Embedding => "Embedding",
    };

    let uptime = Utc::now().signed_duration_since(registration.started_at);
    let uptime_str = if uptime.num_hours() > 0 {
        format!("{}h {}m", uptime.num_hours(), uptime.num_minutes() % 60)
    } else if uptime.num_minutes() > 0 {
        format!("{}m {}s", uptime.num_minutes(), uptime.num_seconds() % 60)
    } else {
        format!("{}s", uptime.num_seconds())
    };

    format!(
        "{} server on port {} (PID: {}, uptime: {})",
        mode_str, registration.port, registration.pid, uptime_str
    )
}
