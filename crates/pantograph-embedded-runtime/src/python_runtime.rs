//! Host-side adapter boundary for Python-backed workflow nodes.
//!
//! Python execution remains out-of-process and consumer-managed so Pantograph
//! itself does not link against a specific Python runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

const BRIDGE_SCRIPT_FILENAME: &str = "pantograph_python_runtime_bridge.py";
const BRIDGE_SCRIPT_SOURCE: &str = include_str!("python_runtime_bridge.py");

/// Request payload forwarded from workflow node execution into the host adapter.
#[derive(Debug, Clone)]
pub struct PythonNodeExecutionRequest {
    pub node_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub env_ids: Vec<String>,
}

/// Callback invoked for each streamed python-backed runtime chunk.
pub type PythonStreamHandler = Arc<dyn Fn(serde_json::Value) + Send + Sync>;

/// Host adapter interface for Python-backed node execution.
#[async_trait]
pub trait PythonRuntimeAdapter: Send + Sync {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String>;

    async fn execute_node_with_stream(
        &self,
        request: PythonNodeExecutionRequest,
        on_stream: Option<PythonStreamHandler>,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let _ = on_stream;
        self.execute_node(request).await
    }
}

/// Default adapter used until a process-based runtime is configured.
#[allow(dead_code)]
pub struct UnconfiguredPythonRuntimeAdapter;

#[async_trait]
impl PythonRuntimeAdapter for UnconfiguredPythonRuntimeAdapter {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let env_hint = if request.env_ids.is_empty() {
            "No dependency env_id was provided in model_ref.".to_string()
        } else {
            format!(
                "Resolved dependency env_id(s): {}",
                request.env_ids.join(", ")
            )
        };

        Err(format!(
            "Node '{}' requires the external Python runtime adapter. \
In-process Python execution is disabled in the default Pantograph build. {}",
            request.node_type, env_hint
        ))
    }
}

/// Process-based Python runtime adapter.
///
/// Python executable resolution is controlled by:
/// - `PANTOGRAPH_PYTHON_ENV_MAP_JSON`: JSON object mapping env_id -> python path
/// - `PANTOGRAPH_PYTHON_ENV_MAP_FILE`: path to JSON file with same mapping shape
/// - `PANTOGRAPH_PYTHON_EXECUTABLE`: default python executable fallback
pub struct ProcessPythonRuntimeAdapter;

#[derive(Debug, Serialize)]
struct BridgePayload {
    node_type: String,
    inputs: HashMap<String, serde_json::Value>,
    worker_paths: BridgeWorkerPaths,
}

#[derive(Debug, Serialize)]
struct BridgeWorkerPaths {
    audio_worker: String,
    onnx_worker: String,
}

#[derive(Debug, Deserialize)]
struct BridgeResponse {
    ok: bool,
    outputs: Option<HashMap<String, serde_json::Value>>,
    error: Option<String>,
    traceback: Option<String>,
}

static BRIDGE_SCRIPT_PATH: OnceLock<PathBuf> = OnceLock::new();

impl ProcessPythonRuntimeAdapter {
    fn resolve_worker_paths() -> Result<BridgeWorkerPaths, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(|path| path.parent())
            .ok_or_else(|| {
                format!(
                    "Unable to resolve repository root from CARGO_MANIFEST_DIR '{}'",
                    manifest_dir.display()
                )
            })?;

        let audio_worker = repo_root
            .join("crates")
            .join("inference")
            .join("audio")
            .join("worker.py");
        let onnx_worker = repo_root
            .join("crates")
            .join("inference")
            .join("onnx")
            .join("worker.py");

        if !audio_worker.exists() {
            return Err(format!(
                "Audio worker script not found at {}",
                audio_worker.display()
            ));
        }
        if !onnx_worker.exists() {
            return Err(format!(
                "ONNX worker script not found at {}",
                onnx_worker.display()
            ));
        }

        Ok(BridgeWorkerPaths {
            audio_worker: audio_worker.to_string_lossy().to_string(),
            onnx_worker: onnx_worker.to_string_lossy().to_string(),
        })
    }

    fn ensure_bridge_script() -> Result<PathBuf, String> {
        if let Some(path) = BRIDGE_SCRIPT_PATH.get() {
            return Ok(path.clone());
        }

        let path = std::env::temp_dir().join(BRIDGE_SCRIPT_FILENAME);
        let needs_write = std::fs::read_to_string(&path)
            .map(|existing| existing != BRIDGE_SCRIPT_SOURCE)
            .unwrap_or(true);
        if needs_write {
            std::fs::write(&path, BRIDGE_SCRIPT_SOURCE).map_err(|err| {
                format!(
                    "Failed to write python runtime bridge script at '{}': {}",
                    path.display(),
                    err
                )
            })?;
        }

        let _ = BRIDGE_SCRIPT_PATH.set(path.clone());
        Ok(path)
    }

    fn parse_bridge_response(stdout: &str) -> Result<BridgeResponse, String> {
        for line in stdout.lines().rev() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<BridgeResponse>(trimmed) {
                return Ok(parsed);
            }
        }

        serde_json::from_str::<BridgeResponse>(stdout)
            .map_err(|err| format!("Failed to parse python runtime response: {}", err))
    }

    fn parse_stream_chunk_line(line: &str) -> Option<serde_json::Value> {
        let parsed: serde_json::Value = serde_json::from_str(line).ok()?;
        if parsed.get("event").and_then(|v| v.as_str()) != Some("stream") {
            return None;
        }
        parsed.get("chunk").cloned()
    }
}

/// Resolve the python executable used for dependency checks/installs for env_id scopes.
pub(crate) fn resolve_python_executable_for_env_ids(env_ids: &[String]) -> Result<PathBuf, String> {
    crate::python_runtime_env_resolution::resolve_python_executable(env_ids)
}

/// Resolve the python executable for one required env_id without default fallback.
pub(crate) fn resolve_python_executable_for_required_env_id(
    env_id: &str,
) -> Result<PathBuf, String> {
    crate::python_runtime_env_resolution::resolve_python_executable_for_required_env_id(env_id)
}

#[async_trait]
impl PythonRuntimeAdapter for ProcessPythonRuntimeAdapter {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        self.execute_node_with_stream(request, None).await
    }

    async fn execute_node_with_stream(
        &self,
        request: PythonNodeExecutionRequest,
        on_stream: Option<PythonStreamHandler>,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let python_executable =
            crate::python_runtime_env_resolution::resolve_python_executable(&request.env_ids)?;
        let worker_paths = Self::resolve_worker_paths()?;
        let bridge_script = Self::ensure_bridge_script()?;

        let payload = BridgePayload {
            node_type: request.node_type.clone(),
            inputs: request.inputs,
            worker_paths,
        };
        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|err| format!("Failed to serialize python bridge payload: {}", err))?;

        let mut child = Command::new(&python_executable)
            .arg(&bridge_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| {
                format!(
                    "Failed to launch python runtime adapter using '{}': {}",
                    python_executable.display(),
                    err
                )
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&payload_bytes).await.map_err(|err| {
                format!(
                    "Failed to write python runtime request to adapter stdin: {}",
                    err
                )
            })?;
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture python runtime stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture python runtime stderr".to_string())?;

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        let mut stdout_lines = Vec::<String>::new();
        let mut stderr_lines = Vec::<String>::new();
        let mut parsed_response: Option<BridgeResponse> = None;
        let mut stdout_done = false;
        let mut stderr_done = false;

        while !stdout_done || !stderr_done {
            tokio::select! {
                line_result = stdout_reader.next_line(), if !stdout_done => {
                    let line_opt = line_result.map_err(|err| {
                        format!(
                            "Failed to read python runtime stdout ('{}'): {}",
                            python_executable.display(),
                            err
                        )
                    })?;
                    match line_opt {
                        Some(line) => {
                            let trimmed = line.trim().to_string();
                            stdout_lines.push(line);
                            if trimmed.is_empty() {
                                continue;
                            }

                            if let Some(handler) = on_stream.as_ref() {
                                if let Some(chunk) = Self::parse_stream_chunk_line(&trimmed) {
                                    handler(chunk);
                                    continue;
                                }
                            }

                            if parsed_response.is_none() {
                                if let Ok(response) =
                                    serde_json::from_str::<BridgeResponse>(&trimmed)
                                {
                                    parsed_response = Some(response);
                                }
                            }
                        }
                        None => {
                            stdout_done = true;
                        }
                    }
                }
                line_result = stderr_reader.next_line(), if !stderr_done => {
                    let line_opt = line_result.map_err(|err| {
                        format!(
                            "Failed to read python runtime stderr ('{}'): {}",
                            python_executable.display(),
                            err
                        )
                    })?;
                    match line_opt {
                        Some(line) => stderr_lines.push(line),
                        None => stderr_done = true,
                    }
                }
            }
        }

        let status = child.wait().await.map_err(|err| {
            format!(
                "Failed to wait for python runtime adapter process ('{}'): {}",
                python_executable.display(),
                err
            )
        })?;

        let stdout = stdout_lines.join("\n");
        let stderr = stderr_lines.join("\n");

        if !status.success() {
            if let Some(response) =
                parsed_response.or_else(|| Self::parse_bridge_response(&stdout).ok())
            {
                let mut details = response
                    .error
                    .unwrap_or_else(|| "Unknown python runtime bridge error".to_string());
                if let Some(traceback) = response.traceback {
                    if !traceback.trim().is_empty() {
                        details.push_str(&format!("\n{}", traceback.trim()));
                    }
                }
                return Err(format!(
                    "Python runtime adapter process exited with status {}. {}",
                    status, details
                ));
            }

            let stderr_trimmed = stderr.trim();
            let stdout_trimmed = stdout.trim();
            return Err(format!(
                "Python runtime adapter process exited with status {}. {}",
                status,
                if !stderr_trimmed.is_empty() {
                    format!("Stderr: {}", stderr_trimmed)
                } else if !stdout_trimmed.is_empty() {
                    format!("Stdout: {}", stdout_trimmed)
                } else {
                    "No stderr/stdout output.".to_string()
                }
            ));
        }

        let response = match parsed_response {
            Some(response) => response,
            None => Self::parse_bridge_response(&stdout)?,
        };
        if response.ok {
            return Ok(response.outputs.unwrap_or_default());
        }

        let mut details = response
            .error
            .unwrap_or_else(|| "Unknown python runtime bridge error".to_string());
        if let Some(traceback) = response.traceback {
            if !traceback.trim().is_empty() {
                details.push_str(&format!("\n{}", traceback.trim()));
            }
        } else if !stderr.trim().is_empty() {
            details.push_str(&format!("\n{}", stderr.trim()));
        }
        Err(details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_worker_paths_includes_onnx_worker() {
        let workers = ProcessPythonRuntimeAdapter::resolve_worker_paths()
            .expect("worker paths should resolve from repository layout");
        assert!(PathBuf::from(workers.audio_worker).exists());
        assert!(PathBuf::from(workers.onnx_worker).exists());
    }

    #[test]
    fn parse_stream_chunk_line_extracts_chunk_payload() {
        let line = r#"{"event":"stream","port":"stream","chunk":{"type":"audio_chunk","audio_base64":"abc","sequence":0,"is_final":true}}"#;
        let parsed = ProcessPythonRuntimeAdapter::parse_stream_chunk_line(line)
            .expect("stream event should parse");
        assert_eq!(parsed["type"], "audio_chunk");
        assert_eq!(parsed["audio_base64"], "abc");
        assert_eq!(parsed["sequence"], 0);
        assert_eq!(parsed["is_final"], true);
    }
}
