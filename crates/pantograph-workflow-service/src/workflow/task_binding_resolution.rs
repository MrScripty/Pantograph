use pantograph_dependency_planning::PumasModelRef;
use pantograph_scheduler::SchedulableTaskIntent;
use serde::{Deserialize, Serialize};

use super::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue,
};

const PORT_PUMAS_MODEL_REF: &str = "pumas_model_ref";

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
        return validated_intent_resolution(task, intent);
    }

    let Some(template) = task.schedulable_intent_template.as_ref() else {
        return resolution(
            WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
            None,
            diagnostic(
                task,
                None,
                WorkflowSchedulerTaskBindingDiagnosticCode::MissingIntentTemplate,
                "scheduler task has no complete intent or materializable intent template",
            ),
        );
    };

    let Some(model_ref_binding) = task
        .input_bindings
        .iter()
        .find(|binding| binding.target_port_id == PORT_PUMAS_MODEL_REF)
    else {
        return resolution(
            WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
            None,
            diagnostic(
                task,
                Some(PORT_PUMAS_MODEL_REF),
                WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput,
                "scheduler task cannot materialize pumas_model_ref without an input binding",
            ),
        );
    };

    let model_ref = match materialized_model_ref(task, task_results, model_ref_binding) {
        MaterializedModelRefResolution::Ready(model_ref) => model_ref,
        MaterializedModelRefResolution::Blocked(diagnostic) => {
            return resolution(
                WorkflowSchedulerTaskBindingResolutionStatus::Blocked,
                None,
                diagnostic,
            );
        }
        MaterializedModelRefResolution::Unavailable(diagnostic) => {
            return resolution(
                WorkflowSchedulerTaskBindingResolutionStatus::Unavailable,
                None,
                diagnostic,
            );
        }
        MaterializedModelRefResolution::Invalid(diagnostic) => {
            return resolution(
                WorkflowSchedulerTaskBindingResolutionStatus::Invalid,
                None,
                diagnostic,
            );
        }
    };

    validated_intent_resolution(
        task,
        SchedulableTaskIntent {
            contract_version: pantograph_scheduler::SCHEDULABLE_TASK_INTENT_CONTRACT_VERSION,
            workflow_id: task.workflow_id.clone(),
            workflow_run_id: task.workflow_run_id.clone(),
            node_id: task.node_id.clone(),
            task_id: task.task_id.clone(),
            fairness_key: None,
            task_type: template.task_type.clone(),
            model_ref,
            constraints: template.constraints.clone(),
            trait_settings: template.trait_settings.clone(),
            dependency_override_patches: template.dependency_override_patches.clone(),
            estimate_hints: template.estimate_hints.clone(),
        },
    )
}

enum MaterializedModelRefResolution {
    Ready(PumasModelRef),
    Blocked(WorkflowSchedulerTaskBindingDiagnostic),
    Invalid(WorkflowSchedulerTaskBindingDiagnostic),
    Unavailable(WorkflowSchedulerTaskBindingDiagnostic),
}

fn materialized_model_ref(
    task: &WorkflowSchedulerTask,
    task_results: &[WorkflowSchedulerTaskResult],
    binding: &super::WorkflowSchedulerTaskInputBinding,
) -> MaterializedModelRefResolution {
    let Some(result) = task_results.iter().find(|result| {
        result.task_id == binding.source_task_id.as_str()
            && result.workflow_run_id == task.workflow_run_id.as_str()
    }) else {
        return MaterializedModelRefResolution::Blocked(diagnostic(
            task,
            Some(PORT_PUMAS_MODEL_REF),
            WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput,
            "required materialized task result is not available",
        ));
    };

    if let Err(error) = result.validate() {
        return MaterializedModelRefResolution::Invalid(diagnostic(
            task,
            Some(PORT_PUMAS_MODEL_REF),
            WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskInvalid,
            format!("materialized task result is invalid: {error}"),
        ));
    }

    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => {}
        WorkflowSchedulerTaskResultStatus::Unavailable => {
            return MaterializedModelRefResolution::Unavailable(diagnostic(
                task,
                Some(PORT_PUMAS_MODEL_REF),
                WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskUnavailable,
                format!("upstream task '{}' is unavailable", result.task_id),
            ));
        }
        WorkflowSchedulerTaskResultStatus::Failed | WorkflowSchedulerTaskResultStatus::Invalid => {
            return MaterializedModelRefResolution::Invalid(diagnostic(
                task,
                Some(PORT_PUMAS_MODEL_REF),
                WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskInvalid,
                format!(
                    "upstream task '{}' did not complete successfully",
                    result.task_id
                ),
            ));
        }
    }

    let Some(output) = result
        .outputs
        .iter()
        .find(|output| output.port_id == binding.source_port_id)
    else {
        return MaterializedModelRefResolution::Blocked(diagnostic(
            task,
            Some(PORT_PUMAS_MODEL_REF),
            WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput,
            format!(
                "upstream task '{}' has not materialized output '{}'",
                result.task_id, binding.source_port_id
            ),
        ));
    };

    match &output.value {
        WorkflowSchedulerTaskResultValue::PumasModelRef(model_ref) => {
            MaterializedModelRefResolution::Ready(model_ref.clone())
        }
        _ => MaterializedModelRefResolution::Invalid(diagnostic(
            task,
            Some(PORT_PUMAS_MODEL_REF),
            WorkflowSchedulerTaskBindingDiagnosticCode::WrongMaterializedValueType,
            format!(
                "materialized output '{}' is not a PumasModelRef",
                output.port_id
            ),
        )),
    }
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
