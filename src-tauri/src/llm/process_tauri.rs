//! Tauri-specific ProcessSpawner implementation.
//!
//! This spawner delegates runtime resolution to the inference crate and
//! launches the resolved executable directly via `tokio::process`.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{managed_runtime_dir, resolve_binary_command, ManagedBinaryId};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

struct TauriProcessHandle {
    pid: u32,
    child: Arc<Mutex<Option<Child>>>,
}

impl ProcessHandle for TauriProcessHandle {
    fn pid(&self) -> u32 {
        self.pid
    }

    fn kill(&self) -> Result<(), String> {
        let child = {
            let mut guard = self
                .child
                .lock()
                .map_err(|e| format!("Failed to acquire process lock: {}", e))?;
            guard.take()
        };

        if let Some(mut child) = child {
            child
                .start_kill()
                .map_err(|e| format!("Failed to kill process: {}", e))?;
        }

        Ok(())
    }
}

pub struct TauriProcessSpawner {
    app: AppHandle,
}

impl TauriProcessSpawner {
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
        let app_data_dir = self.app_data_dir()?;
        let resolved = match sidecar_name {
            "llama-server-wrapper" => {
                resolve_binary_command(&app_data_dir, ManagedBinaryId::LlamaCpp, args)?
            }
            "ollama" => resolve_binary_command(&app_data_dir, ManagedBinaryId::Ollama, args)?,
            other => {
                return Err(format!(
                    "Unsupported direct process spawn target for Tauri runtime: {}",
                    other
                ));
            }
        };

        let mut command = Command::new(&resolved.executable_path);
        command
            .args(&resolved.args)
            .current_dir(&resolved.working_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in resolved.env_overrides {
            command.env(key, value);
        }

        let mut child = command.spawn().map_err(|e| {
            format!(
                "Failed to spawn {}: {}",
                resolved.executable_path.display(),
                e
            )
        })?;

        let pid = child
            .id()
            .ok_or_else(|| "Spawned managed runtime process did not report a PID".to_string())?;

        if let Some(pid_file) = resolved.pid_file {
            if let Some(parent) = pid_file.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create PID file directory: {}", e))?;
            }
            std::fs::write(&pid_file, pid.to_string())
                .map_err(|e| format!("Failed to write PID file: {}", e))?;
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel(100);
        let child = Arc::new(Mutex::new(Some(child)));

        if let Some(stdout) = stdout {
            let tx = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(ProcessEvent::Stdout(line.into_bytes())).await.is_err() {
                        break;
                    }
                }
            });
        }

        if let Some(stderr) = stderr {
            let tx = tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx.send(ProcessEvent::Stderr(line.into_bytes())).await.is_err() {
                        break;
                    }
                }
            });
        }

        let monitor = Arc::clone(&child);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let wait_result = {
                    let mut guard = match monitor.lock() {
                        Ok(guard) => guard,
                        Err(_) => break,
                    };
                    let Some(child) = guard.as_mut() else {
                        break;
                    };
                    child.try_wait()
                };

                match wait_result {
                    Ok(Some(status)) => {
                        let _ = tx.send(ProcessEvent::Terminated(status.code())).await;
                        break;
                    }
                    Ok(None) => continue,
                    Err(error) => {
                        let _ = tx
                            .send(ProcessEvent::Error(format!("Wait error: {}", error)))
                            .await;
                        break;
                    }
                }
            }
        });

        Ok((rx, Box::new(TauriProcessHandle { pid, child })))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        self.app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(managed_runtime_dir(&self.app_data_dir()?))
    }
}

pub fn create_spawner(app: AppHandle) -> Arc<dyn ProcessSpawner> {
    Arc::new(TauriProcessSpawner::new(app))
}
