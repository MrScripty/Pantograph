use std::sync::{Arc, Mutex};

use crate::graph::GraphSessionStore;
use crate::scheduler::WorkflowExecutionSessionStore;

use super::{
    SqliteAttributionStore, SqliteDiagnosticsLedger, WorkflowSchedulerDiagnosticsProvider,
    WorkflowService, WorkflowServiceError,
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
            attribution_store: None,
            diagnostics_ledger: None,
            scheduler_diagnostics_provider: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_attribution_store(mut self, store: SqliteAttributionStore) -> Self {
        self.attribution_store = Some(Arc::new(Mutex::new(store)));
        self
    }

    pub fn with_diagnostics_ledger(mut self, ledger: SqliteDiagnosticsLedger) -> Self {
        self.diagnostics_ledger = Some(Arc::new(Mutex::new(ledger)));
        self
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
        store.max_loaded_sessions = max_loaded_sessions
            .unwrap_or(store.max_sessions)
            .max(1)
            .min(store.max_sessions);
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
