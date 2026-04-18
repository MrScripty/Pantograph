use std::path::PathBuf;
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowErrorCode, WorkflowErrorEnvelope,
    WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest,
    WorkflowGraphEditSessionCloseRequest, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest,
    WorkflowGraphLoadRequest, WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUpdateNodeDataRequest,
    WorkflowGraphUpdateNodePositionRequest, WorkflowIoRequest, WorkflowPreflightRequest,
    WorkflowRunRequest, WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCreateRequest, WorkflowSessionKeepAliveRequest,
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
    /// Optional limit on how many session runtimes may remain loaded at once.
    pub max_loaded_sessions: Option<u64>,
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

        let workflow_service = Arc::new(WorkflowService::new());
        workflow_service
            .set_loaded_runtime_capacity_limit(config.max_loaded_sessions)
            .map_err(map_workflow_service_error)?;

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            config,
            Arc::new(inference::InferenceGateway::new()),
            extensions,
            workflow_service,
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

    /// Save a workflow graph to the runtime project and return WorkflowGraphSaveResponse JSON.
    pub fn workflow_graph_save(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphSaveRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_save(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Load a workflow graph file and return WorkflowFile JSON.
    pub fn workflow_graph_load(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphLoadRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_load(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// List persisted project workflows and return WorkflowGraphListResponse JSON.
    pub fn workflow_graph_list(&self) -> Result<String, FfiError> {
        let response = self
            .runtime
            .workflow_graph_list()
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Start an in-memory graph edit session and return WorkflowGraphEditSessionCreateResponse JSON.
    pub async fn workflow_graph_create_edit_session(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphEditSessionCreateRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_create_edit_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Close a graph edit session and return WorkflowGraphEditSessionCloseResponse JSON.
    pub async fn workflow_graph_close_edit_session(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphEditSessionCloseRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_close_edit_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return the current graph edit-session snapshot as WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_get_edit_session_graph(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphEditSessionGraphRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_get_edit_session_graph(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowGraphUndoRedoStateResponse JSON for a graph edit session.
    pub async fn workflow_graph_get_undo_redo_state(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphUndoRedoStateRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_get_undo_redo_state(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Merge node data in a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_update_node_data(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphUpdateNodeDataRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_update_node_data(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Update node position in a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_update_node_position(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphUpdateNodePositionRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_update_node_position(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Add a node to a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_add_node(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphAddNodeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_add_node(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Remove a node from a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_remove_node(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphRemoveNodeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_remove_node(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Add an edge to a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_add_edge(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphAddEdgeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_add_edge(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Remove an edge from a graph edit session and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_remove_edge(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphRemoveEdgeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_remove_edge(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Undo a graph edit and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_undo(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphEditSessionGraphRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_undo(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Redo a graph edit and return WorkflowGraphEditSessionGraphResponse JSON.
    pub async fn workflow_graph_redo(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphEditSessionGraphRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_redo(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return ConnectionCandidatesResponse JSON for a graph edit session anchor.
    pub async fn workflow_graph_get_connection_candidates(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphGetConnectionCandidatesRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_get_connection_candidates(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Connect two compatible anchors and return ConnectionCommitResponse JSON.
    pub async fn workflow_graph_connect(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowGraphConnectRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_connect(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Insert a node and connect it from an anchor; returns InsertNodeConnectionResponse JSON.
    pub async fn workflow_graph_insert_node_and_connect(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphInsertNodeAndConnectRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_insert_node_and_connect(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Preview node insertion on an edge and return EdgeInsertionPreviewResponse JSON.
    pub async fn workflow_graph_preview_node_insert_on_edge(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphPreviewNodeInsertOnEdgeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_preview_node_insert_on_edge(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Insert a node on an edge and return InsertNodeOnEdgeResponse JSON.
    pub async fn workflow_graph_insert_node_on_edge(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowGraphInsertNodeOnEdgeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_graph_insert_node_on_edge(request)
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
        max_loaded_sessions: config.max_loaded_sessions.map(|value| value as usize),
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
        details: None,
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

    fn write_human_input_workflow(root: &Path, workflow_id: &str) {
        let workflows_dir = root.join(".pantograph").join("workflows");
        std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
        let workflow_json = serde_json::json!({
            "version": "1.0",
            "metadata": {
                "name": "Interactive Workflow",
                "created": "2026-01-01T00:00:00Z",
                "modified": "2026-01-01T00:00:00Z"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "human-input-1",
                        "node_type": "human-input",
                        "data": {
                            "prompt": "Approve deployment?",
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Human Input",
                                "description": "Pauses workflow to wait for interactive input",
                                "inputs": [
                                    {
                                        "id": "prompt",
                                        "label": "Prompt",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "default",
                                        "label": "Default Value",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "auto_accept",
                                        "label": "Auto Accept",
                                        "data_type": "boolean",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "user_response",
                                        "label": "User Response",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "value",
                                        "label": "Value",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
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
                        "position": { "x": 240.0, "y": 0.0 }
                    }
                ],
                "edges": [{
                    "id": "e-human-output",
                    "source": "human-input-1",
                    "source_handle": "value",
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

    fn workflow_error_envelope(err: FfiError) -> WorkflowErrorEnvelope {
        let message = match err {
            FfiError::Other { message } => message,
            other => panic!("expected FfiError::Other with envelope JSON, got {other:?}"),
        };
        serde_json::from_str(&message).expect("parse workflow error envelope")
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
                max_loaded_sessions: None,
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

    #[tokio::test]
    async fn direct_runtime_workflow_run_preserves_invalid_request_envelope() {
        let workflow_id = "uniffi-runtime-interactive-run";
        let root = create_temp_root(workflow_id);
        write_human_input_workflow(&root, workflow_id);

        let runtime = FfiPantographRuntime::new(
            FfiEmbeddedRuntimeConfig {
                app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
                project_root: root.to_string_lossy().into_owned(),
                workflow_roots: Vec::new(),
                max_loaded_sessions: None,
            },
            None,
        )
        .await
        .expect("runtime");

        let err = runtime
            .workflow_run(
                serde_json::json!({
                    "workflow_id": workflow_id,
                    "inputs": [],
                    "output_targets": [{
                        "node_id": "text-output-1",
                        "port_id": "text"
                    }],
                    "run_id": "run-human-input"
                })
                .to_string(),
            )
            .await
            .expect_err("interactive workflow run should preserve invalid-request envelope");

        let envelope = workflow_error_envelope(err);
        assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
        assert_eq!(
            envelope.message,
            "workflow 'uniffi-runtime-interactive-run' requires interactive input at node 'human-input-1'"
        );

        runtime.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn direct_runtime_workflow_run_session_preserves_invalid_request_envelope() {
        let workflow_id = "uniffi-runtime-interactive-session";
        let root = create_temp_root(workflow_id);
        write_human_input_workflow(&root, workflow_id);

        let runtime = FfiPantographRuntime::new(
            FfiEmbeddedRuntimeConfig {
                app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
                project_root: root.to_string_lossy().into_owned(),
                workflow_roots: Vec::new(),
                max_loaded_sessions: None,
            },
            None,
        )
        .await
        .expect("runtime");

        let create_response_json = runtime
            .workflow_create_session(
                serde_json::json!({
                    "workflow_id": workflow_id,
                    "usage_profile": "interactive",
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

        let err = runtime
            .workflow_run_session(
                serde_json::json!({
                    "session_id": session_id,
                    "inputs": [],
                    "output_targets": [{
                        "node_id": "text-output-1",
                        "port_id": "text"
                    }],
                    "run_id": "run-human-input-session"
                })
                .to_string(),
            )
            .await
            .expect_err("interactive session run should preserve invalid-request envelope");

        let envelope = workflow_error_envelope(err);
        assert_eq!(envelope.code, WorkflowErrorCode::InvalidRequest);
        assert_eq!(
            envelope.message,
            "workflow 'uniffi-runtime-interactive-session' requires interactive input at node 'human-input-1'"
        );

        runtime
            .workflow_close_session(serde_json::json!({ "session_id": session_id }).to_string())
            .await
            .expect("close session");
        runtime.shutdown().await;
        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn direct_runtime_exposes_workflow_graph_persistence_and_edit_session() {
        let root = create_temp_root("uniffi-runtime-unused");
        let runtime = FfiPantographRuntime::new(
            FfiEmbeddedRuntimeConfig {
                app_data_dir: root.join("app-data").to_string_lossy().into_owned(),
                project_root: root.to_string_lossy().into_owned(),
                workflow_roots: Vec::new(),
                max_loaded_sessions: None,
            },
            None,
        )
        .await
        .expect("runtime");

        let graph = serde_json::json!({
            "nodes": [{
                "id": "text-input-1",
                "node_type": "text-input",
                "position": { "x": 0.0, "y": 0.0 },
                "data": { "text": "draft" }
            }],
            "edges": []
        });
        let save_response_json = runtime
            .workflow_graph_save(
                serde_json::json!({
                    "name": "Native Edited Workflow",
                    "graph": graph
                })
                .to_string(),
            )
            .expect("save workflow graph");
        let save_response: serde_json::Value =
            serde_json::from_str(&save_response_json).expect("parse save response");
        let path = save_response["path"].as_str().expect("saved path");

        let list_response_json = runtime.workflow_graph_list().expect("list workflow graphs");
        let list_response: serde_json::Value =
            serde_json::from_str(&list_response_json).expect("parse list response");
        assert!(list_response["workflows"]
            .as_array()
            .expect("workflows")
            .iter()
            .any(|metadata| metadata["id"] == "Native Edited Workflow"));

        let load_response_json = runtime
            .workflow_graph_load(serde_json::json!({ "path": path }).to_string())
            .expect("load workflow graph");
        let load_response: serde_json::Value =
            serde_json::from_str(&load_response_json).expect("parse load response");
        assert_eq!(load_response["metadata"]["name"], "Native Edited Workflow");

        let create_response_json = runtime
            .workflow_graph_create_edit_session(
                serde_json::json!({
                    "graph": load_response["graph"]
                })
                .to_string(),
            )
            .await
            .expect("create graph edit session");
        let create_response: serde_json::Value =
            serde_json::from_str(&create_response_json).expect("parse create response");
        let edit_session_id = create_response["session_id"]
            .as_str()
            .expect("edit session id");

        let update_response_json = runtime
            .workflow_graph_update_node_data(
                serde_json::json!({
                    "session_id": edit_session_id,
                    "node_id": "text-input-1",
                    "data": { "text": "native edit" }
                })
                .to_string(),
            )
            .await
            .expect("update node data");
        let update_response: serde_json::Value =
            serde_json::from_str(&update_response_json).expect("parse update response");
        assert_eq!(
            update_response["graph"]["nodes"][0]["data"]["text"],
            "native edit"
        );
        assert_eq!(update_response["workflow_event"]["type"], "graphModified");
        assert_eq!(
            update_response["workflow_event"]["dirtyTasks"],
            serde_json::json!(["text-input-1"])
        );

        let undo_state_json = runtime
            .workflow_graph_get_undo_redo_state(
                serde_json::json!({ "session_id": edit_session_id }).to_string(),
            )
            .await
            .expect("undo-redo state");
        let undo_state: serde_json::Value =
            serde_json::from_str(&undo_state_json).expect("parse undo-redo state");
        assert_eq!(undo_state["can_undo"], true);

        let undo_response_json = runtime
            .workflow_graph_undo(serde_json::json!({ "session_id": edit_session_id }).to_string())
            .await
            .expect("undo graph edit");
        let undo_response: serde_json::Value =
            serde_json::from_str(&undo_response_json).expect("parse undo response");
        assert_eq!(undo_response["graph"]["nodes"][0]["data"]["text"], "draft");
        assert_eq!(undo_response["workflow_event"]["type"], "graphModified");

        runtime
            .workflow_graph_close_edit_session(
                serde_json::json!({ "session_id": edit_session_id }).to_string(),
            )
            .await
            .expect("close graph edit session");
        runtime.shutdown().await;

        let _ = std::fs::remove_dir_all(root);
    }
}
