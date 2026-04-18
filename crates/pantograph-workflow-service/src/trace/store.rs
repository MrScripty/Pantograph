use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::workflow::WorkflowServiceError;

use super::query::{runtime_metrics_selection, snapshot_for_request};
use super::runtime::apply_runtime_snapshot;
use super::scheduler::apply_scheduler_snapshot;
use super::types::{
    WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceNodeRecord,
    WorkflowTraceNodeStatus, WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics,
    WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse, WorkflowTraceStatus,
    WorkflowTraceSummary,
};

const DEFAULT_RETAINED_TRACE_LIMIT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTraceRecordResult {
    pub snapshot: WorkflowTraceSnapshotResponse,
    pub recorded_at_ms: u64,
}

fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Default)]
struct WorkflowTraceExecutionContext {
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    graph_fingerprint: Option<String>,
    node_count_at_start: usize,
    node_types_by_id: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub(super) struct WorkflowTraceRunState {
    pub(super) execution_id: String,
    pub(super) session_id: Option<String>,
    pub(super) workflow_id: Option<String>,
    pub(super) workflow_name: Option<String>,
    pub(super) graph_fingerprint: Option<String>,
    pub(super) status: WorkflowTraceStatus,
    pub(super) started_at_ms: u64,
    pub(super) ended_at_ms: Option<u64>,
    pub(super) duration_ms: Option<u64>,
    pub(super) queue: WorkflowTraceQueueMetrics,
    pub(super) runtime: WorkflowTraceRuntimeMetrics,
    pub(super) node_count_at_start: usize,
    pub(super) event_count: usize,
    pub(super) stream_event_count: usize,
    pub(super) waiting_for_input: bool,
    pub(super) last_error: Option<String>,
    pub(super) nodes_by_id: BTreeMap<String, WorkflowTraceNodeRecord>,
}

impl WorkflowTraceRunState {
    pub(super) fn snapshot(&self) -> WorkflowTraceSummary {
        WorkflowTraceSummary {
            execution_id: self.execution_id.clone(),
            session_id: self.session_id.clone(),
            workflow_id: self.workflow_id.clone(),
            workflow_name: self.workflow_name.clone(),
            graph_fingerprint: self.graph_fingerprint.clone(),
            status: self.status,
            started_at_ms: self.started_at_ms,
            ended_at_ms: self.ended_at_ms,
            duration_ms: self.duration_ms,
            queue: self.queue.clone(),
            runtime: self.runtime.clone(),
            node_count_at_start: self.node_count_at_start,
            event_count: self.event_count,
            stream_event_count: self.stream_event_count,
            waiting_for_input: self.waiting_for_input,
            last_error: self.last_error.clone(),
            nodes: self.nodes_by_id.values().cloned().collect(),
        }
    }
}

#[derive(Debug, Clone)]
struct WorkflowTraceState {
    traces_by_id: BTreeMap<String, WorkflowTraceRunState>,
    trace_order: Vec<String>,
    execution_contexts: HashMap<String, WorkflowTraceExecutionContext>,
    retained_trace_limit: usize,
}

impl WorkflowTraceState {
    fn new(retained_trace_limit: usize) -> Self {
        Self {
            traces_by_id: BTreeMap::new(),
            trace_order: Vec::new(),
            execution_contexts: HashMap::new(),
            retained_trace_limit,
        }
    }

    fn snapshot(&self, request: &WorkflowTraceSnapshotRequest) -> WorkflowTraceSnapshotResponse {
        snapshot_for_request(
            self.trace_order
                .iter()
                .filter_map(|execution_id| self.traces_by_id.get(execution_id)),
            self.retained_trace_limit,
            request,
        )
    }

    fn runtime_metrics_selection(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> super::types::WorkflowTraceRuntimeSelection {
        runtime_metrics_selection(
            self.trace_order
                .iter()
                .filter_map(|execution_id| self.traces_by_id.get(execution_id)),
            request,
        )
    }

    fn snapshot_all(&self) -> WorkflowTraceSnapshotResponse {
        self.snapshot(&WorkflowTraceSnapshotRequest::default())
    }

    fn clear_history(&mut self) {
        self.traces_by_id.clear();
        self.trace_order.clear();
        self.execution_contexts.clear();
    }

    fn set_execution_metadata(
        &mut self,
        execution_id: &str,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) {
        let context = self
            .execution_contexts
            .entry(execution_id.to_string())
            .or_default();
        if let Some(workflow_id) = workflow_id {
            context.workflow_id = Some(workflow_id);
        }
        if let Some(workflow_name) = workflow_name {
            context.workflow_name = Some(workflow_name);
        }

        if let Some(trace) = self.traces_by_id.get_mut(execution_id) {
            if trace.workflow_id.is_none() {
                trace.workflow_id = context.workflow_id.clone();
            }
            if trace.workflow_name.is_none() {
                trace.workflow_name = context.workflow_name.clone();
            }
        }
    }

    fn set_execution_graph_context(
        &mut self,
        execution_id: &str,
        graph_context: &WorkflowTraceGraphContext,
    ) {
        let context = self
            .execution_contexts
            .entry(execution_id.to_string())
            .or_default();
        context.graph_fingerprint = graph_context.graph_fingerprint.clone();
        context.node_count_at_start = graph_context.node_count_at_start;
        context.node_types_by_id = graph_context.node_types_by_id.clone();

        if let Some(trace) = self.traces_by_id.get_mut(execution_id) {
            if trace.graph_fingerprint.is_none() {
                trace.graph_fingerprint = context.graph_fingerprint.clone();
            }
            if trace.node_count_at_start == 0 {
                trace.node_count_at_start = context.node_count_at_start;
            }
            for (node_id, node_type) in &context.node_types_by_id {
                if let Some(node) = trace.nodes_by_id.get_mut(node_id) {
                    if node.node_type.is_none() {
                        node.node_type = Some(node_type.clone());
                    }
                }
            }
        }
    }

    fn record_event(&mut self, event: &WorkflowTraceEvent, timestamp_ms: u64) {
        let execution_id = event.execution_id().to_string();
        let context = self
            .execution_contexts
            .get(&execution_id)
            .cloned()
            .unwrap_or_default();
        let workflow_id = event
            .workflow_id()
            .map(ToOwned::to_owned)
            .or_else(|| context.workflow_id.clone());
        let mut trace = self.traces_by_id.remove(&execution_id).unwrap_or_else(|| {
            create_trace_run_state(
                &execution_id,
                workflow_id.clone(),
                &context,
                timestamp_ms,
                event.node_count().unwrap_or(context.node_count_at_start),
            )
        });

        self.trace_order
            .retain(|candidate| candidate != &execution_id);
        self.trace_order.insert(0, execution_id.clone());

        if trace.workflow_id.is_none() {
            trace.workflow_id = workflow_id;
        }
        if trace.workflow_name.is_none() {
            trace.workflow_name = context.workflow_name.clone();
        }
        if trace.graph_fingerprint.is_none() {
            trace.graph_fingerprint = context.graph_fingerprint.clone();
        }
        if trace.node_count_at_start == 0 && context.node_count_at_start > 0 {
            trace.node_count_at_start = context.node_count_at_start;
        }

        apply_trace_event(&mut trace, &context, event, timestamp_ms);
        self.traces_by_id.insert(execution_id, trace);
        self.enforce_retention_limit();
    }

    fn enforce_retention_limit(&mut self) {
        while self.trace_order.len() > self.retained_trace_limit {
            let Some(removed_execution_id) = self.trace_order.pop() else {
                break;
            };
            self.traces_by_id.remove(&removed_execution_id);
            self.execution_contexts.remove(&removed_execution_id);
        }
    }
}

#[derive(Debug)]
pub struct WorkflowTraceStore {
    state: Mutex<WorkflowTraceState>,
}

impl Default for WorkflowTraceStore {
    fn default() -> Self {
        Self::new(DEFAULT_RETAINED_TRACE_LIMIT)
    }
}

impl WorkflowTraceStore {
    pub fn new(retained_trace_limit: usize) -> Self {
        Self {
            state: Mutex::new(WorkflowTraceState::new(retained_trace_limit)),
        }
    }

    pub fn snapshot(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> Result<WorkflowTraceSnapshotResponse, WorkflowServiceError> {
        let request = request.normalized();
        request.validate()?;
        Ok(self
            .state
            .lock()
            .expect("workflow trace lock poisoned")
            .snapshot(&request))
    }

    pub fn snapshot_all(&self) -> WorkflowTraceSnapshotResponse {
        self.state
            .lock()
            .expect("workflow trace lock poisoned")
            .snapshot_all()
    }

    pub fn select_runtime_metrics(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> Result<super::types::WorkflowTraceRuntimeSelection, WorkflowServiceError> {
        let request = request.normalized();
        request.validate()?;
        Ok(self
            .state
            .lock()
            .expect("workflow trace lock poisoned")
            .runtime_metrics_selection(&request))
    }

    pub fn clear_history(&self) -> WorkflowTraceSnapshotResponse {
        let mut state = self.state.lock().expect("workflow trace lock poisoned");
        state.clear_history();
        state.snapshot_all()
    }

    pub fn set_execution_metadata(
        &self,
        execution_id: &str,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) {
        self.state
            .lock()
            .expect("workflow trace lock poisoned")
            .set_execution_metadata(execution_id, workflow_id, workflow_name);
    }

    pub fn set_execution_graph_context(
        &self,
        execution_id: &str,
        graph_context: &WorkflowTraceGraphContext,
    ) {
        self.state
            .lock()
            .expect("workflow trace lock poisoned")
            .set_execution_graph_context(execution_id, graph_context);
    }

    pub fn record_event(
        &self,
        event: &WorkflowTraceEvent,
        timestamp_ms: u64,
    ) -> WorkflowTraceSnapshotResponse {
        let mut state = self.state.lock().expect("workflow trace lock poisoned");
        state.record_event(event, timestamp_ms);
        state.snapshot_all()
    }

    pub fn record_event_now(&self, event: &WorkflowTraceEvent) -> WorkflowTraceRecordResult {
        let recorded_at_ms = unix_timestamp_ms();
        WorkflowTraceRecordResult {
            snapshot: self.record_event(event, recorded_at_ms),
            recorded_at_ms,
        }
    }
}

fn create_trace_run_state(
    execution_id: &str,
    workflow_id: Option<String>,
    context: &WorkflowTraceExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) -> WorkflowTraceRunState {
    WorkflowTraceRunState {
        execution_id: execution_id.to_string(),
        session_id: None,
        workflow_id,
        workflow_name: context.workflow_name.clone(),
        graph_fingerprint: context.graph_fingerprint.clone(),
        status: WorkflowTraceStatus::Running,
        started_at_ms: timestamp_ms,
        ended_at_ms: None,
        duration_ms: None,
        queue: WorkflowTraceQueueMetrics::default(),
        runtime: WorkflowTraceRuntimeMetrics::default(),
        node_count_at_start,
        event_count: 0,
        stream_event_count: 0,
        waiting_for_input: false,
        last_error: None,
        nodes_by_id: BTreeMap::new(),
    }
}

fn apply_trace_event(
    trace: &mut WorkflowTraceRunState,
    context: &WorkflowTraceExecutionContext,
    event: &WorkflowTraceEvent,
    timestamp_ms: u64,
) {
    if is_idempotent_terminal_trace_event(trace, event) {
        return;
    }

    trace.event_count += 1;

    match event {
        WorkflowTraceEvent::RunStarted { node_count, .. } => {
            if trace_can_restart_attempt(trace) {
                reset_trace_for_restart(trace, context, timestamp_ms, *node_count);
            } else {
                trace.status = WorkflowTraceStatus::Running;
                trace.waiting_for_input = false;
                trace.last_error = None;
                trace.ended_at_ms = None;
                trace.duration_ms = None;
                trace.node_count_at_start = *node_count;
            }
        }
        WorkflowTraceEvent::NodeStarted { .. } if trace.status == WorkflowTraceStatus::Waiting => {
            trace.status = WorkflowTraceStatus::Running;
            trace.waiting_for_input = false;
        }
        WorkflowTraceEvent::NodeStarted { .. } => {}
        WorkflowTraceEvent::NodeStream { .. } => {
            trace.stream_event_count += 1;
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            trace.status = WorkflowTraceStatus::Waiting;
            trace.waiting_for_input = true;
        }
        WorkflowTraceEvent::RunCompleted { .. } => {
            trace.status = WorkflowTraceStatus::Completed;
            trace.waiting_for_input = false;
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
        }
        WorkflowTraceEvent::RunFailed { error, .. } => {
            trace.status = WorkflowTraceStatus::Failed;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
        }
        WorkflowTraceEvent::RunCancelled { error, .. } => {
            trace.status = WorkflowTraceStatus::Cancelled;
            trace.waiting_for_input = false;
            trace.last_error = Some(error.clone());
            trace.ended_at_ms = Some(timestamp_ms);
            trace.duration_ms = Some(timestamp_ms.saturating_sub(trace.started_at_ms));
            cancel_active_trace_nodes(trace, error, timestamp_ms);
        }
        WorkflowTraceEvent::RuntimeSnapshotCaptured {
            captured_at_ms,
            runtime,
            capabilities,
            error,
            ..
        } => apply_runtime_snapshot(
            trace,
            runtime,
            capabilities.as_ref(),
            error.as_deref(),
            *captured_at_ms,
        ),
        WorkflowTraceEvent::SchedulerSnapshotCaptured {
            execution_id,
            session_id,
            captured_at_ms,
            session,
            items,
            diagnostics,
            error,
            ..
        } => apply_scheduler_snapshot(
            trace,
            execution_id,
            session_id,
            session.as_ref(),
            items,
            diagnostics.as_ref(),
            error.as_deref(),
            *captured_at_ms,
        ),
        WorkflowTraceEvent::NodeProgress { .. }
        | WorkflowTraceEvent::NodeCompleted { .. }
        | WorkflowTraceEvent::NodeFailed { .. }
        | WorkflowTraceEvent::GraphModified { .. }
        | WorkflowTraceEvent::IncrementalExecutionStarted { .. } => {}
    }

    let Some(node_id) = event.node_id() else {
        return;
    };
    let explicit_node_type = event.node_type().map(ToOwned::to_owned);
    let node = trace
        .nodes_by_id
        .entry(node_id.to_string())
        .or_insert_with(|| {
            create_trace_node_record(
                node_id,
                explicit_node_type
                    .clone()
                    .or_else(|| context.node_types_by_id.get(node_id).cloned()),
            )
        });
    if node.node_type.is_none() {
        node.node_type =
            explicit_node_type.or_else(|| context.node_types_by_id.get(node_id).cloned());
    }
    node.event_count += 1;

    match event {
        WorkflowTraceEvent::NodeStarted { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.started_at_ms.get_or_insert(timestamp_ms);
            node.ended_at_ms = None;
            node.duration_ms = None;
            node.last_error = None;
        }
        WorkflowTraceEvent::NodeProgress { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
        }
        WorkflowTraceEvent::NodeStream { .. } => {
            node.status = WorkflowTraceNodeStatus::Running;
            node.stream_event_count += 1;
        }
        WorkflowTraceEvent::NodeCompleted { .. } => {
            node.status = WorkflowTraceNodeStatus::Completed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            node.last_error = None;
        }
        WorkflowTraceEvent::NodeFailed { error, .. } => {
            node.status = WorkflowTraceNodeStatus::Failed;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            node.last_error = Some(error.clone());
        }
        WorkflowTraceEvent::WaitingForInput { .. } => {
            node.status = WorkflowTraceNodeStatus::Waiting;
        }
        WorkflowTraceEvent::RunStarted { .. }
        | WorkflowTraceEvent::RunCompleted { .. }
        | WorkflowTraceEvent::RunFailed { .. }
        | WorkflowTraceEvent::RunCancelled { .. }
        | WorkflowTraceEvent::GraphModified { .. }
        | WorkflowTraceEvent::IncrementalExecutionStarted { .. }
        | WorkflowTraceEvent::RuntimeSnapshotCaptured { .. }
        | WorkflowTraceEvent::SchedulerSnapshotCaptured { .. } => {}
    }
}

fn is_idempotent_terminal_trace_event(
    trace: &WorkflowTraceRunState,
    event: &WorkflowTraceEvent,
) -> bool {
    match event {
        WorkflowTraceEvent::RunCompleted { .. } => {
            trace.status == WorkflowTraceStatus::Completed && trace.ended_at_ms.is_some()
        }
        WorkflowTraceEvent::RunFailed { error, .. } => {
            trace.status == WorkflowTraceStatus::Failed
                && trace.ended_at_ms.is_some()
                && trace.last_error.as_deref() == Some(error.as_str())
        }
        WorkflowTraceEvent::RunCancelled { error, .. } => {
            trace.status == WorkflowTraceStatus::Cancelled
                && trace.ended_at_ms.is_some()
                && trace.last_error.as_deref() == Some(error.as_str())
        }
        WorkflowTraceEvent::NodeCompleted { node_id, .. } => {
            trace.nodes_by_id.get(node_id).is_some_and(|node| {
                node.status == WorkflowTraceNodeStatus::Completed && node.ended_at_ms.is_some()
            })
        }
        WorkflowTraceEvent::NodeFailed { node_id, error, .. } => {
            trace.nodes_by_id.get(node_id).is_some_and(|node| {
                node.status == WorkflowTraceNodeStatus::Failed
                    && node.ended_at_ms.is_some()
                    && node.last_error.as_deref() == Some(error.as_str())
            })
        }
        _ => false,
    }
}

fn cancel_active_trace_nodes(trace: &mut WorkflowTraceRunState, error: &str, timestamp_ms: u64) {
    for node in trace.nodes_by_id.values_mut() {
        if matches!(
            node.status,
            WorkflowTraceNodeStatus::Running | WorkflowTraceNodeStatus::Waiting
        ) {
            node.status = WorkflowTraceNodeStatus::Cancelled;
            node.ended_at_ms = Some(timestamp_ms);
            node.duration_ms = node
                .started_at_ms
                .map(|started_at_ms| timestamp_ms.saturating_sub(started_at_ms));
            if node.last_error.is_none() {
                node.last_error = Some(error.to_string());
            }
        }
    }
}

fn trace_can_restart_attempt(trace: &WorkflowTraceRunState) -> bool {
    trace.ended_at_ms.is_some()
        || matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
        )
}

fn reset_trace_for_restart(
    trace: &mut WorkflowTraceRunState,
    context: &WorkflowTraceExecutionContext,
    timestamp_ms: u64,
    node_count_at_start: usize,
) {
    trace.workflow_name = context.workflow_name.clone();
    trace.graph_fingerprint = context.graph_fingerprint.clone();
    trace.status = WorkflowTraceStatus::Running;
    trace.started_at_ms = timestamp_ms;
    trace.ended_at_ms = None;
    trace.duration_ms = None;
    trace.queue = WorkflowTraceQueueMetrics::default();
    trace.runtime = WorkflowTraceRuntimeMetrics::default();
    trace.node_count_at_start = node_count_at_start;
    trace.event_count = 1;
    trace.stream_event_count = 0;
    trace.waiting_for_input = false;
    trace.last_error = None;
    trace.nodes_by_id.clear();
}

fn create_trace_node_record(node_id: &str, node_type: Option<String>) -> WorkflowTraceNodeRecord {
    WorkflowTraceNodeRecord {
        node_id: node_id.to_string(),
        node_type,
        status: WorkflowTraceNodeStatus::Running,
        started_at_ms: None,
        ended_at_ms: None,
        duration_ms: None,
        event_count: 0,
        stream_event_count: 0,
        last_error: None,
    }
}
