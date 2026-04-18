use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowGraph, WorkflowServiceError, WorkflowSessionQueueItem,
    WorkflowSessionSummary, WorkflowTraceEvent, WorkflowTraceRuntimeMetrics,
    WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStore,
};

use super::attempts::{
    overlay_record_decision, trace_attempt_state_for_execution, trace_attempt_state_in_snapshot,
    trace_event_execution_id, OverlayRecordDecision,
};
use super::overlay::{event_execution_id, record_diagnostics_overlay, WorkflowDiagnosticsState};
use super::trace::{graph_trace_context, workflow_trace_event};
use super::types::{
    DiagnosticsRuntimeLifecycleSnapshot, DiagnosticsRuntimeSnapshot, DiagnosticsSchedulerSnapshot,
    WorkflowDiagnosticsProjection,
};
use crate::workflow::events::WorkflowEvent;

const DEFAULT_DIAGNOSTICS_EVENT_LIMIT: usize = 200;

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[derive(Debug)]
pub struct WorkflowDiagnosticsStore {
    state: Mutex<WorkflowDiagnosticsState>,
    trace_store: WorkflowTraceStore,
}

impl Default for WorkflowDiagnosticsStore {
    fn default() -> Self {
        Self::new(DEFAULT_DIAGNOSTICS_EVENT_LIMIT)
    }
}

impl WorkflowDiagnosticsStore {
    pub fn new(retained_event_limit: usize) -> Self {
        Self {
            state: Mutex::new(WorkflowDiagnosticsState::new(retained_event_limit)),
            trace_store: WorkflowTraceStore::new(retained_event_limit),
        }
    }

    pub fn snapshot(&self) -> WorkflowDiagnosticsProjection {
        let traces = self.trace_store.snapshot_all();
        let mut state = self.state.lock();
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn trace_snapshot(
        &self,
        request: WorkflowTraceSnapshotRequest,
    ) -> Result<WorkflowTraceSnapshotResponse, WorkflowServiceError> {
        self.trace_store.snapshot(&request)
    }

    pub fn select_trace_runtime_metrics(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> Result<WorkflowTraceRuntimeSelection, WorkflowServiceError> {
        self.trace_store.select_runtime_metrics(request)
    }

    pub fn clear_history(&self) -> WorkflowDiagnosticsProjection {
        let traces = self.trace_store.clear_history();
        let mut state = self.state.lock();
        state.clear_history();
        state.snapshot(&traces)
    }

    pub fn set_execution_metadata(
        &self,
        execution_id: &str,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) {
        self.trace_store
            .set_execution_metadata(execution_id, workflow_id, workflow_name);
    }

    pub fn set_execution_graph(&self, execution_id: &str, graph: &WorkflowGraph) {
        self.trace_store
            .set_execution_graph_context(execution_id, &graph_trace_context(graph));
    }

    pub fn record_runtime_snapshot(
        &self,
        workflow_id: String,
        execution_id: String,
        captured_at_ms: u64,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
        active_model_target: Option<String>,
        embedding_model_target: Option<String>,
        active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        error: Option<String>,
    ) -> WorkflowDiagnosticsProjection {
        let event = WorkflowEvent::runtime_snapshot(
            workflow_id,
            execution_id,
            captured_at_ms,
            capabilities,
            trace_runtime_metrics,
            active_model_target,
            embedding_model_target,
            active_runtime_snapshot,
            embedding_runtime_snapshot,
            error,
        );
        self.record_workflow_event(&event, captured_at_ms)
    }

    pub fn record_scheduler_snapshot(
        &self,
        workflow_id: Option<String>,
        execution_id: String,
        session_id: String,
        captured_at_ms: u64,
        session: Option<WorkflowSessionSummary>,
        items: Vec<WorkflowSessionQueueItem>,
        diagnostics: Option<pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics>,
        error: Option<String>,
    ) -> WorkflowDiagnosticsProjection {
        let event = WorkflowEvent::scheduler_snapshot(
            workflow_id,
            execution_id,
            session_id,
            captured_at_ms,
            session,
            items,
            diagnostics,
            error,
        );
        self.record_workflow_event(&event, captured_at_ms)
    }

    pub fn update_runtime_snapshot(
        &self,
        workflow_id: Option<String>,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        last_error: Option<String>,
        active_model_target: Option<String>,
        embedding_model_target: Option<String>,
        active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
        captured_at_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self.state.lock();
        state.runtime = match workflow_id {
            Some(workflow_id) => DiagnosticsRuntimeSnapshot::from_capabilities(
                workflow_id,
                capabilities,
                last_error,
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot
                    .as_ref()
                    .map(DiagnosticsRuntimeLifecycleSnapshot::from),
                embedding_runtime_snapshot
                    .as_ref()
                    .map(DiagnosticsRuntimeLifecycleSnapshot::from),
                captured_at_ms,
            ),
            None => DiagnosticsRuntimeSnapshot::default(),
        };
        let traces = self.trace_store.snapshot_all();
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn update_scheduler_snapshot(
        &self,
        workflow_id: Option<String>,
        session_id: Option<String>,
        session: Option<WorkflowSessionSummary>,
        items: Vec<WorkflowSessionQueueItem>,
        diagnostics: Option<pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics>,
        last_error: Option<String>,
        captured_at_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self.state.lock();
        state.scheduler = match session_id {
            Some(session_id) => DiagnosticsSchedulerSnapshot {
                workflow_id,
                session_id: Some(session_id),
                trace_execution_id: None,
                captured_at_ms: Some(captured_at_ms),
                session,
                items,
                diagnostics,
                last_error,
            },
            None => DiagnosticsSchedulerSnapshot::default(),
        };
        let traces = self.trace_store.snapshot_all();
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn record_workflow_event(
        &self,
        event: &WorkflowEvent,
        timestamp_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let (traces, execution_id, overlay_decision) = workflow_trace_event(event)
            .map(|trace_event| {
                let execution_id = trace_event_execution_id(&trace_event).to_string();
                let previous_state =
                    trace_attempt_state_for_execution(&self.trace_store, &execution_id);
                let traces = self.trace_store.record_event(&trace_event, timestamp_ms);
                let current_state = trace_attempt_state_in_snapshot(&traces, &execution_id);
                (
                    traces,
                    Some(execution_id),
                    overlay_record_decision(previous_state, current_state),
                )
            })
            .unwrap_or_else(|| {
                (
                    self.trace_store.snapshot_all(),
                    event_execution_id(event),
                    OverlayRecordDecision {
                        reset_overlay: false,
                        record_overlay: true,
                    },
                )
            });
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            if let Some(execution_id) = execution_id.as_deref() {
                state.overlays_by_execution_id.remove(execution_id);
            }
        }
        if overlay_decision.record_overlay {
            record_diagnostics_overlay(&mut state, event, timestamp_ms);
        }
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn record_workflow_event_now(
        &self,
        event: &WorkflowEvent,
    ) -> WorkflowDiagnosticsProjection {
        let (traces, timestamp_ms, execution_id, overlay_decision) = workflow_trace_event(event)
            .map(|trace_event| {
                let execution_id = trace_event_execution_id(&trace_event).to_string();
                let previous_state =
                    trace_attempt_state_for_execution(&self.trace_store, &execution_id);
                let result = self.trace_store.record_event_now(&trace_event);
                let current_state =
                    trace_attempt_state_in_snapshot(&result.snapshot, &execution_id);
                (
                    result.snapshot,
                    result.recorded_at_ms,
                    Some(execution_id),
                    overlay_record_decision(previous_state, current_state),
                )
            })
            .unwrap_or_else(|| {
                let timestamp_ms = unix_timestamp_ms();
                (
                    self.trace_store.snapshot_all(),
                    timestamp_ms,
                    event_execution_id(event),
                    OverlayRecordDecision {
                        reset_overlay: false,
                        record_overlay: true,
                    },
                )
            });
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            if let Some(execution_id) = execution_id.as_deref() {
                state.overlays_by_execution_id.remove(execution_id);
            }
        }
        if overlay_decision.record_overlay {
            record_diagnostics_overlay(&mut state, event, timestamp_ms);
        }
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn record_trace_event_with_overlay(
        &self,
        trace_event: &WorkflowTraceEvent,
        overlay_event: &WorkflowEvent,
        timestamp_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let execution_id = trace_event_execution_id(trace_event).to_string();
        let previous_state = trace_attempt_state_for_execution(&self.trace_store, &execution_id);
        let traces = self.trace_store.record_event(trace_event, timestamp_ms);
        let current_state = trace_attempt_state_in_snapshot(&traces, &execution_id);
        let overlay_decision = overlay_record_decision(previous_state, current_state);
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            state.overlays_by_execution_id.remove(&execution_id);
        }
        if overlay_decision.record_overlay {
            record_diagnostics_overlay(&mut state, overlay_event, timestamp_ms);
        }
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }
}

pub type SharedWorkflowDiagnosticsStore = Arc<WorkflowDiagnosticsStore>;
