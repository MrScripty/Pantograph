use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowGraph, WorkflowServiceError, WorkflowTraceEvent,
    WorkflowTraceRuntimeMetrics, WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotRequest,
    WorkflowTraceSnapshotResponse, WorkflowTraceStore,
};
use parking_lot::Mutex;

use super::attempts::{
    OverlayRecordDecision, overlay_record_decision, trace_attempt_state_for_workflow_run,
    trace_attempt_state_in_snapshot, trace_event_workflow_run_id,
};
use super::overlay::{WorkflowDiagnosticsState, event_workflow_run_id, record_diagnostics_overlay};
use super::trace::{graph_trace_context, workflow_trace_event};
use super::types::{
    DiagnosticsRuntimeLifecycleSnapshot, DiagnosticsRuntimeSnapshot,
    DiagnosticsRuntimeSnapshotInput, DiagnosticsSchedulerSnapshot,
    DiagnosticsWorkflowTimingHistory, WorkflowDiagnosticsProjection,
};
use crate::workflow::events::{
    WorkflowEvent, WorkflowRuntimeSnapshotEventInput, WorkflowSchedulerSnapshotEventInput,
};

const DEFAULT_DIAGNOSTICS_EVENT_LIMIT: usize = 200;

#[derive(Debug, Clone, Default)]
pub struct WorkflowRuntimeSnapshotRecord {
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub captured_at_ms: u64,
    pub capabilities: Option<WorkflowCapabilitiesResponse>,
    pub trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowSchedulerSnapshotRecord {
    pub workflow_id: Option<String>,
    pub workflow_run_id: String,
    pub session_id: String,
    pub captured_at_ms: u64,
    pub session: Option<WorkflowExecutionSessionSummary>,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    pub diagnostics: Option<pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowRuntimeSnapshotUpdate {
    pub workflow_id: Option<String>,
    pub capabilities: Option<WorkflowCapabilitiesResponse>,
    pub last_error: Option<String>,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub captured_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WorkflowSchedulerSnapshotUpdate {
    pub workflow_id: Option<String>,
    pub session_id: Option<String>,
    pub session: Option<WorkflowExecutionSessionSummary>,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    pub diagnostics: Option<pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics>,
    pub last_error: Option<String>,
    pub captured_at_ms: u64,
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

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

    pub fn with_timing_ledger(
        retained_event_limit: usize,
        timing_ledger: pantograph_workflow_service::SqliteDiagnosticsLedger,
    ) -> Self {
        Self {
            state: Mutex::new(WorkflowDiagnosticsState::new(retained_event_limit)),
            trace_store: WorkflowTraceStore::with_timing_ledger(
                retained_event_limit,
                timing_ledger,
            ),
        }
    }

    pub fn with_default_timing_ledger(
        timing_ledger: pantograph_workflow_service::SqliteDiagnosticsLedger,
    ) -> Self {
        Self::with_timing_ledger(DEFAULT_DIAGNOSTICS_EVENT_LIMIT, timing_ledger)
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

    pub fn set_execution_metadata(&self, workflow_run_id: &str, workflow_id: Option<String>) {
        self.trace_store
            .set_execution_metadata(workflow_run_id, workflow_id);
    }

    pub fn set_execution_graph(&self, workflow_run_id: &str, graph: &WorkflowGraph) {
        self.trace_store
            .set_execution_graph_context(workflow_run_id, &graph_trace_context(graph));
    }

    pub fn workflow_timing_history(
        &self,
        workflow_id: String,
        graph: &WorkflowGraph,
    ) -> DiagnosticsWorkflowTimingHistory {
        DiagnosticsWorkflowTimingHistory::from(
            &self
                .trace_store
                .graph_timing_expectations(workflow_id, &graph_trace_context(graph)),
        )
    }

    pub fn record_runtime_snapshot(
        &self,
        input: WorkflowRuntimeSnapshotRecord,
    ) -> WorkflowDiagnosticsProjection {
        let event = WorkflowEvent::runtime_snapshot(WorkflowRuntimeSnapshotEventInput {
            workflow_id: input.workflow_id,
            workflow_run_id: input.workflow_run_id,
            captured_at_ms: input.captured_at_ms,
            capabilities: input.capabilities,
            trace_runtime_metrics: input.trace_runtime_metrics,
            active_model_target: input.active_model_target,
            embedding_model_target: input.embedding_model_target,
            active_runtime_snapshot: input.active_runtime_snapshot,
            embedding_runtime_snapshot: input.embedding_runtime_snapshot,
            managed_runtimes: input.managed_runtimes,
            error: input.error,
        });
        self.record_workflow_event(&event, input.captured_at_ms)
    }

    pub fn record_scheduler_snapshot(
        &self,
        input: WorkflowSchedulerSnapshotRecord,
    ) -> WorkflowDiagnosticsProjection {
        let event = WorkflowEvent::scheduler_snapshot(WorkflowSchedulerSnapshotEventInput {
            workflow_id: input.workflow_id,
            workflow_run_id: input.workflow_run_id,
            session_id: input.session_id,
            captured_at_ms: input.captured_at_ms,
            session: input.session,
            items: input.items,
            diagnostics: input.diagnostics,
            error: input.error,
        });
        self.record_workflow_event(&event, input.captured_at_ms)
    }

    pub fn update_runtime_snapshot(
        &self,
        input: WorkflowRuntimeSnapshotUpdate,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self.state.lock();
        state.runtime = match input.workflow_id {
            Some(workflow_id) => {
                DiagnosticsRuntimeSnapshot::from_capabilities(DiagnosticsRuntimeSnapshotInput {
                    workflow_id,
                    capabilities: input.capabilities,
                    last_error: input.last_error,
                    active_model_target: input.active_model_target,
                    embedding_model_target: input.embedding_model_target,
                    active_runtime_snapshot: input
                        .active_runtime_snapshot
                        .as_ref()
                        .map(DiagnosticsRuntimeLifecycleSnapshot::from),
                    embedding_runtime_snapshot: input
                        .embedding_runtime_snapshot
                        .as_ref()
                        .map(DiagnosticsRuntimeLifecycleSnapshot::from),
                    managed_runtimes: input.managed_runtimes,
                    captured_at_ms: input.captured_at_ms,
                })
            }
            None => DiagnosticsRuntimeSnapshot::default(),
        };
        let traces = self.trace_store.snapshot_all();
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn update_scheduler_snapshot(
        &self,
        input: WorkflowSchedulerSnapshotUpdate,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self.state.lock();
        state.scheduler = match input.session_id {
            Some(session_id) => DiagnosticsSchedulerSnapshot {
                workflow_id: input.workflow_id,
                session_id: Some(session_id),
                workflow_run_id: None,
                captured_at_ms: Some(input.captured_at_ms),
                session: input.session,
                items: input.items,
                diagnostics: input.diagnostics,
                last_error: input.last_error,
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
        let (traces, workflow_run_id, overlay_decision) = workflow_trace_event(event)
            .map(|trace_event| {
                let workflow_run_id = trace_event_workflow_run_id(&trace_event).to_string();
                let previous_state =
                    trace_attempt_state_for_workflow_run(&self.trace_store, &workflow_run_id);
                let traces = self.trace_store.record_event(&trace_event, timestamp_ms);
                let current_state = trace_attempt_state_in_snapshot(&traces, &workflow_run_id);
                (
                    traces,
                    Some(workflow_run_id),
                    overlay_record_decision(previous_state, current_state),
                )
            })
            .unwrap_or_else(|| {
                (
                    self.trace_store.snapshot_all(),
                    event_workflow_run_id(event),
                    OverlayRecordDecision {
                        reset_overlay: false,
                        record_overlay: true,
                    },
                )
            });
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            if let Some(workflow_run_id) = workflow_run_id.as_deref() {
                state.overlays_by_workflow_run_id.remove(workflow_run_id);
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
        let (traces, timestamp_ms, workflow_run_id, overlay_decision) = workflow_trace_event(event)
            .map(|trace_event| {
                let workflow_run_id = trace_event_workflow_run_id(&trace_event).to_string();
                let previous_state =
                    trace_attempt_state_for_workflow_run(&self.trace_store, &workflow_run_id);
                let result = self.trace_store.record_event_now(&trace_event);
                let current_state =
                    trace_attempt_state_in_snapshot(&result.snapshot, &workflow_run_id);
                (
                    result.snapshot,
                    result.recorded_at_ms,
                    Some(workflow_run_id),
                    overlay_record_decision(previous_state, current_state),
                )
            })
            .unwrap_or_else(|| {
                let timestamp_ms = unix_timestamp_ms();
                (
                    self.trace_store.snapshot_all(),
                    timestamp_ms,
                    event_workflow_run_id(event),
                    OverlayRecordDecision {
                        reset_overlay: false,
                        record_overlay: true,
                    },
                )
            });
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            if let Some(workflow_run_id) = workflow_run_id.as_deref() {
                state.overlays_by_workflow_run_id.remove(workflow_run_id);
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
        let workflow_run_id = trace_event_workflow_run_id(trace_event).to_string();
        let previous_state =
            trace_attempt_state_for_workflow_run(&self.trace_store, &workflow_run_id);
        let traces = self.trace_store.record_event(trace_event, timestamp_ms);
        let current_state = trace_attempt_state_in_snapshot(&traces, &workflow_run_id);
        let overlay_decision = overlay_record_decision(previous_state, current_state);
        let mut state = self.state.lock();
        if overlay_decision.reset_overlay {
            state.overlays_by_workflow_run_id.remove(&workflow_run_id);
        }
        if overlay_decision.record_overlay {
            record_diagnostics_overlay(&mut state, overlay_event, timestamp_ms);
        }
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }
}

pub type SharedWorkflowDiagnosticsStore = Arc<WorkflowDiagnosticsStore>;
