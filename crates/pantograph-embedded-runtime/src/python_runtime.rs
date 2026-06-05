//! Host-side adapter boundary for Python-backed workflow nodes.
//!
//! Python execution remains out-of-process and consumer-managed so Pantograph
//! itself does not link against a specific Python runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

const RETIRED_PROCESS_RUNTIME_ERROR: &str =
    "retired_python_runtime_adapter: Python-backed runtime node execution is scheduler-owned and \
must run through canonical scheduler task state/results plus runtime-host execution. The old \
process Python adapter must not launch from graph model_path, ModelRefV2, or node-engine runtime \
inputs.";

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

#[cfg(test)]
#[derive(Debug)]
struct BridgeWorkerPaths {
    audio_worker: String,
    onnx_worker: String,
}

impl ProcessPythonRuntimeAdapter {
    #[cfg(test)]
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
        let _ = request;
        let _ = on_stream;
        Err(RETIRED_PROCESS_RUNTIME_ERROR.to_string())
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

    #[tokio::test]
    async fn process_python_runtime_adapter_fails_closed_without_spawning_bridge() {
        let adapter = ProcessPythonRuntimeAdapter;
        let err = adapter
            .execute_node(PythonNodeExecutionRequest {
                node_type: "onnx-inference".to_string(),
                inputs: HashMap::from([(
                    "model_path".to_string(),
                    serde_json::json!("/tmp/legacy.onnx"),
                )]),
                env_ids: Vec::new(),
            })
            .await
            .expect_err("retired adapter must not launch python");

        assert!(err.contains("retired_python_runtime_adapter"));
        assert!(err.contains("scheduler-owned"));
        assert!(err.contains("model_path"));
    }
}
