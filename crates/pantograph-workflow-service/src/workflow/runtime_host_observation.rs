use std::sync::{Arc, Mutex};

use pantograph_diagnostics_ledger::{
    DiagnosticsLedgerRepository, RuntimeHostObservationQuery, RuntimeHostObservationSummary,
    RuntimeHostRequestObservation, SqliteDiagnosticsLedger,
};

use super::{WorkflowService, WorkflowServiceError};

/// Shares the existing ledger without retaining the service or execution port.
/// Recording/query failure stays explicit; a missing recorder means no ledger.
#[derive(Clone)]
pub struct WorkflowRuntimeHostObservationRecorder {
    ledger: Arc<Mutex<SqliteDiagnosticsLedger>>,
}

impl WorkflowRuntimeHostObservationRecorder {
    pub fn record(
        &self,
        observation: RuntimeHostRequestObservation,
    ) -> Result<(), WorkflowServiceError> {
        self.ledger
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("diagnostics ledger lock poisoned".into()))?
            .record_runtime_host_observation(observation)
            .map_err(WorkflowServiceError::from)
    }

    pub fn summary(
        &self,
        query: RuntimeHostObservationQuery,
    ) -> Result<RuntimeHostObservationSummary, WorkflowServiceError> {
        self.ledger
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("diagnostics ledger lock poisoned".into()))?
            .runtime_host_observation_summary(query)
            .map_err(WorkflowServiceError::from)
    }
}

impl WorkflowService {
    pub fn runtime_host_observation_recorder(
        &self,
    ) -> Option<WorkflowRuntimeHostObservationRecorder> {
        self.diagnostics_ledger
            .as_ref()
            .map(|ledger| WorkflowRuntimeHostObservationRecorder {
                ledger: ledger.clone(),
            })
    }
}
