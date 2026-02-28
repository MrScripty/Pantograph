//! Host-side adapter boundary for Python-backed workflow nodes.
//!
//! Python execution remains out-of-process and consumer-managed so Pantograph
//! itself does not link against a specific Python runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

const ENV_DEFAULT_PYTHON_EXECUTABLE: &str = "PANTOGRAPH_PYTHON_EXECUTABLE";
const ENV_PYTHON_ENV_MAP_JSON: &str = "PANTOGRAPH_PYTHON_ENV_MAP_JSON";
const ENV_PYTHON_ENV_MAP_FILE: &str = "PANTOGRAPH_PYTHON_ENV_MAP_FILE";

const BRIDGE_SCRIPT_FILENAME: &str = "pantograph_python_runtime_bridge.py";
const BRIDGE_SCRIPT_SOURCE: &str = include_str!("python_runtime_bridge.py");

/// Request payload forwarded from workflow node execution into the host adapter.
#[derive(Debug, Clone)]
pub struct PythonNodeExecutionRequest {
    pub node_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub env_ids: Vec<String>,
}

/// Host adapter interface for Python-backed node execution.
#[async_trait]
pub trait PythonRuntimeAdapter: Send + Sync {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String>;
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
            format!("Resolved dependency env_id(s): {}", request.env_ids.join(", "))
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
    torch_worker: String,
    audio_worker: String,
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
    fn load_python_env_map() -> Result<HashMap<String, PathBuf>, String> {
        let mut out = HashMap::new();

        if let Ok(path) = std::env::var(ENV_PYTHON_ENV_MAP_FILE) {
            let trimmed = path.trim();
            if !trimmed.is_empty() {
                let file_path = PathBuf::from(trimmed);
                let raw = std::fs::read_to_string(&file_path).map_err(|err| {
                    format!(
                        "Failed to read {} at '{}': {}",
                        ENV_PYTHON_ENV_MAP_FILE,
                        file_path.display(),
                        err
                    )
                })?;
                let parsed = Self::parse_python_env_map_json(&raw).map_err(|err| {
                    format!(
                        "Invalid {} JSON in '{}': {}",
                        ENV_PYTHON_ENV_MAP_FILE,
                        file_path.display(),
                        err
                    )
                })?;
                out.extend(parsed);
            }
        }

        if let Ok(raw) = std::env::var(ENV_PYTHON_ENV_MAP_JSON) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                let parsed = Self::parse_python_env_map_json(trimmed)
                    .map_err(|err| format!("Invalid {}: {}", ENV_PYTHON_ENV_MAP_JSON, err))?;
                out.extend(parsed);
            }
        }

        Ok(out)
    }

    fn parse_python_env_map_json(raw: &str) -> Result<HashMap<String, PathBuf>, String> {
        let parsed = serde_json::from_str::<HashMap<String, String>>(raw)
            .map_err(|err| format!("expected JSON object env_id -> python path: {}", err))?;
        let mut out = HashMap::new();
        for (env_id, path) in parsed {
            let env_id_trimmed = env_id.trim();
            let path_trimmed = path.trim();
            if env_id_trimmed.is_empty() || path_trimmed.is_empty() {
                continue;
            }
            out.insert(env_id_trimmed.to_string(), PathBuf::from(path_trimmed));
        }
        Ok(out)
    }

    fn resolve_python_executable(env_ids: &[String]) -> Result<PathBuf, String> {
        let env_map = Self::load_python_env_map()?;
        for env_id in env_ids {
            if let Some(path) = env_map.get(env_id) {
                if path.exists() {
                    return Ok(path.clone());
                }
                return Err(format!(
                    "Configured python executable for env_id '{}' does not exist: {}",
                    env_id,
                    path.display()
                ));
            }
        }

        if let Ok(default_python) = std::env::var(ENV_DEFAULT_PYTHON_EXECUTABLE) {
            let trimmed = default_python.trim();
            if !trimmed.is_empty() {
                let candidate = PathBuf::from(trimmed);
                if candidate.exists() {
                    return Ok(candidate);
                }
                return Err(format!(
                    "{} is set but target does not exist: {}",
                    ENV_DEFAULT_PYTHON_EXECUTABLE,
                    candidate.display()
                ));
            }
        }

        let env_hint = if env_ids.is_empty() {
            "No env_id was provided for this request.".to_string()
        } else {
            format!("Missing python executable mapping for env_id(s): {}", env_ids.join(", "))
        };
        Err(format!(
            "Python runtime is not configured. {} Set {} or {} (or {}).",
            env_hint,
            ENV_DEFAULT_PYTHON_EXECUTABLE,
            ENV_PYTHON_ENV_MAP_JSON,
            ENV_PYTHON_ENV_MAP_FILE
        ))
    }

    fn resolve_worker_paths() -> Result<BridgeWorkerPaths, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().ok_or_else(|| {
            format!(
                "Unable to resolve repository root from CARGO_MANIFEST_DIR '{}'",
                manifest_dir.display()
            )
        })?;

        let torch_worker = repo_root.join("crates").join("inference").join("torch").join("worker.py");
        let audio_worker = repo_root.join("crates").join("inference").join("audio").join("worker.py");

        if !torch_worker.exists() {
            return Err(format!(
                "Torch worker script not found at {}",
                torch_worker.display()
            ));
        }
        if !audio_worker.exists() {
            return Err(format!(
                "Audio worker script not found at {}",
                audio_worker.display()
            ));
        }

        Ok(BridgeWorkerPaths {
            torch_worker: torch_worker.to_string_lossy().to_string(),
            audio_worker: audio_worker.to_string_lossy().to_string(),
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
}

#[async_trait]
impl PythonRuntimeAdapter for ProcessPythonRuntimeAdapter {
    async fn execute_node(
        &self,
        request: PythonNodeExecutionRequest,
    ) -> Result<HashMap<String, serde_json::Value>, String> {
        let python_executable = Self::resolve_python_executable(&request.env_ids)?;
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

        let output = child.wait_with_output().await.map_err(|err| {
            format!(
                "Failed to wait for python runtime adapter process ('{}'): {}",
                python_executable.display(),
                err
            )
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !output.status.success() {
            let stderr_trimmed = stderr.trim();
            return Err(format!(
                "Python runtime adapter process exited with status {}. {}",
                output.status,
                if stderr_trimmed.is_empty() {
                    "No stderr output.".to_string()
                } else {
                    format!("Stderr: {}", stderr_trimmed)
                }
            ));
        }

        let response = Self::parse_bridge_response(&stdout)?;
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
