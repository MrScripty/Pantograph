use std::collections::HashMap;

use node_engine::{NodeEngineSingleTaskError, NodeEngineSingleTaskRequest};
use serde_json::Value;
use thiserror::Error;

use super::task_graph_contracts::{
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerTask,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskInputBinding,
};
use super::task_result_contracts::{
    WorkflowSchedulerTaskMediaArtifactRef, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue, WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

const PORT_TEXT: &str = "text";
const PORT_VECTOR: &str = "vector";
const VECTOR_EXPECTED: &str = "a finite numeric JSON vector of 1..4096 elements, at most 64 KiB";

pub(crate) fn is_bounded_vector_json(value: &Value) -> bool {
    value.as_array().is_some_and(|values| {
        (1..=4096).contains(&values.len())
            && values.iter().all(|value| value.as_f64().is_some_and(f64::is_finite))
            && serde_json::to_vec(value).is_ok_and(|bytes| {
                bytes.len() <= pantograph_runtime_host_contracts::RUNTIME_HOST_STRUCTURED_OUTPUT_MAX_BYTES
            })
    })
}

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
    let mut inputs = node_engine_inputs(task, template, materialized_results)?;
    let outputs = if matches!(
        template,
        WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput
    ) {
        // The generic core vector sink normalizes through f64. The scheduler
        // boundary instead retains the already validated host JSON exactly.
        let node_outputs = HashMap::from([(
            PORT_VECTOR.to_string(),
            inputs.remove(PORT_VECTOR).unwrap_or(Value::Null),
        )]);
        scheduler_outputs(template, &node_outputs)?
    } else {
        let request = NodeEngineSingleTaskRequest::try_new(
            task.task_id.as_str(),
            task.node_type.as_str(),
            inputs,
        )
        .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::NodeEngine)?;
        let response = node_engine::execute_core_task_once(request)
            .await
            .map_err(WorkflowSchedulerNonRuntimeTaskAdapterError::NodeEngine)?;
        scheduler_outputs(template, response.outputs())?
    };
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
        WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput => {
            if task.node_type != "vector-output"
                || task.input_bindings.len() > 1
                || task
                    .input_bindings
                    .iter()
                    .any(|binding| binding.target_port_id != PORT_VECTOR)
            {
                return Err(
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidVectorBinding {
                        task_id: task.task_id.as_str().to_string(),
                    },
                );
            }
            if let Some(binding) = task.input_bindings.first() {
                let WorkflowSchedulerTaskResultValue::Json(value) =
                    materialized_output(task, binding, materialized_results)?
                else {
                    return Err(
                        WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                            source_task_id: binding.source_task_id.as_str().to_string(),
                            source_port_id: binding.source_port_id.clone(),
                            expected: VECTOR_EXPECTED,
                        },
                    );
                };
                if !is_bounded_vector_json(value) {
                    return Err(
                        WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                            source_task_id: binding.source_task_id.as_str().to_string(),
                            source_port_id: binding.source_port_id.clone(),
                            expected: VECTOR_EXPECTED,
                        },
                    );
                }
                inputs.insert(PORT_VECTOR.to_string(), value.clone());
            }
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::ImageOutput => {
            let binding = task
                .input_bindings
                .iter()
                .find(|binding| binding.target_port_id == "image")
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::MissingInputBinding {
                        task_id: task.task_id.as_str().to_string(),
                        target_port_id: "image".into(),
                    }
                })?;
            let WorkflowSchedulerTaskResultValue::MediaArtifactRef(value) =
                materialized_output(task, binding, materialized_results)?
            else {
                return Err(
                    WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                        source_task_id: binding.source_task_id.as_str().to_string(),
                        source_port_id: binding.source_port_id.clone(),
                        expected: "typed media artifact reference",
                    },
                );
            };
            inputs.insert(
                "image".into(),
                serde_json::to_value(value).map_err(
                    WorkflowSchedulerNonRuntimeTaskAdapterError::MediaArtifactSerialization,
                )?,
            );
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::JsonFilter { path } => {
            let binding = task
                .input_bindings
                .iter()
                .find(|binding| binding.target_port_id == "json")
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::MissingInputBinding {
                        task_id: task.task_id.as_str().to_string(),
                        target_port_id: "json".into(),
                    }
                })?;
            let value = materialized_output(task, binding, materialized_results)?
                .node_json()
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                        source_task_id: binding.source_task_id.as_str().to_string(),
                        source_port_id: binding.source_port_id.clone(),
                        expected: "JSON-compatible value",
                    }
                })?;
            inputs.insert("json".into(), value);
            inputs.insert("_data".into(), serde_json::json!({ "path": path }));
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
        WorkflowSchedulerNonRuntimeTaskTemplate::Merge => {
            let values = task.input_bindings.iter().map(|binding| {
                match materialized_output(task, binding, materialized_results)? {
                    WorkflowSchedulerTaskResultValue::String(value) => Ok(Value::String(value.clone())),
                    _ => Err(WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType {
                        source_task_id: binding.source_task_id.as_str().to_string(),
                        source_port_id: binding.source_port_id.clone(),
                        expected: "string",
                    }),
                }
            }).collect::<Result<Vec<_>, _>>()?;
            inputs.insert("inputs".to_string(), Value::Array(values));
        }
    }
    Ok(inputs)
}

fn scheduler_outputs(
    template: &WorkflowSchedulerNonRuntimeTaskTemplate,
    outputs: &HashMap<String, Value>,
) -> Result<Vec<WorkflowSchedulerTaskResultOutput>, WorkflowSchedulerNonRuntimeTaskAdapterError> {
    match template {
        WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput => {
            let value = outputs
                .get(PORT_VECTOR)
                .filter(|value| value.is_null() || is_bounded_vector_json(value))
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                        port_id: PORT_VECTOR.to_string(),
                        expected: VECTOR_EXPECTED,
                    }
                })?;
            Ok(vec![WorkflowSchedulerTaskResultOutput {
                port_id: PORT_VECTOR.to_string(),
                value: WorkflowSchedulerTaskResultValue::Json(value.clone()),
            }])
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::ImageOutput => {
            let value = outputs
                .get("image")
                .cloned()
                .and_then(|value| {
                    serde_json::from_value::<WorkflowSchedulerTaskMediaArtifactRef>(value).ok()
                })
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                        port_id: "image".into(),
                        expected: "media artifact reference",
                    }
                })?;
            Ok(vec![WorkflowSchedulerTaskResultOutput {
                port_id: "image".into(),
                value: WorkflowSchedulerTaskResultValue::MediaArtifactRef(value),
            }])
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::JsonFilter { .. } => {
            let value = outputs.get("value").cloned().ok_or_else(|| {
                WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                    port_id: "value".into(),
                    expected: "JSON value",
                }
            })?;
            let found = outputs
                .get("found")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                        port_id: "found".into(),
                        expected: "boolean",
                    }
                })?;
            Ok(vec![
                WorkflowSchedulerTaskResultOutput {
                    port_id: "value".into(),
                    value: WorkflowSchedulerTaskResultValue::from_node_json(value),
                },
                WorkflowSchedulerTaskResultOutput {
                    port_id: "found".into(),
                    value: WorkflowSchedulerTaskResultValue::Bool(found),
                },
            ])
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput => {
            let value = output_string(outputs, PORT_TEXT)?;
            Ok(vec![WorkflowSchedulerTaskResultOutput {
                port_id: PORT_TEXT.to_string(),
                value: WorkflowSchedulerTaskResultValue::String(value),
            }])
        }
        WorkflowSchedulerNonRuntimeTaskTemplate::Merge => {
            let merged = output_string(outputs, "merged")?;
            let count = outputs
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput {
                        port_id: "count".to_string(),
                        expected: "unsigned integer",
                    }
                })?;
            Ok(vec![
                WorkflowSchedulerTaskResultOutput {
                    port_id: "merged".to_string(),
                    value: WorkflowSchedulerTaskResultValue::String(merged),
                },
                WorkflowSchedulerTaskResultOutput {
                    port_id: "count".to_string(),
                    value: WorkflowSchedulerTaskResultValue::U64(count),
                },
            ])
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
                && (!matches!(
                    task.non_runtime_task_template,
                    Some(WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput)
                ) || (result.workflow_id == task.workflow_id.as_str()
                    && result.workflow_run_id == task.workflow_run_id.as_str()))
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
    #[error("scheduler vector-output task '{task_id}' requires its own template and at most one vector input binding")]
    InvalidVectorBinding { task_id: String },
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
    #[error("media artifact reference serialization failed")]
    MediaArtifactSerialization(#[source] serde_json::Error),
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

    use super::super::WorkflowSchedulerSourceInputTemplate;
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
            source_input_task_template: None,
            inference_descriptor_fingerprint: None,
            runtime_source_context: None,
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

    fn vector_task(connected: bool) -> WorkflowSchedulerTask {
        let mut binding = text_binding("embed");
        binding.source_port_id = PORT_VECTOR.to_string();
        binding.target_port_id = PORT_VECTOR.to_string();
        task(
            "vector-out",
            "vector-output",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput),
            if connected { vec![binding] } else { Vec::new() },
        )
    }

    #[test]
    fn vector_json_enforces_exact_structured_output_byte_boundary() {
        let mut values = vec![Value::from(999_999_999_999_999_u64); 4096];
        values[4095] = Value::from(99_999_999_999_999_u64);
        let at_limit = Value::Array(values.clone());
        assert_eq!(serde_json::to_vec(&at_limit).unwrap().len(), 64 * 1024);
        assert!(is_bounded_vector_json(&at_limit));
        values[4095] = Value::from(999_999_999_999_999_u64);
        let over_limit = Value::Array(values);
        assert_eq!(
            serde_json::to_vec(&over_limit).unwrap().len(),
            64 * 1024 + 1
        );
        assert!(!is_bounded_vector_json(&over_limit));
    }

    #[tokio::test]
    async fn vector_adapter_preserves_exact_json_and_sink_identity() {
        for vector in [
            serde_json::json!([0, -1, 0.125, -0.0, u64::MAX]),
            serde_json::json!(vec![0.25; 4096]),
        ] {
            let task = vector_task(true);
            let upstream = completed_result(
                "embed",
                PORT_VECTOR,
                WorkflowSchedulerTaskResultValue::Json(vector.clone()),
            );
            let encoded = serde_json::to_string(&vector).unwrap();
            let result = execute_non_runtime_scheduler_task(&task, &[upstream])
                .await
                .unwrap();
            assert_eq!(result.workflow_id, task.workflow_id.as_str());
            assert_eq!(result.workflow_run_id, task.workflow_run_id.as_str());
            assert_eq!(result.task_id, task.task_id.as_str());
            assert_eq!(result.node_id, task.node_id.as_str());
            assert_eq!(result.status, WorkflowSchedulerTaskResultStatus::Completed);
            assert_eq!(result.outputs[0].port_id, PORT_VECTOR);
            assert_eq!(
                result.outputs[0].value,
                WorkflowSchedulerTaskResultValue::Json(vector.clone())
            );
            assert_eq!(
                serde_json::to_string(&result.outputs[0].value.node_json().unwrap()).unwrap(),
                encoded
            );
        }
    }

    #[tokio::test]
    async fn vector_adapter_retains_optional_unconnected_null_only() {
        let result = execute_non_runtime_scheduler_task(&vector_task(false), &[])
            .await
            .unwrap();
        assert_eq!(
            result.outputs[0].value,
            WorkflowSchedulerTaskResultValue::Json(Value::Null)
        );
        assert!(matches!(
            execute_non_runtime_scheduler_task(&vector_task(true), &[]).await,
            Err(WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput { .. })
        ));
    }

    #[tokio::test]
    async fn vector_adapter_refuses_connected_invalid_vectors_without_coercion() {
        for value in [
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!([])),
            WorkflowSchedulerTaskResultValue::Json(Value::Null),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!(["0.5"])),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!([true])),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!([[0.5]])),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!([null])),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!(vec![0; 4097])),
            WorkflowSchedulerTaskResultValue::Json(serde_json::json!(vec![u64::MAX; 4096])),
            WorkflowSchedulerTaskResultValue::String("[0.5]".into()),
            WorkflowSchedulerTaskResultValue::Bool(true),
        ] {
            let upstream = completed_result("embed", PORT_VECTOR, value);
            assert!(
                execute_non_runtime_scheduler_task(&vector_task(true), &[upstream])
                    .await
                    .is_err()
            );
        }
    }

    #[tokio::test]
    async fn vector_adapter_requires_same_workflow_run_and_source_identity() {
        for field in ["workflow_id", "workflow_run_id", "node_id", "task_id"] {
            let mut upstream = completed_result(
                "embed",
                PORT_VECTOR,
                WorkflowSchedulerTaskResultValue::Json(serde_json::json!([0.5])),
            );
            match field {
                "workflow_id" => upstream.workflow_id = "other-workflow".into(),
                "workflow_run_id" => upstream.workflow_run_id = "other-run".into(),
                "node_id" => upstream.node_id = "other-node".into(),
                "task_id" => upstream.task_id = "other-task".into(),
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    execute_non_runtime_scheduler_task(&vector_task(true), &[upstream]).await,
                    Err(
                        WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput { .. }
                    )
                ),
                "{field}"
            );
        }
    }

    #[tokio::test]
    async fn adapter_rejects_source_input_task_before_node_engine() {
        let mut task = task("prompt", "text-input", None, Vec::new());
        task.execution_class = WorkflowSchedulerTaskExecutionClass::SourceInput;
        task.source_input_task_template = Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: PORT_TEXT.to_string(),
        });

        let error = execute_non_runtime_scheduler_task(&task, &[])
            .await
            .expect_err("source input task should be rejected");

        assert!(matches!(
            error,
            WorkflowSchedulerNonRuntimeTaskAdapterError::UnsupportedExecutionClass { .. }
        ));
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
    async fn image_output_adapter_preserves_typed_reference_and_rejects_untyped_inputs() {
        let mut binding = text_binding("infer");
        binding.source_port_id = "image".into();
        binding.target_port_id = "image".into();
        let image_output = task(
            "out",
            "image-output",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::ImageOutput),
            vec![binding],
        );
        for media_type in [None, Some("image/png".to_string())] {
            let value = WorkflowSchedulerTaskResultValue::MediaArtifactRef(
                WorkflowSchedulerTaskMediaArtifactRef {
                    artifact_id: "image.result.001".into(),
                    media_type,
                },
            );
            let result = execute_non_runtime_scheduler_task(
                &image_output,
                &[completed_result("infer", "image", value.clone())],
            )
            .await
            .unwrap();
            assert_eq!(
                result.outputs,
                [WorkflowSchedulerTaskResultOutput {
                    port_id: "image".into(),
                    value,
                }]
            );
        }
        assert!(matches!(
            execute_non_runtime_scheduler_task(&image_output, &[]).await,
            Err(WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput { .. })
        ));
        for value in [
            WorkflowSchedulerTaskResultValue::String("artifact://image.result.001".into()),
            WorkflowSchedulerTaskResultValue::Json(
                serde_json::json!({"artifact_id": "image.result.001"}),
            ),
        ] {
            assert!(matches!(
                execute_non_runtime_scheduler_task(
                    &image_output,
                    &[completed_result("infer", "image", value)]
                )
                .await,
                Err(WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType { .. })
            ));
        }
        assert!(matches!(
            scheduler_outputs(
                &WorkflowSchedulerNonRuntimeTaskTemplate::ImageOutput,
                &HashMap::from([(
                    "image".into(),
                    serde_json::json!({"artifact_id": "image.result.001", "path": "/tmp/raw"})
                )])
            ),
            Err(WorkflowSchedulerNonRuntimeTaskAdapterError::InvalidNodeEngineOutput { .. })
        ));
    }

    #[tokio::test]
    async fn json_filter_adapter_retains_typed_value_and_found_without_rewriting_content() {
        let mut binding = text_binding("source");
        binding.source_port_id = "value".into();
        binding.target_port_id = "json".into();
        let upstream = completed_result(
            "source",
            "value",
            WorkflowSchedulerTaskResultValue::Json(
                serde_json::json!({"items": [{"prompt": "  red cube\n", "seed": u64::MAX}]}),
            ),
        );
        for (path, expected, found) in [
            (
                "items[0].prompt",
                WorkflowSchedulerTaskResultValue::String("  red cube\n".into()),
                true,
            ),
            (
                "items[0].seed",
                WorkflowSchedulerTaskResultValue::U64(u64::MAX),
                true,
            ),
            (
                "items[1].prompt",
                WorkflowSchedulerTaskResultValue::Json(Value::Null),
                false,
            ),
        ] {
            let filter = task(
                "filter",
                "json-filter",
                Some(WorkflowSchedulerNonRuntimeTaskTemplate::JsonFilter { path: path.into() }),
                vec![binding.clone()],
            );
            let result =
                execute_non_runtime_scheduler_task(&filter, std::slice::from_ref(&upstream))
                    .await
                    .unwrap();
            assert_eq!(
                result.outputs,
                [
                    WorkflowSchedulerTaskResultOutput {
                        port_id: "value".into(),
                        value: expected
                    },
                    WorkflowSchedulerTaskResultOutput {
                        port_id: "found".into(),
                        value: WorkflowSchedulerTaskResultValue::Bool(found)
                    },
                ]
            );
        }
    }

    #[tokio::test]
    async fn merge_adapter_retains_order_count_and_rejects_incomplete_or_wrong_inputs() {
        let bindings = ["a", "b"]
            .map(|id| {
                let mut binding = text_binding(id);
                binding.target_port_id = "inputs".into();
                binding
            })
            .to_vec();
        let merge = task(
            "join",
            "merge",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::Merge),
            bindings,
        );
        let a = completed_result(
            "a",
            PORT_TEXT,
            WorkflowSchedulerTaskResultValue::String("  first ".into()),
        );
        let b = completed_result(
            "b",
            PORT_TEXT,
            WorkflowSchedulerTaskResultValue::String("second".into()),
        );
        let result = execute_non_runtime_scheduler_task(&merge, &[b.clone(), a.clone()])
            .await
            .unwrap();
        assert_eq!(
            result.outputs,
            [
                WorkflowSchedulerTaskResultOutput {
                    port_id: "merged".into(),
                    value: WorkflowSchedulerTaskResultValue::String("  first \nsecond".into())
                },
                WorkflowSchedulerTaskResultOutput {
                    port_id: "count".into(),
                    value: WorkflowSchedulerTaskResultValue::U64(2)
                },
            ]
        );
        assert!(matches!(
            execute_non_runtime_scheduler_task(&merge, std::slice::from_ref(&a)).await,
            Err(WorkflowSchedulerNonRuntimeTaskAdapterError::MissingMaterializedInput { .. })
        ));
        let mut wrong = b;
        wrong.outputs[0].value = WorkflowSchedulerTaskResultValue::Bool(true);
        assert!(matches!(
            execute_non_runtime_scheduler_task(&merge, &[a, wrong]).await,
            Err(WorkflowSchedulerNonRuntimeTaskAdapterError::WrongMaterializedInputType { .. })
        ));
        let empty = task(
            "empty",
            "merge",
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::Merge),
            vec![],
        );
        let result = execute_non_runtime_scheduler_task(&empty, &[])
            .await
            .unwrap();
        assert_eq!(
            result.outputs[0].value,
            WorkflowSchedulerTaskResultValue::String(String::new())
        );
        assert_eq!(
            result.outputs[1].value,
            WorkflowSchedulerTaskResultValue::U64(0)
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
