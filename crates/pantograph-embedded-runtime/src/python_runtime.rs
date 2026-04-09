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

const ENV_DEFAULT_PYTHON_EXECUTABLE: &str = "PANTOGRAPH_PYTHON_EXECUTABLE";
const ENV_PYTHON_ENV_MAP_JSON: &str = "PANTOGRAPH_PYTHON_ENV_MAP_JSON";
const ENV_PYTHON_ENV_MAP_FILE: &str = "PANTOGRAPH_PYTHON_ENV_MAP_FILE";
const ENV_PYO3_PYTHON: &str = "PYO3_PYTHON";

const BRIDGE_SCRIPT_FILENAME: &str = "pantograph_python_runtime_bridge.py";
const BRIDGE_SCRIPT_SOURCE: &str = include_str!("python_runtime_bridge.py");

/// Request payload forwarded from workflow node execution into the host adapter.
#[derive(Debug, Clone)]
pub struct PythonNodeExecutionRequest {
    pub node_type: String,
    pub inputs: HashMap<String, serde_json::Value>,
    pub env_ids: Vec<String>,
}

/// Callback invoked for each streamed python-sidecar chunk.
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
    torch_worker: String,
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
    fn resolve_python_candidate(raw: &str) -> Option<PathBuf> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }

        let candidate = PathBuf::from(trimmed);
        if candidate.exists() {
            return Some(candidate);
        }

        which::which(trimmed).ok()
    }

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
                if let Some(resolved) = Self::resolve_python_candidate(&path.to_string_lossy()) {
                    return Ok(resolved);
                }
                return Err(format!(
                    "Configured python executable for env_id '{}' was not found as a path or PATH command: {}",
                    env_id,
                    path.display()
                ));
            }
        }

        if let Ok(default_python) = std::env::var(ENV_DEFAULT_PYTHON_EXECUTABLE) {
            if let Some(candidate) = Self::resolve_python_candidate(&default_python) {
                return Ok(candidate);
            }
            let trimmed = default_python.trim();
            if !trimmed.is_empty() {
                return Err(format!(
                    "{} is set but target was not found as a path or PATH command: {}",
                    ENV_DEFAULT_PYTHON_EXECUTABLE, trimmed
                ));
            }
        }

        if let Ok(pyo3_python) = std::env::var(ENV_PYO3_PYTHON) {
            if let Some(candidate) = Self::resolve_python_candidate(&pyo3_python) {
                return Ok(candidate);
            }
            let trimmed = pyo3_python.trim();
            if !trimmed.is_empty() {
                return Err(format!(
                    "{} is set but target was not found as a path or PATH command: {}",
                    ENV_PYO3_PYTHON, trimmed
                ));
            }
        }

        if let Some(candidate) = Self::resolve_repo_local_venv_python() {
            return Ok(candidate);
        }

        for command in ["python3", "python"] {
            if let Some(candidate) = Self::resolve_python_candidate(command) {
                return Ok(candidate);
            }
        }

        let env_hint = if env_ids.is_empty() {
            "No env_id was provided for this request.".to_string()
        } else {
            format!(
                "Missing python executable mapping for env_id(s): {}",
                env_ids.join(", ")
            )
        };
        Err(format!(
            "Python runtime is not configured. {} Set {} or {} (or {}), or ensure {} or a PATH python command is available.",
            env_hint,
            ENV_DEFAULT_PYTHON_EXECUTABLE,
            ENV_PYTHON_ENV_MAP_JSON,
            ENV_PYTHON_ENV_MAP_FILE,
            ENV_PYO3_PYTHON
        ))
    }

    fn resolve_repo_local_venv_python() -> Option<PathBuf> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent()?.parent()?;

        for candidate in [
            repo_root.join(".venv").join("bin").join("python3"),
            repo_root.join(".venv").join("bin").join("python"),
            repo_root.join(".venv").join("Scripts").join("python.exe"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    fn resolve_worker_paths() -> Result<BridgeWorkerPaths, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().and_then(|path| path.parent()).ok_or_else(|| {
            format!(
                "Unable to resolve repository root from CARGO_MANIFEST_DIR '{}'",
                manifest_dir.display()
            )
        })?;

        let torch_worker = repo_root
            .join("crates")
            .join("inference")
            .join("torch")
            .join("worker.py");
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
        if !onnx_worker.exists() {
            return Err(format!(
                "ONNX worker script not found at {}",
                onnx_worker.display()
            ));
        }

        Ok(BridgeWorkerPaths {
            torch_worker: torch_worker.to_string_lossy().to_string(),
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
    ProcessPythonRuntimeAdapter::resolve_python_executable(env_ids)
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
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: Option<String>) -> Self {
            let original = std::env::var(key).ok();
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.original.take() {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn parse_python_env_map_json_trims_and_filters_entries() {
        let raw = r#"{
            " env-one ": " /tmp/python one ",
            "": "/tmp/skip-empty-key",
            "env-two": ""
        }"#;
        let parsed = ProcessPythonRuntimeAdapter::parse_python_env_map_json(raw)
            .expect("parse should succeed");
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed.get("env-one"),
            Some(&PathBuf::from("/tmp/python one"))
        );
    }

    #[test]
    fn resolve_python_executable_from_env_map_file_with_spaces_in_paths() {
        let _lock = env_lock().lock().expect("env lock");
        let unique = format!(
            "pantograph python runtime {}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        );
        let base = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&base).expect("create base dir");

        let python_path = base.join("python executable with spaces");
        std::fs::write(&python_path, "").expect("write fake python executable");

        let env_map_path = base.join("env map with spaces.json");
        let env_map_json = serde_json::json!({
            "venv:space": python_path.to_string_lossy().to_string()
        });
        std::fs::write(
            &env_map_path,
            serde_json::to_string(&env_map_json).expect("serialize env map"),
        )
        .expect("write env map");

        let _map_file_guard = EnvGuard::set(
            ENV_PYTHON_ENV_MAP_FILE,
            Some(env_map_path.to_string_lossy().to_string()),
        );
        let _map_json_guard = EnvGuard::set(ENV_PYTHON_ENV_MAP_JSON, None);
        let _default_guard = EnvGuard::set(ENV_DEFAULT_PYTHON_EXECUTABLE, None);

        let resolved =
            ProcessPythonRuntimeAdapter::resolve_python_executable(&["venv:space".to_string()])
                .expect("resolver should use env-map file entry with spaces");
        assert_eq!(resolved, python_path);

        std::fs::remove_dir_all(base).expect("cleanup temp dir");
    }

    #[test]
    fn resolve_python_executable_accepts_default_command_name() {
        let _lock = env_lock().lock().expect("env lock");
        let _map_file_guard = EnvGuard::set(ENV_PYTHON_ENV_MAP_FILE, None);
        let _map_json_guard = EnvGuard::set(ENV_PYTHON_ENV_MAP_JSON, None);
        let _pyo3_guard = EnvGuard::set(ENV_PYO3_PYTHON, None);
        let _default_guard =
            EnvGuard::set(ENV_DEFAULT_PYTHON_EXECUTABLE, Some("python3".to_string()));

        let resolved = ProcessPythonRuntimeAdapter::resolve_python_executable(&[])
            .expect("resolver should locate python3 from PATH");
        assert!(resolved.exists());
    }

    #[test]
    fn resolve_worker_paths_includes_onnx_worker() {
        let workers = ProcessPythonRuntimeAdapter::resolve_worker_paths()
            .expect("worker paths should resolve from repository layout");
        assert!(PathBuf::from(workers.torch_worker).exists());
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
