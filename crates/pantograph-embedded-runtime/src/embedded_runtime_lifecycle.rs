use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "standalone")]
use node_engine::ExecutorExtensions;
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::WorkflowRuntimeCapability;
#[cfg(feature = "standalone")]
use pantograph_workflow_service::WorkflowService;
#[cfg(feature = "standalone")]
use tokio::sync::RwLock;

use crate::{
    EmbeddedRuntime, EmbeddedRuntimeConfig, EmbeddedWorkflowHost,
    EmbeddedWorkflowSchedulerDiagnosticsProvider, HostRuntimeModeSnapshot,
    ProcessPythonRuntimeAdapter, PythonRuntimeAdapter, RagBackend, SharedExtensions,
    SharedWorkflowService, runtime_capabilities, runtime_registry, workflow_session_execution,
};
#[cfg(feature = "standalone")]
use crate::{EmbeddedRuntimeError, StandaloneRuntimeConfig, TauriModelDependencyResolver};

impl EmbeddedRuntime {
    pub fn from_components(
        config: EmbeddedRuntimeConfig,
        gateway: Arc<inference::InferenceGateway>,
        extensions: SharedExtensions,
        workflow_service: SharedWorkflowService,
        rag_backend: Option<Arc<dyn RagBackend>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        workflow_service
            .set_loaded_runtime_capacity_limit(config.max_loaded_sessions)
            .expect("embedded runtime should apply the configured loaded-session capacity limit");
        Self {
            config,
            gateway,
            extensions,
            workflow_service,
            runtime_registry: None,
            session_runtime_reservations: Arc::new(Mutex::new(HashMap::new())),
            session_executions: Arc::new(
                workflow_session_execution::WorkflowSessionExecutionStore::new(),
            ),
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

    pub(crate) async fn reconcile_runtime_registry_from_gateway(&self) {
        let Some(runtime_registry) = self.runtime_registry.as_ref() else {
            return;
        };

        runtime_registry::sync_runtime_registry(self.gateway.as_ref(), runtime_registry.as_ref())
            .await;
    }

    pub async fn shutdown(&self) {
        if let Err(error) = self.workflow_service.invalidate_all_session_runtimes() {
            log::warn!(
                "failed to invalidate workflow session runtimes before shutdown: {}",
                error
            );
        }
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

    pub(crate) fn host(&self) -> EmbeddedWorkflowHost {
        EmbeddedWorkflowHost {
            app_data_dir: self.config.app_data_dir.clone(),
            project_root: self.config.project_root.clone(),
            workflow_roots: self.config.workflow_roots.clone(),
            gateway: self.gateway.clone(),
            extensions: self.extensions.clone(),
            runtime_registry: self.runtime_registry.clone(),
            session_runtime_reservations: self.session_runtime_reservations.clone(),
            session_executions: self.session_executions.clone(),
            rag_backend: self.rag_backend.clone(),
            python_runtime: self.python_runtime.clone(),
            additional_runtime_capabilities: self.additional_runtime_capabilities.clone(),
        }
    }
}
