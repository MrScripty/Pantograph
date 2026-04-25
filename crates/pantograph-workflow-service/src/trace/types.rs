use std::collections::HashMap;

use node_engine::GraphMemoryImpactSummary;
use pantograph_diagnostics_ledger::WorkflowTimingExpectation;
use serde::{Deserialize, Serialize};

use crate::WorkflowSchedulerSnapshotDiagnostics;
use crate::workflow::{
    WorkflowCapabilitiesResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowServiceError,
};

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
    pub scheduler_admission_outcome: Option<String>,
    #[serde(default)]
    pub scheduler_decision_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheduler_snapshot_diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceRuntimeMetrics {
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub observed_runtime_ids: Vec<String>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_progress_detail: Option<node_engine::TaskProgressDetail>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<WorkflowTimingExpectation>,
}

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_dirty_tasks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_incremental_task_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_graph_memory_impact: Option<GraphMemoryImpactSummary>,
    #[serde(default)]
    pub waiting_for_input: bool,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub nodes: Vec<WorkflowTraceNodeRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<WorkflowTimingExpectation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowTraceGraphContext {
    pub graph_fingerprint: Option<String>,
    pub node_count_at_start: usize,
    pub node_types_by_id: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceGraphTimingExpectations {
    pub workflow_id: String,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub graph_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<WorkflowTimingExpectation>,
    pub nodes: Vec<WorkflowTraceNodeTimingExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceNodeTimingExpectation {
    pub node_id: String,
    #[serde(default)]
    pub node_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing_expectation: Option<WorkflowTimingExpectation>,
}

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
        detail: Option<node_engine::TaskProgressDetail>,
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
        dirty_tasks: Vec<String>,
        memory_impact: Option<GraphMemoryImpactSummary>,
    },
    IncrementalExecutionStarted {
        execution_id: String,
        workflow_id: Option<String>,
        task_ids: Vec<String>,
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
        session: Option<WorkflowExecutionSessionSummary>,
        items: Vec<WorkflowExecutionSessionQueueItem>,
        diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
        error: Option<String>,
    },
}

impl WorkflowTraceEvent {
    pub(crate) fn execution_id(&self) -> &str {
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

    pub(crate) fn workflow_id(&self) -> Option<&str> {
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

    pub(crate) fn node_id(&self) -> Option<&str> {
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

    pub(crate) fn node_type(&self) -> Option<&str> {
        match self {
            Self::NodeStarted { node_type, .. } => node_type.as_deref(),
            _ => None,
        }
    }

    pub(crate) fn node_count(&self) -> Option<usize> {
        match self {
            Self::RunStarted { node_count, .. } => Some(*node_count),
            _ => None,
        }
    }
}

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
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub include_completed: Option<bool>,
}

impl WorkflowTraceSnapshotRequest {
    pub fn normalized(&self) -> Self {
        Self {
            execution_id: normalize_optional_filter(&self.execution_id),
            session_id: normalize_optional_filter(&self.session_id),
            workflow_id: normalize_optional_filter(&self.workflow_id),
            workflow_name: normalize_optional_filter(&self.workflow_name),
            include_completed: self.include_completed,
        }
    }

    pub fn validate(&self) -> Result<(), WorkflowServiceError> {
        validate_optional_filter(&self.execution_id, "execution_id")?;
        validate_optional_filter(&self.session_id, "session_id")?;
        validate_optional_filter(&self.workflow_id, "workflow_id")?;
        validate_optional_filter(&self.workflow_name, "workflow_name")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowTraceSnapshotResponse {
    #[serde(default)]
    pub traces: Vec<WorkflowTraceSummary>,
    #[serde(default)]
    pub retained_trace_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkflowTraceRuntimeSelection {
    pub execution_id: Option<String>,
    pub runtime: Option<WorkflowTraceRuntimeMetrics>,
    pub matched_execution_ids: Vec<String>,
}

impl WorkflowTraceRuntimeSelection {
    pub fn is_ambiguous(&self) -> bool {
        self.execution_id.is_none() && self.matched_execution_ids.len() > 1
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

fn normalize_optional_filter(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).map(ToOwned::to_owned)
}
