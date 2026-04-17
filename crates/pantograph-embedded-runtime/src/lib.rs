use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use node_engine::{
    CoreTaskExecutor, EventSink, ExecutorExtensions, NullEventSink, WorkflowExecutor, WorkflowGraph,
};
use pantograph_runtime_identity::{backend_key_aliases, canonical_runtime_backend_key};
use pantograph_runtime_registry::{
    RuntimeRegistryError, RuntimeReservationRequest, RuntimeReservationRequirements,
    RuntimeRetentionDecision, RuntimeRetentionHint, RuntimeTransition, RuntimeWarmupDecision,
    SharedRuntimeRegistry,
};
use pantograph_workflow_service::capabilities;
use pantograph_workflow_service::{
    convert_graph_to_node_engine, ConnectionCandidatesResponse, ConnectionCommitResponse,
    EdgeInsertionPreviewResponse, FileSystemWorkflowGraphStore, InsertNodeConnectionResponse,
    InsertNodeOnEdgeResponse, WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse,
    WorkflowFile, WorkflowGraphAddEdgeRequest, WorkflowGraphAddNodeRequest,
    WorkflowGraphConnectRequest, WorkflowGraphEditSessionCloseRequest,
    WorkflowGraphEditSessionCloseResponse, WorkflowGraphEditSessionCreateRequest,
    WorkflowGraphEditSessionCreateResponse, WorkflowGraphEditSessionGraphRequest,
    WorkflowGraphEditSessionGraphResponse, WorkflowGraphGetConnectionCandidatesRequest,
    WorkflowGraphInsertNodeAndConnectRequest, WorkflowGraphInsertNodeOnEdgeRequest,
    WorkflowGraphListResponse, WorkflowGraphLoadRequest,
    WorkflowGraphPreviewNodeInsertOnEdgeRequest, WorkflowGraphRemoveEdgeRequest,
    WorkflowGraphRemoveNodeRequest, WorkflowGraphSaveRequest, WorkflowGraphSaveResponse,
    WorkflowGraphUndoRedoStateRequest, WorkflowGraphUndoRedoStateResponse,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest, WorkflowHost,
    WorkflowHostModelDescriptor, WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunOptions,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeCapability,
    WorkflowRuntimeInstallState, WorkflowRuntimeRequirements, WorkflowRuntimeSourceKind,
    WorkflowSchedulerDiagnosticsProvider, WorkflowSchedulerRuntimeDiagnosticsRequest,
    WorkflowSchedulerRuntimeRegistryDiagnostics, WorkflowSchedulerRuntimeWarmupDecision,
    WorkflowSchedulerRuntimeWarmupReason, WorkflowService, WorkflowServiceError,
    WorkflowSessionCloseRequest, WorkflowSessionCloseResponse, WorkflowSessionCreateRequest,
    WorkflowSessionCreateResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRetentionHint,
    WorkflowSessionRunRequest, WorkflowSessionRuntimeSelectionTarget,
    WorkflowSessionRuntimeUnloadCandidate, WorkflowSessionStaleCleanupRequest,
    WorkflowSessionStaleCleanupResponse, WorkflowSessionState, WorkflowSessionStatusRequest,
    WorkflowSessionStatusResponse, WorkflowTechnicalFitDecision, WorkflowTechnicalFitRequest,
    WorkflowTraceRuntimeMetrics,
};
use tokio::sync::RwLock;
use uuid::Uuid;

pub mod embedding_workflow;
pub mod host_runtime;
pub mod model_dependencies;
pub mod python_runtime;
mod python_runtime_execution;
pub mod rag;
pub mod runtime_capabilities;
pub mod runtime_health;
pub mod runtime_recovery;
pub mod runtime_registry;
mod runtime_registry_lifecycle;
mod runtime_registry_observations;
pub mod task_executor;
pub mod technical_fit;
pub mod workflow_runtime;

pub use host_runtime::HostRuntimeModeSnapshot;
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
    apply_runtime_extensions_for_execution(executor, snapshot, None, None, None);
}

#[derive(Clone)]
struct EmbeddedWorkflowSchedulerDiagnosticsProvider {
    gateway: Arc<inference::InferenceGateway>,
    runtime_registry: SharedRuntimeRegistry,
}

impl EmbeddedWorkflowSchedulerDiagnosticsProvider {
    fn new(
        gateway: Arc<inference::InferenceGateway>,
        runtime_registry: SharedRuntimeRegistry,
    ) -> Self {
        Self {
            gateway,
            runtime_registry,
        }
    }
}

fn runtime_registry_reclaim_candidate_for_sessions(
    runtime_registry: &SharedRuntimeRegistry,
    candidates: &[WorkflowSessionRuntimeUnloadCandidate],
) -> Option<(String, String)> {
    let candidates_by_session_id = candidates
        .iter()
        .map(|candidate| (candidate.session_id.clone(), candidate))
        .collect::<HashMap<_, _>>();
    let owner_ids = candidates_by_session_id
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let reservation = runtime_registry.eviction_reservation_candidate_for_owners(&owner_ids)?;
    let owner_id = reservation.reservation_owner_id?;
    let candidate = candidates_by_session_id.get(&owner_id)?;
    Some((candidate.session_id.clone(), reservation.runtime_id))
}

#[async_trait]
impl WorkflowSchedulerDiagnosticsProvider for EmbeddedWorkflowSchedulerDiagnosticsProvider {
    async fn scheduler_runtime_registry_diagnostics(
        &self,
        request: &WorkflowSchedulerRuntimeDiagnosticsRequest,
    ) -> Result<Option<WorkflowSchedulerRuntimeRegistryDiagnostics>, WorkflowServiceError> {
        let mode_info = self.gateway.mode_info().await;
        let host_runtime_mode_info = HostRuntimeModeSnapshot::from_mode_info(&mode_info);
        let descriptor = runtime_registry::register_active_runtime(
            &self.runtime_registry,
            &host_runtime_mode_info,
        );

        let reclaim_candidate = runtime_registry_reclaim_candidate_for_sessions(
            &self.runtime_registry,
            &request.reclaim_candidates,
        );
        let warmup_disposition = if request.next_admission_queue_id.is_some() {
            Some(
                self.runtime_registry
                    .warmup_disposition(&descriptor.runtime_id)
                    .map_err(EmbeddedWorkflowHost::workflow_service_error_from_runtime_registry)?,
            )
        } else {
            None
        };

        Ok(Some(WorkflowSchedulerRuntimeRegistryDiagnostics {
            target_runtime_id: Some(descriptor.runtime_id),
            reclaim_candidate_session_id: reclaim_candidate
                .as_ref()
                .map(|(session_id, _)| session_id.clone()),
            reclaim_candidate_runtime_id: reclaim_candidate.map(|(_, runtime_id)| runtime_id),
            next_warmup_decision: warmup_disposition
                .as_ref()
                .map(|disposition| match disposition.decision {
                    RuntimeWarmupDecision::StartRuntime => {
                        WorkflowSchedulerRuntimeWarmupDecision::StartRuntime
                    }
                    RuntimeWarmupDecision::ReuseLoadedRuntime => {
                        WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime
                    }
                    RuntimeWarmupDecision::WaitForTransition => {
                        WorkflowSchedulerRuntimeWarmupDecision::WaitForTransition
                    }
                }),
            next_warmup_reason: warmup_disposition.as_ref().map(|disposition| {
                match disposition.reason {
                    pantograph_runtime_registry::RuntimeWarmupReason::NoLoadedInstance => {
                        WorkflowSchedulerRuntimeWarmupReason::NoLoadedInstance
                    }
                    pantograph_runtime_registry::RuntimeWarmupReason::RecoveryRequired => {
                        WorkflowSchedulerRuntimeWarmupReason::RecoveryRequired
                    }
                    pantograph_runtime_registry::RuntimeWarmupReason::LoadedInstanceReady => {
                        WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady
                    }
                    pantograph_runtime_registry::RuntimeWarmupReason::LoadedInstanceBusy => {
                        WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceBusy
                    }
                    pantograph_runtime_registry::RuntimeWarmupReason::WarmupInProgress => {
                        WorkflowSchedulerRuntimeWarmupReason::WarmupInProgress
                    }
                    pantograph_runtime_registry::RuntimeWarmupReason::StopInProgress => {
                        WorkflowSchedulerRuntimeWarmupReason::StopInProgress
                    }
                }
            }),
        }))
    }
}

#[async_trait]
impl runtime_registry::HostRuntimeRegistryController for inference::InferenceGateway {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        HostRuntimeModeSnapshot::from_mode_info(&self.mode_info().await)
    }

    async fn stop_runtime_producer(&self, producer: runtime_registry::HostRuntimeProducer) {
        match producer {
            runtime_registry::HostRuntimeProducer::Active => self.stop().await,
            runtime_registry::HostRuntimeProducer::Embedding => {
                debug_assert!(
                    false,
                    "embedded inference gateway cannot stop a dedicated embedding producer"
                );
            }
        }
    }
}

#[async_trait]
impl runtime_registry::HostRuntimeRegistryLifecycleController for inference::InferenceGateway {
    async fn stop_all_runtime_producers(&self) {
        self.stop().await;
    }

    async fn restore_runtime(
        &self,
        restore_config: Option<inference::BackendConfig>,
    ) -> Result<(), inference::GatewayError> {
        self.restore_inference_runtime(restore_config).await
    }
}

pub fn apply_runtime_extensions_for_execution(
    executor: &mut WorkflowExecutor,
    snapshot: &RuntimeExtensionsSnapshot,
    event_sink: Option<Arc<dyn EventSink>>,
    execution_id: Option<String>,
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
    if let Some(event_sink) = event_sink {
        executor.extensions_mut().set(
            task_executor::runtime_extension_keys::EVENT_SINK,
            event_sink,
        );
    }
    if let Some(execution_id) = execution_id {
        executor.extensions_mut().set(
            task_executor::runtime_extension_keys::EXECUTION_ID,
            execution_id,
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

#[derive(Debug, Clone)]
pub struct EditSessionGraphExecutionOutcome {
    pub runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub runtime_model_target: Option<String>,
    pub waiting_for_input: bool,
    pub error: Option<String>,
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

    pub async fn hosted_with_default_python_runtime(
        config: EmbeddedRuntimeConfig,
        gateway: Arc<inference::InferenceGateway>,
        extensions: SharedExtensions,
        workflow_service: SharedWorkflowService,
        rag_backend: Option<Arc<dyn RagBackend>>,
        runtime_registry: Option<SharedRuntimeRegistry>,
        host_runtime_mode_info: Option<HostRuntimeModeSnapshot>,
    ) -> Self {
        if let (Some(runtime_registry), Some(mode_info)) =
            (runtime_registry.as_ref(), host_runtime_mode_info.as_ref())
        {
            runtime_registry::reconcile_runtime_registry_mode_info(
                runtime_registry.as_ref(),
                mode_info,
            );
        }

        let additional_runtime_capabilities = host_runtime_mode_info
            .as_ref()
            .map(runtime_capabilities::runtime_capabilities_from_mode_info)
            .unwrap_or_default();

        let mut runtime = Self::with_default_python_runtime(
            config,
            gateway.clone(),
            extensions,
            workflow_service,
            rag_backend,
        )
        .with_additional_runtime_capabilities(additional_runtime_capabilities);

        if let Some(runtime_registry) = runtime_registry {
            runtime = runtime.with_runtime_registry(runtime_registry);
        }

        runtime
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
        self.workflow_service
            .set_scheduler_diagnostics_provider(Some(Arc::new(
                EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
                    self.gateway.clone(),
                    runtime_registry.clone(),
                ),
            )))
            .expect("scheduler diagnostics provider should be configured");
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

    async fn reconcile_runtime_registry_from_gateway(&self) {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return;
        };

        runtime_registry::sync_runtime_registry(self.gateway.as_ref(), runtime_registry.as_ref())
            .await;
    }

    pub async fn shutdown(&self) {
        if let Some(runtime_registry) = self.runtime_registry.as_ref() {
            runtime_registry::stop_all_runtime_producers_and_reconcile_runtime_registry(
                self.gateway.as_ref(),
                runtime_registry.as_ref(),
            )
            .await;
        } else {
            self.gateway.stop().await;
        }
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
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowServiceError> {
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
        for terminal_id in &terminal_nodes {
            match executor.demand(terminal_id, &task_executor).await {
                Ok(outputs) => {
                    node_outputs.insert(terminal_id.clone(), outputs);
                }
                Err(error) => {
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
            .observe_python_runtime_execution_metadata(&python_runtime_execution_metadata)?;

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

        let _ = event_sink.send(node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: session_id.to_string(),
            execution_id: session_id.to_string(),
        });

        let mut workflow_result: Result<(), String> = Ok(());
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
                    workflow_result = Err(error.to_string());
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
            let _ = event_sink.send(node_engine::WorkflowEvent::WorkflowCompleted {
                workflow_id: session_id.to_string(),
                execution_id: session_id.to_string(),
            });
        } else if let Err(error) = &workflow_result {
            let _ = event_sink.send(node_engine::WorkflowEvent::WorkflowFailed {
                workflow_id: session_id.to_string(),
                execution_id: session_id.to_string(),
                error: error.clone(),
            });
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
            error: workflow_result.err(),
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

    fn workflow_service_error_from_runtime_registry(
        error: RuntimeRegistryError,
    ) -> WorkflowServiceError {
        match error {
            RuntimeRegistryError::RuntimeNotFound(_)
            | RuntimeRegistryError::ReservationRejected(_)
            | RuntimeRegistryError::AdmissionRejected { .. } => {
                WorkflowServiceError::RuntimeNotReady(error.to_string())
            }
            RuntimeRegistryError::ReservationOwnerConflict { .. } => {
                WorkflowServiceError::InvalidRequest(error.to_string())
            }
            RuntimeRegistryError::ReservationNotFound(_)
            | RuntimeRegistryError::InvalidTransition { .. } => {
                WorkflowServiceError::Internal(error.to_string())
            }
        }
    }

    fn reconcile_active_runtime_mode_info(
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        mode_info: &inference::ServerModeInfo,
        include_stopped: bool,
    ) {
        let snapshot = HostRuntimeModeSnapshot::from_mode_info(mode_info);
        runtime_registry::reconcile_active_runtime_mode_info(
            runtime_registry,
            &snapshot,
            include_stopped,
        );
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

        runtime_registry
            .update_reservation_retention_hint_if_present(
                reservation_id,
                Self::runtime_retention_hint(if keep_alive {
                    WorkflowSessionRetentionHint::KeepAlive
                } else {
                    WorkflowSessionRetentionHint::Ephemeral
                }),
            )
            .map_err(Self::workflow_service_error_from_runtime_registry)?;

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
                Self::reconcile_active_runtime_mode_info(runtime_registry, &mode_info, false);

                let disposition = runtime_registry
                    .warmup_disposition(runtime_id)
                    .map_err(Self::workflow_service_error_from_runtime_registry)?;
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
                            .map_err(Self::workflow_service_error_from_runtime_registry)?;
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
            .map_err(Self::workflow_service_error_from_runtime_registry)?
            .decision
        {
            RuntimeWarmupDecision::ReuseLoadedRuntime => Ok(()),
            RuntimeWarmupDecision::StartRuntime => {
                let mode_info = self.gateway.mode_info().await;
                Self::reconcile_active_runtime_mode_info(runtime_registry, &mode_info, false);

                match runtime_registry
                    .warmup_disposition(runtime_id)
                    .map_err(Self::workflow_service_error_from_runtime_registry)?
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
                        .map_err(Self::workflow_service_error_from_runtime_registry),
                }
            }
            RuntimeWarmupDecision::WaitForTransition => {
                self.wait_for_runtime_warmup_transition(runtime_registry, runtime_id)
                    .await
            }
        }
    }

    async fn apply_runtime_retention_disposition(
        &self,
        runtime_registry: &pantograph_runtime_registry::RuntimeRegistry,
        disposition: Option<pantograph_runtime_registry::RuntimeRetentionDisposition>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(disposition) = disposition else {
            return Ok(());
        };
        if disposition.decision != RuntimeRetentionDecision::Evict {
            return Ok(());
        }

        runtime_registry::reclaim_runtime_and_reconcile_runtime_registry(
            self.gateway.as_ref(),
            runtime_registry,
            &disposition.runtime_id,
        )
        .await
        .map_err(Self::workflow_service_error_from_runtime_registry)?;

        Ok(())
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
        let descriptor =
            runtime_registry::register_active_runtime(runtime_registry, &host_runtime_mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );

        let lease = runtime_registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: descriptor.runtime_id.clone(),
                workflow_id: workflow_id.to_string(),
                reservation_owner_id: Some(session_id.to_string()),
                usage_profile: Self::trimmed_optional(usage_profile),
                model_id: mode_info.active_model_target.clone(),
                pin_runtime: false,
                requirements,
                retention_hint: Self::runtime_retention_hint(retention_hint),
            })
            .map_err(Self::workflow_service_error_from_runtime_registry)?;

        let previous_reservation_id =
            self.record_session_runtime_reservation(session_id, lease.reservation_id)?;
        if let Err(error) = self
            .consume_runtime_warmup_disposition(runtime_registry.as_ref(), &descriptor.runtime_id)
            .await
        {
            self.restore_session_runtime_reservation(session_id, previous_reservation_id)?;
            if previous_reservation_id != Some(lease.reservation_id) {
                let disposition = runtime_registry
                    .release_reservation_if_present(lease.reservation_id)
                    .map_err(Self::workflow_service_error_from_runtime_registry)?;
                self.apply_runtime_retention_disposition(runtime_registry.as_ref(), disposition)
                    .await?;
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
            let disposition = runtime_registry
                .release_reservation_if_present(reservation_id)
                .map_err(Self::workflow_service_error_from_runtime_registry)?;
            self.apply_runtime_retention_disposition(runtime_registry.as_ref(), disposition)
                .await?;
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
        let descriptor =
            runtime_registry::register_active_runtime(runtime_registry, &host_runtime_mode_info);
        let requirements = Self::reservation_requirements(
            &WorkflowHost::workflow_capabilities(self, workflow_id)
                .await?
                .runtime_requirements,
        );

        match runtime_registry.can_acquire_reservation(&RuntimeReservationRequest {
            runtime_id: descriptor.runtime_id,
            workflow_id: workflow_id.to_string(),
            reservation_owner_id: Some(session_id.to_string()),
            usage_profile: Self::trimmed_optional(usage_profile),
            model_id: mode_info.active_model_target,
            pin_runtime: false,
            requirements,
            retention_hint: Self::runtime_retention_hint(retention_hint),
        }) {
            Ok(()) => Ok(true),
            Err(RuntimeRegistryError::AdmissionRejected { .. })
            | Err(RuntimeRegistryError::ReservationRejected(_)) => Ok(false),
            Err(error) => Err(Self::workflow_service_error_from_runtime_registry(error)),
        }
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
        self.release_loaded_session_runtime(session_id).await
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
            runtime_registry_reclaim_candidate_for_sessions(runtime_registry, candidates)
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
mod tests {
    use super::*;
    use futures_util::stream;
    use inference::backend::{
        BackendCapabilities, BackendConfig, BackendError, BackendStartOutcome, ChatChunk,
        EmbeddingResult, InferenceBackend,
    };
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{RerankRequest, RerankResponse};
    use pantograph_runtime_registry::{
        RuntimeRegistration, RuntimeRegistry, RuntimeRegistrySnapshot, RuntimeRegistryStatus,
    };
    use pantograph_workflow_service::{GraphEdge, GraphNode, Position, WorkflowGraph};
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

    struct MockRestoreFailureBackend {
        ready: bool,
        inference_model_path: PathBuf,
        embedding_model_path: PathBuf,
        embedding_started: bool,
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

    #[async_trait::async_trait]
    impl InferenceBackend for MockRestoreFailureBackend {
        fn name(&self) -> &'static str {
            "Mock"
        }

        fn description(&self) -> &'static str {
            "Mock backend that fails inference restore after embedding mode"
        }

        fn capabilities(&self) -> BackendCapabilities {
            BackendCapabilities::default()
        }

        async fn start(
            &mut self,
            config: &BackendConfig,
            _spawner: Arc<dyn ProcessSpawner>,
        ) -> Result<BackendStartOutcome, BackendError> {
            let model_path = config.model_path.clone().unwrap_or_default();
            if model_path == self.embedding_model_path {
                self.embedding_started = true;
                self.ready = true;
                return Ok(BackendStartOutcome {
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("started_mock_embedding_runtime".to_string()),
                });
            }

            if model_path == self.inference_model_path && self.embedding_started {
                self.ready = false;
                return Err(BackendError::StartupFailed(
                    "mock restore failure after embedding mode".to_string(),
                ));
            }

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
                    }
                ],
                "edges": []
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

    fn write_imported_embedding_model(root: &Path) -> (String, PathBuf) {
        let model_dir = root
            .join("shared-resources")
            .join("models")
            .join("embedding")
            .join("imported")
            .join("test-embed");
        std::fs::create_dir_all(&model_dir).expect("create embedding model dir");

        let model_file = model_dir.join("embed.gguf");
        std::fs::write(&model_file, b"gguf").expect("write embedding model");
        std::fs::write(
            model_dir.join("metadata.json"),
            serde_json::json!({
                "schema_version": 2,
                "model_id": "embedding/imported/test-embed",
                "family": "imported",
                "model_type": "embedding",
                "official_name": "test-embed",
                "cleaned_name": "test-embed",
                "source_path": model_dir.display().to_string(),
                "storage_kind": "library_owned",
                "import_state": "ready",
                "validation_state": "valid",
                "task_type_primary": "feature-extraction",
                "recommended_backend": "llamacpp",
                "runtime_engine_hints": ["llamacpp"]
            })
            .to_string(),
        )
        .expect("write embedding metadata");

        ("embedding/imported/test-embed".to_string(), model_file)
    }

    fn edit_session_embedding_graph(model_id: &str) -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "puma-lib-1".to_string(),
                    node_type: "puma-lib".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({ "model_id": model_id }),
                },
                GraphNode {
                    id: "embedding-1".to_string(),
                    node_type: "embedding".to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: serde_json::json!({}),
                },
            ],
            edges: vec![GraphEdge {
                id: "edge-model".to_string(),
                source: "puma-lib-1".to_string(),
                source_handle: "model_path".to_string(),
                target: "embedding-1".to_string(),
                target_handle: "model".to_string(),
            }],
            ..WorkflowGraph::default()
        }
    }

    fn multi_python_edit_session_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "text-input-1".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: serde_json::json!({ "text": "painted robot" }),
                },
                GraphNode {
                    id: "text-input-2".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 180.0 },
                    data: serde_json::json!({ "text": "tiny waveform" }),
                },
                GraphNode {
                    id: "diffusion-inference-1".to_string(),
                    node_type: "diffusion-inference".to_string(),
                    position: Position { x: 240.0, y: 0.0 },
                    data: serde_json::json!({
                        "model_path": "/tmp/mock-diffusion-model",
                        "backend_key": "diffusers",
                        "model_type": "diffusion",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-python-env"]
                        }
                    }),
                },
                GraphNode {
                    id: "onnx-inference-1".to_string(),
                    node_type: "onnx-inference".to_string(),
                    position: Position { x: 240.0, y: 180.0 },
                    data: serde_json::json!({
                        "model_path": "/tmp/mock-onnx-model",
                        "backend_key": "onnxruntime",
                        "model_type": "audio",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-onnx-env"]
                        }
                    }),
                },
            ],
            edges: vec![
                GraphEdge {
                    id: "e-prompt".to_string(),
                    source: "text-input-1".to_string(),
                    source_handle: "text".to_string(),
                    target: "diffusion-inference-1".to_string(),
                    target_handle: "prompt".to_string(),
                },
                GraphEdge {
                    id: "e-audio".to_string(),
                    source: "text-input-2".to_string(),
                    source_handle: "text".to_string(),
                    target: "onnx-inference-1".to_string(),
                    target_handle: "prompt".to_string(),
                },
            ],
            ..WorkflowGraph::default()
        }
    }

    fn runtime_diffusion_data_graph() -> node_engine::WorkflowGraph {
        node_engine::WorkflowGraph {
            id: "runtime-diffusion-data-graph".to_string(),
            name: "Runtime Diffusion Data Graph".to_string(),
            nodes: vec![
                node_engine::GraphNode {
                    id: "text-input-1".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({ "text": "a tiny painted robot" }),
                    position: (0.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "diffusion-inference-1".to_string(),
                    node_type: "diffusion-inference".to_string(),
                    data: serde_json::json!({
                        "model_path": "/tmp/mock-diffusion-model",
                        "model_type": "diffusion",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-python-env"]
                        }
                    }),
                    position: (240.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "image-output-1".to_string(),
                    node_type: "image-output".to_string(),
                    data: serde_json::json!({}),
                    position: (520.0, 0.0),
                },
            ],
            edges: vec![
                node_engine::GraphEdge {
                    id: "e-prompt".to_string(),
                    source: "text-input-1".to_string(),
                    source_handle: "text".to_string(),
                    target: "diffusion-inference-1".to_string(),
                    target_handle: "prompt".to_string(),
                },
                node_engine::GraphEdge {
                    id: "e-image".to_string(),
                    source: "diffusion-inference-1".to_string(),
                    source_handle: "image".to_string(),
                    target: "image-output-1".to_string(),
                    target_handle: "image".to_string(),
                },
            ],
            groups: Vec::new(),
        }
    }

    fn multi_python_runtime_data_graph() -> node_engine::WorkflowGraph {
        node_engine::WorkflowGraph {
            id: "multi-python-runtime-data-graph".to_string(),
            name: "Multi Python Runtime Data Graph".to_string(),
            nodes: vec![
                node_engine::GraphNode {
                    id: "text-input-1".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({ "text": "painted robot" }),
                    position: (0.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "text-input-2".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({ "text": "tiny waveform" }),
                    position: (0.0, 180.0),
                },
                node_engine::GraphNode {
                    id: "diffusion-inference-1".to_string(),
                    node_type: "diffusion-inference".to_string(),
                    data: serde_json::json!({
                        "model_path": "/tmp/mock-diffusion-model",
                        "backend_key": "diffusers",
                        "model_type": "diffusion",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-python-env"]
                        }
                    }),
                    position: (240.0, 0.0),
                },
                node_engine::GraphNode {
                    id: "onnx-inference-1".to_string(),
                    node_type: "onnx-inference".to_string(),
                    data: serde_json::json!({
                        "model_path": "/tmp/mock-onnx-model",
                        "backend_key": "onnxruntime",
                        "model_type": "audio",
                        "environment_ref": {
                            "state": "ready",
                            "env_ids": ["mock-onnx-env"]
                        }
                    }),
                    position: (240.0, 180.0),
                },
            ],
            edges: vec![
                node_engine::GraphEdge {
                    id: "e-prompt".to_string(),
                    source: "text-input-1".to_string(),
                    source_handle: "text".to_string(),
                    target: "diffusion-inference-1".to_string(),
                    target_handle: "prompt".to_string(),
                },
                node_engine::GraphEdge {
                    id: "e-audio".to_string(),
                    source: "text-input-2".to_string(),
                    source_handle: "text".to_string(),
                    target: "onnx-inference-1".to_string(),
                    target_handle: "prompt".to_string(),
                },
            ],
            groups: Vec::new(),
        }
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
                override_selection: None,
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
                override_selection: None,
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
    async fn workflow_run_returns_invalid_request_for_human_input_workflow() {
        let temp = TempDir::new().expect("temp dir");
        write_human_input_workflow(temp.path(), "interactive-human-input");

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

        let error = runtime
            .workflow_run(WorkflowRunRequest {
                workflow_id: "interactive-human-input".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "human-input-1".to_string(),
                    port_id: "value".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("run-human-input".to_string()),
            })
            .await
            .expect_err("interactive workflow run should fail for non-streaming callers");

        match error {
            WorkflowServiceError::InvalidRequest(message) => {
                assert!(
                    message.contains("interactive") || message.contains("input"),
                    "unexpected invalid-request message: {message}"
                );
            }
            other => panic!("expected invalid request error, got {other:?}"),
        }
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
                override_selection: None,
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
                override_selection: None,
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
        assert_eq!(pytorch.status, RuntimeRegistryStatus::Stopped);
        assert!(pytorch.runtime_instance_id.is_none());
        assert!(pytorch.models.is_empty());
    }

    #[tokio::test]
    async fn execute_data_graph_reconciles_python_sidecar_runtime_into_registry() {
        let temp = TempDir::new().expect("temp dir");

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

        let outputs = runtime
            .execute_data_graph(
                "runtime-diffusion-data-graph",
                &runtime_diffusion_data_graph(),
                &HashMap::from([(
                    "text".to_string(),
                    serde_json::json!("a tiny painted robot"),
                )]),
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("data graph execution");

        assert_eq!(
            outputs.get("image"),
            Some(&serde_json::json!("data:image/png;base64,bW9jay1pbWFnZQ=="))
        );
        assert_eq!(
            outputs.get("_graph_id"),
            Some(&serde_json::json!("runtime-diffusion-data-graph"))
        );

        let snapshot = runtime_registry.snapshot();
        let pytorch = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "pytorch")
            .expect("python runtime should be observed");
        assert_eq!(pytorch.display_name, "PyTorch (Python sidecar)");
        assert_eq!(pytorch.status, RuntimeRegistryStatus::Stopped);
        assert!(pytorch.runtime_instance_id.is_none());
        assert!(pytorch.models.is_empty());
    }

    #[tokio::test]
    async fn execute_data_graph_reconciles_multiple_python_sidecar_runtimes_into_registry() {
        let temp = TempDir::new().expect("temp dir");

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
            .execute_data_graph(
                "multi-python-runtime-data-graph",
                &multi_python_runtime_data_graph(),
                &HashMap::new(),
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("data graph execution");

        let snapshot = runtime_registry.snapshot();
        let diffusers = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "diffusers")
            .expect("diffusers runtime should be observed");
        assert_eq!(diffusers.status, RuntimeRegistryStatus::Stopped);
        assert!(diffusers.runtime_instance_id.is_none());
        assert!(diffusers.models.is_empty());

        let onnx = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "onnx-runtime")
            .expect("onnx runtime should be observed");
        assert_eq!(onnx.status, RuntimeRegistryStatus::Stopped);
        assert!(onnx.runtime_instance_id.is_none());
        assert!(onnx.models.is_empty());
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
        assert_eq!(
            released_snapshot.runtimes[0].status,
            RuntimeRegistryStatus::Stopped
        );
    }

    #[tokio::test]
    async fn keep_alive_disable_reclaim_flips_scheduler_runtime_registry_diagnostics_to_start_runtime(
    ) {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&BackendConfig::default())
            .await
            .expect("gateway should start");

        let host_runtime_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: Some(1),
            },
            gateway.clone(),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::with_capacity_limits(4, 1)),
            None,
            Some(runtime_registry.clone()),
            Some(host_runtime_mode_info),
        )
        .await;

        let created = runtime
            .create_workflow_session(WorkflowSessionCreateRequest {
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            })
            .await
            .expect("create keep-alive session");

        runtime
            .workflow_set_session_keep_alive(WorkflowSessionKeepAliveRequest {
                session_id: created.session_id,
                keep_alive: false,
            })
            .await
            .expect("disable keep alive");

        let snapshot = runtime_registry.snapshot();
        let runtime_record = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("runtime should remain observable after reclaim");
        assert_eq!(runtime_record.status, RuntimeRegistryStatus::Stopped);

        let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
            gateway.clone(),
            runtime_registry.clone(),
        );
        let diagnostics = diagnostics_provider
            .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
                session_id: "queued-after-reclaim".to_string(),
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
                runtime_loaded: false,
                next_admission_queue_id: Some("queue-after-reclaim".to_string()),
                reclaim_candidates: Vec::new(),
            })
            .await
            .expect("scheduler diagnostics provider should succeed")
            .expect("runtime registry diagnostics should be present");

        assert_eq!(
            diagnostics,
            WorkflowSchedulerRuntimeRegistryDiagnostics {
                target_runtime_id: Some("llama_cpp".to_string()),
                reclaim_candidate_session_id: None,
                reclaim_candidate_runtime_id: None,
                next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::StartRuntime,),
                next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::NoLoadedInstance),
            }
        );
    }

    #[tokio::test]
    async fn test_sync_loaded_session_runtime_retention_hint_updates_running_session() {
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

        runtime_registry.register_runtime(RuntimeRegistration::new("llama.cpp", "llama.cpp"));
        runtime_registry
            .transition_runtime(
                "llama.cpp",
                RuntimeTransition::Ready {
                    runtime_instance_id: Some("llama-runtime-1".to_string()),
                },
            )
            .expect("ready transition");

        let lease = runtime_registry
            .acquire_reservation(RuntimeReservationRequest {
                runtime_id: "llama.cpp".to_string(),
                workflow_id: "runtime-text".to_string(),
                reservation_owner_id: Some("session-running".to_string()),
                usage_profile: Some("interactive".to_string()),
                model_id: None,
                pin_runtime: false,
                requirements: None,
                retention_hint: RuntimeRetentionHint::Ephemeral,
            })
            .expect("reservation should be created");
        let host = runtime.host();
        host.record_session_runtime_reservation("session-running", lease.reservation_id)
            .expect("reservation id should be recorded");

        host.sync_loaded_session_runtime_retention_hint(
            "session-running",
            true,
            WorkflowSessionState::Running,
        )
        .expect("running session retention hint should update");

        let snapshot = runtime_registry.snapshot();
        assert_eq!(snapshot.reservations.len(), 1);
        assert_eq!(
            snapshot.reservations[0].retention_hint,
            RuntimeRetentionHint::KeepAlive
        );
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
    async fn test_session_runtime_unload_stops_active_gateway_runtime_when_evictable() {
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
                "session-stop",
                "runtime-text",
                None,
                WorkflowSessionRetentionHint::KeepAlive,
            )
            .await
            .expect("ready runtime should load");
        runtime
            .host()
            .unload_session_runtime(
                "session-stop",
                "runtime-text",
                pantograph_workflow_service::WorkflowSessionUnloadReason::SessionClosed,
            )
            .await
            .expect("runtime should unload");

        let snapshot = runtime_registry.snapshot();
        assert!(snapshot.reservations.is_empty());
        assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
        assert!(!runtime.gateway().is_ready().await);
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
        assert_eq!(snapshot.runtimes[0].status, RuntimeRegistryStatus::Stopped);
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
                override_selection: None,
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
            .select_runtime_unload_candidate(
                &WorkflowSessionRuntimeSelectionTarget {
                    session_id: "session-target".to_string(),
                    workflow_id: "wf-a".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                },
                &[
                    WorkflowSessionRuntimeUnloadCandidate {
                        session_id: "session-a".to_string(),
                        workflow_id: "wf-a".to_string(),
                        usage_profile: Some("interactive".to_string()),
                        required_backends: Vec::new(),
                        required_models: Vec::new(),
                        keep_alive: true,
                        access_tick: 1,
                        run_count: 0,
                    },
                    WorkflowSessionRuntimeUnloadCandidate {
                        session_id: "session-b".to_string(),
                        workflow_id: "wf-b".to_string(),
                        usage_profile: Some("batch".to_string()),
                        required_backends: Vec::new(),
                        required_models: Vec::new(),
                        keep_alive: false,
                        access_tick: 99,
                        run_count: 5,
                    },
                ],
            )
            .await
            .expect("select unload candidate")
            .expect("candidate should exist");

        assert_eq!(selected.session_id, "session-b");
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
                override_selection: None,
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
    async fn hosted_runtime_constructor_syncs_registry_and_derives_capabilities_from_mode_info() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: Some("http://127.0.0.1:11434".to_string()),
            model_path: None,
            is_embedding_mode: false,
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-2".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("llama-cpp-embedding-8".to_string()),
                warmup_started_at_ms: Some(11),
                warmup_completed_at_ms: Some(19),
                warmup_duration_ms: Some(8),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
        });
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
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
            Some(runtime_registry.clone()),
            Some(mode_info),
        )
        .await;

        let snapshot = runtime_registry.snapshot();
        assert_eq!(snapshot.runtimes.len(), 2);
        let active = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("active runtime");
        assert_eq!(active.status, RuntimeRegistryStatus::Ready);
        let embedding = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
            .expect("embedding runtime");
        assert_eq!(embedding.status, RuntimeRegistryStatus::Ready);

        let capabilities = runtime
            .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                workflow_id: "runtime-text".to_string(),
            })
            .await
            .expect("workflow capabilities");
        assert!(capabilities
            .runtime_capabilities
            .iter()
            .any(|capability| capability.runtime_id == "llama.cpp.embedding"));
    }

    #[tokio::test]
    async fn embedded_runtime_shutdown_reconciles_registry_to_stopped() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let mode_info = HostRuntimeModeSnapshot::from_mode_info(&inference::ServerModeInfo {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            mode: "sidecar_inference".to_string(),
            ready: true,
            url: Some("http://127.0.0.1:11434".to_string()),
            model_path: None,
            is_embedding_mode: false,
            active_model_target: Some("/models/qwen.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-9".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        });
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
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
            Some(runtime_registry.clone()),
            Some(mode_info),
        )
        .await;

        let ready_snapshot = runtime_registry.snapshot();
        let ready_runtime = ready_snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("active runtime should be registered before shutdown");
        assert_eq!(ready_runtime.status, RuntimeRegistryStatus::Ready);

        runtime.shutdown().await;

        let stopped_snapshot = runtime_registry.snapshot();
        let stopped_runtime = stopped_snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("active runtime should remain observable after shutdown");
        assert_eq!(stopped_runtime.status, RuntimeRegistryStatus::Stopped);
    }

    #[tokio::test]
    async fn execute_edit_session_graph_reconciles_registry_after_restore() {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");
        let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp.path())
                .build()
                .await
                .expect("build pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("rebuild model index");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let inference_model_path = temp.path().join("main.gguf");
        std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
        let mmproj_path = temp.path().join("main.mmproj");
        std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&inference::BackendConfig {
                model_path: Some(inference_model_path.clone()),
                mmproj_path: Some(mmproj_path),
                ..inference::BackendConfig::default()
            })
            .await
            .expect("gateway should start in inference mode");

        let host_runtime_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
        let initial_runtime_instance_id = host_runtime_mode_info
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.clone())
            .expect("initial runtime instance id");

        let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        extensions
            .write()
            .await
            .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            gateway.clone(),
            extensions,
            Arc::new(WorkflowService::new()),
            None,
            Some(runtime_registry.clone()),
            Some(host_runtime_mode_info),
        )
        .await;

        let graph = edit_session_embedding_graph(&model_id);
        let session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");

        let outcome = runtime
            .execute_edit_session_graph(
                &session.session_id,
                &graph,
                inference::EmbeddingStartRequest {
                    gguf_model_path: Some(embedding_model_path),
                    ..inference::EmbeddingStartRequest::default()
                },
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("edit-session execution should restore runtime even when node demand fails");
        assert!(outcome.error.is_some());

        let restored_mode_info = gateway.mode_info().await;
        let restored_runtime_instance_id = restored_mode_info
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.clone())
            .expect("restored runtime instance id");
        assert_ne!(
            restored_runtime_instance_id, initial_runtime_instance_id,
            "restore path should produce a fresh runtime instance for this regression check"
        );

        let snapshot = runtime_registry.snapshot();
        let registry_runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("active runtime should remain registered after restore");
        assert_eq!(
            registry_runtime.runtime_instance_id.as_deref(),
            Some(restored_runtime_instance_id.as_str())
        );
        assert_eq!(registry_runtime.status, RuntimeRegistryStatus::Ready);
    }

    #[tokio::test]
    async fn execute_edit_session_graph_restore_keeps_scheduler_runtime_registry_diagnostics_ready()
    {
        let temp = TempDir::new().expect("temp dir");
        write_test_workflow(temp.path(), "runtime-text");
        let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp.path())
                .build()
                .await
                .expect("build pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("rebuild model index");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let inference_model_path = temp.path().join("main.gguf");
        std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
        let mmproj_path = temp.path().join("main.mmproj");
        std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&inference::BackendConfig {
                model_path: Some(inference_model_path),
                mmproj_path: Some(mmproj_path),
                ..inference::BackendConfig::default()
            })
            .await
            .expect("gateway should start in inference mode");

        let host_runtime_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
        let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        extensions
            .write()
            .await
            .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: Some(1),
            },
            gateway.clone(),
            extensions,
            Arc::new(WorkflowService::with_capacity_limits(4, 1)),
            None,
            Some(runtime_registry.clone()),
            Some(host_runtime_mode_info),
        )
        .await;

        let graph = edit_session_embedding_graph(&model_id);
        let edit_session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");

        let outcome = runtime
            .execute_edit_session_graph(
                &edit_session.session_id,
                &graph,
                inference::EmbeddingStartRequest {
                    gguf_model_path: Some(embedding_model_path),
                    ..inference::EmbeddingStartRequest::default()
                },
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("edit-session execution should restore runtime even when node demand fails");
        assert!(outcome.error.is_some());

        let restored_runtime_instance_id = gateway
            .mode_info()
            .await
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.clone())
            .expect("restored runtime instance id");
        let restored_runtime = runtime_registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("restored runtime should remain registered");
        assert_eq!(restored_runtime.status, RuntimeRegistryStatus::Ready);
        assert_eq!(
            restored_runtime.runtime_instance_id.as_deref(),
            Some(restored_runtime_instance_id.as_str())
        );

        let loaded = runtime
            .create_workflow_session(WorkflowSessionCreateRequest {
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
            })
            .await
            .expect("create loaded session");

        let diagnostics_provider = EmbeddedWorkflowSchedulerDiagnosticsProvider::new(
            gateway.clone(),
            runtime_registry.clone(),
        );
        let diagnostics = diagnostics_provider
            .scheduler_runtime_registry_diagnostics(&WorkflowSchedulerRuntimeDiagnosticsRequest {
                session_id: "queued-session".to_string(),
                workflow_id: "runtime-text".to_string(),
                usage_profile: Some("interactive".to_string()),
                keep_alive: false,
                runtime_loaded: false,
                next_admission_queue_id: Some("queue-after-restore".to_string()),
                reclaim_candidates: vec![WorkflowSessionRuntimeUnloadCandidate {
                    session_id: loaded.session_id.clone(),
                    workflow_id: "runtime-text".to_string(),
                    usage_profile: Some("interactive".to_string()),
                    required_backends: Vec::new(),
                    required_models: Vec::new(),
                    keep_alive: true,
                    access_tick: 1,
                    run_count: 0,
                }],
            })
            .await
            .expect("scheduler diagnostics provider should succeed")
            .expect("runtime registry diagnostics should be present");

        assert_eq!(
            diagnostics,
            WorkflowSchedulerRuntimeRegistryDiagnostics {
                target_runtime_id: Some("llama_cpp".to_string()),
                reclaim_candidate_session_id: Some(loaded.session_id),
                reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                next_warmup_decision: Some(
                    WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                ),
                next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady),
            }
        );
    }

    #[tokio::test]
    async fn execute_edit_session_graph_reconciles_registry_after_embedding_prepare() {
        let temp = TempDir::new().expect("temp dir");
        let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp.path())
                .build()
                .await
                .expect("build pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("rebuild model index");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let inference_model_path = temp.path().join("main.gguf");
        std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
        let mmproj_path = temp.path().join("main.mmproj");
        std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockReadyBackend { ready: false }),
            "llama.cpp",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&inference::BackendConfig {
                model_path: Some(inference_model_path.clone()),
                mmproj_path: Some(mmproj_path),
                ..inference::BackendConfig::default()
            })
            .await
            .expect("gateway should start in inference mode");

        let host_runtime_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
        let initial_runtime_instance_id = host_runtime_mode_info
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.clone())
            .expect("initial runtime instance id");

        let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        extensions
            .write()
            .await
            .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            gateway.clone(),
            extensions,
            Arc::new(WorkflowService::new()),
            None,
            Some(runtime_registry.clone()),
            Some(host_runtime_mode_info),
        )
        .await;

        let started_snapshot = Arc::new(Mutex::new(None::<RuntimeRegistrySnapshot>));
        let started_snapshot_sink = started_snapshot.clone();
        let runtime_registry_for_sink = runtime_registry.clone();
        let event_sink = Arc::new(node_engine::CallbackEventSink::new(move |event| {
            if matches!(event, node_engine::WorkflowEvent::WorkflowStarted { .. }) {
                let mut guard = started_snapshot_sink
                    .lock()
                    .expect("started snapshot lock poisoned");
                *guard = Some(runtime_registry_for_sink.snapshot());
            }
        }));

        let graph = edit_session_embedding_graph(&model_id);
        let session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");

        let outcome = runtime
            .execute_edit_session_graph(
                &session.session_id,
                &graph,
                inference::EmbeddingStartRequest {
                    gguf_model_path: Some(embedding_model_path),
                    ..inference::EmbeddingStartRequest::default()
                },
                event_sink,
            )
            .await
            .expect("edit-session execution should still finish");
        assert!(outcome.error.is_some());

        let started_snapshot = started_snapshot
            .lock()
            .expect("started snapshot lock poisoned")
            .clone()
            .expect("workflow started snapshot");
        let started_runtime = started_snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == "llama_cpp")
            .expect("active runtime snapshot at workflow start");
        assert_eq!(started_runtime.status, RuntimeRegistryStatus::Ready);
        assert_ne!(
            started_runtime.runtime_instance_id.as_deref(),
            Some(initial_runtime_instance_id.as_str()),
            "registry should be refreshed to the prepared embedding runtime before execution starts"
        );
    }

    #[tokio::test]
    async fn execute_edit_session_graph_reconciles_registry_after_failed_restore() {
        let temp = TempDir::new().expect("temp dir");
        let (model_id, embedding_model_path) = write_imported_embedding_model(temp.path());

        let pumas_api = Arc::new(
            pumas_library::PumasApi::builder(temp.path())
                .build()
                .await
                .expect("build pumas api"),
        );
        pumas_api
            .rebuild_model_index()
            .await
            .expect("rebuild model index");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

        let inference_model_path = temp.path().join("main.gguf");
        std::fs::write(&inference_model_path, b"gguf").expect("write inference model");
        let mmproj_path = temp.path().join("main.mmproj");
        std::fs::write(&mmproj_path, b"mmproj").expect("write mmproj");

        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(MockRestoreFailureBackend {
                ready: false,
                inference_model_path: inference_model_path.clone(),
                embedding_model_path: embedding_model_path.clone(),
                embedding_started: false,
            }),
            "llama.cpp",
        ));
        gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
        gateway
            .start(&inference::BackendConfig {
                model_path: Some(inference_model_path.clone()),
                mmproj_path: Some(mmproj_path),
                ..inference::BackendConfig::default()
            })
            .await
            .expect("gateway should start in inference mode");

        let host_runtime_mode_info =
            HostRuntimeModeSnapshot::from_mode_info(&gateway.mode_info().await);
        let runtime_registry = Arc::new(RuntimeRegistry::new());
        let extensions = Arc::new(RwLock::new(ExecutorExtensions::new()));
        extensions
            .write()
            .await
            .set(node_engine::extension_keys::PUMAS_API, pumas_api.clone());

        let runtime = EmbeddedRuntime::hosted_with_default_python_runtime(
            EmbeddedRuntimeConfig {
                app_data_dir,
                project_root: temp.path().to_path_buf(),
                workflow_roots: vec![temp.path().join(".pantograph").join("workflows")],
                max_loaded_sessions: None,
            },
            gateway.clone(),
            extensions,
            Arc::new(WorkflowService::new()),
            None,
            Some(runtime_registry.clone()),
            Some(host_runtime_mode_info),
        )
        .await;

        let graph = edit_session_embedding_graph(&model_id);
        let session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");

        let outcome = runtime
            .execute_edit_session_graph(
                &session.session_id,
                &graph,
                inference::EmbeddingStartRequest {
                    gguf_model_path: Some(embedding_model_path),
                    ..inference::EmbeddingStartRequest::default()
                },
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("edit-session execution should still complete when restore fails");
        assert!(outcome.error.is_some());

        let mode_info = gateway.mode_info().await;
        let expected_observation = runtime_registry::active_runtime_observation(
            &HostRuntimeModeSnapshot::from_mode_info(&mode_info),
            true,
        )
        .expect("active runtime observation after failed restore");

        let snapshot = runtime_registry.snapshot();
        let registry_runtime = snapshot
            .runtimes
            .iter()
            .find(|runtime| runtime.runtime_id == expected_observation.runtime_id)
            .expect("active runtime should remain observable after failed restore");
        assert_eq!(registry_runtime.status, expected_observation.status);
        assert_eq!(
            registry_runtime.runtime_instance_id,
            expected_observation.runtime_instance_id
        );
    }

    #[tokio::test]
    async fn execute_edit_session_graph_reports_all_python_runtime_ids_in_trace_metrics() {
        let temp = TempDir::new().expect("temp dir");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

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
        );

        let graph = multi_python_edit_session_graph();
        let session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");

        let outcome = runtime
            .execute_edit_session_graph(
                &session.session_id,
                &graph,
                inference::EmbeddingStartRequest::default(),
                Arc::new(node_engine::NullEventSink),
            )
            .await
            .expect("edit-session execution");

        assert_eq!(
            outcome.trace_runtime_metrics.runtime_id.as_deref(),
            Some("onnx-runtime")
        );
        assert_eq!(
            outcome.trace_runtime_metrics.observed_runtime_ids,
            vec!["onnx-runtime".to_string(), "diffusers".to_string()]
        );
        assert_eq!(
            outcome.trace_runtime_metrics.model_target.as_deref(),
            Some("/tmp/mock-onnx-model")
        );
        assert_eq!(
            outcome.runtime_snapshot.runtime_id.as_deref(),
            Some("onnx-runtime")
        );
        assert_eq!(
            outcome.runtime_model_target.as_deref(),
            Some("/tmp/mock-onnx-model")
        );
        assert!(!outcome.waiting_for_input);
    }

    #[tokio::test]
    async fn execute_edit_session_graph_waiting_for_input_does_not_emit_workflow_failed() {
        let temp = TempDir::new().expect("temp dir");

        let app_data_dir = temp.path().join("app-data");
        std::fs::create_dir_all(&app_data_dir).expect("app data dir");
        install_fake_default_runtime(&app_data_dir);

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
            Arc::new(ProcessPythonRuntimeAdapter),
        );

        let graph = WorkflowGraph {
            nodes: vec![GraphNode {
                id: "approval".to_string(),
                node_type: "human-input".to_string(),
                data: serde_json::json!({ "prompt": "Approve deployment?" }),
                position: Position::default(),
            }],
            edges: Vec::new(),
            derived_graph: None,
        };
        let session = runtime
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
            })
            .await
            .expect("create edit session");
        let event_sink = Arc::new(node_engine::VecEventSink::new());

        let outcome = runtime
            .execute_edit_session_graph(
                &session.session_id,
                &graph,
                inference::EmbeddingStartRequest::default(),
                event_sink.clone(),
            )
            .await
            .expect("edit-session execution should pause instead of failing");

        assert!(outcome.waiting_for_input);
        assert!(outcome.error.is_none());

        let events = event_sink.events();
        assert!(events.iter().any(|event| matches!(
            event,
            node_engine::WorkflowEvent::WaitingForInput {
                task_id,
                prompt: Some(prompt),
                ..
            } if task_id == "approval" && prompt == "Approve deployment?"
        )));
        assert!(!events
            .iter()
            .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowFailed { .. })));
        assert!(!events
            .iter()
            .any(|event| matches!(event, node_engine::WorkflowEvent::WorkflowCompleted { .. })));
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

    #[test]
    fn runtime_registry_admission_errors_map_to_runtime_not_ready() {
        let error = EmbeddedWorkflowHost::workflow_service_error_from_runtime_registry(
            RuntimeRegistryError::AdmissionRejected {
                runtime_id: "pytorch".to_string(),
                failure: pantograph_runtime_registry::RuntimeAdmissionFailure::InsufficientRam {
                    requested_mb: 1024,
                    available_mb: 0,
                    reserved_mb: 2048,
                    total_mb: 2048,
                    safety_margin_mb: 0,
                },
            },
        );

        assert!(matches!(error, WorkflowServiceError::RuntimeNotReady(_)));
        assert_eq!(
            error.code(),
            pantograph_workflow_service::WorkflowErrorCode::RuntimeNotReady
        );
    }

    #[test]
    fn runtime_registry_owner_conflicts_map_to_invalid_request() {
        let error = EmbeddedWorkflowHost::workflow_service_error_from_runtime_registry(
            RuntimeRegistryError::ReservationOwnerConflict {
                owner_id: "session-a".to_string(),
                existing_runtime_id: "llama_cpp".to_string(),
                requested_runtime_id: "pytorch".to_string(),
            },
        );

        assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
        assert_eq!(
            error.code(),
            pantograph_workflow_service::WorkflowErrorCode::InvalidRequest
        );
    }
}
