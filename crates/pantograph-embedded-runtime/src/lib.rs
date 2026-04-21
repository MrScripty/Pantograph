use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
#[cfg(any(test, feature = "standalone"))]
use node_engine::ExecutorExtensions;
use node_engine::{CoreTaskExecutor, EventSink, NullEventSink, WorkflowExecutor, WorkflowGraph};
use pantograph_runtime_identity::canonical_runtime_backend_key;
use pantograph_runtime_registry::{
    RuntimeRegistryError, RuntimeReservationRequirements, RuntimeRetentionHint,
    SharedRuntimeRegistry,
};
use pantograph_workflow_service::capabilities;
use pantograph_workflow_service::graph::WorkflowGraphSessionStateView;
use pantograph_workflow_service::{
    ConnectionCandidatesResponse, ConnectionCommitResponse, EdgeInsertionPreviewResponse,
    FileSystemWorkflowGraphStore, InsertNodeConnectionResponse, InsertNodeOnEdgeResponse,
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowFile,
    WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest, WorkflowGraphConnectRequest,
    WorkflowGraphEditSessionCloseRequest, WorkflowGraphEditSessionCloseResponse,
    WorkflowGraphEditSessionCreateRequest, WorkflowGraphEditSessionCreateResponse,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphEditSessionGraphResponse,
    WorkflowGraphGetConnectionCandidatesRequest, WorkflowGraphInsertNodeAndConnectRequest,
    WorkflowGraphInsertNodeOnEdgeRequest, WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphSaveResponse,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest, WorkflowHost,
    WorkflowHostModelDescriptor, WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunOptions,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeCapability,
    WorkflowRuntimeRequirements, WorkflowService, WorkflowServiceError,
    WorkflowSessionCloseRequest, WorkflowSessionCloseResponse, WorkflowSessionCreateRequest,
    WorkflowSessionCreateResponse, WorkflowSessionInspectionRequest,
    WorkflowSessionInspectionResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRetentionHint,
    WorkflowSessionRunRequest, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowSessionStaleCleanupRequest,
    WorkflowSessionStaleCleanupResponse, WorkflowSessionState, WorkflowSessionStatusRequest,
    WorkflowSessionStatusResponse, WorkflowTechnicalFitDecision, WorkflowTechnicalFitRequest,
    WorkflowTraceRuntimeMetrics, convert_graph_to_node_engine,
};
#[cfg(test)]
use pantograph_workflow_service::{
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerRuntimeRegistryDiagnostics,
};
#[cfg(any(test, feature = "standalone"))]
use tokio::sync::RwLock;
use uuid::Uuid;

mod embedded_runtime_lifecycle;
pub mod embedding_workflow;
pub mod host_runtime;
pub mod managed_runtime_manager;
pub mod model_dependencies;
pub mod python_runtime;
mod python_runtime_execution;
pub mod rag;
pub mod runtime_capabilities;
mod runtime_config;
mod runtime_extensions;
pub mod runtime_health;
pub mod runtime_recovery;
pub mod runtime_registry;
mod runtime_registry_controller;
mod runtime_registry_errors;
mod runtime_registry_lifecycle;
mod runtime_registry_observations;
pub mod task_executor;
pub mod technical_fit;
pub mod workflow_runtime;
mod workflow_scheduler_diagnostics;
mod workflow_session_execution;

pub use host_runtime::HostRuntimeModeSnapshot;
pub use managed_runtime_manager::{
    ManagedRuntimeManagerProgress, ManagedRuntimeManagerRuntimeView,
    cancel_managed_runtime_manager_job, inspect_managed_runtime_manager_runtime,
    install_managed_runtime_manager_runtime, list_managed_runtime_manager_runtimes,
    pause_managed_runtime_manager_job, refresh_managed_runtime_manager_catalog_views,
    remove_managed_runtime_manager_runtime, select_managed_runtime_manager_version,
    set_default_managed_runtime_manager_version_view,
};
pub use model_dependencies::{SharedModelDependencyResolver, TauriModelDependencyResolver};
pub use python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
    PythonStreamHandler,
};
pub use rag::{RagBackend, RagDocument};
#[cfg(feature = "standalone")]
pub use runtime_config::StandaloneRuntimeConfig;
pub use runtime_config::{EmbeddedRuntimeConfig, EmbeddedRuntimeError};
pub use runtime_extensions::{
    RuntimeExtensionsSnapshot, SharedExtensions, apply_runtime_extensions,
    apply_runtime_extensions_for_execution,
};
pub use task_executor::{TauriTaskExecutor as PantographTaskExecutor, runtime_extension_keys};
pub(crate) use workflow_scheduler_diagnostics::EmbeddedWorkflowSchedulerDiagnosticsProvider;

pub type SharedWorkflowService = Arc<WorkflowService>;

const RUNTIME_WARMUP_POLL_INTERVAL_MS: u64 = 25;

#[cfg(not(test))]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 5_000;

#[cfg(test)]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 250;

pub struct EmbeddedRuntime {
    config: EmbeddedRuntimeConfig,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    workflow_service: SharedWorkflowService,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_executions: Arc<workflow_session_execution::WorkflowSessionExecutionStore>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

#[derive(Debug, Clone)]
pub struct EditSessionGraphExecutionOutcome {
    pub runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
    pub waiting_for_input: bool,
    pub error: Option<String>,
}

impl EmbeddedRuntime {
    fn graph_store(&self) -> FileSystemWorkflowGraphStore {
        FileSystemWorkflowGraphStore::new(self.config.project_root.clone())
    }

    pub async fn workflow_run(
        &self,
        request: WorkflowRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_run(&self.host(), request)
            .await
    }

    pub async fn workflow_get_capabilities(
        &self,
        request: WorkflowCapabilitiesRequest,
    ) -> Result<WorkflowCapabilitiesResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_capabilities(&self.host(), request)
            .await
    }

    pub async fn workflow_get_io(
        &self,
        request: WorkflowIoRequest,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_io(&self.host(), request)
            .await
    }

    pub async fn workflow_preflight(
        &self,
        request: WorkflowPreflightRequest,
    ) -> Result<WorkflowPreflightResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_preflight(&self.host(), request)
            .await
    }

    pub async fn create_workflow_session(
        &self,
        request: WorkflowSessionCreateRequest,
    ) -> Result<WorkflowSessionCreateResponse, WorkflowServiceError> {
        self.workflow_service
            .create_workflow_session(&self.host(), request)
            .await
    }

    pub async fn run_workflow_session(
        &self,
        request: WorkflowSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.workflow_service
            .run_workflow_session(&self.host(), request)
            .await
    }

    pub async fn close_workflow_session(
        &self,
        request: WorkflowSessionCloseRequest,
    ) -> Result<WorkflowSessionCloseResponse, WorkflowServiceError> {
        self.workflow_service
            .close_workflow_session(&self.host(), request)
            .await
    }

    pub async fn workflow_get_session_status(
        &self,
        request: WorkflowSessionStatusRequest,
    ) -> Result<WorkflowSessionStatusResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_get_session_status(request)
            .await
    }

    pub async fn workflow_get_session_inspection(
        &self,
        request: WorkflowSessionInspectionRequest,
    ) -> Result<WorkflowSessionInspectionResponse, WorkflowServiceError> {
        let host = self.host();
        self.workflow_service
            .workflow_get_session_inspection(&host, request)
            .await
    }

    pub async fn workflow_list_session_queue(
        &self,
        request: WorkflowSessionQueueListRequest,
    ) -> Result<WorkflowSessionQueueListResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_list_session_queue(request)
            .await
    }

    pub async fn workflow_cleanup_stale_sessions(
        &self,
        request: WorkflowSessionStaleCleanupRequest,
    ) -> Result<WorkflowSessionStaleCleanupResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cleanup_stale_sessions(request)
            .await
    }

    pub async fn workflow_cancel_session_queue_item(
        &self,
        request: WorkflowSessionQueueCancelRequest,
    ) -> Result<WorkflowSessionQueueCancelResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_cancel_session_queue_item(request)
            .await
    }

    pub async fn workflow_reprioritize_session_queue_item(
        &self,
        request: WorkflowSessionQueueReprioritizeRequest,
    ) -> Result<WorkflowSessionQueueReprioritizeResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_reprioritize_session_queue_item(request)
            .await
    }

    pub async fn workflow_set_session_keep_alive(
        &self,
        request: WorkflowSessionKeepAliveRequest,
    ) -> Result<WorkflowSessionKeepAliveResponse, WorkflowServiceError> {
        let host = self.host();
        let response = self
            .workflow_service
            .workflow_set_session_keep_alive(&host, request)
            .await?;
        host.sync_loaded_session_runtime_retention_hint(
            &response.session_id,
            response.keep_alive,
            response.state,
        )?;
        Ok(response)
    }

    pub async fn execute_data_graph(
        &self,
        graph_id: &str,
        graph: &WorkflowGraph,
        inputs: &HashMap<String, serde_json::Value>,
        event_sink: Arc<dyn EventSink>,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&self.extensions).await;
        let execution_id = format!("data-graph-{}-{}", graph_id, Uuid::new_v4());
        let workflow_event_sink = event_sink.clone();
        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.config.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_event_sink(event_sink.clone())
                .with_execution_id(execution_id.clone()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );
        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());
        let mut graph = graph.clone();
        let terminal_nodes = EmbeddedWorkflowHost::terminal_data_graph_node_ids(&graph);
        EmbeddedWorkflowHost::apply_data_graph_inputs(&mut graph, inputs);

        let mut executor = WorkflowExecutor::new(execution_id.clone(), graph, event_sink);
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            Some(workflow_event_sink),
            Some(execution_id),
            Some(python_runtime_execution_recorder.clone()),
        );

        let mut node_outputs = HashMap::new();
        let mut terminal_errors = HashMap::new();
        let mut terminal_outcome_error = None;
        for terminal_id in &terminal_nodes {
            match executor.demand(terminal_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(terminal_id.clone(), outputs);
                }
                Err(error) => {
                    if matches!(
                        error,
                        node_engine::NodeEngineError::WaitingForInput { .. }
                            | node_engine::NodeEngineError::Cancelled
                    ) {
                        terminal_outcome_error = Some(error);
                        break;
                    }
                    log::error!(
                        "Error executing terminal node '{}' in data graph '{}': {}",
                        terminal_id,
                        graph_id,
                        error
                    );
                    terminal_errors.insert(
                        format!("{}.error", terminal_id),
                        serde_json::Value::String(error.to_string()),
                    );
                }
            }
        }

        let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshots();
        self.host()
            .observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)
            .map_err(|error| node_engine::NodeEngineError::failed(error.to_string()))?;

        if let Some(error) = terminal_outcome_error {
            return Err(error);
        }

        let mut outputs = EmbeddedWorkflowHost::collect_data_graph_outputs(
            graph_id,
            &terminal_nodes,
            &node_outputs,
        );
        outputs.extend(terminal_errors);
        Ok(outputs)
    }

    pub fn workflow_graph_save(
        &self,
        request: WorkflowGraphSaveRequest,
    ) -> Result<WorkflowGraphSaveResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_save(&self.graph_store(), request)
    }

    pub fn workflow_graph_load(
        &self,
        request: WorkflowGraphLoadRequest,
    ) -> Result<WorkflowFile, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_load(&self.graph_store(), request)
    }

    pub fn workflow_graph_list(&self) -> Result<WorkflowGraphListResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_list(&self.graph_store())
    }

    pub async fn workflow_graph_create_edit_session(
        &self,
        request: WorkflowGraphEditSessionCreateRequest,
    ) -> Result<WorkflowGraphEditSessionCreateResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_create_edit_session(request)
            .await
    }

    pub async fn workflow_graph_close_edit_session(
        &self,
        request: WorkflowGraphEditSessionCloseRequest,
    ) -> Result<WorkflowGraphEditSessionCloseResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_close_edit_session(request)
            .await
    }

    pub async fn execute_edit_session_graph(
        &self,
        session_id: &str,
        session_graph: &pantograph_workflow_service::WorkflowGraph,
        embedding_request: inference::EmbeddingStartRequest,
        event_sink: Arc<dyn EventSink>,
    ) -> Result<EditSessionGraphExecutionOutcome, String> {
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&self.extensions).await;
        let restore_config = embedding_workflow::prepare_embedding_runtime_for_workflow(
            self.gateway.as_ref(),
            runtime_ext.pumas_api.as_deref(),
            embedding_request,
            embedding_workflow::resolve_embedding_model_id_from_workflow_graph(session_graph)?,
            embedding_workflow::workflow_graph_has_embedding_node(session_graph),
            embedding_workflow::workflow_graph_has_llamacpp_inference_node(session_graph),
        )
        .await?;
        self.reconcile_runtime_registry_from_gateway().await;

        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.config.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_event_sink(event_sink.clone())
                .with_execution_id(session_id.to_string()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );

        let terminal_nodes: Vec<String> = session_graph
            .nodes
            .iter()
            .filter(|node| {
                !session_graph
                    .edges
                    .iter()
                    .any(|edge| edge.source == node.id)
            })
            .map(|node| node.id.clone())
            .collect();

        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());
        let mut executor = WorkflowExecutor::new(
            session_id,
            convert_graph_to_node_engine(session_graph),
            event_sink.clone(),
        );
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            Some(event_sink.clone()),
            Some(session_id.to_string()),
            Some(python_runtime_execution_recorder.clone()),
        );
        executor.set_event_sink(event_sink.clone());
        workflow_runtime::sync_embedding_emit_metadata_flags(&mut executor)
            .await
            .map_err(|error| error.to_string())?;

        self.workflow_service
            .workflow_graph_mark_edit_session_running(session_id)
            .await
            .map_err(|error| error.to_envelope_json())?;

        let _ = event_sink.send(
            node_engine::WorkflowEvent::WorkflowStarted {
                workflow_id: session_id.to_string(),
                execution_id: session_id.to_string(),
                occurred_at_ms: None,
            }
            .now(),
        );

        let mut workflow_result: Result<(), node_engine::NodeEngineError> = Ok(());
        let mut waiting_for_input = false;
        for node_id in &terminal_nodes {
            match executor.demand(node_id, &task_executor).await {
                Ok(_outputs) => {
                    log::debug!("Demanded outputs from node: {}", node_id);
                }
                Err(node_engine::NodeEngineError::WaitingForInput { task_id, prompt }) => {
                    log::info!(
                        "Workflow session '{}' is waiting for input at node '{}' (prompt: {:?})",
                        session_id,
                        task_id,
                        prompt
                    );
                    waiting_for_input = true;
                    break;
                }
                Err(error) => {
                    log::error!("Error demanding from node {}: {}", node_id, error);
                    workflow_result = Err(error);
                    break;
                }
            }
        }

        self.workflow_service
            .workflow_graph_mark_edit_session_finished(session_id)
            .await
            .map_err(|error| error.to_envelope_json())?;

        if waiting_for_input {
            log::debug!(
                "Workflow session '{}' paused in waiting-for-input state",
                session_id
            );
        } else if workflow_result.is_ok() {
            let _ = event_sink.send(
                node_engine::WorkflowEvent::WorkflowCompleted {
                    workflow_id: session_id.to_string(),
                    execution_id: session_id.to_string(),
                    occurred_at_ms: None,
                }
                .now(),
            );
        } else if let Err(error) = &workflow_result {
            let terminal_event = match error {
                node_engine::NodeEngineError::Cancelled => {
                    node_engine::WorkflowEvent::WorkflowCancelled {
                        workflow_id: session_id.to_string(),
                        execution_id: session_id.to_string(),
                        error: error.to_string(),
                        occurred_at_ms: None,
                    }
                }
                _ => node_engine::WorkflowEvent::WorkflowFailed {
                    workflow_id: session_id.to_string(),
                    execution_id: session_id.to_string(),
                    error: error.to_string(),
                    occurred_at_ms: None,
                },
            };
            let _ = event_sink.send(terminal_event.now());
        }

        let execution_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&self.gateway.mode_info().await);
        let recorded_python_runtimes = python_runtime_execution_recorder.snapshots();
        let recorded_python_runtime = recorded_python_runtimes.last();
        let runtime_snapshot = if let Some(metadata) = recorded_python_runtime.as_ref() {
            metadata.snapshot.clone()
        } else {
            self.gateway.runtime_lifecycle_snapshot().await
        };
        let runtime_model_target = recorded_python_runtime
            .and_then(|metadata| metadata.model_target.clone())
            .or_else(|| {
                workflow_runtime::resolve_runtime_model_target(
                    &execution_mode_info,
                    &runtime_snapshot,
                )
            });
        let observed_runtime_ids = recorded_python_runtimes
            .iter()
            .filter_map(|metadata| metadata.snapshot.runtime_id.clone())
            .collect::<Vec<_>>();
        let trace_runtime_metrics =
            workflow_runtime::trace_runtime_metrics_with_observed_runtime_ids(
                &runtime_snapshot,
                runtime_model_target.as_deref(),
                &observed_runtime_ids,
            );
        let restore_result = if let Some(runtime_registry) = self.runtime_registry.as_ref() {
            runtime_registry::restore_runtime_and_reconcile_runtime_registry(
                self.gateway.as_ref(),
                runtime_registry.as_ref(),
                restore_config,
            )
            .await
        } else {
            self.gateway.restore_inference_runtime(restore_config).await
        };
        if let Err(error) = restore_result {
            log::warn!(
                "Workflow executed in embedding mode but failed to restore previous inference mode: {}",
                error
            );
        }

        Ok(EditSessionGraphExecutionOutcome {
            runtime_snapshot,
            trace_runtime_metrics,
            runtime_model_target,
            waiting_for_input,
            error: workflow_result.err().map(|error| error.to_string()),
        })
    }

    pub async fn workflow_graph_get_edit_session_graph(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_get_edit_session_graph(request)
            .await
    }

    pub async fn workflow_graph_get_undo_redo_state(
        &self,
        request: WorkflowGraphUndoRedoStateRequest,
    ) -> Result<WorkflowGraphUndoRedoStateResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_get_undo_redo_state(request)
            .await
    }

    pub async fn workflow_graph_update_node_data(
        &self,
        request: WorkflowGraphUpdateNodeDataRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_update_node_data(request)
            .await
    }

    pub async fn workflow_graph_update_node_position(
        &self,
        request: WorkflowGraphUpdateNodePositionRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_update_node_position(request)
            .await
    }

    pub async fn workflow_graph_add_node(
        &self,
        request: WorkflowGraphAddNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service.workflow_graph_add_node(request).await
    }

    pub async fn workflow_graph_remove_node(
        &self,
        request: WorkflowGraphRemoveNodeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_remove_node(request)
            .await
    }

    pub async fn workflow_graph_add_edge(
        &self,
        request: WorkflowGraphAddEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service.workflow_graph_add_edge(request).await
    }

    pub async fn workflow_graph_remove_edge(
        &self,
        request: WorkflowGraphRemoveEdgeRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_remove_edge(request)
            .await
    }

    pub async fn workflow_graph_undo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service.workflow_graph_undo(request).await
    }

    pub async fn workflow_graph_redo(
        &self,
        request: WorkflowGraphEditSessionGraphRequest,
    ) -> Result<WorkflowGraphEditSessionGraphResponse, WorkflowServiceError> {
        self.workflow_service.workflow_graph_redo(request).await
    }

    pub async fn workflow_graph_get_connection_candidates(
        &self,
        request: WorkflowGraphGetConnectionCandidatesRequest,
    ) -> Result<ConnectionCandidatesResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_get_connection_candidates(request)
            .await
    }

    pub async fn workflow_graph_connect(
        &self,
        request: WorkflowGraphConnectRequest,
    ) -> Result<ConnectionCommitResponse, WorkflowServiceError> {
        self.workflow_service.workflow_graph_connect(request).await
    }

    pub async fn workflow_graph_insert_node_and_connect(
        &self,
        request: WorkflowGraphInsertNodeAndConnectRequest,
    ) -> Result<InsertNodeConnectionResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_insert_node_and_connect(request)
            .await
    }

    pub async fn workflow_graph_preview_node_insert_on_edge(
        &self,
        request: WorkflowGraphPreviewNodeInsertOnEdgeRequest,
    ) -> Result<EdgeInsertionPreviewResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_preview_node_insert_on_edge(request)
            .await
    }

    pub async fn workflow_graph_insert_node_on_edge(
        &self,
        request: WorkflowGraphInsertNodeOnEdgeRequest,
    ) -> Result<InsertNodeOnEdgeResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_graph_insert_node_on_edge(request)
            .await
    }
}

pub(crate) struct EmbeddedWorkflowHost {
    app_data_dir: PathBuf,
    project_root: PathBuf,
    workflow_roots: Vec<PathBuf>,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    session_executions: Arc<workflow_session_execution::WorkflowSessionExecutionStore>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

impl EmbeddedWorkflowHost {
    async fn pumas_api(&self) -> Option<Arc<pumas_library::PumasApi>> {
        let guard = self.extensions.read().await;
        guard
            .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
            .cloned()
    }

    fn observe_python_runtime_execution_metadata(
        &self,
        metadata: &[task_executor::PythonRuntimeExecutionMetadata],
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };
        for metadata in metadata {
            runtime_registry::reconcile_runtime_registry_snapshot_override_with_health_assessment(
                runtime_registry.as_ref(),
                &metadata.snapshot,
                metadata.model_target.as_deref(),
                metadata.health_assessment.as_ref(),
            );
        }

        Ok(())
    }

    fn trimmed_optional(value: Option<&str>) -> Option<String> {
        value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }

    fn reservation_requirements(
        runtime_requirements: &WorkflowRuntimeRequirements,
    ) -> Option<RuntimeReservationRequirements> {
        let requirements = RuntimeReservationRequirements {
            estimated_peak_vram_mb: runtime_requirements.estimated_peak_vram_mb,
            estimated_peak_ram_mb: runtime_requirements.estimated_peak_ram_mb,
            estimated_min_vram_mb: runtime_requirements.estimated_min_vram_mb,
            estimated_min_ram_mb: runtime_requirements.estimated_min_ram_mb,
        };

        if requirements.estimated_peak_vram_mb.is_none()
            && requirements.estimated_peak_ram_mb.is_none()
            && requirements.estimated_min_vram_mb.is_none()
            && requirements.estimated_min_ram_mb.is_none()
        {
            return None;
        }

        Some(requirements)
    }

    fn runtime_retention_hint(
        retention_hint: WorkflowSessionRetentionHint,
    ) -> RuntimeRetentionHint {
        match retention_hint {
            WorkflowSessionRetentionHint::Ephemeral => RuntimeRetentionHint::Ephemeral,
            WorkflowSessionRetentionHint::KeepAlive => RuntimeRetentionHint::KeepAlive,
        }
    }

    async fn ensure_workflow_runtime_ready_for_session_load(
        &self,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let capabilities = WorkflowHost::workflow_capabilities(self, workflow_id).await?;
        let (_, blocking_runtime_issues) = pantograph_workflow_service::evaluate_runtime_preflight(
            &capabilities.runtime_requirements.required_backends,
            &capabilities.runtime_capabilities,
        );

        if blocking_runtime_issues.is_empty() {
            return Ok(());
        }

        Err(WorkflowServiceError::RuntimeNotReady(
            pantograph_workflow_service::format_runtime_not_ready_message(&blocking_runtime_issues),
        ))
    }

    fn record_session_runtime_reservation(
        &self,
        session_id: &str,
        reservation_id: u64,
    ) -> Result<Option<u64>, WorkflowServiceError> {
        let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
            WorkflowServiceError::Internal("session runtime reservation lock poisoned".to_string())
        })?;

        Ok(reservations.insert(session_id.to_string(), reservation_id))
    }

    fn restore_session_runtime_reservation(
        &self,
        session_id: &str,
        previous_reservation_id: Option<u64>,
    ) -> Result<(), WorkflowServiceError> {
        let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
            WorkflowServiceError::Internal("session runtime reservation lock poisoned".to_string())
        })?;

        if let Some(previous_reservation_id) = previous_reservation_id {
            reservations.insert(session_id.to_string(), previous_reservation_id);
        } else {
            reservations.remove(session_id);
        }

        Ok(())
    }

    fn sync_loaded_session_runtime_retention_hint(
        &self,
        session_id: &str,
        keep_alive: bool,
        session_state: WorkflowSessionState,
    ) -> Result<(), WorkflowServiceError> {
        if session_state == WorkflowSessionState::IdleUnloaded {
            return Ok(());
        }

        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };

        let reservation_id = {
            let reservations = self.session_runtime_reservations.lock().map_err(|_| {
                WorkflowServiceError::Internal(
                    "session runtime reservation lock poisoned".to_string(),
                )
            })?;
            reservations.get(session_id).copied()
        };

        let Some(reservation_id) = reservation_id else {
            return Ok(());
        };

        runtime_registry::sync_runtime_reservation_retention_hint(
            runtime_registry.as_ref(),
            reservation_id,
            Self::runtime_retention_hint(if keep_alive {
                WorkflowSessionRetentionHint::KeepAlive
            } else {
                WorkflowSessionRetentionHint::Ephemeral
            }),
        )
        .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;

        Ok(())
    }

    async fn consume_runtime_warmup_disposition(
        &self,
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        runtime_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        runtime_registry::consume_active_runtime_warmup_disposition(
            self.gateway.as_ref(),
            runtime_registry,
            runtime_id,
            Duration::from_millis(RUNTIME_WARMUP_POLL_INTERVAL_MS),
            Duration::from_millis(RUNTIME_WARMUP_WAIT_TIMEOUT_MS),
        )
        .await
        .map_err(runtime_registry_errors::workflow_service_error_from_runtime_warmup_coordination)
    }

    async fn reserve_loaded_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };

        let mode_info = self.gateway.mode_info().await;
        let host_runtime_mode_info = HostRuntimeModeSnapshot::from_mode_info(&mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );
        let trimmed_usage_profile = Self::trimmed_optional(usage_profile);
        let reservation_request = runtime_registry::active_runtime_reservation_request(
            runtime_registry,
            &host_runtime_mode_info,
            workflow_id,
            Some(session_id),
            trimmed_usage_profile.as_deref(),
            requirements,
            Self::runtime_retention_hint(retention_hint),
        );
        let descriptor = runtime_registry::active_runtime_descriptor(&host_runtime_mode_info);
        let lease = runtime_registry
            .acquire_reservation(reservation_request)
            .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;

        let previous_reservation_id =
            self.record_session_runtime_reservation(session_id, lease.reservation_id)?;
        if let Err(error) = self
            .consume_runtime_warmup_disposition(runtime_registry.as_ref(), &descriptor.runtime_id)
            .await
        {
            self.restore_session_runtime_reservation(session_id, previous_reservation_id)?;
            if previous_reservation_id != Some(lease.reservation_id) {
                runtime_registry::release_reservation_and_reconcile_runtime_registry(
                    self.gateway.as_ref(),
                    runtime_registry.as_ref(),
                    lease.reservation_id,
                )
                .await
                .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;
            }
            return Err(error);
        }

        Ok(())
    }

    async fn release_loaded_session_runtime(
        &self,
        session_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };

        let reservation_id = {
            let mut reservations = self.session_runtime_reservations.lock().map_err(|_| {
                WorkflowServiceError::Internal(
                    "session runtime reservation lock poisoned".to_string(),
                )
            })?;
            reservations.remove(session_id)
        };

        if let Some(reservation_id) = reservation_id {
            runtime_registry::release_reservation_and_reconcile_runtime_registry(
                self.gateway.as_ref(),
                runtime_registry.as_ref(),
                reservation_id,
            )
            .await
            .map_err(runtime_registry_errors::workflow_service_error_from_runtime_registry)?;
        }

        Ok(())
    }

    fn apply_input_bindings(
        graph: &mut WorkflowGraph,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        for binding in inputs {
            let node = graph
                .nodes
                .iter_mut()
                .find(|node| node.id == binding.node_id)
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "input binding references unknown node_id '{}'",
                        binding.node_id
                    ))
                })?;

            if node.data.is_null() {
                node.data = serde_json::json!({});
            }

            let map = node.data.as_object_mut().ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "input node '{}' has non-object data payload",
                    binding.node_id
                ))
            })?;
            map.insert(binding.port_id.clone(), binding.value.clone());
        }

        Ok(())
    }

    fn resolve_output_node_ids(
        graph: &WorkflowGraph,
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<String>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let known_nodes = graph
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<HashSet<_>>();
            let mut dedup = HashSet::new();
            let mut node_ids = Vec::new();

            for target in targets {
                if !known_nodes.contains(target.node_id.as_str()) {
                    return Err(WorkflowServiceError::InvalidRequest(format!(
                        "output target references unknown node_id '{}'",
                        target.node_id
                    )));
                }
                if dedup.insert(target.node_id.clone()) {
                    node_ids.push(target.node_id.clone());
                }
            }
            return Ok(node_ids);
        }

        let output_node_ids = graph
            .nodes
            .iter()
            .filter(|node| node.node_type.ends_with("-output"))
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        if output_node_ids.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow has no output nodes; add explicit `*-output` nodes or provide output_targets"
                    .to_string(),
            ));
        }

        Ok(output_node_ids)
    }

    fn collect_run_outputs(
        node_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
        output_node_ids: &[String],
        output_targets: Option<&[WorkflowOutputTarget]>,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            let mut outputs = Vec::with_capacity(targets.len());
            for target in targets {
                let Some(value) = node_outputs
                    .get(&target.node_id)
                    .and_then(|ports| ports.get(&target.port_id))
                    .cloned()
                else {
                    continue;
                };

                outputs.push(WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value,
                });
            }
            return Ok(outputs);
        }

        let mut outputs = Vec::new();
        for node_id in output_node_ids {
            let Some(ports) = node_outputs.get(node_id) else {
                continue;
            };

            let mut keys = ports.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for port_id in keys {
                if let Some(value) = ports.get(&port_id) {
                    outputs.push(WorkflowPortBinding {
                        node_id: node_id.clone(),
                        port_id,
                        value: value.clone(),
                    });
                }
            }
        }

        Ok(outputs)
    }

    fn apply_data_graph_inputs(
        graph: &mut WorkflowGraph,
        inputs: &HashMap<String, serde_json::Value>,
    ) {
        for (port_name, value) in inputs {
            for node in &mut graph.nodes {
                if node.node_type == "text-input" && (port_name == "text" || port_name == "input") {
                    if let Some(obj) = node.data.as_object_mut() {
                        obj.insert("text".to_string(), value.clone());
                    } else {
                        node.data = serde_json::json!({ "text": value });
                    }
                }

                if let Some(obj) = node.data.as_object_mut() {
                    obj.insert(format!("_input_{}", port_name), value.clone());
                }
            }
        }
    }

    fn terminal_data_graph_node_ids(graph: &WorkflowGraph) -> Vec<String> {
        graph
            .nodes
            .iter()
            .filter(|node| !graph.edges.iter().any(|edge| edge.source == node.id))
            .map(|node| node.id.clone())
            .collect()
    }

    fn collect_data_graph_outputs(
        graph_id: &str,
        terminal_nodes: &[String],
        node_outputs: &HashMap<String, HashMap<String, serde_json::Value>>,
    ) -> HashMap<String, serde_json::Value> {
        let mut outputs = HashMap::new();

        for terminal_id in terminal_nodes {
            let Some(terminal_outputs) = node_outputs.get(terminal_id) else {
                continue;
            };

            for (output_port, output_value) in terminal_outputs {
                outputs.insert(
                    format!("{}.{}", terminal_id, output_port),
                    output_value.clone(),
                );
                outputs.insert(output_port.clone(), output_value.clone());
            }
        }

        outputs.insert(
            "_graph_id".to_string(),
            serde_json::Value::String(graph_id.to_string()),
        );
        outputs.insert(
            "_terminal_nodes".to_string(),
            serde_json::Value::Array(
                terminal_nodes
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );

        outputs
    }

    fn fallback_runtime_unload_candidate(
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowSessionRuntimeUnloadCandidate> {
        pantograph_workflow_service::select_runtime_unload_candidate_by_affinity(target, candidates)
    }
}

#[async_trait]
impl WorkflowHost for EmbeddedWorkflowHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        self.workflow_roots.clone()
    }

    fn max_input_bindings(&self) -> usize {
        capabilities::DEFAULT_MAX_INPUT_BINDINGS
    }

    fn max_output_targets(&self) -> usize {
        capabilities::DEFAULT_MAX_OUTPUT_TARGETS
    }

    fn max_value_bytes(&self) -> usize {
        capabilities::DEFAULT_MAX_VALUE_BYTES
    }

    async fn default_backend_name(&self) -> Result<String, WorkflowServiceError> {
        Ok(canonical_runtime_backend_key(
            &self.gateway.current_backend_name().await,
        ))
    }

    async fn model_metadata(
        &self,
        model_id: &str,
    ) -> Result<Option<serde_json::Value>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| m.metadata))
    }

    async fn model_descriptor(
        &self,
        model_id: &str,
    ) -> Result<Option<WorkflowHostModelDescriptor>, WorkflowServiceError> {
        let Some(api) = self.pumas_api().await else {
            return Ok(None);
        };

        let model = api
            .get_model(model_id)
            .await
            .map_err(|e| WorkflowServiceError::RuntimeNotReady(e.to_string()))?;
        Ok(model.map(|m| WorkflowHostModelDescriptor {
            model_type: Some(m.model_type.trim().to_string()).filter(|v| !v.is_empty()),
            hashes: m.hashes,
        }))
    }

    async fn runtime_capabilities(
        &self,
    ) -> Result<Vec<WorkflowRuntimeCapability>, WorkflowServiceError> {
        let selected_backend_key =
            canonical_runtime_backend_key(&self.gateway.current_backend_name().await);
        let available_backends = self.gateway.available_backends();
        let managed_runtimes = inference::list_managed_runtime_snapshots(&self.app_data_dir)
            .map_err(WorkflowServiceError::RuntimeNotReady)?;
        let mut runtimes = runtime_capabilities::managed_runtime_capabilities(
            &managed_runtimes,
            &available_backends,
            &selected_backend_key,
        );
        runtimes.extend(runtime_capabilities::host_runtime_capabilities(
            &available_backends,
            &selected_backend_key,
        ));
        runtimes.extend(runtime_capabilities::python_runtime_capabilities(
            python_runtime::resolve_python_executable_for_env_ids(&[]),
            &selected_backend_key,
        ));
        runtimes.extend(self.additional_runtime_capabilities.clone());
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(runtimes)
    }

    async fn can_load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<bool, WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(true);
        };

        let mode_info = self.gateway.mode_info().await;
        let host_runtime_mode_info = HostRuntimeModeSnapshot::from_mode_info(&mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );
        let trimmed_usage_profile = Self::trimmed_optional(usage_profile);
        let reservation_request = runtime_registry::active_runtime_reservation_request(
            runtime_registry,
            &host_runtime_mode_info,
            workflow_id,
            Some(session_id),
            trimmed_usage_profile.as_deref(),
            requirements,
            Self::runtime_retention_hint(retention_hint),
        );

        match runtime_registry.can_acquire_reservation(&reservation_request) {
            Ok(()) => Ok(true),
            Err(RuntimeRegistryError::AdmissionRejected { .. })
            | Err(RuntimeRegistryError::ReservationRejected(_)) => Ok(false),
            Err(error) => {
                Err(runtime_registry_errors::workflow_service_error_from_runtime_registry(error))
            }
        }
    }

    async fn load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.ensure_workflow_runtime_ready_for_session_load(workflow_id)
            .await?;
        self.reserve_loaded_session_runtime(session_id, workflow_id, usage_profile, retention_hint)
            .await
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        reason: pantograph_workflow_service::WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.release_loaded_session_runtime(session_id).await?;
        workflow_session_execution::apply_session_execution_unload_transition(
            &self.session_executions,
            session_id,
            reason,
        )
        .await
    }

    async fn select_runtime_unload_candidate(
        &self,
        target: &WorkflowSessionRuntimeSelectionTarget,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(Self::fallback_runtime_unload_candidate(target, candidates));
        };

        if let Some((session_id, _runtime_id)) =
            runtime_registry::runtime_registry_reclaim_candidate_for_sessions(
                runtime_registry,
                candidates,
            )
        {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.session_id == session_id)
            {
                return Ok(Some(candidate.clone()));
            }
        }

        Ok(Self::fallback_runtime_unload_candidate(target, candidates))
    }

    async fn workflow_technical_fit_decision(
        &self,
        request: &WorkflowTechnicalFitRequest,
    ) -> Result<Option<WorkflowTechnicalFitDecision>, WorkflowServiceError> {
        technical_fit::workflow_technical_fit_decision(self, request).await
    }

    async fn workflow_session_inspection_state(
        &self,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<Option<WorkflowGraphSessionStateView>, WorkflowServiceError> {
        let Some(entry) = self.session_executions.get(session_id)? else {
            return Ok(None);
        };
        if entry.workflow_id != workflow_id {
            return Ok(None);
        }

        let executor = entry.executor.lock().await;
        let residency = executor.workflow_session_residency().await;
        let node_memory = executor
            .workflow_session_node_memory_snapshots(session_id)
            .await;
        let checkpoint = Some(
            executor
                .workflow_session_checkpoint_summary(session_id)
                .await,
        );
        Ok(Some(WorkflowGraphSessionStateView::new(
            residency,
            node_memory,
            None,
            checkpoint,
        )))
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        run_options: WorkflowRunOptions,
        run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(workflow_session_id) = run_options.workflow_session_id.as_deref() {
            return workflow_session_execution::run_session_workflow(
                self,
                workflow_id,
                workflow_session_id,
                inputs,
                output_targets,
                run_handle,
            )
            .await;
        }

        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::Cancelled(
                "workflow run cancelled before execution started".to_string(),
            ));
        }

        let stored = capabilities::load_and_validate_workflow(workflow_id, &self.workflow_roots)?;
        let mut graph = stored.to_workflow_graph(workflow_id);
        Self::apply_input_bindings(&mut graph, inputs)?;

        let output_node_ids = Self::resolve_output_node_ids(&graph, output_targets)?;
        let runtime_ext = RuntimeExtensionsSnapshot::from_shared(&self.extensions).await;

        let execution_id = Uuid::new_v4().to_string();
        let core = Arc::new(
            CoreTaskExecutor::new()
                .with_project_root(self.project_root.clone())
                .with_gateway(self.gateway.clone())
                .with_execution_id(execution_id.clone()),
        );
        let host = Arc::new(task_executor::TauriTaskExecutor::with_python_runtime(
            self.rag_backend.clone(),
            self.python_runtime.clone(),
        ));
        let task_executor = node_engine::CompositeTaskExecutor::new(
            Some(host as Arc<dyn node_engine::TaskExecutor>),
            core,
        );
        let python_runtime_execution_recorder =
            Arc::new(task_executor::PythonRuntimeExecutionRecorder::default());

        let mut executor =
            WorkflowExecutor::new(execution_id.clone(), graph, Arc::new(NullEventSink));
        apply_runtime_extensions_for_execution(
            &mut executor,
            &runtime_ext,
            None,
            Some(execution_id.clone()),
            Some(python_runtime_execution_recorder.clone()),
        );

        let mut node_outputs = HashMap::new();
        let mut run_result = Ok(());
        for node_id in &output_node_ids {
            if run_handle.is_cancelled() {
                run_result = Err(WorkflowServiceError::Cancelled(
                    "workflow run cancelled during execution".to_string(),
                ));
                break;
            }
            match executor.demand(node_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(node_id.clone(), outputs);
                }
                Err(error) => {
                    run_result = Err(match error {
                        node_engine::NodeEngineError::WaitingForInput { task_id, .. } => {
                            WorkflowServiceError::InvalidRequest(format!(
                                "workflow '{}' requires interactive input at node '{}'",
                                workflow_id, task_id
                            ))
                        }
                        other => WorkflowServiceError::Internal(other.to_string()),
                    });
                    break;
                }
            }
        }

        let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshots();
        self.observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)?;

        run_result?;
        Self::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
