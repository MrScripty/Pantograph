//! Process spawning abstraction
//!
//! This module provides a trait-based abstraction for spawning external processes.
//! This allows the inference library to work with different process management systems:
//! - Tauri's `tauri-plugin-shell` for desktop apps
//! - Standard `std::process::Command` for CLI tools and servers
//!
//! # Example
//!
//! ```rust,ignore
//! use inference::process::{ProcessSpawner, StdProcessSpawner};
//! use std::path::PathBuf;
//!
//! let spawner = StdProcessSpawner::new(
//!     PathBuf::from("/path/to/binaries"),
//!     PathBuf::from("/path/to/data"),
//! );
//!
//! let (mut rx, handle) = spawner.spawn_sidecar("llama-server", &["-m", "model.gguf"]).await?;
//!
//! while let Some(event) = rx.recv().await {
//!     match event {
//!         ProcessEvent::Stdout(data) => println!("stdout: {}", String::from_utf8_lossy(&data)),
//!         ProcessEvent::Stderr(data) => eprintln!("stderr: {}", String::from_utf8_lossy(&data)),
//!         ProcessEvent::Terminated(code) => break,
//!         _ => {}
//!     }
//! }
//! ```

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::sync::mpsc;

/// Output event from a spawned process
#[derive(Debug, Clone)]
pub enum ProcessEvent {
    /// Data written to stdout
    Stdout(Vec<u8>),
    /// Data written to stderr
    Stderr(Vec<u8>),
    /// Process error (e.g., failed to spawn)
    Error(String),
    /// Process terminated with optional exit code
    Terminated(Option<i32>),
}

/// Handle to a spawned process
pub trait ProcessHandle: Send + Sync {
    /// Get the process ID
    fn pid(&self) -> u32;
    /// Kill the process
    fn kill(&self) -> Result<(), String>;
}

/// Trait for spawning external processes
///
/// Implementations can use different process management systems:
/// - `TauriProcessSpawner` for Tauri desktop apps (uses `tauri-plugin-shell`)
/// - `StdProcessSpawner` for standalone use (uses `std::process::Command`)
#[async_trait]
pub trait ProcessSpawner: Send + Sync {
    /// Spawn a sidecar process
    ///
    /// # Arguments
    /// * `sidecar_name` - Name of the sidecar binary (e.g., "llama-server-wrapper")
    /// * `args` - Command line arguments
    ///
    /// # Returns
    /// A tuple of (event receiver, process handle)
    async fn spawn_sidecar(
        &self,
        sidecar_name: &str,
        args: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String>;

    /// Get the app data directory for storing PID files and other runtime data
    fn app_data_dir(&self) -> Result<PathBuf, String>;

    /// Get the directory containing sidecar binaries
    fn binaries_dir(&self) -> Result<PathBuf, String>;
}

// ============================================================================
// Standard Process Spawner (for non-Tauri use)
// ============================================================================

#[cfg(feature = "std-process")]
mod std_process {
    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncBufReadExt, BufReader};

    /// Standard library process handle
    struct StdProcessHandle {
        child: Arc<Mutex<Option<Child>>>,
        pid: u32,
    }

    impl ProcessHandle for StdProcessHandle {
        fn pid(&self) -> u32 {
            self.pid
        }

        fn kill(&self) -> Result<(), String> {
            let mut guard = self.child.lock().map_err(|e| e.to_string())?;
            if let Some(child) = guard.as_mut() {
                child.kill().map_err(|e| format!("Failed to kill process: {}", e))?;
            }
            Ok(())
        }
    }

    /// Process spawner using standard library
    ///
    /// This is suitable for CLI tools, servers, and other non-Tauri applications.
    pub struct StdProcessSpawner {
        binaries_dir: PathBuf,
        data_dir: PathBuf,
    }

    impl StdProcessSpawner {
        /// Create a new standard process spawner
        ///
        /// # Arguments
        /// * `binaries_dir` - Directory containing sidecar binaries
        /// * `data_dir` - Directory for storing runtime data (PID files, etc.)
        pub fn new(binaries_dir: PathBuf, data_dir: PathBuf) -> Self {
            Self {
                binaries_dir,
                data_dir,
            }
        }
    }

    #[async_trait]
    impl ProcessSpawner for StdProcessSpawner {
        async fn spawn_sidecar(
            &self,
            sidecar_name: &str,
            args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            let binary_path = self.binaries_dir.join(sidecar_name);

            if !binary_path.exists() {
                return Err(format!(
                    "Sidecar binary not found: {}",
                    binary_path.display()
                ));
            }

            let mut child = Command::new(&binary_path)
                .args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn {}: {}", sidecar_name, e))?;

            let pid = child.id();
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();

            let (tx, rx) = mpsc::channel(100);
            let child_arc = Arc::new(Mutex::new(Some(child)));

            // Spawn stdout reader
            if let Some(stdout) = stdout {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(tokio::process::ChildStdout::from_std(stdout).unwrap());
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _ = tx.send(ProcessEvent::Stdout(line.into_bytes())).await;
                    }
                });
            }

            // Spawn stderr reader
            if let Some(stderr) = stderr {
                let tx = tx.clone();
                tokio::spawn(async move {
                    let reader = BufReader::new(tokio::process::ChildStderr::from_std(stderr).unwrap());
                    let mut lines = reader.lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        let _ = tx.send(ProcessEvent::Stderr(line.into_bytes())).await;
                    }
                });
            }

            // Spawn process monitor
            let child_arc_clone = child_arc.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                    // Check process status without holding lock across await
                    let check_result = {
                        let mut guard = match child_arc_clone.lock() {
                            Ok(g) => g,
                            Err(_) => break,
                        };
                        if let Some(child) = guard.as_mut() {
                            child.try_wait()
                        } else {
                            break;
                        }
                    };

                    match check_result {
                        Ok(Some(status)) => {
                            let _ = tx
                                .send(ProcessEvent::Terminated(status.code()))
                                .await;
                            break;
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            let _ = tx
                                .send(ProcessEvent::Error(format!("Wait error: {}", e)))
                                .await;
                            break;
                        }
                    }
                }
            });

            let handle = StdProcessHandle {
                child: child_arc,
                pid,
            };

            Ok((rx, Box::new(handle)))
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(self.data_dir.clone())
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(self.binaries_dir.clone())
        }
    }
}

#[cfg(feature = "std-process")]
pub use std_process::StdProcessSpawner;
