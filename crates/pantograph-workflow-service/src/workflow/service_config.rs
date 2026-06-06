use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentService, DependencyReadinessWorkQueue, DependencyRequirementsPayload,
    DependencyRequirementsRegistryError, InMemoryDependencyRequirementsRegistry,
    NotImplementedDependencyEnvironmentProvider, SharedDependencyEnvironmentProvider,
};
use pantograph_dependency_planning::ValidatedDependencyEnvironmentResult;
use pantograph_runtime_host_contracts::{
    ReservationLifecyclePort, RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionPort,
    RuntimeHostExecutionPortError, RuntimeHostExecutionRequest, RuntimeHostExecutionResponse,
    SchedulerRuntimeHostDispatcher,
};

use crate::graph::{
    GraphSessionStore, InferenceInterfaceFactsProvider, UnavailableInferenceInterfaceFactsProvider,
};
use crate::scheduler::{
    WorkflowDependencyReadinessProvider, WorkflowExecutionSessionStore,
    WorkflowSchedulerTaskOrchestrator,
};

use super::{
    ArtifactFormatDependencyVersions, ArtifactFormatSettings, ArtifactStore,
    NoRuntimeDispatchCandidatesProvider, NoRuntimeDispatchSourceRefresher, SqliteAttributionStore,
    SqliteDiagnosticsLedger, WorkflowDiagnosticsProjectionRefreshSink,
    WorkflowRuntimeDispatchCandidateProvider, WorkflowRuntimeDispatchSourceRefresher,
    WorkflowSchedulerDiagnosticsProvider, WorkflowService, WorkflowServiceError,
};

const DEFAULT_MAX_SESSIONS: usize = 8;

impl Default for WorkflowService {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkflowService {
    pub fn new() -> Self {
        Self::with_capacity_limits(DEFAULT_MAX_SESSIONS, DEFAULT_MAX_SESSIONS)
    }

    pub fn with_max_sessions(max_sessions: usize) -> Self {
        Self::with_capacity_limits(max_sessions, max_sessions)
    }

    pub fn with_capacity_limits(max_sessions: usize, max_loaded_sessions: usize) -> Self {
        Self {
            session_store: Arc::new(Mutex::new(WorkflowExecutionSessionStore::new(
                max_sessions,
                max_loaded_sessions,
            ))),
            graph_session_store: Arc::new(GraphSessionStore::new()),
            artifact_writer: None,
            artifact_format_settings: Arc::new(Mutex::new(ArtifactFormatSettings::default())),
            artifact_format_settings_path: None,
            artifact_format_dependency_versions: Arc::new(Mutex::new(
                ArtifactFormatDependencyVersions::default(),
            )),
            attribution_store: None,
            diagnostics_ledger: None,
            diagnostics_projection_refresh_sink: Arc::new(Mutex::new(None)),
            scheduler_diagnostics_provider: Arc::new(Mutex::new(None)),
            scheduler_task_orchestrator: default_scheduler_task_orchestrator(),
            dependency_readiness_provider: default_dependency_readiness_provider(),
            runtime_dispatch_source_refresher: default_runtime_dispatch_source_refresher(),
            runtime_dispatch_candidate_provider: default_runtime_dispatch_candidate_provider(),
            dependency_readiness_work_queue: Arc::new(DependencyReadinessWorkQueue::new()),
            dependency_requirements_registry: Arc::new(
                InMemoryDependencyRequirementsRegistry::new(),
            ),
            task_execution_worker: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn with_runtime_host_execution_port(
        mut self,
        port: Arc<dyn RuntimeHostExecutionPort>,
    ) -> Self {
        self.scheduler_task_orchestrator = self
            .scheduler_task_orchestrator
            .with_runtime_host_dispatcher(SchedulerRuntimeHostDispatcher::new(port));
        self
    }

    #[must_use]
    pub fn with_reservation_lifecycle_port(
        mut self,
        port: Arc<dyn ReservationLifecyclePort>,
    ) -> Self {
        self.scheduler_task_orchestrator = self
            .scheduler_task_orchestrator
            .with_reservation_lifecycle_port(port);
        self
    }

    #[must_use]
    pub fn with_runtime_dispatch_candidate_provider(
        mut self,
        provider: Arc<dyn WorkflowRuntimeDispatchCandidateProvider>,
    ) -> Self {
        self.runtime_dispatch_candidate_provider = provider;
        self
    }

    #[must_use]
    pub fn with_runtime_dispatch_source_refresher(
        mut self,
        refresher: Arc<dyn WorkflowRuntimeDispatchSourceRefresher>,
    ) -> Self {
        self.runtime_dispatch_source_refresher = refresher;
        self
    }

    #[must_use]
    pub fn with_dependency_environment_provider(
        mut self,
        provider: SharedDependencyEnvironmentProvider,
    ) -> Self {
        self = self.with_graph_session_fact_providers(
            Arc::new(UnavailableInferenceInterfaceFactsProvider),
            provider,
        );
        self
    }

    #[must_use]
    pub fn with_inference_interface_facts_provider(
        mut self,
        provider: Arc<dyn InferenceInterfaceFactsProvider>,
    ) -> Self {
        self.graph_session_store = Arc::new(
            GraphSessionStore::with_inference_interface_facts_provider(provider),
        );
        self
    }

    #[must_use]
    pub fn with_graph_session_fact_providers(
        mut self,
        inference_provider: Arc<dyn InferenceInterfaceFactsProvider>,
        provider: SharedDependencyEnvironmentProvider,
    ) -> Self {
        self.graph_session_store = Arc::new(GraphSessionStore::with_timeout_and_providers(
            std::time::Duration::from_secs(5 * 60),
            inference_provider,
            provider.clone(),
        ));
        self.dependency_readiness_provider = Arc::new(DependencyEnvironmentService::new(provider));
        self
    }

    #[must_use]
    pub fn with_dependency_readiness_work_queue(
        mut self,
        work_queue: Arc<DependencyReadinessWorkQueue>,
    ) -> Self {
        self.dependency_readiness_work_queue = work_queue;
        self
    }

    #[must_use]
    pub fn with_dependency_requirements_registry(
        mut self,
        registry: Arc<InMemoryDependencyRequirementsRegistry>,
    ) -> Self {
        self.dependency_requirements_registry = registry;
        self
    }

    pub fn store_dependency_requirements_payload_from_result(
        &self,
        result: &ValidatedDependencyEnvironmentResult,
    ) -> Result<(), WorkflowServiceError> {
        let payload = DependencyRequirementsPayload::from_result(result)
            .map_err(dependency_requirements_registry_error)?;
        self.dependency_requirements_registry
            .insert_payload(payload);
        Ok(())
    }

    pub fn with_artifact_store(mut self, store: ArtifactStore) -> Self {
        self.artifact_writer = Some(super::WorkflowArtifactWriter::new(store));
        self
    }

    pub fn with_artifact_writer(mut self, writer: super::WorkflowArtifactWriter) -> Self {
        self.artifact_writer = Some(writer);
        self
    }

    pub fn with_artifact_format_dependency_versions(
        mut self,
        versions: ArtifactFormatDependencyVersions,
    ) -> Self {
        self.artifact_format_dependency_versions = Arc::new(Mutex::new(versions));
        self
    }

    pub fn with_artifact_format_settings_path(
        mut self,
        path: impl Into<PathBuf>,
    ) -> Result<Self, WorkflowServiceError> {
        let path = path.into();
        let settings = load_artifact_format_settings(&path)?;
        self.artifact_format_settings = Arc::new(Mutex::new(settings));
        self.artifact_format_settings_path = Some(Arc::new(path));
        Ok(self)
    }

    pub fn with_attribution_store(mut self, store: SqliteAttributionStore) -> Self {
        self.attribution_store = Some(Arc::new(Mutex::new(store)));
        self
    }

    pub fn with_diagnostics_ledger(mut self, ledger: SqliteDiagnosticsLedger) -> Self {
        self.diagnostics_ledger = Some(Arc::new(Mutex::new(ledger)));
        self
    }

    pub fn set_diagnostics_projection_refresh_sink(
        &self,
        sink: Option<Arc<dyn WorkflowDiagnosticsProjectionRefreshSink>>,
    ) -> Result<(), WorkflowServiceError> {
        let mut guard = self
            .diagnostics_projection_refresh_sink
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "diagnostics projection refresh sink lock poisoned".to_string(),
                )
            })?;
        *guard = sink;
        Ok(())
    }

    pub fn with_ephemeral_attribution_store() -> Result<Self, WorkflowServiceError> {
        Ok(Self::new().with_attribution_store(
            SqliteAttributionStore::open_in_memory().map_err(WorkflowServiceError::from)?,
        ))
    }

    pub fn with_ephemeral_diagnostics_ledger() -> Result<Self, WorkflowServiceError> {
        Ok(Self::new().with_diagnostics_ledger(
            SqliteDiagnosticsLedger::open_in_memory().map_err(WorkflowServiceError::from)?,
        ))
    }

    pub fn set_scheduler_diagnostics_provider(
        &self,
        provider: Option<Arc<dyn WorkflowSchedulerDiagnosticsProvider>>,
    ) -> Result<(), WorkflowServiceError> {
        let mut guard = self.scheduler_diagnostics_provider.lock().map_err(|_| {
            WorkflowServiceError::Internal(
                "scheduler diagnostics provider lock poisoned".to_string(),
            )
        })?;
        *guard = provider;
        Ok(())
    }

    #[must_use]
    pub fn with_scheduler_diagnostics_provider(
        mut self,
        provider: Arc<dyn WorkflowSchedulerDiagnosticsProvider>,
    ) -> Self {
        self.scheduler_diagnostics_provider = Arc::new(Mutex::new(Some(provider)));
        self
    }

    pub fn set_loaded_runtime_capacity_limit(
        &self,
        max_loaded_sessions: Option<usize>,
    ) -> Result<(), WorkflowServiceError> {
        let mut store = self.session_store_guard()?;
        let Some(max_loaded_sessions) = max_loaded_sessions else {
            store.max_loaded_sessions = store.max_sessions;
            return Ok(());
        };
        if max_loaded_sessions == 0 {
            return Err(WorkflowServiceError::InvalidRequest(
                "max_loaded_sessions must be greater than zero".to_string(),
            ));
        }
        if max_loaded_sessions > store.max_sessions {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "max_loaded_sessions must be less than or equal to max_sessions ({})",
                store.max_sessions
            )));
        }
        store.max_loaded_sessions = max_loaded_sessions;
        Ok(())
    }

    pub(crate) fn session_store_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, WorkflowExecutionSessionStore>, WorkflowServiceError>
    {
        self.session_store
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("session store lock poisoned".to_string()))
    }

    pub(crate) fn attribution_store_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, SqliteAttributionStore>, WorkflowServiceError> {
        let Some(store) = self.attribution_store.as_ref() else {
            return Err(WorkflowServiceError::Internal(
                "attribution store is not configured".to_string(),
            ));
        };
        store.lock().map_err(|_| {
            WorkflowServiceError::Internal("attribution store lock poisoned".to_string())
        })
    }

    pub fn artifact_writer(&self) -> Result<super::WorkflowArtifactWriter, WorkflowServiceError> {
        let Some(writer) = self.artifact_writer.as_ref() else {
            return Err(WorkflowServiceError::Internal(
                "artifact store is not configured".to_string(),
            ));
        };
        Ok(writer.clone())
    }

    pub(crate) fn artifact_format_settings_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ArtifactFormatSettings>, WorkflowServiceError> {
        self.artifact_format_settings.lock().map_err(|_| {
            WorkflowServiceError::Internal("artifact format settings lock poisoned".to_string())
        })
    }

    pub(crate) fn artifact_format_settings_path(&self) -> Option<Arc<PathBuf>> {
        self.artifact_format_settings_path.clone()
    }

    pub fn set_artifact_format_dependency_versions(
        &self,
        versions: ArtifactFormatDependencyVersions,
    ) -> Result<(), WorkflowServiceError> {
        let mut guard = self
            .artifact_format_dependency_versions
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "artifact format dependency versions lock poisoned".to_string(),
                )
            })?;
        *guard = versions;
        Ok(())
    }

    pub(crate) fn artifact_format_dependency_versions(&self) -> ArtifactFormatDependencyVersions {
        self.artifact_format_dependency_versions
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub(crate) fn diagnostics_ledger_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, SqliteDiagnosticsLedger>, WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Err(WorkflowServiceError::Internal(
                "diagnostics ledger is not configured".to_string(),
            ));
        };
        ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })
    }
}

#[derive(Debug)]
struct RuntimeHostExecutionUnavailablePort;

#[async_trait]
impl RuntimeHostExecutionPort for RuntimeHostExecutionUnavailablePort {
    async fn execute_runtime_host_request(
        &self,
        _request: RuntimeHostExecutionRequest,
        _cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        Err(RuntimeHostExecutionPortError::ExecutionFailed {
            message: "runtime-host execution port is not configured for workflow-service"
                .to_string(),
        })
    }
}

fn default_scheduler_task_orchestrator() -> WorkflowSchedulerTaskOrchestrator {
    WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(Arc::new(
        RuntimeHostExecutionUnavailablePort,
    )))
}

fn default_dependency_readiness_provider() -> Arc<dyn WorkflowDependencyReadinessProvider> {
    Arc::new(DependencyEnvironmentService::new(
        NotImplementedDependencyEnvironmentProvider,
    ))
}

fn default_runtime_dispatch_candidate_provider() -> Arc<dyn WorkflowRuntimeDispatchCandidateProvider>
{
    Arc::new(NoRuntimeDispatchCandidatesProvider)
}

fn default_runtime_dispatch_source_refresher() -> Arc<dyn WorkflowRuntimeDispatchSourceRefresher> {
    Arc::new(NoRuntimeDispatchSourceRefresher)
}

fn dependency_requirements_registry_error(
    error: DependencyRequirementsRegistryError,
) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!(
        "dependency requirements registry seed failed: {error}"
    ))
}

fn load_artifact_format_settings(
    path: &Path,
) -> Result<ArtifactFormatSettings, WorkflowServiceError> {
    if !path.exists() {
        return Ok(ArtifactFormatSettings::default());
    }
    let content = std::fs::read_to_string(path).map_err(|error| {
        WorkflowServiceError::Internal(format!(
            "failed to read artifact format settings {:?}: {error}",
            path
        ))
    })?;
    serde_json::from_str(&content).map_err(|error| {
        WorkflowServiceError::InvalidRequest(format!(
            "artifact format settings file {:?} is invalid: {error}",
            path
        ))
    })
}
