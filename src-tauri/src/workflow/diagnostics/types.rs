use std::collections::BTreeMap;

use node_engine::GraphMemoryImpactSummary;
use pantograph_embedded_runtime::workflow_runtime::{
    capability_runtime_lifecycle_snapshot, normalized_runtime_lifecycle_snapshot,
};
use pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView;
use pantograph_workflow_service::{
    graph::WorkflowGraphSessionStateView, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionSummary, WorkflowSchedulerSnapshotDiagnostics, WorkflowServiceError,
    WorkflowTraceNodeStatus, WorkflowTraceRuntimeMetrics, WorkflowTraceStatus,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsRunStatus {
    Running,
    Waiting,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticsNodeStatus {
    Running,
    Waiting,
    Completed,
    Cancelled,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_progress_detail: Option<node_engine::TaskProgressDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRunTrace {
    pub execution_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
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
    pub runtime: DiagnosticsTraceRuntimeMetrics,
    pub event_count: usize,
    pub stream_event_count: usize,
    pub last_dirty_tasks: Vec<String>,
    pub last_incremental_task_ids: Vec<String>,
    #[serde(default)]
    pub last_graph_memory_impact: Option<GraphMemoryImpactSummary>,
    pub nodes: BTreeMap<String, DiagnosticsNodeTrace>,
    pub events: Vec<DiagnosticsEventRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsTraceRuntimeMetrics {
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

impl From<&WorkflowTraceRuntimeMetrics> for DiagnosticsTraceRuntimeMetrics {
    fn from(metrics: &WorkflowTraceRuntimeMetrics) -> Self {
        Self {
            runtime_id: metrics.runtime_id.clone(),
            observed_runtime_ids: metrics.observed_runtime_ids.clone(),
            runtime_instance_id: metrics.runtime_instance_id.clone(),
            model_target: metrics.model_target.clone(),
            warmup_started_at_ms: metrics.warmup_started_at_ms,
            warmup_completed_at_ms: metrics.warmup_completed_at_ms,
            warmup_duration_ms: metrics.warmup_duration_ms,
            runtime_reused: metrics.runtime_reused,
            lifecycle_decision_reason: metrics.lifecycle_decision_reason.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsRuntimeLifecycleSnapshot {
    #[serde(default)]
    pub runtime_id: Option<String>,
    #[serde(default)]
    pub runtime_instance_id: Option<String>,
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
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub last_error: Option<String>,
}

impl From<&inference::RuntimeLifecycleSnapshot> for DiagnosticsRuntimeLifecycleSnapshot {
    fn from(snapshot: &inference::RuntimeLifecycleSnapshot) -> Self {
        let snapshot = normalized_runtime_lifecycle_snapshot(snapshot);
        Self {
            runtime_id: snapshot.runtime_id,
            runtime_instance_id: snapshot.runtime_instance_id,
            warmup_started_at_ms: snapshot.warmup_started_at_ms,
            warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
            warmup_duration_ms: snapshot.warmup_duration_ms,
            runtime_reused: snapshot.runtime_reused,
            lifecycle_decision_reason: snapshot.lifecycle_decision_reason,
            active: snapshot.active,
            last_error: snapshot.last_error,
        }
    }
}

impl From<&DiagnosticsRuntimeLifecycleSnapshot> for inference::RuntimeLifecycleSnapshot {
    fn from(snapshot: &DiagnosticsRuntimeLifecycleSnapshot) -> Self {
        let lifecycle_snapshot = Self {
            runtime_id: snapshot.runtime_id.clone(),
            runtime_instance_id: snapshot.runtime_instance_id.clone(),
            warmup_started_at_ms: snapshot.warmup_started_at_ms,
            warmup_completed_at_ms: snapshot.warmup_completed_at_ms,
            warmup_duration_ms: snapshot.warmup_duration_ms,
            runtime_reused: snapshot.runtime_reused,
            lifecycle_decision_reason: snapshot.lifecycle_decision_reason.clone(),
            active: snapshot.active,
            last_error: snapshot.last_error.clone(),
        };
        normalized_runtime_lifecycle_snapshot(&lifecycle_snapshot)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
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
    #[serde(default)]
    pub active_model_target: Option<String>,
    #[serde(default)]
    pub embedding_model_target: Option<String>,
    #[serde(default)]
    pub active_runtime: Option<DiagnosticsRuntimeLifecycleSnapshot>,
    #[serde(default)]
    pub embedding_runtime: Option<DiagnosticsRuntimeLifecycleSnapshot>,
    #[serde(default)]
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
}

pub(crate) struct DiagnosticsRuntimeSnapshotInput {
    pub workflow_id: String,
    pub capabilities: Option<pantograph_workflow_service::WorkflowCapabilitiesResponse>,
    pub last_error: Option<String>,
    pub active_model_target: Option<String>,
    pub embedding_model_target: Option<String>,
    pub active_runtime_snapshot: Option<DiagnosticsRuntimeLifecycleSnapshot>,
    pub embedding_runtime_snapshot: Option<DiagnosticsRuntimeLifecycleSnapshot>,
    pub managed_runtimes: Vec<ManagedRuntimeManagerRuntimeView>,
    pub captured_at_ms: u64,
}

impl DiagnosticsRuntimeSnapshot {
    pub(crate) fn from_capabilities(input: DiagnosticsRuntimeSnapshotInput) -> Self {
        Self {
            workflow_id: Some(input.workflow_id),
            captured_at_ms: Some(input.captured_at_ms),
            max_input_bindings: input
                .capabilities
                .as_ref()
                .map(|value| value.max_input_bindings),
            max_output_targets: input
                .capabilities
                .as_ref()
                .map(|value| value.max_output_targets),
            max_value_bytes: input
                .capabilities
                .as_ref()
                .map(|value| value.max_value_bytes),
            runtime_requirements: input
                .capabilities
                .as_ref()
                .map(|value| value.runtime_requirements.clone()),
            runtime_capabilities: input
                .capabilities
                .as_ref()
                .map(|value| value.runtime_capabilities.clone())
                .unwrap_or_default(),
            models: input
                .capabilities
                .as_ref()
                .map(|value| value.models.clone())
                .unwrap_or_default(),
            last_error: input.last_error,
            active_model_target: input.active_model_target,
            embedding_model_target: input.embedding_model_target,
            active_runtime: input.active_runtime_snapshot.or_else(|| {
                capability_runtime_lifecycle_snapshot(input.capabilities.as_ref())
                    .map(|snapshot| DiagnosticsRuntimeLifecycleSnapshot::from(&snapshot))
            }),
            embedding_runtime: input.embedding_runtime_snapshot,
            managed_runtimes: input.managed_runtimes,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsSchedulerSnapshot {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub trace_execution_id: Option<String>,
    #[serde(default)]
    pub captured_at_ms: Option<u64>,
    #[serde(default)]
    pub session: Option<WorkflowExecutionSessionSummary>,
    pub items: Vec<WorkflowExecutionSessionQueueItem>,
    #[serde(default)]
    pub diagnostics: Option<WorkflowSchedulerSnapshotDiagnostics>,
    #[serde(default)]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDiagnosticsProjection {
    #[serde(default)]
    pub context: WorkflowDiagnosticsProjectionContext,
    pub runs_by_id: BTreeMap<String, DiagnosticsRunTrace>,
    pub run_order: Vec<String>,
    pub runtime: DiagnosticsRuntimeSnapshot,
    pub scheduler: DiagnosticsSchedulerSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_session_state: Option<WorkflowGraphSessionStateView>,
    pub retained_event_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDiagnosticsProjectionContext {
    #[serde(default)]
    pub requested_session_id: Option<String>,
    #[serde(default)]
    pub requested_workflow_id: Option<String>,
    #[serde(default)]
    pub requested_workflow_name: Option<String>,
    #[serde(default)]
    pub source_execution_id: Option<String>,
    #[serde(default)]
    pub relevant_execution_id: Option<String>,
    #[serde(default = "default_projection_relevance")]
    pub relevant: bool,
}

fn default_projection_relevance() -> bool {
    true
}

impl Default for WorkflowDiagnosticsProjectionContext {
    fn default() -> Self {
        Self {
            requested_session_id: None,
            requested_workflow_id: None,
            requested_workflow_name: None,
            source_execution_id: None,
            relevant_execution_id: None,
            relevant: true,
        }
    }
}

impl WorkflowDiagnosticsProjection {
    pub(crate) fn with_context(mut self, context: WorkflowDiagnosticsProjectionContext) -> Self {
        self.context = context;
        self
    }

    pub(crate) fn with_source_execution_id(mut self, source_execution_id: Option<String>) -> Self {
        self.context.source_execution_id = source_execution_id.clone();
        self.context.relevant_execution_id = source_execution_id;
        self.context.relevant = true;
        self
    }
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

impl WorkflowDiagnosticsSnapshotRequest {
    pub(crate) fn normalized(&self) -> Self {
        Self {
            session_id: normalize_optional_filter(&self.session_id),
            workflow_id: normalize_optional_filter(&self.workflow_id),
            workflow_name: normalize_optional_filter(&self.workflow_name),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), WorkflowServiceError> {
        validate_optional_filter(&self.session_id, "session_id")?;
        validate_optional_filter(&self.workflow_id, "workflow_id")?;
        validate_optional_filter(&self.workflow_name, "workflow_name")?;
        Ok(())
    }
}

fn normalize_optional_filter(value: &Option<String>) -> Option<String> {
    value.as_deref().map(str::trim).map(ToOwned::to_owned)
}

fn validate_optional_filter(
    value: &Option<String>,
    field_name: &'static str,
) -> Result<(), WorkflowServiceError> {
    if let Some(value) = value {
        if value.trim().is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow diagnostics snapshot request field '{}' must not be blank",
                field_name
            )));
        }
    }

    Ok(())
}

pub(crate) fn diagnostics_run_status(status: WorkflowTraceStatus) -> DiagnosticsRunStatus {
    match status {
        WorkflowTraceStatus::Queued | WorkflowTraceStatus::Running => DiagnosticsRunStatus::Running,
        WorkflowTraceStatus::Waiting => DiagnosticsRunStatus::Waiting,
        WorkflowTraceStatus::Completed => DiagnosticsRunStatus::Completed,
        WorkflowTraceStatus::Cancelled => DiagnosticsRunStatus::Cancelled,
        WorkflowTraceStatus::Failed => DiagnosticsRunStatus::Failed,
    }
}

pub(crate) fn diagnostics_node_status(status: WorkflowTraceNodeStatus) -> DiagnosticsNodeStatus {
    match status {
        WorkflowTraceNodeStatus::Pending | WorkflowTraceNodeStatus::Running => {
            DiagnosticsNodeStatus::Running
        }
        WorkflowTraceNodeStatus::Waiting => DiagnosticsNodeStatus::Waiting,
        WorkflowTraceNodeStatus::Completed => DiagnosticsNodeStatus::Completed,
        WorkflowTraceNodeStatus::Cancelled => DiagnosticsNodeStatus::Cancelled,
        WorkflowTraceNodeStatus::Failed => DiagnosticsNodeStatus::Failed,
    }
}
