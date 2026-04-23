//! Tauri-specific ProcessSpawner implementation.
//!
//! This spawner delegates runtime resolution to the inference crate and
//! launches the resolved executable directly via `tokio::process`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{managed_runtime_dir, resolve_binary_command, ManagedBinaryId};
use serde::Serialize;
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

struct TauriProcessHandle {
    pid: u32,
    child: Arc<Mutex<Option<Child>>>,
    auxiliary_tasks: Arc<Mutex<Vec<OwnedProcessTask>>>,
}

struct OwnedProcessTask {
    name: &'static str,
    handle: JoinHandle<()>,
}

#[derive(Serialize)]
struct ManagedRuntimePidRecord {
    schema_version: u32,
    pid: u32,
    started_at_ms: u64,
    owner: &'static str,
    owner_version: &'static str,
    mode: String,
    executable: String,
}

impl TauriProcessHandle {
    fn abort_auxiliary_tasks(&self) {
        abort_auxiliary_tasks(&self.auxiliary_tasks);
    }
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
        self.abort_auxiliary_tasks();

        Ok(())
    }
}

impl Drop for TauriProcessHandle {
    fn drop(&mut self) {
        self.abort_auxiliary_tasks();
    }
}

fn track_auxiliary_task(
    auxiliary_tasks: &Arc<Mutex<Vec<OwnedProcessTask>>>,
    name: &'static str,
    handle: JoinHandle<()>,
) {
    match auxiliary_tasks.lock() {
        Ok(mut tasks) => {
            tasks.push(OwnedProcessTask { name, handle });
        }
        Err(error) => {
            log::error!("Failed to track managed-runtime task '{name}': {error}");
            handle.abort();
        }
    }
}

fn abort_auxiliary_tasks(auxiliary_tasks: &Arc<Mutex<Vec<OwnedProcessTask>>>) {
    let tasks = match auxiliary_tasks.lock() {
        Ok(mut tasks) => std::mem::take(&mut *tasks),
        Err(error) => {
            log::error!("Failed to acquire managed-runtime task registry: {error}");
            return;
        }
    };

    for task in tasks {
        task.handle.abort();
        log::debug!("Aborted managed-runtime task '{}'", task.name);
    }
}

fn write_managed_runtime_pid_record(
    pid_file: &Path,
    pid: u32,
    sidecar_name: &str,
    args: &[&str],
    executable_path: &Path,
) -> Result<(), String> {
    let record = ManagedRuntimePidRecord {
        schema_version: 1,
        pid,
        started_at_ms: unix_timestamp_ms(),
        owner: "pantograph-tauri",
        owner_version: env!("CARGO_PKG_VERSION"),
        mode: managed_runtime_mode(sidecar_name, args).to_string(),
        executable: executable_path.display().to_string(),
    };
    let json = serde_json::to_string_pretty(&record)
        .map_err(|error| format!("Failed to serialize PID record: {error}"))?;
    std::fs::write(pid_file, json).map_err(|e| format!("Failed to write PID file: {}", e))
}

fn managed_runtime_mode(sidecar_name: &str, args: &[&str]) -> &'static str {
    match sidecar_name {
        "ollama" => "ollama",
        "llama-server-wrapper" if args.contains(&"--embedding") => "llama.cpp.embedding",
        "llama-server-wrapper" if args.contains(&"--reranking") => "llama.cpp.reranking",
        "llama-server-wrapper" => "llama.cpp.inference",
        _ => "unknown",
    }
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64
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
            write_managed_runtime_pid_record(
                &pid_file,
                pid,
                sidecar_name,
                args,
                &resolved.executable_path,
            )?;
        }

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let (tx, rx) = mpsc::channel(100);
        let child = Arc::new(Mutex::new(Some(child)));
        let auxiliary_tasks = Arc::new(Mutex::new(Vec::new()));

        if let Some(stdout) = stdout {
            let tx = tx.clone();
            let stdout_task = tokio::spawn(async move {
                let reader = BufReader::new(stdout);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx
                        .send(ProcessEvent::Stdout(line.into_bytes()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            track_auxiliary_task(&auxiliary_tasks, "stdout-reader", stdout_task);
        }

        if let Some(stderr) = stderr {
            let tx = tx.clone();
            let stderr_task = tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    if tx
                        .send(ProcessEvent::Stderr(line.into_bytes()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            });
            track_auxiliary_task(&auxiliary_tasks, "stderr-reader", stderr_task);
        }

        let monitor = Arc::clone(&child);
        let monitor_task = tokio::spawn(async move {
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
        track_auxiliary_task(&auxiliary_tasks, "process-monitor", monitor_task);

        Ok((
            rx,
            Box::new(TauriProcessHandle {
                pid,
                child,
                auxiliary_tasks,
            }),
        ))
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
