use pantograph_scheduler::{
    SchedulerContractError, SchedulerRuntimeHandoff, SchedulerRuntimeHandoffState,
    ValidatedSchedulerRuntimeHandoff,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const MAX_ID_LEN: usize = 128;
const MAX_TEXT_LEN: usize = 1024;
const MAX_RUNTIME_HOST_OUTPUTS: usize = 64;
const MAX_RUNTIME_HOST_DIAGNOSTICS: usize = 64;

/// Current contract version for runtime-host execution requests and responses.
pub const RUNTIME_HOST_EXECUTION_CONTRACT_VERSION: u16 = 1;

/// Host-owned request to execute one scheduler-dispatched task.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionRequest {
    #[serde(default = "default_runtime_host_execution_contract_version")]
    pub contract_version: u16,
    pub execution_request_id: String,
    pub handoff: SchedulerRuntimeHandoff,
}

impl RuntimeHostExecutionRequest {
    pub fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        validate_contract_version(self.contract_version)?;
        validate_identifier("execution_request_id", &self.execution_request_id)?;
        let validated_handoff = ValidatedSchedulerRuntimeHandoff::try_from(self.handoff.clone())?;
        if validated_handoff.as_ref().state != SchedulerRuntimeHandoffState::DispatchSelected {
            return Err(RuntimeHostExecutionContractError::InvalidField {
                field: "handoff.state",
                reason: "runtime host execution requires a dispatch-selected scheduler handoff",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedRuntimeHostExecutionRequest(RuntimeHostExecutionRequest);

impl ValidatedRuntimeHostExecutionRequest {
    #[must_use]
    pub fn as_ref(&self) -> &RuntimeHostExecutionRequest {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> RuntimeHostExecutionRequest {
        self.0
    }
}

impl TryFrom<RuntimeHostExecutionRequest> for ValidatedRuntimeHostExecutionRequest {
    type Error = RuntimeHostExecutionContractError;

    fn try_from(value: RuntimeHostExecutionRequest) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeHostExecutionState {
    Accepted,
    Rejected,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeHostExecutionDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RuntimeHostExecutionDiagnosticCode {
    HandoffAccepted,
    HandoffRejected,
    PumasLoadTargetRequired,
    PumasLoadTargetUnavailable,
    RuntimeUnavailable,
    ExecutionFailed,
    ExecutionCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionDiagnostic {
    pub severity: RuntimeHostExecutionDiagnosticSeverity,
    pub code: RuntimeHostExecutionDiagnosticCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl RuntimeHostExecutionDiagnostic {
    fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        validate_text("diagnostic.message", &self.message)?;
        if let Some(hint) = &self.hint {
            validate_text("diagnostic.hint", hint)?;
        }
        Ok(())
    }
}

/// Typed, path-free output produced by runtime-host execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionOutput {
    pub port_id: String,
    pub value: RuntimeHostExecutionOutputValue,
}

impl RuntimeHostExecutionOutput {
    fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        validate_identifier("output.port_id", &self.port_id)?;
        self.value.validate()
    }
}

/// Explicit runtime-host output value variants.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "value_type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[non_exhaustive]
pub enum RuntimeHostExecutionOutputValue {
    String(String),
    Bool(bool),
    I64(i64),
    U64(u64),
    MediaArtifactRef(RuntimeHostExecutionMediaArtifactRef),
    DiagnosticOnly,
}

impl RuntimeHostExecutionOutputValue {
    fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        match self {
            Self::String(value) => validate_optional_text("output.string", value),
            Self::MediaArtifactRef(value) => value.validate(),
            Self::Bool(_) | Self::I64(_) | Self::U64(_) | Self::DiagnosticOnly => Ok(()),
        }
    }
}

/// Path-free media or artifact reference returned by a runtime host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionMediaArtifactRef {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

impl RuntimeHostExecutionMediaArtifactRef {
    fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        validate_identifier("media_artifact.artifact_id", &self.artifact_id)?;
        if let Some(media_type) = self.media_type.as_ref() {
            validate_identifier("media_artifact.media_type", media_type)?;
        }
        Ok(())
    }
}

/// Optional path-free terminal metadata for a runtime-host execution response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionTerminalMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u32>,
}

impl RuntimeHostExecutionTerminalMetadata {
    fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        Ok(())
    }
}

/// Host-owned response for one runtime execution request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct RuntimeHostExecutionResponse {
    #[serde(default = "default_runtime_host_execution_contract_version")]
    pub contract_version: u16,
    pub execution_request_id: String,
    pub workflow_id: pantograph_scheduler::SchedulerWorkflowId,
    pub workflow_run_id: pantograph_scheduler::SchedulerWorkflowRunId,
    pub node_id: pantograph_scheduler::SchedulerNodeId,
    pub task_id: pantograph_scheduler::SchedulerTaskId,
    pub state: RuntimeHostExecutionState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<RuntimeHostExecutionOutput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<RuntimeHostExecutionDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_metadata: Option<RuntimeHostExecutionTerminalMetadata>,
}

impl RuntimeHostExecutionResponse {
    pub fn validate(&self) -> Result<(), RuntimeHostExecutionContractError> {
        validate_contract_version(self.contract_version)?;
        validate_identifier("execution_request_id", &self.execution_request_id)?;
        if self.outputs.len() > MAX_RUNTIME_HOST_OUTPUTS {
            return Err(RuntimeHostExecutionContractError::TooManyOutputs {
                actual: self.outputs.len(),
                max: MAX_RUNTIME_HOST_OUTPUTS,
            });
        }
        if !matches!(self.state, RuntimeHostExecutionState::Completed) && !self.outputs.is_empty() {
            return Err(RuntimeHostExecutionContractError::InvalidField {
                field: "outputs",
                reason: "runtime-host outputs are valid only on completed responses",
            });
        }
        if self.diagnostics.len() > MAX_RUNTIME_HOST_DIAGNOSTICS {
            return Err(RuntimeHostExecutionContractError::TooManyDiagnostics {
                actual: self.diagnostics.len(),
                max: MAX_RUNTIME_HOST_DIAGNOSTICS,
            });
        }
        if matches!(
            self.state,
            RuntimeHostExecutionState::Rejected | RuntimeHostExecutionState::Failed
        ) && self.diagnostics.is_empty()
        {
            return Err(RuntimeHostExecutionContractError::MissingField {
                field: "diagnostics",
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ValidatedRuntimeHostExecutionResponse(RuntimeHostExecutionResponse);

impl ValidatedRuntimeHostExecutionResponse {
    #[must_use]
    pub fn as_ref(&self) -> &RuntimeHostExecutionResponse {
        &self.0
    }

    #[must_use]
    pub fn into_inner(self) -> RuntimeHostExecutionResponse {
        self.0
    }
}

impl TryFrom<RuntimeHostExecutionResponse> for ValidatedRuntimeHostExecutionResponse {
    type Error = RuntimeHostExecutionContractError;

    fn try_from(value: RuntimeHostExecutionResponse) -> Result<Self, Self::Error> {
        value.validate()?;
        Ok(Self(value))
    }
}

/// Runtime-host execution DTO validation error.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeHostExecutionContractError {
    #[error("missing required field `{field}`")]
    MissingField { field: &'static str },
    #[error("invalid field `{field}`: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("field `{field}` is too long; max {max_len} bytes")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("invalid identifier field `{field}`")]
    InvalidIdentifier { field: &'static str },
    #[error("invalid text field `{field}`")]
    InvalidText { field: &'static str },
    #[error("runtime-host response has {actual} outputs, maximum is {max}")]
    TooManyOutputs { actual: usize, max: usize },
    #[error("runtime-host response has {actual} diagnostics, maximum is {max}")]
    TooManyDiagnostics { actual: usize, max: usize },
    #[error(transparent)]
    Scheduler(#[from] SchedulerContractError),
}

fn default_runtime_host_execution_contract_version() -> u16 {
    RUNTIME_HOST_EXECUTION_CONTRACT_VERSION
}

fn validate_contract_version(value: u16) -> Result<(), RuntimeHostExecutionContractError> {
    if value == RUNTIME_HOST_EXECUTION_CONTRACT_VERSION {
        Ok(())
    } else {
        Err(RuntimeHostExecutionContractError::InvalidField {
            field: "contract_version",
            reason: "unsupported runtime-host execution contract version",
        })
    }
}

fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeHostExecutionContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.len() > MAX_ID_LEN
        || trimmed
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':')))
    {
        return Err(RuntimeHostExecutionContractError::InvalidIdentifier { field });
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeHostExecutionContractError> {
    if value.trim().is_empty() {
        return Err(RuntimeHostExecutionContractError::InvalidText { field });
    }
    if value.len() > MAX_TEXT_LEN {
        return Err(RuntimeHostExecutionContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    Ok(())
}

fn validate_optional_text(
    field: &'static str,
    value: &str,
) -> Result<(), RuntimeHostExecutionContractError> {
    if value.len() > MAX_TEXT_LEN {
        return Err(RuntimeHostExecutionContractError::FieldTooLong {
            field,
            max_len: MAX_TEXT_LEN,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "runtime_host_execution_tests.rs"]
mod tests;
