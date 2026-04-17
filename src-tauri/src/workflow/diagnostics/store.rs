use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowGraph, WorkflowServiceError, WorkflowSessionQueueItem,
    WorkflowSessionSummary, WorkflowTraceEvent, WorkflowTraceRuntimeMetrics,
    WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse, WorkflowTraceStore,
};

use super::trace::{diagnostics_run_trace, graph_trace_context, workflow_trace_event};
use super::types::{
    DiagnosticsEventRecord, DiagnosticsRuntimeLifecycleSnapshot, DiagnosticsRuntimeSnapshot,
    DiagnosticsSchedulerSnapshot, WorkflowDiagnosticsProjection,
};
use crate::workflow::events::WorkflowEvent;

const DEFAULT_DIAGNOSTICS_EVENT_LIMIT: usize = 200;

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DiagnosticsNodeOverlay {
    pub(crate) last_progress: Option<f32>,
    pub(crate) last_message: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiagnosticsRunOverlay {
    pub(crate) last_updated_at_ms: u64,
    pub(crate) last_dirty_tasks: Vec<String>,
    pub(crate) last_incremental_task_ids: Vec<String>,
    pub(crate) nodes_by_id: BTreeMap<String, DiagnosticsNodeOverlay>,
    pub(crate) events: Vec<DiagnosticsEventRecord>,
}

impl DiagnosticsRunOverlay {
    pub(crate) fn new(timestamp_ms: u64) -> Self {
        Self {
            last_updated_at_ms: timestamp_ms,
            last_dirty_tasks: Vec::new(),
            last_incremental_task_ids: Vec::new(),
            nodes_by_id: BTreeMap::new(),
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct WorkflowDiagnosticsState {
    overlays_by_execution_id: BTreeMap<String, DiagnosticsRunOverlay>,
    runtime: DiagnosticsRuntimeSnapshot,
    scheduler: DiagnosticsSchedulerSnapshot,
    retained_event_limit: usize,
}

impl WorkflowDiagnosticsState {
    fn new(retained_event_limit: usize) -> Self {
        Self {
            overlays_by_execution_id: BTreeMap::new(),
            runtime: DiagnosticsRuntimeSnapshot::default(),
            scheduler: DiagnosticsSchedulerSnapshot::default(),
            retained_event_limit,
        }
    }

    fn snapshot(&self, traces: &WorkflowTraceSnapshotResponse) -> WorkflowDiagnosticsProjection {
        let run_order = traces
            .traces
            .iter()
            .map(|trace| trace.execution_id.clone())
            .collect::<Vec<_>>();
        let runs_by_id = traces
            .traces
            .iter()
            .map(|trace| {
                let overlay = self
                    .overlays_by_execution_id
                    .get(&trace.execution_id)
                    .cloned();
                (
                    trace.execution_id.clone(),
                    diagnostics_run_trace(trace, overlay),
                )
            })
            .collect();

        WorkflowDiagnosticsProjection {
            runs_by_id,
            run_order,
            runtime: self.runtime.clone(),
            scheduler: self.scheduler.clone(),
            retained_event_limit: self.retained_event_limit,
        }
    }

    fn clear_history(&mut self) {
        self.overlays_by_execution_id.clear();
    }

    fn prune_overlays(&mut self, traces: &WorkflowTraceSnapshotResponse) {
        let retained_execution_ids = traces
            .traces
            .iter()
            .map(|trace| trace.execution_id.as_str())
            .collect::<HashSet<_>>();
        self.overlays_by_execution_id
            .retain(|execution_id, _| retained_execution_ids.contains(execution_id.as_str()));
    }
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
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn trace_snapshot(
        &self,
        request: WorkflowTraceSnapshotRequest,
    ) -> Result<WorkflowTraceSnapshotResponse, WorkflowServiceError> {
        self.trace_store.snapshot(&request)
    }

    pub fn clear_history(&self) -> WorkflowDiagnosticsProjection {
        let traces = self.trace_store.clear_history();
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
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
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
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
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
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
        let traces = workflow_trace_event(event)
            .map(|trace_event| self.trace_store.record_event(&trace_event, timestamp_ms))
            .unwrap_or_else(|| self.trace_store.snapshot_all());
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        record_diagnostics_overlay(&mut state, event, timestamp_ms);
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn record_workflow_event_now(
        &self,
        event: &WorkflowEvent,
    ) -> WorkflowDiagnosticsProjection {
        let (traces, timestamp_ms) = workflow_trace_event(event)
            .map(|trace_event| {
                let result = self.trace_store.record_event_now(&trace_event);
                (result.snapshot, result.recorded_at_ms)
            })
            .unwrap_or_else(|| {
                let timestamp_ms = unix_timestamp_ms();
                (self.trace_store.snapshot_all(), timestamp_ms)
            });
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        record_diagnostics_overlay(&mut state, event, timestamp_ms);
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }

    pub fn record_trace_event_with_overlay(
        &self,
        trace_event: &WorkflowTraceEvent,
        overlay_event: &WorkflowEvent,
        timestamp_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let traces = self.trace_store.record_event(trace_event, timestamp_ms);
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        record_diagnostics_overlay(&mut state, overlay_event, timestamp_ms);
        state.prune_overlays(&traces);
        state.snapshot(&traces)
    }
}

pub type SharedWorkflowDiagnosticsStore = Arc<WorkflowDiagnosticsStore>;

fn record_diagnostics_overlay(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if matches!(event, WorkflowEvent::RuntimeSnapshot { .. }) {
        apply_runtime_event(state, event, timestamp_ms);
    }
    if matches!(event, WorkflowEvent::SchedulerSnapshot { .. }) {
        apply_scheduler_event(state, event, timestamp_ms);
    }

    let Some(execution_id) = event_execution_id(event) else {
        return;
    };

    let overlay = state
        .overlays_by_execution_id
        .entry(execution_id.clone())
        .or_insert_with(|| DiagnosticsRunOverlay::new(timestamp_ms));
    overlay.last_updated_at_ms = timestamp_ms;

    if let Some(node_id) = event_node_id(event) {
        let node_overlay = overlay.nodes_by_id.entry(node_id).or_default();
        match event {
            WorkflowEvent::NodeStarted { .. } => {
                node_overlay.last_progress = None;
                node_overlay.last_message = None;
            }
            WorkflowEvent::NodeProgress {
                progress, message, ..
            } => {
                node_overlay.last_progress = Some(*progress);
                node_overlay.last_message = message.clone();
            }
            WorkflowEvent::WaitingForInput { message, .. } => {
                node_overlay.last_message = message
                    .clone()
                    .or_else(|| Some("Waiting for input".to_string()));
            }
            _ => {}
        }
    }

    match event {
        WorkflowEvent::GraphModified { dirty_tasks, .. } => {
            overlay.last_dirty_tasks = dirty_tasks.clone();
        }
        WorkflowEvent::IncrementalExecutionStarted { task_ids, .. } => {
            overlay.last_incremental_task_ids = task_ids.clone();
        }
        _ => {}
    }

    let sequence = overlay.events.len() + 1;
    overlay.events.push(DiagnosticsEventRecord {
        id: format!("{}-{}", execution_id, sequence),
        sequence,
        timestamp_ms,
        event_type: event_type_name(event).to_string(),
        execution_id,
        workflow_id: event_workflow_id(event),
        node_id: event_node_id(event),
        summary: summarize_event(event),
        payload: event_payload(event),
    });
    if overlay.events.len() > state.retained_event_limit {
        let excess = overlay.events.len() - state.retained_event_limit;
        overlay.events.drain(0..excess);
    }
}

fn apply_runtime_event(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if let WorkflowEvent::RuntimeSnapshot {
        workflow_id,
        capabilities,
        active_model_target,
        embedding_model_target,
        active_runtime_snapshot,
        embedding_runtime_snapshot,
        error,
        ..
    } = event
    {
        state.runtime = DiagnosticsRuntimeSnapshot::from_capabilities(
            workflow_id.clone(),
            capabilities.clone(),
            error.clone(),
            active_model_target.clone(),
            embedding_model_target.clone(),
            active_runtime_snapshot.clone(),
            embedding_runtime_snapshot.clone(),
            timestamp_ms,
        );
    }
}

fn apply_scheduler_event(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if let WorkflowEvent::SchedulerSnapshot {
        workflow_id,
        execution_id,
        session_id,
        session,
        items,
        diagnostics,
        error,
        ..
    } = event
    {
        state.scheduler = DiagnosticsSchedulerSnapshot {
            workflow_id: workflow_id.clone(),
            session_id: Some(session_id.clone()),
            trace_execution_id: Some(execution_id.clone()),
            captured_at_ms: Some(timestamp_ms),
            session: session.clone(),
            items: items.clone(),
            diagnostics: diagnostics.clone(),
            last_error: error.clone(),
        };
    }
}

fn event_type_name(event: &WorkflowEvent) -> &'static str {
    match event {
        WorkflowEvent::Started { .. } => "Started",
        WorkflowEvent::NodeStarted { .. } => "NodeStarted",
        WorkflowEvent::NodeProgress { .. } => "NodeProgress",
        WorkflowEvent::NodeStream { .. } => "NodeStream",
        WorkflowEvent::NodeCompleted { .. } => "NodeCompleted",
        WorkflowEvent::NodeError { .. } => "NodeError",
        WorkflowEvent::Completed { .. } => "Completed",
        WorkflowEvent::Failed { .. } => "Failed",
        WorkflowEvent::Cancelled { .. } => "Cancelled",
        WorkflowEvent::GraphModified { .. } => "GraphModified",
        WorkflowEvent::WaitingForInput { .. } => "WaitingForInput",
        WorkflowEvent::IncrementalExecutionStarted { .. } => "IncrementalExecutionStarted",
        WorkflowEvent::RuntimeSnapshot { .. } => "RuntimeSnapshot",
        WorkflowEvent::SchedulerSnapshot { .. } => "SchedulerSnapshot",
        WorkflowEvent::DiagnosticsSnapshot { .. } => "DiagnosticsSnapshot",
    }
}

fn event_execution_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::Started { execution_id, .. }
        | WorkflowEvent::NodeStarted { execution_id, .. }
        | WorkflowEvent::NodeProgress { execution_id, .. }
        | WorkflowEvent::NodeStream { execution_id, .. }
        | WorkflowEvent::NodeCompleted { execution_id, .. }
        | WorkflowEvent::NodeError { execution_id, .. }
        | WorkflowEvent::Completed { execution_id, .. }
        | WorkflowEvent::Failed { execution_id, .. }
        | WorkflowEvent::Cancelled { execution_id, .. }
        | WorkflowEvent::GraphModified { execution_id, .. }
        | WorkflowEvent::WaitingForInput { execution_id, .. }
        | WorkflowEvent::IncrementalExecutionStarted { execution_id, .. }
        | WorkflowEvent::RuntimeSnapshot { execution_id, .. }
        | WorkflowEvent::SchedulerSnapshot { execution_id, .. }
        | WorkflowEvent::DiagnosticsSnapshot { execution_id, .. } => Some(execution_id.clone()),
    }
}

fn event_workflow_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::Started { workflow_id, .. }
        | WorkflowEvent::Completed { workflow_id, .. }
        | WorkflowEvent::Failed { workflow_id, .. }
        | WorkflowEvent::Cancelled { workflow_id, .. }
        | WorkflowEvent::GraphModified { workflow_id, .. }
        | WorkflowEvent::WaitingForInput { workflow_id, .. }
        | WorkflowEvent::IncrementalExecutionStarted { workflow_id, .. } => {
            Some(workflow_id.clone())
        }
        WorkflowEvent::RuntimeSnapshot { workflow_id, .. } => Some(workflow_id.clone()),
        WorkflowEvent::SchedulerSnapshot { workflow_id, .. } => workflow_id.clone(),
        _ => None,
    }
}

fn event_node_id(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::NodeStarted { node_id, .. }
        | WorkflowEvent::NodeProgress { node_id, .. }
        | WorkflowEvent::NodeStream { node_id, .. }
        | WorkflowEvent::NodeCompleted { node_id, .. }
        | WorkflowEvent::NodeError { node_id, .. }
        | WorkflowEvent::WaitingForInput { node_id, .. } => Some(node_id.clone()),
        _ => None,
    }
}

fn event_payload(event: &WorkflowEvent) -> serde_json::Value {
    match serde_json::to_value(event) {
        Ok(serde_json::Value::Object(mut value)) => {
            value.remove("data").unwrap_or(serde_json::Value::Null)
        }
        Ok(_) | Err(_) => serde_json::Value::Null,
    }
}

fn summarize_event(event: &WorkflowEvent) -> String {
    match event {
        WorkflowEvent::Started { node_count, .. } => {
            format!("Workflow started ({} nodes)", node_count)
        }
        WorkflowEvent::NodeStarted { node_id, .. } => format!("Node {} started", node_id),
        WorkflowEvent::NodeProgress {
            node_id,
            progress,
            message,
            ..
        } => message
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                format!("Node {} progress {}%", node_id, (progress * 100.0).round())
            }),
        WorkflowEvent::NodeStream { node_id, port, .. } => {
            format!("Node {} streamed on {}", node_id, port)
        }
        WorkflowEvent::NodeCompleted { node_id, .. } => format!("Node {} completed", node_id),
        WorkflowEvent::NodeError { node_id, error, .. } => {
            format!("Node {} failed: {}", node_id, error)
        }
        WorkflowEvent::Completed { .. } => "Workflow completed".to_string(),
        WorkflowEvent::Failed { error, .. } => format!("Workflow failed: {}", error),
        WorkflowEvent::Cancelled { error, .. } => format!("Workflow cancelled: {}", error),
        WorkflowEvent::GraphModified { dirty_tasks, .. } if !dirty_tasks.is_empty() => {
            format!("Graph modified; dirty tasks: {}", dirty_tasks.join(", "))
        }
        WorkflowEvent::GraphModified { .. } => "Graph modified".to_string(),
        WorkflowEvent::WaitingForInput {
            node_id, message, ..
        } => message
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("Waiting for input on {}", node_id)),
        WorkflowEvent::IncrementalExecutionStarted { task_ids, .. } if !task_ids.is_empty() => {
            format!("Incremental execution for {}", task_ids.join(", "))
        }
        WorkflowEvent::IncrementalExecutionStarted { .. } => {
            "Incremental execution started".to_string()
        }
        WorkflowEvent::RuntimeSnapshot { error, .. } => error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("Runtime snapshot failed: {}", value))
            .unwrap_or_else(|| "Runtime snapshot captured".to_string()),
        WorkflowEvent::SchedulerSnapshot { items, error, .. } => error
            .clone()
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("Scheduler snapshot failed: {}", value))
            .unwrap_or_else(|| {
                format!("Scheduler snapshot captured ({} queue items)", items.len())
            }),
        WorkflowEvent::DiagnosticsSnapshot { .. } => "Diagnostics snapshot captured".to_string(),
    }
}
