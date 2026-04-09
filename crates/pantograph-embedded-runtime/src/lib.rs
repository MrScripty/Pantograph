use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use node_engine::{
    CoreTaskExecutor, ExecutorExtensions, NullEventSink, WorkflowExecutor, WorkflowGraph,
};
use pantograph_workflow_service::capabilities;
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowHost,
    WorkflowHostModelDescriptor, WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunOptions,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeCapability,
    WorkflowRuntimeInstallState, WorkflowService, WorkflowServiceError,
    WorkflowSessionCloseRequest, WorkflowSessionCloseResponse, WorkflowSessionCreateRequest,
    WorkflowSessionCreateResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRunRequest,
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
pub use task_executor::{TauriTaskExecutor as PantographTaskExecutor, runtime_extension_keys};

pub type SharedExtensions = Arc<RwLock<ExecutorExtensions>>;
pub type SharedWorkflowService = Arc<WorkflowService>;

#[derive(Debug, Clone)]
pub struct EmbeddedRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
}

impl EmbeddedRuntimeConfig {
    pub fn new(app_data_dir: PathBuf, project_root: PathBuf) -> Self {
        Self {
            app_data_dir,
            workflow_roots: capabilities::default_workflow_roots(&project_root),
            project_root,
        }
    }
}

#[cfg(feature = "standalone")]
#[derive(Debug, Clone)]
pub struct StandaloneRuntimeConfig {
    pub app_data_dir: PathBuf,
    pub project_root: PathBuf,
    pub workflow_roots: Vec<PathBuf>,
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
}

pub struct EmbeddedRuntime {
    config: EmbeddedRuntimeConfig,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    workflow_service: SharedWorkflowService,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
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
            rag_backend,
            python_runtime,
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
            rag_backend: self.rag_backend.clone(),
            python_runtime: self.python_runtime.clone(),
        }
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
}

struct EmbeddedWorkflowHost {
    app_data_dir: PathBuf,
    project_root: PathBuf,
    workflow_roots: Vec<PathBuf>,
    gateway: Arc<inference::InferenceGateway>,
    extensions: SharedExtensions,
    rag_backend: Option<Arc<dyn RagBackend>>,
    python_runtime: Arc<dyn PythonRuntimeAdapter>,
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
            inference::ManagedBinaryId::LlamaCpp => {
                vec!["llama.cpp".to_string(), "llamacpp".to_string()]
            }
            inference::ManagedBinaryId::Ollama => vec!["ollama".to_string()],
        }
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
        Ok(self.gateway.current_backend_name().await)
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
        let mut runtimes = inference::list_binary_capabilities(&self.app_data_dir)
            .map_err(WorkflowServiceError::RuntimeNotReady)?
            .into_iter()
            .map(|runtime| WorkflowRuntimeCapability {
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
                backend_keys: Self::runtime_backend_keys(runtime.id),
                missing_files: runtime.missing_files,
                unavailable_reason: runtime.unavailable_reason,
            })
            .collect::<Vec<_>>();
        runtimes.sort_by(|left, right| left.runtime_id.cmp(&right.runtime_id));
        Ok(runtimes)
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

        let mut executor = WorkflowExecutor::new(execution_id, graph, Arc::new(NullEventSink));
        apply_runtime_extensions(&mut executor, &runtime_ext);

        let mut node_outputs = HashMap::new();
        for node_id in &output_node_ids {
            if run_handle.is_cancelled() {
                return Err(WorkflowServiceError::RuntimeTimeout(
                    "workflow run cancelled during execution".to_string(),
                ));
            }
            let outputs = executor
                .demand(node_id, &task_executor)
                .await
                .map_err(|e| WorkflowServiceError::Internal(e.to_string()))?;
            node_outputs.insert(node_id.clone(), outputs);
        }

        Self::collect_run_outputs(&node_outputs, &output_node_ids, output_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::Mutex;
    use tempfile::TempDir;

    struct MockImagePythonRuntime {
        requests: Mutex<Vec<PythonNodeExecutionRequest>>,
    }

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
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
        );

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
            },
            Arc::new(inference::InferenceGateway::new()),
            Arc::new(RwLock::new(ExecutorExtensions::new())),
            Arc::new(WorkflowService::new()),
            None,
            python_runtime.clone(),
        );

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
}
