use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentService, NotImplementedDependencyEnvironmentProvider,
    SharedDependencyEnvironmentProvider,
};
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionPort, RuntimeHostExecutionPortError, RuntimeHostExecutionRequest,
    RuntimeHostExecutionResponse, SchedulerRuntimeHostDispatcher,
};

use crate::graph::GraphSessionStore;
use crate::scheduler::{
    WorkflowDependencyReadinessProvider, WorkflowExecutionSessionStore,
    WorkflowSchedulerTaskOrchestrator,
};

use super::{
    ArtifactFormatDependencyVersions, ArtifactFormatSettings, ArtifactStore,
    SqliteAttributionStore, SqliteDiagnosticsLedger, WorkflowDiagnosticsProjectionRefreshSink,
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
            artifact_store: None,
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
        }
    }

    pub fn with_runtime_host_execution_port(
        mut self,
        port: Arc<dyn RuntimeHostExecutionPort>,
    ) -> Self {
        self.scheduler_task_orchestrator =
            WorkflowSchedulerTaskOrchestrator::new(SchedulerRuntimeHostDispatcher::new(port));
        self
    }

    #[must_use]
    pub fn with_dependency_environment_provider(
        mut self,
        provider: SharedDependencyEnvironmentProvider,
    ) -> Self {
        self.graph_session_store = Arc::new(
            GraphSessionStore::with_dependency_environment_provider(provider.clone()),
        );
        self.dependency_readiness_provider = Arc::new(DependencyEnvironmentService::new(provider));
        self
    }

    pub fn with_artifact_store(mut self, store: ArtifactStore) -> Self {
        self.artifact_store = Some(Arc::new(Mutex::new(store)));
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

    pub(crate) fn artifact_store_guard(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, ArtifactStore>, WorkflowServiceError> {
        let Some(store) = self.artifact_store.as_ref() else {
            return Err(WorkflowServiceError::Internal(
                "artifact store is not configured".to_string(),
            ));
        };
        store
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("artifact store lock poisoned".to_string()))
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
