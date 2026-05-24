use pantograph_runtime_host_contracts::{
    RuntimeHostDispatchError, SchedulerRuntimeHostDispatcher, ValidatedRuntimeHostExecutionResponse,
};
use pantograph_scheduler::{
    SchedulerContractError, SchedulerNonRuntimeTaskIntent, SchedulerNonRuntimeTaskKind,
    SchedulerRuntimeHandoff, SchedulerTaskExecutionIntent, SchedulerTaskState,
    SchedulerTaskStateDiagnostic, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateRecord, SchedulerTaskStateTransitionId,
    SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};
use thiserror::Error;

use crate::workflow::{
    WorkflowSchedulerTask, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskProjectionDiagnostic, WorkflowSchedulerTaskProjectionDiagnosticSeverity,
    WorkflowServiceError,
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
