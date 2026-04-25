use std::path::PathBuf;
use std::sync::Arc;

use node_engine::ExecutorExtensions;
use pantograph_embedded_runtime::{EmbeddedRuntime, EmbeddedRuntimeConfig};
use pantograph_workflow_service::{
    BucketCreateRequest, BucketDeleteRequest, ClientRegistrationRequest, ClientSessionOpenRequest,
    ClientSessionResumeRequest, NodeRegistry as WorkflowNodeRegistry, WorkflowCapabilitiesRequest,
    WorkflowErrorCode, WorkflowErrorEnvelope, WorkflowExecutionSessionCloseRequest,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionKeepAliveRequest,
    WorkflowExecutionSessionQueueCancelRequest, WorkflowExecutionSessionQueueListRequest,
    WorkflowExecutionSessionQueueReprioritizeRequest, WorkflowExecutionSessionRunRequest,
    WorkflowExecutionSessionStatusRequest, WorkflowGraphAddEdgeRequest,
    WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphUndoRedoStateRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest, WorkflowIoRequest,
    WorkflowPreflightRequest, WorkflowService, WorkflowServiceError,
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
    node_registry: Arc<node_engine::NodeRegistry>,
    extensions: Arc<RwLock<ExecutorExtensions>>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct QueryablePortProjection<'a> {
    node_type: &'a str,
    port_id: &'a str,
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
            extensions.clone(),
            workflow_service,
            None,
        );

        Ok(Arc::new(Self {
            runtime: Arc::new(runtime),
            node_registry: Arc::new(node_engine::NodeRegistry::with_builtins()),
            extensions,
        }))
    }

    /// Stop inference backends owned by this runtime.
    pub async fn shutdown(&self) {
        self.runtime.shutdown().await;
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

    /// Create a workflow execution session and return WorkflowExecutionSessionCreateResponse JSON.
    pub async fn workflow_create_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionCreateRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .create_workflow_execution_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Run work inside a workflow execution session and return WorkflowRunResponse JSON.
    pub async fn workflow_run_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionRunRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .run_workflow_execution_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Close a workflow execution session and return WorkflowExecutionSessionCloseResponse JSON.
    pub async fn workflow_close_session(&self, request_json: String) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionCloseRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .close_workflow_execution_session(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowExecutionSessionStatusResponse JSON.
    pub async fn workflow_get_session_status(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionStatusRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_get_execution_session_status(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Return WorkflowExecutionSessionQueueListResponse JSON.
    pub async fn workflow_list_session_queue(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionQueueListRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_list_execution_session_queue(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Cancel a queued workflow execution-session item and return WorkflowExecutionSessionQueueCancelResponse JSON.
    pub async fn workflow_cancel_session_queue_item(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionQueueCancelRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_cancel_execution_session_queue_item(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Reprioritize a queued workflow execution-session item and return WorkflowExecutionSessionQueueReprioritizeResponse JSON.
    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionQueueReprioritizeRequest =
            parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_reprioritize_execution_session_queue_item(request)
            .await
            .map_err(map_workflow_service_error)?;
        serialize_response(&response)
    }

    /// Update workflow execution-session keep-alive policy and return WorkflowExecutionSessionKeepAliveResponse JSON.
    pub async fn workflow_set_session_keep_alive(
        &self,
        request_json: String,
    ) -> Result<String, FfiError> {
        let request: WorkflowExecutionSessionKeepAliveRequest = parse_request(request_json)?;
        let response = self
            .runtime
            .workflow_set_execution_session_keep_alive(request)
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

    /// Return registry-backed NodeDefinition JSON array for graph authoring.
    pub fn workflow_graph_list_node_definitions(&self) -> Result<String, FfiError> {
        let registry = WorkflowNodeRegistry::new();
        serialize_response(&registry.all_definitions())
    }

    /// Return one registry-backed NodeDefinition JSON object by node type.
    pub fn workflow_graph_get_node_definition(
        &self,
        node_type: String,
    ) -> Result<String, FfiError> {
        let registry = WorkflowNodeRegistry::new();
        let definition = registry.get_definition(&node_type).ok_or_else(|| {
            workflow_adapter_error(
                WorkflowErrorCode::InvalidRequest,
                format!("unknown node_type '{node_type}'"),
            )
        })?;
        serialize_response(definition)
    }

    /// Return registry-backed NodeDefinition JSON grouped by category.
    pub fn workflow_graph_get_node_definitions_by_category(&self) -> Result<String, FfiError> {
        let registry = WorkflowNodeRegistry::new();
        serialize_response(&registry.definitions_by_category())
    }

    /// Return backend-owned queryable `(node_type, port_id)` pairs as JSON.
    pub fn workflow_graph_get_queryable_ports(&self) -> Result<String, FfiError> {
        let ports = self
            .node_registry
            .queryable_ports()
            .into_iter()
            .map(|(node_type, port_id)| QueryablePortProjection { node_type, port_id })
            .collect::<Vec<_>>();
        serialize_response(&ports)
    }

    /// Query backend-owned port options and return PortOptionsResult JSON.
    pub async fn workflow_graph_query_port_options(
        &self,
        node_type: String,
        port_id: String,
        query_json: String,
    ) -> Result<String, FfiError> {
        let query: node_engine::PortOptionsQuery = parse_request(query_json)?;
        let extensions = self.extensions.read().await;
        let result = self
            .node_registry
            .query_port_options(&node_type, &port_id, &query, &extensions)
            .await
            .map_err(map_node_engine_error)?;
        serialize_response(&result)
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

fn map_node_engine_error(err: node_engine::NodeEngineError) -> FfiError {
    workflow_adapter_error(WorkflowErrorCode::InvalidRequest, err.to_string())
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
