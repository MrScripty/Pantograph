use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::GraphMemoryImpactSummary;

/// Events emitted during workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowEvent {
    /// Workflow execution started.
    #[serde(rename_all = "camelCase")]
    WorkflowStarted {
        workflow_id: String,
        execution_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Workflow execution completed successfully.
    #[serde(rename_all = "camelCase")]
    WorkflowCompleted {
        workflow_id: String,
        execution_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Workflow execution failed.
    #[serde(rename_all = "camelCase")]
    WorkflowFailed {
        workflow_id: String,
        execution_id: String,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Workflow execution was cancelled before completing successfully.
    #[serde(rename_all = "camelCase")]
    WorkflowCancelled {
        workflow_id: String,
        execution_id: String,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Workflow is waiting for user input.
    #[serde(rename_all = "camelCase")]
    WaitingForInput {
        workflow_id: String,
        execution_id: String,
        task_id: String,
        prompt: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// A task started executing.
    #[serde(rename_all = "camelCase")]
    TaskStarted {
        task_id: String,
        execution_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// A task completed successfully.
    #[serde(rename_all = "camelCase")]
    TaskCompleted {
        task_id: String,
        execution_id: String,
        output: Option<serde_json::Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// A task failed.
    #[serde(rename_all = "camelCase")]
    TaskFailed {
        task_id: String,
        execution_id: String,
        error: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Progress update for a task.
    #[serde(rename_all = "camelCase")]
    TaskProgress {
        task_id: String,
        execution_id: String,
        progress: f32,
        message: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Streaming output from a task.
    #[serde(rename_all = "camelCase")]
    TaskStream {
        task_id: String,
        execution_id: String,
        port: String,
        data: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Graph was modified during execution.
    #[serde(rename_all = "camelCase")]
    GraphModified {
        workflow_id: String,
        execution_id: String,
        dirty_tasks: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        memory_impact: Option<GraphMemoryImpactSummary>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },

    /// Incremental re-execution started.
    #[serde(rename_all = "camelCase")]
    IncrementalExecutionStarted {
        workflow_id: String,
        execution_id: String,
        tasks: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        occurred_at_ms: Option<u64>,
    },
}

impl WorkflowEvent {
    /// Create a task progress event.
    pub fn task_progress(
        task_id: &str,
        execution_id: &str,
        progress: f32,
        message: Option<String>,
    ) -> Self {
        Self::TaskProgress {
            task_id: task_id.to_string(),
            execution_id: execution_id.to_string(),
            progress,
            message,
            occurred_at_ms: Some(unix_timestamp_ms()),
        }
    }

    /// Create a task stream event.
    pub fn task_stream(
        task_id: &str,
        execution_id: &str,
        port: &str,
        data: serde_json::Value,
    ) -> Self {
        Self::TaskStream {
            task_id: task_id.to_string(),
            execution_id: execution_id.to_string(),
            port: port.to_string(),
            data,
            occurred_at_ms: Some(unix_timestamp_ms()),
        }
    }

    pub fn occurred_at_ms(&self) -> Option<u64> {
        match self {
            Self::WorkflowStarted { occurred_at_ms, .. }
            | Self::WorkflowCompleted { occurred_at_ms, .. }
            | Self::WorkflowFailed { occurred_at_ms, .. }
            | Self::WorkflowCancelled { occurred_at_ms, .. }
            | Self::WaitingForInput { occurred_at_ms, .. }
            | Self::TaskStarted { occurred_at_ms, .. }
            | Self::TaskCompleted { occurred_at_ms, .. }
            | Self::TaskFailed { occurred_at_ms, .. }
            | Self::TaskProgress { occurred_at_ms, .. }
            | Self::TaskStream { occurred_at_ms, .. }
            | Self::GraphModified { occurred_at_ms, .. }
            | Self::IncrementalExecutionStarted { occurred_at_ms, .. } => *occurred_at_ms,
        }
    }

    pub fn now(mut self) -> Self {
        let occurred_at_ms = Some(unix_timestamp_ms());
        match &mut self {
            Self::WorkflowStarted {
                occurred_at_ms: slot,
                ..
            }
            | Self::WorkflowCompleted {
                occurred_at_ms: slot,
                ..
            }
            | Self::WorkflowFailed {
                occurred_at_ms: slot,
                ..
            }
            | Self::WorkflowCancelled {
                occurred_at_ms: slot,
                ..
            }
            | Self::WaitingForInput {
                occurred_at_ms: slot,
                ..
            }
            | Self::TaskStarted {
                occurred_at_ms: slot,
                ..
            }
            | Self::TaskCompleted {
                occurred_at_ms: slot,
                ..
            }
            | Self::TaskFailed {
                occurred_at_ms: slot,
                ..
            }
            | Self::TaskProgress {
                occurred_at_ms: slot,
                ..
            }
            | Self::TaskStream {
                occurred_at_ms: slot,
                ..
            }
            | Self::GraphModified {
                occurred_at_ms: slot,
                ..
            }
            | Self::IncrementalExecutionStarted {
                occurred_at_ms: slot,
                ..
            } => {
                *slot = occurred_at_ms;
            }
        }

        self
    }
}

pub(crate) fn unix_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
