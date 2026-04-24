use std::path::PathBuf;
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, ClientRegistrationRequest, ClientSessionOpenRequest,
    ClientSessionResumeRequest, WorkflowAttributedRunRequest, WorkflowCapabilitiesRequest,
    WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowGraphAddEdgeRequest,
    WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphUndoRedoStateRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest, WorkflowIoRequest,
    WorkflowPreflightRequest, WorkflowRunRequest, WorkflowService, WorkflowServiceError,
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

        let workflow_service = Arc::new(
            WorkflowService::with_ephemeral_attribution_store()
                .map_err(map_workflow_service_error)?,
        );
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

    /// Register an attribution client and return ClientRegistrationResponse JSON.
    pub fn workflow_register_attribution_client(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: ClientRegistrationRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .register_attribution_client(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Open a durable client session and return ClientSessionOpenResponse JSON.
    pub fn workflow_open_client_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: ClientSessionOpenRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .open_client_session(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Resume a durable client session and return ClientSessionRecord JSON.
    pub fn workflow_resume_client_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: ClientSessionResumeRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .resume_client_session(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Create a durable client bucket and return BucketRecord JSON.
    pub fn workflow_create_client_bucket(&self, request_json: String) -> Result<String, FfiError> {
        let request: BucketCreateRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .create_client_bucket(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Delete a durable client bucket and return BucketRecord JSON.
    pub fn workflow_delete_client_bucket(&self, request_json: String) -> Result<String, FfiError> {
        let request: BucketDeleteRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .delete_client_bucket(request)
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Run a workflow with durable attribution and return WorkflowAttributedRunResponse JSON.
    pub async fn workflow_run_attributed(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowAttributedRunRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_run_attributed(request)
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
#[path = "runtime_tests.rs"]
mod runtime_tests;
