use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "standalone")]
use node_engine::ExecutorExtensions;
use pantograph_runtime_registry::SharedRuntimeRegistry;
use pantograph_workflow_service::WorkflowRuntimeCapability;
#[cfg(feature = "standalone")]
use tokio::sync::RwLock;

#[cfg(feature = "standalone")]
use crate::dependency_inventory::DependencyInventoryService;
#[cfg(feature = "standalone")]
use crate::workflow_service_composition::EmbeddedWorkflowServiceComposition;
use crate::{
    runtime_capabilities, runtime_registry, workflow_execution_session_execution, EmbeddedRuntime,
    EmbeddedRuntimeConfig, EmbeddedWorkflowHost, EmbeddedWorkflowSchedulerDiagnosticsProvider,
    HostRuntimeModeSnapshot, ProcessPythonRuntimeAdapter, PythonRuntimeAdapter, RagBackend,
    SharedExtensions, SharedWorkflowService,
};
#[cfg(feature = "standalone")]
use crate::{
    EmbeddedDependencyReadinessAutoResumeConfig, EmbeddedDependencyReadinessSnapshotProducer,
    EmbeddedRuntimeError, StandaloneRuntimeConfig,
};

impl EmbeddedRuntime {
    pub fn from_components(
        config: EmbeddedRuntimeConfig,
        gateway: Arc<inference::InferenceGateway>,
        extensions: SharedExtensions,
        workflow_service: SharedWorkflowService,
        rag_backend: Option<Arc<dyn RagBackend>>,
        python_runtime: Arc<dyn PythonRuntimeAdapter>,
    ) -> Self {
        if let Err(error) =
            inference::reconcile_interrupted_managed_runtime_jobs(&config.app_data_dir)
        {
            log::warn!("Failed to reconcile interrupted managed runtime jobs: {error}");
        }
        workflow_service
            .set_loaded_runtime_capacity_limit(config.max_loaded_sessions)
            .expect("embedded runtime should apply the configured loaded-session capacity limit");
        Self {
            config,
            gateway,
            extensions,
            workflow_service,
            runtime_registry: None,
            dependency_readiness_auto_resume: None,
            dependency_readiness_snapshot_producer: None,
            session_runtime_reservations: Arc::new(Mutex::new(HashMap::new())),
            session_runtime_load_proofs: Arc::new(Mutex::new(HashMap::new())),
            session_executions: Arc::new(
                workflow_execution_session_execution::WorkflowExecutionSessionExecutionStore::new(),
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

    /// Build a hosted runtime around a workflow service that was already
    /// composed by `EmbeddedWorkflowServiceComposition`.
    ///
    /// This constructor intentionally does not mutate workflow-service
    /// capacity, scheduler diagnostics, or runtime-dispatch dependencies. Those
    /// concerns must be attached before the service is shared.
    pub fn from_hosted_composition(
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

        Self {
            config,
            gateway,
            extensions,
            workflow_service,
            runtime_registry,
            dependency_readiness_auto_resume: None,
            dependency_readiness_snapshot_producer: None,
            session_runtime_reservations: Arc::new(Mutex::new(HashMap::new())),
            session_runtime_load_proofs: Arc::new(Mutex::new(HashMap::new())),
            session_executions: Arc::new(
                workflow_execution_session_execution::WorkflowExecutionSessionExecutionStore::new(),
            ),
            rag_backend,
            python_runtime: Arc::new(ProcessPythonRuntimeAdapter),
            additional_runtime_capabilities,
        }
    }

    /// Test-only legacy hosted constructor for exercising runtime-registry and
    /// capability behavior without the resource-backed hosted composition
    /// boundary.
    #[cfg(test)]
    pub(crate) async fn test_hosted_with_default_python_runtime(
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

        let workflow_service_composition = EmbeddedWorkflowServiceComposition::new();
        let dependency_readiness = workflow_service_composition.dependency_readiness();
        let dependency_readiness_snapshot_producer =
            EmbeddedDependencyReadinessSnapshotProducer::new(
                dependency_readiness.snapshot_provider(),
                dependency_readiness.work_queue(),
                dependency_readiness.requirements_registry(),
            )
            .with_dependency_inventory(Arc::new(DependencyInventoryService::from_app_data_dir(
                config.app_data_dir.clone(),
                gateway.clone(),
            )))
            .spawn(tokio::runtime::Handle::current())
            .map_err(|error| EmbeddedRuntimeError::Initialization {
                message: error.to_string(),
            })?;
        let workflow_service = workflow_service_composition
            .into_shared_workflow_service(config.max_loaded_sessions)
            .map_err(|error| EmbeddedRuntimeError::Initialization {
                message: error.to_string(),
            })?;
        let extensions: SharedExtensions = Arc::new(RwLock::new(ExecutorExtensions::new()));

        {
            let mut guard = extensions.write().await;
            workflow_nodes::setup_extensions_with_path(
                &mut guard,
                config.pumas_library_path.as_deref(),
            )
            .await;
            guard.set(
                node_engine::extension_keys::KV_CACHE_STORE,
                Arc::new(inference::kv_cache::KvCacheStore::new(
                    config.app_data_dir.join("kv_cache"),
                    inference::kv_cache::StoragePolicy::MemoryAndDisk,
                )),
            );
        }

        let runtime = Self::with_default_python_runtime(
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
        )
        .with_dependency_readiness_snapshot_producer(dependency_readiness_snapshot_producer);
        let auto_resume = runtime
            .spawn_dependency_readiness_auto_resume(
                tokio::runtime::Handle::current(),
                EmbeddedDependencyReadinessAutoResumeConfig::default(),
            )
            .map_err(|error| EmbeddedRuntimeError::Initialization {
                message: error.to_string(),
            })?;
        Ok(runtime.with_dependency_readiness_auto_resume(auto_resume))
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

    pub fn with_dependency_readiness_snapshot_producer(
        mut self,
        producer: crate::EmbeddedDependencyReadinessSnapshotProducerHandle,
    ) -> Self {
        self.dependency_readiness_snapshot_producer = Some(producer);
        self
    }

    pub fn with_dependency_readiness_auto_resume(
        mut self,
        auto_resume: crate::EmbeddedDependencyReadinessAutoResumeHandle,
    ) -> Self {
        self.dependency_readiness_auto_resume = Some(auto_resume);
        self
    }

    pub fn spawn_dependency_readiness_auto_resume(
        &self,
        runtime_handle: tokio::runtime::Handle,
        config: crate::EmbeddedDependencyReadinessAutoResumeConfig,
    ) -> Result<crate::EmbeddedDependencyReadinessAutoResumeHandle, crate::EmbeddedRuntimeError>
    {
        crate::EmbeddedDependencyReadinessAutoResume::new(Arc::new(
            crate::dependency_readiness_auto_resume::EmbeddedWorkflowServiceAutoResumePort::new(
                self.workflow_service.clone(),
                self.host(),
            ),
        ))
        .with_config(config)
        .spawn(runtime_handle)
    }

    pub fn record_workflow_session_runtime_load_proof(
        &self,
        workflow_id: &str,
        proof: pantograph_workflow_service::WorkflowSessionRuntimeLoadProof,
    ) -> Result<
        Option<pantograph_workflow_service::WorkflowSessionRuntimeLoadProof>,
        pantograph_workflow_service::WorkflowServiceError,
    > {
        proof.validate().map_err(|error| {
            pantograph_workflow_service::WorkflowServiceError::InvalidRequest(format!(
                "invalid workflow session runtime load proof: {error}"
            ))
        })?;
        if proof.workflow_id != workflow_id {
            return Err(pantograph_workflow_service::WorkflowServiceError::InvalidRequest(
                format!(
                    "runtime load proof workflow_id '{}' does not match requested workflow '{workflow_id}'",
                    proof.workflow_id
                ),
            ));
        }
        let mut proofs = self.session_runtime_load_proofs.lock().map_err(|_| {
            pantograph_workflow_service::WorkflowServiceError::Internal(
                "session runtime load proof lock poisoned".to_string(),
            )
        })?;

        Ok(proofs.insert(workflow_id.to_string(), proof))
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
        if let Some(auto_resume) = self.dependency_readiness_auto_resume.as_ref() {
            auto_resume.shutdown().await;
        }
        if let Err(error) = self.workflow_service.invalidate_all_session_runtimes() {
            log::warn!(
                "failed to invalidate workflow execution session runtimes before shutdown: {}",
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
        if let Some(producer) = self.dependency_readiness_snapshot_producer.as_ref() {
            producer.shutdown().await;
        }
    }

    pub(crate) fn host(&self) -> EmbeddedWorkflowHost {
        EmbeddedWorkflowHost {
            app_data_dir: self.config.app_data_dir.clone(),
            project_root: self.config.project_root.clone(),
            workflow_roots: self.config.workflow_roots.clone(),
            gateway: self.gateway.clone(),
            extensions: self.extensions.clone(),
            workflow_service: self.workflow_service.clone(),
            runtime_registry: self.runtime_registry.clone(),
            session_runtime_reservations: self.session_runtime_reservations.clone(),
            session_runtime_load_proofs: self.session_runtime_load_proofs.clone(),
            session_executions: self.session_executions.clone(),
            rag_backend: self.rag_backend.clone(),
            python_runtime: self.python_runtime.clone(),
            additional_runtime_capabilities: self.additional_runtime_capabilities.clone(),
            node_event_sink: None,
        }
    }
}
