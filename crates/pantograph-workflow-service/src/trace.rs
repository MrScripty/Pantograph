use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::workflow::{
    WorkflowCapabilitiesResponse, WorkflowServiceError, WorkflowSessionQueueItem,
    WorkflowSessionQueueItemStatus, WorkflowSessionState, WorkflowSessionSummary,
};

const DEFAULT_RETAINED_TRACE_LIMIT: usize = 200;

/// Canonical status for a workflow trace at the service boundary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTraceStatus {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

/// Canonical status for a node-level trace at the service boundary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTraceNodeStatus {
    Pending,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

/// Queue timing metrics attached to a workflow trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceQueueMetrics {
    #[serde(default)]
    pub enqueued_at_ms: Option<u64>,
    #[serde(default)]
    pub dequeued_at_ms: Option<u64>,
    #[serde(default)]
    pub queue_wait_ms: Option<u64>,
    #[serde(default)]
    pub scheduler_decision_reason: Option<String>,
}

/// Runtime lifecycle metrics attached to a workflow trace.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceRuntimeMetrics {
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
    #[serde(default)]
    pub model_target: Option<String>,
    #[serde(default)]
    pub warmup_started_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_completed_at_ms: Option<u64>,
    #[serde(default)]
    pub warmup_duration_ms: Option<u64>,
    #[serde(default)]
    pub runtime_reused: Option<bool>,
    #[serde(default)]
    pub lifecycle_decision_reason: Option<String>,
}

/// Backend-owned node timing record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceNodeRecord {
    pub node_id: String,
    #[serde(default)]
    pub node_type: Option<String>,
    pub status: WorkflowTraceNodeStatus,
    #[serde(default)]
    pub started_at_ms: Option<u64>,
    #[serde(default)]
    pub ended_at_ms: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub event_count: usize,
    #[serde(default)]
    pub stream_event_count: usize,
    #[serde(default)]
    pub last_error: Option<String>,
}

/// Backend-owned run/session trace summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceSummary {
    pub execution_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub graph_fingerprint: Option<String>,
    pub status: WorkflowTraceStatus,
    pub started_at_ms: u64,
    #[serde(default)]
    pub ended_at_ms: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub queue: WorkflowTraceQueueMetrics,
    #[serde(default)]
    pub runtime: WorkflowTraceRuntimeMetrics,
    #[serde(default)]
    pub node_count_at_start: usize,
    #[serde(default)]
    pub event_count: usize,
    #[serde(default)]
    pub stream_event_count: usize,
    #[serde(default)]
    pub waiting_for_input: bool,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub nodes: Vec<WorkflowTraceNodeRecord>,
}

/// Graph metadata captured at execution start so traces can preserve workflow
/// shape context without depending on adapter-owned projection state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowTraceGraphContext {
    pub graph_fingerprint: Option<String>,
    pub node_count_at_start: usize,
    pub node_types_by_id: HashMap<String, String>,
}

/// Canonical backend-owned trace event model consumed by trace readers/stores.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkflowTraceEvent {
    RunStarted {
        execution_id: String,
        workflow_id: Option<String>,
        node_count: usize,
    },
    NodeStarted {
        execution_id: String,
        node_id: String,
        node_type: Option<String>,
    },
    NodeProgress {
        execution_id: String,
        node_id: String,
    },
    NodeStream {
        execution_id: String,
        node_id: String,
    },
    NodeCompleted {
        execution_id: String,
        node_id: String,
    },
    NodeFailed {
        execution_id: String,
        node_id: String,
        error: String,
    },
    RunCompleted {
        execution_id: String,
        workflow_id: Option<String>,
    },
    RunFailed {
        execution_id: String,
        workflow_id: Option<String>,
        error: String,
    },
    RunCancelled {
        execution_id: String,
        workflow_id: Option<String>,
        error: String,
    },
    WaitingForInput {
        execution_id: String,
        workflow_id: Option<String>,
        node_id: String,
    },
    GraphModified {
        execution_id: String,
        workflow_id: Option<String>,
    },
    IncrementalExecutionStarted {
        execution_id: String,
        workflow_id: Option<String>,
    },
    RuntimeSnapshotCaptured {
        execution_id: String,
        workflow_id: Option<String>,
        captured_at_ms: u64,
        runtime: WorkflowTraceRuntimeMetrics,
        capabilities: Option<WorkflowCapabilitiesResponse>,
        error: Option<String>,
    },
    SchedulerSnapshotCaptured {
        execution_id: String,
        workflow_id: Option<String>,
        session_id: String,
        captured_at_ms: u64,
        session: Option<WorkflowSessionSummary>,
        items: Vec<WorkflowSessionQueueItem>,
        error: Option<String>,
    },
}

impl WorkflowTraceEvent {
    fn execution_id(&self) -> &str {
        match self {
            Self::RunStarted { execution_id, .. }
            | Self::NodeStarted { execution_id, .. }
            | Self::NodeProgress { execution_id, .. }
            | Self::NodeStream { execution_id, .. }
            | Self::NodeCompleted { execution_id, .. }
            | Self::NodeFailed { execution_id, .. }
            | Self::RunCompleted { execution_id, .. }
            | Self::RunFailed { execution_id, .. }
            | Self::RunCancelled { execution_id, .. }
            | Self::WaitingForInput { execution_id, .. }
            | Self::GraphModified { execution_id, .. }
            | Self::IncrementalExecutionStarted { execution_id, .. }
            | Self::RuntimeSnapshotCaptured { execution_id, .. }
            | Self::SchedulerSnapshotCaptured { execution_id, .. } => execution_id,
        }
    }

    fn workflow_id(&self) -> Option<&str> {
        match self {
            Self::RunStarted { workflow_id, .. }
            | Self::RunCompleted { workflow_id, .. }
            | Self::RunFailed { workflow_id, .. }
            | Self::RunCancelled { workflow_id, .. }
            | Self::WaitingForInput { workflow_id, .. }
            | Self::GraphModified { workflow_id, .. }
            | Self::IncrementalExecutionStarted { workflow_id, .. }
            | Self::RuntimeSnapshotCaptured { workflow_id, .. }
            | Self::SchedulerSnapshotCaptured { workflow_id, .. } => workflow_id.as_deref(),
            Self::NodeStarted { .. }
            | Self::NodeProgress { .. }
            | Self::NodeStream { .. }
            | Self::NodeCompleted { .. }
            | Self::NodeFailed { .. } => None,
        }
    }

    fn node_id(&self) -> Option<&str> {
        match self {
            Self::NodeStarted { node_id, .. }
            | Self::NodeProgress { node_id, .. }
            | Self::NodeStream { node_id, .. }
            | Self::NodeCompleted { node_id, .. }
            | Self::NodeFailed { node_id, .. }
            | Self::WaitingForInput { node_id, .. } => Some(node_id),
            Self::RunStarted { .. }
            | Self::RunCompleted { .. }
            | Self::RunFailed { .. }
            | Self::RunCancelled { .. }
            | Self::GraphModified { .. }
            | Self::IncrementalExecutionStarted { .. }
            | Self::RuntimeSnapshotCaptured { .. }
            | Self::SchedulerSnapshotCaptured { .. } => None,
        }
    }

    fn node_type(&self) -> Option<&str> {
        match self {
            Self::NodeStarted { node_type, .. } => node_type.as_deref(),
            _ => None,
        }
    }

    fn node_count(&self) -> Option<usize> {
        match self {
            Self::RunStarted { node_count, .. } => Some(*node_count),
            _ => None,
        }
    }
}

/// Debug/internal request surface for workflow trace snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceSnapshotRequest {
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub include_completed: Option<bool>,
}

impl WorkflowTraceSnapshotRequest {
    /// Validate optional snapshot filters at the service boundary before an
    /// adapter forwards the request into backend-owned trace readers.
    pub fn validate(&self) -> Result<(), WorkflowServiceError> {
        validate_optional_filter(&self.execution_id, "execution_id")?;
        validate_optional_filter(&self.session_id, "session_id")?;
        validate_optional_filter(&self.workflow_id, "workflow_id")?;
        Ok(())
    }
}

/// Debug/internal snapshot response for workflow traces.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceSnapshotResponse {
    #[serde(default)]
    pub traces: Vec<WorkflowTraceSummary>,
    #[serde(default)]
    pub retained_trace_limit: usize,
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
struct WorkflowTraceRunState {
    execution_id: String,
    session_id: Option<String>,
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    graph_fingerprint: Option<String>,
    status: WorkflowTraceStatus,
    started_at_ms: u64,
    ended_at_ms: Option<u64>,
    duration_ms: Option<u64>,
    queue: WorkflowTraceQueueMetrics,
    runtime: WorkflowTraceRuntimeMetrics,
    node_count_at_start: usize,
    event_count: usize,
    stream_event_count: usize,
    waiting_for_input: bool,
    last_error: Option<String>,
    nodes_by_id: BTreeMap<String, WorkflowTraceNodeRecord>,
}

impl WorkflowTraceRunState {
    fn snapshot(&self) -> WorkflowTraceSummary {
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
        let traces = self
            .trace_order
            .iter()
            .filter_map(|execution_id| self.traces_by_id.get(execution_id))
            .filter(|trace| trace_matches_request(trace, request))
            .map(WorkflowTraceRunState::snapshot)
            .collect();

        WorkflowTraceSnapshotResponse {
            traces,
            retained_trace_limit: self.retained_trace_limit,
        }
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

/// Backend-owned in-memory store for recent workflow traces.
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
        request.validate()?;
        Ok(self
            .state
            .lock()
            .expect("workflow trace lock poisoned")
            .snapshot(request))
    }

    pub fn snapshot_all(&self) -> WorkflowTraceSnapshotResponse {
        self.state
            .lock()
            .expect("workflow trace lock poisoned")
            .snapshot_all()
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
}

fn validate_optional_filter(
    value: &Option<String>,
    field_name: &'static str,
) -> Result<(), WorkflowServiceError> {
    if let Some(value) = value {
        if value.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow trace snapshot request field '{}' must not be blank",
                field_name
            )));
        }
    }

    Ok(())
}

fn trace_matches_request(
    trace: &WorkflowTraceRunState,
    request: &WorkflowTraceSnapshotRequest,
) -> bool {
    if let Some(execution_id) = request.execution_id.as_deref() {
        if trace.execution_id != execution_id {
            return false;
        }
    }
    if let Some(session_id) = request.session_id.as_deref() {
        if trace.session_id.as_deref() != Some(session_id) && trace.execution_id != session_id {
            return false;
        }
    }
    if let Some(workflow_id) = request.workflow_id.as_deref() {
        if trace.workflow_id.as_deref() != Some(workflow_id) {
            return false;
        }
    }
    if request.include_completed == Some(false)
        && matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
        )
    {
        return false;
    }

    true
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
            error,
            ..
        } => apply_scheduler_snapshot(
            trace,
            execution_id,
            session_id,
            session.as_ref(),
            items,
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

fn apply_runtime_snapshot(
    trace: &mut WorkflowTraceRunState,
    runtime: &WorkflowTraceRuntimeMetrics,
    capabilities: Option<&WorkflowCapabilitiesResponse>,
    error: Option<&str>,
    _captured_at_ms: u64,
) {
    merge_runtime_metrics(&mut trace.runtime, runtime);

    if let Some(capabilities) = capabilities {
        if trace.runtime.runtime_id.is_none() {
            trace.runtime.runtime_id = infer_runtime_id(capabilities);
        }
        if trace.runtime.lifecycle_decision_reason.is_none() {
            trace.runtime.lifecycle_decision_reason =
                Some(runtime_lifecycle_reason(capabilities).to_string());
        }
        return;
    }

    if error.is_some() && trace.runtime.lifecycle_decision_reason.is_none() {
        trace.runtime.lifecycle_decision_reason = Some("capabilities_snapshot_failed".to_string());
    }
}

fn merge_runtime_metrics(
    target: &mut WorkflowTraceRuntimeMetrics,
    source: &WorkflowTraceRuntimeMetrics,
) {
    if let Some(runtime_id) = source.runtime_id.clone() {
        target.runtime_id = Some(runtime_id);
    }
    if let Some(runtime_instance_id) = source.runtime_instance_id.clone() {
        target.runtime_instance_id = Some(runtime_instance_id);
    }
    if let Some(model_target) = source.model_target.clone() {
        target.model_target = Some(model_target);
    }
    if let Some(warmup_started_at_ms) = source.warmup_started_at_ms {
        target.warmup_started_at_ms = Some(warmup_started_at_ms);
    }
    if let Some(warmup_completed_at_ms) = source.warmup_completed_at_ms {
        target.warmup_completed_at_ms = Some(warmup_completed_at_ms);
    }
    if let Some(warmup_duration_ms) = source.warmup_duration_ms {
        target.warmup_duration_ms = Some(warmup_duration_ms);
    }
    if let Some(runtime_reused) = source.runtime_reused {
        target.runtime_reused = Some(runtime_reused);
    }
    if let Some(lifecycle_decision_reason) = source.lifecycle_decision_reason.clone() {
        target.lifecycle_decision_reason = Some(lifecycle_decision_reason);
    }
}

fn infer_runtime_id(capabilities: &WorkflowCapabilitiesResponse) -> Option<String> {
    if capabilities.runtime_requirements.required_backends.len() == 1 {
        return capabilities
            .runtime_requirements
            .required_backends
            .first()
            .cloned();
    }

    if capabilities.runtime_capabilities.len() == 1 {
        return capabilities
            .runtime_capabilities
            .first()
            .map(|capability| capability.runtime_id.clone());
    }

    None
}

fn runtime_lifecycle_reason(capabilities: &WorkflowCapabilitiesResponse) -> &'static str {
    if capabilities
        .runtime_capabilities
        .iter()
        .any(|capability| capability.available && capability.configured)
    {
        "configured_runtime_available"
    } else if !capabilities
        .runtime_requirements
        .required_backends
        .is_empty()
    {
        "runtime_requirements_reported"
    } else {
        "capabilities_snapshot_available"
    }
}

fn apply_scheduler_snapshot(
    trace: &mut WorkflowTraceRunState,
    execution_id: &str,
    session_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
    error: Option<&str>,
    captured_at_ms: u64,
) {
    if trace.session_id.is_none() {
        trace.session_id = Some(session_id.to_string());
    }

    if error.is_some() {
        trace.queue.scheduler_decision_reason = Some("scheduler_snapshot_failed".to_string());
        return;
    }

    let matched_item = matched_queue_item(execution_id, session_id, items);
    let pending_visible = matched_item
        .map(|item| item.status == WorkflowSessionQueueItemStatus::Pending)
        .unwrap_or_else(|| {
            session
                .map(|summary| summary.queued_runs > 0)
                .unwrap_or(false)
                || items
                    .iter()
                    .any(|item| item.status == WorkflowSessionQueueItemStatus::Pending)
        });
    let running_visible = matched_item
        .map(|item| item.status == WorkflowSessionQueueItemStatus::Running)
        .unwrap_or_else(|| {
            matches!(
                session.map(|summary| summary.state),
                Some(WorkflowSessionState::Running)
            ) || items
                .iter()
                .any(|item| item.status == WorkflowSessionQueueItemStatus::Running)
        });

    if pending_visible {
        if let Some(enqueued_at_ms) = matched_item.and_then(|item| item.enqueued_at_ms) {
            trace.queue.enqueued_at_ms.get_or_insert(enqueued_at_ms);
        } else {
            trace.queue.enqueued_at_ms.get_or_insert(captured_at_ms);
        }
        if !matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
        ) && !running_visible
        {
            trace.status = WorkflowTraceStatus::Queued;
        }
    }

    if running_visible {
        if let Some(enqueued_at_ms) = matched_item.and_then(|item| item.enqueued_at_ms) {
            trace.queue.enqueued_at_ms.get_or_insert(enqueued_at_ms);
        }
        if let Some(dequeued_at_ms) = matched_item.and_then(|item| item.dequeued_at_ms) {
            trace.queue.dequeued_at_ms.get_or_insert(dequeued_at_ms);
        } else {
            trace.queue.dequeued_at_ms.get_or_insert(captured_at_ms);
        }
        if !matches!(
            trace.status,
            WorkflowTraceStatus::Completed
                | WorkflowTraceStatus::Failed
                | WorkflowTraceStatus::Cancelled
                | WorkflowTraceStatus::Waiting
        ) {
            trace.status = WorkflowTraceStatus::Running;
        }
    }

    trace.queue.queue_wait_ms = match (trace.queue.enqueued_at_ms, trace.queue.dequeued_at_ms) {
        (Some(enqueued_at_ms), Some(dequeued_at_ms)) => {
            Some(dequeued_at_ms.saturating_sub(enqueued_at_ms))
        }
        _ => None,
    };
    trace.queue.scheduler_decision_reason =
        scheduler_decision_reason(execution_id, session_id, session, items);
}

fn scheduler_decision_reason(
    execution_id: &str,
    session_id: &str,
    session: Option<&WorkflowSessionSummary>,
    items: &[WorkflowSessionQueueItem],
) -> Option<String> {
    let matched_item = matched_queue_item(execution_id, session_id, items);
    let reason = if let Some(item) = matched_item {
        match item.status {
            WorkflowSessionQueueItemStatus::Pending => Some("matched_pending_item"),
            WorkflowSessionQueueItemStatus::Running => Some("matched_running_item"),
        }
    } else {
        let pending_visible = session
            .map(|summary| summary.queued_runs > 0)
            .unwrap_or(false)
            || items
                .iter()
                .any(|item| item.status == WorkflowSessionQueueItemStatus::Pending);
        let running_visible = matches!(
            session.map(|summary| summary.state),
            Some(WorkflowSessionState::Running)
        ) || items
            .iter()
            .any(|item| item.status == WorkflowSessionQueueItemStatus::Running);

        if running_visible && pending_visible {
            Some("session_running_with_backlog")
        } else if running_visible {
            Some("session_running")
        } else if pending_visible {
            Some("session_queued")
        } else {
            match session.map(|summary| summary.state) {
                Some(WorkflowSessionState::IdleLoaded) => Some("idle_loaded"),
                Some(WorkflowSessionState::IdleUnloaded) => Some("idle_unloaded"),
                Some(WorkflowSessionState::Running) | None => None,
            }
        }
    }?;

    Some(reason.to_string())
}

fn matched_queue_item<'a>(
    execution_id: &str,
    session_id: &str,
    items: &'a [WorkflowSessionQueueItem],
) -> Option<&'a WorkflowSessionQueueItem> {
    items
        .iter()
        .find(|item| item.run_id.as_deref() == Some(execution_id))
        .or_else(|| items.iter().find(|item| item.queue_id == execution_id))
        .or_else(|| {
            items
                .iter()
                .find(|item| item.run_id.as_deref() == Some(session_id))
        })
        .or_else(|| items.iter().find(|item| item.queue_id == session_id))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowServiceError;

    #[test]
    fn workflow_trace_summary_serializes_with_snake_case_contract() {
        let value = serde_json::to_value(WorkflowTraceSummary {
            execution_id: "exec-1".to_string(),
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            workflow_name: Some("Workflow".to_string()),
            graph_fingerprint: Some("graph-1".to_string()),
            status: WorkflowTraceStatus::Running,
            started_at_ms: 100,
            ended_at_ms: Some(200),
            duration_ms: Some(100),
            queue: WorkflowTraceQueueMetrics {
                enqueued_at_ms: Some(80),
                dequeued_at_ms: Some(100),
                queue_wait_ms: Some(20),
                scheduler_decision_reason: Some("warm_session_reused".to_string()),
            },
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:13b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("already_ready".to_string()),
            },
            node_count_at_start: 2,
            event_count: 3,
            stream_event_count: 1,
            waiting_for_input: false,
            last_error: None,
            nodes: vec![WorkflowTraceNodeRecord {
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
                status: WorkflowTraceNodeStatus::Completed,
                started_at_ms: Some(110),
                ended_at_ms: Some(180),
                duration_ms: Some(70),
                event_count: 2,
                stream_event_count: 1,
                last_error: None,
            }],
        })
        .expect("serialize trace summary");

        let expected = serde_json::json!({
            "execution_id": "exec-1",
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "workflow_name": "Workflow",
            "graph_fingerprint": "graph-1",
            "status": "running",
            "started_at_ms": 100,
            "ended_at_ms": 200,
            "duration_ms": 100,
            "queue": {
                "enqueued_at_ms": 80,
                "dequeued_at_ms": 100,
                "queue_wait_ms": 20,
                "scheduler_decision_reason": "warm_session_reused"
            },
            "runtime": {
                "runtime_id": "llama_cpp",
                "runtime_instance_id": "runtime-1",
                "model_target": "llava:13b",
                "warmup_started_at_ms": 90,
                "warmup_completed_at_ms": 99,
                "warmup_duration_ms": 9,
                "runtime_reused": true,
                "lifecycle_decision_reason": "already_ready"
            },
            "node_count_at_start": 2,
            "event_count": 3,
            "stream_event_count": 1,
            "waiting_for_input": false,
            "last_error": null,
            "nodes": [{
                "node_id": "node-1",
                "node_type": "llm-inference",
                "status": "completed",
                "started_at_ms": 110,
                "ended_at_ms": 180,
                "duration_ms": 70,
                "event_count": 2,
                "stream_event_count": 1,
                "last_error": null
            }]
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn workflow_trace_snapshot_request_serializes_optional_filters() {
        let request = WorkflowTraceSnapshotRequest {
            execution_id: Some("exec-1".to_string()),
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            include_completed: Some(true),
        };
        request.validate().expect("valid trace snapshot request");

        let value = serde_json::to_value(request).expect("serialize snapshot request");

        let expected = serde_json::json!({
            "execution_id": "exec-1",
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "include_completed": true
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn workflow_trace_snapshot_request_rejects_blank_filter_values() {
        let request = WorkflowTraceSnapshotRequest {
            execution_id: Some("   ".to_string()),
            session_id: None,
            workflow_id: None,
            include_completed: None,
        };

        let error = request
            .validate()
            .expect_err("blank execution_id should be rejected");
        assert!(
            matches!(
                error,
                WorkflowServiceError::InvalidRequest(ref message)
                    if message
                        == "workflow trace snapshot request field 'execution_id' must not be blank"
            ),
            "unexpected validation error: {:?}",
            error
        );
    }

    #[test]
    fn workflow_trace_store_records_run_and_node_timing() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Workflow".to_string()),
        );
        store.set_execution_graph_context(
            "exec-1",
            &WorkflowTraceGraphContext {
                graph_fingerprint: Some("graph-1".to_string()),
                node_count_at_start: 1,
                node_types_by_id: HashMap::from([(
                    "node-1".to_string(),
                    "llm-inference".to_string(),
                )]),
            },
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            1_000,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            1_010,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStream {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
            },
            1_030,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeCompleted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
            },
            1_050,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
            },
            1_100,
        );

        let trace = snapshot.traces.first().expect("trace summary");
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow"));
        assert_eq!(trace.graph_fingerprint.as_deref(), Some("graph-1"));
        assert_eq!(trace.status, WorkflowTraceStatus::Completed);
        assert_eq!(trace.duration_ms, Some(100));
        assert_eq!(trace.event_count, 5);
        assert_eq!(trace.stream_event_count, 1);

        let node = trace.nodes.first().expect("node summary");
        assert_eq!(node.node_type.as_deref(), Some("llm-inference"));
        assert_eq!(node.status, WorkflowTraceNodeStatus::Completed);
        assert_eq!(node.duration_ms, Some(40));
        assert_eq!(node.stream_event_count, 1);
    }

    #[test]
    fn workflow_trace_store_filters_completed_runs() {
        let store = WorkflowTraceStore::new(10);
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
            },
            150,
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-2".to_string(),
                workflow_id: Some("wf-2".to_string()),
                node_count: 0,
            },
            200,
        );

        let filtered = store
            .snapshot(&WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: None,
                workflow_id: None,
                include_completed: Some(false),
            })
            .expect("filtered snapshot");

        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].execution_id, "exec-2");
        assert_eq!(filtered.traces[0].status, WorkflowTraceStatus::Running);
    }

    #[test]
    fn workflow_trace_store_filters_by_session_id_when_execution_differs() {
        let store = WorkflowTraceStore::new(10);
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "run-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "run-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 105,
                session: Some(WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(105),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                error: None,
            },
            105,
        );

        let filtered = store
            .snapshot(&WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: None,
                include_completed: None,
            })
            .expect("session-filtered snapshot");

        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].execution_id, "run-1");
        assert_eq!(filtered.traces[0].session_id.as_deref(), Some("session-1"));
    }

    #[test]
    fn workflow_trace_store_enforces_retention_limit() {
        let store = WorkflowTraceStore::new(1);
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-2".to_string(),
                workflow_id: Some("wf-2".to_string()),
                node_count: 0,
            },
            200,
        );

        assert_eq!(snapshot.retained_trace_limit, 1);
        assert_eq!(snapshot.traces.len(), 1);
        assert_eq!(snapshot.traces[0].execution_id, "exec-2");
    }

    #[test]
    fn workflow_trace_store_records_queue_and_runtime_snapshot_metrics() {
        let store = WorkflowTraceStore::new(10);
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 90,
                session: Some(WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: WorkflowSessionState::IdleLoaded,
                    queued_runs: 1,
                    run_count: 2,
                }),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(80),
                    dequeued_at_ms: None,
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Pending,
                }],
                error: None,
            },
            90,
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                captured_at_ms: 110,
                runtime: WorkflowTraceRuntimeMetrics {
                    runtime_id: Some("llama_cpp".to_string()),
                    runtime_instance_id: Some("llama_cpp-1".to_string()),
                    model_target: Some("/models/demo.gguf".to_string()),
                    warmup_started_at_ms: Some(100),
                    warmup_completed_at_ms: Some(110),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                },
                capabilities: Some(WorkflowCapabilitiesResponse {
                    max_input_bindings: 4,
                    max_output_targets: 2,
                    max_value_bytes: 2_048,
                    runtime_requirements: crate::workflow::WorkflowRuntimeRequirements {
                        estimated_peak_vram_mb: None,
                        estimated_peak_ram_mb: None,
                        estimated_min_vram_mb: None,
                        estimated_min_ram_mb: None,
                        estimation_confidence: "high".to_string(),
                        required_models: vec!["model-a".to_string()],
                        required_backends: vec!["llama_cpp".to_string()],
                        required_extensions: vec!["kv-cache".to_string()],
                    },
                    models: Vec::new(),
                    runtime_capabilities: vec![crate::workflow::WorkflowRuntimeCapability {
                        runtime_id: "llama_cpp".to_string(),
                        display_name: "llama.cpp".to_string(),
                        install_state: crate::workflow::WorkflowRuntimeInstallState::Installed,
                        available: true,
                        configured: true,
                        can_install: false,
                        can_remove: false,
                        source_kind: crate::workflow::WorkflowRuntimeSourceKind::Managed,
                        selected: true,
                        supports_external_connection: true,
                        backend_keys: vec!["llama_cpp".to_string()],
                        missing_files: Vec::new(),
                        unavailable_reason: None,
                    }],
                }),
                error: None,
            },
            110,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 120,
                session: Some(WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 3,
                }),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(80),
                    dequeued_at_ms: Some(115),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                error: None,
            },
            120,
        );

        let trace = snapshot.traces.first().expect("trace summary");
        assert_eq!(trace.status, WorkflowTraceStatus::Running);
        assert_eq!(trace.queue.enqueued_at_ms, Some(80));
        assert_eq!(trace.queue.dequeued_at_ms, Some(115));
        assert_eq!(trace.queue.queue_wait_ms, Some(35));
        assert_eq!(
            trace.queue.scheduler_decision_reason.as_deref(),
            Some("matched_running_item")
        );
        assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            trace.runtime.runtime_instance_id.as_deref(),
            Some("llama_cpp-1")
        );
        assert_eq!(
            trace.runtime.model_target.as_deref(),
            Some("/models/demo.gguf")
        );
        assert_eq!(trace.runtime.warmup_started_at_ms, Some(100));
        assert_eq!(trace.runtime.warmup_completed_at_ms, Some(110));
        assert_eq!(trace.runtime.warmup_duration_ms, Some(10));
        assert_eq!(trace.runtime.runtime_reused, Some(false));
        assert_eq!(
            trace.runtime.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
    }

    #[test]
    fn workflow_trace_store_resets_attempt_state_when_run_restarts_after_failure() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Workflow".to_string()),
        );
        store.set_execution_graph_context(
            "exec-1",
            &WorkflowTraceGraphContext {
                graph_fingerprint: Some("graph-1".to_string()),
                node_count_at_start: 2,
                node_types_by_id: HashMap::from([
                    ("node-1".to_string(), "llm-inference".to_string()),
                    ("node-2".to_string(), "embedding".to_string()),
                ]),
            },
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            110,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeFailed {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                error: "boom".to_string(),
            },
            120,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                captured_at_ms: 125,
                runtime: WorkflowTraceRuntimeMetrics {
                    runtime_id: Some("llama_cpp".to_string()),
                    runtime_instance_id: Some("runtime-1".to_string()),
                    model_target: Some("/models/restarted.gguf".to_string()),
                    warmup_started_at_ms: Some(101),
                    warmup_completed_at_ms: Some(109),
                    warmup_duration_ms: Some(8),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("loaded_runtime".to_string()),
                },
                capabilities: None,
                error: None,
            },
            125,
        );
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 126,
                session: Some(WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: false,
                    state: WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(90),
                    dequeued_at_ms: Some(100),
                    priority: 0,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                error: None,
            },
            126,
        );
        store.record_event(
            &WorkflowTraceEvent::RunFailed {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                error: "boom".to_string(),
            },
            130,
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 2,
            },
            200,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-2".to_string(),
                node_type: None,
            },
            210,
        );

        let trace = snapshot.traces.first().expect("trace");
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow"));
        assert_eq!(trace.graph_fingerprint.as_deref(), Some("graph-1"));
        assert_eq!(trace.status, WorkflowTraceStatus::Running);
        assert_eq!(trace.started_at_ms, 200);
        assert_eq!(trace.ended_at_ms, None);
        assert_eq!(trace.duration_ms, None);
        assert_eq!(trace.last_error, None);
        assert_eq!(trace.node_count_at_start, 2);
        assert_eq!(trace.event_count, 2);
        assert_eq!(trace.stream_event_count, 0);
        assert_eq!(trace.queue, WorkflowTraceQueueMetrics::default());
        assert_eq!(trace.runtime, WorkflowTraceRuntimeMetrics::default());
        assert_eq!(trace.nodes.len(), 1);
        assert_eq!(trace.nodes[0].node_id, "node-2");
        assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Running);
    }

    #[test]
    fn workflow_trace_store_keeps_inflight_state_on_duplicate_run_started() {
        let store = WorkflowTraceStore::new(10);

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
            },
            110,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            120,
        );

        let trace = snapshot.traces.first().expect("trace");
        assert_eq!(trace.status, WorkflowTraceStatus::Running);
        assert_eq!(trace.started_at_ms, 100);
        assert_eq!(trace.node_count_at_start, 1);
        assert_eq!(trace.nodes.len(), 1);
        assert_eq!(trace.nodes[0].node_id, "node-1");
        assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Running);
        assert_eq!(trace.event_count, 3);
    }

    #[test]
    fn workflow_trace_store_records_cancelled_runs_and_marks_active_nodes_cancelled() {
        let store = WorkflowTraceStore::new(10);

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
            },
            110,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunCancelled {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                error: "workflow run cancelled during execution".to_string(),
            },
            140,
        );

        let trace = snapshot.traces.first().expect("trace");
        assert_eq!(trace.status, WorkflowTraceStatus::Cancelled);
        assert_eq!(trace.ended_at_ms, Some(140));
        assert_eq!(trace.duration_ms, Some(40));
        assert_eq!(
            trace.last_error.as_deref(),
            Some("workflow run cancelled during execution")
        );
        assert_eq!(trace.nodes.len(), 1);
        assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Cancelled);
        assert_eq!(trace.nodes[0].ended_at_ms, Some(140));
        assert_eq!(trace.nodes[0].duration_ms, Some(30));
    }

    #[test]
    fn workflow_trace_store_prefers_matching_queue_items_over_session_backlog() {
        let store = WorkflowTraceStore::new(10);
        let snapshot = store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-target".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 200,
                session: Some(WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: WorkflowSessionState::Running,
                    queued_runs: 2,
                    run_count: 3,
                }),
                items: vec![
                    WorkflowSessionQueueItem {
                        queue_id: "queue-other".to_string(),
                        run_id: Some("other-run".to_string()),
                        enqueued_at_ms: Some(100),
                        dequeued_at_ms: Some(150),
                        priority: 10,
                        status: WorkflowSessionQueueItemStatus::Running,
                    },
                    WorkflowSessionQueueItem {
                        queue_id: "queue-target".to_string(),
                        run_id: Some("exec-target".to_string()),
                        enqueued_at_ms: Some(180),
                        dequeued_at_ms: None,
                        priority: 5,
                        status: WorkflowSessionQueueItemStatus::Pending,
                    },
                ],
                error: None,
            },
            200,
        );

        let trace = snapshot.traces.first().expect("trace summary");
        assert_eq!(trace.status, WorkflowTraceStatus::Queued);
        assert_eq!(trace.queue.enqueued_at_ms, Some(180));
        assert_eq!(trace.queue.dequeued_at_ms, None);
        assert_eq!(trace.queue.queue_wait_ms, None);
        assert_eq!(
            trace.queue.scheduler_decision_reason.as_deref(),
            Some("matched_pending_item")
        );
    }

    #[test]
    fn workflow_trace_store_preserves_enqueue_time_when_first_snapshot_is_running() {
        let store = WorkflowTraceStore::new(10);
        let snapshot = store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "edit-session-1".to_string(),
                workflow_id: None,
                session_id: "edit-session-1".to_string(),
                captured_at_ms: 5_000,
                session: Some(WorkflowSessionSummary {
                    session_id: "edit-session-1".to_string(),
                    workflow_id: "edit-session-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Edit,
                    usage_profile: None,
                    keep_alive: false,
                    state: WorkflowSessionState::Running,
                    queued_runs: 1,
                    run_count: 2,
                }),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "edit-session-1".to_string(),
                    run_id: Some("edit-session-1".to_string()),
                    enqueued_at_ms: Some(4_750),
                    dequeued_at_ms: Some(4_750),
                    priority: 0,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                error: None,
            },
            5_000,
        );

        let trace = snapshot.traces.first().expect("trace summary");
        assert_eq!(trace.status, WorkflowTraceStatus::Running);
        assert_eq!(trace.queue.enqueued_at_ms, Some(4_750));
        assert_eq!(trace.queue.dequeued_at_ms, Some(4_750));
        assert_eq!(trace.queue.queue_wait_ms, Some(0));
        assert_eq!(
            trace.queue.scheduler_decision_reason.as_deref(),
            Some("matched_running_item")
        );
    }
}
