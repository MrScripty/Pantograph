//! Backend-owned diagnostics projection for workflow execution traces.
//!
//! This module replaces the previous TypeScript-side diagnostics accumulator so
//! workflow run state, node lifecycle state, and retained event history are all
//! derived in Rust before the GUI renders them.

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowGraph, WorkflowServiceError, WorkflowSessionQueueItem,
    WorkflowSessionSummary, WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceNodeRecord,
    WorkflowTraceNodeStatus, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStatus, WorkflowTraceStore, WorkflowTraceSummary,
};
use serde::{Deserialize, Serialize};

use super::events::WorkflowEvent;

const DEFAULT_DIAGNOSTICS_EVENT_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsRunStatus {
    Running,
    Waiting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsNodeStatus {
    Running,
    Waiting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsEventRecord {
    pub id: String,
    pub sequence: usize,
    pub timestamp_ms: u64,
    #[serde(rename = "type")]
    pub event_type: String,
    pub execution_id: String,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
    pub summary: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsNodeTrace {
    pub node_id: String,
    #[serde(default)]
    pub node_type: Option<String>,
    pub status: DiagnosticsNodeStatus,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
    #[serde(default)]
    pub ended_at_ms: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub last_progress: Option<f32>,
    #[serde(default)]
    pub last_message: Option<String>,
    pub stream_event_count: usize,
    pub event_count: usize,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRunTrace {
    pub execution_id: String,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub graph_fingerprint_at_start: Option<String>,
    pub node_count_at_start: usize,
    pub status: DiagnosticsRunStatus,
    pub started_at_ms: u64,
    #[serde(default)]
    pub ended_at_ms: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub last_updated_at_ms: u64,
    #[serde(default)]
    pub error: Option<String>,
    pub waiting_for_input: bool,
    pub event_count: usize,
    pub stream_event_count: usize,
    pub last_dirty_tasks: Vec<String>,
    pub last_incremental_task_ids: Vec<String>,
    pub nodes: BTreeMap<String, DiagnosticsNodeTrace>,
    pub events: Vec<DiagnosticsEventRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRuntimeSnapshot {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub captured_at_ms: Option<u64>,
    #[serde(default)]
    pub max_input_bindings: Option<usize>,
    #[serde(default)]
    pub max_output_targets: Option<usize>,
    #[serde(default)]
    pub max_value_bytes: Option<usize>,
    #[serde(default)]
    pub runtime_requirements: Option<pantograph_workflow_service::WorkflowRuntimeRequirements>,
    #[serde(default)]
    pub runtime_capabilities: Vec<pantograph_workflow_service::WorkflowRuntimeCapability>,
    #[serde(default)]
    pub models: Vec<pantograph_workflow_service::WorkflowCapabilityModel>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSchedulerSnapshot {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub captured_at_ms: Option<u64>,
    #[serde(default)]
    pub session: Option<WorkflowSessionSummary>,
    pub items: Vec<WorkflowSessionQueueItem>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDiagnosticsProjection {
    pub runs_by_id: BTreeMap<String, DiagnosticsRunTrace>,
    pub run_order: Vec<String>,
    pub runtime: DiagnosticsRuntimeSnapshot,
    pub scheduler: DiagnosticsSchedulerSnapshot,
    pub retained_event_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDiagnosticsSnapshotRequest {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct DiagnosticsNodeOverlay {
    last_progress: Option<f32>,
    last_message: Option<String>,
}

#[derive(Debug, Clone)]
struct DiagnosticsRunOverlay {
    last_updated_at_ms: u64,
    last_dirty_tasks: Vec<String>,
    last_incremental_task_ids: Vec<String>,
    nodes_by_id: BTreeMap<String, DiagnosticsNodeOverlay>,
    events: Vec<DiagnosticsEventRecord>,
}

impl DiagnosticsRunOverlay {
    fn new(timestamp_ms: u64) -> Self {
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
            runtime: DiagnosticsRuntimeSnapshot {
                workflow_id: None,
                captured_at_ms: None,
                max_input_bindings: None,
                max_output_targets: None,
                max_value_bytes: None,
                runtime_requirements: None,
                runtime_capabilities: Vec::new(),
                models: Vec::new(),
                last_error: None,
            },
            scheduler: DiagnosticsSchedulerSnapshot {
                workflow_id: None,
                session_id: None,
                captured_at_ms: None,
                session: None,
                items: Vec::new(),
                last_error: None,
            },
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
                let overlay = self.overlays_by_execution_id.get(&trace.execution_id);
                (
                    trace.execution_id.clone(),
                    diagnostics_run_trace(trace, overlay.cloned()),
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

    pub fn update_runtime_snapshot(
        &self,
        workflow_id: Option<String>,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        last_error: Option<String>,
        captured_at_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        state.runtime = match workflow_id {
            Some(workflow_id) => DiagnosticsRuntimeSnapshot {
                workflow_id: Some(workflow_id),
                captured_at_ms: Some(captured_at_ms),
                max_input_bindings: capabilities.as_ref().map(|value| value.max_input_bindings),
                max_output_targets: capabilities.as_ref().map(|value| value.max_output_targets),
                max_value_bytes: capabilities.as_ref().map(|value| value.max_value_bytes),
                runtime_requirements: capabilities
                    .as_ref()
                    .map(|value| value.runtime_requirements.clone()),
                runtime_capabilities: capabilities
                    .as_ref()
                    .map(|value| value.runtime_capabilities.clone())
                    .unwrap_or_default(),
                models: capabilities
                    .as_ref()
                    .map(|value| value.models.clone())
                    .unwrap_or_default(),
                last_error,
            },
            None => WorkflowDiagnosticsState::new(state.retained_event_limit).runtime,
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
                captured_at_ms: Some(captured_at_ms),
                session,
                items,
                last_error,
            },
            None => WorkflowDiagnosticsState::new(state.retained_event_limit).scheduler,
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

fn diagnostics_run_trace(
    trace: &WorkflowTraceSummary,
    overlay: Option<DiagnosticsRunOverlay>,
) -> DiagnosticsRunTrace {
    let DiagnosticsRunOverlay {
        last_updated_at_ms,
        last_dirty_tasks,
        last_incremental_task_ids,
        nodes_by_id,
        events,
    } = overlay.unwrap_or_else(|| DiagnosticsRunOverlay::new(trace.started_at_ms));

    DiagnosticsRunTrace {
        execution_id: trace.execution_id.clone(),
        workflow_id: trace.workflow_id.clone(),
        workflow_name: trace.workflow_name.clone(),
        graph_fingerprint_at_start: trace.graph_fingerprint.clone(),
        node_count_at_start: trace.node_count_at_start,
        status: diagnostics_run_status(trace.status),
        started_at_ms: trace.started_at_ms,
        ended_at_ms: trace.ended_at_ms,
        duration_ms: trace.duration_ms,
        last_updated_at_ms: last_updated_at_ms
            .max(trace.ended_at_ms.unwrap_or(trace.started_at_ms)),
        error: trace.last_error.clone(),
        waiting_for_input: trace.waiting_for_input,
        event_count: trace.event_count,
        stream_event_count: trace.stream_event_count,
        last_dirty_tasks,
        last_incremental_task_ids,
        nodes: trace
            .nodes
            .iter()
            .map(|node| {
                let overlay = nodes_by_id.get(&node.node_id).cloned();
                (node.node_id.clone(), diagnostics_node_trace(node, overlay))
            })
            .collect(),
        events,
    }
}

fn diagnostics_node_trace(
    node: &WorkflowTraceNodeRecord,
    overlay: Option<DiagnosticsNodeOverlay>,
) -> DiagnosticsNodeTrace {
    let overlay = overlay.unwrap_or_default();
    DiagnosticsNodeTrace {
        node_id: node.node_id.clone(),
        node_type: node.node_type.clone(),
        status: diagnostics_node_status(node.status),
        started_at_ms: node.started_at_ms,
        ended_at_ms: node.ended_at_ms,
        duration_ms: node.duration_ms,
        last_progress: overlay.last_progress,
        last_message: overlay.last_message,
        stream_event_count: node.stream_event_count,
        event_count: node.event_count,
        error: node.last_error.clone(),
    }
}

fn diagnostics_run_status(status: WorkflowTraceStatus) -> DiagnosticsRunStatus {
    match status {
        WorkflowTraceStatus::Queued | WorkflowTraceStatus::Running => DiagnosticsRunStatus::Running,
        WorkflowTraceStatus::Waiting => DiagnosticsRunStatus::Waiting,
        WorkflowTraceStatus::Completed => DiagnosticsRunStatus::Completed,
        WorkflowTraceStatus::Failed | WorkflowTraceStatus::Cancelled => {
            DiagnosticsRunStatus::Failed
        }
    }
}

fn diagnostics_node_status(status: WorkflowTraceNodeStatus) -> DiagnosticsNodeStatus {
    match status {
        WorkflowTraceNodeStatus::Pending | WorkflowTraceNodeStatus::Running => {
            DiagnosticsNodeStatus::Running
        }
        WorkflowTraceNodeStatus::Waiting => DiagnosticsNodeStatus::Waiting,
        WorkflowTraceNodeStatus::Completed => DiagnosticsNodeStatus::Completed,
        WorkflowTraceNodeStatus::Failed | WorkflowTraceNodeStatus::Cancelled => {
            DiagnosticsNodeStatus::Failed
        }
    }
}

fn graph_trace_context(graph: &WorkflowGraph) -> WorkflowTraceGraphContext {
    WorkflowTraceGraphContext {
        graph_fingerprint: graph
            .derived_graph
            .as_ref()
            .map(|derived| derived.graph_fingerprint.clone()),
        node_count_at_start: graph.nodes.len(),
        node_types_by_id: graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.node_type.clone()))
            .collect(),
    }
}

fn workflow_trace_event(event: &WorkflowEvent) -> Option<WorkflowTraceEvent> {
    match event {
        WorkflowEvent::Started {
            workflow_id,
            node_count,
            execution_id,
        } => Some(WorkflowTraceEvent::RunStarted {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_count: *node_count,
        }),
        WorkflowEvent::NodeStarted {
            node_id,
            node_type,
            execution_id,
        } => Some(WorkflowTraceEvent::NodeStarted {
            execution_id: execution_id.clone(),
            node_id: node_id.clone(),
            node_type: (!node_type.trim().is_empty()).then(|| node_type.clone()),
        }),
        WorkflowEvent::NodeProgress {
            node_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::NodeProgress {
            execution_id: execution_id.clone(),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::NodeStream {
            node_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::NodeStream {
            execution_id: execution_id.clone(),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::NodeCompleted {
            node_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::NodeCompleted {
            execution_id: execution_id.clone(),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::NodeError {
            node_id,
            error,
            execution_id,
        } => Some(WorkflowTraceEvent::NodeFailed {
            execution_id: execution_id.clone(),
            node_id: node_id.clone(),
            error: error.clone(),
        }),
        WorkflowEvent::Completed {
            workflow_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::RunCompleted {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        }),
        WorkflowEvent::Failed {
            workflow_id,
            error,
            execution_id,
        } => Some(WorkflowTraceEvent::RunFailed {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            error: error.clone(),
        }),
        WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            node_id,
            ..
        } => Some(WorkflowTraceEvent::WaitingForInput {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
            node_id: node_id.clone(),
        }),
        WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::GraphModified {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        }),
        WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::IncrementalExecutionStarted {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        }),
        WorkflowEvent::RuntimeSnapshot {
            workflow_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::RuntimeSnapshotCaptured {
            execution_id: execution_id.clone(),
            workflow_id: Some(workflow_id.clone()),
        }),
        WorkflowEvent::SchedulerSnapshot {
            workflow_id,
            execution_id,
            ..
        } => Some(WorkflowTraceEvent::SchedulerSnapshotCaptured {
            execution_id: execution_id.clone(),
            workflow_id: workflow_id.clone(),
        }),
        WorkflowEvent::DiagnosticsSnapshot { .. } => None,
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
        error,
        ..
    } = event
    {
        state.runtime = DiagnosticsRuntimeSnapshot {
            workflow_id: Some(workflow_id.clone()),
            captured_at_ms: Some(timestamp_ms),
            max_input_bindings: capabilities.as_ref().map(|value| value.max_input_bindings),
            max_output_targets: capabilities.as_ref().map(|value| value.max_output_targets),
            max_value_bytes: capabilities.as_ref().map(|value| value.max_value_bytes),
            runtime_requirements: capabilities
                .as_ref()
                .map(|value| value.runtime_requirements.clone()),
            runtime_capabilities: capabilities
                .as_ref()
                .map(|value| value.runtime_capabilities.clone())
                .unwrap_or_default(),
            models: capabilities
                .as_ref()
                .map(|value| value.models.clone())
                .unwrap_or_default(),
            last_error: error.clone(),
        };
    }
}

fn apply_scheduler_event(
    state: &mut WorkflowDiagnosticsState,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    if let WorkflowEvent::SchedulerSnapshot {
        workflow_id,
        session_id,
        session,
        items,
        error,
        ..
    } = event
    {
        state.scheduler = DiagnosticsSchedulerSnapshot {
            workflow_id: workflow_id.clone(),
            session_id: Some(session_id.clone()),
            captured_at_ms: Some(timestamp_ms),
            session: session.clone(),
            items: items.clone(),
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use pantograph_workflow_service::graph::WorkflowDerivedGraph;
    use pantograph_workflow_service::graph::WorkflowSessionKind;

    fn sample_graph() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![pantograph_workflow_service::GraphNode {
                id: "llm-1".to_string(),
                node_type: "llm-inference".to_string(),
                position: pantograph_workflow_service::Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            }],
            edges: Vec::new(),
            derived_graph: Some(WorkflowDerivedGraph {
                schema_version: 1,
                graph_fingerprint: "graph-123".to_string(),
                consumer_count_map: HashMap::new(),
            }),
        }
    }

    #[test]
    fn record_workflow_event_tracks_run_and_node_timing() {
        let store = WorkflowDiagnosticsStore::default();
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Test Workflow".to_string()),
        );
        store.set_execution_graph("exec-1", &sample_graph());

        store.record_workflow_event(
            &WorkflowEvent::Started {
                workflow_id: "wf-1".to_string(),
                node_count: 1,
                execution_id: "exec-1".to_string(),
            },
            1_000,
        );
        store.record_workflow_event(
            &WorkflowEvent::NodeStarted {
                node_id: "llm-1".to_string(),
                node_type: String::new(),
                execution_id: "exec-1".to_string(),
            },
            1_010,
        );
        store.record_workflow_event(
            &WorkflowEvent::NodeCompleted {
                node_id: "llm-1".to_string(),
                outputs: HashMap::new(),
                execution_id: "exec-1".to_string(),
            },
            1_050,
        );
        let snapshot = store.record_workflow_event(
            &WorkflowEvent::Completed {
                workflow_id: "wf-1".to_string(),
                outputs: HashMap::new(),
                execution_id: "exec-1".to_string(),
            },
            1_100,
        );

        let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
        assert_eq!(run.workflow_name.as_deref(), Some("Test Workflow"));
        assert_eq!(run.graph_fingerprint_at_start.as_deref(), Some("graph-123"));
        assert_eq!(run.node_count_at_start, 1);
        assert_eq!(run.status, DiagnosticsRunStatus::Completed);
        assert_eq!(run.duration_ms, Some(100));
        assert_eq!(run.events.len(), 4);

        let node = run.nodes.get("llm-1").expect("node trace");
        assert_eq!(node.node_type.as_deref(), Some("llm-inference"));
        assert_eq!(node.status, DiagnosticsNodeStatus::Completed);
        assert_eq!(node.duration_ms, Some(40));
    }

    #[test]
    fn runtime_and_scheduler_snapshots_are_backend_owned() {
        let store = WorkflowDiagnosticsStore::default();
        store.update_runtime_snapshot(
            Some("wf-runtime".to_string()),
            Some(WorkflowCapabilitiesResponse {
                max_input_bindings: 4,
                max_output_targets: 2,
                max_value_bytes: 1000,
                runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: None,
                    estimation_confidence: "high".to_string(),
                    required_models: vec!["model-a".to_string()],
                    required_backends: vec!["llama-cpp".to_string()],
                    required_extensions: vec!["kv-cache".to_string()],
                },
                models: vec![pantograph_workflow_service::WorkflowCapabilityModel {
                    model_id: "model-a".to_string(),
                    model_revision_or_hash: None,
                    model_type: None,
                    node_ids: vec!["node-a".to_string()],
                    roles: vec!["generation".to_string()],
                }],
                runtime_capabilities: Vec::new(),
            }),
            None,
            5_000,
        );
        let snapshot = store.update_scheduler_snapshot(
            Some("wf-runtime".to_string()),
            Some("session-1".to_string()),
            Some(WorkflowSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-runtime".to_string(),
                session_kind: WorkflowSessionKind::Workflow,
                usage_profile: None,
                keep_alive: true,
                state: pantograph_workflow_service::WorkflowSessionState::Running,
                queued_runs: 1,
                run_count: 3,
            }),
            vec![WorkflowSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("run-1".to_string()),
                priority: 10,
                status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
            }],
            None,
            6_000,
        );

        assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-runtime"));
        assert_eq!(snapshot.runtime.max_input_bindings, Some(4));
        assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("session-1"));
        assert_eq!(
            snapshot
                .scheduler
                .session
                .as_ref()
                .map(|session| session.session_kind.clone()),
            Some(WorkflowSessionKind::Workflow)
        );
        assert_eq!(snapshot.scheduler.items.len(), 1);
    }

    #[test]
    fn clear_history_preserves_runtime_and_scheduler_snapshots() {
        let store = WorkflowDiagnosticsStore::default();
        store.record_workflow_event(
            &WorkflowEvent::Started {
                workflow_id: "wf-1".to_string(),
                node_count: 1,
                execution_id: "exec-1".to_string(),
            },
            1_000,
        );
        store.update_runtime_snapshot(Some("wf-1".to_string()), None, None, 2_000);
        store.update_scheduler_snapshot(
            Some("wf-1".to_string()),
            Some("exec-1".to_string()),
            None,
            Vec::new(),
            None,
            2_100,
        );

        let snapshot = store.clear_history();

        assert!(snapshot.runs_by_id.is_empty());
        assert!(snapshot.run_order.is_empty());
        assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("exec-1"));
    }

    #[test]
    fn trace_snapshot_filters_runs_without_projection_overlay_rules() {
        let store = WorkflowDiagnosticsStore::default();
        store.record_workflow_event(
            &WorkflowEvent::Started {
                workflow_id: "wf-1".to_string(),
                node_count: 1,
                execution_id: "exec-1".to_string(),
            },
            1_000,
        );
        store.record_workflow_event(
            &WorkflowEvent::Completed {
                workflow_id: "wf-1".to_string(),
                outputs: HashMap::new(),
                execution_id: "exec-1".to_string(),
            },
            1_100,
        );
        store.record_workflow_event(
            &WorkflowEvent::Started {
                workflow_id: "wf-2".to_string(),
                node_count: 1,
                execution_id: "exec-2".to_string(),
            },
            1_200,
        );

        let snapshot = store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: None,
                workflow_id: None,
                include_completed: Some(false),
            })
            .expect("trace snapshot");

        assert_eq!(snapshot.traces.len(), 1);
        assert_eq!(snapshot.traces[0].execution_id, "exec-2");
        assert_eq!(snapshot.traces[0].status, WorkflowTraceStatus::Running);
    }
}
