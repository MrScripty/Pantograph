use pantograph_runtime_host_contracts::{
    RuntimeHostDispatchError, SchedulerRuntimeHostDispatcher, ValidatedRuntimeHostExecutionResponse,
};
use pantograph_scheduler::{
    SchedulerContractError, SchedulerNonRuntimeTaskIntent, SchedulerNonRuntimeTaskKind,
    SchedulerRuntimeHandoff, SchedulerTaskExecutionIntent, SchedulerTaskState,
    SchedulerTaskStateDiagnostic, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionId,
    SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};
use thiserror::Error;

use crate::workflow::{
    execute_non_runtime_scheduler_task, WorkflowSchedulerNonRuntimeTaskAdapterError,
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskProjectionDiagnostic, WorkflowSchedulerTaskProjectionDiagnosticSeverity,
    WorkflowSchedulerTaskResult, WorkflowServiceError,
};

use super::WorkflowExecutionSessionStore;

/// Workflow-service async shell for scheduler task orchestration.
///
/// This type owns application-layer calls into lower-level scheduler and
/// runtime-host contracts. Scheduler policy remains in `pantograph-scheduler`;
/// runtime execution remains behind the shared runtime-host port.
#[derive(Clone)]
#[must_use]
pub(crate) struct WorkflowSchedulerTaskOrchestrator {
    runtime_host_dispatcher: SchedulerRuntimeHostDispatcher,
}

impl WorkflowSchedulerTaskOrchestrator {
    pub(crate) fn new(runtime_host_dispatcher: SchedulerRuntimeHostDispatcher) -> Self {
        Self {
            runtime_host_dispatcher,
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn dispatch_runtime_handoff(
        &self,
        execution_request_id: impl Into<String>,
        handoff: SchedulerRuntimeHandoff,
    ) -> Result<ValidatedRuntimeHostExecutionResponse, WorkflowSchedulerTaskOrchestratorError> {
        self.runtime_host_dispatcher
            .dispatch(execution_request_id, handoff)
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::RuntimeHostDispatch)
    }

    pub(crate) fn initial_task_state_records(
        &self,
        task_graph: &WorkflowSchedulerTaskGraph,
    ) -> Result<Vec<SchedulerTaskStateRecord>, WorkflowSchedulerTaskOrchestratorError> {
        let mut records = Vec::with_capacity(task_graph.tasks.len());
        for task in &task_graph.tasks {
            let state = initial_task_state(task)?;
            let record = SchedulerTaskStateRecord {
                contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                workflow_id: task_graph.workflow_id.clone(),
                workflow_run_id: task_graph.workflow_run_id.clone(),
                node_id: task.node_id.clone(),
                task_id: task.task_id.clone(),
                state,
                state_version: 1,
                last_transition_id: SchedulerTaskStateTransitionId::parse(format!(
                    "initial:{}",
                    task.task_id.as_str()
                ))
                .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?,
            };
            record
                .validate()
                .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?;
            records.push(record);
        }
        Ok(records)
    }

    pub(crate) fn initialize_active_run_task_state(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_graph: WorkflowSchedulerTaskGraph,
    ) -> Result<(), WorkflowSchedulerTaskOrchestratorError> {
        let records = self.initial_task_state_records(&task_graph)?;
        store
            .set_active_run_scheduler_task_state(session_id, workflow_run_id, task_graph, records)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
    }

    #[allow(dead_code)]
    pub(crate) async fn execute_ready_non_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerTaskOrchestratorError> {
        let (task_graph, records) = store
            .active_run_scheduler_task_state(session_id, workflow_run_id)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?
            .ok_or_else(|| {
                WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                    WorkflowServiceError::InvalidRequest(format!(
                        "active workflow run '{}' has no scheduler task graph",
                        workflow_run_id
                    )),
                )
            })?;
        let task = task_graph
            .tasks
            .iter()
            .find(|task| task.task_id.as_str() == task_id)
            .ok_or_else(|| {
                WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler task '{}' is not in active workflow run '{}'",
                        task_id, workflow_run_id
                    )),
                )
            })?;
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' is not a non-runtime node-engine task",
                    task_id
                )),
            ));
        }

        let ready_record = records
            .iter()
            .find(|record| record.task_id.as_str() == task_id)
            .ok_or_else(|| {
                WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler task '{}' has no active task-state record",
                        task_id
                    )),
                )
            })?;
        let ready_execution_intent = ready_non_runtime_execution_intent(ready_record)?;
        let running_transition =
            running_transition_from_ready(ready_record, ready_execution_intent.clone())?;
        let running_record = store
            .apply_active_run_scheduler_task_transition(
                session_id,
                workflow_run_id,
                running_transition,
            )
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
            .and_then(applied_task_state_record)?;

        let materialized_results = store
            .active_run_scheduler_task_results(session_id, workflow_run_id)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?;
        let result = match execute_non_runtime_scheduler_task(task, &materialized_results).await {
            Ok(result) => result,
            Err(error) => {
                let failure_transition = terminal_failure_transition_from_running(
                    &running_record,
                    non_runtime_adapter_failure_diagnostic(&error),
                )?;
                let _ = store
                    .apply_active_run_scheduler_task_transition(
                        session_id,
                        workflow_run_id,
                        failure_transition,
                    )
                    .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?;
                return Err(WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter(error));
            }
        };

        let completion_transition = completion_transition_from_running(&running_record)?;
        let _ = store
            .complete_active_run_scheduler_task(
                session_id,
                workflow_run_id,
                completion_transition,
                result.clone(),
            )
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?;
        Ok(result)
    }
}

fn initial_task_state(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskState, WorkflowSchedulerTaskOrchestratorError> {
    if !task.diagnostics.is_empty() {
        return Ok(SchedulerTaskState::Invalid {
            diagnostics: task
                .diagnostics
                .iter()
                .map(projection_diagnostic_to_task_state_diagnostic)
                .collect(),
        });
    }

    match task.execution_class {
        WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
            if let Some(task_intent) = task.schedulable_intent.clone() {
                Ok(SchedulerTaskState::Ready {
                    execution_intent: SchedulerTaskExecutionIntent::Runtime { task_intent },
                })
            } else {
                Ok(awaiting_inputs_state())
            }
        }
        WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => {
            if task.non_runtime_task_template.is_none() {
                return Ok(SchedulerTaskState::Invalid {
                    diagnostics: vec![SchedulerTaskStateDiagnostic {
                        severity: SchedulerTaskStateDiagnosticSeverity::Error,
                        code: SchedulerTaskStateDiagnosticCode::InvalidTask,
                        message: format!(
                            "workflow task type '{}' is missing a typed non-runtime execution template",
                            task.node_type
                        ),
                        hint: Some(
                            "Add a concrete typed scheduler template before executing this non-runtime node."
                                .to_string(),
                        ),
                    }],
                });
            }
            if task.dependency_task_ids.is_empty() {
                Ok(SchedulerTaskState::Ready {
                    execution_intent: SchedulerTaskExecutionIntent::NonRuntime {
                        task_intent: SchedulerNonRuntimeTaskIntent {
                            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
                            workflow_id: task.workflow_id.clone(),
                            workflow_run_id: task.workflow_run_id.clone(),
                            node_id: task.node_id.clone(),
                            task_id: task.task_id.clone(),
                            task_kind: SchedulerNonRuntimeTaskKind::parse(&task.node_type)
                                .map_err(
                                    WorkflowSchedulerTaskOrchestratorError::SchedulerContract,
                                )?,
                        },
                    },
                })
            } else {
                Ok(awaiting_inputs_state())
            }
        }
        WorkflowSchedulerTaskExecutionClass::PumasMaterialization => {
            Ok(SchedulerTaskState::AwaitingInputs {
                diagnostics: vec![SchedulerTaskStateDiagnostic {
                    severity: SchedulerTaskStateDiagnosticSeverity::Info,
                    code: SchedulerTaskStateDiagnosticCode::AwaitingInputs,
                    message: "task is awaiting Pumas model-reference materialization".to_string(),
                    hint: Some(
                        "Materialize the Pumas model reference through the dedicated model-selection boundary."
                            .to_string(),
                    ),
                }],
            })
        }
        WorkflowSchedulerTaskExecutionClass::Unsupported => Ok(SchedulerTaskState::Invalid {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::InvalidTask,
                message: format!(
                    "workflow task type '{}' is not supported by scheduler task orchestration",
                    task.node_type
                ),
                hint: Some(
                    "Add an explicit scheduler execution class and typed value contract before scheduling this node."
                        .to_string(),
                ),
            }],
        }),
    }
}

fn awaiting_inputs_state() -> SchedulerTaskState {
    SchedulerTaskState::AwaitingInputs {
        diagnostics: Vec::new(),
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub(crate) enum WorkflowSchedulerTaskOrchestratorError {
    #[allow(dead_code)]
    #[error("runtime-host dispatch failed")]
    RuntimeHostDispatch(RuntimeHostDispatchError),
    #[error("scheduler contract validation failed")]
    SchedulerContract(SchedulerContractError),
    #[error("workflow service operation failed")]
    WorkflowService(WorkflowServiceError),
    #[error("non-runtime scheduler task execution failed")]
    NonRuntimeTaskAdapter(WorkflowSchedulerNonRuntimeTaskAdapterError),
}

fn ready_non_runtime_execution_intent(
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskExecutionIntent, WorkflowSchedulerTaskOrchestratorError> {
    let SchedulerTaskState::Ready { execution_intent } = &record.state else {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be ready before non-runtime execution",
                record.task_id.as_str()
            )),
        ));
    };
    if execution_intent.non_runtime_task_intent().is_none() {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' ready state is not non-runtime",
                record.task_id.as_str()
            )),
        ));
    }
    Ok(execution_intent.clone())
}

fn running_transition_from_ready(
    record: &SchedulerTaskStateRecord,
    execution_intent: SchedulerTaskExecutionIntent,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "running",
        SchedulerTaskStateKind::Ready,
        SchedulerTaskState::Running { execution_intent },
    )
}

fn completion_transition_from_running(
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    let SchedulerTaskState::Running { execution_intent } = &record.state else {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be running before completion",
                record.task_id.as_str()
            )),
        ));
    };
    task_state_transition(
        record,
        "completed",
        SchedulerTaskStateKind::Running,
        SchedulerTaskState::Completed {
            execution_intent: execution_intent.clone(),
        },
    )
}

fn terminal_failure_transition_from_running(
    record: &SchedulerTaskStateRecord,
    diagnostic: SchedulerTaskStateDiagnostic,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "terminal-failed",
        SchedulerTaskStateKind::Running,
        SchedulerTaskState::TerminalFailed {
            diagnostics: vec![diagnostic],
        },
    )
}

fn task_state_transition(
    record: &SchedulerTaskStateRecord,
    label: &str,
    expected_previous_state: SchedulerTaskStateKind,
    next_state: SchedulerTaskState,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    let transition_id = SchedulerTaskStateTransitionId::parse(format!(
        "scheduler-task:{label}:{}",
        record.state_version + 1
    ))
    .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?;
    Ok(SchedulerTaskStateTransition {
        contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
        transition_id,
        workflow_id: record.workflow_id.clone(),
        workflow_run_id: record.workflow_run_id.clone(),
        node_id: record.node_id.clone(),
        task_id: record.task_id.clone(),
        expected_previous_state: Some(expected_previous_state),
        next_state,
    })
}

fn applied_task_state_record(
    result: pantograph_scheduler::SchedulerTaskStateTransitionApplyResult,
) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
    match result {
        pantograph_scheduler::SchedulerTaskStateTransitionApplyResult::Applied(record) => {
            Ok(record)
        }
        pantograph_scheduler::SchedulerTaskStateTransitionApplyResult::AlreadyApplied(record) => {
            Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' transition was already applied",
                    record.task_id.as_str()
                )),
            ))
        }
    }
}

fn non_runtime_adapter_failure_diagnostic(
    error: &WorkflowSchedulerNonRuntimeTaskAdapterError,
) -> SchedulerTaskStateDiagnostic {
    SchedulerTaskStateDiagnostic {
        severity: SchedulerTaskStateDiagnosticSeverity::Error,
        code: SchedulerTaskStateDiagnosticCode::TerminalFailure,
        message: format!("non-runtime scheduler task execution failed: {error}"),
        hint: Some(
            "Fix the typed non-runtime task template or materialized upstream inputs before retrying."
                .to_string(),
        ),
    }
}

fn projection_diagnostic_to_task_state_diagnostic(
    diagnostic: &WorkflowSchedulerTaskProjectionDiagnostic,
) -> SchedulerTaskStateDiagnostic {
    SchedulerTaskStateDiagnostic {
        severity: match diagnostic.severity {
            WorkflowSchedulerTaskProjectionDiagnosticSeverity::Error => {
                SchedulerTaskStateDiagnosticSeverity::Error
            }
        },
        code: SchedulerTaskStateDiagnosticCode::InvalidTask,
        message: diagnostic.message.clone(),
        hint: diagnostic.port_id.as_ref().map(|port_id| {
            format!(
                "Fix scheduler task graph input '{}' for node '{}'.",
                port_id,
                diagnostic.node_id.as_str()
            )
        }),
    }
}

#[cfg(test)]
#[path = "task_orchestrator_tests.rs"]
mod tests;
