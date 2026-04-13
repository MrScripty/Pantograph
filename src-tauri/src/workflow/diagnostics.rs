//! Backend-owned diagnostics projection for workflow execution traces.
//!
//! This module replaces the previous TypeScript-side diagnostics accumulator so
//! workflow run state, node lifecycle state, and retained event history are all
//! derived in Rust before the GUI renders them.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowGraph, WorkflowSessionQueueItem, WorkflowSessionSummary,
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
struct DiagnosticsExecutionContext {
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    graph_fingerprint: Option<String>,
    node_count_at_start: usize,
    node_types_by_id: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct WorkflowDiagnosticsState {
    runs_by_id: BTreeMap<String, DiagnosticsRunTrace>,
    run_order: Vec<String>,
    runtime: DiagnosticsRuntimeSnapshot,
    scheduler: DiagnosticsSchedulerSnapshot,
    retained_event_limit: usize,
    execution_contexts: HashMap<String, DiagnosticsExecutionContext>,
}

impl WorkflowDiagnosticsState {
    fn new(retained_event_limit: usize) -> Self {
        Self {
            runs_by_id: BTreeMap::new(),
            run_order: Vec::new(),
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
            execution_contexts: HashMap::new(),
        }
    }

    fn snapshot(&self) -> WorkflowDiagnosticsProjection {
        WorkflowDiagnosticsProjection {
            runs_by_id: self.runs_by_id.clone(),
            run_order: self.run_order.clone(),
            runtime: self.runtime.clone(),
            scheduler: self.scheduler.clone(),
            retained_event_limit: self.retained_event_limit,
        }
    }
}

#[derive(Debug)]
pub struct WorkflowDiagnosticsStore {
    state: Mutex<WorkflowDiagnosticsState>,
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
        }
    }

    pub fn snapshot(&self) -> WorkflowDiagnosticsProjection {
        self.state
            .lock()
            .expect("workflow diagnostics lock poisoned")
            .snapshot()
    }

    pub fn clear_history(&self) -> WorkflowDiagnosticsProjection {
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        state.runs_by_id.clear();
        state.run_order.clear();
        state.snapshot()
    }

    pub fn set_execution_metadata(
        &self,
        execution_id: &str,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) {
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        let context = state
            .execution_contexts
            .entry(execution_id.to_string())
            .or_default();
        if let Some(workflow_id) = workflow_id {
            context.workflow_id = Some(workflow_id);
        }
        if let Some(workflow_name) = workflow_name {
            context.workflow_name = Some(workflow_name);
        }

        let workflow_id = context.workflow_id.clone();
        let workflow_name = context.workflow_name.clone();
        if let Some(run) = state.runs_by_id.get_mut(execution_id) {
            if run.workflow_id.is_none() {
                run.workflow_id = workflow_id;
            }
            if run.workflow_name.is_none() {
                run.workflow_name = workflow_name;
            }
        }
    }

    pub fn set_execution_graph(&self, execution_id: &str, graph: &WorkflowGraph) {
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        let context = state
            .execution_contexts
            .entry(execution_id.to_string())
            .or_default();
        context.graph_fingerprint = graph
            .derived_graph
            .as_ref()
            .map(|derived| derived.graph_fingerprint.clone());
        context.node_count_at_start = graph.nodes.len();
        context.node_types_by_id = graph
            .nodes
            .iter()
            .map(|node| (node.id.clone(), node.node_type.clone()))
            .collect();

        let graph_fingerprint = context.graph_fingerprint.clone();
        let node_count_at_start = context.node_count_at_start;
        let node_types_by_id = context.node_types_by_id.clone();
        if let Some(run) = state.runs_by_id.get_mut(execution_id) {
            if run.graph_fingerprint_at_start.is_none() {
                run.graph_fingerprint_at_start = graph_fingerprint;
            }
            if run.node_count_at_start == 0 {
                run.node_count_at_start = node_count_at_start;
            }
            for (node_id, node_type) in &node_types_by_id {
                if let Some(node) = run.nodes.get_mut(node_id) {
                    if node.node_type.is_none() {
                        node.node_type = Some(node_type.clone());
                    }
                }
            }
        }
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
        state.snapshot()
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
        state.snapshot()
    }

    pub fn record_workflow_event(
        &self,
        event: &WorkflowEvent,
        timestamp_ms: u64,
    ) -> WorkflowDiagnosticsProjection {
        let mut state = self
            .state
            .lock()
            .expect("workflow diagnostics lock poisoned");
        record_workflow_event(&mut state, event, timestamp_ms);
        state.snapshot()
    }
}

pub type SharedWorkflowDiagnosticsStore = Arc<WorkflowDiagnosticsStore>;

fn record_workflow_event(
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

    let context = state
        .execution_contexts
        .get(&execution_id)
        .cloned()
        .unwrap_or_default();
    let workflow_id = event_workflow_id(event).or_else(|| context.workflow_id.clone());
    let existing_run = state.runs_by_id.remove(&execution_id);
    let mut run = existing_run.unwrap_or_else(|| {
        create_run_trace(
            &execution_id,
            workflow_id.clone(),
            &context,
            timestamp_ms,
            event_started_node_count(event).unwrap_or(context.node_count_at_start),
        )
    });

    state.run_order.retain(|run_id| run_id != &execution_id);
    state.run_order.insert(0, execution_id.clone());

    if run.workflow_id.is_none() {
        run.workflow_id = workflow_id.clone();
    }
    if run.workflow_name.is_none() {
        run.workflow_name = context.workflow_name.clone();
    }
    if run.graph_fingerprint_at_start.is_none() {
        run.graph_fingerprint_at_start = context.graph_fingerprint.clone();
    }
    if run.node_count_at_start == 0 && context.node_count_at_start > 0 {
        run.node_count_at_start = context.node_count_at_start;
    }

    apply_run_lifecycle(&mut run, event, timestamp_ms);
    apply_node_lifecycle(&mut run, &context, event, timestamp_ms);

    let event_record = DiagnosticsEventRecord {
        id: format!("{}-{}", execution_id, run.event_count),
        sequence: run.event_count,
        timestamp_ms,
        event_type: event_type_name(event).to_string(),
        execution_id: execution_id.clone(),
        workflow_id,
        node_id: event_node_id(event),
        summary: summarize_event(event),
        payload: event_payload(event),
    };
    run.events.push(event_record);
    if run.events.len() > state.retained_event_limit {
        let excess = run.events.len() - state.retained_event_limit;
        run.events.drain(0..excess);
    }

    state.runs_by_id.insert(execution_id, run);
}

fn create_run_trace(
    execution_id: &str,
    workflow_id: Option<String>,
    context: &DiagnosticsExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) -> DiagnosticsRunTrace {
    DiagnosticsRunTrace {
        execution_id: execution_id.to_string(),
        workflow_id,
        workflow_name: context.workflow_name.clone(),
        graph_fingerprint_at_start: context.graph_fingerprint.clone(),
        node_count_at_start,
        status: DiagnosticsRunStatus::Running,
        started_at_ms: timestamp_ms,
        ended_at_ms: None,
        duration_ms: None,
        last_updated_at_ms: timestamp_ms,
        error: None,
        waiting_for_input: false,
        event_count: 0,
        stream_event_count: 0,
        last_dirty_tasks: Vec::new(),
        last_incremental_task_ids: Vec::new(),
        nodes: BTreeMap::new(),
        events: Vec::new(),
    }
}

fn create_node_trace(node_id: &str, node_type: Option<String>) -> DiagnosticsNodeTrace {
    DiagnosticsNodeTrace {
        node_id: node_id.to_string(),
        node_type,
        status: DiagnosticsNodeStatus::Running,
        started_at_ms: None,
        ended_at_ms: None,
        duration_ms: None,
        last_progress: None,
        last_message: None,
        stream_event_count: 0,
        event_count: 0,
        error: None,
    }
}

fn apply_run_lifecycle(run: &mut DiagnosticsRunTrace, event: &WorkflowEvent, timestamp_ms: u64) {
    run.last_updated_at_ms = timestamp_ms;
    run.event_count += 1;

    match event {
        WorkflowEvent::Started { .. } => {
            run.status = DiagnosticsRunStatus::Running;
            run.waiting_for_input = false;
            run.error = None;
            run.ended_at_ms = None;
            run.duration_ms = None;
        }
        WorkflowEvent::NodeStream { .. } => {
            run.stream_event_count += 1;
        }
        WorkflowEvent::WaitingForInput { .. } => {
            run.status = DiagnosticsRunStatus::Waiting;
            run.waiting_for_input = true;
        }
        WorkflowEvent::Completed { .. } => {
            run.status = DiagnosticsRunStatus::Completed;
            run.waiting_for_input = false;
            run.ended_at_ms = Some(timestamp_ms);
            run.duration_ms = Some(timestamp_ms.saturating_sub(run.started_at_ms));
        }
        WorkflowEvent::Failed { error, .. } => {
            run.status = DiagnosticsRunStatus::Failed;
            run.waiting_for_input = false;
            run.error = Some(error.clone());
            run.ended_at_ms = Some(timestamp_ms);
            run.duration_ms = Some(timestamp_ms.saturating_sub(run.started_at_ms));
        }
        WorkflowEvent::GraphModified { dirty_tasks, .. } => {
            run.last_dirty_tasks = dirty_tasks.clone();
        }
        WorkflowEvent::IncrementalExecutionStarted { task_ids, .. } => {
            run.last_incremental_task_ids = task_ids.clone();
        }
        WorkflowEvent::NodeStarted { .. } if run.status == DiagnosticsRunStatus::Waiting => {
            run.status = DiagnosticsRunStatus::Running;
            run.waiting_for_input = false;
        }
        _ => {}
    }
}

fn apply_node_lifecycle(
    run: &mut DiagnosticsRunTrace,
    context: &DiagnosticsExecutionContext,
    event: &WorkflowEvent,
    timestamp_ms: u64,
) {
    let Some(node_id) = event_node_id(event) else {
        return;
    };
    let explicit_node_type = event_node_type(event);
    let node = run.nodes.entry(node_id.clone()).or_insert_with(|| {
        create_node_trace(
            &node_id,
            explicit_node_type
                .clone()
                .or_else(|| context.node_types_by_id.get(&node_id).cloned()),
        )
    });
    if node.node_type.is_none() {
        node.node_type =
            explicit_node_type.or_else(|| context.node_types_by_id.get(&node_id).cloned());
    }
    node.event_count += 1;

    match event {
        WorkflowEvent::NodeStarted { .. } => {
            node.status = DiagnosticsNodeStatus::Running;
            node.started_at_ms.get_or_insert(timestamp_ms);
            node.ended_at_ms = None;
            node.duration_ms = None;
            node.error = None;
            node.last_message = None;
            node.last_progress = None;
        }
        WorkflowEvent::NodeProgress {
            progress, message, ..
        } => {
            node.status = DiagnosticsNodeStatus::Running;
            node.last_progress = Some(*progress);
            node.last_message = message.clone();
        }
        WorkflowEvent::NodeStream { .. } => {
            node.status = DiagnosticsNodeStatus::Running;
            node.stream_event_count += 1;
        }
        WorkflowEvent::NodeCompleted { .. } => {
            node.status = DiagnosticsNodeStatus::Completed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started| timestamp_ms.saturating_sub(started));
            node.error = None;
        }
        WorkflowEvent::NodeError { error, .. } => {
            node.status = DiagnosticsNodeStatus::Failed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started| timestamp_ms.saturating_sub(started));
            node.error = Some(error.clone());
        }
        WorkflowEvent::WaitingForInput { message, .. } => {
            node.status = DiagnosticsNodeStatus::Waiting;
            node.last_message = message
                .clone()
                .or_else(|| Some("Waiting for input".to_string()));
        }
        _ => {}
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

fn event_node_type(event: &WorkflowEvent) -> Option<String> {
    match event {
        WorkflowEvent::NodeStarted { node_type, .. } if !node_type.trim().is_empty() => {
            Some(node_type.clone())
        }
        _ => None,
    }
}

fn event_started_node_count(event: &WorkflowEvent) -> Option<usize> {
    match event {
        WorkflowEvent::Started { node_count, .. } => Some(*node_count),
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
}
