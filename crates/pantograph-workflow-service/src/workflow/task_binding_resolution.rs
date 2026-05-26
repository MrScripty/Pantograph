use pantograph_scheduler::SchedulableTaskIntent;
use serde::{Deserialize, Serialize};

use super::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskBindingResolution {
    pub status: WorkflowSchedulerTaskBindingResolutionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedulable_intent: Option<SchedulableTaskIntent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowSchedulerTaskBindingDiagnostic>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskBindingResolutionStatus {
    Ready,
    Blocked,
    Invalid,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct WorkflowSchedulerTaskBindingDiagnostic {
    pub code: WorkflowSchedulerTaskBindingDiagnosticCode,
    pub severity: WorkflowSchedulerTaskBindingDiagnosticSeverity,
    pub node_id: String,
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskBindingDiagnosticCode {
    TaskProjectionInvalid,
    MissingIntentTemplate,
    MissingMaterializedInput,
    UpstreamTaskUnavailable,
    UpstreamTaskInvalid,
    WrongMaterializedValueType,
    InvalidMaterializedIntent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WorkflowSchedulerTaskBindingDiagnosticSeverity {
    Error,
}

pub fn workflow_scheduler_resolve_task_intent(
    task: &WorkflowSchedulerTask,
    task_results: &[WorkflowSchedulerTaskResult],
) -> WorkflowSchedulerTaskBindingResolution {
    if !task.diagnostics.is_empty() {
        return resolution(
            WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
            None,
            diagnostic(
                task,
                None,
                WorkflowSchedulerTaskBindingDiagnosticCode::TaskProjectionInvalid,
                "scheduler task projection has blocking diagnostics",
            ),
        );
    }

    if let Some(intent) = task.schedulable_intent.clone() {
        if let Some(diagnostic) = first_unready_materialized_input(task, task_results) {
            return resolution(
                status_for_materialized_input_diagnostic(diagnostic.code),
                None,
                diagnostic,
            );
        }
        return validated_intent_resolution(task, intent);
    }

    resolution(
        WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
        None,
        diagnostic(
            task,
            None,
            WorkflowSchedulerTaskBindingDiagnosticCode::MissingIntentTemplate,
            "scheduler task has no descriptor-backed schedulable intent",
        ),
    )
}

fn first_unready_materialized_input(
    task: &WorkflowSchedulerTask,
    task_results: &[WorkflowSchedulerTaskResult],
) -> Option<WorkflowSchedulerTaskBindingDiagnostic> {
    task.input_bindings
        .iter()
        .find_map(|binding| materialized_output(task, task_results, binding).err())
}

fn status_for_materialized_input_diagnostic(
    code: WorkflowSchedulerTaskBindingDiagnosticCode,
) -> WorkflowSchedulerTaskBindingResolutionStatus {
    match code {
        WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput => {
            WorkflowSchedulerTaskBindingResolutionStatus::Blocked
        }
        WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskUnavailable => {
            WorkflowSchedulerTaskBindingResolutionStatus::Unavailable
        }
        WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskInvalid
        | WorkflowSchedulerTaskBindingDiagnosticCode::WrongMaterializedValueType
        | WorkflowSchedulerTaskBindingDiagnosticCode::TaskProjectionInvalid
        | WorkflowSchedulerTaskBindingDiagnosticCode::MissingIntentTemplate
        | WorkflowSchedulerTaskBindingDiagnosticCode::InvalidMaterializedIntent => {
            WorkflowSchedulerTaskBindingResolutionStatus::Invalid
        }
    }
}

fn materialized_output<'a>(
    task: &WorkflowSchedulerTask,
    task_results: &'a [WorkflowSchedulerTaskResult],
    binding: &WorkflowSchedulerTaskInputBinding,
) -> Result<&'a WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskBindingDiagnostic> {
    let Some(result) = task_results.iter().find(|result| {
        result.task_id == binding.source_task_id.as_str()
            && result.workflow_run_id == task.workflow_run_id.as_str()
    }) else {
        return Err(diagnostic(
            task,
            Some(binding.target_port_id.as_str()),
            WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput,
            "required materialized task result is not available",
        ));
    };

    if let Err(error) = result.validate() {
        return Err(diagnostic(
            task,
            Some(binding.target_port_id.as_str()),
            WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskInvalid,
            format!("materialized task result is invalid: {error}"),
        ));
    }

    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => {}
        WorkflowSchedulerTaskResultStatus::Unavailable => {
            return Err(diagnostic(
                task,
                Some(binding.target_port_id.as_str()),
                WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskUnavailable,
                format!("upstream task '{}' is unavailable", result.task_id),
            ));
        }
        WorkflowSchedulerTaskResultStatus::Failed | WorkflowSchedulerTaskResultStatus::Invalid => {
            return Err(diagnostic(
                task,
                Some(binding.target_port_id.as_str()),
                WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskInvalid,
                format!(
                    "upstream task '{}' did not complete successfully",
                    result.task_id
                ),
            ));
        }
    }

    result
        .outputs
        .iter()
        .find(|output| output.port_id == binding.source_port_id)
        .ok_or_else(|| {
            diagnostic(
                task,
                Some(binding.target_port_id.as_str()),
                WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput,
                format!(
                    "upstream task '{}' has not materialized output '{}'",
                    result.task_id, binding.source_port_id
                ),
            )
        })
}

fn validated_intent_resolution(
    task: &WorkflowSchedulerTask,
    intent: SchedulableTaskIntent,
) -> WorkflowSchedulerTaskBindingResolution {
    match intent.validate() {
        Ok(()) => WorkflowSchedulerTaskBindingResolution {
            status: WorkflowSchedulerTaskBindingResolutionStatus::Ready,
            schedulable_intent: Some(intent),
            diagnostics: Vec::new(),
        },
        Err(error) => resolution(
            WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
            None,
            diagnostic(
                task,
                None,
                WorkflowSchedulerTaskBindingDiagnosticCode::InvalidMaterializedIntent,
                format!("materialized schedulable task intent is invalid: {error}"),
            ),
        ),
    }
}

fn resolution(
    status: WorkflowSchedulerTaskBindingResolutionStatus,
    schedulable_intent: Option<SchedulableTaskIntent>,
    diagnostic: WorkflowSchedulerTaskBindingDiagnostic,
) -> WorkflowSchedulerTaskBindingResolution {
    WorkflowSchedulerTaskBindingResolution {
        status,
        schedulable_intent,
        diagnostics: vec![diagnostic],
    }
}

fn diagnostic(
    task: &WorkflowSchedulerTask,
    port_id: Option<&str>,
    code: WorkflowSchedulerTaskBindingDiagnosticCode,
    message: impl Into<String>,
) -> WorkflowSchedulerTaskBindingDiagnostic {
    WorkflowSchedulerTaskBindingDiagnostic {
        code,
        severity: WorkflowSchedulerTaskBindingDiagnosticSeverity::Error,
        node_id: task.node_id.as_str().to_string(),
        task_id: task.task_id.as_str().to_string(),
        port_id: port_id.map(str::to_string),
        message: message.into(),
    }
}
