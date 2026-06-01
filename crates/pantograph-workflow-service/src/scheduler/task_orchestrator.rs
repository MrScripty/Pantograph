use std::sync::Arc;

use async_trait::async_trait;
use pantograph_dependency_planning::{DependencyReadinessPolicy, DependencyReadinessProofEnvelope};
use pantograph_runtime_host_contracts::{
    ReservationLifecycleApplicationState, ReservationLifecycleContractError,
    ReservationLifecycleDiagnostic, ReservationLifecycleDiagnosticCode,
    ReservationLifecycleDiagnosticSeverity, ReservationLifecycleEvent, ReservationLifecycleOutcome,
    ReservationLifecyclePort, ReservationLifecyclePortError, RuntimeHostDispatchError,
    RuntimeHostExecutionInput, SchedulerRuntimeHostDispatcher,
    ValidatedReservationLifecycleApplication, ValidatedReservationLifecycleEvent,
    RESERVATION_LIFECYCLE_CONTRACT_VERSION,
};
use pantograph_scheduler::{
    plan_scheduler_readiness_admission, select_scheduler_dispatch, SchedulerContractError,
    SchedulerDispatchCandidateId, SchedulerDispatchDecision, SchedulerDispatchSelectionDecision,
    SchedulerDispatchSelectionDiagnostic, SchedulerDispatchSelectionDiagnosticSeverity,
    SchedulerDispatchSelectionRequest, SchedulerDispatchSelectionState,
    SchedulerNonRuntimeTaskIntent, SchedulerNonRuntimeTaskKind,
    SchedulerReadinessAdmissionDecision, SchedulerReadinessAdmissionDiagnostic,
    SchedulerReadinessAdmissionDiagnosticCode, SchedulerReadinessAdmissionRequest,
    SchedulerReadinessAdmissionSeverity, SchedulerReadinessAdmissionState,
    SchedulerReservationLeaseId, SchedulerRuntimeHandoff, SchedulerRuntimeHandoffState,
    SchedulerSourceInputTaskIntent, SchedulerSourceInputTaskKind, SchedulerTaskExecutionIntent,
    SchedulerTaskState, SchedulerTaskStateDiagnostic, SchedulerTaskStateDiagnosticCode,
    SchedulerTaskStateDiagnosticSeverity, SchedulerTaskStateKind, SchedulerTaskStateRecord,
    SchedulerTaskStateTransition, SchedulerTaskStateTransitionId,
    ValidatedSchedulerDispatchSelectionRequest, SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
    SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION, SCHEDULER_TASK_STATE_CONTRACT_VERSION,
};
use thiserror::Error;

use crate::workflow::{
    execute_non_runtime_scheduler_task, materialize_external_workflow_inputs,
    materialize_runtime_host_inputs, runtime_host_response_to_task_result,
    WorkflowExternalInputMaterializationError, WorkflowPortBinding,
    WorkflowRuntimeHostTaskInputMappingError, WorkflowRuntimeHostTaskResultMappingError,
    WorkflowSchedulerNonRuntimeTaskAdapterError, WorkflowSchedulerNonRuntimeTaskTemplate,
    WorkflowSchedulerSourceInputTemplate, WorkflowSchedulerTask,
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
    reservation_lifecycle_port: Arc<dyn ReservationLifecyclePort>,
}

#[derive(Debug, Clone)]
#[must_use]
pub(crate) struct StartedNonRuntimeTaskExecution {
    task: WorkflowSchedulerTask,
    materialized_results: Vec<WorkflowSchedulerTaskResult>,
    running_record: SchedulerTaskStateRecord,
}

#[derive(Debug, Clone)]
#[must_use]
pub(crate) struct StartedRuntimeTaskExecution {
    pub(crate) task: WorkflowSchedulerTask,
    pub(crate) materialized_results: Vec<WorkflowSchedulerTaskResult>,
    running_record: SchedulerTaskStateRecord,
}

impl WorkflowSchedulerTaskOrchestrator {
    pub(crate) fn new(runtime_host_dispatcher: SchedulerRuntimeHostDispatcher) -> Self {
        Self {
            runtime_host_dispatcher,
            reservation_lifecycle_port: Arc::new(UnavailableReservationLifecyclePort),
        }
    }

    pub(crate) fn with_runtime_host_dispatcher(
        mut self,
        runtime_host_dispatcher: SchedulerRuntimeHostDispatcher,
    ) -> Self {
        self.runtime_host_dispatcher = runtime_host_dispatcher;
        self
    }

    pub(crate) fn with_reservation_lifecycle_port(
        mut self,
        port: Arc<dyn ReservationLifecyclePort>,
    ) -> Self {
        self.reservation_lifecycle_port = port;
        self
    }

    pub(crate) async fn dispatch_runtime_handoff(
        &self,
        execution_request_id: impl Into<String>,
        handoff: SchedulerRuntimeHandoff,
        materialized_inputs: Vec<RuntimeHostExecutionInput>,
    ) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerTaskOrchestratorError> {
        let response = self
            .runtime_host_dispatcher
            .dispatch(execution_request_id, handoff, materialized_inputs)
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::RuntimeHostDispatch)?;
        runtime_host_response_to_task_result(&response)
            .map_err(WorkflowSchedulerTaskOrchestratorError::RuntimeHostTaskResultMapping)
    }

    pub(crate) async fn select_and_dispatch_runtime_task(
        &self,
        execution_request_id: impl Into<String>,
        task: &WorkflowSchedulerTask,
        materialized_results: &[WorkflowSchedulerTaskResult],
        selection_request: ValidatedSchedulerDispatchSelectionRequest,
    ) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerTaskOrchestratorError> {
        let selection_request = selection_request.into_inner();
        let selection = select_scheduler_dispatch(
            ValidatedSchedulerDispatchSelectionRequest::try_from(selection_request.clone())
                .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?,
        )
        .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?
        .into_inner();
        if selection.state != SchedulerDispatchSelectionState::Selected {
            self.apply_unselected_candidate_lifecycle_events(task, &selection_request)
                .await?;
            return Err(
                WorkflowSchedulerTaskOrchestratorError::RuntimeDispatchSelectionNoSelection(
                    selection,
                ),
            );
        }
        let handoff = dispatch_selected_handoff_from_selection(selection)?;
        let dispatch_decision = handoff.dispatch_decision.as_ref().ok_or_else(|| {
            WorkflowSchedulerTaskOrchestratorError::SchedulerContract(
                SchedulerContractError::MissingField {
                    field: "dispatch_decision",
                },
            )
        })?;
        let reservation_lease_id = dispatch_decision.reservation_lease_id.clone();
        let candidate_id = selected_candidate_id(&selection_request, dispatch_decision);
        let _ = self
            .apply_reservation_lifecycle_event(reservation_lifecycle_event(
                task,
                reservation_lease_id.clone(),
                candidate_id.clone(),
                ReservationLifecycleOutcome::DispatchStarted,
                vec![reservation_lifecycle_diagnostic(
                    ReservationLifecycleDiagnosticSeverity::Info,
                    ReservationLifecycleDiagnosticCode::DispatchStarted,
                    "runtime dispatch started for selected scheduler reservation",
                )],
            )?)
            .await?;
        let materialized_inputs = materialize_runtime_host_inputs(task, materialized_results)
            .map_err(WorkflowSchedulerTaskOrchestratorError::RuntimeHostTaskInputMapping)?;
        let dispatch_result = self
            .dispatch_runtime_handoff(execution_request_id, handoff, materialized_inputs)
            .await;
        match dispatch_result {
            Ok(result) => {
                let _ = self
                    .apply_reservation_lifecycle_event(runtime_host_terminal_lifecycle_event(
                        task,
                        reservation_lease_id.clone(),
                        candidate_id,
                        &result,
                    )?)
                    .await?;
                Ok(result)
            }
            Err(error) => {
                let _ = self
                    .apply_reservation_lifecycle_event(reservation_lifecycle_event(
                        task,
                        reservation_lease_id,
                        candidate_id,
                        ReservationLifecycleOutcome::RuntimeHostDispatchRejected,
                        vec![reservation_lifecycle_diagnostic(
                            ReservationLifecycleDiagnosticSeverity::Error,
                            ReservationLifecycleDiagnosticCode::RuntimeHostRejected,
                            format!("runtime-host dispatch failed: {error}"),
                        )],
                    )?)
                    .await?;
                Err(error)
            }
        }
    }

    async fn apply_unselected_candidate_lifecycle_events(
        &self,
        task: &WorkflowSchedulerTask,
        selection_request: &SchedulerDispatchSelectionRequest,
    ) -> Result<(), WorkflowSchedulerTaskOrchestratorError> {
        for candidate in &selection_request.candidates {
            let Some(reservation) = candidate.reservations.first() else {
                continue;
            };
            let _ = self
                .apply_reservation_lifecycle_event(reservation_lifecycle_event(
                    task,
                    reservation.reservation_lease_id.clone(),
                    Some(candidate.candidate_id.clone()),
                    ReservationLifecycleOutcome::CandidateUnselected,
                    vec![reservation_lifecycle_diagnostic(
                        ReservationLifecycleDiagnosticSeverity::Info,
                        ReservationLifecycleDiagnosticCode::CandidateUnselected,
                        "scheduler dispatch selection did not select this reserved candidate",
                    )],
                )?)
                .await?;
        }
        Ok(())
    }

    async fn apply_reservation_lifecycle_event(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ValidatedReservationLifecycleApplication, WorkflowSchedulerTaskOrchestratorError>
    {
        let validated_event = ValidatedReservationLifecycleEvent::try_from(event)
            .map_err(WorkflowSchedulerTaskOrchestratorError::ReservationLifecycleContract)?;
        let expected_event_id = validated_event.as_ref().lifecycle_event_id.clone();
        let expected_reservation_lease_id = validated_event.as_ref().reservation_lease_id.clone();
        let application = self
            .reservation_lifecycle_port
            .apply_reservation_lifecycle(validated_event.into_inner())
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::ReservationLifecyclePort)?;
        let validated_application = ValidatedReservationLifecycleApplication::try_from(application)
            .map_err(WorkflowSchedulerTaskOrchestratorError::ReservationLifecycleContract)?;
        let application = validated_application.as_ref();
        if application.lifecycle_event_id != expected_event_id {
            return Err(
                WorkflowSchedulerTaskOrchestratorError::ReservationLifecycleContract(
                    ReservationLifecycleContractError::InvalidField {
                        field: "lifecycle_event_id",
                        reason: "reservation lifecycle application must match event id",
                    },
                ),
            );
        }
        if application.reservation_lease_id != expected_reservation_lease_id {
            return Err(
                WorkflowSchedulerTaskOrchestratorError::ReservationLifecycleContract(
                    ReservationLifecycleContractError::InvalidField {
                        field: "reservation_lease_id",
                        reason: "reservation lifecycle application must match reservation lease id",
                    },
                ),
            );
        }
        if application.state == ReservationLifecycleApplicationState::Failed {
            return Err(
                WorkflowSchedulerTaskOrchestratorError::ReservationLifecyclePort(
                    ReservationLifecyclePortError::Failed {
                        message: "reservation lifecycle application failed".to_string(),
                    },
                ),
            );
        }
        Ok(validated_application)
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

    #[cfg(test)]
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

    pub(crate) fn materialize_external_inputs_for_active_run(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        inputs: &[WorkflowPortBinding],
    ) -> Result<Vec<SchedulerTaskStateRecord>, WorkflowSchedulerTaskOrchestratorError> {
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
        let results = materialize_external_workflow_inputs(&task_graph, inputs)
            .map_err(WorkflowSchedulerTaskOrchestratorError::ExternalInputMaterialization)?;

        let mut completed_records = Vec::with_capacity(results.len());
        for result in results {
            let task = task_graph
                .tasks
                .iter()
                .find(|task| task.task_id.as_str() == result.task_id)
                .ok_or_else(|| {
                    WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler source-input task '{}' is not in active workflow run '{}'",
                            result.task_id, workflow_run_id
                        )),
                    )
                })?;
            let record = records
                .iter()
                .find(|record| record.task_id.as_str() == result.task_id)
                .ok_or_else(|| {
                    WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler source-input task '{}' has no active task-state record",
                            result.task_id
                        )),
                    )
                })?;
            let transition = source_input_materialization_transition(record, task)?;
            let completed = store
                .materialize_active_run_source_input_task(
                    session_id,
                    workflow_run_id,
                    transition,
                    result,
                )
                .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                .and_then(applied_task_state_record)?;
            completed_records.push(completed);
        }
        Ok(completed_records)
    }

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

    pub(crate) async fn execute_started_non_runtime_task(
        &self,
        started: &StartedNonRuntimeTaskExecution,
    ) -> Result<WorkflowSchedulerTaskResult, WorkflowSchedulerTaskOrchestratorError> {
        execute_non_runtime_scheduler_task(&started.task, &started.materialized_results)
            .await
            .map_err(WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter)
    }

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

    pub(crate) fn start_ready_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
    ) -> Result<StartedRuntimeTaskExecution, WorkflowSchedulerTaskOrchestratorError> {
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
                        "scheduler runtime task '{}' is not in active workflow run '{}'",
                        task_id, workflow_run_id
                    )),
                )
            })?;
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' is not a runtime inference task",
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
                        "scheduler runtime task '{}' has no active task-state record",
                        task_id
                    )),
                )
            })?;
        let ready_execution_intent = ready_runtime_execution_intent(ready_record)?;
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
        Ok(StartedRuntimeTaskExecution {
            task: task.clone(),
            materialized_results,
            running_record,
        })
    }

    pub(crate) fn complete_started_runtime_task(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        started: &StartedRuntimeTaskExecution,
        result: WorkflowSchedulerTaskResult,
    ) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
        let completion_transition =
            runtime_result_transition_from_running(&started.running_record, &result)?;
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

    pub(crate) fn fail_started_runtime_task_dispatch_selection(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        started: &StartedRuntimeTaskExecution,
        selection: &SchedulerDispatchSelectionDecision,
    ) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
        let failure_transition = terminal_failure_transition_from_running_diagnostics(
            &started.running_record,
            runtime_dispatch_selection_task_diagnostics(selection),
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

    pub(crate) fn advance_awaiting_runtime_task_inputs(
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
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' is not a runtime inference task",
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
                    "scheduler task '{}' must be awaiting inputs before runtime readiness advancement",
                    task_id
                )),
            ));
        }

        let results = store
            .active_run_scheduler_task_results(session_id, workflow_run_id)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)?;
        match runtime_input_readiness(task, &results) {
            RuntimeInputReadiness::Blocked => Ok(None),
            RuntimeInputReadiness::Ready => {
                let transition =
                    waiting_dependency_readiness_transition_from_awaiting_inputs(task, record)?;
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
            RuntimeInputReadiness::InputUnavailable(diagnostic) => {
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
            RuntimeInputReadiness::Invalid(diagnostic) => {
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

    pub(crate) fn apply_runtime_dependency_readiness_admission(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
        policy: DependencyReadinessPolicy,
        readiness_proof: Option<DependencyReadinessProofEnvelope>,
    ) -> Result<SchedulerTaskStateRecord, WorkflowSchedulerTaskOrchestratorError> {
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
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' is not a runtime inference task",
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
        if record.state.kind() != SchedulerTaskStateKind::WaitingDependencyReadiness {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' must be waiting for dependency readiness before readiness admission",
                    task_id
                )),
            ));
        }
        let Some(execution_intent) = record.state.execution_intent().cloned() else {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no runtime execution intent",
                    task_id
                )),
            ));
        };
        let Some(task_intent) = execution_intent.runtime_task_intent().cloned() else {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' does not carry runtime task intent",
                    task_id
                )),
            ));
        };

        let request = SchedulerReadinessAdmissionRequest {
            contract_version: SCHEDULER_READINESS_ADMISSION_CONTRACT_VERSION,
            task_intent,
            policy,
        };
        let decision = plan_scheduler_readiness_admission(
            request
                .try_into()
                .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?,
            readiness_proof,
        )
        .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)?
        .into_inner();
        let transition =
            readiness_admission_transition_from_waiting(record, execution_intent, decision)?;
        store
            .apply_active_run_scheduler_task_transition(session_id, workflow_run_id, transition)
            .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
            .and_then(applied_task_state_record)
    }

    pub(crate) fn fail_runtime_dispatch_not_wired_for_active_run(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<Vec<SchedulerTaskStateRecord>, WorkflowSchedulerTaskOrchestratorError> {
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
        let mut failed_records = Vec::new();
        for task in task_graph.tasks.iter().filter(|task| {
            task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
        }) {
            let record = records
                .iter()
                .find(|record| record.task_id.as_str() == task.task_id.as_str())
                .ok_or_else(|| {
                    WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler runtime task '{}' has no active task-state record",
                            task.task_id.as_str()
                        )),
                    )
                })?;
            if record.state.kind() == SchedulerTaskStateKind::TerminalFailed {
                failed_records.push(record.clone());
                continue;
            }
            if record.state.kind() == SchedulerTaskStateKind::Completed {
                return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler runtime task '{}' is already completed before runtime dispatch",
                        task.task_id.as_str()
                    )),
                ));
            }
            let transition = runtime_dispatch_not_wired_transition(record)?;
            let failed = store
                .apply_active_run_scheduler_task_transition(session_id, workflow_run_id, transition)
                .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                .and_then(applied_task_state_record)?;
            failed_records.push(failed);
        }
        if failed_records.is_empty() {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "active workflow run '{}' has no runtime inference scheduler tasks",
                    workflow_run_id
                )),
            ));
        }
        Ok(failed_records)
    }

    pub(crate) fn fail_unhandled_task_classes_for_active_run(
        &self,
        store: &mut WorkflowExecutionSessionStore,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<Vec<SchedulerTaskStateRecord>, WorkflowSchedulerTaskOrchestratorError> {
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
        let mut failed_records = Vec::new();
        for task in &task_graph.tasks {
            let record = records
                .iter()
                .find(|record| record.task_id.as_str() == task.task_id.as_str())
                .ok_or_else(|| {
                    WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler task '{}' has no active task-state record",
                            task.task_id.as_str()
                        )),
                    )
                })?;
            if matches!(
                record.state.kind(),
                SchedulerTaskStateKind::Completed | SchedulerTaskStateKind::TerminalFailed
            ) {
                continue;
            }
            let transition = unhandled_task_class_transition(record, task.execution_class)?;
            let failed = store
                .apply_active_run_scheduler_task_transition(session_id, workflow_run_id, transition)
                .map_err(WorkflowSchedulerTaskOrchestratorError::WorkflowService)
                .and_then(applied_task_state_record)?;
            failed_records.push(failed);
        }
        if failed_records.is_empty() {
            return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
                WorkflowServiceError::InvalidRequest(format!(
                    "active workflow run '{}' has no unhandled scheduler tasks to fail",
                    workflow_run_id
                )),
            ));
        }
        Ok(failed_records)
    }
}

fn dispatch_selected_handoff_from_selection(
    selection: SchedulerDispatchSelectionDecision,
) -> Result<SchedulerRuntimeHandoff, WorkflowSchedulerTaskOrchestratorError> {
    if selection.state != SchedulerDispatchSelectionState::Selected {
        return Err(
            WorkflowSchedulerTaskOrchestratorError::RuntimeDispatchSelectionNoSelection(selection),
        );
    }
    let Some(dispatch_decision) = selection.dispatch_decision else {
        return Err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract(
            SchedulerContractError::MissingField {
                field: "dispatch_decision",
            },
        ));
    };
    Ok(SchedulerRuntimeHandoff {
        contract_version: SCHEDULER_RUNTIME_HANDOFF_CONTRACT_VERSION,
        workflow_id: dispatch_decision.workflow_id.clone(),
        workflow_run_id: dispatch_decision.workflow_run_id.clone(),
        node_id: dispatch_decision.node_id.clone(),
        task_id: dispatch_decision.task_id.clone(),
        task_intent: dispatch_decision.task_intent.clone(),
        state: SchedulerRuntimeHandoffState::DispatchSelected,
        readiness_proof: dispatch_decision.readiness_proof.clone(),
        environment_ref: dispatch_decision.environment_ref.clone(),
        dispatch_decision: Some(dispatch_decision),
        diagnostics: Vec::new(),
    })
}

fn selected_candidate_id(
    selection_request: &SchedulerDispatchSelectionRequest,
    dispatch_decision: &SchedulerDispatchDecision,
) -> Option<SchedulerDispatchCandidateId> {
    selection_request
        .candidates
        .iter()
        .find(|candidate| {
            candidate.reservations.iter().any(|reservation| {
                reservation.reservation_lease_id == dispatch_decision.reservation_lease_id
            })
        })
        .map(|candidate| candidate.candidate_id.clone())
}

fn runtime_host_terminal_lifecycle_event(
    task: &WorkflowSchedulerTask,
    reservation_lease_id: SchedulerReservationLeaseId,
    candidate_id: Option<SchedulerDispatchCandidateId>,
    result: &WorkflowSchedulerTaskResult,
) -> Result<ReservationLifecycleEvent, WorkflowSchedulerTaskOrchestratorError> {
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => reservation_lifecycle_event(
            task,
            reservation_lease_id,
            candidate_id,
            ReservationLifecycleOutcome::RuntimeHostCompleted,
            vec![reservation_lifecycle_diagnostic(
                ReservationLifecycleDiagnosticSeverity::Info,
                ReservationLifecycleDiagnosticCode::RuntimeHostCompleted,
                "runtime host completed scheduler reservation",
            )],
        ),
        WorkflowSchedulerTaskResultStatus::Failed
        | WorkflowSchedulerTaskResultStatus::Unavailable
        | WorkflowSchedulerTaskResultStatus::Invalid => {
            let diagnostics = if result.diagnostics.is_empty() {
                vec![reservation_lifecycle_diagnostic(
                    ReservationLifecycleDiagnosticSeverity::Error,
                    ReservationLifecycleDiagnosticCode::RuntimeHostFailed,
                    "runtime host returned a failed scheduler task result",
                )]
            } else {
                result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| {
                        reservation_lifecycle_diagnostic(
                            ReservationLifecycleDiagnosticSeverity::Error,
                            ReservationLifecycleDiagnosticCode::RuntimeHostFailed,
                            diagnostic.message.clone(),
                        )
                    })
                    .collect()
            };
            reservation_lifecycle_event(
                task,
                reservation_lease_id,
                candidate_id,
                ReservationLifecycleOutcome::RuntimeHostFailed,
                diagnostics,
            )
        }
    }
}

fn reservation_lifecycle_event(
    task: &WorkflowSchedulerTask,
    reservation_lease_id: SchedulerReservationLeaseId,
    candidate_id: Option<SchedulerDispatchCandidateId>,
    outcome: ReservationLifecycleOutcome,
    diagnostics: Vec<ReservationLifecycleDiagnostic>,
) -> Result<ReservationLifecycleEvent, WorkflowSchedulerTaskOrchestratorError> {
    Ok(ReservationLifecycleEvent {
        contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
        lifecycle_event_id: reservation_lifecycle_event_id(
            task,
            &reservation_lease_id,
            candidate_id.as_ref(),
            &outcome,
        ),
        reservation_lease_id,
        workflow_id: task.workflow_id.clone(),
        workflow_run_id: task.workflow_run_id.clone(),
        node_id: task.node_id.clone(),
        task_id: task.task_id.clone(),
        outcome,
        candidate_id,
        diagnostics,
    })
}

fn reservation_lifecycle_event_id(
    task: &WorkflowSchedulerTask,
    reservation_lease_id: &SchedulerReservationLeaseId,
    candidate_id: Option<&SchedulerDispatchCandidateId>,
    outcome: &ReservationLifecycleOutcome,
) -> String {
    let hash = stable_lifecycle_hash(&[
        task.workflow_run_id.as_str(),
        task.task_id.as_str(),
        reservation_lease_id.as_str(),
        candidate_id
            .map(SchedulerDispatchCandidateId::as_str)
            .unwrap_or(""),
        reservation_lifecycle_outcome_key(outcome),
    ]);
    format!(
        "reservation.lifecycle.{:016x}.{}",
        hash,
        reservation_lifecycle_outcome_key(outcome)
    )
}

fn stable_lifecycle_hash(parts: &[&str]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for part in parts {
        for byte in part.as_bytes().iter().copied().chain([0xff]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

fn reservation_lifecycle_outcome_key(outcome: &ReservationLifecycleOutcome) -> &'static str {
    match outcome {
        ReservationLifecycleOutcome::CandidateUnselected => "candidate_unselected",
        ReservationLifecycleOutcome::CandidateRequestRejected => "candidate_request_rejected",
        ReservationLifecycleOutcome::DispatchStarted => "dispatch_started",
        ReservationLifecycleOutcome::RuntimeHostDispatchRejected => {
            "runtime_host_dispatch_rejected"
        }
        ReservationLifecycleOutcome::RuntimeHostCompleted => "runtime_host_completed",
        ReservationLifecycleOutcome::RuntimeHostFailed => "runtime_host_failed",
        ReservationLifecycleOutcome::WorkflowCancelled => "workflow_cancelled",
        ReservationLifecycleOutcome::RetryDeferred => "retry_deferred",
        ReservationLifecycleOutcome::SessionClosed => "session_closed",
        ReservationLifecycleOutcome::DuplicateReplay => "duplicate_replay",
        _ => "unknown",
    }
}

fn reservation_lifecycle_diagnostic(
    severity: ReservationLifecycleDiagnosticSeverity,
    code: ReservationLifecycleDiagnosticCode,
    message: impl Into<String>,
) -> ReservationLifecycleDiagnostic {
    ReservationLifecycleDiagnostic {
        severity,
        code,
        message: message.into(),
        hint: None,
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
            if !task.dependency_task_ids.is_empty() {
                return Ok(awaiting_inputs_state());
            }
            if let Some(task_intent) = task.schedulable_intent.clone() {
                Ok(SchedulerTaskState::WaitingDependencyReadiness {
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
    #[allow(dead_code)]
    #[error("runtime-host task result mapping failed")]
    RuntimeHostTaskResultMapping(WorkflowRuntimeHostTaskResultMappingError),
    #[allow(dead_code)]
    #[error("runtime-host task input mapping failed")]
    RuntimeHostTaskInputMapping(WorkflowRuntimeHostTaskInputMappingError),
    #[allow(dead_code)]
    #[error("scheduler dispatch selection did not select a runtime task")]
    RuntimeDispatchSelectionNoSelection(SchedulerDispatchSelectionDecision),
    #[allow(dead_code)]
    #[error("reservation lifecycle contract validation failed: {0}")]
    ReservationLifecycleContract(ReservationLifecycleContractError),
    #[allow(dead_code)]
    #[error("reservation lifecycle port failed: {0}")]
    ReservationLifecyclePort(ReservationLifecyclePortError),
    #[error("scheduler contract validation failed")]
    SchedulerContract(SchedulerContractError),
    #[error("workflow service operation failed: {0}")]
    WorkflowService(WorkflowServiceError),
    #[error("external workflow input materialization failed")]
    ExternalInputMaterialization(WorkflowExternalInputMaterializationError),
    #[error("non-runtime scheduler task execution failed")]
    NonRuntimeTaskAdapter(WorkflowSchedulerNonRuntimeTaskAdapterError),
}

#[derive(Debug)]
struct UnavailableReservationLifecyclePort;

#[async_trait]
impl ReservationLifecyclePort for UnavailableReservationLifecyclePort {
    async fn apply_reservation_lifecycle(
        &self,
        _event: ReservationLifecycleEvent,
    ) -> Result<
        pantograph_runtime_host_contracts::ReservationLifecycleApplication,
        ReservationLifecyclePortError,
    > {
        Err(ReservationLifecyclePortError::Failed {
            message: "reservation lifecycle port is not configured for workflow-service"
                .to_string(),
        })
    }
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

#[allow(dead_code)]
fn ready_runtime_execution_intent(
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskExecutionIntent, WorkflowSchedulerTaskOrchestratorError> {
    let SchedulerTaskState::Ready { execution_intent } = &record.state else {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' must be ready before runtime execution",
                record.task_id.as_str()
            )),
        ));
    };
    if execution_intent.runtime_task_intent().is_none() {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler task '{}' ready state is not runtime",
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

fn source_input_materialization_transition(
    record: &SchedulerTaskStateRecord,
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "source-input-materialized",
        SchedulerTaskStateKind::AwaitingInputs,
        SchedulerTaskState::Completed {
            execution_intent: source_input_execution_intent(task)?,
        },
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

fn runtime_result_transition_from_running(
    record: &SchedulerTaskStateRecord,
    result: &WorkflowSchedulerTaskResult,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => completion_transition_from_running(record),
        WorkflowSchedulerTaskResultStatus::Failed
        | WorkflowSchedulerTaskResultStatus::Unavailable
        | WorkflowSchedulerTaskResultStatus::Invalid => {
            terminal_failure_transition_from_running_diagnostics(
                record,
                runtime_result_failure_diagnostics(result),
            )
        }
    }
}

fn terminal_failure_transition_from_running(
    record: &SchedulerTaskStateRecord,
    diagnostic: SchedulerTaskStateDiagnostic,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    terminal_failure_transition_from_running_diagnostics(record, vec![diagnostic])
}

fn terminal_failure_transition_from_running_diagnostics(
    record: &SchedulerTaskStateRecord,
    diagnostics: Vec<SchedulerTaskStateDiagnostic>,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "terminal-failed",
        SchedulerTaskStateKind::Running,
        SchedulerTaskState::TerminalFailed { diagnostics },
    )
}

fn runtime_result_failure_diagnostics(
    result: &WorkflowSchedulerTaskResult,
) -> Vec<SchedulerTaskStateDiagnostic> {
    if result.diagnostics.is_empty() {
        return vec![SchedulerTaskStateDiagnostic {
            severity: SchedulerTaskStateDiagnosticSeverity::Error,
            code: SchedulerTaskStateDiagnosticCode::TerminalFailure,
            message: format!(
                "runtime host returned {:?} for scheduler task '{}'",
                result.status, result.task_id
            ),
            hint: Some("Inspect the runtime-host task result diagnostics.".to_string()),
        }];
    }
    result
        .diagnostics
        .iter()
        .map(|diagnostic| SchedulerTaskStateDiagnostic {
            severity: SchedulerTaskStateDiagnosticSeverity::Error,
            code: SchedulerTaskStateDiagnosticCode::TerminalFailure,
            message: diagnostic.message.clone(),
            hint: Some(diagnostic.code.clone()),
        })
        .collect()
}

fn runtime_dispatch_selection_task_diagnostics(
    selection: &SchedulerDispatchSelectionDecision,
) -> Vec<SchedulerTaskStateDiagnostic> {
    if selection.diagnostics.is_empty() {
        return vec![SchedulerTaskStateDiagnostic {
            severity: SchedulerTaskStateDiagnosticSeverity::Error,
            code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
            message: "runtime scheduler dispatch selection did not select a candidate".to_string(),
            hint: Some(
                "Provide canonical runtime, device, model, reservation, and resource-fit candidates before runtime-host dispatch."
                    .to_string(),
            ),
        }];
    }
    selection
        .diagnostics
        .iter()
        .map(runtime_dispatch_selection_task_diagnostic)
        .collect()
}

fn runtime_dispatch_selection_task_diagnostic(
    diagnostic: &SchedulerDispatchSelectionDiagnostic,
) -> SchedulerTaskStateDiagnostic {
    SchedulerTaskStateDiagnostic {
        severity: match diagnostic.severity {
            SchedulerDispatchSelectionDiagnosticSeverity::Info => {
                SchedulerTaskStateDiagnosticSeverity::Info
            }
            SchedulerDispatchSelectionDiagnosticSeverity::Warning => {
                SchedulerTaskStateDiagnosticSeverity::Warning
            }
            SchedulerDispatchSelectionDiagnosticSeverity::Error => {
                SchedulerTaskStateDiagnosticSeverity::Error
            }
            _ => SchedulerTaskStateDiagnosticSeverity::Error,
        },
        code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
        message: format!(
            "runtime dispatch selection {:?}: {}",
            diagnostic.code, diagnostic.message
        ),
        hint: diagnostic.hint.clone().or_else(|| {
            Some(
                "Resolve scheduler dispatch candidate diagnostics before runtime-host dispatch."
                    .to_string(),
            )
        }),
    }
}

fn runtime_dispatch_not_wired_transition(
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "runtime-dispatch-not-wired",
        record.state.kind(),
        SchedulerTaskState::TerminalFailed {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
                message:
                    "runtime scheduler task dispatch is not wired through scheduler-selected runtime-host handoff"
                        .to_string(),
                hint: Some(
                    "Wire this task through a dispatch-selected SchedulerRuntimeHandoff before executing runtime inference."
                        .to_string(),
                ),
            }],
        },
    )
}

fn unhandled_task_class_transition(
    record: &SchedulerTaskStateRecord,
    execution_class: WorkflowSchedulerTaskExecutionClass,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "unhandled-task-class",
        record.state.kind(),
        SchedulerTaskState::TerminalFailed {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
                message: format!(
                    "scheduler task class '{execution_class:?}' has no execution path in the scheduler-task session runner"
                ),
                hint: Some(
                    "Add a typed scheduler execution path for this task class before running the workflow."
                        .to_string(),
                ),
            }],
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

#[allow(dead_code)]
fn waiting_dependency_readiness_transition_from_awaiting_inputs(
    task: &WorkflowSchedulerTask,
    record: &SchedulerTaskStateRecord,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    task_state_transition(
        record,
        "runtime-inputs-ready",
        SchedulerTaskStateKind::AwaitingInputs,
        SchedulerTaskState::WaitingDependencyReadiness {
            execution_intent: runtime_execution_intent(task)?,
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

fn readiness_admission_transition_from_waiting(
    record: &SchedulerTaskStateRecord,
    execution_intent: SchedulerTaskExecutionIntent,
    decision: SchedulerReadinessAdmissionDecision,
) -> Result<SchedulerTaskStateTransition, WorkflowSchedulerTaskOrchestratorError> {
    let next_state = match decision.state {
        SchedulerReadinessAdmissionState::Ready => SchedulerTaskState::Ready { execution_intent },
        SchedulerReadinessAdmissionState::Deferred => SchedulerTaskState::PausedDeferred {
            execution_intent,
            diagnostics: readiness_admission_diagnostics(
                SchedulerReadinessAdmissionState::Deferred,
                decision.diagnostics,
            ),
        },
        SchedulerReadinessAdmissionState::RetryableFailed => SchedulerTaskState::RetryableFailed {
            execution_intent,
            diagnostics: readiness_admission_diagnostics(
                SchedulerReadinessAdmissionState::RetryableFailed,
                decision.diagnostics,
            ),
        },
        SchedulerReadinessAdmissionState::TerminalFailed => SchedulerTaskState::TerminalFailed {
            diagnostics: readiness_admission_diagnostics(
                SchedulerReadinessAdmissionState::TerminalFailed,
                decision.diagnostics,
            ),
        },
        _ => SchedulerTaskState::TerminalFailed {
            diagnostics: vec![SchedulerTaskStateDiagnostic {
                severity: SchedulerTaskStateDiagnosticSeverity::Error,
                code: SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
                message: "scheduler readiness admission returned an unsupported state"
                    .to_string(),
                hint: Some(
                    "Update workflow-service readiness admission mapping before executing runtime tasks."
                        .to_string(),
                ),
            }],
        },
    };
    task_state_transition(
        record,
        "dependency-readiness",
        SchedulerTaskStateKind::WaitingDependencyReadiness,
        next_state,
    )
}

fn readiness_admission_diagnostics(
    state: SchedulerReadinessAdmissionState,
    diagnostics: Vec<SchedulerReadinessAdmissionDiagnostic>,
) -> Vec<SchedulerTaskStateDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| readiness_admission_diagnostic(&state, diagnostic))
        .collect()
}

fn readiness_admission_diagnostic(
    state: &SchedulerReadinessAdmissionState,
    diagnostic: SchedulerReadinessAdmissionDiagnostic,
) -> SchedulerTaskStateDiagnostic {
    SchedulerTaskStateDiagnostic {
        severity: match diagnostic.severity {
            SchedulerReadinessAdmissionSeverity::Info => SchedulerTaskStateDiagnosticSeverity::Info,
            SchedulerReadinessAdmissionSeverity::Warning => {
                SchedulerTaskStateDiagnosticSeverity::Warning
            }
            SchedulerReadinessAdmissionSeverity::Error => {
                SchedulerTaskStateDiagnosticSeverity::Error
            }
            _ => SchedulerTaskStateDiagnosticSeverity::Error,
        },
        code: match diagnostic.code {
            SchedulerReadinessAdmissionDiagnosticCode::DependencyNotReady
            | SchedulerReadinessAdmissionDiagnosticCode::DependencyPolicyRejected
            | SchedulerReadinessAdmissionDiagnosticCode::MissingReadinessProof
            | SchedulerReadinessAdmissionDiagnosticCode::StaleReadinessProof => {
                SchedulerTaskStateDiagnosticCode::TaskDeferred
            }
            SchedulerReadinessAdmissionDiagnosticCode::DependencyUnavailable => {
                if *state == SchedulerReadinessAdmissionState::RetryableFailed {
                    SchedulerTaskStateDiagnosticCode::RetryableFailure
                } else {
                    SchedulerTaskStateDiagnosticCode::TerminalFailure
                }
            }
            SchedulerReadinessAdmissionDiagnosticCode::InvalidReadinessProof
            | SchedulerReadinessAdmissionDiagnosticCode::SchedulerPolicyError => {
                SchedulerTaskStateDiagnosticCode::SchedulerPolicyError
            }
            _ => SchedulerTaskStateDiagnosticCode::SchedulerPolicyError,
        },
        message: diagnostic.message,
        hint: diagnostic.hint,
    }
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

fn source_input_execution_intent(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskExecutionIntent, WorkflowSchedulerTaskOrchestratorError> {
    Ok(SchedulerTaskExecutionIntent::SourceInput {
        task_intent: SchedulerSourceInputTaskIntent {
            contract_version: SCHEDULER_TASK_STATE_CONTRACT_VERSION,
            workflow_id: task.workflow_id.clone(),
            workflow_run_id: task.workflow_run_id.clone(),
            node_id: task.node_id.clone(),
            task_id: task.task_id.clone(),
            task_kind: source_input_task_kind(task)?,
        },
    })
}

fn source_input_task_kind(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerSourceInputTaskKind, WorkflowSchedulerTaskOrchestratorError> {
    let Some(template) = task.source_input_task_template.as_ref() else {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "scheduler source-input task '{}' has no typed source-input template",
                task.task_id.as_str()
            )),
        ));
    };
    let task_kind = match template {
        WorkflowSchedulerSourceInputTemplate::Text { .. } => "text-input",
        WorkflowSchedulerSourceInputTemplate::Boolean { .. } => "boolean-input",
    };
    SchedulerSourceInputTaskKind::parse(task_kind)
        .map_err(WorkflowSchedulerTaskOrchestratorError::SchedulerContract)
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

#[allow(dead_code)]
enum RuntimeInputReadiness {
    Ready,
    Blocked,
    InputUnavailable(SchedulerTaskStateDiagnostic),
    Invalid(SchedulerTaskStateDiagnostic),
}

#[allow(dead_code)]
fn runtime_input_readiness(
    task: &WorkflowSchedulerTask,
    results: &[WorkflowSchedulerTaskResult],
) -> RuntimeInputReadiness {
    if task.schedulable_intent.is_none() {
        return RuntimeInputReadiness::Invalid(scheduler_input_diagnostic(
            SchedulerTaskStateDiagnosticCode::InvalidTask,
            "runtime scheduler task is missing a typed runtime execution intent",
        ));
    }
    for binding in &task.input_bindings {
        match materialized_bound_output(task, results, binding) {
            MaterializedBindingValue::Ready(_) => {}
            MaterializedBindingValue::Blocked => return RuntimeInputReadiness::Blocked,
            MaterializedBindingValue::Unavailable(diagnostic) => {
                return RuntimeInputReadiness::InputUnavailable(diagnostic);
            }
            MaterializedBindingValue::Invalid(diagnostic) => {
                return RuntimeInputReadiness::Invalid(diagnostic);
            }
        }
    }
    RuntimeInputReadiness::Ready
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

#[allow(dead_code)]
fn runtime_execution_intent(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskExecutionIntent, WorkflowSchedulerTaskOrchestratorError> {
    let Some(task_intent) = task.schedulable_intent.clone() else {
        return Err(WorkflowSchedulerTaskOrchestratorError::WorkflowService(
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' is missing a typed runtime execution intent",
                task.task_id.as_str()
            )),
        ));
    };
    Ok(SchedulerTaskExecutionIntent::Runtime { task_intent })
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
