//! Tauri-specific ProcessSpawner implementation
//!
//! This module provides a ProcessSpawner that uses Tauri's shell plugin
//! to spawn and manage sidecar processes.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc;

/// Tauri process handle wrapper
struct TauriProcessHandle {
    child: CommandChild,
}

impl ProcessHandle for TauriProcessHandle {
    fn pid(&self) -> u32 {
        self.child.pid()
    }

    fn kill(&self) -> Result<(), String> {
        // CommandChild::kill takes ownership in Tauri's API
        // But we need to keep the reference for pid(), so we use system kill
        #[cfg(unix)]
        {
            use std::process::Command;
            let pid_arg = self.child.pid().to_string();
            Command::new("kill")
                .arg("-TERM")
                .arg(&pid_arg)
                .status()
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }

        #[cfg(not(unix))]
        {
            // On Windows, we'd need a different approach
            // For now, just return an error
            return Err("Process termination not implemented for this platform".to_string());
        }

        Ok(())
    }
}

/// Process spawner using Tauri's shell plugin
///
/// This spawner is designed for use in Tauri desktop applications.
/// It uses the `tauri-plugin-shell` for secure sidecar process management.
pub struct TauriProcessSpawner {
    app: AppHandle,
}

impl TauriProcessSpawner {
    /// Create a new Tauri process spawner
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl ProcessSpawner for TauriProcessSpawner {
    async fn spawn_sidecar(
        &self,
        sidecar_name: &str,
        args: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
        // Create the sidecar command
        let sidecar = self
            .app
            .shell()
            .sidecar(sidecar_name)
            .map_err(|e| format!("Failed to create sidecar: {}", e))?
            .args(args);

        // Spawn the process
        let (mut rx, child) = sidecar
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", sidecar_name, e))?;

        // Create channel for ProcessEvents
        let (tx, out_rx) = mpsc::channel(100);

        // Spawn a task to convert Tauri events to ProcessEvents
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let process_event = match event {
                    CommandEvent::Stdout(data) => ProcessEvent::Stdout(data),
                    CommandEvent::Stderr(data) => ProcessEvent::Stderr(data),
                    CommandEvent::Error(err) => ProcessEvent::Error(err),
                    CommandEvent::Terminated(status) => {
                        ProcessEvent::Terminated(status.code.map(|c| c as i32))
                    }
                    _ => continue, // Skip other event types
                };

                if tx.send(process_event).await.is_err() {
                    break; // Receiver dropped
                }
            }
        });

        let handle = TauriProcessHandle { child };

        Ok((out_rx, Box::new(handle)))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        self.app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        // Tauri sidecars are resolved by the shell plugin automatically
        // Return the app's resource directory for reference
        self.app
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))
    }
}

/// Create a shared process spawner from an AppHandle
pub fn create_spawner(app: AppHandle) -> Arc<dyn ProcessSpawner> {
    Arc::new(TauriProcessSpawner::new(app))
}
