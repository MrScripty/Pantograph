use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInput, RuntimeHostExecutionInputValue, RuntimeHostExecutionMediaArtifactRef,
};
use thiserror::Error;

use super::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskInputBinding,
    WorkflowSchedulerTaskMediaArtifactRef, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultError, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue,
};

const MODEL_REF_TARGET_PORTS: &[&str] = &["pumas_model_ref", "model_ref"];

pub(crate) fn materialize_runtime_host_inputs(
    task: &WorkflowSchedulerTask,
    materialized_results: &[WorkflowSchedulerTaskResult],
) -> Result<Vec<RuntimeHostExecutionInput>, WorkflowRuntimeHostTaskInputMappingError> {
    let mut inputs = Vec::new();
    for binding in &task.input_bindings {
        let Some(input) = materialized_runtime_host_input(task, binding, materialized_results)?
        else {
            continue;
        };
        inputs.push(input);
    }
    Ok(inputs)
}

fn materialized_runtime_host_input(
    task: &WorkflowSchedulerTask,
    binding: &WorkflowSchedulerTaskInputBinding,
    materialized_results: &[WorkflowSchedulerTaskResult],
) -> Result<Option<RuntimeHostExecutionInput>, WorkflowRuntimeHostTaskInputMappingError> {
    let value = materialized_output(task, binding, materialized_results)?;
    let Some(value) = runtime_host_input_value(binding, value)? else {
        return Ok(None);
    };
    Ok(Some(RuntimeHostExecutionInput {
        port_id: binding.target_port_id.clone(),
        value,
    }))
}

fn materialized_output<'a>(
    task: &WorkflowSchedulerTask,
    binding: &WorkflowSchedulerTaskInputBinding,
    materialized_results: &'a [WorkflowSchedulerTaskResult],
) -> Result<&'a WorkflowSchedulerTaskResultValue, WorkflowRuntimeHostTaskInputMappingError> {
    let result = materialized_results
        .iter()
        .find(|result| {
            result.task_id == binding.source_task_id.as_str()
                && result.workflow_run_id == task.workflow_run_id.as_str()
        })
        .ok_or_else(
            || WorkflowRuntimeHostTaskInputMappingError::MissingMaterializedInput {
                task_id: task.task_id.as_str().to_string(),
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
            },
        )?;
    result
        .validate()
        .map_err(WorkflowRuntimeHostTaskInputMappingError::InvalidTaskResult)?;
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => {}
        WorkflowSchedulerTaskResultStatus::Unavailable => {
            return Err(
                WorkflowRuntimeHostTaskInputMappingError::UnavailableMaterializedInput {
                    source_task_id: binding.source_task_id.as_str().to_string(),
                    source_port_id: binding.source_port_id.clone(),
                },
            );
        }
        WorkflowSchedulerTaskResultStatus::Failed | WorkflowSchedulerTaskResultStatus::Invalid => {
            return Err(
                WorkflowRuntimeHostTaskInputMappingError::InvalidMaterializedInput {
                    source_task_id: binding.source_task_id.as_str().to_string(),
                    source_port_id: binding.source_port_id.clone(),
                },
            );
        }
    }
    result
        .outputs
        .iter()
        .find(|output| output.port_id == binding.source_port_id)
        .map(|output| &output.value)
        .ok_or_else(
            || WorkflowRuntimeHostTaskInputMappingError::MissingMaterializedInput {
                task_id: task.task_id.as_str().to_string(),
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
            },
        )
}

fn runtime_host_input_value(
    binding: &WorkflowSchedulerTaskInputBinding,
    value: &WorkflowSchedulerTaskResultValue,
) -> Result<Option<RuntimeHostExecutionInputValue>, WorkflowRuntimeHostTaskInputMappingError> {
    match value {
        WorkflowSchedulerTaskResultValue::String(value) => {
            Ok(Some(RuntimeHostExecutionInputValue::String(value.clone())))
        }
        WorkflowSchedulerTaskResultValue::Bool(value) => {
            Ok(Some(RuntimeHostExecutionInputValue::Bool(*value)))
        }
        WorkflowSchedulerTaskResultValue::I64(value) => {
            Ok(Some(RuntimeHostExecutionInputValue::I64(*value)))
        }
        WorkflowSchedulerTaskResultValue::U64(value) => {
            Ok(Some(RuntimeHostExecutionInputValue::U64(*value)))
        }
        WorkflowSchedulerTaskResultValue::MediaArtifactRef(value) => Ok(Some(
            RuntimeHostExecutionInputValue::MediaArtifactRef(runtime_host_media_ref(value)),
        )),
        WorkflowSchedulerTaskResultValue::PumasModelRef(_) => {
            if MODEL_REF_TARGET_PORTS.contains(&binding.target_port_id.as_str()) {
                Ok(None)
            } else {
                Err(
                    WorkflowRuntimeHostTaskInputMappingError::UnsupportedMaterializedInput {
                        source_task_id: binding.source_task_id.as_str().to_string(),
                        source_port_id: binding.source_port_id.clone(),
                        target_port_id: binding.target_port_id.clone(),
                        value_type: "pumas_model_ref",
                    },
                )
            }
        }
        WorkflowSchedulerTaskResultValue::DiagnosticOnly => Err(
            WorkflowRuntimeHostTaskInputMappingError::UnsupportedMaterializedInput {
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
                target_port_id: binding.target_port_id.clone(),
                value_type: "diagnostic_only",
            },
        ),
    }
}

fn runtime_host_media_ref(
    value: &WorkflowSchedulerTaskMediaArtifactRef,
) -> RuntimeHostExecutionMediaArtifactRef {
    RuntimeHostExecutionMediaArtifactRef {
        artifact_id: value.artifact_id.clone(),
        media_type: value.media_type.clone(),
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowRuntimeHostTaskInputMappingError {
    #[error(
        "scheduler task '{task_id}' is missing materialized input '{source_task_id}.{source_port_id}'"
    )]
    MissingMaterializedInput {
        task_id: String,
        source_task_id: String,
        source_port_id: String,
    },
    #[error("materialized input '{source_task_id}.{source_port_id}' is unavailable")]
    UnavailableMaterializedInput {
        source_task_id: String,
        source_port_id: String,
    },
    #[error("materialized input '{source_task_id}.{source_port_id}' is invalid or failed")]
    InvalidMaterializedInput {
        source_task_id: String,
        source_port_id: String,
    },
    #[error(
        "materialized input '{source_task_id}.{source_port_id}' cannot be used for runtime-host target '{target_port_id}' because {value_type} is not a runtime input value"
    )]
    UnsupportedMaterializedInput {
        source_task_id: String,
        source_port_id: String,
        target_port_id: String,
        value_type: &'static str,
    },
    #[error("scheduler task result contract validation failed: {0}")]
    InvalidTaskResult(WorkflowSchedulerTaskResultError),
}

#[cfg(test)]
#[path = "runtime_host_task_input_mapping_tests.rs"]
mod tests;
