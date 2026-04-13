use serde::{Deserialize, Serialize};

use crate::workflow::WorkflowServiceError;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::WorkflowServiceError;

    #[test]
    fn workflow_trace_summary_serializes_with_snake_case_contract() {
        let value = serde_json::to_value(WorkflowTraceSummary {
            execution_id: "exec-1".to_string(),
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
}
