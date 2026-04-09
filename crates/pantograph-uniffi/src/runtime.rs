use std::path::PathBuf;
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowIoRequest,
    WorkflowPreflightRequest, WorkflowRunRequest, WorkflowService, WorkflowServiceError,
    WorkflowSessionCloseRequest, WorkflowSessionCreateRequest, WorkflowSessionKeepAliveRequest,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionRunRequest,
    WorkflowSessionStatusRequest,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::RwLock;

use crate::{FfiError, FfiPumasApi};

/// FFI-safe configuration for an embedded Pantograph runtime.
#[derive(uniffi::Record)]
pub struct FfiEmbeddedRuntimeConfig {
    /// Runtime data directory. Managed runtime binaries live below this path.
    pub app_data_dir: String,
    /// Pantograph project root. Used to resolve local workflow/runtime assets.
    pub project_root: String,
    /// Directories containing persisted Pantograph workflow JSON files.
    ///
    /// If empty, the runtime uses `<project_root>/.pantograph/workflows`.
    pub workflow_roots: Vec<String>,
}

/// Native embedded Pantograph runtime.
///
/// This is the direct workflow/session facade for language bindings. Methods
/// accept and return the same JSON service DTOs as `pantograph-workflow-service`;
/// no `base_url` or HTTP transport is involved.
#[derive(uniffi::Object)]
pub struct FfiPantographRuntime {
    runtime: Arc<EmbeddedRuntime>,
}

#[uniffi::export(async_runtime = "tokio")]
impl FfiPantographRuntime {
    /// Create an embedded runtime with the default inference gateway and Python adapter.
    #[uniffi::constructor]
    pub async fn new(
        config: FfiEmbeddedRuntimeConfig,
        pumas_api: Option<Arc<FfiPumasApi>>,
    ) -> Result<Arc<Self>, FfiError> {
        let config = to_embedded_config(config)?;
        std::fs::create_dir_all(&config.app_data_dir).map_err(|e| {
            workflow_adapter_error(
                WorkflowErrorCode::InvalidRequest,
                format!(
                    "failed to create app_data_dir '{}': {}",
                    config.app_data_dir.display(),
                    e
                ),
            )
        })?;

        let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        if let Some(api) = pumas_api {
            extensions
                .write()
                .await
                .set(node_engine::extension_keys::PUMAS_API, api.api_arc());
        }

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            config,
            Arc::new(inference::InferenceGateway::new()),
            extensions,
            Arc::new(WorkflowService::new()),
            None,
        );

        Ok(Arc::new(Self {
            runtime: Arc::new(runtime),
        }))
    }

    /// Stop inference backends owned by this runtime.
    pub async fn shutdown(&self) {
        self.runtime.shutdown().await;
    }

    /// Run a workflow and return WorkflowRunResponse JSON.
    pub async fn workflow_run(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowRunRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_run(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowCapabilitiesResponse JSON.
    pub async fn workflow_get_capabilities(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowCapabilitiesRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_get_capabilities(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowIoResponse JSON.
    pub async fn workflow_get_io(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowIoRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_get_io(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Run workflow preflight and return WorkflowPreflightResponse JSON.
    pub async fn workflow_preflight(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowPreflightRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_preflight(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Create a workflow session and return WorkflowSessionCreateResponse JSON.
    pub async fn workflow_create_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowSessionCreateRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .create_workflow_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Run an existing workflow session and return WorkflowRunResponse JSON.
    pub async fn workflow_run_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowSessionRunRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .run_workflow_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Close a workflow session and return WorkflowSessionCloseResponse JSON.
    pub async fn workflow_close_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowSessionCloseRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .close_workflow_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowSessionStatusResponse JSON.
    pub async fn workflow_get_session_status(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowSessionStatusRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_get_session_status(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowSessionQueueListResponse JSON.
    pub async fn workflow_list_session_queue(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowSessionQueueListRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_list_session_queue(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Cancel a queued workflow-session run and return WorkflowSessionQueueCancelResponse JSON.
    pub async fn workflow_cancel_session_queue_item(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowSessionQueueCancelRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_cancel_session_queue_item(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Reprioritize a queued workflow-session run and return response JSON.
    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowSessionQueueReprioritizeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_reprioritize_session_queue_item(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Update workflow-session keep-alive state and return WorkflowSessionKeepAliveResponse JSON.
    pub async fn workflow_set_session_keep_alive(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowSessionKeepAliveRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_set_session_keep_alive(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }
}

fn to_embedded_config(config: FfiEmbeddedRuntimeConfig) -> Result<EmbeddedRuntimeConfig, FfiError> {
    let app_data_dir = non_empty_path(config.app_data_dir, "app_data_dir")?;
    let project_root = non_empty_path(config.project_root, "project_root")?;
    let workflow_roots = if config.workflow_roots.is_empty() {
        vec![project_root.join(".pantograph").join("workflows")]
    } else {
        config
            .workflow_roots
            .into_iter()
            .map(|root| non_empty_path(root, "workflow_roots"))
            .collect::<Result<Vec<_>, _>>()?
    };

    Ok(EmbeddedRuntimeConfig {
        app_data_dir,
        project_root,
        workflow_roots,
    })
}

fn non_empty_path(value: String, field_name: &str) -> Result<PathBuf, FfiError> {
    if value.trim().is_empty() {
        return Err(workflow_adapter_error(
            WorkflowErrorCode::InvalidRequest,
            format!("{field_name} must not be empty"),
        ));
    }
    Ok(PathBuf::from(value))
}

fn parse_request<T>(request_json: String) -> Result<T, FfiError>
where
    T: DeserializeOwned,
{
    serde_json::from_str(&request_json).map_err(|e| {
        workflow_adapter_error(
            WorkflowErrorCode::InvalidRequest,
            format!("invalid request: {}", e),
        )
    })
}

fn serialize_response<T>(response: &T) -> Result<String, FfiError>
where
    T: Serialize,
{
    serde_json::to_string(response).map_err(|e| {
        workflow_adapter_error(
            WorkflowErrorCode::InternalError,
            format!("response serialization error: {}", e),
        )
    })
}

fn map_workflow_service_error(err: WorkflowServiceError) -> FfiError {
    FfiError::Other {
        message: err.to_envelope_json(),
    }
}

fn workflow_error_json(code: WorkflowErrorCode, message: impl Into<String>) -> String {
    let envelope = WorkflowErrorEnvelope {
        code,
        message: message.into(),
    };
    serde_json::to_string(&envelope).unwrap_or_else(|_| {
        r#"{"code":"internal_error","message":"failed to serialize workflow error envelope"}"#
            .to_string()
    })
}

fn workflow_adapter_error(code: WorkflowErrorCode, message: impl Into<String>) -> FfiError {
    FfiError::Other {
        message: workflow_error_json(code, message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_root(workflow_id: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pantograph-uniffi-runtime-tests-{suffix}"));
        write_test_workflow(&root, workflow_id);
        install_fake_default_runtime(&root.join("app-data"));
        root
    }

    fn install_fake_default_runtime(app_data_dir: &Path) {
        let runtime_dir = app_data_dir.join("runtimes").join("llama-cpp");
        std::fs::create_dir_all(&runtime_dir).expect("create fake runtime dir");

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let file_names = [
            "llama-server-x86_64-unknown-linux-gnu",
            "libllama.so",
            "libggml.so",
        ];
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let file_names = ["llama-server-aarch64-apple-darwin", "libllama.dylib"];
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let file_names = ["llama-server-x86_64-apple-darwin", "libllama.dylib"];
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let file_names = [
            "llama-server-x86_64-pc-windows-msvc.exe",
            "llama-runtime.dll",
        ];

        for file_name in file_names {
            std::fs::write(runtime_dir.join(file_name), [])
                .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
        }
    }

    fn write_test_workflow(root: &Path, workflow_id: &str) {
        let workflows_dir = root.join(".pantograph").join("workflows");
        std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
        let workflow_json = serde_json::json!({
            "version": "1.0",
            "metadata": {
                "name": "Test Workflow",
                "created": "2026-01-01T00:00:00Z",
                "modified": "2026-01-01T00:00:00Z"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "name": "Prompt",
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Text Input",
                                "description": "Provides text input",
                                "inputs": [{
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }],
                                "outputs": [{
                                    "id": "legacy-out",
                                    "label": "Legacy Out",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }]
                            },
                            "text": "hello"
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    },
                    {
                        "id": "text-output-1",
                        "node_type": "text-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "label": "Text Output",
                                "description": "Displays text output",
                                "inputs": [{
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }],
                                "outputs": [{
                                    "id": "text",
                                    "label": "Text",
                                    "data_type": "string",
                                    "required": false,
                                    "multiple": false
                                }]
                            }
                        },
                        "position": { "x": 200.0, "y": 0.0 }
                    }
                ],
                "edges": [{
                    "id": "e-text",
                    "source": "text-input-1",
                    "source_handle": "text",
                    "target": "text-output-1",
                    "target_handle": "text"
                }]
            }
        });
        std::fs::write(
            workflows_dir.join(format!("{workflow_id}.json")),
            serde_json::to_vec(&workflow_json).expect("serialize workflow"),
        )
        .expect("write workflow");
    }

    #[tokio::test]
    async fn direct_runtime_runs_workflow_and_session_from_json() {
        let workflow_id = "uniffi-runtime-text";
        let root = create_temp_root(workflow_id);

        let runtime = FfiPantographRuntime::new(
            FfiEmbeddedRuntimeConfig {
                app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
                project_root: root.to_string_lossy().into_owned(),
                workflow_roots: Vec::new(),
            },
            None,
        )
        .await
        .expect("runtime");

        let run_response_json = runtime
            .workflow_run(
                serde_json::json!({
                    "workflow_id": workflow_id,
                    "inputs": [{
                        "node_id": "text-input-1",
                        "port_id": "text",
                        "value": "direct run"
                    }],
                    "output_targets": [{
                        "node_id": "text-output-1",
                        "port_id": "text"
                    }],
                    "run_id": "run-1"
                })
                .to_string(),
            )
            .await
            .expect("workflow run");
        let run_response: serde_json::Value =
            serde_json::from_str(&run_response_json).expect("parse run response");
        assert_eq!(run_response["outputs"][0]["value"], "direct run");

        let create_response_json = runtime
            .workflow_create_session(
                serde_json::json!({
                    "workflow_id": workflow_id,
                    "keep_alive": false
                })
                .to_string(),
            )
            .await
            .expect("create session");
        let session_id = serde_json::from_str::<serde_json::Value>(&create_response_json)
            .expect("parse create response")["session_id"]
            .as_str()
            .expect("session_id")
            .to_string();

        let session_response_json = runtime
            .workflow_run_session(
                serde_json::json!({
                    "session_id": session_id,
                    "inputs": [{
                        "node_id": "text-input-1",
                        "port_id": "text",
                        "value": "session run"
                    }],
                    "output_targets": [{
                        "node_id": "text-output-1",
                        "port_id": "text"
                    }]
                })
                .to_string(),
            )
            .await
            .expect("run session");
        let session_response: serde_json::Value =
            serde_json::from_str(&session_response_json).expect("parse session response");
        assert_eq!(session_response["outputs"][0]["value"], "session run");

        runtime
            .workflow_close_session(serde_json::json!({ "session_id": session_id }).to_string())
            .await
            .expect("close session");
        runtime.shutdown().await;

        let _ = std::fs::remove_dir_all(root);
    }
}
