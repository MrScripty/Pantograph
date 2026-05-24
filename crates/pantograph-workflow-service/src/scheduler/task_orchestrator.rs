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
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerTask,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskInputBinding, WorkflowSchedulerTaskProjectionDiagnostic,
    WorkflowSchedulerTaskProjectionDiagnosticSeverity, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue, WorkflowServiceError,
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

#[derive(Debug, Clone)]
#[must_use]
#[allow(dead_code)]
pub(crate) struct StartedNonRuntimeTaskExecution {
    task: WorkflowSchedulerTask,
    materialized_results: Vec<WorkflowSchedulerTaskResult>,
    running_record: SchedulerTaskStateRecord,
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
    pub(crate) fn start_ready_non_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) -> Result<StartedNonRuntimeTaskExecution, WorkflowSchedulerTaskOrchestratorError> {
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
        Ok(StartedNonRuntimeTaskExecution {
            task: task.clone(),
            materialized_results,
            running_record,
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn execute_started_non_runtime_task(
        &self,
        started: &StartedNonRuntimeTaskExecution,
    ) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerTaskOrchestratorError> {
        execute_non_runtime_scheduler_task(&started.task, &started.materialized_results)
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter)
    }

    #[allow(dead_code)]
    pub(crate) fn complete_started_non_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        started: &StartedNonRuntimeTaskExecution,
        result: WorkflowSchedulerTaskResult,
    ) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
        let completion_transition = completion_transition_from_running(&started.running_record)?;
        store
            .complete_active_run_scheduler_task(
                session_id,
                workflow_run_id,
                completion_transition,
                result,
            )
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
            .and_then(applied_task_state_record)
    }

    #[allow(dead_code)]
    pub(crate) fn fail_started_non_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        started: &StartedNonRuntimeTaskExecution,
        error: &WorkflowSchedulerNonRuntimeTaskAdapterError,
    ) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
        let failure_transition = terminal_failure_transition_from_running(
            &started.running_record,
            non_runtime_adapter_failure_diagnostic(error),
        )?;
        store
            .apply_active_run_scheduler_task_transition(
                session_id,
                workflow_run_id,
                failure_transition,
            )
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
            .and_then(applied_task_state_record)
    }

    #[allow(dead_code)]
    pub(crate) fn advance_awaiting_non_runtime_task_inputs(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) -> Result<Option<SchedulerTaskStateRecord>, WorkflowSchedulerTaskOrchestratorError> {
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
        let record = records
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
        if record.state.kind() != SchedulerTaskStateKind::AwaitingInputs {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' must be awaiting inputs before readiness advancement",
                    task_id
                )),
            ));
        }

        let results = store
            .active_run_scheduler_task_results(session_id, workflow_run_id)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?;
        match non_runtime_input_readiness(task, &results) {
            NonRuntimeInputReadiness::Blocked => Ok(None),
            NonRuntimeInputReadiness::Ready => {
                let transition = ready_transition_from_awaiting_inputs(task, record)?;
                store
                    .apply_active_run_scheduler_task_transition(
                        session_id,
                        workflow_run_id,
                        transition,
                    )
                    .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                    .and_then(applied_task_state_record)
                    .map(Some)
            }
            NonRuntimeInputReadiness::InputUnavailable(diagnostic) => {
                let transition =
                    input_unavailable_transition_from_awaiting_inputs(record, diagnostic)?;
                store
                    .apply_active_run_scheduler_task_transition(
                        session_id,
                        workflow_run_id,
                        transition,
                    )
                    .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                    .and_then(applied_task_state_record)
                    .map(Some)
            }
            NonRuntimeInputReadiness::Invalid(diagnostic) => {
                let transition = invalid_transition_from_awaiting_inputs(record, diagnostic)?;
                store
                    .apply_active_run_scheduler_task_transition(
                        session_id,
                        workflow_run_id,
                        transition,
                    )
                    .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                    .and_then(applied_task_state_record)
                    .map(Some)
            }
        }
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
        WorkflowSchedulerTaskExecutionClass::SourceInput => Ok(awaiting_inputs_state()),
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
                    execution_intent: non_runtime_execution_intent(task)?,
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

fn non_runtime_execution_intent(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskExecutionIntent, WorkflowSchedulerTaskOrchestratorError> {
    Ok(SchedulerTaskExecutionIntent::NonRuntime {
        task_intent: SchedulerNonRuntimeTaskIntent {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: task.workflow_id.clone(),
            workflow_run_id: task.workflow_run_id.clone(),
            node_id: task.node_id.clone(),
            task_id: task.task_id.clone(),
            task_kind: SchedulerNonRuntimeTaskKind::parse(&task.node_type)
                .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?,
        },
    })
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

fn ready_transition_from_awaiting_inputs(
    task: &WorkflowSchedulerTask,
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "inputs-ready",
        SchedulerTaskStateKind::AwaitingInputs,
        SchedulerTaskState::Ready {
            execution_intent: non_runtime_execution_intent(task)?,
        },
    )
}

fn input_unavailable_transition_from_awaiting_inputs(
    record: &SchedulerTaskStateRecord,
    diagnostic: SchedulerTaskStateDiagnostic,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "input-unavailable",
        SchedulerTaskStateKind::AwaitingInputs,
        SchedulerTaskState::InputUnavailable {
            diagnostics: vec![diagnostic],
        },
    )
}

fn invalid_transition_from_awaiting_inputs(
    record: &SchedulerTaskStateRecord,
    diagnostic: SchedulerTaskStateDiagnostic,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "invalid-input",
        SchedulerTaskStateKind::AwaitingInputs,
        SchedulerTaskState::Invalid {
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

enum NonRuntimeInputReadiness {
    Ready,
    Blocked,
    InputUnavailable(SchedulerTaskStateDiagnostic),
    Invalid(SchedulerTaskStateDiagnostic),
}

fn non_runtime_input_readiness(
    task: &WorkflowSchedulerTask,
    results: &[WorkflowSchedulerTaskResult],
) -> NonRuntimeInputReadiness {
    let Some(template) = task.non_runtime_task_template.as_ref() else {
        return NonRuntimeInputReadiness::Invalid(scheduler_input_diagnostic(
            SchedulerTaskStateDiagnosticCode::InvalidTask,
            "non-runtime scheduler task is missing a typed execution template",
        ));
    };

    match template {
        WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput => {
            match materialized_binding_value(task, results, "text") {
                MaterializedBindingValue::Ready(WorkflowSchedulerTaskResultValue::String(_)) => {
                    NonRuntimeInputReadiness::Ready
                }
                MaterializedBindingValue::Ready(_) => {
                    NonRuntimeInputReadiness::Invalid(scheduler_input_diagnostic(
                        SchedulerTaskStateDiagnosticCode::InvalidTask,
                        "materialized text input has the wrong value type",
                    ))
                }
                MaterializedBindingValue::Blocked => NonRuntimeInputReadiness::Blocked,
                MaterializedBindingValue::Unavailable(diagnostic) => {
                    NonRuntimeInputReadiness::InputUnavailable(diagnostic)
                }
                MaterializedBindingValue::Invalid(diagnostic) => {
                    NonRuntimeInputReadiness::Invalid(diagnostic)
                }
            }
        }
    }
}

enum MaterializedBindingValue<'a> {
    Ready(&'a WorkflowSchedulerTaskResultValue),
    Blocked,
    Unavailable(SchedulerTaskStateDiagnostic),
    Invalid(SchedulerTaskStateDiagnostic),
}

fn materialized_binding_value<'a>(
    task: &WorkflowSchedulerTask,
    results: &'a [WorkflowSchedulerTaskResult],
    target_port_id: &str,
) -> MaterializedBindingValue<'a> {
    let Some(binding) = task
        .input_bindings
        .iter()
        .find(|binding| binding.target_port_id == target_port_id)
    else {
        return MaterializedBindingValue::Invalid(scheduler_input_diagnostic(
            SchedulerTaskStateDiagnosticCode::InvalidTask,
            format!(
                "non-runtime scheduler task is missing input binding '{}'",
                target_port_id
            ),
        ));
    };
    materialized_bound_output(task, results, binding)
}

fn materialized_bound_output<'a>(
    task: &WorkflowSchedulerTask,
    results: &'a [WorkflowSchedulerTaskResult],
    binding: &WorkflowSchedulerTaskInputBinding,
) -> MaterializedBindingValue<'a> {
    let Some(result) = results.iter().find(|result| {
        result.workflow_run_id == task.workflow_run_id.as_str()
            && result.task_id == binding.source_task_id.as_str()
            && result.node_id == binding.source_node_id.as_str()
    }) else {
        return MaterializedBindingValue::Blocked;
    };
    if let Err(error) = result.validate() {
        return MaterializedBindingValue::Invalid(scheduler_input_diagnostic(
            SchedulerTaskStateDiagnosticCode::InvalidTask,
            format!("materialized task result is invalid: {error}"),
        ));
    }
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => {}
        WorkflowSchedulerTaskResultStatus::Unavailable => {
            return MaterializedBindingValue::Unavailable(scheduler_input_diagnostic(
                SchedulerTaskStateDiagnosticCode::InputUnavailable,
                format!("upstream task '{}' is unavailable", result.task_id),
            ));
        }
        WorkflowSchedulerTaskResultStatus::Failed | WorkflowSchedulerTaskResultStatus::Invalid => {
            return MaterializedBindingValue::Invalid(scheduler_input_diagnostic(
                SchedulerTaskStateDiagnosticCode::InvalidTask,
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
        .map(|output| MaterializedBindingValue::Ready(&output.value))
        .unwrap_or_else(|| MaterializedBindingValue::Blocked)
}

fn scheduler_input_diagnostic(
    code: SchedulerTaskStateDiagnosticCode,
    message: impl Into<String>,
) -> SchedulerTaskStateDiagnostic {
    SchedulerTaskStateDiagnostic {
        severity: SchedulerTaskStateDiagnosticSeverity::Error,
        code,
        message: message.into(),
        hint: Some(
            "Materialize the required upstream scheduler task output before running this task."
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
