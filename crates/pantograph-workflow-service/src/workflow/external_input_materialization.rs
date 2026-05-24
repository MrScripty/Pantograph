use std::collections::BTreeSet;

use thiserror::Error;

use super::{
    WorkflowPortBinding, WorkflowSchedulerSourceInputTemplate, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultOutput,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
    WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

const PORT_TEXT: &str = "text";
const PORT_VALUE: &str = "value";

/// Convert request-level workflow inputs into scheduler-owned task results.
///
/// This boundary lets session execution seed source/input tasks without
/// mutating graph node data or passing raw graph/editor values deeper into the
/// scheduler task loop.
pub(crate) fn materialize_external_workflow_inputs(
    task_graph: &WorkflowSchedulerTaskGraph,
    inputs: &[WorkflowPortBinding],
) -> Result<Vec<WorkflowSchedulerTaskResult>, WorkflowExternalInputMaterializationError> {
    let mut seen_inputs = BTreeSet::<(&str, &str)>::new();
    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
        if !seen_inputs.insert((input.node_id.as_str(), input.port_id.as_str())) {
            return Err(WorkflowExternalInputMaterializationError::DuplicateInput {
                node_id: input.node_id.clone(),
                port_id: input.port_id.clone(),
            });
        }

        let task = task_graph
            .tasks
            .iter()
            .find(|task| task.node_id.as_str() == input.node_id.as_str())
            .ok_or_else(
                || WorkflowExternalInputMaterializationError::UnknownInputNode {
                    node_id: input.node_id.clone(),
                    port_id: input.port_id.clone(),
                },
            )?;

        if task.execution_class != WorkflowSchedulerTaskExecutionClass::SourceInput {
            return Err(
                WorkflowExternalInputMaterializationError::UnsupportedInputTask {
                    node_id: input.node_id.clone(),
                    port_id: input.port_id.clone(),
                    node_type: task.node_type.clone(),
                },
            );
        }

        let Some(template) = task.source_input_task_template.as_ref() else {
            return Err(
                WorkflowExternalInputMaterializationError::UnsupportedInputTask {
                    node_id: input.node_id.clone(),
                    port_id: input.port_id.clone(),
                    node_type: task.node_type.clone(),
                },
            );
        };

        let value = match (template, input.port_id.as_str()) {
            (WorkflowSchedulerSourceInputTemplate::Text { port_id }, PORT_TEXT)
                if port_id == PORT_TEXT =>
            {
                input
                    .value
                    .as_str()
                    .map(|value| WorkflowSchedulerTaskResultValue::String(value.to_string()))
                    .ok_or_else(|| {
                        WorkflowExternalInputMaterializationError::WrongInputValueType {
                            node_id: input.node_id.clone(),
                            port_id: input.port_id.clone(),
                            expected: "string",
                        }
                    })?
            }
            (WorkflowSchedulerSourceInputTemplate::Boolean { port_id }, PORT_VALUE)
                if port_id == PORT_VALUE =>
            {
                input
                    .value
                    .as_bool()
                    .map(WorkflowSchedulerTaskResultValue::Bool)
                    .ok_or_else(|| {
                        WorkflowExternalInputMaterializationError::WrongInputValueType {
                            node_id: input.node_id.clone(),
                            port_id: input.port_id.clone(),
                            expected: "boolean",
                        }
                    })?
            }
            _ => {
                return Err(
                    WorkflowExternalInputMaterializationError::UnsupportedInputTask {
                        node_id: input.node_id.clone(),
                        port_id: input.port_id.clone(),
                        node_type: task.node_type.clone(),
                    },
                );
            }
        };

        let result = WorkflowSchedulerTaskResult {
            schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: task_graph.workflow_id.as_str().to_string(),
            workflow_run_id: task_graph.workflow_run_id.as_str().to_string(),
            node_id: task.node_id.as_str().to_string(),
            task_id: task.task_id.as_str().to_string(),
            status: WorkflowSchedulerTaskResultStatus::Completed,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: input.port_id.clone(),
                value,
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        };
        result
            .validate()
            .map_err(WorkflowExternalInputMaterializationError::InvalidTaskResult)?;
        results.push(result);
    }

    Ok(results)
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowExternalInputMaterializationError {
    #[error("workflow input '{node_id}.{port_id}' does not match a scheduler task")]
    UnknownInputNode { node_id: String, port_id: String },
    #[error("workflow input '{node_id}.{port_id}' was provided more than once")]
    DuplicateInput { node_id: String, port_id: String },
    #[error("workflow input '{node_id}.{port_id}' is not supported for task type '{node_type}'")]
    UnsupportedInputTask {
        node_id: String,
        port_id: String,
        node_type: String,
    },
    #[error("workflow input '{node_id}.{port_id}' must be a {expected}")]
    WrongInputValueType {
        node_id: String,
        port_id: String,
        expected: &'static str,
    },
    #[error("materialized workflow input produced an invalid scheduler task result")]
    InvalidTaskResult(super::WorkflowSchedulerTaskResultError),
}

#[cfg(test)]
mod tests {
    use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
    use serde_json::json;

    use super::*;
    use crate::graph::{GraphNode, Position, WorkflowGraph};
    use crate::workflow::workflow_scheduler_task_graph;

    fn workflow_id() -> WorkflowId {
        WorkflowId::try_from("workflow-external-inputs".to_string()).expect("workflow id")
    }

    fn workflow_run_id() -> WorkflowRunId {
        WorkflowRunId::try_from("run-external-inputs".to_string()).expect("workflow run id")
    }

    fn graph_with_external_inputs() -> WorkflowGraph {
        WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "prompt".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: json!({}),
                },
                GraphNode {
                    id: "flag".to_string(),
                    node_type: "boolean-input".to_string(),
                    position: Position { x: 100.0, y: 0.0 },
                    data: json!({}),
                },
                GraphNode {
                    id: "out".to_string(),
                    node_type: "text-output".to_string(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: json!({}),
                },
            ],
            edges: Vec::new(),
            derived_graph: None,
        }
    }

    fn task_graph() -> WorkflowSchedulerTaskGraph {
        workflow_scheduler_task_graph(
            &workflow_id(),
            &workflow_run_id(),
            &graph_with_external_inputs(),
        )
        .expect("scheduler task graph")
    }

    #[test]
    fn materializes_text_and_boolean_inputs_as_task_results() {
        let task_graph = task_graph();

        let results = materialize_external_workflow_inputs(
            &task_graph,
            &[
                WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: json!("paint a red cube"),
                },
                WorkflowPortBinding {
                    node_id: "flag".to_string(),
                    port_id: "value".to_string(),
                    value: json!(true),
                },
            ],
        )
        .expect("materialized inputs");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].workflow_id, "workflow-external-inputs");
        assert_eq!(results[0].workflow_run_id, "run-external-inputs");
        assert_eq!(results[0].node_id, "prompt");
        assert_eq!(results[0].task_id, "prompt");
        assert_eq!(
            results[0].status,
            WorkflowSchedulerTaskResultStatus::Completed
        );
        assert_eq!(
            results[0].outputs[0].value,
            WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string())
        );
        assert_eq!(
            results[1].outputs[0].value,
            WorkflowSchedulerTaskResultValue::Bool(true)
        );
    }

    #[test]
    fn rejects_duplicate_external_input() {
        let error = materialize_external_workflow_inputs(
            &task_graph(),
            &[
                WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: json!("one"),
                },
                WorkflowPortBinding {
                    node_id: "prompt".to_string(),
                    port_id: "text".to_string(),
                    value: json!("two"),
                },
            ],
        )
        .expect_err("duplicate input fails");

        assert!(matches!(
            error,
            WorkflowExternalInputMaterializationError::DuplicateInput { .. }
        ));
    }

    #[test]
    fn rejects_wrong_external_input_value_type() {
        let error = materialize_external_workflow_inputs(
            &task_graph(),
            &[WorkflowPortBinding {
                node_id: "prompt".to_string(),
                port_id: "text".to_string(),
                value: json!(true),
            }],
        )
        .expect_err("wrong value type fails");

        assert!(matches!(
            error,
            WorkflowExternalInputMaterializationError::WrongInputValueType {
                expected: "string",
                ..
            }
        ));
    }

    #[test]
    fn rejects_unknown_external_input_node() {
        let error = materialize_external_workflow_inputs(
            &task_graph(),
            &[WorkflowPortBinding {
                node_id: "missing".to_string(),
                port_id: "text".to_string(),
                value: json!("text"),
            }],
        )
        .expect_err("unknown node fails");

        assert!(matches!(
            error,
            WorkflowExternalInputMaterializationError::UnknownInputNode { .. }
        ));
    }

    #[test]
    fn rejects_non_source_external_input_task() {
        let error = materialize_external_workflow_inputs(
            &task_graph(),
            &[WorkflowPortBinding {
                node_id: "out".to_string(),
                port_id: "text".to_string(),
                value: json!("text"),
            }],
        )
        .expect_err("unsupported task fails");

        assert!(matches!(
            error,
            WorkflowExternalInputMaterializationError::UnsupportedInputTask { .. }
        ));
    }
}
