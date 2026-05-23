use pantograph_dependency_planning::PumasModelRef;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current schema version for workflow scheduler task results.
pub const WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION: u16 = 1;

/// Maximum number of output values a task result may carry.
pub const WORKFLOW_SCHEDULER_TASK_RESULT_MAX_OUTPUTS: usize = 64;

/// Maximum number of diagnostics a task result may carry.
pub const WORKFLOW_SCHEDULER_TASK_RESULT_MAX_DIAGNOSTICS: usize = 64;

const TASK_RESULT_ID_MAX_LEN: usize = 256;
const TASK_RESULT_MESSAGE_MAX_LEN: usize = 2048;

/// Typed task completion value for scheduler-owned workflow progress.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskResult {
    #[serde(default = "default_workflow_scheduler_task_result_schema_version")]
    pub schema_version: u16,
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub node_id: String,
    pub task_id: String,
    pub status: WorkflowSchedulerTaskResultStatus,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<WorkflowSchedulerTaskResultOutput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowSchedulerTaskResultDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_metadata: Option<WorkflowSchedulerTaskResultTerminalMetadata>,
}

impl WorkflowSchedulerTaskResult {
    /// Validate the task-result DTO before it is persisted or projected.
    pub fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        if self.schema_version != WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION {
            return Err(WorkflowSchedulerTaskResultError::UnsupportedSchemaVersion {
                actual: self.schema_version,
                expected: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            });
        }
        validate_id("workflow_id", &self.workflow_id)?;
        validate_id("workflow_run_id", &self.workflow_run_id)?;
        validate_id("node_id", &self.node_id)?;
        validate_id("task_id", &self.task_id)?;
        if self.outputs.len() > WORKFLOW_SCHEDULER_TASK_RESULT_MAX_OUTPUTS {
            return Err(WorkflowSchedulerTaskResultError::TooManyOutputs {
                actual: self.outputs.len(),
                max: WORKFLOW_SCHEDULER_TASK_RESULT_MAX_OUTPUTS,
            });
        }
        if self.diagnostics.len() > WORKFLOW_SCHEDULER_TASK_RESULT_MAX_DIAGNOSTICS {
            return Err(WorkflowSchedulerTaskResultError::TooManyDiagnostics {
                actual: self.diagnostics.len(),
                max: WORKFLOW_SCHEDULER_TASK_RESULT_MAX_DIAGNOSTICS,
            });
        }
        for output in &self.outputs {
            output.validate()?;
        }
        for diagnostic in &self.diagnostics {
            diagnostic.validate()?;
        }
        if let Some(metadata) = self.terminal_metadata.as_ref() {
            metadata.validate()?;
        }
        Ok(())
    }
}

/// Scheduler task-result terminal status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskResultStatus {
    Completed,
    Failed,
    Unavailable,
    Invalid,
}

/// A typed output value produced by one scheduler-owned task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskResultOutput {
    pub port_id: String,
    pub value: WorkflowSchedulerTaskResultValue,
}

impl WorkflowSchedulerTaskResultOutput {
    fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        validate_id("port_id", &self.port_id)?;
        self.value.validate()
    }
}

/// Explicit materialized task-result value variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "value_type", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskResultValue {
    PumasModelRef(PumasModelRef),
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
    MediaArtifactRef(WorkflowSchedulerTaskMediaArtifactRef),
    DiagnosticOnly,
}

impl WorkflowSchedulerTaskResultValue {
    fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        match self {
            Self::PumasModelRef(model_ref) => model_ref.validate().map_err(|error| {
                WorkflowSchedulerTaskResultError::InvalidPumasModelRef {
                    message: error.to_string(),
                }
            }),
            Self::String(value) => validate_message("string output", value),
            Self::MediaArtifactRef(media_ref) => media_ref.validate(),
            Self::Bool(_) | Self::I64(_) | Self::U64(_) | Self::DiagnosticOnly => Ok(()),
        }
    }
}

/// Path-free media/artifact reference produced by a scheduler task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskMediaArtifactRef {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

impl WorkflowSchedulerTaskMediaArtifactRef {
    fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        validate_id("artifact_id", &self.artifact_id)?;
        if let Some(media_type) = self.media_type.as_ref() {
            validate_id("media_type", media_type)?;
        }
        Ok(())
    }
}

/// Bounded diagnostic carried with a task result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskResultDiagnostic {
    pub code: String,
    pub severity: WorkflowSchedulerTaskResultDiagnosticSeverity,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
}

impl WorkflowSchedulerTaskResultDiagnostic {
    fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        validate_id("diagnostic.code", &self.code)?;
        validate_message("diagnostic.message", &self.message)?;
        if let Some(port_id) = self.port_id.as_ref() {
            validate_id("diagnostic.port_id", port_id)?;
        }
        Ok(())
    }
}

/// Task-result diagnostic severity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskResultDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Optional path-free terminal metadata for completed or failed tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskResultTerminalMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
}

impl WorkflowSchedulerTaskResultTerminalMetadata {
    fn validate(&self) -> Result<(), WorkflowSchedulerTaskResultError> {
        Ok(())
    }
}

/// Validation failure for task-result contracts.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WorkflowSchedulerTaskResultError {
    #[error("unsupported task-result schema version {actual}, expected {expected}")]
    UnsupportedSchemaVersion { actual: u16, expected: u16 },
    #[error("{field} must be non-empty")]
    BlankId { field: &'static str },
    #[error("{field} must be at most {max} bytes, got {actual}")]
    IdTooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    #[error("{field} must not contain control characters")]
    IdContainsControlCharacter { field: &'static str },
    #[error("{field} must be at most {max} bytes, got {actual}")]
    MessageTooLong {
        field: &'static str,
        max: usize,
        actual: usize,
    },
    #[error("task result has {actual} outputs, maximum is {max}")]
    TooManyOutputs { actual: usize, max: usize },
    #[error("task result has {actual} diagnostics, maximum is {max}")]
    TooManyDiagnostics { actual: usize, max: usize },
    #[error("invalid pumas model ref: {message}")]
    InvalidPumasModelRef { message: String },
}

fn default_workflow_scheduler_task_result_schema_version() -> u16 {
    WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION
}

fn validate_id(field: &'static str, value: &str) -> Result<(), WorkflowSchedulerTaskResultError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(WorkflowSchedulerTaskResultError::BlankId { field });
    }
    if value.len() > TASK_RESULT_ID_MAX_LEN {
        return Err(WorkflowSchedulerTaskResultError::IdTooLong {
            field,
            max: TASK_RESULT_ID_MAX_LEN,
            actual: value.len(),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(WorkflowSchedulerTaskResultError::IdContainsControlCharacter { field });
    }
    Ok(())
}

fn validate_message(
    field: &'static str,
    value: &str,
) -> Result<(), WorkflowSchedulerTaskResultError> {
    if value.len() > TASK_RESULT_MESSAGE_MAX_LEN {
        return Err(WorkflowSchedulerTaskResultError::MessageTooLong {
            field,
            max: TASK_RESULT_MESSAGE_MAX_LEN,
            actual: value.len(),
        });
    }
    Ok(())
}
