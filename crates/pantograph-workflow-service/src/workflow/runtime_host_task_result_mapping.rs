use pantograph_runtime_host_contracts::{
    RuntimeHostBatchExecutionMemberResponse, RuntimeHostBatchExecutionMemberState,
    RuntimeHostExecutionDiagnosticCode, RuntimeHostExecutionDiagnosticSeverity,
    RuntimeHostExecutionMediaArtifactRef, RuntimeHostExecutionOutputValue,
    RuntimeHostExecutionState, ValidatedRuntimeHostExecutionResponse,
};
use thiserror::Error;

use super::{
    WorkflowSchedulerTaskMediaArtifactRef, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultDiagnostic, WorkflowSchedulerTaskResultDiagnosticSeverity,
    WorkflowSchedulerTaskResultError, WorkflowSchedulerTaskResultOutput,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultTerminalMetadata,
    WorkflowSchedulerTaskResultValue, WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

pub(crate) fn runtime_host_response_to_task_result(
    response: &ValidatedRuntimeHostExecutionResponse,
) -> Result<WorkflowSchedulerTaskResult, WorkflowRuntimeHostTaskResultMappingError> {
    let response = response.as_ref();
    let status = task_result_status(response.state.clone())?;
    let result = WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: response.workflow_id.as_str().to_owned(),
        workflow_run_id: response.workflow_run_id.as_str().to_owned(),
        node_id: response.node_id.as_str().to_owned(),
        task_id: response.task_id.as_str().to_owned(),
        status,
        outputs: response
            .outputs
            .iter()
            .map(|output| {
                Ok(WorkflowSchedulerTaskResultOutput {
                    port_id: output.port_id.clone(),
                    value: task_result_value(output.value.clone())?,
                })
            })
            .collect::<Result<Vec<_>, WorkflowRuntimeHostTaskResultMappingError>>()?,
        diagnostics: response
            .diagnostics
            .iter()
            .map(|diagnostic| {
                Ok(WorkflowSchedulerTaskResultDiagnostic {
                    code: diagnostic_code(&diagnostic.code)?.to_owned(),
                    severity: diagnostic_severity(&diagnostic.severity)?,
                    message: diagnostic.message.clone(),
                    port_id: None,
                })
            })
            .collect::<Result<Vec<_>, WorkflowRuntimeHostTaskResultMappingError>>()?,
        terminal_metadata: response.terminal_metadata.as_ref().map(|metadata| {
            WorkflowSchedulerTaskResultTerminalMetadata {
                completed_at_ms: metadata.completed_at_ms,
                attempt: metadata.attempt,
            }
        }),
    };
    result.validate()?;
    Ok(result)
}

#[allow(dead_code)]
pub(crate) fn runtime_host_batch_member_response_to_task_result(
    member: &RuntimeHostBatchExecutionMemberResponse,
) -> Result<WorkflowSchedulerTaskResult, WorkflowRuntimeHostTaskResultMappingError> {
    let status = batch_member_task_result_status(member.state.clone())?;
    let result = WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: member.workflow_id.as_str().to_owned(),
        workflow_run_id: member.workflow_run_id.as_str().to_owned(),
        node_id: member.node_id.as_str().to_owned(),
        task_id: member.task_id.as_str().to_owned(),
        status,
        outputs: member
            .outputs
            .iter()
            .map(|output| {
                Ok(WorkflowSchedulerTaskResultOutput {
                    port_id: output.port_id.clone(),
                    value: task_result_value(output.value.clone())?,
                })
            })
            .collect::<Result<Vec<_>, WorkflowRuntimeHostTaskResultMappingError>>()?,
        diagnostics: member
            .diagnostics
            .iter()
            .map(|diagnostic| {
                Ok(WorkflowSchedulerTaskResultDiagnostic {
                    code: diagnostic_code(&diagnostic.code)?.to_owned(),
                    severity: diagnostic_severity(&diagnostic.severity)?,
                    message: diagnostic.message.clone(),
                    port_id: None,
                })
            })
            .collect::<Result<Vec<_>, WorkflowRuntimeHostTaskResultMappingError>>()?,
        terminal_metadata: member.terminal_metadata.as_ref().map(|metadata| {
            WorkflowSchedulerTaskResultTerminalMetadata {
                completed_at_ms: metadata.completed_at_ms,
                attempt: metadata.attempt,
            }
        }),
    };
    result.validate()?;
    Ok(result)
}

fn task_result_status(
    state: RuntimeHostExecutionState,
) -> Result<WorkflowSchedulerTaskResultStatus, WorkflowRuntimeHostTaskResultMappingError> {
    match state {
        RuntimeHostExecutionState::Completed => Ok(WorkflowSchedulerTaskResultStatus::Completed),
        RuntimeHostExecutionState::Failed => Ok(WorkflowSchedulerTaskResultStatus::Failed),
        RuntimeHostExecutionState::Rejected => Ok(WorkflowSchedulerTaskResultStatus::Failed),
        RuntimeHostExecutionState::Accepted => {
            Err(WorkflowRuntimeHostTaskResultMappingError::NonTerminalRuntimeHostState)
        }
        _ => Err(WorkflowRuntimeHostTaskResultMappingError::UnsupportedRuntimeHostState),
    }
}

#[allow(dead_code)]
fn batch_member_task_result_status(
    state: RuntimeHostBatchExecutionMemberState,
) -> Result<WorkflowSchedulerTaskResultStatus, WorkflowRuntimeHostTaskResultMappingError> {
    match state {
        RuntimeHostBatchExecutionMemberState::Completed => {
            Ok(WorkflowSchedulerTaskResultStatus::Completed)
        }
        RuntimeHostBatchExecutionMemberState::Failed
        | RuntimeHostBatchExecutionMemberState::Rejected
        | RuntimeHostBatchExecutionMemberState::Cancelled => {
            Ok(WorkflowSchedulerTaskResultStatus::Failed)
        }
        RuntimeHostBatchExecutionMemberState::Accepted
        | RuntimeHostBatchExecutionMemberState::Deferred => {
            Err(WorkflowRuntimeHostTaskResultMappingError::NonTerminalRuntimeHostState)
        }
        _ => Err(WorkflowRuntimeHostTaskResultMappingError::UnsupportedRuntimeHostState),
    }
}

fn task_result_value(
    value: RuntimeHostExecutionOutputValue,
) -> Result<WorkflowSchedulerTaskResultValue, WorkflowRuntimeHostTaskResultMappingError> {
    let value = match value {
        RuntimeHostExecutionOutputValue::String(value) => {
            WorkflowSchedulerTaskResultValue::String(value)
        }
        RuntimeHostExecutionOutputValue::Bool(value) => {
            WorkflowSchedulerTaskResultValue::Bool(value)
        }
        RuntimeHostExecutionOutputValue::I64(value) => WorkflowSchedulerTaskResultValue::I64(value),
        RuntimeHostExecutionOutputValue::U64(value) => WorkflowSchedulerTaskResultValue::U64(value),
        RuntimeHostExecutionOutputValue::MediaArtifactRef(value) => {
            WorkflowSchedulerTaskResultValue::MediaArtifactRef(media_artifact_ref(value))
        }
        RuntimeHostExecutionOutputValue::DiagnosticOnly => {
            WorkflowSchedulerTaskResultValue::DiagnosticOnly
        }
        _ => return Err(WorkflowRuntimeHostTaskResultMappingError::UnsupportedRuntimeHostOutput),
    };
    Ok(value)
}

fn media_artifact_ref(
    value: RuntimeHostExecutionMediaArtifactRef,
) -> WorkflowSchedulerTaskMediaArtifactRef {
    WorkflowSchedulerTaskMediaArtifactRef {
        artifact_id: value.artifact_id,
        media_type: value.media_type,
    }
}

fn diagnostic_severity(
    severity: &RuntimeHostExecutionDiagnosticSeverity,
) -> Result<WorkflowSchedulerTaskResultDiagnosticSeverity, WorkflowRuntimeHostTaskResultMappingError>
{
    let severity = match severity {
        RuntimeHostExecutionDiagnosticSeverity::Info => {
            WorkflowSchedulerTaskResultDiagnosticSeverity::Info
        }
        RuntimeHostExecutionDiagnosticSeverity::Warning => {
            WorkflowSchedulerTaskResultDiagnosticSeverity::Warning
        }
        RuntimeHostExecutionDiagnosticSeverity::Error => {
            WorkflowSchedulerTaskResultDiagnosticSeverity::Error
        }
        _ => {
            return Err(WorkflowRuntimeHostTaskResultMappingError::UnsupportedRuntimeHostDiagnostic)
        }
    };
    Ok(severity)
}

fn diagnostic_code(
    code: &RuntimeHostExecutionDiagnosticCode,
) -> Result<&'static str, WorkflowRuntimeHostTaskResultMappingError> {
    let code = match code {
        RuntimeHostExecutionDiagnosticCode::HandoffAccepted => "runtime_host.handoff_accepted",
        RuntimeHostExecutionDiagnosticCode::HandoffRejected => "runtime_host.handoff_rejected",
        RuntimeHostExecutionDiagnosticCode::PumasLoadTargetRequired => {
            "runtime_host.pumas_load_target_required"
        }
        RuntimeHostExecutionDiagnosticCode::PumasLoadTargetUnavailable => {
            "runtime_host.pumas_load_target_unavailable"
        }
        RuntimeHostExecutionDiagnosticCode::RuntimeUnavailable => {
            "runtime_host.runtime_unavailable"
        }
        RuntimeHostExecutionDiagnosticCode::CancellationRequested => {
            "runtime_host.cancellation_requested"
        }
        RuntimeHostExecutionDiagnosticCode::ShutdownRequested => "runtime_host.shutdown_requested",
        RuntimeHostExecutionDiagnosticCode::ExecutionFailed => "runtime_host.execution_failed",
        RuntimeHostExecutionDiagnosticCode::ExecutionCompleted => {
            "runtime_host.execution_completed"
        }
        _ => {
            return Err(WorkflowRuntimeHostTaskResultMappingError::UnsupportedRuntimeHostDiagnostic)
        }
    };
    Ok(code)
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeHostTaskResultMappingError {
    #[error("runtime-host response state is not terminal")]
    NonTerminalRuntimeHostState,
    #[error("runtime-host response state is unsupported by workflow task results")]
    UnsupportedRuntimeHostState,
    #[error("runtime-host response output is unsupported by workflow task results")]
    UnsupportedRuntimeHostOutput,
    #[error("runtime-host response diagnostic is unsupported by workflow task results")]
    UnsupportedRuntimeHostDiagnostic,
    #[error("mapped runtime-host task result is invalid")]
    InvalidTaskResult(#[from] WorkflowSchedulerTaskResultError),
}

#[cfg(test)]
#[path = "runtime_host_task_result_mapping_tests.rs"]
mod tests;
