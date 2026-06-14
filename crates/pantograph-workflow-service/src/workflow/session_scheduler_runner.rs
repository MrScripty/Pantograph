use std::time::Instant;

use pantograph_dependency_environment_service::{
    DependencyReadinessDiagnosticContext, DependencyReadinessTaskId, DependencyReadinessWorkItem,
    DependencyReadinessWorkItemProvenance, DependencyReadinessWorkQueueError,
    DependencyReadinessWorkflowRunId, DependencyReadinessWorkflowSessionId,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentRequest, DependencyReadinessPolicy,
    DependencyReadinessProofEnvelope, ValidatedDependencyEnvironmentRequest,
    ValidatedDependencyReadinessRequestEnvelope,
};
use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    SchedulerTaskAttemptExecutionClass, SchedulerTaskAttemptLifecycleChangedPayload,
    SchedulerTaskAttemptLifecycleTransition,
};
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord};

use crate::scheduler::task_orchestrator::{
    SelectedRuntimeTaskDispatch, StartedRuntimeTaskExecution,
};
use crate::scheduler::{
    WorkflowDependencyReadinessLifecycle, WorkflowDependencyReadinessLifecycleError,
    WorkflowSchedulerRetryLifecycle, WorkflowSchedulerTaskTerminalMutation,
};

use super::io_contract::validate_workflow_io;
use super::runtime_branch_run_finalization::{
    scheduler_task_attempt_terminal_diagnostic_event,
    WorkflowSchedulerTaskAttemptDiagnosticAttribution,
    WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest,
};
use super::validation::{
    validate_host_output_bindings, validate_output_targets_against_io,
    validate_requested_outputs_produced,
};
use super::{
    project_scheduler_task_results_to_outputs, WorkflowHost, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowRunResponse, WorkflowRuntimeDispatchPreselectionError,
    WorkflowRuntimeDispatchSelectionBoundary, WorkflowSchedulerTask,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskRunSummary, WorkflowService,
    WorkflowServiceError,
};

pub(super) struct WorkflowSchedulerSessionRunner<'a> {
    service: &'a WorkflowService,
}

pub(super) struct AdmittedRuntimeTaskReadiness {
    task_id: String,
    readiness_proof: DependencyReadinessProofEnvelope,
}

struct RuntimeDependencyReadinessAdmissionResult {
    admitted: Vec<AdmittedRuntimeTaskReadiness>,
    deferred_task_ids: Vec<String>,
}

pub(super) struct WorkflowPreDispatchPreparationOutcome {
    admitted_runtime_readiness: Vec<AdmittedRuntimeTaskReadiness>,
    deferred_task_ids: Vec<String>,
}

pub(super) struct WorkflowStartedRuntimeDispatchAttempt {
    pub(super) started_runtime_task: StartedRuntimeTaskExecution,
    pub(super) selected_dispatch: SelectedRuntimeTaskDispatch,
    pub(super) selected_candidate_fact: super::WorkflowRuntimeDispatchCandidateFact,
}

pub(super) struct WorkflowPreDispatchPreparationBoundary<'a> {
    service: &'a WorkflowService,
}

impl<'a> WorkflowPreDispatchPreparationBoundary<'a> {
    pub(super) fn new(service: &'a WorkflowService) -> Self {
        Self { service }
    }

    pub(super) fn materialize_external_inputs(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        self.runner()
            .materialize_external_inputs(session_id, workflow_run_id, inputs)
    }

    pub(super) async fn run_progress_loop(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.runner()
            .run_progress_loop(session_id, workflow_run_id)
            .await
    }

    pub(super) async fn prepare_runtime_dispatch(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<WorkflowPreDispatchPreparationOutcome, WorkflowServiceError> {
        let runner = self.runner();
        runner
            .run_progress_loop(session_id, workflow_run_id)
            .await?;
        runner.retry_deferred_runtime_dependency_readiness(session_id, workflow_run_id)?;
        let readiness_admission =
            runner.admit_runtime_dependency_readiness(session_id, workflow_run_id)?;
        if readiness_admission.deferred_task_ids.is_empty() {
            runner.ensure_runtime_tasks_ready_for_dispatch(session_id, workflow_run_id)?;
        }
        Ok(WorkflowPreDispatchPreparationOutcome {
            admitted_runtime_readiness: readiness_admission.admitted,
            deferred_task_ids: readiness_admission.deferred_task_ids,
        })
    }

    pub(super) async fn start_runtime_branch_dispatch_attempt(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
        admitted_runtime_readiness: &[AdmittedRuntimeTaskReadiness],
        attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
    ) -> Result<WorkflowStartedRuntimeDispatchAttempt, WorkflowServiceError> {
        let readiness_proof = runtime_dispatch_readiness_proof(
            self.service,
            session_id,
            workflow_run_id,
            task_id,
            admitted_runtime_readiness,
        )?;
        let dispatch_context =
            ready_runtime_dispatch_context(self.service, session_id, workflow_run_id, task_id)?;
        let runtime_dispatch_selection_boundary =
            WorkflowRuntimeDispatchSelectionBoundary::from_service(self.service);
        let prepared_dispatch_selection = runtime_dispatch_selection_boundary
            .prepare_ready_runtime_task_dispatch(
                &dispatch_context.task,
                &dispatch_context.ready_record,
                readiness_proof,
            )
            .await
            .map_err(runtime_dispatch_preselection_invalid_request)?;
        let started_runtime_task = {
            let mut store = self.service.session_store_guard()?;
            self.service
                .scheduler_task_orchestrator
                .start_ready_runtime_task(&mut store, session_id, workflow_run_id, task_id)
                .map_err(|error| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler runtime task start failed: {error}"
                    ))
                })?
        };
        WorkflowSchedulerSessionRunner::new(self.service).record_scheduler_task_attempt_started(
            session_id,
            started_runtime_task.task(),
            started_runtime_task.attempt_id().as_str(),
            started_runtime_task.started_at_ms(),
            attempt_start_transition,
        )?;
        let preselection = runtime_dispatch_selection_boundary
            .select_prepared_started_runtime_task_dispatch(
                &started_runtime_task,
                prepared_dispatch_selection,
            )
            .await
            .map_err(runtime_dispatch_preselection_invalid_request)?;
        {
            let mut store = self.service.session_store_guard()?;
            self.service
                .scheduler_task_orchestrator
                .bind_started_runtime_task_reservation(
                    &mut store,
                    session_id,
                    workflow_run_id,
                    &started_runtime_task,
                    &preselection.selected_dispatch,
                )
                .map_err(|error| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler runtime task reservation binding failed: {error}"
                    ))
                })?;
        }
        Ok(WorkflowStartedRuntimeDispatchAttempt {
            started_runtime_task,
            selected_dispatch: preselection.selected_dispatch,
            selected_candidate_fact: preselection.selected_candidate_fact,
        })
    }

    fn runner(&self) -> WorkflowSchedulerSessionRunner<'a> {
        WorkflowSchedulerSessionRunner::new(self.service)
    }
}

impl WorkflowPreDispatchPreparationOutcome {
    pub(super) fn admitted_runtime_readiness(&self) -> &[AdmittedRuntimeTaskReadiness] {
        &self.admitted_runtime_readiness
    }

    pub(super) fn deferred_task_ids(&self) -> &[String] {
        &self.deferred_task_ids
    }

    pub(super) fn into_deferred_task_ids(self) -> Vec<String> {
        self.deferred_task_ids
    }
}

struct ReadyRuntimeDispatchContext {
    task: WorkflowSchedulerTask,
    ready_record: SchedulerTaskStateRecord,
}

impl<'a> WorkflowSchedulerSessionRunner<'a> {
    pub(super) fn new(service: &'a WorkflowService) -> Self {
        Self { service }
    }

    pub(super) async fn run_non_runtime_only<H: WorkflowHost + ?Sized>(
        &self,
        host: &H,
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: Instant,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if !summary.is_non_runtime_only() || summary.has_runtime_inference() {
            return Err(WorkflowServiceError::Internal(
                "scheduler session runner received a runtime-containing run".to_string(),
            ));
        }
        let preparation = WorkflowPreDispatchPreparationBoundary::new(self.service);
        preparation.materialize_external_inputs(session_id, workflow_run_id, inputs)?;
        preparation
            .run_progress_loop(session_id, workflow_run_id)
            .await?;

        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        ensure_all_scheduler_tasks_completed(&records)?;
        let results = {
            let mut store = self.service.session_store_guard()?;
            store.active_run_scheduler_task_results(session_id, workflow_run_id)?
        };
        let targets = scheduler_output_targets_for_run(host, workflow_id, output_targets).await?;
        let outputs = project_scheduler_task_results_to_outputs(&task_graph, &results, &targets)
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task output projection failed: {error}"
                ))
            })?;
        validate_host_output_bindings(&outputs, "outputs")?;
        validate_requested_outputs_produced(&targets, &outputs)?;
        Ok(WorkflowRunResponse {
            workflow_run_id: workflow_run_id.to_string(),
            outputs,
            timing_ms: started_at.elapsed().as_millis(),
        })
    }

    pub(super) async fn resume_runtime_dependency_readiness(
        &self,
        host: &(impl WorkflowHost + ?Sized),
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: Instant,
        attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if !summary.has_runtime_inference() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow run '{}' is not a runtime inference run",
                workflow_run_id
            )));
        }
        self.continue_runtime_dependency_readiness(
            host,
            session_id,
            workflow_run_id,
            workflow_id,
            output_targets,
            summary,
            started_at,
            attempt_start_transition,
        )
        .await
    }

    pub(super) async fn resume_progress_loop(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        WorkflowPreDispatchPreparationBoundary::new(self.service)
            .run_progress_loop(session_id, workflow_run_id)
            .await
    }

    async fn continue_runtime_dependency_readiness(
        &self,
        host: &(impl WorkflowHost + ?Sized),
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: Instant,
        attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let preparation = WorkflowPreDispatchPreparationBoundary::new(self.service)
            .prepare_runtime_dispatch(session_id, workflow_run_id)
            .await?;
        if !preparation.deferred_task_ids.is_empty() {
            let deferred_task_ids = preparation.into_deferred_task_ids();
            return Err(WorkflowServiceError::RuntimeDependencyReadinessPending {
                message: format!(
                    "runtime dependency readiness is pending for scheduler task(s): {}",
                    deferred_task_ids.join(", ")
                ),
                task_ids: deferred_task_ids,
            });
        }
        self.run_runtime_dispatch_ready_tasks(
            host,
            session_id,
            workflow_run_id,
            workflow_id,
            output_targets,
            summary,
            started_at,
            preparation.admitted_runtime_readiness(),
            attempt_start_transition,
        )
        .await
    }

    fn materialize_external_inputs(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        inputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        let mut store = self.service.session_store_guard()?;
        self.service
            .scheduler_task_orchestrator
            .materialize_external_inputs_for_active_run(
                &mut store,
                session_id,
                workflow_run_id,
                inputs,
            )
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler source-input materialization failed: {error}"
                ))
            })?;
        Ok(())
    }

    async fn run_progress_loop(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let mut progressed = true;
        while progressed {
            progressed = self.advance_awaiting_task_inputs(session_id, workflow_run_id)?;
            progressed |= self
                .execute_ready_non_runtime_tasks(session_id, workflow_run_id)
                .await?;
        }
        Ok(())
    }

    fn advance_awaiting_task_inputs(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<bool, WorkflowServiceError> {
        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        let mut progressed = false;
        for record in records
            .iter()
            .filter(|record| record.state.kind() == SchedulerTaskStateKind::AwaitingInputs)
        {
            let Some(task) = task_graph
                .tasks
                .iter()
                .find(|task| task.task_id.as_str() == record.task_id.as_str())
            else {
                return Err(WorkflowServiceError::Internal(format!(
                    "scheduler task '{}' has state but no task graph entry",
                    record.task_id.as_str()
                )));
            };
            let advanced = match task.execution_class {
                WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => {
                    let mut store = self.service.session_store_guard()?;
                    self.service
                        .scheduler_task_orchestrator
                        .advance_awaiting_non_runtime_task_inputs(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            record.task_id.as_str(),
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler non-runtime input readiness failed: {error}"
                            ))
                        })?
                }
                WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
                    let mut store = self.service.session_store_guard()?;
                    self.service
                        .scheduler_task_orchestrator
                        .advance_awaiting_runtime_task_inputs(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            record.task_id.as_str(),
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler runtime input readiness failed: {error}"
                            ))
                        })?
                }
                _ => None,
            };
            progressed |= advanced.is_some();
        }
        Ok(progressed)
    }

    async fn execute_ready_non_runtime_tasks(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<bool, WorkflowServiceError> {
        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        let ready_task_ids = records
            .iter()
            .filter(|record| {
                record.state.kind() == SchedulerTaskStateKind::Ready
                    && task_graph.tasks.iter().any(|task| {
                        task.task_id.as_str() == record.task_id.as_str()
                            && task.execution_class
                                == WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine
                    })
            })
            .map(|record| record.task_id.as_str().to_string())
            .collect::<Vec<_>>();
        let mut progressed = false;
        for task_id in ready_task_ids {
            let started = {
                let mut store = self.service.session_store_guard()?;
                self.service
                    .scheduler_task_orchestrator
                    .start_ready_non_runtime_task(&mut store, session_id, workflow_run_id, &task_id)
                    .map_err(|error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler non-runtime task start failed: {error}"
                        ))
                    })?
            };
            self.record_scheduler_task_attempt_started(
                session_id,
                started.task(),
                started.attempt_id().as_str(),
                started.started_at_ms(),
                SchedulerTaskAttemptLifecycleTransition::Started,
            )?;
            let execution_result = self
                .service
                .scheduler_task_orchestrator
                .execute_started_non_runtime_task(&started)
                .await;
            match execution_result {
                Ok(result) => {
                    let mut store = self.service.session_store_guard()?;
                    self.service
                        .scheduler_task_orchestrator
                        .complete_started_non_runtime_task(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            &started,
                            result,
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler non-runtime task completion failed: {error}"
                            ))
                        })?;
                    self.record_scheduler_task_attempt_terminal(
                        session_id,
                        started.task(),
                        started.attempt_id().as_str(),
                        started.started_at_ms(),
                        SchedulerTaskAttemptLifecycleTransition::Completed,
                        "scheduler task attempt completed",
                        None,
                        None,
                        None,
                    )?;
                }
                Err(
                    crate::scheduler::WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter(
                        error,
                    ),
                ) => {
                    let mut store = self.service.session_store_guard()?;
                    let failed = self
                        .service
                        .scheduler_task_orchestrator
                        .fail_started_non_runtime_task(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            &started,
                            &error,
                        );
                    if failed.is_ok() {
                        self.record_scheduler_task_attempt_terminal(
                            session_id,
                            started.task(),
                            started.attempt_id().as_str(),
                            started.started_at_ms(),
                            SchedulerTaskAttemptLifecycleTransition::Failed,
                            "scheduler non-runtime task execution failed",
                            Some(error.to_string()),
                            None,
                            None,
                        )?;
                    }
                    return Err(WorkflowServiceError::InvalidRequest(format!(
                        "scheduler non-runtime task execution failed: {error}"
                    )));
                }
                Err(error) => {
                    return Err(WorkflowServiceError::InvalidRequest(format!(
                        "scheduler non-runtime task execution failed: {error}"
                    )));
                }
            }
            progressed = true;
        }
        Ok(progressed)
    }

    fn admit_runtime_dependency_readiness(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<RuntimeDependencyReadinessAdmissionResult, WorkflowServiceError> {
        let runtime_task_ids =
            runtime_task_ids_in_state(self.service, session_id, workflow_run_id, |kind| {
                kind == SchedulerTaskStateKind::WaitingDependencyReadiness
            })?;
        let lifecycle = WorkflowDependencyReadinessLifecycle::new(
            self.service.scheduler_task_orchestrator.clone(),
        );
        let mut admitted_runtime_readiness = Vec::with_capacity(runtime_task_ids.len());
        let mut deferred_task_ids = Vec::new();
        for task_id in runtime_task_ids {
            let request = {
                let store = self.service.session_store_guard()?;
                lifecycle
                    .readiness_request_for_active_runtime_task(
                        &store,
                        session_id,
                        workflow_run_id,
                        &task_id,
                        DependencyReadinessPolicy::CheckOnly,
                    )
                    .map_err(dependency_readiness_error)?
            };
            let seed_result = match lifecycle
                .resolve_dependency_requirements_seed(
                    self.service.dependency_readiness_provider.as_ref(),
                    &request,
                )
                .map_err(dependency_readiness_error)?
            {
                Some(seed_result) => seed_result,
                None => {
                    self.defer_runtime_dependency_readiness(
                        &lifecycle,
                        session_id,
                        workflow_run_id,
                        &task_id,
                        &request,
                    )?;
                    deferred_task_ids.push(task_id);
                    continue;
                }
            };
            if self
                .service
                .store_dependency_requirements_payload_from_result(&seed_result)
                .is_err()
            {
                self.defer_runtime_dependency_readiness(
                    &lifecycle,
                    session_id,
                    workflow_run_id,
                    &task_id,
                    &request,
                )?;
                deferred_task_ids.push(task_id);
                continue;
            }
            let work_item = dependency_readiness_work_item(
                session_id,
                workflow_run_id,
                &task_id,
                dependency_environment_request_from_readiness_envelope(&request)
                    .map_err(dependency_readiness_work_queue_error)?,
            )
            .map_err(dependency_readiness_work_queue_error)?;
            self.service
                .dependency_readiness_work_queue
                .enqueue(work_item);
            let readiness_proof = lifecycle
                .resolve_dependency_readiness_proof(
                    self.service.dependency_readiness_provider.as_ref(),
                    &request,
                )
                .map_err(dependency_readiness_error)?;
            let readiness_proof_for_dispatch = readiness_proof.clone();
            let mut store = self.service.session_store_guard()?;
            let admitted_record = lifecycle
                .admit_active_runtime_task(
                    &mut store,
                    session_id,
                    workflow_run_id,
                    &task_id,
                    DependencyReadinessPolicy::CheckOnly,
                    readiness_proof,
                )
                .map_err(dependency_readiness_error)?;
            if admitted_record.state.kind() == SchedulerTaskStateKind::Ready {
                let readiness_proof = readiness_proof_for_dispatch.ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "runtime scheduler task '{}' has no dependency readiness proof for dispatch selection",
                        task_id
                    ))
                })?;
                admitted_runtime_readiness.push(AdmittedRuntimeTaskReadiness {
                    task_id,
                    readiness_proof,
                });
            } else {
                deferred_task_ids.push(task_id);
            }
        }
        Ok(RuntimeDependencyReadinessAdmissionResult {
            admitted: admitted_runtime_readiness,
            deferred_task_ids,
        })
    }

    fn retry_deferred_runtime_dependency_readiness(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let retry_lifecycle = WorkflowSchedulerRetryLifecycle::new(
            self.service
                .scheduler_task_orchestrator
                .scheduler_lifecycle_handle(),
        );
        retry_lifecycle.run_retry_loop(|| {
            let runtime_task_ids =
                runtime_task_ids_in_state(self.service, session_id, workflow_run_id, |kind| {
                    matches!(
                        kind,
                        SchedulerTaskStateKind::PausedDeferred
                            | SchedulerTaskStateKind::RetryableFailed
                    )
                })?;
            for task_id in runtime_task_ids {
                let mut store = self.service.session_store_guard()?;
                self.service
                    .scheduler_task_orchestrator
                    .retry_deferred_runtime_dependency_readiness(
                        &mut store,
                        session_id,
                        workflow_run_id,
                        &task_id,
                    )
                    .map_err(|error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler dependency readiness retry failed: {error}"
                        ))
                    })?;
            }
            Ok(())
        })
    }

    fn defer_runtime_dependency_readiness(
        &self,
        lifecycle: &WorkflowDependencyReadinessLifecycle,
        session_id: &str,
        workflow_run_id: &str,
        task_id: &str,
        request: &ValidatedDependencyReadinessRequestEnvelope,
    ) -> Result<(), WorkflowServiceError> {
        let work_item = dependency_readiness_work_item(
            session_id,
            workflow_run_id,
            task_id,
            dependency_environment_request_from_readiness_envelope(request)
                .map_err(dependency_readiness_work_queue_error)?,
        )
        .map_err(dependency_readiness_work_queue_error)?;
        self.service
            .dependency_readiness_work_queue
            .enqueue(work_item);
        let mut store = self.service.session_store_guard()?;
        lifecycle
            .admit_active_runtime_task(
                &mut store,
                session_id,
                workflow_run_id,
                task_id,
                DependencyReadinessPolicy::CheckOnly,
                None,
            )
            .map_err(dependency_readiness_error)?;
        Ok(())
    }

    fn ensure_runtime_tasks_ready_for_dispatch(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        for task in &task_graph.tasks {
            let record = records
                .iter()
                .find(|record| record.task_id.as_str() == task.task_id.as_str())
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler task '{}' has no active task-state record",
                        task.task_id.as_str()
                    ))
                })?;
            match task.execution_class {
                WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
                    if record.state.kind() != SchedulerTaskStateKind::Ready {
                        return Err(WorkflowServiceError::CapabilityViolation(format!(
                            "runtime scheduler task '{}' was not admitted for dispatch; final state was {:?}",
                            record.task_id.as_str(),
                            record.state.kind()
                        )));
                    }
                }
                _ => {
                    if record.state.kind() != SchedulerTaskStateKind::Completed {
                        return Err(WorkflowServiceError::InvalidRequest(format!(
                            "scheduler task '{}' did not complete before runtime dispatch boundary; final state was {:?}",
                            record.task_id.as_str(),
                            record.state.kind()
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    async fn run_runtime_dispatch_ready_tasks(
        &self,
        host: &(impl WorkflowHost + ?Sized),
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: Instant,
        admitted_runtime_readiness: &[AdmittedRuntimeTaskReadiness],
        attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let runtime_task_ids =
            runtime_task_ids_in_state(self.service, session_id, workflow_run_id, |kind| {
                kind == SchedulerTaskStateKind::Ready
            })?;
        let runtime_dispatch_selection_boundary =
            WorkflowRuntimeDispatchSelectionBoundary::from_service(self.service);
        for task_id in &runtime_task_ids {
            let readiness_proof = runtime_dispatch_readiness_proof(
                self.service,
                session_id,
                workflow_run_id,
                task_id,
                admitted_runtime_readiness,
            )?;
            let dispatch_context =
                ready_runtime_dispatch_context(self.service, session_id, workflow_run_id, task_id)?;
            let prepared_dispatch_selection = runtime_dispatch_selection_boundary
                .prepare_ready_runtime_task_dispatch(
                    &dispatch_context.task,
                    &dispatch_context.ready_record,
                    readiness_proof,
                )
                .await
                .map_err(runtime_dispatch_preselection_invalid_request)?;
            let started_runtime_task = {
                let mut store = self.service.session_store_guard()?;
                self.service
                    .scheduler_task_orchestrator
                    .start_ready_runtime_task(&mut store, session_id, workflow_run_id, task_id)
                    .map_err(|error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler runtime task start failed: {error}"
                        ))
                    })?
            };
            self.record_scheduler_task_attempt_started(
                session_id,
                started_runtime_task.task(),
                started_runtime_task.attempt_id().as_str(),
                started_runtime_task.started_at_ms(),
                attempt_start_transition,
            )?;
            let preselection = runtime_dispatch_selection_boundary
                .select_prepared_started_runtime_task_dispatch(
                    &started_runtime_task,
                    prepared_dispatch_selection,
                )
                .await;
            let preselection = match preselection {
                Ok(preselection) => preselection,
                Err(error) => {
                    let Some(scheduler_error) = error.scheduler_selection_error() else {
                        return Err(runtime_dispatch_preselection_invalid_request(error));
                    };
                    if let crate::scheduler::WorkflowSchedulerTaskOrchestratorError::RuntimeDispatchSelectionNoSelection(selection) = scheduler_error {
                        let terminal_mutation = {
                            let mut store = self.service.session_store_guard()?;
                            self.service
                                .scheduler_task_orchestrator
                                .fail_started_runtime_task_dispatch_selection_terminal_mutation(
                                    &mut store,
                                    session_id,
                                    workflow_run_id,
                                    &started_runtime_task,
                                    selection,
                                )
                                .map_err(|error| {
                                    WorkflowServiceError::InvalidRequest(format!(
                                        "scheduler runtime dispatch no-selection transition failed: {error}"
                                    ))
                                })?
                        };
                        self.record_scheduler_task_attempt_terminal(
                            session_id,
                            started_runtime_task.task(),
                            started_runtime_task.attempt_id().as_str(),
                            started_runtime_task.started_at_ms(),
                            SchedulerTaskAttemptLifecycleTransition::Failed,
                            "scheduler runtime dispatch selection failed",
                            Some(scheduler_error.to_string()),
                            None,
                            Some(&terminal_mutation),
                        )?;
                    } else {
                        let terminal_mutation = {
                            let mut store = self.service.session_store_guard()?;
                        self.service
                            .scheduler_task_orchestrator
                            .fail_started_runtime_task_dispatch_error_terminal_mutation(
                                &mut store,
                                session_id,
                                workflow_run_id,
                                &started_runtime_task,
                                scheduler_error,
                            )
                            .map_err(|error| {
                                WorkflowServiceError::InvalidRequest(format!(
                                    "scheduler runtime dispatch error transition failed: {error}"
                                ))
                            })?
                        };
                        self.record_scheduler_task_attempt_terminal(
                            session_id,
                            started_runtime_task.task(),
                            started_runtime_task.attempt_id().as_str(),
                            started_runtime_task.started_at_ms(),
                            SchedulerTaskAttemptLifecycleTransition::Failed,
                            "scheduler runtime dispatch failed",
                            Some(scheduler_error.to_string()),
                            None,
                            Some(&terminal_mutation),
                        )?;
                    }
                    return Err(WorkflowServiceError::CapabilityViolation(format!(
                        "runtime scheduler dispatch selection failed closed for {count} runtime inference task(s): {scheduler_error}",
                        count = summary.runtime_inference_tasks
                    )));
                }
            };
            let selected_dispatch = preselection.selected_dispatch;
            let _selected_candidate_fact = preselection.selected_candidate_fact;
            {
                let mut store = self.service.session_store_guard()?;
                self.service
                    .scheduler_task_orchestrator
                    .bind_started_runtime_task_reservation(
                        &mut store,
                        session_id,
                        workflow_run_id,
                        &started_runtime_task,
                        &selected_dispatch,
                    )
                    .map_err(|error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler runtime task reservation binding failed: {error}"
                        ))
                    })?;
            }
            let execution_request_id =
                format!("workflow-runtime-task:{}:{}", workflow_run_id, task_id);
            let dispatch_result = self
                .service
                .scheduler_task_orchestrator
                .spawn_started_runtime_task_supervisor(
                    execution_request_id,
                    started_runtime_task.clone(),
                    selected_dispatch.clone(),
                )
                .map_err(|error| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler runtime task supervisor start failed: {error}"
                    ))
                })?
                .join()
                .await;
            match dispatch_result {
                Ok(result) => {
                    let terminal_mutation = {
                        let mut store = self.service.session_store_guard()?;
                        self.service
                            .scheduler_task_orchestrator
                            .complete_started_runtime_task_terminal_mutation(
                                &mut store,
                                session_id,
                                workflow_run_id,
                                &started_runtime_task,
                                result.clone(),
                            )
                            .map_err(|error| {
                                WorkflowServiceError::InvalidRequest(format!(
                                    "scheduler runtime task completion failed: {error}"
                                ))
                            })?
                    };
                    let (transition, reason, error_summary) =
                        scheduler_task_attempt_terminal_transition_from_result(&result);
                    self.record_scheduler_task_attempt_terminal(
                        session_id,
                        started_runtime_task.task(),
                        started_runtime_task.attempt_id().as_str(),
                        started_runtime_task.started_at_ms(),
                        transition,
                        reason,
                        error_summary,
                        Some(&selected_dispatch),
                        Some(&terminal_mutation),
                    )?;
                    self.service
                        .scheduler_task_orchestrator
                        .apply_runtime_task_result_reservation_lifecycle(
                            &started_runtime_task.task,
                            &terminal_mutation,
                            &result,
                        )
                        .await
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler runtime task reservation release failed: {error}"
                            ))
                        })?;
                }
                Err(error) => {
                    if let crate::scheduler::WorkflowSchedulerTaskOrchestratorError::RuntimeTaskSupervisorCancelled { message } = &error {
                        let terminal_mutation = {
                            let mut store = self.service.session_store_guard()?;
                            self.service
                                .scheduler_task_orchestrator
                                .cancel_started_runtime_task_terminal_mutation(
                                    &mut store,
                                    session_id,
                                    workflow_run_id,
                                    &started_runtime_task,
                                    message,
                                )
                                .map_err(|error| {
                                    WorkflowServiceError::InvalidRequest(format!(
                                        "scheduler runtime cancellation transition failed: {error}"
                                    ))
                                })?
                        };
                        self.record_scheduler_task_attempt_terminal(
                            session_id,
                            started_runtime_task.task(),
                            started_runtime_task.attempt_id().as_str(),
                            started_runtime_task.started_at_ms(),
                            SchedulerTaskAttemptLifecycleTransition::Cancelled,
                            "scheduler runtime task cancellation observed",
                            Some(message.clone()),
                            Some(&selected_dispatch),
                            Some(&terminal_mutation),
                        )?;
                        self.service
                            .scheduler_task_orchestrator
                            .apply_runtime_task_cancellation_reservation_lifecycle(
                                &started_runtime_task.task,
                                &terminal_mutation,
                                message,
                            )
                            .await
                            .map_err(|release_error| {
                                WorkflowServiceError::InvalidRequest(format!(
                                    "scheduler runtime task reservation release failed: {release_error}"
                                ))
                            })?;
                        return Err(WorkflowServiceError::Cancelled(message.clone()));
                    } else {
                        let terminal_mutation = {
                            let mut store = self.service.session_store_guard()?;
                            self.service
                                .scheduler_task_orchestrator
                                .fail_started_runtime_task_dispatch_error_terminal_mutation(
                                    &mut store,
                                    session_id,
                                    workflow_run_id,
                                    &started_runtime_task,
                                    &error,
                                )
                                .map_err(|error| {
                                    WorkflowServiceError::InvalidRequest(format!(
                                        "scheduler runtime dispatch error transition failed: {error}"
                                    ))
                                })?
                        };
                        self.record_scheduler_task_attempt_terminal(
                            session_id,
                            started_runtime_task.task(),
                            started_runtime_task.attempt_id().as_str(),
                            started_runtime_task.started_at_ms(),
                            SchedulerTaskAttemptLifecycleTransition::Failed,
                            "scheduler runtime task dispatch failed",
                            Some(error.to_string()),
                            Some(&selected_dispatch),
                            Some(&terminal_mutation),
                        )?;
                        self.service
                            .scheduler_task_orchestrator
                            .apply_runtime_task_dispatch_error_reservation_lifecycle(
                                &started_runtime_task.task,
                                &terminal_mutation,
                                &error,
                            )
                            .await
                            .map_err(|release_error| {
                                WorkflowServiceError::InvalidRequest(format!(
                                    "scheduler runtime task reservation release failed: {release_error}"
                                ))
                            })?;
                        return Err(WorkflowServiceError::CapabilityViolation(format!(
                            "runtime scheduler dispatch selection failed closed for {count} runtime inference task(s): {error}",
                            count = summary.runtime_inference_tasks
                        )));
                    }
                }
            }
        }
        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        ensure_all_scheduler_tasks_completed(&records)?;
        let results = {
            let mut store = self.service.session_store_guard()?;
            store.active_run_scheduler_task_results(session_id, workflow_run_id)?
        };
        let targets = scheduler_output_targets_for_run(host, workflow_id, output_targets).await?;
        let outputs = project_scheduler_task_results_to_outputs(&task_graph, &results, &targets)
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task output projection failed: {error}"
                ))
            })?;
        validate_host_output_bindings(&outputs, "outputs")?;
        validate_requested_outputs_produced(&targets, &outputs)?;
        Ok(WorkflowRunResponse {
            workflow_run_id: workflow_run_id.to_string(),
            outputs,
            timing_ms: started_at.elapsed().as_millis(),
        })
    }

    pub(super) async fn run_started_runtime_dispatch_task_to_completion(
        &self,
        host: &(impl WorkflowHost + ?Sized),
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: Instant,
        started_runtime_task: &StartedRuntimeTaskExecution,
        selected_dispatch: &SelectedRuntimeTaskDispatch,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let execution_request_id = format!(
            "workflow-runtime-task:{}:{}",
            workflow_run_id,
            started_runtime_task.task().task_id.as_str()
        );
        let dispatch_result = self
            .service
            .scheduler_task_orchestrator
            .spawn_started_runtime_task_supervisor(
                execution_request_id,
                started_runtime_task.clone(),
                selected_dispatch.clone(),
            )
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler runtime task supervisor start failed: {error}"
                ))
            })?
            .join()
            .await;
        match dispatch_result {
            Ok(result) => {
                let terminal_mutation = {
                    let mut store = self.service.session_store_guard()?;
                    self.service
                        .scheduler_task_orchestrator
                        .complete_started_runtime_task_terminal_mutation(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            started_runtime_task,
                            result.clone(),
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler runtime task completion failed: {error}"
                            ))
                        })?
                };
                let (transition, reason, error_summary) =
                    scheduler_task_attempt_terminal_transition_from_result(&result);
                self.record_scheduler_task_attempt_terminal(
                    session_id,
                    started_runtime_task.task(),
                    started_runtime_task.attempt_id().as_str(),
                    started_runtime_task.started_at_ms(),
                    transition,
                    reason,
                    error_summary,
                    Some(selected_dispatch),
                    Some(&terminal_mutation),
                )?;
                self.service
                    .scheduler_task_orchestrator
                    .apply_runtime_task_result_reservation_lifecycle(
                        started_runtime_task.task(),
                        &terminal_mutation,
                        &result,
                    )
                    .await
                    .map_err(|error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler runtime task reservation release failed: {error}"
                        ))
                    })?;
            }
            Err(error) => {
                if let crate::scheduler::WorkflowSchedulerTaskOrchestratorError::RuntimeTaskSupervisorCancelled { message } = &error {
                    let terminal_mutation = {
                        let mut store = self.service.session_store_guard()?;
                        self.service
                            .scheduler_task_orchestrator
                            .cancel_started_runtime_task_terminal_mutation(
                                &mut store,
                                session_id,
                                workflow_run_id,
                                started_runtime_task,
                                message,
                            )
                            .map_err(|error| {
                                WorkflowServiceError::InvalidRequest(format!(
                                    "scheduler runtime cancellation transition failed: {error}"
                                ))
                            })?
                    };
                    self.record_scheduler_task_attempt_terminal(
                        session_id,
                        started_runtime_task.task(),
                        started_runtime_task.attempt_id().as_str(),
                        started_runtime_task.started_at_ms(),
                        SchedulerTaskAttemptLifecycleTransition::Cancelled,
                        "scheduler runtime task cancellation observed",
                        Some(message.clone()),
                        Some(selected_dispatch),
                        Some(&terminal_mutation),
                    )?;
                    self.service
                        .scheduler_task_orchestrator
                        .apply_runtime_task_cancellation_reservation_lifecycle(
                            started_runtime_task.task(),
                            &terminal_mutation,
                            message,
                        )
                        .await
                        .map_err(|release_error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler runtime task reservation release failed: {release_error}"
                            ))
                        })?;
                    return Err(WorkflowServiceError::Cancelled(message.clone()));
                }
                let terminal_mutation = {
                    let mut store = self.service.session_store_guard()?;
                    self.service
                        .scheduler_task_orchestrator
                        .fail_started_runtime_task_dispatch_error_terminal_mutation(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            started_runtime_task,
                            &error,
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler runtime dispatch error transition failed: {error}"
                            ))
                        })?
                };
                self.record_scheduler_task_attempt_terminal(
                    session_id,
                    started_runtime_task.task(),
                    started_runtime_task.attempt_id().as_str(),
                    started_runtime_task.started_at_ms(),
                    SchedulerTaskAttemptLifecycleTransition::Failed,
                    "scheduler runtime task dispatch failed",
                    Some(error.to_string()),
                    Some(selected_dispatch),
                    Some(&terminal_mutation),
                )?;
                self.service
                    .scheduler_task_orchestrator
                    .apply_runtime_task_dispatch_error_reservation_lifecycle(
                        started_runtime_task.task(),
                        &terminal_mutation,
                        &error,
                    )
                    .await
                    .map_err(|release_error| {
                        WorkflowServiceError::InvalidRequest(format!(
                            "scheduler runtime task reservation release failed: {release_error}"
                        ))
                    })?;
                return Err(WorkflowServiceError::CapabilityViolation(format!(
                    "runtime scheduler dispatch selection failed closed for {count} runtime inference task(s): {error}",
                    count = summary.runtime_inference_tasks
                )));
            }
        }
        let (task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        ensure_all_scheduler_tasks_completed(&records)?;
        let results = {
            let mut store = self.service.session_store_guard()?;
            store.active_run_scheduler_task_results(session_id, workflow_run_id)?
        };
        let targets = scheduler_output_targets_for_run(host, workflow_id, output_targets).await?;
        let outputs = project_scheduler_task_results_to_outputs(&task_graph, &results, &targets)
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task output projection failed: {error}"
                ))
            })?;
        validate_host_output_bindings(&outputs, "outputs")?;
        validate_requested_outputs_produced(&targets, &outputs)?;
        Ok(WorkflowRunResponse {
            workflow_run_id: workflow_run_id.to_string(),
            outputs,
            timing_ms: started_at.elapsed().as_millis(),
        })
    }

    fn record_scheduler_task_attempt_started(
        &self,
        _session_id: &str,
        task: &WorkflowSchedulerTask,
        attempt_id: &str,
        started_at_ms: u64,
        transition: SchedulerTaskAttemptLifecycleTransition,
    ) -> Result<(), WorkflowServiceError> {
        if !matches!(
            transition,
            SchedulerTaskAttemptLifecycleTransition::Started
                | SchedulerTaskAttemptLifecycleTransition::Redispatched
        ) {
            return Err(WorkflowServiceError::Internal(format!(
                "scheduler task '{}' start event received terminal transition {:?}",
                task.task_id.as_str(),
                transition
            )));
        }
        let started_at_ms = i64::try_from(started_at_ms).map_err(|_| {
            WorkflowServiceError::Internal(format!(
                "scheduler task '{}' start time exceeded diagnostics ledger timestamp range",
                task.task_id.as_str()
            ))
        })?;
        let attribution =
            self.scheduler_task_attempt_diagnostic_attribution(task.workflow_run_id.as_str())?;
        self.service
            .workflow_diagnostic_event_record(DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms: started_at_ms,
                workflow_run_id: Some(WorkflowRunId::try_from(
                    task.workflow_run_id.as_str().to_string(),
                )?),
                workflow_id: Some(WorkflowId::try_from(task.workflow_id.as_str().to_string())?),
                workflow_version_id: None,
                workflow_semantic_version: None,
                node_id: Some(task.node_id.as_str().to_string()),
                node_type: Some(task.node_type.clone()),
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: attribution.client_id,
                client_session_id: attribution.client_session_id,
                bucket_id: attribution.bucket_id,
                scheduler_policy_id: Some("priority_then_fifo".to_string()),
                retention_policy_id: None,
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::SchedulerTaskAttemptLifecycleChanged(
                    SchedulerTaskAttemptLifecycleChangedPayload {
                        scheduler_task_id: task.task_id.as_str().to_string(),
                        scheduler_attempt_id: attempt_id.to_string(),
                        execution_class: scheduler_task_attempt_execution_class(task)?,
                        transition,
                        started_at_ms: Some(started_at_ms),
                        ended_at_ms: None,
                        duration_ms: None,
                        selected_runtime_id: None,
                        selected_runtime_variant_id: None,
                        selected_backend_key: None,
                        selected_device_class: None,
                        selected_device_id: None,
                        selected_network_node_id: None,
                        reservation_id: None,
                        reason: Some(scheduler_task_attempt_start_reason(transition).to_string()),
                        error_summary: None,
                        canonical_error_event_id: None,
                    },
                ),
            })?;
        Ok(())
    }

    fn scheduler_task_attempt_diagnostic_attribution(
        &self,
        workflow_run_id: &str,
    ) -> Result<WorkflowSchedulerTaskAttemptDiagnosticAttribution, WorkflowServiceError> {
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let snapshot = self
            .service
            .workflow_run_snapshot_for_execution_resume_if_configured(&workflow_run_id)?;
        let Some(snapshot) = snapshot else {
            return Ok(WorkflowSchedulerTaskAttemptDiagnosticAttribution::none());
        };
        Ok(WorkflowSchedulerTaskAttemptDiagnosticAttribution {
            client_id: snapshot.client_id,
            client_session_id: snapshot.client_session_id,
            bucket_id: snapshot.bucket_id,
        })
    }

    fn record_scheduler_task_attempt_terminal(
        &self,
        _session_id: &str,
        task: &WorkflowSchedulerTask,
        attempt_id: &str,
        started_at_ms: u64,
        transition: SchedulerTaskAttemptLifecycleTransition,
        reason: &str,
        error_summary: Option<String>,
        selected_dispatch: Option<&SelectedRuntimeTaskDispatch>,
        terminal_mutation: Option<&WorkflowSchedulerTaskTerminalMutation>,
    ) -> Result<(), WorkflowServiceError> {
        let attribution =
            self.scheduler_task_attempt_diagnostic_attribution(task.workflow_run_id.as_str())?;
        self.service.workflow_diagnostic_event_record(
            scheduler_task_attempt_terminal_diagnostic_event(
                WorkflowSchedulerTaskAttemptTerminalDiagnosticRequest {
                    task,
                    attempt_id,
                    started_at_ms,
                    transition,
                    reason,
                    error_summary,
                    selected_dispatch,
                    terminal_mutation,
                    attribution,
                },
            )?,
        )?;
        Ok(())
    }
}

fn scheduler_task_attempt_terminal_transition_from_result(
    result: &WorkflowSchedulerTaskResult,
) -> (
    SchedulerTaskAttemptLifecycleTransition,
    &'static str,
    Option<String>,
) {
    match result.status {
        WorkflowSchedulerTaskResultStatus::Completed => (
            SchedulerTaskAttemptLifecycleTransition::Completed,
            "scheduler runtime task attempt completed",
            None,
        ),
        WorkflowSchedulerTaskResultStatus::Failed
        | WorkflowSchedulerTaskResultStatus::Unavailable
        | WorkflowSchedulerTaskResultStatus::Invalid => (
            SchedulerTaskAttemptLifecycleTransition::Failed,
            "scheduler runtime task result failed",
            Some(
                result
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone())
                    .unwrap_or_else(|| {
                        format!(
                            "scheduler runtime task result status {}",
                            scheduler_task_result_status_label(result.status)
                        )
                    }),
            ),
        ),
    }
}

fn scheduler_task_attempt_start_reason(
    transition: SchedulerTaskAttemptLifecycleTransition,
) -> &'static str {
    match transition {
        SchedulerTaskAttemptLifecycleTransition::Started => "scheduler task attempt started",
        SchedulerTaskAttemptLifecycleTransition::Redispatched => {
            "scheduler task attempt redispatched"
        }
        SchedulerTaskAttemptLifecycleTransition::Completed
        | SchedulerTaskAttemptLifecycleTransition::Failed
        | SchedulerTaskAttemptLifecycleTransition::Cancelled => {
            unreachable!("terminal scheduler task attempt transition cannot start an attempt")
        }
    }
}

fn scheduler_task_result_status_label(status: WorkflowSchedulerTaskResultStatus) -> &'static str {
    match status {
        WorkflowSchedulerTaskResultStatus::Completed => "completed",
        WorkflowSchedulerTaskResultStatus::Failed => "failed",
        WorkflowSchedulerTaskResultStatus::Unavailable => "unavailable",
        WorkflowSchedulerTaskResultStatus::Invalid => "invalid",
    }
}

fn runtime_dispatch_preselection_invalid_request(
    error: WorkflowRuntimeDispatchPreselectionError,
) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(error.to_string())
}

fn scheduler_task_attempt_execution_class(
    task: &WorkflowSchedulerTask,
) -> Result<SchedulerTaskAttemptExecutionClass, WorkflowServiceError> {
    match task.execution_class {
        WorkflowSchedulerTaskExecutionClass::RuntimeInference => {
            Ok(SchedulerTaskAttemptExecutionClass::Runtime)
        }
        WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine => {
            Ok(SchedulerTaskAttemptExecutionClass::NonRuntimeNodeEngine)
        }
        other => Err(WorkflowServiceError::Internal(format!(
            "scheduler task '{}' has unsupported started-attempt execution class {:?}",
            task.task_id.as_str(),
            other
        ))),
    }
}

fn runtime_dispatch_readiness_proof(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
    task_id: &str,
    admitted_runtime_readiness: &[AdmittedRuntimeTaskReadiness],
) -> Result<DependencyReadinessProofEnvelope, WorkflowServiceError> {
    if let Some(admitted) = admitted_runtime_readiness
        .iter()
        .find(|admitted| admitted.task_id == task_id)
    {
        return Ok(admitted.readiness_proof.clone());
    }

    let store = service.session_store_guard()?;
    store
        .active_run_runtime_dispatch_readiness_proof(session_id, workflow_run_id, task_id)?
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' has no persisted readiness proof for dispatch selection",
                task_id
            ))
        })
}

fn ready_runtime_dispatch_context(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
    task_id: &str,
) -> Result<ReadyRuntimeDispatchContext, WorkflowServiceError> {
    let store = service.session_store_guard()?;
    let (task_graph, records) = store
        .active_run_scheduler_task_state(session_id, workflow_run_id)?
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "active workflow run '{}' has no scheduler task state",
                workflow_run_id
            ))
        })?;
    let task = task_graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == task_id)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' is not in active workflow run '{}'",
                task_id, workflow_run_id
            ))
        })?
        .clone();
    let ready_record = records
        .iter()
        .find(|record| record.task_id.as_str() == task_id)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "runtime scheduler task '{}' has no active task-state record",
                task_id
            ))
        })?
        .clone();
    Ok(ReadyRuntimeDispatchContext { task, ready_record })
}

fn runtime_task_ids_in_state(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
    is_state: impl Fn(SchedulerTaskStateKind) -> bool,
) -> Result<Vec<String>, WorkflowServiceError> {
    let (task_graph, records) =
        active_run_scheduler_task_state_required(service, session_id, workflow_run_id)?;
    let mut task_ids = Vec::new();
    for task in &task_graph.tasks {
        if task.execution_class != WorkflowSchedulerTaskExecutionClass::RuntimeInference {
            continue;
        }
        let record = records
            .iter()
            .find(|record| record.task_id.as_str() == task.task_id.as_str())
            .ok_or_else(|| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler task '{}' has no active task-state record",
                    task.task_id.as_str()
                ))
            })?;
        if is_state(record.state.kind()) {
            task_ids.push(task.task_id.as_str().to_string());
        }
    }
    Ok(task_ids)
}

fn dependency_readiness_error(
    error: WorkflowDependencyReadinessLifecycleError,
) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!(
        "scheduler dependency readiness admission failed: {error}"
    ))
}

fn dependency_readiness_work_queue_error(
    error: DependencyReadinessWorkQueueError,
) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!(
        "scheduler dependency readiness work queue failed: {error}"
    ))
}

fn dependency_readiness_work_item(
    session_id: &str,
    workflow_run_id: &str,
    task_id: &str,
    request: ValidatedDependencyEnvironmentRequest,
) -> Result<DependencyReadinessWorkItem, DependencyReadinessWorkQueueError> {
    Ok(DependencyReadinessWorkItem::new(
        DependencyReadinessWorkItemProvenance::new(
            DependencyReadinessWorkflowSessionId::parse(session_id)?,
            DependencyReadinessWorkflowRunId::parse(workflow_run_id)?,
            DependencyReadinessTaskId::parse(task_id)?,
        ),
        request,
    )
    .with_diagnostic_context(DependencyReadinessDiagnosticContext::parse(
        "runtime task entered WaitingDependencyReadiness",
    )?))
}

fn dependency_environment_request_from_readiness_envelope(
    request: &ValidatedDependencyReadinessRequestEnvelope,
) -> Result<ValidatedDependencyEnvironmentRequest, DependencyReadinessWorkQueueError> {
    let envelope = request.as_envelope();
    let readiness_request = &envelope.readiness_request;
    ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Check,
        identity_key: readiness_request.identity_key.clone(),
        planning_request: readiness_request.planning_request.clone(),
        dependency_requirements_id: Some(
            envelope
                .execution_context
                .dependency_requirements_id
                .clone(),
        ),
        environment_ref: None,
    })
    .map_err(DependencyReadinessWorkQueueError::from)
}

fn ensure_all_scheduler_tasks_completed(
    records: &[SchedulerTaskStateRecord],
) -> Result<(), WorkflowServiceError> {
    if let Some(record) = records
        .iter()
        .find(|record| record.state.kind() != SchedulerTaskStateKind::Completed)
    {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "scheduler task '{}' did not complete; final state was {:?}",
            record.task_id.as_str(),
            record.state.kind()
        )));
    }
    Ok(())
}

fn active_run_scheduler_task_state_required(
    service: &WorkflowService,
    session_id: &str,
    workflow_run_id: &str,
) -> Result<(WorkflowSchedulerTaskGraph, Vec<SchedulerTaskStateRecord>), WorkflowServiceError> {
    let store = service.session_store_guard()?;
    store
        .active_run_scheduler_task_state(session_id, workflow_run_id)?
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "active workflow run '{}' has no scheduler task state",
                workflow_run_id
            ))
        })
}

async fn scheduler_output_targets_for_run<H: WorkflowHost + ?Sized>(
    host: &H,
    workflow_id: &str,
    output_targets: Option<&[WorkflowOutputTarget]>,
) -> Result<Vec<WorkflowOutputTarget>, WorkflowServiceError> {
    let io = host.workflow_io(workflow_id).await?;
    validate_workflow_io(&io)?;
    if let Some(targets) = output_targets {
        validate_output_targets_against_io(targets, &io)?;
        return Ok(targets.to_vec());
    }
    Ok(io
        .outputs
        .iter()
        .flat_map(|node| {
            node.ports.iter().map(|port| WorkflowOutputTarget {
                node_id: node.node_id.clone(),
                port_id: port.port_id.clone(),
            })
        })
        .collect())
}
