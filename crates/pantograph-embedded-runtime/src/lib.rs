use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use node_engine::{
    CoreTaskExecutor, ExecutorExtensions, NullEventSink, WorkflowExecutor, WorkflowGraph,
};
use pantograph_runtime_identity::{
    backend_key_aliases, canonical_runtime_backend_key, canonical_runtime_id,
    runtime_backend_key_aliases, runtime_display_name,
};
use pantograph_runtime_registry::{
    observed_runtime_status_from_lifecycle, RuntimeObservation, RuntimeRegistration,
    RuntimeReservationRequest, RuntimeReservationRequirements, RuntimeRetentionHint,
    RuntimeTransition, RuntimeWarmupDecision, SharedRuntimeRegistry,
};
use pantograph_workflow_service::capabilities;
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
    WorkflowRuntimeInstallState, WorkflowRuntimeRequirements, WorkflowRuntimeSourceKind,
    WorkflowService, WorkflowServiceError, WorkflowSessionCloseRequest,
    WorkflowSessionCloseResponse, WorkflowSessionCreateRequest, WorkflowSessionCreateResponse,
    WorkflowSessionKeepAliveRequest, WorkflowSessionKeepAliveResponse,
    WorkflowSessionQueueCancelRequest, WorkflowSessionQueueCancelResponse,
    WorkflowSessionQueueListRequest, WorkflowSessionQueueListResponse,
    WorkflowSessionQueueReprioritizeRequest, WorkflowSessionQueueReprioritizeResponse,
    WorkflowSessionRetentionHint, WorkflowSessionRunRequest, WorkflowSessionRuntimeUnloadCandidate,
    WorkflowSessionStatusRequest, WorkflowSessionStatusResponse,
};
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod model_dependencies;
pub mod python_runtime;
pub mod rag;
pub mod task_executor;

pub use model_dependencies::{SharedModelDependencyResolver, TauriModelDependencyResolver};
pub use python_runtime::{
    ProcessPythonRuntimeAdapter, PythonNodeExecutionRequest, PythonRuntimeAdapter,
    PythonStreamHandler,
};
pub use rag::{RagBackend, RagDocument};
pub use task_executor::{runtime_extension_keys, TauriTaskExecutor as PantographTaskExecutor};

pub type SharedExtensions = Arc<RwLock<ExecutorExtensions>>;
pub type SharedWorkflowService = Arc<WorkflowService>;

const RUNTIME_WARMUP_POLL_INTERVAL_MS: u64 = 25;

#[cfg(not(test))]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 5_000;

#[cfg(test)]
const RUNTIME_WARMUP_WAIT_TIMEOUT_MS: u64 = 250;

#[derive(Debug, Clone)]
pub struct EmbeddedRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
    pub max_loaded_sessions: Option<usize>,
}

impl EmbeddedRuntimeConfig {
    pub fn new(app_data_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            app_data_dir,
            workflow_roots: capabilities::default_workflow_roots(&project_root),
            project_root,
            max_loaded_sessions: None,
        }
    }
}

#[cfg(feature = "standalone")]
#[derive(Debug, Clone)]
pub struct StandaloneRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
    pub max_loaded_sessions: Option<usize>,
    pub binaries_dir: PathBuf,
    pub pumas_library_path: Option<PathBuf>,
}

#[cfg(feature = "standalone")]
impl StandaloneRuntimeConfig {
    pub fn new(app_data_dir: PathBuf, project_root: PathBuf, binaries_dir: PathBuf) -> Self {
        Self {
            app_data_dir,
            workflow_roots: capabilities::default_workflow_roots(&project_root),
            project_root,
            max_loaded_sessions: None,
            binaries_dir,
            pumas_library_path: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EmbeddedRuntimeError {
    #[error("configuration error: {message}")]
    Config { message: String },

    #[error("runtime initialization error: {message}")]
    Initialization { message: String },
}

#[derive(Clone, Default)]
pub struct RuntimeExtensionsSnapshot {
    pub pumas_api: Option<Arc<pumas_library::PumasApi>>,
    pub kv_cache_store: Option<Arc<inference::kv_cache::KvCacheStore>>,
    pub dependency_resolver: Option<Arc<dyn node_engine::ModelDependencyResolver>>,
}

impl RuntimeExtensionsSnapshot {
    pub async fn from_shared(shared: &SharedExtensions) -> Self {
        let guard = shared.read().await;
        Self::from_extensions(&guard)
    }

    pub fn from_extensions(shared: &ExecutorExtensions) -> Self {
        Self {
            pumas_api: shared
                .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .cloned(),
            kv_cache_store: shared
                .get::<Arc<inference::kv_cache::KvCacheStore>>(
                    node_engine::extension_keys::KV_CACHE_STORE,
                )
                .cloned(),
            dependency_resolver: shared
                .get::<Arc<dyn node_engine::ModelDependencyResolver>>(
                    node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
                )
                .cloned(),
        }
    }
}

pub fn apply_runtime_extensions(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
) {
    apply_runtime_extensions_with_python_runtime_recorder(executor, snapshot, None);
}

fn apply_runtime_extensions_with_python_runtime_recorder(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
    python_runtime_execution_recorder: Option<Arc<task_executor::PythonRuntimeExecutionRecorder>>,
) {
    if let Some(api) = &snapshot.pumas_api {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::PUMAS_API, api.clone());
    }
    if let Some(store) = &snapshot.kv_cache_store {
        executor
            .extensions_mut()
            .set(node_engine::extension_keys::KV_CACHE_STORE, store.clone());
    }
    if let Some(resolver) = &snapshot.dependency_resolver {
        executor.extensions_mut().set(
            node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
            resolver.clone(),
        );
    }
    if let Some(recorder) = python_runtime_execution_recorder {
        executor.extensions_mut().set(
            task_executor::runtime_extension_keys::PYTHON_RUNTIME_EXECUTION_RECORDER,
            recorder,
        );
    }
}

pub struct EmbeddedRuntime {
    config: EmbeddedRuntimeConfig,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    workflow_service: SharedWorkflowService,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
    additional_runtime_capabilities: Vec<WorkflowRuntimeCapability>,
}

impl EmbeddedRuntime {
    pub fn from_components(
        config: EmbeddedRuntimeConfig,
        gateway: Arc<inference::InferenceGateway>,
        extensions: SharedExtensions,
        workflow_service: SharedWorkflowService,
        rag_backend: Option<Arc<dyn RagBackend>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        Self {
            config,
            gateway,
            extensions,
            workflow_service,
            runtime_registry: None,
            session_runtime_reservations: Arc::new(Mutex::new(HashMap::new())),
            rag_backend,
            python_runtime,
            additional_runtime_capabilities: Vec::new(),
        }
    }

    pub fn with_default_python_runtime(
        config: EmbeddedRuntimeConfig,
        gateway: Arc<inference::InferenceGateway>,
        extensions: SharedExtensions,
        workflow_service: SharedWorkflowService,
        rag_backend: Option<Arc<dyn RagBackend>>,
    ) -> Self {
        Self::from_components(
            config,
            gateway,
            extensions,
            workflow_service,
            rag_backend,
            Arc::new(ProcessPythonRuntimeAdapter),
        )
    }

    #[cfg(feature = "standalone")]
    pub async fn standalone(config: StandaloneRuntimeConfig) -> Result<Self, EmbeddedRuntimeError> {
        use inference::process::StdProcessSpawner;

        let gateway = Arc::new(inference::InferenceGateway::new());
        gateway
            .set_spawner(Arc::new(StdProcessSpawner::new(
                config.binaries_dir.clone(),
                config.app_data_dir.clone(),
            )))
            .await;

        let workflow_service = Arc::new(WorkflowService::new());
        workflow_service
            .set_loaded_runtime_capacity_limit(config.max_loaded_sessions)
            .map_err(|error| EmbeddedRuntimeError::Initialization {
                message: error.to_string(),
            })?;
        let extensions: SharedExtensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        let dependency_resolver: Arc<dyn node_engine::ModelDependencyResolver> = Arc::new(
            TauriModelDependencyResolver::new(extensions.clone(), config.project_root.clone()),
        );

        {
            let mut guard = extensions.write().await;
            workflow_nodes::setup_extensions_with_path(
                &mut guard,
                config.pumas_library_path.as_deref(),
            )
            .await;
            guard.set(
                node_engine::extension_keys::MODEL_DEPENDENCY_RESOLVER,
                dependency_resolver,
            );
            guard.set(
                node_engine::extension_keys::KV_CACHE_STORE,
                Arc::new(inference::kv_cache::KvCacheStore::new(
                    config.app_data_dir.join("kv_cache"),
                    inference::kv_cache::StoragePolicy::MemoryAndDisk,
                )),
            );
        }

        Ok(Self::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir: config.app_data_dir,
                project_root: config.project_root,
                workflow_roots: config.workflow_roots,
                max_loaded_sessions: config.max_loaded_sessions,
            },
            gateway,
            extensions,
            workflow_service,
            None,
        ))
    }

    pub fn config(&self) -> &EmbeddedRuntimeConfig {
        &self.config
    }

    pub fn with_additional_runtime_capabilities(
        mut self,
        capabilities: Vec<WorkflowRuntimeCapability>,
    ) -> Self {
        self.additional_runtime_capabilities = capabilities;
        self
    }

    pub fn with_runtime_registry(mut self, runtime_registry: SharedRuntimeRegistry) -> Self {
        self.runtime_registry = Some(runtime_registry);
        self
    }

    pub fn workflow_service(&self) -> &SharedWorkflowService {
        &self.workflow_service
    }

    pub fn shared_extensions(&self) -> &SharedExtensions {
        &self.extensions
    }

    pub fn gateway(&self) -> &Arc<inference::InferenceGateway> {
        &self.gateway
    }

    pub async fn shutdown(&self) {
        self.gateway.stop().await;
    }

    fn host(&self) -> EmbeddedWorkflowHost {
        EmbeddedWorkflowHost {
            app_data_dir: self.config.app_data_dir.clone(),
            project_root: self.config.project_root.clone(),
            workflow_roots: self.config.workflow_roots.clone(),
            gateway: self.gateway.clone(),
            extensions: self.extensions.clone(),
            runtime_registry: self.runtime_registry.clone(),
            session_runtime_reservations: self.session_runtime_reservations.clone(),
            rag_backend: self.rag_backend.clone(),
            python_runtime: self.python_runtime.clone(),
            additional_runtime_capabilities: self.additional_runtime_capabilities.clone(),
        }
    }

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

    pub async fn workflow_list_session_queue(
        &self,
        request: WorkflowSessionQueueListRequest,
    ) -> Result<WorkflowSessionQueueListResponse, WorkflowServiceError> {
        self.workflow_service
            .workflow_list_session_queue(request)
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
        self.workflow_service
            .workflow_set_session_keep_alive(&self.host(), request)
            .await
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

struct EmbeddedWorkflowHost {
    app_data_dir: PathBuf,
    project_root: PathBuf,
    workflow_roots: Vec<PathBuf>,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    runtime_registry: Option<SharedRuntimeRegistry>,
    session_runtime_reservations: Arc<Mutex<HashMap<String, u64>>>,
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

    fn runtime_backend_keys(binary_id: inference::ManagedBinaryId) -> Vec<String> {
        match binary_id {
            inference::ManagedBinaryId::LlamaCpp => backend_key_aliases("llama.cpp", "llama_cpp"),
            inference::ManagedBinaryId::Ollama => backend_key_aliases("Ollama", "ollama"),
        }
    }

    fn runtime_matches_backend(backend_keys: &[String], selected_backend_key: &str) -> bool {
        backend_keys
            .iter()
            .any(|backend_key| canonical_runtime_backend_key(backend_key) == selected_backend_key)
    }

    fn runtime_supports_external_connection(
        available_backends: &[inference::BackendInfo],
        backend_keys: &[String],
    ) -> bool {
        let normalized_backend_keys = backend_keys
            .iter()
            .map(|backend_key| inference::backend::canonical_backend_key(backend_key))
            .collect::<HashSet<_>>();

        available_backends.iter().any(|backend| {
            normalized_backend_keys.contains(&backend.backend_key)
                && backend.capabilities.external_connection
        })
    }

    fn is_python_sidecar_backend(backend: &inference::BackendInfo) -> bool {
        backend.backend_key == "pytorch"
    }

    fn host_runtime_capability(
        backend: &inference::BackendInfo,
        selected_backend_key: &str,
    ) -> Option<WorkflowRuntimeCapability> {
        if backend.runtime_binary_id.is_some() || Self::is_python_sidecar_backend(backend) {
            return None;
        }

        let backend_keys = backend_key_aliases(&backend.name, &backend.backend_key);
        Some(WorkflowRuntimeCapability {
            runtime_id: backend.backend_key.clone(),
            display_name: backend.name.clone(),
            install_state: if backend.available {
                WorkflowRuntimeInstallState::SystemProvided
            } else {
                WorkflowRuntimeInstallState::Missing
            },
            available: backend.available,
            configured: backend.available,
            can_install: backend.can_install,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Host,
            selected: Self::runtime_matches_backend(&backend_keys, selected_backend_key),
            supports_external_connection: backend.capabilities.external_connection,
            backend_keys,
            missing_files: Vec::new(),
            unavailable_reason: backend.unavailable_reason.clone(),
        })
    }

    fn python_runtime_backend_keys(display_name: &str, runtime_id: &str) -> Vec<String> {
        runtime_backend_key_aliases(display_name, runtime_id)
    }

    fn observe_python_runtime_execution_metadata(
        &self,
        metadata: Option<&task_executor::PythonRuntimeExecutionMetadata>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(());
        };
        let Some(metadata) = metadata else {
            return Ok(());
        };
        let Some(runtime_id) = metadata
            .snapshot
            .runtime_id
            .as_deref()
            .map(canonical_runtime_id)
            .filter(|runtime_id| !runtime_id.is_empty())
        else {
            return Ok(());
        };

        let display_name = runtime_display_name(&runtime_id).unwrap_or("Python runtime");
        runtime_registry.observe_runtime(RuntimeObservation {
            runtime_id: runtime_id.clone(),
            display_name: display_name.to_string(),
            backend_keys: Self::python_runtime_backend_keys(display_name, &runtime_id),
            model_id: metadata.model_target.clone(),
            status: observed_runtime_status_from_lifecycle(
                metadata.snapshot.active,
                metadata.snapshot.warmup_started_at_ms,
                metadata.snapshot.warmup_completed_at_ms,
                metadata.snapshot.last_error.is_some(),
            ),
            runtime_instance_id: metadata.snapshot.runtime_instance_id.clone(),
            last_error: metadata.snapshot.last_error.clone(),
        });

        Ok(())
    }

    fn python_runtime_capabilities(
        executable_probe: Result<PathBuf, String>,
        selected_backend_key: &str,
    ) -> Vec<WorkflowRuntimeCapability> {
        let (available, unavailable_reason) = match executable_probe {
            Ok(_) => (true, None),
            Err(reason) => (false, Some(reason)),
        };
        [
            (
                runtime_display_name("pytorch").unwrap_or("PyTorch (Python sidecar)"),
                "pytorch",
            ),
            (
                runtime_display_name("diffusers").unwrap_or("Diffusers (Python sidecar)"),
                "diffusers",
            ),
            (
                runtime_display_name("onnx-runtime").unwrap_or("ONNX Runtime (Python sidecar)"),
                "onnx-runtime",
            ),
            (
                runtime_display_name("stable_audio").unwrap_or("Stable Audio (Python sidecar)"),
                "stable_audio",
            ),
        ]
        .into_iter()
        .map(|(display_name, runtime_id)| {
            let backend_keys = Self::python_runtime_backend_keys(display_name, runtime_id);
            WorkflowRuntimeCapability {
                runtime_id: runtime_id.to_string(),
                display_name: display_name.to_string(),
                install_state: if available {
                    WorkflowRuntimeInstallState::SystemProvided
                } else {
                    WorkflowRuntimeInstallState::Missing
                },
                available,
                configured: available,
                can_install: false,
                can_remove: false,
                source_kind: WorkflowRuntimeSourceKind::System,
                selected: Self::runtime_matches_backend(&backend_keys, selected_backend_key),
                supports_external_connection: false,
                backend_keys,
                missing_files: Vec::new(),
                unavailable_reason: unavailable_reason.clone(),
            }
        })
        .collect()
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

    fn active_runtime_descriptor(
        mode_info: &inference::ServerModeInfo,
    ) -> (String, String, Vec<String>, Option<String>) {
        let runtime_id = mode_info
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.clone())
            .or_else(|| mode_info.backend_key.clone())
            .or_else(|| mode_info.backend_name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let display_name = mode_info
            .backend_name
            .clone()
            .unwrap_or_else(|| runtime_id.clone());
        let backend_keys = mode_info
            .backend_key
            .clone()
            .into_iter()
            .collect::<Vec<_>>();
        let runtime_instance_id = mode_info
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.clone());

        (runtime_id, display_name, backend_keys, runtime_instance_id)
    }

    fn active_runtime_observation(
        mode_info: &inference::ServerModeInfo,
    ) -> Option<RuntimeObservation> {
        let snapshot = mode_info
            .active_runtime
            .as_ref()
            .cloned()
            .unwrap_or_default();
        let (runtime_id, display_name, backend_keys, _) =
            Self::active_runtime_descriptor(mode_info);
        let status = observed_runtime_status_from_lifecycle(
            snapshot.active,
            snapshot.warmup_started_at_ms,
            snapshot.warmup_completed_at_ms,
            snapshot.last_error.is_some(),
        );

        if matches!(
            status,
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
        ) && snapshot.last_error.is_none()
        {
            return None;
        }

        Some(RuntimeObservation {
            runtime_id,
            display_name,
            backend_keys,
            model_id: mode_info.active_model_target.clone(),
            status,
            runtime_instance_id: snapshot.runtime_instance_id,
            last_error: snapshot.last_error,
        })
    }

    fn reconcile_active_runtime_mode_info(
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        mode_info: &inference::ServerModeInfo,
    ) {
        if let Some(observation) = Self::active_runtime_observation(mode_info) {
            runtime_registry.observe_runtime(observation);
        }
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

    async fn wait_for_runtime_warmup_transition(
        &self,
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        runtime_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let wait_future = async {
            loop {
                let mode_info = self.gateway.mode_info().await;
                Self::reconcile_active_runtime_mode_info(runtime_registry, &mode_info);

                let disposition = runtime_registry
                    .warmup_disposition(runtime_id)
                    .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?;
                match disposition.decision {
                    RuntimeWarmupDecision::ReuseLoadedRuntime => return Ok(()),
                    RuntimeWarmupDecision::StartRuntime => {
                        let runtime_instance_id = mode_info
                            .active_runtime
                            .as_ref()
                            .and_then(|snapshot| snapshot.runtime_instance_id.clone());
                        runtime_registry
                            .transition_runtime(
                                runtime_id,
                                RuntimeTransition::WarmupStarted {
                                    runtime_instance_id,
                                },
                            )
                            .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?;
                        return Ok(());
                    }
                    RuntimeWarmupDecision::WaitForTransition => {
                        tokio::time::sleep(Duration::from_millis(RUNTIME_WARMUP_POLL_INTERVAL_MS))
                            .await;
                    }
                }
            }
        };

        tokio::time::timeout(
            Duration::from_millis(RUNTIME_WARMUP_WAIT_TIMEOUT_MS),
            wait_future,
        )
        .await
        .map_err(|_| {
            WorkflowServiceError::RuntimeTimeout(format!(
                "timed out waiting for runtime '{}' to finish warmup or shutdown transition",
                runtime_id
            ))
        })?
    }

    async fn consume_runtime_warmup_disposition(
        &self,
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        runtime_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        match runtime_registry
            .warmup_disposition(runtime_id)
            .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?
            .decision
        {
            RuntimeWarmupDecision::ReuseLoadedRuntime => Ok(()),
            RuntimeWarmupDecision::StartRuntime => {
                let mode_info = self.gateway.mode_info().await;
                Self::reconcile_active_runtime_mode_info(runtime_registry, &mode_info);

                match runtime_registry
                    .warmup_disposition(runtime_id)
                    .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?
                    .decision
                {
                    RuntimeWarmupDecision::ReuseLoadedRuntime => Ok(()),
                    RuntimeWarmupDecision::WaitForTransition => {
                        self.wait_for_runtime_warmup_transition(runtime_registry, runtime_id)
                            .await
                    }
                    RuntimeWarmupDecision::StartRuntime => runtime_registry
                        .transition_runtime(
                            runtime_id,
                            RuntimeTransition::WarmupStarted {
                                runtime_instance_id: mode_info
                                    .active_runtime
                                    .as_ref()
                                    .and_then(|snapshot| snapshot.runtime_instance_id.clone()),
                            },
                        )
                        .map(|_| ())
                        .map_err(|error| WorkflowServiceError::Internal(error.to_string())),
                }
            }
            RuntimeWarmupDecision::WaitForTransition => {
                self.wait_for_runtime_warmup_transition(runtime_registry, runtime_id)
                    .await
            }
        }
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
        let (runtime_id, display_name, backend_keys, _) =
            Self::active_runtime_descriptor(&mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );

        runtime_registry.register_runtime(
            RuntimeRegistration::new(runtime_id.clone(), display_name)
                .with_backend_keys(backend_keys),
        );

        let lease = runtime_registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: runtime_id.clone(),
                workflow_id: workflow_id.to_string(),
                reservation_owner_id: Some(session_id.to_string()),
                usage_profile: Self::trimmed_optional(usage_profile),
                model_id: mode_info.active_model_target.clone(),
                pin_runtime: false,
                requirements,
                retention_hint: Self::runtime_retention_hint(retention_hint),
            })
            .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?;

        let previous_reservation_id =
            self.record_session_runtime_reservation(session_id, lease.reservation_id)?;
        if let Err(error) = self
            .consume_runtime_warmup_disposition(runtime_registry.as_ref(), &runtime_id)
            .await
        {
            self.restore_session_runtime_reservation(session_id, previous_reservation_id)?;
            if previous_reservation_id != Some(lease.reservation_id) {
                runtime_registry
                    .release_reservation_if_present(lease.reservation_id)
                    .map_err(|release_error| {
                        WorkflowServiceError::Internal(release_error.to_string())
                    })?;
            }
            return Err(error);
        }

        Ok(())
    }

    fn release_loaded_session_runtime(&self, session_id: &str) -> Result<(), WorkflowServiceError> {
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
            runtime_registry
                .release_reservation_if_present(reservation_id)
                .map_err(|error| WorkflowServiceError::Internal(error.to_string()))?;
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

    fn fallback_runtime_unload_candidate(
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Option<WorkflowSessionRuntimeUnloadCandidate> {
        candidates.iter().cloned().min_by(|left, right| {
            left.access_tick
                .cmp(&right.access_tick)
                .then_with(|| left.run_count.cmp(&right.run_count))
                .then_with(|| left.session_id.cmp(&right.session_id))
        })
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
        let mut runtimes = inference::list_binary_capabilities(&self.app_data_dir)
            .map_err(WorkflowServiceError::RuntimeNotReady)?
            .into_iter()
            .map(|runtime| {
                let backend_keys = Self::runtime_backend_keys(runtime.id);
                WorkflowRuntimeCapability {
                    runtime_id: runtime.id.key().to_string(),
                    display_name: runtime.display_name,
                    install_state: match runtime.install_state {
                        inference::ManagedBinaryInstallState::Installed => {
                            WorkflowRuntimeInstallState::Installed
                        }
                        inference::ManagedBinaryInstallState::SystemProvided => {
                            WorkflowRuntimeInstallState::SystemProvided
                        }
                        inference::ManagedBinaryInstallState::Missing => {
                            WorkflowRuntimeInstallState::Missing
                        }
                        inference::ManagedBinaryInstallState::Unsupported => {
                            WorkflowRuntimeInstallState::Unsupported
                        }
                    },
                    available: runtime.available,
                    configured: runtime.available,
                    can_install: runtime.can_install,
                    can_remove: runtime.can_remove,
                    source_kind: WorkflowRuntimeSourceKind::Managed,
                    selected: Self::runtime_matches_backend(&backend_keys, &selected_backend_key),
                    supports_external_connection: Self::runtime_supports_external_connection(
                        &available_backends,
                        &backend_keys,
                    ),
                    backend_keys,
                    missing_files: runtime.missing_files,
                    unavailable_reason: runtime.unavailable_reason,
                }
            })
            .collect::<Vec<_>>();
        runtimes.extend(
            available_backends.iter().filter_map(|backend| {
                Self::host_runtime_capability(backend, &selected_backend_key)
            }),
        );
        runtimes.extend(Self::python_runtime_capabilities(
            python_runtime::resolve_python_executable_for_env_ids(&[]),
            &selected_backend_key,
        ));
        runtimes.extend(self.additional_runtime_capabilities.clone());
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(runtimes)
    }

    async fn load_session_runtime(
        &self,
        session_id: &str,
        workflow_id: &str,
        usage_profile: Option<&str>,
        retention_hint: WorkflowSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.reserve_loaded_session_runtime(session_id, workflow_id, usage_profile, retention_hint)
            .await
    }

    async fn unload_session_runtime(
        &self,
        session_id: &str,
        _workflow_id: &str,
        _reason: pantograph_workflow_service::WorkflowSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        self.release_loaded_session_runtime(session_id)
    }

    async fn select_runtime_unload_candidate(
        &self,
        candidates: &[WorkflowSessionRuntimeUnloadCandidate],
    ) -> Result<Option<WorkflowSessionRuntimeUnloadCandidate>, WorkflowServiceError> {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return Ok(Self::fallback_runtime_unload_candidate(candidates));
        };

        let candidates_by_session_id = candidates
            .iter()
            .cloned()
            .map(|candidate| (candidate.session_id.clone(), candidate))
            .collect::<HashMap<_, _>>();
        let owner_ids = candidates_by_session_id
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if let Some(reservation) =
            runtime_registry.eviction_reservation_candidate_for_owners(&owner_ids)
        {
            if let Some(owner_id) = reservation.reservation_owner_id.as_deref() {
                if let Some(candidate) = candidates_by_session_id.get(owner_id) {
                    return Ok(Some(candidate.clone()));
                }
            }
        }

        Ok(Self::fallback_runtime_unload_candidate(candidates))
    }

    async fn run_workflow(
        &self,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: WorkflowRunOptions,
        run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if run_handle.is_cancelled() {
            return Err(WorkflowServiceError::RuntimeTimeout(
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

        let mut executor = WorkflowExecutor::new(execution_id, graph, Arc::new(NullEventSink));
        apply_runtime_extensions_with_python_runtime_recorder(
            &mut executor,
            &runtime_ext,
            Some(python_runtime_execution_recorder.clone()),
        );

        let mut node_outputs = HashMap::new();
        let mut run_result = Ok(());
        for node_id in &output_node_ids {
            if run_handle.is_cancelled() {
                run_result = Err(WorkflowServiceError::RuntimeTimeout(
                    "workflow run cancelled during execution".to_string(),
                ));
                break;
            }
            match executor.demand(node_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(node_id.clone(), outputs);
                }
                Err(error) => {
                    run_result = Err(WorkflowServiceError::Internal(error.to_string()));
                    break;
                }
            }
        }

        let python_runtime_execution_metadata = python_runtime_execution_recorder.snapshot();
        self.observe_python_runtime_execution_metadata(python_runtime_execution_metadata.as_ref())?;

        run_result?;
        Self::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{RerankRequest, RerankResponse};
    use pantograph_runtime_registry::{RuntimeRegistry, RuntimeRegistryStatus};
    use std::path::Path;
    use std::pin::Pin;
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    struct MockImagePythonRuntime {
        requests: Mutex<Vec<PythonNodeExecutionRequest>>,
    }

    struct MockReadyBackend {
        ready: bool,
    }

    struct MockProcessHandle;

    struct MockProcessSpawner;

    #[async_trait::async_trait]
    impl PythonRuntimeAdapter for MockImagePythonRuntime {
        async fn execute_node(
            &self,
            request: PythonNodeExecutionRequest,
        ) -> Result<HashMap<String, serde_json::Value>, String> {
            self.requests.lock().expect("requests lock").push(request);
            Ok(HashMap::from([(
                "image".to_string(),
                serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ=="),
            )]))
        }
    }

    impl ProcessHandle for MockProcessHandle {
        fn pid(&self) -> u32 {
            1
        }

        fn kill(&self) -> Result<(), String> {
            Ok(())
        }
    }

    #[async_trait::async_trait]
    impl ProcessSpawner for MockProcessSpawner {
        async fn spawn_sidecar(
            &self,
            _sidecar_name: &str,
            _args: &[&str],
        ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
            let (_tx, rx) = mpsc::channel(1);
            Ok((rx, Box::new(MockProcessHandle)))
        }

        fn app_data_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }

        fn binaries_dir(&self) -> Result<PathBuf, String> {
            Ok(PathBuf::from("/tmp"))
        }
    }

    #[async_trait::async_trait]
    impl InferenceBackend for MockReadyBackend {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn description(&self) -> &'static str {
            "Mock ready backend"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            _config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            self.ready = true;
            Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
            })
        }

        fn stop(&mut self) {
            self.ready = false;
        }

        fn is_ready(&self) -> bool {
            self.ready
        }

        async fn health_check(&self) -> bool {
            self.ready
        }

        fn base_url(&self) -> Option<String> {
            None
        }

        async fn chat_completion_stream(
            &self,
            _request_json: String,
        ) -> Result<
            Pin<Box<dyn futures_util::Stream<Item = Result<ChatChunk, BackendError>> + Send>>,
            BackendError,
        > {
            Ok(Box::pin(stream::empty()))
        }

        async fn embeddings(
            &self,
            _texts: Vec<String>,
            _model: &str,
        ) -> Result<Vec<EmbeddingResult>, BackendError> {
            Ok(Vec::new())
        }

        async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
            Ok(RerankResponse {
                results: Vec::new(),
                metadata: serde_json::Value::Null,
            })
        }
    }

    fn install_fake_default_runtime(app_data_dir: &Path) {
        let runtime_dir = app_data_dir.join("runtimes").join("llama-cpp");
        std::fs::create_dir_all(&runtime_dir).expect("create fake runtime dir");

        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            for file_name in [
                "llama-server-x86_64-unknown-linux-gnu",
                "libllama.so",
                "libggml.so",
            ] {
                std::fs::write(runtime_dir.join(file_name), [])
                    .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
            }
        }

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            for file_name in ["llama-server-aarch64-apple-darwin", "libllama.dylib"] {
                std::fs::write(runtime_dir.join(file_name), [])
                    .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
            }
        }

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            for file_name in ["llama-server-x86_64-apple-darwin", "libllama.dylib"] {
                std::fs::write(runtime_dir.join(file_name), [])
                    .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
            }
        }

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            for file_name in [
                "llama-server-x86_64-pc-windows-msvc.exe",
                "llama-runtime.dll",
            ] {
                std::fs::write(runtime_dir.join(file_name), [])
                    .unwrap_or_else(|_| panic!("write fake runtime file {file_name}"));
            }
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
                            "description": "Prompt supplied by the caller",
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Text Input",
                                "description": "Provides text input",
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "legacy-out",
                                        "label": "Legacy Out",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
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
                                "inputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    },
                                    {
                                        "id": "stream",
                                        "label": "Stream",
                                        "data_type": "stream",
                                        "required": false,
                                        "multiple": false
                                    }
                                ],
                                "outputs": [
                                    {
                                        "id": "text",
                                        "label": "Text",
                                        "data_type": "string",
                                        "required": false,
                                        "multiple": false
                                    }
                                ]
                            }
                        },
                        "position": { "x": 200.0, "y": 0.0 }
                    }
                ],
                "edges": [
                    {
                        "id": "e-text",
                        "source": "text-input-1",
                        "source_handle": "text",
                        "target": "text-output-1",
                        "target_handle": "text"
                    }
                ]
            }
        });
        std::fs::write(
            workflows_dir.join(format!("{workflow_id}.json")),
            serde_json::to_vec(&workflow_json).expect("serialize workflow"),
        )
        .expect("write workflow");
    }

    fn workflow_port_definition(id: &str, label: &str, data_type: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "label": label,
            "data_type": data_type,
            "required": false,
            "multiple": false
        })
    }

    fn write_mock_diffusion_workflow(root: &Path, workflow_id: &str) {
        let workflows_dir = root.join(".pantograph").join("workflows");
        std::fs::create_dir_all(&workflows_dir).expect("create workflows dir");
        let workflow_json = serde_json::json!({
            "version": "1.0",
            "metadata": {
                "name": "Mock Diffusion Workflow",
                "created": "2026-01-01T00:00:00Z",
                "modified": "2026-01-01T00:00:00Z"
            },
            "graph": {
                "nodes": [
                    {
                        "id": "text-input-1",
                        "node_type": "text-input",
                        "data": {
                            "definition": {
                                "category": "input",
                                "io_binding_origin": "client_session",
                                "label": "Prompt",
                                "description": "Prompt supplied by the caller",
                                "inputs": [workflow_port_definition("text", "Text", "string")],
                                "outputs": [workflow_port_definition("text", "Text", "string")]
                            },
                            "text": "a tiny painted robot"
                        },
                        "position": { "x": 0.0, "y": 0.0 }
                    },
                    {
                        "id": "diffusion-inference-1",
                        "node_type": "diffusion-inference",
                        "data": {
                            "model_path": "/tmp/mock-diffusion-model",
                            "model_type": "diffusion",
                            "environment_ref": {
                                "state": "ready",
                                "env_ids": ["mock-python-env"]
                            }
                        },
                        "position": { "x": 240.0, "y": 0.0 }
                    },
                    {
                        "id": "image-output-1",
                        "node_type": "image-output",
                        "data": {
                            "definition": {
                                "category": "output",
                                "io_binding_origin": "client_session",
                                "label": "Generated Image",
                                "description": "Generated image output",
                                "inputs": [workflow_port_definition("image", "Image", "image")],
                                "outputs": [workflow_port_definition("image", "Image", "image")]
                            }
                        },
                        "position": { "x": 520.0, "y": 0.0 }
                    }
                ],
                "edges": [
                    {
                        "id": "e-prompt",
                        "source": "text-input-1",
                        "source_handle": "text",
                        "target": "diffusion-inference-1",
                        "target_handle": "prompt"
                    },
                    {
                        "id": "e-image",
                        "source": "diffusion-inference-1",
                        "source_handle": "image",
                        "target": "image-output-1",
                        "target_handle": "image"
                    }
                ]
            }
        });
        std::fs::write(
            workflows_dir.join(format!("{workflow_id}.json")),
            serde_json::to_vec(&workflow_json).expect("serialize workflow"),
        )
        .expect("write workflow");
    }

    #[tokio::test]
    async fn test_runtime_run_and_session_execution() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

        let run_response = runtime
            .workflow_run(WorkflowRunRequest {
                workflow_id: "runtime-text".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                timeout_ms: None,
                run_id: Some("run-1".to_string()),
            })
            .await
            .expect("workflow run");
        assert_eq!(run_response.outputs.len(), 1);
        assert_eq!(run_response.outputs[0].value, serde_json::json!("hello"));

        let created = runtime
            .create_workflow_session(WorkflowSessionCreateRequest {
                workflow_id: "runtime-text".to_string(),
                usage_profile: None,
                keep_alive: false,
            })
            .await
            .expect("create session");

        let session_response = runtime
            .run_workflow_session(WorkflowSessionRunRequest {
                session_id: created.session_id.clone(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                timeout_ms: None,
                priority: None,
                run_id: Some("run-2".to_string()),
            })
            .await
            .expect("run session");
        assert_eq!(session_response.outputs.len(), 1);
        assert_eq!(
            session_response.outputs[0].value,
            serde_json::json!("world")
        );

        runtime
            .close_workflow_session(WorkflowSessionCloseRequest {
                session_id: created.session_id,
            })
            .await
            .expect("close session");
    }

    #[tokio::test]
    async fn test_runtime_routes_diffusion_workflow_through_python_adapter() {
        let temp = TempDir::new().expect("temp dir");
        write_mock_diffusion_workflow(temp.path(), "runtime-diffusion");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let python_runtime = Arc::new(MockImagePythonRuntime {
            requests: Mutex::new(Vec::new()),
        });
        let runtime = EmbeddedRuntime::from_components(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
            python_runtime.clone(),
        )
        .with_runtime_registry(Arc::new(RuntimeRegistry::new()));

        let response = runtime
            .workflow_run(WorkflowRunRequest {
                workflow_id: "runtime-diffusion".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("a tiny painted robot"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "image-output-1".to_string(),
                    port_id: "image".to_string(),
                }]),
                timeout_ms: None,
                run_id: Some("diffusion-run-1".to_string()),
            })
            .await
            .expect("workflow run");

        assert_eq!(response.outputs.len(), 1);
        assert_eq!(response.outputs[0].node_id, "image-output-1");
        assert_eq!(response.outputs[0].port_id, "image");
        assert_eq!(
            response.outputs[0].value,
            serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ==")
        );

        let requests = python_runtime.requests.lock().expect("requests lock");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].node_type, "diffusion-inference");
        assert_eq!(
            requests[0].inputs.get("prompt"),
            Some(&serde_json::json!("a tiny painted robot"))
        );
    }

    #[tokio::test]
    async fn test_runtime_run_reconciles_python_sidecar_runtime_into_registry() {
        let temp = TempDir::new().expect("temp dir");
        write_mock_diffusion_workflow(temp.path(), "runtime-diffusion");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::from_components(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
            Arc::new(MockImagePythonRuntime {
                requests: Mutex::new(Vec::new()),
            }),
        )
        .with_runtime_registry(runtime_registry.clone());

        runtime
            .workflow_run(WorkflowRunRequest {
                workflow_id: "runtime-diffusion".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("a tiny painted robot"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "image-output-1".to_string(),
                    port_id: "image".to_string(),
                }]),
                timeout_ms: None,
                run_id: Some("diffusion-run-2".to_string()),
            })
            .await
            .expect("workflow run");

        let snapshot = runtime_registry.snapshot();
        let pytorch = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("python runtime should be observed");
        assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
        assert_eq!(pytorch.status, RuntimeRegistryStatus::Ready);
        assert!(pytorch
            .runtime_instance_id
            .as_deref()
            .is_some_and(|value| value.starts_with("python-runtime:pytorch:")));
        assert_eq!(pytorch.models.len(), 1);
        assert_eq!(pytorch.models[0].model_id, "/tmp/mock-diffusion-model");
    }

    #[tokio::test]
    async fn test_keep_alive_session_load_tracks_registry_reservation_lifecycle() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry.clone());

        let created = runtime
            .create_workflow_session(WorkflowSessionCreateRequest {
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            })
            .await
            .expect("create keep-alive session");

        let reserved_snapshot = runtime_registry.snapshot();
        assert_eq!(reserved_snapshot.reservations.len(), 1);
        assert_eq!(
            reserved_snapshot.reservations[0].workflow_id,
            "runtime-text"
        );
        assert_eq!(
            reserved_snapshot.reservations[0].usage_profile.as_deref(),
            Some("interactive")
        );
        assert_eq!(
            reserved_snapshot.reservations[0].retention_hint,
            RuntimeRetentionHint::KeepAlive
        );
        assert_eq!(
            reserved_snapshot.runtimes[0].active_reservation_ids.len(),
            1
        );
        assert_eq!(
            reserved_snapshot.runtimes[0].status,
            RuntimeRegistryStatus::Warming
        );

        runtime
            .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
                session_id: created.session_id.clone(),
                keep_alive: false,
            })
            .await
            .expect("disable keep alive");

        let released_snapshot = runtime_registry.snapshot();
        assert!(released_snapshot.reservations.is_empty());
        assert!(released_snapshot.runtimes[0]
            .active_reservation_ids
            .is_empty());
    }

    #[tokio::test]
    async fn test_session_runtime_load_reuses_ready_gateway_runtime_in_registry() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "mock",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            gateway,
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry.clone());

        runtime
            .host()
            .load_session_runtime(
                "session-ready",
                "runtime-text",
                Some("interactive"),
                WorkflowSessionRetentionHint::KeepAlive,
            )
            .await
            .expect("ready runtime should be reused");

        let snapshot = runtime_registry.snapshot();
        assert_eq!(snapshot.reservations.len(), 1);
        assert_eq!(snapshot.runtimes.len(), 1);
        assert_eq!(snapshot.runtimes[0].runtime_id, "mock");
        assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
        assert!(snapshot.runtimes[0].runtime_instance_id.is_some());
    }

    #[tokio::test]
    async fn test_session_runtime_load_waits_for_existing_warmup_transition() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
        runtime_registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::WarmupStarted {
                    runtime_instance_id: Some("llama-1".to_string()),
                },
            )
            .expect("runtime should enter warming");

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry.clone());

        let ready_registry = runtime_registry.clone();
        let ready_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            ready_registry
                .transition_runtime(
                    "llama.cpp",
                    RuntimeTransition::Ready {
                        runtime_instance_id: Some("llama-1".to_string()),
                    },
                )
                .expect("runtime should become ready");
        });

        runtime
            .host()
            .load_session_runtime(
                "session-wait",
                "runtime-text",
                None,
                WorkflowSessionRetentionHint::KeepAlive,
            )
            .await
            .expect("load should wait for warmup completion");
        ready_task.await.expect("ready transition task");

        let snapshot = runtime_registry.snapshot();
        assert_eq!(snapshot.reservations.len(), 1);
        assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Ready);
        assert_eq!(
            snapshot.runtimes[0].runtime_instance_id.as_deref(),
            Some("llama-1")
        );
    }

    #[tokio::test]
    async fn test_session_runtime_load_releases_reservation_after_warmup_timeout() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
        runtime_registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::WarmupStarted {
                    runtime_instance_id: Some("llama-timeout".to_string()),
                },
            )
            .expect("runtime should enter warming");

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry.clone());

        let error = runtime
            .host()
            .load_session_runtime(
                "session-timeout",
                "runtime-text",
                None,
                WorkflowSessionRetentionHint::KeepAlive,
            )
            .await
            .expect_err("warming timeout should fail");
        assert!(matches!(error, WorkflowServiceError::RuntimeTimeout(_)));

        let snapshot = runtime_registry.snapshot();
        assert!(snapshot.reservations.is_empty());
        assert!(snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty()));
    }

    #[tokio::test]
    async fn test_session_run_without_keep_alive_releases_runtime_reservation_after_run() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry.clone());

        let created = runtime
            .create_workflow_session(WorkflowSessionCreateRequest {
                workflow_id: "runtime-text".to_string(),
                usage_profile: None,
                keep_alive: false,
            })
            .await
            .expect("create session");

        let run_response = runtime
            .run_workflow_session(WorkflowSessionRunRequest {
                session_id: created.session_id,
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("session-world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "text-output-1".to_string(),
                    port_id: "text".to_string(),
                }]),
                timeout_ms: None,
                priority: None,
                run_id: Some("run-queued".to_string()),
            })
            .await
            .expect("run queued session");
        assert_eq!(run_response.outputs.len(), 1);
        assert_eq!(
            run_response.outputs[0].value,
            serde_json::json!("session-world")
        );

        let snapshot = runtime_registry.snapshot();
        assert!(snapshot.reservations.is_empty());
        assert!(snapshot
            .runtimes
            .iter()
            .all(|runtime| runtime.active_reservation_ids.is_empty()));
    }

    #[tokio::test]
    async fn test_runtime_unload_candidate_selection_uses_registry_eviction_order() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        runtime_registry.observe_runtimes(vec![pantograph_runtime_registry::RuntimeObservation {
            runtime_id: "shared-runtime".to_string(),
            display_name: "shared-runtime".to_string(),
            backend_keys: vec!["llama_cpp".to_string()],
            model_id: Some("model-a".to_string()),
            status: pantograph_runtime_registry::RuntimeRegistryStatus::Ready,
            runtime_instance_id: Some("shared-runtime-1".to_string()),
            last_error: None,
        }]);
        runtime_registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "shared-runtime".to_string(),
                workflow_id: "wf-a".to_string(),
                reservation_owner_id: Some("session-a".to_string()),
                usage_profile: Some("interactive".to_string()),
                model_id: Some("model-a".to_string()),
                pin_runtime: false,
                requirements: None,
                retention_hint: RuntimeRetentionHint::KeepAlive,
            })
            .expect("keep-alive reservation");
        runtime_registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "shared-runtime".to_string(),
                workflow_id: "wf-b".to_string(),
                reservation_owner_id: Some("session-b".to_string()),
                usage_profile: Some("batch".to_string()),
                model_id: Some("model-a".to_string()),
                pin_runtime: false,
                requirements: None,
                retention_hint: RuntimeRetentionHint::Ephemeral,
            })
            .expect("ephemeral reservation");

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_runtime_registry(runtime_registry);

        let selected = runtime
            .host()
            .select_runtime_unload_candidate(&[
                WorkflowSessionRuntimeUnloadCandidate {
                    session_id: "session-a".to_string(),
                    workflow_id: "wf-a".to_string(),
                    keep_alive: true,
                    access_tick: 1,
                    run_count: 0,
                },
                WorkflowSessionRuntimeUnloadCandidate {
                    session_id: "session-b".to_string(),
                    workflow_id: "wf-b".to_string(),
                    keep_alive: false,
                    access_tick: 99,
                    run_count: 5,
                },
            ])
            .await
            .expect("select unload candidate")
            .expect("candidate should exist");

        assert_eq!(selected.session_id, "session-b");
    }

    #[test]
    fn python_runtime_capabilities_report_python_backed_engines() {
        let capabilities = EmbeddedWorkflowHost::python_runtime_capabilities(
            Ok(PathBuf::from("/usr/bin/python3")),
            "pytorch",
        );

        assert_eq!(capabilities.len(), 4);

        let pytorch = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "pytorch")
            .expect("pytorch capability");
        assert!(pytorch.available);
        assert!(pytorch.configured);
        assert_eq!(pytorch.source_kind, WorkflowRuntimeSourceKind::System);
        assert!(pytorch.selected);
        assert!(!pytorch.supports_external_connection);
        assert!(pytorch.backend_keys.contains(&"pytorch".to_string()));
        assert!(pytorch.backend_keys.contains(&"torch".to_string()));

        let diffusion = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "diffusers")
            .expect("diffusers capability");
        assert!(diffusion.backend_keys.contains(&"diffusers".to_string()));

        let onnx = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "onnx-runtime")
            .expect("onnx capability");
        assert!(onnx.backend_keys.contains(&"onnx-runtime".to_string()));

        let stable_audio = capabilities
            .iter()
            .find(|capability| capability.runtime_id == "stable_audio")
            .expect("stable audio capability");
        assert!(stable_audio
            .backend_keys
            .contains(&"stable_audio".to_string()));
    }

    #[test]
    fn python_runtime_capabilities_keep_unavailable_reason() {
        let capabilities = EmbeddedWorkflowHost::python_runtime_capabilities(
            Err("python executable is not configured".to_string()),
            "llama_cpp",
        );

        assert_eq!(capabilities.len(), 4);
        for capability in capabilities {
            assert!(!capability.available);
            assert!(!capability.configured);
            assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::System);
            assert!(!capability.selected);
            assert_eq!(
                capability.unavailable_reason.as_deref(),
                Some("python executable is not configured")
            );
        }
    }

    #[test]
    fn host_runtime_capability_reports_candle_backend() {
        let capability = EmbeddedWorkflowHost::host_runtime_capability(
            &inference::BackendInfo {
                name: "Candle".to_string(),
                backend_key: "candle".to_string(),
                description: "In-process Candle inference".to_string(),
                capabilities: inference::BackendCapabilities {
                    external_connection: false,
                    ..inference::BackendCapabilities::default()
                },
                default_start_mode: inference::backend::BackendDefaultStartMode::Embedding,
                active: true,
                available: true,
                unavailable_reason: None,
                can_install: false,
                runtime_binary_id: None,
            },
            "candle",
        )
        .expect("candle host capability");

        assert_eq!(capability.runtime_id, "candle");
        assert_eq!(capability.display_name, "Candle");
        assert_eq!(
            capability.install_state,
            WorkflowRuntimeInstallState::SystemProvided
        );
        assert_eq!(capability.source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(capability.selected);
        assert!(capability.backend_keys.contains(&"candle".to_string()));
        assert!(capability.backend_keys.contains(&"Candle".to_string()));
    }

    #[tokio::test]
    async fn workflow_preflight_reports_candle_runtime_as_available() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::with_backend(
                Box::new(inference::CandleBackend::new()),
                "Candle",
            )),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        );

        let response = runtime
            .workflow_preflight(WorkflowPreflightRequest {
                workflow_id: "runtime-text".to_string(),
                inputs: Vec::new(),
                output_targets: None,
            })
            .await
            .expect("workflow preflight");

        assert!(response.blocking_runtime_issues.is_empty());
        assert!(response.can_run);

        let capabilities = runtime
            .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                workflow_id: "runtime-text".to_string(),
            })
            .await
            .expect("workflow capabilities");
        assert_eq!(
            capabilities.runtime_requirements.required_backends,
            vec!["candle".to_string()]
        );
        let candle = capabilities
            .runtime_capabilities
            .iter()
            .find(|capability| capability.runtime_id == "candle")
            .expect("candle capability");
        assert_eq!(candle.source_kind, WorkflowRuntimeSourceKind::Host);
        assert!(candle.selected);
    }

    #[tokio::test]
    async fn workflow_capabilities_include_injected_runtime_capabilities() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime = EmbeddedRuntime::with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        )
        .with_additional_runtime_capabilities(vec![WorkflowRuntimeCapability {
            runtime_id: "llama.cpp.embedding".to_string(),
            display_name: "Dedicated embedding runtime".to_string(),
            install_state: WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: WorkflowRuntimeSourceKind::Host,
            selected: false,
            supports_external_connection: false,
            backend_keys: vec!["llama_cpp".to_string(), "llamacpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }]);

        let capabilities = runtime
            .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                workflow_id: "runtime-text".to_string(),
            })
            .await
            .expect("workflow capabilities");

        let embedding_runtime = capabilities
            .runtime_capabilities
            .iter()
            .find(|capability| capability.runtime_id == "llama.cpp.embedding")
            .expect("dedicated embedding capability");
        assert_eq!(
            embedding_runtime.source_kind,
            WorkflowRuntimeSourceKind::Host
        );
        assert!(!embedding_runtime.selected);
        assert!(embedding_runtime.available);
    }

    #[test]
    fn reservation_requirements_returns_none_when_workflow_estimate_is_unknown() {
        assert_eq!(
            EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements::default()),
            None
        );
    }

    #[test]
    fn reservation_requirements_maps_workflow_memory_estimates() {
        let requirements =
            EmbeddedWorkflowHost::reservation_requirements(&WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(2048),
                estimated_peak_ram_mb: Some(1024),
                estimated_min_vram_mb: Some(1536),
                estimated_min_ram_mb: Some(768),
                estimation_confidence: "estimated_from_model_sizes".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: Vec::new(),
            })
            .expect("requirements should be forwarded when estimates exist");

        assert_eq!(requirements.estimated_peak_vram_mb, Some(2048));
        assert_eq!(requirements.estimated_peak_ram_mb, Some(1024));
        assert_eq!(requirements.estimated_min_vram_mb, Some(1536));
        assert_eq!(requirements.estimated_min_ram_mb, Some(768));
    }
}
