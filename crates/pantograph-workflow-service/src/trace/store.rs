use std::collections::{BTreeMap, HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

use pantograph_diagnostics_ledger::SqliteDiagnosticsLedger;
use parking_lot::Mutex;

use crate::workflow::WorkflowServiceError;

use super::query::{runtime_metrics_selection, snapshot_for_request};
use super::state::{apply_trace_event, create_trace_run_state};
use super::timing::{
    enrich_snapshot_timing, graph_timing_expectations, terminal_timing_observations,
};
use super::types::{
    WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceGraphTimingExpectations,
    WorkflowTraceNodeRecord, WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics,
    WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse, WorkflowTraceStatus,
    WorkflowTraceSummary,
};
use node_engine::GraphMemoryImpactSummary;

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
pub(super) struct WorkflowTraceExecutionContext {
    pub(super) workflow_id: Option<String>,
    pub(super) workflow_name: Option<String>,
    pub(super) graph_fingerprint: Option<String>,
    pub(super) node_count_at_start: usize,
    pub(super) node_types_by_id: HashMap<String, String>,
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
    pub(super) last_dirty_tasks: Vec<String>,
    pub(super) last_incremental_task_ids: Vec<String>,
    pub(super) last_graph_memory_impact: Option<GraphMemoryImpactSummary>,
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
            last_dirty_tasks: self.last_dirty_tasks.clone(),
            last_incremental_task_ids: self.last_incremental_task_ids.clone(),
            last_graph_memory_impact: self.last_graph_memory_impact.clone(),
            waiting_for_input: self.waiting_for_input,
            last_error: self.last_error.clone(),
            nodes: self.nodes_by_id.values().cloned().collect(),
            timing_expectation: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct WorkflowTraceState {
    pub(super) traces_by_id: BTreeMap<String, WorkflowTraceRunState>,
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
            if context.workflow_id.is_some() {
                trace.workflow_id = context.workflow_id.clone();
            }
            if context.workflow_name.is_some() {
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

pub struct WorkflowTraceStore {
    state: Mutex<WorkflowTraceState>,
    timing_ledger: Option<Mutex<SqliteDiagnosticsLedger>>,
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
            timing_ledger: None,
        }
    }

    pub fn with_timing_ledger(
        retained_trace_limit: usize,
        timing_ledger: SqliteDiagnosticsLedger,
    ) -> Self {
        Self {
            state: Mutex::new(WorkflowTraceState::new(retained_trace_limit)),
            timing_ledger: Some(Mutex::new(timing_ledger)),
        }
    }

    pub fn snapshot(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> Result<WorkflowTraceSnapshotResponse, WorkflowServiceError> {
        let request = request.normalized();
        request.validate()?;
        let snapshot = self.state.lock().snapshot(&request);
        Ok(self.enrich_timing(snapshot))
    }

    pub fn snapshot_all(&self) -> WorkflowTraceSnapshotResponse {
        let snapshot = self.state.lock().snapshot_all();
        self.enrich_timing(snapshot)
    }

    pub fn select_runtime_metrics(
        &self,
        request: &WorkflowTraceSnapshotRequest,
    ) -> Result<super::types::WorkflowTraceRuntimeSelection, WorkflowServiceError> {
        let request = request.normalized();
        request.validate()?;
        Ok(self.state.lock().runtime_metrics_selection(&request))
    }

    pub fn clear_history(&self) -> WorkflowTraceSnapshotResponse {
        let mut state = self.state.lock();
        state.clear_history();
        let snapshot = state.snapshot_all();
        drop(state);
        self.enrich_timing(snapshot)
    }

    pub fn set_execution_metadata(
        &self,
        execution_id: &str,
        workflow_id: Option<String>,
        workflow_name: Option<String>,
    ) {
        self.state
            .lock()
            .set_execution_metadata(execution_id, workflow_id, workflow_name);
    }

    pub fn set_execution_graph_context(
        &self,
        execution_id: &str,
        graph_context: &WorkflowTraceGraphContext,
    ) {
        self.state
            .lock()
            .set_execution_graph_context(execution_id, graph_context);
    }

    pub fn graph_timing_expectations(
        &self,
        workflow_id: String,
        workflow_name: Option<String>,
        graph_context: &WorkflowTraceGraphContext,
    ) -> WorkflowTraceGraphTimingExpectations {
        let ledger = self.timing_ledger.as_ref().map(|ledger| ledger.lock());
        graph_timing_expectations(workflow_id, workflow_name, graph_context, ledger.as_deref())
    }

    pub fn record_event(
        &self,
        event: &WorkflowTraceEvent,
        timestamp_ms: u64,
    ) -> WorkflowTraceSnapshotResponse {
        let (snapshot, observations) = {
            let mut state = self.state.lock();
            state.record_event(event, timestamp_ms);
            (
                state.snapshot_all(),
                terminal_timing_observations(&state, event, timestamp_ms),
            )
        };
        self.record_timing_observations(observations);
        let snapshot = self.enrich_timing(snapshot);
        snapshot
    }

    pub fn record_event_now(&self, event: &WorkflowTraceEvent) -> WorkflowTraceRecordResult {
        let recorded_at_ms = unix_timestamp_ms();
        WorkflowTraceRecordResult {
            snapshot: self.record_event(event, recorded_at_ms),
            recorded_at_ms,
        }
    }

    fn record_timing_observations(
        &self,
        observations: Vec<pantograph_diagnostics_ledger::WorkflowTimingObservation>,
    ) {
        if observations.is_empty() {
            return;
        }
        let Some(ledger) = self.timing_ledger.as_ref() else {
            return;
        };
        let mut ledger = ledger.lock();
        for observation in observations {
            let _ = pantograph_diagnostics_ledger::DiagnosticsLedgerRepository::record_timing_observation(
                &mut *ledger,
                observation,
            );
        }
    }

    fn enrich_timing(
        &self,
        snapshot: WorkflowTraceSnapshotResponse,
    ) -> WorkflowTraceSnapshotResponse {
        let Some(ledger) = self.timing_ledger.as_ref() else {
            return snapshot;
        };
        let ledger = ledger.lock();
        enrich_snapshot_timing(snapshot, &*ledger, unix_timestamp_ms())
    }
}
