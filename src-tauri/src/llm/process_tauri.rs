//! Tauri-specific ProcessSpawner implementation
//!
//! This module provides a ProcessSpawner that uses Tauri's shell plugin
//! to spawn and manage sidecar processes.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc;

/// Tauri process handle wrapper
struct TauriProcessHandle {
    pid: u32,
    child: Mutex<Option<CommandChild>>,
}

impl ProcessHandle for TauriProcessHandle {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn kill(&self) -> Result<(), String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|e| format!("Failed to acquire process lock: {}", e))?;

        if let Some(child) = guard.take() {
            child
                .kill()
                .map_err(|e| format!("Failed to kill process: {}", e))?;
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

        let handle = TauriProcessHandle {
            pid: child.pid(),
            child: Mutex::new(Some(child)),
        };

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
