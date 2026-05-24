use std::collections::HashMap;

use node_engine::{NodeEngineSingleTaskError, NodeEngineSingleTaskRequest};
use serde_json::Value;
use thiserror::Error;

use super::task_graph_contracts::{
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerTask,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskInputBinding,
};
use super::task_result_contracts::{
    WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultOutput,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
    WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

const PORT_TEXT: &str = "text";
const PORT_VALUE: &str = "value";

pub(crate) async fn execute_non_runtime_scheduler_task(
    task: &WorkflowSchedulerTask,
    materialized_results: &[WorkflowSchedulerTaskResult],
) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    if task.execution_class != WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine {
        return Err(
            WorkflowSchedulerNonRuntimeTaskAdapterError::UnsupportedExecutionClass {
                node_type: task.node_type.clone(),
            },
        );
    }

    let template = task.non_runtime_task_template.as_ref().ok_or_else(|| {
        WorkflowSchedulerNonRuntimeTaskAdapterError::MissingTaskTemplate {
            task_id: task.task_id.as_str().to_string(),
        }
    })?;
    let inputs = node_engine_inputs(task, template, materialized_results)?;
    let request = NodeEngineSingleTaskRequest::try_new(
        task.task_id.as_str(),
        task.node_type.as_str(),
        inputs,
    )
    .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::NodeEngine)?;
    let response = node_engine::execute_core_task_once(request)
        .await
        .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::NodeEngine)?;
    let outputs = scheduler_outputs(template, response.outputs())?;
    let result = WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: task.workflow_id.as_str().to_string(),
        workflow_run_id: task.workflow_run_id.as_str().to_string(),
        node_id: task.node_id.as_str().to_string(),
        task_id: task.task_id.as_str().to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs,
        diagnostics: Vec::new(),
        terminal_metadata: None,
    };
    result
        .validate()
        .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidTaskResult)?;
    Ok(result)
}

fn node_engine_inputs(
    task: &WorkflowSchedulerTask,
    template: &WorkflowSchedulerNonRuntimeTaskTemplate,
    materialized_results: &[WorkflowSchedulerTaskResult],
) -> Result<HashMap<String, Value>, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    let mut inputs = HashMap::new();
    match template {
        WorkflowSchedulerNonRuntimeTaskTemplate::TextInput { value } => {
            inputs.insert(PORT_TEXT.to_string(), Value::String(value.clone()));
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::BooleanInput { value } => {
            inputs.insert(PORT_VALUE.to_string(), Value::Bool(*value));
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput => {
            inputs.insert(
                PORT_TEXT.to_string(),
                Value::String(materialized_string_input(
                    task,
                    materialized_results,
                    PORT_TEXT,
                )?),
            );
        }
    }
    Ok(inputs)
}

fn scheduler_outputs(
    template: &WorkflowSchedulerNonRuntimeTaskTemplate,
    outputs: &HashMap<String, Value>,
) -> Result<Vec<WorkflowSchedulerTaskResultOutput>, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    match template {
        WorkflowSchedulerNonRuntimeTaskTemplate::TextInput { .. }
        | WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput => {
            let value = output_string(outputs, PORT_TEXT)?;
            Ok(vec![WorkflowSchedulerTaskResultOutput {
                port_id: PORT_TEXT.to_string(),
                value: WorkflowSchedulerTaskResultValue::String(value),
            }])
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::BooleanInput { .. } => {
            let value = outputs
                .get(PORT_VALUE)
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                        port_id: PORT_VALUE.to_string(),
                        expected: "boolean",
                    }
                })?;
            Ok(vec![WorkflowSchedulerTaskResultOutput {
                port_id: PORT_VALUE.to_string(),
                value: WorkflowSchedulerTaskResultValue::Bool(value),
            }])
        }
    }
}

fn output_string(
    outputs: &HashMap<String, Value>,
    port_id: &'static str,
) -> Result<String, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    outputs
        .get(port_id)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(
            || WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                port_id: port_id.to_string(),
                expected: "string",
            },
        )
}

fn materialized_string_input(
    task: &WorkflowSchedulerTask,
    materialized_results: &[WorkflowSchedulerTaskResult],
    target_port_id: &'static str,
) -> Result<String, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    let binding = task
        .input_bindings
        .iter()
        .find(|binding| binding.target_port_id == target_port_id)
        .ok_or_else(
            || WorkflowSchedulerNonRuntimeTaskAdapterError::MissingInputBinding {
                task_id: task.task_id.as_str().to_string(),
                target_port_id: target_port_id.to_string(),
            },
        )?;
    match materialized_output(task, binding, materialized_results)? {
        WorkflowSchedulerTaskResultValue::String(value) => Ok(value.clone()),
        _ => Err(
            WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
                expected: "string",
            },
        ),
    }
}

fn materialized_output<'a>(
    task: &WorkflowSchedulerTask,
    binding: &WorkflowSchedulerTaskInputBinding,
    materialized_results: &'a [WorkflowSchedulerTaskResult],
) -> Result<&'a WorkflowSchedulerTaskResultValue, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    let result = materialized_results
        .iter()
        .find(|result| {
            result.task_id == binding.source_task_id.as_str()
                && result.node_id == binding.source_node_id.as_str()
        })
        .ok_or_else(
            || WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput {
                task_id: task.task_id.as_str().to_string(),
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
            },
        )?;
    result
        .validate()
        .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidTaskResult)?;
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => {}
        WorkflowSchedulerTaskResultStatus::Unavailable => {
            return Err(
                WorkflowSchedulerNonRuntimeTaskAdapterError::UnavailableMaterializedInput {
                    source_task_id: binding.source_task_id.as_str().to_string(),
                    source_port_id: binding.source_port_id.clone(),
                },
            );
        }
        WorkflowSchedulerTaskResultStatus::Failed | WorkflowSchedulerTaskResultStatus::Invalid => {
            return Err(
                WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidMaterializedInput {
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
            || WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput {
                task_id: task.task_id.as_str().to_string(),
                source_task_id: binding.source_task_id.as_str().to_string(),
                source_port_id: binding.source_port_id.clone(),
            },
        )
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowSchedulerNonRuntimeTaskAdapterError {
    #[error("workflow task type '{node_type}' is not a non-runtime node-engine task")]
    UnsupportedExecutionClass { node_type: String },
    #[error("scheduler task '{task_id}' is missing a typed non-runtime task template")]
    MissingTaskTemplate { task_id: String },
    #[error("scheduler task '{task_id}' is missing an input binding for '{target_port_id}'")]
    MissingInputBinding {
        task_id: String,
        target_port_id: String,
    },
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
        "materialized input '{source_task_id}.{source_port_id}' has the wrong type, expected {expected}"
    )]
    WrongMaterializedInputType {
        source_task_id: String,
        source_port_id: String,
        expected: &'static str,
    },
    #[error("node-engine output '{port_id}' has the wrong type, expected {expected}")]
    InvalidNodeEngineOutput {
        port_id: String,
        expected: &'static str,
    },
    #[error("node-engine single-task execution failed")]
    NodeEngine(NodeEngineSingleTaskError),
    #[error("scheduler task result contract validation failed: {0}")]
    InvalidTaskResult(super::task_result_contracts::WorkflowSchedulerTaskResultError),
}

#[cfg(test)]
mod tests {
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
    };

    use super::*;

    fn workflow_id() -> SchedulerWorkflowId {
        SchedulerWorkflowId::parse("workflow.non_runtime_adapter").expect("workflow id")
    }

    fn workflow_run_id() -> SchedulerWorkflowRunId {
        SchedulerWorkflowRunId::parse("run.non_runtime_adapter").expect("run id")
    }

    fn task(
        task_id: &str,
        node_type: &str,
        template: Option<WorkflowSchedulerNonRuntimeTaskTemplate>,
        input_bindings: Vec<WorkflowSchedulerTaskInputBinding>,
    ) -> WorkflowSchedulerTask {
        WorkflowSchedulerTask {
            workflow_id: workflow_id(),
            workflow_run_id: workflow_run_id(),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            node_type: node_type.to_string(),
            execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
            dependency_task_ids: input_bindings
                .iter()
                .map(|binding| binding.source_task_id.clone())
                .collect(),
            input_bindings,
            schedulable_intent: None,
            schedulable_intent_template: None,
            non_runtime_task_template: template,
            diagnostics: Vec::new(),
        }
    }

    fn completed_result(
        task_id: &str,
        port_id: &str,
        value: WorkflowSchedulerTaskResultValue,
    ) -> WorkflowSchedulerTaskResult {
        WorkflowSchedulerTaskResult {
            schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: workflow_id().as_str().to_string(),
            workflow_run_id: workflow_run_id().as_str().to_string(),
            node_id: task_id.to_string(),
            task_id: task_id.to_string(),
            status: WorkflowSchedulerTaskResultStatus::Completed,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: port_id.to_string(),
                value,
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        }
    }

    fn text_binding(source_task_id: &str) -> WorkflowSchedulerTaskInputBinding {
        WorkflowSchedulerTaskInputBinding {
            source_node_id: SchedulerNodeId::parse(source_task_id).expect("node id"),
            source_task_id: SchedulerTaskId::parse(source_task_id).expect("task id"),
            source_port_id: PORT_TEXT.to_string(),
            target_port_id: PORT_TEXT.to_string(),
        }
    }

    #[tokio::test]
    async fn adapter_executes_text_input_from_typed_template() {
        let task = task(
            "prompt",
            "text-input",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextInput {
                value: "paint a red cube".to_string(),
            }),
            Vec::new(),
        );

        let result = execute_non_runtime_scheduler_task(&task, &[])
            .await
            .expect("non-runtime result");

        assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
        assert_eq!(
            result.outputs[0].value,
            WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
        );
    }

    #[tokio::test]
    async fn adapter_executes_boolean_input_from_typed_template() {
        let task = task(
            "flag",
            "boolean-input",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::BooleanInput { value: true }),
            Vec::new(),
        );

        let result = execute_non_runtime_scheduler_task(&task, &[])
            .await
            .expect("non-runtime result");

        assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
        assert_eq!(
            result.outputs[0].value,
            WorkflowSchedulerTaskResultValue::Bool(true)
        );
    }

    #[tokio::test]
    async fn adapter_executes_text_output_from_materialized_input() {
        let task = task(
            "out",
            "text-output",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
            vec![text_binding("prompt")],
        );
        let upstream = completed_result(
            "prompt",
            PORT_TEXT,
            WorkflowSchedulerTaskResultValue::String("ready text".to_string()),
        );

        let result = execute_non_runtime_scheduler_task(&task, &[upstream])
            .await
            .expect("non-runtime result");

        assert_eq!(
            result.outputs[0].value,
            WorkflowSchedulerTaskResultValue::String("ready text".to_string())
        );
    }

    #[tokio::test]
    async fn adapter_rejects_runtime_task_before_node_engine() {
        let mut task = task("infer", "llm-inference", None, Vec::new());
        task.execution_class = WorkflowSchedulerTaskExecutionClass::RuntimeInference;

        let error = execute_non_runtime_scheduler_task(&task, &[])
            .await
            .expect_err("runtime task should be rejected");

        assert!(matches!(
            error,
            WorkflowSchedulerNonRuntimeTaskAdapterError::UnsupportedExecutionClass { .. }
        ));
    }

    #[tokio::test]
    async fn adapter_rejects_wrong_materialized_input_type() {
        let task = task(
            "out",
            "text-output",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput),
            vec![text_binding("prompt")],
        );
        let upstream = completed_result(
            "prompt",
            PORT_TEXT,
            WorkflowSchedulerTaskResultValue::Bool(true),
        );

        let error = execute_non_runtime_scheduler_task(&task, &[upstream])
            .await
            .expect_err("wrong input type should fail");

        assert!(matches!(
            error,
            WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType { .. }
        ));
    }
}
