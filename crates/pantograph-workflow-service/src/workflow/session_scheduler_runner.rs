use std::time::Instant;

use pantograph_dependency_environment_service::{
    DependencyReadinessDiagnosticContext, DependencyReadinessTaskId, DependencyReadinessWorkItem,
    DependencyReadinessWorkItemProvenance, DependencyReadinessWorkQueueError,
    DependencyReadinessWorkflowRunId, DependencyReadinessWorkflowSessionId,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentAction, DependencyEnvironmentRequest, DependencyReadinessPolicy,
    ValidatedDependencyEnvironmentRequest, ValidatedDependencyReadinessRequestEnvelope,
};
use pantograph_scheduler::{SchedulerTaskStateKind, SchedulerTaskStateRecord};

use crate::scheduler::{
    WorkflowDependencyReadinessLifecycle, WorkflowDependencyReadinessLifecycleError,
};

use super::io_contract::validate_workflow_io;
use super::validation::{
    validate_host_output_bindings, validate_output_targets_against_io,
    validate_requested_outputs_produced,
};
use super::{
    project_scheduler_task_results_to_outputs, WorkflowHost, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowRunResponse, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskRunSummary, WorkflowService,
    WorkflowServiceError,
};

pub(super) struct WorkflowSchedulerSessionRunner<'a> {
    service: &'a WorkflowService,
}

impl<'a> WorkflowSchedulerSessionRunner<'a> {
    pub(super) fn new(service: &'a WorkflowService) -> Self {
        Self { service }
    }

    pub(super) async fn run_non_runtime_only<H: WorkflowHost>(
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
        self.materialize_external_inputs(session_id, workflow_run_id, inputs)?;
        self.run_progress_loop(session_id, workflow_run_id).await?;

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

    pub(super) async fn run_until_runtime_dispatch_boundary(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        inputs: &[WorkflowPortBinding],
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if !summary.has_runtime_inference() {
            return Err(WorkflowServiceError::Internal(
                "scheduler runtime runner received a run without runtime inference".to_string(),
            ));
        }

        self.materialize_external_inputs(session_id, workflow_run_id, inputs)?;
        self.run_progress_loop(session_id, workflow_run_id).await?;
        self.admit_runtime_dependency_readiness(session_id, workflow_run_id)?;
        self.ensure_runtime_tasks_ready_for_dispatch(session_id, workflow_run_id)?;
        self.fail_runtime_dispatch_not_wired(session_id, workflow_run_id, summary)
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
        let (_task_graph, records) =
            active_run_scheduler_task_state_required(self.service, session_id, workflow_run_id)?;
        let ready_task_ids = records
            .iter()
            .filter(|record| record.state.kind() == SchedulerTaskStateKind::Ready)
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
                }
                Err(
                    crate::scheduler::WorkflowSchedulerTaskOrchestratorError::NonRuntimeTaskAdapter(
                        error,
                    ),
                ) => {
                    let mut store = self.service.session_store_guard()?;
                    let _ = self
                        .service
                        .scheduler_task_orchestrator
                        .fail_started_non_runtime_task(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            &started,
                            &error,
                        );
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
    ) -> Result<(), WorkflowServiceError> {
        let runtime_task_ids =
            runtime_task_ids_in_state(self.service, session_id, workflow_run_id, |kind| {
                kind == SchedulerTaskStateKind::WaitingDependencyReadiness
            })?;
        let lifecycle = WorkflowDependencyReadinessLifecycle::new(
            self.service.scheduler_task_orchestrator.clone(),
        );
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
            let mut store = self.service.session_store_guard()?;
            lifecycle
                .admit_active_runtime_task(
                    &mut store,
                    session_id,
                    workflow_run_id,
                    &task_id,
                    DependencyReadinessPolicy::CheckOnly,
                    readiness_proof,
                )
                .map_err(dependency_readiness_error)?;
        }
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

    fn fail_runtime_dispatch_not_wired(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let mut store = self.service.session_store_guard()?;
        self.service
            .scheduler_task_orchestrator
            .fail_runtime_dispatch_not_wired_for_active_run(&mut store, session_id, workflow_run_id)
            .map_err(|error| {
                WorkflowServiceError::InvalidRequest(format!(
                    "scheduler runtime dispatch fail-closed transition failed: {error}"
                ))
            })?;
        Err(WorkflowServiceError::CapabilityViolation(format!(
            "runtime scheduler dispatch is not wired for {count} runtime inference task(s); runtime tasks must execute only through dispatch-selected scheduler runtime-host handoff",
            count = summary.runtime_inference_tasks
        )))
    }
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
        action: DependencyEnvironmentAction::Resolve,
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

async fn scheduler_output_targets_for_run<H: WorkflowHost>(
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
