use std::collections::BTreeSet;

use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord};
use thiserror::Error;

use super::{WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph};

/// Class summary used by session execution before runtime admission/loading.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub(crate) struct WorkflowSchedulerTaskRunSummary {
    pub runtime_inference_tasks: usize,
    pub source_input_tasks: usize,
    pub non_runtime_node_engine_tasks: usize,
    pub pumas_materialization_tasks: usize,
    pub unsupported_tasks: usize,
    pub invalid_task_states: usize,
}

impl WorkflowSchedulerTaskRunSummary {
    pub(crate) fn has_runtime_inference(&self) -> bool {
        self.runtime_inference_tasks > 0
    }

    pub(crate) fn is_non_runtime_only(&self) -> bool {
        (self.source_input_tasks > 0 || self.non_runtime_node_engine_tasks > 0)
            && self.runtime_inference_tasks == 0
            && self.pumas_materialization_tasks == 0
            && self.unsupported_tasks == 0
            && self.invalid_task_states == 0
    }
}

pub(crate) fn workflow_scheduler_task_run_summary(
    task_graph: &WorkflowSchedulerTaskGraph,
    records: &[SchedulerTaskStateRecord],
) -> Result<WorkflowSchedulerTaskRunSummary, WorkflowSchedulerTaskRunSummaryError> {
    let mut record_task_ids = records
        .iter()
        .map(|record| record.task_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut summary = WorkflowSchedulerTaskRunSummary {
        runtime_inference_tasks: 0,
        source_input_tasks: 0,
        non_runtime_node_engine_tasks: 0,
        pumas_materialization_tasks: 0,
        unsupported_tasks: 0,
        invalid_task_states: 0,
    };

    for task in &task_graph.tasks {
        let task_id = task.task_id.as_str();
        let Some(record) = records
            .iter()
            .find(|record| record.task_id.as_str() == task_id)
        else {
            return Err(WorkflowSchedulerTaskRunSummaryError::MissingTaskState {
                task_id: task_id.to_string(),
            });
        };
        if record.workflow_id.as_str() != task.workflow_id.as_str()
            || record.workflow_run_id.as_str() != task.workflow_run_id.as_str()
            || record.node_id.as_str() != task.node_id.as_str()
        {
            return Err(WorkflowSchedulerTaskRunSummaryError::MismatchedTaskState {
                task_id: task_id.to_string(),
            });
        }
        record_task_ids.remove(task_id);

        match task.execution_class {
            WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
                summary.runtime_inference_tasks += 1;
            }
            WorkflowSchedulerTaskExecutionClass::SourceInput => {
                summary.source_input_tasks += 1;
            }
            WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => {
                summary.non_runtime_node_engine_tasks += 1;
            }
            WorkflowSchedulerTaskExecutionClass::PumasMaterialization => {
                summary.pumas_materialization_tasks += 1;
            }
            WorkflowSchedulerTaskExecutionClass::Unsupported => {
                summary.unsupported_tasks += 1;
            }
        }

        if matches!(
            record.state.kind(),
            SchedulerTaskStateKind::Invalid | SchedulerTaskStateKind::TerminalFailed
        ) {
            summary.invalid_task_states += 1;
        }
    }

    if let Some(extra_task_id) = record_task_ids.into_iter().next() {
        return Err(WorkflowSchedulerTaskRunSummaryError::UnexpectedTaskState {
            task_id: extra_task_id.to_string(),
        });
    }

    Ok(summary)
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum WorkflowSchedulerTaskRunSummaryError {
    #[error("scheduler task '{task_id}' has no active task-state record")]
    MissingTaskState { task_id: String },
    #[error("active task-state record for scheduler task '{task_id}' has mismatched correlation")]
    MismatchedTaskState { task_id: String },
    #[error("active task-state record exists for unknown scheduler task '{task_id}'")]
    UnexpectedTaskState { task_id: String },
}

#[cfg(test)]
mod tests {
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerTaskState, SchedulerTaskStateDiagnostic,
        SchedulerTaskStateDiagnosticCode, SchedulerTaskStateDiagnosticSeverity,
        SchedulerTaskStateTransitionId, SchedulerWorkflowId, SchedulerWorkflowRunId,
        SCHEDULER_TASK_STATE_CONTRACT_VERSION,
    };

    use super::*;
    use crate::workflow::{
        WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };

    fn task_graph(
        classes: &[(&str, WorkflowSchedulerTaskExecutionClass)],
    ) -> WorkflowSchedulerTaskGraph {
        let workflow_id = SchedulerWorkflowId::parse("workflow-summary").expect("workflow id");
        let workflow_run_id = SchedulerWorkflowRunId::parse("run-summary").expect("run id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: classes
                .iter()
                .map(|(task_id, execution_class)| WorkflowSchedulerTask {
                    workflow_id: workflow_id.clone(),
                    workflow_run_id: workflow_run_id.clone(),
                    node_id: SchedulerNodeId::parse(*task_id).expect("node id"),
                    task_id: SchedulerTaskId::parse(*task_id).expect("task id"),
                    node_type: "summary-node".to_string(),
                    execution_class: *execution_class,
                    dependency_task_ids: Vec::new(),
                    input_bindings: Vec::new(),
                    schedulable_intent: None,
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: None,
                    diagnostics: Vec::new(),
                })
                .collect(),
        }
    }

    fn record(task_id: &str, state: SchedulerTaskState) -> SchedulerTaskStateRecord {
        SchedulerTaskStateRecord {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: SchedulerWorkflowId::parse("workflow-summary").expect("workflow id"),
            workflow_run_id: SchedulerWorkflowRunId::parse("run-summary").expect("run id"),
            node_id: SchedulerNodeId::parse(task_id).expect("node id"),
            task_id: SchedulerTaskId::parse(task_id).expect("task id"),
            state,
            state_version: 1,
            last_transition_id: SchedulerTaskStateTransitionId::parse("initial")
                .expect("transition id"),
        }
    }

    fn awaiting_inputs() -> SchedulerTaskState {
        SchedulerTaskState::AwaitingInputs {
            diagnostics: Vec::new(),
        }
    }

    fn invalid_state() -> SchedulerTaskState {
        SchedulerTaskState::Invalid {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::InvalidTask,
                message: "invalid task".to_string(),
                hint: None,
            }],
        }
    }

    #[test]
    fn summarizes_non_runtime_only_run() {
        let graph = task_graph(&[
            ("prompt", WorkflowSchedulerTaskExecutionClass::SourceInput),
            (
                "out",
                WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
            ),
        ]);

        let summary = workflow_scheduler_task_run_summary(
            &graph,
            &[
                record("prompt", awaiting_inputs()),
                record("out", awaiting_inputs()),
            ],
        )
        .expect("summary");

        assert_eq!(summary.source_input_tasks, 1);
        assert_eq!(summary.non_runtime_node_engine_tasks, 1);
        assert!(summary.is_non_runtime_only());
        assert!(!summary.has_runtime_inference());
    }

    #[test]
    fn summarizes_mixed_runtime_run() {
        let graph = task_graph(&[
            ("prompt", WorkflowSchedulerTaskExecutionClass::SourceInput),
            (
                "infer",
                WorkflowSchedulerTaskExecutionClass::RuntimeInference,
            ),
            (
                "model",
                WorkflowSchedulerTaskExecutionClass::PumasMaterialization,
            ),
        ]);

        let summary = workflow_scheduler_task_run_summary(
            &graph,
            &[
                record("prompt", awaiting_inputs()),
                record("infer", awaiting_inputs()),
                record("model", awaiting_inputs()),
            ],
        )
        .expect("summary");

        assert_eq!(summary.runtime_inference_tasks, 1);
        assert_eq!(summary.source_input_tasks, 1);
        assert_eq!(summary.pumas_materialization_tasks, 1);
        assert!(summary.has_runtime_inference());
        assert!(!summary.is_non_runtime_only());
    }

    #[test]
    fn includes_unsupported_and_invalid_states() {
        let graph = task_graph(&[("unknown", WorkflowSchedulerTaskExecutionClass::Unsupported)]);

        let summary =
            workflow_scheduler_task_run_summary(&graph, &[record("unknown", invalid_state())])
                .expect("summary");

        assert_eq!(summary.unsupported_tasks, 1);
        assert_eq!(summary.invalid_task_states, 1);
        assert!(!summary.is_non_runtime_only());
    }

    #[test]
    fn rejects_missing_task_state() {
        let graph = task_graph(&[(
            "prompt",
            WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        )]);

        let error = workflow_scheduler_task_run_summary(&graph, &[]).expect_err("missing record");

        assert_eq!(
            error,
            WorkflowSchedulerTaskRunSummaryError::MissingTaskState {
                task_id: "prompt".to_string()
            }
        );
    }

    #[test]
    fn rejects_unexpected_task_state() {
        let graph = task_graph(&[(
            "prompt",
            WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine,
        )]);

        let error = workflow_scheduler_task_run_summary(
            &graph,
            &[
                record("prompt", awaiting_inputs()),
                record("extra", awaiting_inputs()),
            ],
        )
        .expect_err("unexpected record");

        assert_eq!(
            error,
            WorkflowSchedulerTaskRunSummaryError::UnexpectedTaskState {
                task_id: "extra".to_string()
            }
        );
    }
}
