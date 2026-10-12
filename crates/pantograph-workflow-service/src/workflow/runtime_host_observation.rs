use std::sync::{Arc, Mutex};

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticsLedgerRepository,
    RuntimeHostObservationQuery, RuntimeHostObservationSummary, RuntimeHostRequestObservation,
    SqliteDiagnosticsLedger,
};

use super::{WorkflowDiagnosticsProjectionRefreshSink, WorkflowService, WorkflowServiceError};

/// Shares the existing ledger without retaining the service or execution port.
/// Recording/query failure stays explicit; a missing recorder means no ledger.
#[derive(Clone)]
pub struct WorkflowRuntimeHostObservationRecorder {
    ledger: Arc<Mutex<SqliteDiagnosticsLedger>>,
    refresh_sink: Arc<Mutex<Option<Arc<dyn WorkflowDiagnosticsProjectionRefreshSink>>>>,
}

impl WorkflowRuntimeHostObservationRecorder {
    /// Record a host-owned node status without retaining the service or port.
    /// Scheduler-attempt events alone are deliberately not node-status evidence.
    pub fn record_execution_status(
        &self,
        request: DiagnosticEventAppendRequest,
    ) -> Result<(), WorkflowServiceError> {
        if !matches!(
            &request.payload,
            DiagnosticEventPayload::NodeExecutionStatus(_)
        ) {
            return Err(WorkflowServiceError::InvalidRequest(
                "runtime-host status recorder requires a node execution status".into(),
            ));
        }
        let refresh =
            super::diagnostics_api::diagnostics_projection_refresh_request_for_event(&request);
        self.ledger
            .lock()
            .map_err(|_| WorkflowServiceError::Internal("diagnostics ledger lock poisoned".into()))?
            .append_diagnostic_event(request)?;
        let sink = self
            .refresh_sink
            .lock()
            .ok()
            .and_then(|guard| guard.clone());
        if let (Some(sink), Some(refresh)) = (sink, refresh) {
            sink.request_projection_refresh(refresh);
        }
        Ok(())
    }

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
                refresh_sink: self.diagnostics_projection_refresh_sink.clone(),
            })
    }
}
