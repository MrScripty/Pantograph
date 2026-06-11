use serde_json::Value;
use thiserror::Error;

use super::{
    WorkflowOutputTarget, WorkflowPortBinding, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskMediaArtifactRef, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
};

/// Project completed scheduler task results into requested workflow outputs.
pub(crate) fn project_scheduler_task_results_to_outputs(
    task_graph: &WorkflowSchedulerTaskGraph,
    results: &[WorkflowSchedulerTaskResult],
    output_targets: &[WorkflowOutputTarget],
) -> Result<Vec<WorkflowPortBinding>, WorkflowSchedulerTaskOutputProjectionError> {
    output_targets
        .iter()
        .map(|target| project_output_target(task_graph, results, target))
        .collect()
}

fn project_output_target(
    task_graph: &WorkflowSchedulerTaskGraph,
    results: &[WorkflowSchedulerTaskResult],
    target: &WorkflowOutputTarget,
) -> Result<WorkflowPortBinding, WorkflowSchedulerTaskOutputProjectionError> {
    let task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == target.node_id.as_str())
        .ok_or_else(
            || WorkflowSchedulerTaskOutputProjectionError::UnknownOutputNode {
                node_id: target.node_id.clone(),
                port_id: target.port_id.clone(),
            },
        )?;

    let mut matching_results = results.iter().filter(|result| {
        result.workflow_id == task_graph.workflow_id.as_str()
            && result.workflow_run_id == task_graph.workflow_run_id.as_str()
            && result.node_id == task.node_id.as_str()
            && result.task_id == task.task_id.as_str()
            && result
                .outputs
                .iter()
                .any(|output| output.port_id == target.port_id)
    });
    let result = matching_results.next().ok_or_else(|| {
        WorkflowSchedulerTaskOutputProjectionError::MissingOutput {
            node_id: target.node_id.clone(),
            port_id: target.port_id.clone(),
        }
    })?;
    if matching_results.next().is_some() {
        return Err(
            WorkflowSchedulerTaskOutputProjectionError::AmbiguousOutput {
                node_id: target.node_id.clone(),
                port_id: target.port_id.clone(),
            },
        );
    }

    if result.status != WorkflowSchedulerTaskResultStatus::Completed {
        return Err(
            WorkflowSchedulerTaskOutputProjectionError::NonCompletedOutput {
                node_id: target.node_id.clone(),
                port_id: target.port_id.clone(),
                status: result.status,
            },
        );
    }
    result
        .validate()
        .map_err(WorkflowSchedulerTaskOutputProjectionError::InvalidTaskResult)?;

    let output = result
        .outputs
        .iter()
        .find(|output| output.port_id == target.port_id)
        .ok_or_else(
            || WorkflowSchedulerTaskOutputProjectionError::MissingOutput {
                node_id: target.node_id.clone(),
                port_id: target.port_id.clone(),
            },
        )?;
    Ok(WorkflowPortBinding {
        node_id: target.node_id.clone(),
        port_id: target.port_id.clone(),
        value: task_result_value_to_workflow_output(&output.value).map_err(|kind| {
            WorkflowSchedulerTaskOutputProjectionError::UnsupportedOutputValue {
                node_id: target.node_id.clone(),
                port_id: target.port_id.clone(),
                value_kind: kind,
            }
        })?,
    })
}

fn task_result_value_to_workflow_output(
    value: &WorkflowSchedulerTaskResultValue,
) -> Result<Value, &'static str> {
    match value {
        WorkflowSchedulerTaskResultValue::String(value) => Ok(Value::String(value.clone())),
        WorkflowSchedulerTaskResultValue::Bool(value) => Ok(Value::Bool(*value)),
        WorkflowSchedulerTaskResultValue::I64(value) => Ok(Value::Number((*value).into())),
        WorkflowSchedulerTaskResultValue::U64(value) => Ok(Value::Number((*value).into())),
        WorkflowSchedulerTaskResultValue::MediaArtifactRef(value) => media_artifact_ref_json(value),
        WorkflowSchedulerTaskResultValue::PumasModelRef(_) => Err("pumas_model_ref"),
        WorkflowSchedulerTaskResultValue::DiagnosticOnly => Err("diagnostic_only"),
    }
}

fn media_artifact_ref_json(
    value: &WorkflowSchedulerTaskMediaArtifactRef,
) -> Result<Value, &'static str> {
    serde_json::to_value(value).map_err(|_| "media_artifact_ref")
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowSchedulerTaskOutputProjectionError {
    #[error("requested output '{node_id}.{port_id}' does not match a scheduler task")]
    UnknownOutputNode { node_id: String, port_id: String },
    #[error("requested output '{node_id}.{port_id}' was not produced by scheduler task results")]
    MissingOutput { node_id: String, port_id: String },
    #[error("requested output '{node_id}.{port_id}' has more than one producer result")]
    AmbiguousOutput { node_id: String, port_id: String },
    #[error("requested output '{node_id}.{port_id}' has non-completed task status {status:?}")]
    NonCompletedOutput {
        node_id: String,
        port_id: String,
        status: WorkflowSchedulerTaskResultStatus,
    },
    #[error("requested output '{node_id}.{port_id}' has unsupported value kind {value_kind}")]
    UnsupportedOutputValue {
        node_id: String,
        port_id: String,
        value_kind: &'static str,
    },
    #[error("scheduler task output result is invalid")]
    InvalidTaskResult(super::WorkflowSchedulerTaskResultError),
}

#[cfg(test)]
mod tests {
    use pantograph_dependency_planning::PumasModelRef;
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
    };
    use serde_json::json;

    use super::*;
    use crate::workflow::{
        WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
        WorkflowSchedulerTaskResultOutput, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
        WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
    };

    fn task_graph() -> WorkflowSchedulerTaskGraph {
        let workflow_id =
            SchedulerWorkflowId::parse("workflow-output-projection").expect("workflow id");
        let workflow_run_id =
            SchedulerWorkflowRunId::parse("run-output-projection").expect("run id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: vec![WorkflowSchedulerTask {
                workflow_id,
                workflow_run_id,
                node_id: SchedulerNodeId::parse("out").expect("node id"),
                task_id: SchedulerTaskId::parse("out").expect("task id"),
                node_type: "text-output".to_string(),
                execution_class: WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
                dependency_task_ids: Vec::new(),
                input_bindings: Vec::new(),
                schedulable_intent: None,
                schedulable_intent_template: None,
                non_runtime_task_template: None,
                source_input_task_template: None,
                inference_descriptor_fingerprint: None,
                runtime_source_context: None,
                diagnostics: Vec::new(),
            }],
        }
    }

    fn target() -> WorkflowOutputTarget {
        WorkflowOutputTarget {
            node_id: "out".to_string(),
            port_id: "text".to_string(),
        }
    }

    fn result(
        status: WorkflowSchedulerTaskResultStatus,
        value: WorkflowSchedulerTaskResultValue,
    ) -> WorkflowSchedulerTaskResult {
        WorkflowSchedulerTaskResult {
            schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
            workflow_id: "workflow-output-projection".to_string(),
            workflow_run_id: "run-output-projection".to_string(),
            node_id: "out".to_string(),
            task_id: "out".to_string(),
            status,
            outputs: vec![WorkflowSchedulerTaskResultOutput {
                port_id: "text".to_string(),
                value,
            }],
            diagnostics: Vec::new(),
            terminal_metadata: None,
        }
    }

    #[test]
    fn projects_completed_scalar_result_to_workflow_output() {
        let outputs = project_scheduler_task_results_to_outputs(
            &task_graph(),
            &[result(
                WorkflowSchedulerTaskResultStatus::Completed,
                WorkflowSchedulerTaskResultValue::String("ready".to_string()),
            )],
            &[target()],
        )
        .expect("outputs");

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].node_id, "out");
        assert_eq!(outputs[0].port_id, "text");
        assert_eq!(outputs[0].value, json!("ready"));
    }

    #[test]
    fn rejects_missing_requested_output() {
        let error = project_scheduler_task_results_to_outputs(&task_graph(), &[], &[target()])
            .expect_err("missing output");

        assert!(matches!(
            error,
            WorkflowSchedulerTaskOutputProjectionError::MissingOutput { .. }
        ));
    }

    #[test]
    fn rejects_non_completed_result_output() {
        let error = project_scheduler_task_results_to_outputs(
            &task_graph(),
            &[result(
                WorkflowSchedulerTaskResultStatus::Failed,
                WorkflowSchedulerTaskResultValue::String("failed".to_string()),
            )],
            &[target()],
        )
        .expect_err("non-completed output");

        assert!(matches!(
            error,
            WorkflowSchedulerTaskOutputProjectionError::NonCompletedOutput {
                status: WorkflowSchedulerTaskResultStatus::Failed,
                ..
            }
        ));
    }

    #[test]
    fn rejects_unsupported_task_result_value() {
        let error = project_scheduler_task_results_to_outputs(
            &task_graph(),
            &[result(
                WorkflowSchedulerTaskResultStatus::Completed,
                WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
                    model_id: "image/example/tiny-diffusion".to_string(),
                    revision: None,
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                }),
            )],
            &[target()],
        )
        .expect_err("unsupported output");

        assert!(matches!(
            error,
            WorkflowSchedulerTaskOutputProjectionError::UnsupportedOutputValue {
                value_kind: "pumas_model_ref",
                ..
            }
        ));
    }

    #[test]
    fn rejects_ambiguous_producer_results() {
        let task_graph = task_graph();
        let first = result(
            WorkflowSchedulerTaskResultStatus::Completed,
            WorkflowSchedulerTaskResultValue::String("one".to_string()),
        );
        let second = result(
            WorkflowSchedulerTaskResultStatus::Completed,
            WorkflowSchedulerTaskResultValue::String("two".to_string()),
        );

        let error =
            project_scheduler_task_results_to_outputs(&task_graph, &[first, second], &[target()])
                .expect_err("ambiguous output");

        assert!(matches!(
            error,
            WorkflowSchedulerTaskOutputProjectionError::AmbiguousOutput { .. }
        ));
    }
}
