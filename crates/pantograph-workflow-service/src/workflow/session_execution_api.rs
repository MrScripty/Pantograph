use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

use crate::graph::{
    workflow_executable_topology, workflow_graph_run_settings, workflow_graph_run_settings_json,
    WorkflowExecutionSessionKind, WorkflowGraph, WorkflowGraphRunSettings,
};
use crate::scheduler::{
    unix_timestamp_ms, WorkflowSchedulerBootstrapRecoveryAction,
    WorkflowSchedulerBootstrapRecoverySnapshot, WorkflowSchedulerBootstrapRecoveryTask,
    WorkflowSchedulerQueueAdmissionCommand, WorkflowSchedulerQueueTaskStateCommand,
    WorkflowSchedulerQueueWorker,
};
use crate::technical_fit::{
    WorkflowTechnicalFitOverride, WorkflowTechnicalFitResourceEstimateKind,
    WorkflowTechnicalFitResourceEstimateState,
};
use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    IoArtifactObservedPayload, IoArtifactRole, LibraryAssetAccessedPayload, LibraryAssetOperation,
    RunResourceObservationRollupQuery, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload,
    RunStartedPayload, RunTerminalPayload, RunTerminalStatus, SchedulerEstimateBlockingCondition,
    SchedulerEstimateProducedPayload, SchedulerModelCacheState, SchedulerQueuePlacementPayload,
    SchedulerTaskAttemptLifecycleTransition,
};
use pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION;
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttributionResolveRequest,
    WorkflowRunId, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
};

use super::diagnostic_errors::{
    WorkflowDiagnosticErrorRecordRequest, WorkflowDiagnosticRunContext, WorkflowDiagnosticRunScope,
    WorkflowDiagnosticSchedulerScope,
};
use super::runtime_branch_task_event::{
    WorkflowRuntimeBranchTaskEventId, WorkflowRuntimeBranchTaskEventRecord,
    WorkflowRuntimeBranchTaskEventRepository, WorkflowRuntimeBranchTaskEventRequest,
    WorkflowRuntimeBranchTaskEventState,
};
#[cfg(test)]
use super::runtime_dispatch_assignment::{
    WorkflowRuntimeDispatchAssignmentId, WorkflowRuntimeDispatchAssignmentRecord,
    WorkflowRuntimeDispatchAssignmentRepository,
};
use super::session_io_artifacts::workflow_io_artifact_metadata;
use super::session_scheduler_runner::WorkflowSchedulerSessionRunner;
use super::task_execution_owner::WorkflowTaskExecutionOwner;
use super::task_execution_runtime::WorkflowTaskExecutionRuntimeOwner;
use super::task_execution_worker::{
    WorkflowTaskExecutionWorkerOutcome, WorkflowTaskExecutionWorkerRuntimeBranchCommand,
    WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason,
    WorkflowTaskExecutionWorkerRuntimeBranchStartReason,
};
use super::validation::{
    validate_bindings, validate_output_targets, validate_timeout_ms,
    validate_workflow_graph_submit_readiness, validate_workflow_id,
    validate_workflow_semantic_version,
};
use super::workflow_run_finalization::{
    finalize_admitted_workflow_run, WorkflowRunFinalizationRequest,
};
use super::{
    workflow_scheduler_task_graph, workflow_scheduler_task_graph_with_inference_projections,
    workflow_scheduler_task_run_summary, AttributionRepository, WorkflowCapabilityModel,
    WorkflowExecutableValidationSnapshotLookupRequest,
    WorkflowExecutionSessionAttributedCreateRequest, WorkflowExecutionSessionAttributionContext,
    WorkflowExecutionSessionBootstrapRecoveryAction,
    WorkflowExecutionSessionBootstrapRecoveryDecision,
    WorkflowExecutionSessionBootstrapRecoveryDecisionKind,
    WorkflowExecutionSessionBootstrapRecoveryPlan, WorkflowExecutionSessionBootstrapRecoveryReport,
    WorkflowExecutionSessionBootstrapRecoveryResult, WorkflowExecutionSessionBootstrapRecoveryRun,
    WorkflowExecutionSessionBootstrapRecoveryTask, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionCreateResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionResumeRequest, WorkflowExecutionSessionRunRequest,
    WorkflowExecutionSessionSummary, WorkflowHost, WorkflowPortBinding, WorkflowRunResponse,
    WorkflowRuntimeCapability, WorkflowRuntimeRequirements, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskRunSummary, WorkflowService,
    WorkflowServiceError,
};

const WORKFLOW_SESSION_SCHEDULER_POLICY: &str = "priority_then_fifo";
const WORKFLOW_SESSION_RETENTION_KEEP_ALIVE: &str = "keep_alive";
const WORKFLOW_SESSION_RETENTION_EPHEMERAL: &str = "ephemeral";

fn remaining_runtime_resume_timeout_ms(
    timeout_ms: Option<u64>,
    dequeued_at_ms: u64,
) -> Result<Option<u64>, WorkflowServiceError> {
    let Some(timeout_ms) = timeout_ms else {
        return Ok(None);
    };
    let elapsed_ms = unix_timestamp_ms().saturating_sub(dequeued_at_ms);
    if elapsed_ms >= timeout_ms {
        return Err(WorkflowServiceError::RuntimeTimeout(format!(
            "workflow run exceeded timeout_ms {}",
            timeout_ms
        )));
    }
    Ok(Some(timeout_ms - elapsed_ms))
}

fn resumed_run_started_at(dequeued_at_ms: u64) -> std::time::Instant {
    let elapsed_ms = unix_timestamp_ms().saturating_sub(dequeued_at_ms);
    std::time::Instant::now()
        .checked_sub(Duration::from_millis(elapsed_ms))
        .unwrap_or_else(std::time::Instant::now)
}

impl WorkflowService {
    fn resolve_execution_session_attribution(
        &self,
        request: super::WorkflowExecutionSessionAttributionRequest,
    ) -> Result<WorkflowExecutionSessionAttributionContext, WorkflowServiceError> {
        let client_session_id = ClientSessionId::try_from(request.client_session_id)?;
        let store = self.attribution_store_guard()?;
        let context = store.resolve_workflow_run_attribution_context(
            WorkflowRunAttributionResolveRequest {
                credential: request.credential,
                client_session_id,
                bucket_selection: request.bucket_selection,
            },
        )?;
        Ok(WorkflowExecutionSessionAttributionContext {
            client_id: context.client_id.as_str().to_string(),
            client_session_id: context.client_session_id.as_str().to_string(),
            bucket_id: context.bucket_id.as_str().to_string(),
        })
    }

    pub async fn create_workflow_execution_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        self.create_workflow_execution_session_internal(
            host,
            request.workflow_id,
            request.usage_profile,
            request.keep_alive,
            None,
        )
        .await
    }

    pub async fn create_attributed_workflow_execution_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionAttributedCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        let attribution = self.resolve_execution_session_attribution(request.attribution)?;
        self.create_workflow_execution_session_internal(
            host,
            request.workflow_id,
            request.usage_profile,
            request.keep_alive,
            Some(attribution),
        )
        .await
    }

    async fn create_workflow_execution_session_internal<H: WorkflowHost>(
        &self,
        host: &H,
        workflow_id: String,
        usage_profile: Option<String>,
        keep_alive: bool,
        attribution: Option<WorkflowExecutionSessionAttributionContext>,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        validate_workflow_id(&workflow_id)?;
        host.validate_workflow(&workflow_id).await?;

        let session_id = {
            let mut store = self.session_store_guard()?;
            store.create_session(
                workflow_id.clone(),
                usage_profile
                    .clone()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty()),
                attribution.clone(),
                Vec::new(),
                Vec::new(),
                keep_alive,
            )?
        };

        if keep_alive {
            if let Err(error) = self
                .ensure_keep_alive_session_runtime_ready(host, &session_id, &workflow_id)
                .await
            {
                if let Ok(mut rollback_store) = self.session_store.lock() {
                    let _ = rollback_store.close_session(&session_id);
                }
                return Err(error);
            }
        }

        Ok(WorkflowExecutionSessionCreateResponse {
            session_id,
            attribution,
            runtime_capabilities: host.runtime_capabilities().await?,
        })
    }

    pub async fn run_workflow_execution_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionRunRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.run_workflow_execution_session_with_runtime_owner(host, request, None)
            .await
    }

    pub(super) async fn run_workflow_execution_session_with_runtime_owner<
        H: WorkflowHost + ?Sized,
    >(
        &self,
        host: &H,
        request: WorkflowExecutionSessionRunRequest,
        task_execution_runtime_owner: Option<&WorkflowTaskExecutionRuntimeOwner>,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        validate_workflow_semantic_version(&request.workflow_semantic_version)?;
        validate_timeout_ms(request.timeout_ms)?;
        validate_bindings(&request.inputs, "inputs")?;
        if let Some(targets) = request.output_targets.as_ref() {
            validate_output_targets(targets)?;
        }

        let session = {
            let store = self.session_store_guard()?;
            store.session_summary(&session_id)?
        };
        let workflow_run_id = WorkflowRunId::generate().to_string();
        let run_snapshot = match self
            .create_queued_run_snapshot_if_configured(host, &session, &workflow_run_id, &request)
            .await
        {
            Ok(run_snapshot) => run_snapshot,
            Err(error) => {
                return Err(self.record_run_snapshot_failure_error(
                    &session,
                    &workflow_run_id,
                    Some(&request.workflow_semantic_version),
                    error,
                )?);
            }
        };
        let scheduler_task_graph = match self
            .scheduler_task_graph_for_session_run(
                host,
                &session.workflow_id,
                &workflow_run_id,
                run_snapshot.as_ref(),
            )
            .await
        {
            Ok(task_graph) => task_graph,
            Err(error) => {
                return Err(self.record_scheduler_admission_failure_error(
                    &session,
                    run_snapshot.as_ref(),
                    &workflow_run_id,
                    Some(&request.workflow_semantic_version),
                    error,
                )?);
            }
        };
        let queued_item = {
            let mut store = self.session_store_guard()?;
            store.enqueue_run_with_id(&session_id, &request, workflow_run_id.clone())?;
            store
                .list_queue(&session_id)?
                .into_iter()
                .find(|item| item.workflow_run_id == workflow_run_id)
                .ok_or_else(|| {
                    WorkflowServiceError::Internal(format!(
                        "queued run '{}' missing from session '{}' after enqueue",
                        workflow_run_id, session_id
                    ))
                })?
        };
        if let Err(error) = self
            .record_scheduler_estimate_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &queued_item,
                Some(&request.workflow_semantic_version),
            )
            .and_then(|_| {
                self.record_scheduler_queue_placement_event_if_configured(
                    &session,
                    run_snapshot.as_ref(),
                    &queued_item,
                    &request,
                )
            })
        {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.cancel_queue_item(&session_id, &workflow_run_id);
            }
            return Err(self.record_scheduler_admission_failure_error(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&request.workflow_semantic_version),
                error,
            )?);
        }
        let initial_scheduler_task_records = match self
            .scheduler_task_orchestrator
            .initial_task_state_records(&scheduler_task_graph)
        {
            Ok(records) => records,
            Err(error) => {
                if let Ok(mut store) = self.session_store.lock() {
                    let _ = store.cancel_queue_item(&session_id, &workflow_run_id);
                }
                return Err(self.record_scheduler_admission_failure_error(
                    &session,
                    run_snapshot.as_ref(),
                    &workflow_run_id,
                    Some(&request.workflow_semantic_version),
                    WorkflowServiceError::Internal(format!(
                        "scheduler task-state initialization failed: {error}"
                    )),
                )?);
            }
        };
        let scheduler_task_run_summary = workflow_scheduler_task_run_summary(
            &scheduler_task_graph,
            &initial_scheduler_task_records,
        )
        .map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "scheduler task run summary failed before admission: {error}"
            ))
        })?;
        if scheduler_task_run_summary.has_runtime_inference()
            && task_execution_runtime_owner.is_none()
        {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.cancel_queue_item(&session_id, &workflow_run_id);
            }
            return Err(self.record_scheduler_admission_failure_error(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&request.workflow_semantic_version),
                WorkflowServiceError::CapabilityViolation(
                    "runtime inference session execution requires WorkflowSessionExecutionRuntime composition-root entrypoint"
                        .to_string(),
                ),
            )?);
        }
        if let Err(error) = WorkflowTaskExecutionOwner::ensure_task_execution_available(self) {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.cancel_queue_item(&session_id, &workflow_run_id);
            }
            return Err(self.record_scheduler_admission_failure_error(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&request.workflow_semantic_version),
                error,
            )?);
        }

        let queued_run = WorkflowSchedulerQueueWorker::admit_queued_run(
            WorkflowSchedulerQueueAdmissionCommand::new(
                self.session_store.clone(),
                session_id.clone(),
                workflow_run_id.clone(),
            ),
        )
        .await?;
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        if let Err(error) = WorkflowSchedulerQueueWorker::initialize_admitted_task_state(
            WorkflowSchedulerQueueTaskStateCommand::new(
                self.session_store.clone(),
                session_id.clone(),
                workflow_run_id.clone(),
                scheduler_task_graph.clone(),
                initial_scheduler_task_records,
            ),
        ) {
            let terminal_result = Err(error);
            let finalization = finalize_admitted_workflow_run(
                self,
                WorkflowRunFinalizationRequest {
                    session: &session,
                    run_snapshot: run_snapshot.as_ref(),
                    session_id: &session_id,
                    workflow_run_id: &workflow_run_id,
                    workflow_semantic_version: &queued_workflow_semantic_version,
                    io_artifact_inputs: Some(&queued_run.queued.inputs),
                    run_result: terminal_result,
                },
            )?;
            return finalization.run_result;
        }

        if scheduler_task_run_summary.is_non_runtime_only() {
            return WorkflowTaskExecutionOwner::run_non_runtime_to_completion(
                self,
                host,
                &session,
                run_snapshot.as_ref(),
                &session_id,
                &workflow_run_id,
                &queued_run,
                &scheduler_task_run_summary,
            )
            .await;
        }

        if scheduler_task_run_summary.has_runtime_inference() {
            let task_execution_runtime_owner = task_execution_runtime_owner.ok_or_else(|| {
                WorkflowServiceError::CapabilityViolation(
                    "runtime inference session execution requires WorkflowSessionExecutionRuntime composition-root entrypoint"
                        .to_string(),
                )
            })?;
            self.persist_runtime_branch_task_events_for_admission(
                &session_id,
                &queued_run.workflow_id,
                &workflow_run_id,
                queued_run.queued.output_targets.clone(),
                queued_run.queued.timeout_ms,
                &scheduler_task_graph,
            )?;
            let command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
                session_id: session_id.clone(),
                workflow_run_id: workflow_run_id.clone(),
                workflow_id: queued_run.workflow_id.clone(),
                output_targets: queued_run.queued.output_targets.clone(),
                timeout_ms: queued_run.queued.timeout_ms,
                start_reason: WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started,
            };
            let outcome = task_execution_runtime_owner
                .enqueue_runtime_branch_and_wait(command)
                .await?;
            return workflow_response_from_runtime_branch_worker_outcome(outcome);
        }

        WorkflowTaskExecutionOwner::fail_unhandled_scheduler_classes_to_completion(
            self,
            &session,
            run_snapshot.as_ref(),
            &session_id,
            &workflow_run_id,
            &queued_run,
            &scheduler_task_run_summary,
        )
    }

    pub async fn resume_workflow_execution_session_runtime_dependency_readiness<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionResumeRequest,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        self.resume_workflow_execution_session_runtime_dependency_readiness_with_attempt_transition(
            host,
            request,
            SchedulerTaskAttemptLifecycleTransition::Started,
            None,
        )
        .await
    }

    async fn resume_workflow_execution_session_runtime_dependency_readiness_with_attempt_transition<
        H: WorkflowHost + ?Sized,
    >(
        &self,
        host: &H,
        request: WorkflowExecutionSessionResumeRequest,
        attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
        task_execution_runtime_owner: Option<&WorkflowTaskExecutionRuntimeOwner>,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim().to_string();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        let (session, active_run, scheduler_task_graph, scheduler_task_run_summary) = {
            let store = self.session_store_guard()?;
            let session = store.session_summary(&session_id)?;
            let active_run = store.active_run_context(&session_id, &workflow_run_id)?;
            let (task_graph, records) = store
                .active_run_scheduler_task_state(&session_id, &workflow_run_id)?
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "workflow run '{}' has no active scheduler task state to resume",
                        workflow_run_id
                    ))
                })?;
            let summary =
                workflow_scheduler_task_run_summary(&task_graph, &records).map_err(|error| {
                    WorkflowServiceError::Internal(format!(
                        "scheduler task run summary failed before resume: {error}"
                    ))
                })?;
            (session, active_run, task_graph, summary)
        };
        if !scheduler_task_run_summary.has_runtime_inference() {
            return Err(WorkflowServiceError::InvalidRequest(format!(
                "workflow run '{}' is not a runtime inference run",
                workflow_run_id
            )));
        }

        if let Some(task_execution_runtime_owner) = task_execution_runtime_owner {
            self.ensure_runtime_branch_task_events_for_recovery(
                &session_id,
                &active_run.workflow_id,
                &workflow_run_id,
                active_run.output_targets.clone(),
                active_run.timeout_ms,
                &scheduler_task_graph,
            )?;
            let command = WorkflowTaskExecutionWorkerRuntimeBranchCommand {
                session_id,
                workflow_run_id,
                workflow_id: active_run.workflow_id,
                output_targets: active_run.output_targets,
                timeout_ms: active_run.timeout_ms,
                start_reason: workflow_task_execution_worker_runtime_branch_start_reason(
                    attempt_start_transition,
                )?,
            };
            let outcome = task_execution_runtime_owner
                .enqueue_runtime_branch_and_wait(command)
                .await?;
            return workflow_response_from_runtime_branch_worker_outcome(outcome);
        }

        let workflow_run_id_typed = WorkflowRunId::try_from(workflow_run_id.clone())?;
        let run_snapshot =
            self.workflow_run_snapshot_for_execution_resume_if_configured(&workflow_run_id_typed)?;
        let remaining_timeout_ms =
            remaining_runtime_resume_timeout_ms(active_run.timeout_ms, active_run.dequeued_at_ms);
        let started_at = resumed_run_started_at(active_run.dequeued_at_ms);
        let runner = WorkflowSchedulerSessionRunner::new(self);
        let run_result = match remaining_timeout_ms {
            Err(error) => Err(error),
            Ok(Some(timeout_ms)) => {
                let run_future = runner.resume_runtime_dependency_readiness(
                    host,
                    &session_id,
                    &workflow_run_id,
                    &active_run.workflow_id,
                    active_run.output_targets.as_deref(),
                    &scheduler_task_run_summary,
                    started_at,
                    attempt_start_transition,
                );
                match tokio::time::timeout(Duration::from_millis(timeout_ms), run_future).await {
                    Ok(result) => result,
                    Err(_) => Err(WorkflowServiceError::RuntimeTimeout(format!(
                        "workflow run exceeded timeout_ms {}",
                        active_run.timeout_ms.unwrap_or(timeout_ms)
                    ))),
                }
            }
            Ok(None) => {
                runner
                    .resume_runtime_dependency_readiness(
                        host,
                        &session_id,
                        &workflow_run_id,
                        &active_run.workflow_id,
                        active_run.output_targets.as_deref(),
                        &scheduler_task_run_summary,
                        started_at,
                        attempt_start_transition,
                    )
                    .await
            }
        };
        if run_result
            .as_ref()
            .is_err_and(WorkflowServiceError::is_runtime_dependency_readiness_pending)
        {
            return run_result;
        }

        let finalization = finalize_admitted_workflow_run(
            self,
            WorkflowRunFinalizationRequest {
                session: &session,
                run_snapshot: run_snapshot.as_ref(),
                session_id: &session_id,
                workflow_run_id: &workflow_run_id,
                workflow_semantic_version: &active_run.workflow_semantic_version,
                io_artifact_inputs: Some(&active_run.inputs),
                run_result,
            },
        )?;
        finalization.run_result
    }

    pub fn workflow_execution_session_runtime_dependency_readiness_resume_candidates(
        &self,
    ) -> Result<Vec<WorkflowExecutionSessionResumeRequest>, WorkflowServiceError> {
        let store = self.session_store_guard()?;
        Ok(store.dependency_readiness_resume_candidates())
    }

    pub fn workflow_execution_session_bootstrap_recovery_report(
        &self,
    ) -> Result<WorkflowExecutionSessionBootstrapRecoveryReport, WorkflowServiceError> {
        let store = self.session_store_guard()?;
        let active_runs = store
            .bootstrap_recovery_snapshots()?
            .into_iter()
            .map(workflow_bootstrap_recovery_run_from_scheduler)
            .collect();
        Ok(WorkflowExecutionSessionBootstrapRecoveryReport { active_runs })
    }

    pub fn workflow_execution_session_bootstrap_recovery_plan(
        &self,
    ) -> Result<WorkflowExecutionSessionBootstrapRecoveryPlan, WorkflowServiceError> {
        let report = self.workflow_execution_session_bootstrap_recovery_report()?;
        Ok(workflow_bootstrap_recovery_plan_from_report(report))
    }

    pub async fn recover_workflow_execution_session_bootstrap<H: WorkflowHost + ?Sized>(
        &self,
        host: &H,
    ) -> Result<WorkflowExecutionSessionBootstrapRecoveryResult, WorkflowServiceError> {
        self.recover_workflow_execution_session_bootstrap_with_runtime_owner(host, None)
            .await
    }

    pub(super) async fn recover_workflow_execution_session_bootstrap_with_runtime_owner<
        H: WorkflowHost + ?Sized,
    >(
        &self,
        host: &H,
        task_execution_runtime_owner: Option<&WorkflowTaskExecutionRuntimeOwner>,
    ) -> Result<WorkflowExecutionSessionBootstrapRecoveryResult, WorkflowServiceError> {
        let plan = self.workflow_execution_session_bootstrap_recovery_plan()?;
        workflow_bootstrap_recovery_apply_gate(&plan)?;

        for request in workflow_bootstrap_recovery_progress_loop_requests(&plan) {
            self.resume_workflow_execution_session_progress_loop(request)
                .await?;
        }

        let resume_plan = self.workflow_execution_session_bootstrap_recovery_plan()?;
        workflow_bootstrap_recovery_apply_gate(&resume_plan)?;
        let runtime_resume_requests =
            workflow_bootstrap_recovery_runtime_resume_requests(&resume_plan);
        if !runtime_resume_requests.is_empty() && task_execution_runtime_owner.is_none() {
            return Err(WorkflowServiceError::CapabilityViolation(
                "bootstrap runtime recovery requires WorkflowSessionExecutionRuntime composition-root entrypoint"
                    .to_string(),
            ));
        }
        let mut resumed_runs = Vec::new();
        for runtime_resume in runtime_resume_requests {
            resumed_runs.push(
                self.resume_workflow_execution_session_runtime_dependency_readiness_with_attempt_transition(
                    host,
                    runtime_resume.request,
                    runtime_resume.attempt_start_transition,
                    task_execution_runtime_owner,
                )
                .await?,
            );
        }

        let final_plan = self.workflow_execution_session_bootstrap_recovery_plan()?;
        Ok(WorkflowExecutionSessionBootstrapRecoveryResult {
            plan,
            final_plan,
            resumed_runs,
        })
    }

    async fn resume_workflow_execution_session_progress_loop(
        &self,
        request: WorkflowExecutionSessionResumeRequest,
    ) -> Result<(), WorkflowServiceError> {
        let session_id = request.session_id.trim().to_string();
        if session_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "session_id must be non-empty".to_string(),
            ));
        }
        let workflow_run_id = request.workflow_run_id.trim().to_string();
        if workflow_run_id.is_empty() {
            return Err(WorkflowServiceError::InvalidRequest(
                "workflow_run_id must be non-empty".to_string(),
            ));
        }

        {
            let store = self.session_store_guard()?;
            let (_, records) = store
                .active_run_scheduler_task_state(&session_id, &workflow_run_id)?
                .ok_or_else(|| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "workflow run '{}' has no active scheduler task state to resume",
                        workflow_run_id
                    ))
                })?;
            if !records.iter().any(|record| {
                record.state.kind() == pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs
            }) {
                return Err(WorkflowServiceError::InvalidRequest(format!(
                    "workflow run '{}' has no scheduler task awaiting input progress",
                    workflow_run_id
                )));
            }
        }

        WorkflowSchedulerSessionRunner::new(self)
            .resume_progress_loop(&session_id, &workflow_run_id)
            .await
    }

    pub(super) fn workflow_run_snapshot_for_execution_resume_if_configured(
        &self,
        workflow_run_id: &WorkflowRunId,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, WorkflowServiceError> {
        if self.attribution_store.is_none() {
            return Ok(None);
        }
        let store = self.attribution_store_guard()?;
        store
            .workflow_run_snapshot(workflow_run_id)
            .map_err(WorkflowServiceError::from)
    }

    async fn scheduler_task_graph_for_session_run<H: WorkflowHost + ?Sized>(
        &self,
        host: &H,
        workflow_id: &str,
        workflow_run_id: &str,
        run_snapshot: Option<&WorkflowRunSnapshotRecord>,
    ) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
        let graph = host.workflow_graph(workflow_id).await?;
        validate_workflow_graph_submit_readiness(&graph)?;
        let workflow_id = WorkflowId::try_from(workflow_id.to_string())?;
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        if let Some(run_snapshot) = run_snapshot {
            let snapshot = self.workflow_executable_validation_snapshot(
                WorkflowExecutableValidationSnapshotLookupRequest {
                    workflow_version_id: run_snapshot.workflow_version_id.clone(),
                    workflow_execution_fingerprint: run_snapshot
                        .workflow_execution_fingerprint
                        .clone(),
                    descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
                },
            )?;
            let projections = snapshot
                .scheduler_inference_task_projections()
                .map_err(|error| WorkflowServiceError::InvalidRequest(error.to_string()))?;
            return workflow_scheduler_task_graph_with_inference_projections(
                &workflow_id,
                &workflow_run_id,
                &graph,
                &projections,
            );
        }

        let task_graph = workflow_scheduler_task_graph(&workflow_id, &workflow_run_id, &graph)?;
        if task_graph.tasks.iter().any(|task| {
            task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
        }) {
            return Err(WorkflowServiceError::InvalidRequest(
                "runtime inference queue admission requires a saved executable validation snapshot"
                    .to_string(),
            ));
        }
        Ok(task_graph)
    }

    pub(super) fn fail_unhandled_scheduler_session_classes(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        {
            let mut store = self.session_store_guard()?;
            self.scheduler_task_orchestrator
                .fail_unhandled_task_classes_for_active_run(&mut store, session_id, workflow_run_id)
                .map_err(|error| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler unhandled task-class fail-closed transition failed: {error}"
                    ))
                })?;
        }
        Err(WorkflowServiceError::CapabilityViolation(format!(
            "scheduler task session runner has no execution path for pumas_materialization={pumas}, unsupported={unsupported}, invalid_task_states={invalid}; add a typed scheduler execution path before running this graph",
            pumas = summary.pumas_materialization_tasks,
            unsupported = summary.unsupported_tasks,
            invalid = summary.invalid_task_states
        )))
    }

    async fn create_queued_run_snapshot_if_configured<H: WorkflowHost + ?Sized>(
        &self,
        host: &H,
        session: &WorkflowExecutionSessionSummary,
        workflow_run_id: &str,
        request: &WorkflowExecutionSessionRunRequest,
    ) -> Result<Option<WorkflowRunSnapshotRecord>, WorkflowServiceError> {
        if self.attribution_store.is_none() {
            return Ok(None);
        }

        let graph = host.workflow_graph(&session.workflow_id).await?;
        validate_workflow_graph_submit_readiness(&graph)?;
        let capabilities = host.workflow_capabilities(&session.workflow_id).await?;
        let version = self.resolve_workflow_graph_version(
            &session.workflow_id,
            &request.workflow_semantic_version,
            &graph,
        )?;
        let presentation_revision = self.resolve_workflow_graph_presentation_revision(
            &session.workflow_id,
            version.workflow_version_id.as_str(),
            &graph,
        )?;
        let override_selection = request
            .override_selection
            .as_ref()
            .and_then(WorkflowTechnicalFitOverride::normalized);
        let graph_settings = workflow_graph_run_settings(&graph);
        let snapshot = WorkflowRunSnapshotRequest {
            workflow_run_id: WorkflowRunId::try_from(workflow_run_id.to_string())?,
            workflow_id: version.workflow_id.clone(),
            workflow_version_id: version.workflow_version_id.clone(),
            workflow_presentation_revision_id: presentation_revision
                .workflow_presentation_revision_id
                .clone(),
            workflow_semantic_version: version.semantic_version,
            workflow_execution_fingerprint: version.execution_fingerprint,
            client_id: session_attribution_client_id(session)?,
            client_session_id: session_attribution_client_session_id(session)?,
            bucket_id: session_attribution_bucket_id(session)?,
            workflow_execution_session_id: session.session_id.clone(),
            workflow_execution_session_kind: workflow_execution_session_kind_label(
                &session.session_kind,
            )
            .to_string(),
            usage_profile: session.usage_profile.clone(),
            keep_alive: session.keep_alive,
            retention_policy: workflow_execution_session_retention_policy(session).to_string(),
            scheduler_policy: WORKFLOW_SESSION_SCHEDULER_POLICY.to_string(),
            priority: request.priority.unwrap_or(0),
            timeout_ms: request.timeout_ms,
            inputs_json: serde_json::to_string(&request.inputs).map_err(|error| {
                WorkflowServiceError::CapabilityViolation(format!(
                    "failed to encode workflow run snapshot inputs: {error}"
                ))
            })?,
            output_targets_json: request
                .output_targets
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| {
                    WorkflowServiceError::CapabilityViolation(format!(
                        "failed to encode workflow run snapshot output targets: {error}"
                    ))
                })?,
            override_selection_json: override_selection
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| {
                    WorkflowServiceError::CapabilityViolation(format!(
                        "failed to encode workflow run snapshot override selection: {error}"
                    ))
                })?,
            graph_settings_json: workflow_graph_run_settings_json(&graph_settings)?,
            runtime_requirements_json: encode_workflow_run_snapshot_json(
                "runtime requirements",
                &capabilities.runtime_requirements,
            )?,
            capability_models_json: encode_workflow_run_snapshot_json(
                "capability models",
                &capabilities.models,
            )?,
            runtime_capabilities_json: encode_workflow_run_snapshot_json(
                "runtime capabilities",
                &capabilities.runtime_capabilities,
            )?,
        };
        let mut store = self.attribution_store_guard()?;
        let snapshot = store
            .create_workflow_run_snapshot(snapshot)
            .map_err(WorkflowServiceError::from)?;
        drop(store);
        self.record_run_snapshot_accepted_event_if_configured(&snapshot, &graph)?;
        self.record_library_model_access_events_if_configured(&snapshot, &capabilities.models)?;
        Ok(Some(snapshot))
    }

    fn record_run_snapshot_failure_error(
        &self,
        session: &WorkflowExecutionSessionSummary,
        workflow_run_id: &str,
        workflow_semantic_version: Option<&str>,
        error: WorkflowServiceError,
    ) -> Result<WorkflowServiceError, WorkflowServiceError> {
        let scope = WorkflowDiagnosticRunScope {
            run: workflow_diagnostic_run_context(
                session,
                None,
                workflow_run_id,
                workflow_semantic_version,
            )?,
        };
        let outcome = self.record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::run_snapshot_failed(scope, &error)
                .with_source_instance_id("workflow-service")
                .with_cause("workflow run snapshot creation failed before queue admission"),
        )?;
        Ok(error.with_diagnostics(outcome.into_error_link(Some(workflow_run_id))))
    }

    fn record_scheduler_admission_failure_error(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        workflow_semantic_version: Option<&str>,
        error: WorkflowServiceError,
    ) -> Result<WorkflowServiceError, WorkflowServiceError> {
        let scope = WorkflowDiagnosticSchedulerScope {
            run: workflow_diagnostic_run_context(
                session,
                snapshot,
                workflow_run_id,
                workflow_semantic_version,
            )?,
            selected_runtime_id: None,
        };
        let outcome = self.record_workflow_diagnostic_error_if_configured(
            WorkflowDiagnosticErrorRecordRequest::scheduler_admission_failed(scope, &error)
                .with_source_instance_id("workflow-session-scheduler")
                .with_cause("scheduler queue/admission diagnostics failed before run start"),
        )?;
        Ok(error.with_diagnostics(outcome.into_error_link(Some(workflow_run_id))))
    }

    fn record_run_snapshot_accepted_event_if_configured(
        &self,
        snapshot: &WorkflowRunSnapshotRecord,
        graph: &WorkflowGraph,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let node_versions = workflow_executable_topology(graph)?
            .nodes
            .into_iter()
            .map(|node| RunSnapshotNodeVersionPayload {
                node_id: node.node_id,
                node_type: node.node_type,
                contract_version: node.contract_version,
                behavior_digest: node.behavior_digest,
            })
            .collect();
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::WorkflowService,
                source_instance_id: Some("workflow-service".to_string()),
                occurred_at_ms: snapshot.created_at_ms,
                workflow_run_id: Some(snapshot.workflow_run_id.clone()),
                workflow_id: Some(snapshot.workflow_id.clone()),
                workflow_version_id: Some(snapshot.workflow_version_id.clone()),
                workflow_semantic_version: Some(snapshot.workflow_semantic_version.clone()),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: snapshot.client_id.clone(),
                client_session_id: snapshot.client_session_id.clone(),
                bucket_id: snapshot.bucket_id.clone(),
                scheduler_policy_id: Some(snapshot.scheduler_policy.clone()),
                retention_policy_id: Some(snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RunSnapshotAccepted(RunSnapshotAcceptedPayload {
                    workflow_run_snapshot_id: snapshot
                        .workflow_run_snapshot_id
                        .as_str()
                        .to_string(),
                    workflow_presentation_revision_id: snapshot
                        .workflow_presentation_revision_id
                        .as_str()
                        .to_string(),
                    workflow_execution_session_id: snapshot.workflow_execution_session_id.clone(),
                    node_versions,
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn record_library_model_access_events_if_configured(
        &self,
        snapshot: &WorkflowRunSnapshotRecord,
        models: &[WorkflowCapabilityModel],
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        if models.is_empty() {
            return Ok(());
        }

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        for model in models {
            self.append_diagnostic_event_and_request_projection_refresh(
                &mut *ledger,
                DiagnosticEventAppendRequest {
                    source_component: DiagnosticEventSourceComponent::Library,
                    source_instance_id: Some("workflow-run-library-audit".to_string()),
                    occurred_at_ms: snapshot.created_at_ms,
                    workflow_run_id: Some(snapshot.workflow_run_id.clone()),
                    workflow_id: Some(snapshot.workflow_id.clone()),
                    workflow_version_id: Some(snapshot.workflow_version_id.clone()),
                    workflow_semantic_version: Some(snapshot.workflow_semantic_version.clone()),
                    node_id: single_model_node_id(model),
                    node_type: None,
                    node_version: None,
                    runtime_id: None,
                    runtime_version: None,
                    model_id: Some(model.model_id.clone()),
                    model_version: model.model_revision_or_hash.clone(),
                    client_id: snapshot.client_id.clone(),
                    client_session_id: snapshot.client_session_id.clone(),
                    bucket_id: snapshot.bucket_id.clone(),
                    scheduler_policy_id: Some(snapshot.scheduler_policy.clone()),
                    retention_policy_id: Some(snapshot.retention_policy.clone()),
                    privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                    retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                    payload_ref: None,
                    payload: DiagnosticEventPayload::LibraryAssetAccessed(
                        LibraryAssetAccessedPayload {
                            asset_id: pumas_model_asset_id(&model.model_id),
                            operation: LibraryAssetOperation::RunUsage,
                            cache_status: None,
                            network_bytes: None,
                        },
                    ),
                },
            )
            .map_err(WorkflowServiceError::from)?;
        }
        Ok(())
    }

    pub(super) fn record_scheduler_estimate_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        queued_item: &WorkflowExecutionSessionQueueItem,
        workflow_semantic_version: Option<&str>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let queue_position = queue_position_u32(queued_item)?;
        let workflow_run_id = WorkflowRunId::try_from(queued_item.workflow_run_id.clone())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let estimate = scheduler_estimate_context_from_snapshot(queue_position, snapshot)?;

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms: queued_item
                    .enqueued_at_ms
                    .map(|value| value as i64)
                    .unwrap_or_else(|| unix_timestamp_ms() as i64),
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: snapshot
                    .map(|snapshot| snapshot.workflow_semantic_version.clone())
                    .or_else(|| workflow_semantic_version.map(str::to_string)),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: event_client_id(session, snapshot)?,
                client_session_id: event_client_session_id(session, snapshot)?,
                bucket_id: event_bucket_id(session, snapshot)?,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::SchedulerEstimateProduced(
                    SchedulerEstimateProducedPayload {
                        estimate_version: "session-scheduler-v1".to_string(),
                        confidence: estimate.confidence,
                        estimated_queue_wait_ms: None,
                        estimated_duration_ms: None,
                        model_cache_state: Some(estimate.model_cache_state),
                        blocking_conditions: estimate.blocking_conditions,
                        missing_asset_ids: Vec::new(),
                        candidate_runtime_ids: estimate.candidate_runtime_ids,
                        candidate_device_ids: Vec::new(),
                        candidate_network_node_ids: Vec::new(),
                        reasons: estimate.reasons,
                    },
                ),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn record_scheduler_queue_placement_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        queued_item: &WorkflowExecutionSessionQueueItem,
        request: &WorkflowExecutionSessionRunRequest,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let queue_position = queue_position_u32(queued_item)?;
        let workflow_run_id = WorkflowRunId::try_from(queued_item.workflow_run_id.clone())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = queued_item.enqueued_at_ms.unwrap_or_default() as i64;

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms,
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: Some(
                    snapshot
                        .map(|snapshot| snapshot.workflow_semantic_version.clone())
                        .unwrap_or_else(|| request.workflow_semantic_version.clone()),
                ),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: event_client_id(session, snapshot)?,
                client_session_id: event_client_session_id(session, snapshot)?,
                bucket_id: event_bucket_id(session, snapshot)?,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::SchedulerQueuePlacement(
                    SchedulerQueuePlacementPayload {
                        queue_position,
                        priority: queued_item.priority,
                        scheduler_policy_id: WORKFLOW_SESSION_SCHEDULER_POLICY.to_string(),
                    },
                ),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    pub(super) fn record_run_started_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        queued_run: &crate::scheduler::WorkflowExecutionSessionDequeuedRun,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(queued_run.queued.workflow_run_id.clone())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = i64::try_from(queued_run.dequeued_at_ms).unwrap_or(i64::MAX);
        let queue_wait_ms = queued_run
            .dequeued_at_ms
            .checked_sub(queued_run.enqueued_at_ms);

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms,
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: Some(
                    snapshot
                        .map(|snapshot| snapshot.workflow_semantic_version.clone())
                        .unwrap_or_else(|| queued_run.queued.workflow_semantic_version.clone()),
                ),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: event_client_id(session, snapshot)?,
                client_session_id: event_client_session_id(session, snapshot)?,
                bucket_id: event_bucket_id(session, snapshot)?,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RunStarted(RunStartedPayload {
                    queue_wait_ms,
                    scheduler_decision_reason: Some(
                        queued_run.scheduler_decision_reason.as_str().to_string(),
                    ),
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    pub(super) fn record_active_run_started_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        active_run: &crate::scheduler::WorkflowExecutionSessionActiveRunContext,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = i64::try_from(active_run.dequeued_at_ms).unwrap_or(i64::MAX);
        let queue_wait_ms = active_run
            .dequeued_at_ms
            .checked_sub(active_run.enqueued_at_ms);

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms,
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: Some(
                    snapshot
                        .map(|snapshot| snapshot.workflow_semantic_version.clone())
                        .unwrap_or_else(|| active_run.workflow_semantic_version.clone()),
                ),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: event_client_id(session, snapshot)?,
                client_session_id: event_client_session_id(session, snapshot)?,
                bucket_id: event_bucket_id(session, snapshot)?,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RunStarted(RunStartedPayload {
                    queue_wait_ms,
                    scheduler_decision_reason: Some(
                        active_run.scheduler_decision_reason.as_str().to_string(),
                    ),
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    pub(super) fn record_workflow_io_artifact_events_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        workflow_semantic_version: &str,
        inputs: &[WorkflowPortBinding],
        outputs: &[WorkflowPortBinding],
    ) -> Result<(), WorkflowServiceError> {
        let Some(diagnostics_ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = unix_timestamp_ms() as i64;
        let node_types = workflow_run_node_types(snapshot)?;

        for (role, role_label, binding) in inputs
            .iter()
            .map(|binding| (IoArtifactRole::WorkflowInput, "workflow_input", binding))
            .chain(outputs.iter().flat_map(|binding| {
                [
                    (IoArtifactRole::WorkflowOutput, "workflow_output", binding),
                    (IoArtifactRole::NodeOutput, "node_output", binding),
                ]
            }))
        {
            let metadata = workflow_io_artifact_metadata(
                self,
                workflow_run_id.as_str(),
                workflow_id.as_str(),
                snapshot
                    .map(|snapshot| snapshot.workflow_version_id.as_str())
                    .unwrap_or(workflow_semantic_version),
                role_label,
                binding,
            )?;
            let mut ledger = diagnostics_ledger.lock().map_err(|_| {
                WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
            })?;
            self.append_diagnostic_event_and_request_projection_refresh(
                &mut *ledger,
                DiagnosticEventAppendRequest {
                    source_component: DiagnosticEventSourceComponent::WorkflowService,
                    source_instance_id: Some("workflow-service".to_string()),
                    occurred_at_ms,
                    workflow_run_id: Some(workflow_run_id.clone()),
                    workflow_id: Some(workflow_id.clone()),
                    workflow_version_id: snapshot
                        .map(|snapshot| snapshot.workflow_version_id.clone()),
                    workflow_semantic_version: Some(
                        snapshot
                            .map(|snapshot| snapshot.workflow_semantic_version.clone())
                            .unwrap_or_else(|| workflow_semantic_version.to_string()),
                    ),
                    node_id: Some(binding.node_id.clone()),
                    node_type: node_types.get(&binding.node_id).cloned(),
                    node_version: None,
                    runtime_id: None,
                    runtime_version: None,
                    model_id: None,
                    model_version: None,
                    client_id: event_client_id(session, snapshot)?,
                    client_session_id: event_client_session_id(session, snapshot)?,
                    bucket_id: event_bucket_id(session, snapshot)?,
                    scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                    retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                    privacy_class: metadata.privacy_class,
                    retention_class: metadata.retention_class,
                    payload_ref: metadata.payload_ref.clone(),
                    payload: DiagnosticEventPayload::IoArtifactObserved(
                        IoArtifactObservedPayload {
                            artifact_fact_id: Some(metadata.artifact_fact_id),
                            payload_artifact_id: Some(metadata.payload_artifact_id),
                            artifact_id: metadata.artifact_id,
                            artifact_role: role.clone(),
                            logical_payload_lineage_id: Some(metadata.logical_payload_lineage_id),
                            producer_node_id: matches!(
                                role,
                                IoArtifactRole::NodeOutput | IoArtifactRole::WorkflowOutput
                            )
                            .then(|| binding.node_id.clone()),
                            producer_port_id: matches!(
                                role,
                                IoArtifactRole::NodeOutput | IoArtifactRole::WorkflowOutput
                            )
                            .then(|| binding.port_id.clone()),
                            consumer_node_id: matches!(
                                role,
                                IoArtifactRole::NodeInput | IoArtifactRole::WorkflowInput
                            )
                            .then(|| binding.node_id.clone()),
                            consumer_port_id: matches!(
                                role,
                                IoArtifactRole::NodeInput | IoArtifactRole::WorkflowInput
                            )
                            .then(|| binding.port_id.clone()),
                            media_type: metadata.media_type,
                            size_bytes: metadata.size_bytes,
                            content_hash: metadata.content_hash,
                            retention_state: Some(metadata.retention_state),
                            retention_reason: metadata.retention_reason,
                            payload_kind: metadata.payload_kind,
                            lifecycle_state: metadata.lifecycle_state,
                            access_modes: metadata.access_modes,
                            read_handle: metadata.read_handle,
                            stream_handle: metadata.stream_handle,
                            format: metadata.format,
                        },
                    ),
                },
            )
            .map_err(WorkflowServiceError::from)?;
        }
        Ok(())
    }

    pub(super) fn record_run_terminal_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        workflow_semantic_version: Option<&str>,
        run_result: &Result<WorkflowRunResponse, WorkflowServiceError>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = unix_timestamp_ms() as i64;
        let (status, duration_ms, error, canonical_error_event_id) = match run_result {
            Ok(response) => (
                RunTerminalStatus::Completed,
                Some(response.timing_ms.min(u128::from(u64::MAX)) as u64),
                None,
                None,
            ),
            Err(WorkflowServiceError::Cancelled(message)) => (
                RunTerminalStatus::Cancelled,
                None,
                Some(sanitize_diagnostic_error_text(message)),
                None,
            ),
            Err(error) => (
                RunTerminalStatus::Failed,
                None,
                Some(sanitize_diagnostic_error_text(&error.to_string())),
                error
                    .diagnostics()
                    .and_then(|diagnostics| diagnostics.diagnostic_event_id.clone()),
            ),
        };

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        let resource_observation = DiagnosticsLedgerRepository::run_resource_observation_rollup(
            &*ledger,
            RunResourceObservationRollupQuery {
                workflow_run_id: workflow_run_id.clone(),
            },
        )
        .map_err(WorkflowServiceError::from)?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::WorkflowService,
                source_instance_id: Some("workflow-service".to_string()),
                occurred_at_ms,
                workflow_run_id: Some(workflow_run_id),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: snapshot
                    .map(|snapshot| snapshot.workflow_semantic_version.clone())
                    .or_else(|| workflow_semantic_version.map(str::to_string)),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: None,
                runtime_version: None,
                model_id: None,
                model_version: None,
                client_id: event_client_id(session, snapshot)?,
                client_session_id: event_client_session_id(session, snapshot)?,
                bucket_id: event_bucket_id(session, snapshot)?,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::RunTerminal(RunTerminalPayload {
                    status,
                    duration_ms,
                    error,
                    canonical_error_event_id,
                    resource_observation,
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }
}

fn workflow_response_from_runtime_branch_worker_outcome(
    outcome: WorkflowTaskExecutionWorkerOutcome,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    match outcome {
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchCompleted(outcome) => Ok(outcome.response),
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchDeferred(outcome) => match outcome.reason {
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::DependencyReadinessPending => {
                Err(WorkflowServiceError::RuntimeDependencyReadinessPending {
                    message: worker_diagnostic_message(
                        "runtime dependency readiness is pending for runtime branch worker",
                        &outcome.diagnostics,
                    ),
                    task_ids: outcome.deferred_task_ids,
                })
            }
            WorkflowTaskExecutionWorkerRuntimeBranchDeferredReason::RuntimeDispatchUnavailable => {
                Err(WorkflowServiceError::RuntimeNotReady(worker_diagnostic_message(
                    "runtime branch dispatch is unavailable in the task execution worker",
                    &outcome.diagnostics,
                )))
            }
        },
        WorkflowTaskExecutionWorkerOutcome::RuntimeBranchFailed(outcome) => {
            if outcome
                .error_message
                .contains("runtime dispatch task was cancelled before completion")
            {
                return Err(WorkflowServiceError::Cancelled(worker_diagnostic_message(
                    &outcome.error_message,
                    &outcome.diagnostics,
                )));
            }
            Err(WorkflowServiceError::Internal(worker_diagnostic_message(
                &outcome.error_message,
                &outcome.diagnostics,
            )))
        }
        WorkflowTaskExecutionWorkerOutcome::WorkerUnavailable(diagnostic) => {
            Err(WorkflowServiceError::Internal(diagnostic.message))
        }
        WorkflowTaskExecutionWorkerOutcome::ShutdownAccepted
        | WorkflowTaskExecutionWorkerOutcome::TaskTerminal(_)
        | WorkflowTaskExecutionWorkerOutcome::TaskDeferred(_) => Err(WorkflowServiceError::Internal(
            "task execution worker returned a non-runtime-branch outcome for runtime branch execution"
                .to_string(),
        )),
    }
}

fn workflow_task_execution_worker_runtime_branch_start_reason(
    transition: SchedulerTaskAttemptLifecycleTransition,
) -> Result<WorkflowTaskExecutionWorkerRuntimeBranchStartReason, WorkflowServiceError> {
    match transition {
        SchedulerTaskAttemptLifecycleTransition::Started => {
            Ok(WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Started)
        }
        SchedulerTaskAttemptLifecycleTransition::Redispatched => {
            Ok(WorkflowTaskExecutionWorkerRuntimeBranchStartReason::Redispatched)
        }
        SchedulerTaskAttemptLifecycleTransition::Completed
        | SchedulerTaskAttemptLifecycleTransition::Failed
        | SchedulerTaskAttemptLifecycleTransition::Cancelled => {
            Err(WorkflowServiceError::Internal(
                "terminal scheduler task attempt transition cannot start runtime branch execution"
                    .to_string(),
            ))
        }
    }
}

fn worker_diagnostic_message(
    fallback: &str,
    diagnostics: &[super::task_execution_worker::WorkflowTaskExecutionWorkerDiagnostic],
) -> String {
    diagnostics
        .first()
        .map(|diagnostic| diagnostic.message.clone())
        .unwrap_or_else(|| fallback.to_string())
}

fn workflow_bootstrap_recovery_run_from_scheduler(
    snapshot: WorkflowSchedulerBootstrapRecoverySnapshot,
) -> WorkflowExecutionSessionBootstrapRecoveryRun {
    WorkflowExecutionSessionBootstrapRecoveryRun {
        session_id: snapshot.session_id,
        workflow_run_id: snapshot.workflow_run_id,
        runtime_tasks: snapshot
            .runtime_tasks
            .into_iter()
            .map(workflow_bootstrap_recovery_task_from_scheduler)
            .collect(),
    }
}

fn workflow_bootstrap_recovery_task_from_scheduler(
    task: WorkflowSchedulerBootstrapRecoveryTask,
) -> WorkflowExecutionSessionBootstrapRecoveryTask {
    WorkflowExecutionSessionBootstrapRecoveryTask {
        task_id: task.task_id,
        state_kind: task.state_kind,
        action: workflow_bootstrap_recovery_action_from_scheduler(task.action),
        runtime_dispatch_recovery_state_available: task.runtime_dispatch_recovery_state_available,
    }
}

fn workflow_bootstrap_recovery_action_from_scheduler(
    action: WorkflowSchedulerBootstrapRecoveryAction,
) -> WorkflowExecutionSessionBootstrapRecoveryAction {
    match action {
        WorkflowSchedulerBootstrapRecoveryAction::ResumeProgressLoop => {
            WorkflowExecutionSessionBootstrapRecoveryAction::ResumeProgressLoop
        }
        WorkflowSchedulerBootstrapRecoveryAction::RetryDependencyReadiness => {
            WorkflowExecutionSessionBootstrapRecoveryAction::RetryDependencyReadiness
        }
        WorkflowSchedulerBootstrapRecoveryAction::RedispatchReadyRuntime => {
            WorkflowExecutionSessionBootstrapRecoveryAction::RedispatchReadyRuntime
        }
        WorkflowSchedulerBootstrapRecoveryAction::RuntimeRecoveryRequired => {
            WorkflowExecutionSessionBootstrapRecoveryAction::RuntimeRecoveryRequired
        }
        WorkflowSchedulerBootstrapRecoveryAction::Completed => {
            WorkflowExecutionSessionBootstrapRecoveryAction::Completed
        }
        WorkflowSchedulerBootstrapRecoveryAction::TerminalDiagnostic => {
            WorkflowExecutionSessionBootstrapRecoveryAction::TerminalDiagnostic
        }
        WorkflowSchedulerBootstrapRecoveryAction::MissingTaskStateRecord => {
            WorkflowExecutionSessionBootstrapRecoveryAction::MissingTaskStateRecord
        }
    }
}

fn workflow_bootstrap_recovery_plan_from_report(
    report: WorkflowExecutionSessionBootstrapRecoveryReport,
) -> WorkflowExecutionSessionBootstrapRecoveryPlan {
    let mut decisions = Vec::new();
    let mut resume_requests = Vec::new();
    let mut resume_request_keys = BTreeSet::new();
    let mut blocking_decision_count = 0;

    for run in report.active_runs {
        for task in run.runtime_tasks {
            let decision_kind = workflow_bootstrap_recovery_decision_kind(&task);
            let diagnostic = workflow_bootstrap_recovery_diagnostic(decision_kind, &task);
            if workflow_bootstrap_recovery_decision_blocks(decision_kind) {
                blocking_decision_count += 1;
            }
            if decision_kind
                == WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness
            {
                let key = (run.session_id.clone(), run.workflow_run_id.clone());
                if resume_request_keys.insert(key) {
                    resume_requests.push(WorkflowExecutionSessionResumeRequest {
                        session_id: run.session_id.clone(),
                        workflow_run_id: run.workflow_run_id.clone(),
                    });
                }
            }
            decisions.push(WorkflowExecutionSessionBootstrapRecoveryDecision {
                session_id: run.session_id.clone(),
                workflow_run_id: run.workflow_run_id.clone(),
                task_id: task.task_id,
                state_kind: task.state_kind,
                recovery_action: task.action,
                runtime_dispatch_recovery_state_available: task
                    .runtime_dispatch_recovery_state_available,
                decision_kind,
                diagnostic,
            });
        }
    }

    WorkflowExecutionSessionBootstrapRecoveryPlan {
        decisions,
        resume_requests,
        blocking_decision_count,
    }
}

fn workflow_bootstrap_recovery_decision_kind(
    task: &WorkflowExecutionSessionBootstrapRecoveryTask,
) -> WorkflowExecutionSessionBootstrapRecoveryDecisionKind {
    match task.action {
        WorkflowExecutionSessionBootstrapRecoveryAction::ResumeProgressLoop => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::RetryDependencyReadiness => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::RedispatchReadyRuntime => {
            if task.runtime_dispatch_recovery_state_available {
                WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
            } else {
                WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRedispatchRecoveryStateRequired
            }
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::RuntimeRecoveryRequired => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRecoveryRequired
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::Completed => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::NoopCompleted
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::TerminalDiagnostic => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedTerminalDiagnostic
        }
        WorkflowExecutionSessionBootstrapRecoveryAction::MissingTaskStateRecord => {
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedMissingTaskStateRecord
        }
    }
}

fn workflow_bootstrap_recovery_decision_blocks(
    decision_kind: WorkflowExecutionSessionBootstrapRecoveryDecisionKind,
) -> bool {
    matches!(
        decision_kind,
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRedispatchRecoveryStateRequired
            | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRecoveryRequired
            | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedTerminalDiagnostic
            | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedMissingTaskStateRecord
    )
}

fn workflow_bootstrap_recovery_diagnostic(
    decision_kind: WorkflowExecutionSessionBootstrapRecoveryDecisionKind,
    task: &WorkflowExecutionSessionBootstrapRecoveryTask,
) -> Option<String> {
    match decision_kind {
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRedispatchRecoveryStateRequired => {
            if task.runtime_dispatch_recovery_state_available {
                Some(format!(
                    "runtime task '{}' is ready for dispatch with persisted readiness proof; bootstrap recovery requires duplicate-dispatch guard state before redispatch",
                    task.task_id
                ))
            } else {
                Some(format!(
                    "runtime task '{}' is ready for dispatch; bootstrap recovery requires persisted readiness proof and duplicate-dispatch guard state before redispatch",
                    task.task_id
                ))
            }
        }
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRecoveryRequired => {
            Some(format!(
                "runtime task '{}' requires runtime recovery before bootstrap can resume work",
                task.task_id
            ))
        }
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedTerminalDiagnostic => {
            Some(format!(
                "runtime task '{}' is in a terminal diagnostic state and cannot be replayed",
                task.task_id
            ))
        }
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedMissingTaskStateRecord => {
            Some(format!(
                "runtime task '{}' is missing canonical scheduler task state and cannot be replayed",
                task.task_id
            ))
        }
        _ => None,
    }
}

fn workflow_bootstrap_recovery_apply_gate(
    plan: &WorkflowExecutionSessionBootstrapRecoveryPlan,
) -> Result<(), WorkflowServiceError> {
    if plan.blocking_decision_count > 0 {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "bootstrap recovery plan contains {} blocking decision(s); inspect the recovery plan diagnostics before applying recovery",
            plan.blocking_decision_count
        )));
    }

    if let Some(decision) = plan.decisions.iter().find(|decision| {
        !matches!(
            decision.decision_kind,
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness
                | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
                | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
                | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::NoopCompleted
        )
    }) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "bootstrap recovery decision {:?} for task '{}' is not supported by the current recovery runner",
            decision.decision_kind, decision.task_id
        )));
    }

    Ok(())
}

fn workflow_bootstrap_recovery_progress_loop_requests(
    plan: &WorkflowExecutionSessionBootstrapRecoveryPlan,
) -> Vec<WorkflowExecutionSessionResumeRequest> {
    let mut requests = Vec::new();
    let mut request_keys = BTreeSet::new();
    for decision in plan.decisions.iter().filter(|decision| {
        decision.decision_kind
            == WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
    }) {
        let key = (
            decision.session_id.clone(),
            decision.workflow_run_id.clone(),
        );
        if request_keys.insert(key) {
            requests.push(WorkflowExecutionSessionResumeRequest {
                session_id: decision.session_id.clone(),
                workflow_run_id: decision.workflow_run_id.clone(),
            });
        }
    }
    requests
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowBootstrapRecoveryRuntimeResume {
    request: WorkflowExecutionSessionResumeRequest,
    attempt_start_transition: SchedulerTaskAttemptLifecycleTransition,
}

fn workflow_bootstrap_recovery_runtime_resume_requests(
    plan: &WorkflowExecutionSessionBootstrapRecoveryPlan,
) -> Vec<WorkflowBootstrapRecoveryRuntimeResume> {
    let mut requests = Vec::new();
    let mut request_keys = BTreeSet::new();
    for decision in plan.decisions.iter().filter(|decision| {
        matches!(
            decision.decision_kind,
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness
                | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
        )
    }) {
        let key = (
            decision.session_id.clone(),
            decision.workflow_run_id.clone(),
        );
        if request_keys.insert(key) {
            requests.push(WorkflowBootstrapRecoveryRuntimeResume {
                request: WorkflowExecutionSessionResumeRequest {
                    session_id: decision.session_id.clone(),
                    workflow_run_id: decision.workflow_run_id.clone(),
                },
                attempt_start_transition: workflow_bootstrap_recovery_attempt_start_transition(
                    decision.decision_kind,
                ),
            });
        }
    }
    requests
}

fn workflow_bootstrap_recovery_attempt_start_transition(
    decision_kind: WorkflowExecutionSessionBootstrapRecoveryDecisionKind,
) -> SchedulerTaskAttemptLifecycleTransition {
    match decision_kind {
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime => {
            SchedulerTaskAttemptLifecycleTransition::Redispatched
        }
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeRuntimeDependencyReadiness => {
            SchedulerTaskAttemptLifecycleTransition::Started
        }
        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop
        | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRedispatchRecoveryStateRequired
        | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRecoveryRequired
        | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::NoopCompleted
        | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedTerminalDiagnostic
        | WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedMissingTaskStateRecord => {
            SchedulerTaskAttemptLifecycleTransition::Started
        }
    }
}

fn queue_position_u32(
    queued_item: &WorkflowExecutionSessionQueueItem,
) -> Result<u32, WorkflowServiceError> {
    queued_item
        .queue_position
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "queued run '{}' missing queue position",
                queued_item.workflow_run_id
            ))
        })
        .and_then(|position| {
            u32::try_from(position).map_err(|_| {
                WorkflowServiceError::Internal(format!(
                    "queue position '{}' exceeds scheduler event limit",
                    position
                ))
            })
        })
}

fn sanitize_diagnostic_error_text(value: &str) -> String {
    const MAX_DIAGNOSTIC_TEXT_BYTES: usize = 65_536;

    let mut sanitized = String::with_capacity(value.len().min(MAX_DIAGNOSTIC_TEXT_BYTES));
    for ch in value.chars() {
        let replacement = if ch.is_control() { ' ' } else { ch };
        if sanitized.len() + replacement.len_utf8() > MAX_DIAGNOSTIC_TEXT_BYTES {
            break;
        }
        sanitized.push(replacement);
    }

    if sanitized.trim().is_empty() && !value.is_empty() {
        "runtime error contained only control characters".to_string()
    } else {
        sanitized
    }
}

fn workflow_id_for_scheduler_event(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<WorkflowId, WorkflowServiceError> {
    match snapshot {
        Some(snapshot) => Ok(snapshot.workflow_id.clone()),
        None => {
            WorkflowId::try_from(session.workflow_id.clone()).map_err(WorkflowServiceError::from)
        }
    }
}

fn workflow_diagnostic_run_context(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
    workflow_run_id: &str,
    workflow_semantic_version: Option<&str>,
) -> Result<WorkflowDiagnosticRunContext, WorkflowServiceError> {
    Ok(WorkflowDiagnosticRunContext {
        workflow_run_id: WorkflowRunId::try_from(workflow_run_id.to_string())?,
        workflow_id: workflow_id_for_scheduler_event(session, snapshot)?,
        workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
        workflow_semantic_version: snapshot
            .map(|snapshot| snapshot.workflow_semantic_version.clone())
            .or_else(|| workflow_semantic_version.map(str::to_string)),
        client_id: event_client_id(session, snapshot)?,
        client_session_id: event_client_session_id(session, snapshot)?,
        bucket_id: event_bucket_id(session, snapshot)?,
        scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
        retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
    })
}

fn session_attribution_client_id(
    session: &WorkflowExecutionSessionSummary,
) -> Result<Option<ClientId>, WorkflowServiceError> {
    session
        .attribution
        .as_ref()
        .map(|context| ClientId::try_from(context.client_id.clone()))
        .transpose()
        .map_err(WorkflowServiceError::from)
}

fn session_attribution_client_session_id(
    session: &WorkflowExecutionSessionSummary,
) -> Result<Option<ClientSessionId>, WorkflowServiceError> {
    session
        .attribution
        .as_ref()
        .map(|context| ClientSessionId::try_from(context.client_session_id.clone()))
        .transpose()
        .map_err(WorkflowServiceError::from)
}

fn session_attribution_bucket_id(
    session: &WorkflowExecutionSessionSummary,
) -> Result<Option<BucketId>, WorkflowServiceError> {
    session
        .attribution
        .as_ref()
        .map(|context| BucketId::try_from(context.bucket_id.clone()))
        .transpose()
        .map_err(WorkflowServiceError::from)
}

fn event_client_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<ClientId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.client_id.clone()) {
        Some(client_id) => Ok(Some(client_id)),
        None => session_attribution_client_id(session),
    }
}

fn event_client_session_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<ClientSessionId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.client_session_id.clone()) {
        Some(client_session_id) => Ok(Some(client_session_id)),
        None => session_attribution_client_session_id(session),
    }
}

fn event_bucket_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<BucketId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.bucket_id.clone()) {
        Some(bucket_id) => Ok(Some(bucket_id)),
        None => session_attribution_bucket_id(session),
    }
}

fn encode_workflow_run_snapshot_json<T: serde::Serialize>(
    label: &str,
    value: &T,
) -> Result<String, WorkflowServiceError> {
    serde_json::to_string(value).map_err(|error| {
        WorkflowServiceError::CapabilityViolation(format!(
            "failed to encode workflow run snapshot {label}: {error}"
        ))
    })
}

fn workflow_run_node_types(
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<HashMap<String, String>, WorkflowServiceError> {
    let Some(snapshot) = snapshot else {
        return Ok(HashMap::new());
    };
    let graph_settings: WorkflowGraphRunSettings =
        serde_json::from_str(&snapshot.graph_settings_json).map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "failed to decode workflow run snapshot graph settings: {error}"
            ))
        })?;
    Ok(graph_settings
        .nodes
        .into_iter()
        .map(|node| (node.node_id, node.node_type))
        .collect())
}

fn pumas_model_asset_id(model_id: &str) -> String {
    format!("pumas://models/{model_id}")
}

fn single_model_node_id(model: &WorkflowCapabilityModel) -> Option<String> {
    (model.node_ids.len() == 1).then(|| model.node_ids[0].clone())
}

struct SchedulerEstimateContext {
    confidence: String,
    model_cache_state: SchedulerModelCacheState,
    blocking_conditions: Vec<SchedulerEstimateBlockingCondition>,
    candidate_runtime_ids: Vec<String>,
    reasons: Vec<String>,
}

const SCHEDULER_ESTIMATE_REASON_MAX_LEN: usize = 128;
const SCHEDULER_ESTIMATE_REASON_TRUNCATION_SUFFIX: &str = "...";

fn scheduler_estimate_context_from_snapshot(
    queue_position: u32,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<SchedulerEstimateContext, WorkflowServiceError> {
    let mut reasons = Vec::new();
    push_scheduler_estimate_reason(
        &mut reasons,
        if queue_position == 0 {
            "next admission candidate pending runtime readiness".to_string()
        } else {
            format!("{queue_position} run(s) ahead in session queue")
        },
    );
    let mut blocking_conditions = if queue_position == 0 {
        vec![SchedulerEstimateBlockingCondition::RuntimeAdmissionPending]
    } else {
        vec![SchedulerEstimateBlockingCondition::QueueBacklog]
    };
    let Some(snapshot) = snapshot else {
        return Ok(SchedulerEstimateContext {
            confidence: "low".to_string(),
            model_cache_state: SchedulerModelCacheState::Unknown,
            blocking_conditions,
            candidate_runtime_ids: Vec::new(),
            reasons,
        });
    };

    let runtime_requirements = workflow_run_snapshot_runtime_requirements(snapshot)?;
    append_scheduler_estimate_runtime_reasons(&mut reasons, &runtime_requirements);
    let runtime_capabilities: Vec<WorkflowRuntimeCapability> =
        serde_json::from_str(&snapshot.runtime_capabilities_json).map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "failed to decode workflow run snapshot runtime capabilities: {error}"
            ))
        })?;
    let candidate_runtime_ids =
        scheduler_candidate_runtime_ids(&runtime_requirements, &runtime_capabilities);
    if !candidate_runtime_ids.is_empty() {
        push_scheduler_estimate_reason(
            &mut reasons,
            format!("candidate runtime(s): {}", candidate_runtime_ids.join(", ")),
        );
    } else if !runtime_requirements.required_backends.is_empty() {
        blocking_conditions.push(SchedulerEstimateBlockingCondition::RuntimeUnavailable);
        push_scheduler_estimate_reason(
            &mut reasons,
            format!(
                "no compatible candidate runtime for backend(s): {}",
                runtime_requirements.required_backends.join(", ")
            ),
        );
    }
    let confidence = scheduler_estimate_confidence(&runtime_requirements);
    let model_cache_state = if runtime_requirements.required_models.is_empty() {
        SchedulerModelCacheState::NotRequired
    } else {
        blocking_conditions.push(SchedulerEstimateBlockingCondition::ModelCacheUnknown);
        SchedulerModelCacheState::Unknown
    };

    Ok(SchedulerEstimateContext {
        confidence,
        model_cache_state,
        blocking_conditions,
        candidate_runtime_ids,
        reasons,
    })
}

fn workflow_run_snapshot_runtime_requirements(
    snapshot: &WorkflowRunSnapshotRecord,
) -> Result<WorkflowRuntimeRequirements, WorkflowServiceError> {
    serde_json::from_str(&snapshot.runtime_requirements_json).map_err(|error| {
        WorkflowServiceError::Internal(format!(
            "failed to decode workflow run snapshot runtime requirements: {error}"
        ))
    })
}

fn append_scheduler_estimate_runtime_reasons(
    reasons: &mut Vec<String>,
    runtime_requirements: &WorkflowRuntimeRequirements,
) {
    if !runtime_requirements.required_backends.is_empty() {
        push_scheduler_estimate_reason(
            reasons,
            format!(
                "requires backend(s): {}",
                runtime_requirements.required_backends.join(", ")
            ),
        );
    }
    if !runtime_requirements.required_models.is_empty() {
        push_scheduler_estimate_reason(
            reasons,
            format!(
                "requires model(s): {}",
                runtime_requirements.required_models.join(", ")
            ),
        );
    }
    if !runtime_requirements.required_extensions.is_empty() {
        push_scheduler_estimate_reason(
            reasons,
            format!(
                "requires extension(s): {}",
                runtime_requirements.required_extensions.join(", ")
            ),
        );
    }
    let mut memory_estimates = Vec::new();
    for estimate in &runtime_requirements.resource_estimates {
        if estimate.state() != WorkflowTechnicalFitResourceEstimateState::Available {
            continue;
        }
        let Some(value_bytes) = estimate.value_bytes() else {
            continue;
        };
        match estimate.kind() {
            WorkflowTechnicalFitResourceEstimateKind::PeakVramBytes => {
                memory_estimates.push(format!("{value_bytes} bytes peak VRAM"));
            }
            WorkflowTechnicalFitResourceEstimateKind::PeakRamBytes => {
                memory_estimates.push(format!("{value_bytes} bytes peak RAM"));
            }
            _ => {}
        }
    }
    if !memory_estimates.is_empty() {
        push_scheduler_estimate_reason(
            reasons,
            format!("estimated peak memory: {}", memory_estimates.join(", ")),
        );
    }
}

fn scheduler_estimate_confidence(runtime_requirements: &WorkflowRuntimeRequirements) -> String {
    if runtime_requirements
        .resource_estimates
        .iter()
        .any(|estimate| estimate.state().is_available())
    {
        "estimated".to_string()
    } else {
        "low".to_string()
    }
}

fn push_scheduler_estimate_reason(reasons: &mut Vec<String>, reason: String) {
    reasons.push(truncate_scheduler_estimate_reason(reason));
}

fn truncate_scheduler_estimate_reason(reason: String) -> String {
    if reason.len() <= SCHEDULER_ESTIMATE_REASON_MAX_LEN {
        return reason;
    }

    let max_prefix_len =
        SCHEDULER_ESTIMATE_REASON_MAX_LEN - SCHEDULER_ESTIMATE_REASON_TRUNCATION_SUFFIX.len();
    let mut prefix = String::new();
    for character in reason.chars() {
        if prefix.len() + character.len_utf8() > max_prefix_len {
            break;
        }
        prefix.push(character);
    }
    prefix.push_str(SCHEDULER_ESTIMATE_REASON_TRUNCATION_SUFFIX);
    prefix
}

fn scheduler_candidate_runtime_ids(
    runtime_requirements: &WorkflowRuntimeRequirements,
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> Vec<String> {
    let mut candidate_runtime_ids = runtime_capabilities
        .iter()
        .filter(|capability| capability.available && capability.configured)
        .filter(|capability| {
            runtime_requirements.required_backends.is_empty()
                || runtime_requirements
                    .required_backends
                    .iter()
                    .any(|required| {
                        capability.runtime_id == *required
                            || capability
                                .backend_keys
                                .iter()
                                .any(|backend_key| backend_key == required)
                    })
        })
        .map(|capability| capability.runtime_id.clone())
        .collect::<Vec<_>>();
    candidate_runtime_ids.sort();
    candidate_runtime_ids.dedup();
    candidate_runtime_ids
}

fn workflow_execution_session_kind_label(kind: &WorkflowExecutionSessionKind) -> &'static str {
    match kind {
        WorkflowExecutionSessionKind::Edit => "edit",
        WorkflowExecutionSessionKind::Workflow => "workflow",
    }
}

impl WorkflowService {
    fn ensure_runtime_branch_task_events_for_recovery(
        &self,
        session_id: &str,
        workflow_id: &str,
        workflow_run_id: &str,
        output_targets: Option<Vec<super::WorkflowOutputTarget>>,
        timeout_ms: Option<u64>,
        task_graph: &WorkflowSchedulerTaskGraph,
    ) -> Result<usize, WorkflowServiceError> {
        let mut repository = self
            .runtime_branch_task_event_repository
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "runtime branch task-event repository lock poisoned".to_string(),
                )
            })?;
        let ready_at_ms = unix_timestamp_ms();
        let mut ensured = 0;
        for task in task_graph.tasks.iter().filter(|task| {
            task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
        }) {
            let event_id = WorkflowRuntimeBranchTaskEventId::parse(format!(
                "runtime-branch-task-event.{}.{}",
                workflow_run_id,
                task.task_id.as_str()
            ))
            .map_err(runtime_branch_task_event_diagnostic_error)?;
            if let Some(record) = repository.get(&event_id) {
                match record.state {
                    WorkflowRuntimeBranchTaskEventState::Ready
                    | WorkflowRuntimeBranchTaskEventState::Claimed
                    | WorkflowRuntimeBranchTaskEventState::Dispatching
                    | WorkflowRuntimeBranchTaskEventState::Running => {
                        ensured += 1;
                        continue;
                    }
                    WorkflowRuntimeBranchTaskEventState::Deferred => {
                        let _record = repository
                            .mark_deferred_ready(&event_id, ready_at_ms)
                            .map_err(runtime_branch_task_event_diagnostic_error)?;
                        ensured += 1;
                        continue;
                    }
                    WorkflowRuntimeBranchTaskEventState::Completed
                    | WorkflowRuntimeBranchTaskEventState::Failed => {
                        return Err(WorkflowServiceError::InvalidRequest(format!(
                            "runtime branch task event '{}' is terminal and cannot be recovered",
                            event_id.as_str()
                        )));
                    }
                }
            }
            let queued_input_keys = task
                .input_bindings
                .iter()
                .map(|binding| {
                    format!(
                        "{}:{}",
                        binding.source_task_id.as_str(),
                        binding.target_port_id
                    )
                })
                .collect::<Vec<_>>();
            let runtime_source_context = task
                .runtime_source_context
                .clone()
                .ok_or_else(|| missing_runtime_source_context_error(task.task_id.as_str()))?;
            let record = WorkflowRuntimeBranchTaskEventRecord::ready(
                WorkflowRuntimeBranchTaskEventRequest {
                    event_id,
                    session_id: session_id.to_string(),
                    workflow_id: workflow_id.to_string(),
                    workflow_run_id: workflow_run_id.to_string(),
                    scheduler_task_id: task.task_id.as_str().to_string(),
                    scheduler_task_attempt_id: None,
                    attempt_generation: 1,
                    queued_input_keys,
                    output_targets: output_targets.clone(),
                    timeout_ms,
                    batching_key: Some(format!(
                        "runtime-branch-task.{}.{}",
                        workflow_id,
                        task.task_id.as_str()
                    )),
                    runtime_source_context,
                    batch_eligibility: None,
                    ready_at_ms,
                },
            )
            .map_err(runtime_branch_task_event_diagnostic_error)?;
            repository
                .enqueue(record)
                .map_err(runtime_branch_task_event_diagnostic_error)?;
            ensured += 1;
        }
        if ensured == 0 {
            return Err(WorkflowServiceError::Internal(
                "runtime branch recovery found no runtime inference scheduler tasks".to_string(),
            ));
        }
        Ok(ensured)
    }

    fn persist_runtime_branch_task_events_for_admission(
        &self,
        session_id: &str,
        workflow_id: &str,
        workflow_run_id: &str,
        output_targets: Option<Vec<super::WorkflowOutputTarget>>,
        timeout_ms: Option<u64>,
        task_graph: &WorkflowSchedulerTaskGraph,
    ) -> Result<usize, WorkflowServiceError> {
        let mut repository = self
            .runtime_branch_task_event_repository
            .lock()
            .map_err(|_| {
                WorkflowServiceError::Internal(
                    "runtime branch task-event repository lock poisoned".to_string(),
                )
            })?;
        let ready_at_ms = unix_timestamp_ms();
        let mut persisted = 0;
        for task in task_graph.tasks.iter().filter(|task| {
            task.execution_class == WorkflowSchedulerTaskExecutionClass::RuntimeInference
        }) {
            let event_id = WorkflowRuntimeBranchTaskEventId::parse(format!(
                "runtime-branch-task-event.{}.{}",
                workflow_run_id,
                task.task_id.as_str()
            ))
            .map_err(runtime_branch_task_event_diagnostic_error)?;
            let queued_input_keys = task
                .input_bindings
                .iter()
                .map(|binding| {
                    format!(
                        "{}:{}",
                        binding.source_task_id.as_str(),
                        binding.target_port_id
                    )
                })
                .collect::<Vec<_>>();
            let runtime_source_context = task
                .runtime_source_context
                .clone()
                .ok_or_else(|| missing_runtime_source_context_error(task.task_id.as_str()))?;
            let record = WorkflowRuntimeBranchTaskEventRecord::ready(
                WorkflowRuntimeBranchTaskEventRequest {
                    event_id,
                    session_id: session_id.to_string(),
                    workflow_id: workflow_id.to_string(),
                    workflow_run_id: workflow_run_id.to_string(),
                    scheduler_task_id: task.task_id.as_str().to_string(),
                    scheduler_task_attempt_id: None,
                    attempt_generation: 1,
                    queued_input_keys,
                    output_targets: output_targets.clone(),
                    timeout_ms,
                    batching_key: Some(format!(
                        "runtime-branch-task.{}.{}",
                        workflow_id,
                        task.task_id.as_str()
                    )),
                    runtime_source_context,
                    batch_eligibility: None,
                    ready_at_ms,
                },
            )
            .map_err(runtime_branch_task_event_diagnostic_error)?;
            repository
                .enqueue(record)
                .map_err(runtime_branch_task_event_diagnostic_error)?;
            persisted += 1;
        }
        if persisted == 0 {
            return Err(WorkflowServiceError::Internal(
                "runtime branch admission found no runtime inference scheduler tasks".to_string(),
            ));
        }
        Ok(persisted)
    }

    #[cfg(test)]
    pub(super) fn runtime_branch_task_event_for_test(
        &self,
        event_id: &WorkflowRuntimeBranchTaskEventId,
    ) -> Option<WorkflowRuntimeBranchTaskEventRecord> {
        self.runtime_branch_task_event_repository
            .lock()
            .expect("runtime branch event repository lock")
            .get(event_id)
    }

    #[cfg(test)]
    pub(super) fn runtime_dispatch_assignment_for_test(
        &self,
        assignment_id: &WorkflowRuntimeDispatchAssignmentId,
    ) -> Option<WorkflowRuntimeDispatchAssignmentRecord> {
        self.runtime_dispatch_assignment_repository
            .lock()
            .expect("runtime dispatch assignment repository lock")
            .get(assignment_id)
    }
}

fn runtime_branch_task_event_diagnostic_error(
    diagnostic: super::runtime_branch_task_event::WorkflowRuntimeBranchTaskEventDiagnostic,
) -> WorkflowServiceError {
    WorkflowServiceError::Internal(format!(
        "runtime branch task-event admission failed ({:?}): {}",
        diagnostic.code, diagnostic.message
    ))
}

fn workflow_execution_session_retention_policy(
    session: &WorkflowExecutionSessionSummary,
) -> &'static str {
    if session.keep_alive {
        WORKFLOW_SESSION_RETENTION_KEEP_ALIVE
    } else {
        WORKFLOW_SESSION_RETENTION_EPHEMERAL
    }
}

fn missing_runtime_source_context_error(task_id: &str) -> WorkflowServiceError {
    WorkflowServiceError::Internal(format!(
        "runtime inference scheduler task '{task_id}' is missing workflow runtime source context"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::WorkflowRuntimeSourceContext;
    use crate::workflow::{
        WorkflowSchedulerTask, WorkflowSchedulerTaskInputBinding,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
    };
    use pantograph_scheduler::{
        SchedulerNodeId, SchedulerTaskId, SchedulerWorkflowId, SchedulerWorkflowRunId,
    };

    fn empty_runtime_requirements() -> WorkflowRuntimeRequirements {
        WorkflowRuntimeRequirements {
            resource_estimates: Vec::new(),
            required_models: Vec::new(),
            required_backends: Vec::new(),
            required_extensions: Vec::new(),
        }
    }

    fn runtime_source_context() -> WorkflowRuntimeSourceContext {
        WorkflowRuntimeSourceContext {
            operation_type: "image_generation".to_string(),
            context_shape_key: "image_generation.default".to_string(),
            cancellation_mode: "cooperative".to_string(),
        }
    }

    fn runtime_task_graph() -> WorkflowSchedulerTaskGraph {
        let workflow_id = SchedulerWorkflowId::parse("workflow.image").expect("workflow id");
        let workflow_run_id = SchedulerWorkflowRunId::parse("run.runtime").expect("run id");
        WorkflowSchedulerTaskGraph {
            schema_version: WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
            workflow_id: workflow_id.clone(),
            workflow_run_id: workflow_run_id.clone(),
            tasks: vec![
                WorkflowSchedulerTask {
                    workflow_id: workflow_id.clone(),
                    workflow_run_id: workflow_run_id.clone(),
                    node_id: SchedulerNodeId::parse("prompt").expect("node id"),
                    task_id: SchedulerTaskId::parse("prompt").expect("task id"),
                    node_type: "text_input".to_string(),
                    execution_class: WorkflowSchedulerTaskExecutionClass::SourceInput,
                    dependency_task_ids: Vec::new(),
                    input_bindings: Vec::new(),
                    schedulable_intent: None,
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: None,
                    inference_descriptor_fingerprint: None,
                    runtime_source_context: None,
                    diagnostics: Vec::new(),
                },
                WorkflowSchedulerTask {
                    workflow_id,
                    workflow_run_id,
                    node_id: SchedulerNodeId::parse("image").expect("node id"),
                    task_id: SchedulerTaskId::parse("image").expect("task id"),
                    node_type: "image_generation".to_string(),
                    execution_class: WorkflowSchedulerTaskExecutionClass::RuntimeInference,
                    dependency_task_ids: vec![SchedulerTaskId::parse("prompt").expect("task id")],
                    input_bindings: vec![WorkflowSchedulerTaskInputBinding {
                        source_node_id: SchedulerNodeId::parse("prompt").expect("node id"),
                        source_task_id: SchedulerTaskId::parse("prompt").expect("task id"),
                        source_port_id: "text".to_string(),
                        target_port_id: "prompt".to_string(),
                    }],
                    schedulable_intent: None,
                    schedulable_intent_template: None,
                    non_runtime_task_template: None,
                    source_input_task_template: None,
                    inference_descriptor_fingerprint: None,
                    runtime_source_context: Some(runtime_source_context()),
                    diagnostics: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn scheduler_estimate_reasons_are_bounded_for_diagnostics_ledger() {
        let mut requirements = empty_runtime_requirements();
        requirements.required_models = vec![format!(
            "llm/vendor/{}",
            "very-long-model-segment-".repeat(12)
        )];

        let mut reasons = Vec::new();
        append_scheduler_estimate_runtime_reasons(&mut reasons, &requirements);

        assert_eq!(reasons.len(), 1);
        assert!(reasons[0].len() <= SCHEDULER_ESTIMATE_REASON_MAX_LEN);
        assert!(reasons[0].ends_with(SCHEDULER_ESTIMATE_REASON_TRUNCATION_SUFFIX));
    }

    #[test]
    fn scheduler_estimate_reason_truncation_preserves_utf8_boundaries() {
        let reason = format!("requires model(s): {}", "模型".repeat(128));

        let truncated = truncate_scheduler_estimate_reason(reason);

        assert!(truncated.len() <= SCHEDULER_ESTIMATE_REASON_MAX_LEN);
        assert!(truncated.ends_with(SCHEDULER_ESTIMATE_REASON_TRUNCATION_SUFFIX));
    }

    #[test]
    fn runtime_branch_admission_persists_claimable_task_event() {
        let service = WorkflowService::new();
        let output_targets = Some(vec![super::super::WorkflowOutputTarget {
            node_id: "image-output".to_string(),
            port_id: "image".to_string(),
        }]);
        let persisted = service
            .persist_runtime_branch_task_events_for_admission(
                "session.runtime",
                "workflow.image",
                "run.runtime",
                output_targets.clone(),
                Some(30_000),
                &runtime_task_graph(),
            )
            .expect("runtime branch events persist");

        assert_eq!(persisted, 1);
        let event_id =
            WorkflowRuntimeBranchTaskEventId::parse("runtime-branch-task-event.run.runtime.image")
                .expect("event id");
        let record = service
            .runtime_branch_task_event_for_test(&event_id)
            .expect("runtime branch event record");
        assert_eq!(record.session_id, "session.runtime");
        assert_eq!(record.workflow_id, "workflow.image");
        assert_eq!(record.workflow_run_id, "run.runtime");
        assert_eq!(record.scheduler_task_id, "image");
        assert_eq!(record.scheduler_task_attempt_id, None);
        assert_eq!(record.attempt_generation, 1);
        assert_eq!(record.queued_input_keys, vec!["prompt:prompt".to_string()]);
        assert_eq!(record.output_targets, output_targets);
        assert_eq!(record.timeout_ms, Some(30_000));
        assert_eq!(
            record.batching_key.as_deref(),
            Some("runtime-branch-task.workflow.image.image")
        );
    }

    #[test]
    fn runtime_branch_admission_rejects_duplicate_claimable_task_event() {
        let service = WorkflowService::new();
        service
            .persist_runtime_branch_task_events_for_admission(
                "session.runtime",
                "workflow.image",
                "run.runtime",
                None,
                None,
                &runtime_task_graph(),
            )
            .expect("runtime branch events persist");

        let error = service
            .persist_runtime_branch_task_events_for_admission(
                "session.runtime",
                "workflow.image",
                "run.runtime",
                None,
                None,
                &runtime_task_graph(),
            )
            .expect_err("duplicate runtime branch event fails");

        assert!(error
            .message()
            .contains("runtime branch task-event admission failed"));
        assert!(error.message().contains("DuplicateEvent"));
    }

    #[test]
    fn bootstrap_recovery_plan_blocks_ready_runtime_redispatch_without_recovery_state() {
        let report = WorkflowExecutionSessionBootstrapRecoveryReport {
            active_runs: vec![WorkflowExecutionSessionBootstrapRecoveryRun {
                session_id: "session-a".to_string(),
                workflow_run_id: "run-a".to_string(),
                runtime_tasks: vec![WorkflowExecutionSessionBootstrapRecoveryTask {
                    task_id: "infer".to_string(),
                    state_kind: Some(pantograph_scheduler::SchedulerTaskStateKind::Ready),
                    action: WorkflowExecutionSessionBootstrapRecoveryAction::RedispatchReadyRuntime,
                    runtime_dispatch_recovery_state_available: false,
                }],
            }],
        };

        let plan = workflow_bootstrap_recovery_plan_from_report(report);

        assert!(plan.resume_requests.is_empty());
        assert_eq!(plan.blocking_decision_count, 1);
        assert_eq!(plan.decisions.len(), 1);
        assert_eq!(
            plan.decisions[0].decision_kind,
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::BlockedRuntimeRedispatchRecoveryStateRequired
        );
        assert!(plan.decisions[0]
            .diagnostic
            .as_deref()
            .expect("blocking diagnostic")
            .contains("persisted readiness proof"));
        assert!(plan.decisions[0]
            .diagnostic
            .as_deref()
            .expect("blocking diagnostic")
            .contains("duplicate-dispatch guard"));
    }

    #[test]
    fn bootstrap_recovery_plan_accepts_ready_redispatch_with_recovery_state() {
        let report = WorkflowExecutionSessionBootstrapRecoveryReport {
            active_runs: vec![WorkflowExecutionSessionBootstrapRecoveryRun {
                session_id: "session-a".to_string(),
                workflow_run_id: "run-a".to_string(),
                runtime_tasks: vec![WorkflowExecutionSessionBootstrapRecoveryTask {
                    task_id: "infer".to_string(),
                    state_kind: Some(pantograph_scheduler::SchedulerTaskStateKind::Ready),
                    action: WorkflowExecutionSessionBootstrapRecoveryAction::RedispatchReadyRuntime,
                    runtime_dispatch_recovery_state_available: true,
                }],
            }],
        };

        let plan = workflow_bootstrap_recovery_plan_from_report(report);

        assert_eq!(plan.blocking_decision_count, 0);
        assert!(plan.resume_requests.is_empty());
        assert_eq!(plan.decisions.len(), 1);
        assert!(plan.decisions[0].runtime_dispatch_recovery_state_available);
        assert_eq!(
            plan.decisions[0].decision_kind,
            WorkflowExecutionSessionBootstrapRecoveryDecisionKind::RedispatchReadyRuntime
        );
        assert!(plan.decisions[0].diagnostic.is_none());
        assert_eq!(
            workflow_bootstrap_recovery_runtime_resume_requests(&plan),
            vec![WorkflowBootstrapRecoveryRuntimeResume {
                request: WorkflowExecutionSessionResumeRequest {
                    session_id: "session-a".to_string(),
                    workflow_run_id: "run-a".to_string(),
                },
                attempt_start_transition: SchedulerTaskAttemptLifecycleTransition::Redispatched,
            }]
        );
    }

    #[test]
    fn bootstrap_recovery_progress_loop_requests_dedupe_by_active_run() {
        let plan = WorkflowExecutionSessionBootstrapRecoveryPlan {
            decisions: vec![
                WorkflowExecutionSessionBootstrapRecoveryDecision {
                    session_id: "session-a".to_string(),
                    workflow_run_id: "run-a".to_string(),
                    task_id: "infer-a".to_string(),
                    state_kind: Some(pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs),
                    recovery_action:
                        WorkflowExecutionSessionBootstrapRecoveryAction::ResumeProgressLoop,
                    runtime_dispatch_recovery_state_available: false,
                    decision_kind:
                        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop,
                    diagnostic: None,
                },
                WorkflowExecutionSessionBootstrapRecoveryDecision {
                    session_id: "session-a".to_string(),
                    workflow_run_id: "run-a".to_string(),
                    task_id: "infer-b".to_string(),
                    state_kind: Some(pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs),
                    recovery_action:
                        WorkflowExecutionSessionBootstrapRecoveryAction::ResumeProgressLoop,
                    runtime_dispatch_recovery_state_available: false,
                    decision_kind:
                        WorkflowExecutionSessionBootstrapRecoveryDecisionKind::ResumeProgressLoop,
                    diagnostic: None,
                },
            ],
            resume_requests: Vec::new(),
            blocking_decision_count: 0,
        };

        workflow_bootstrap_recovery_apply_gate(&plan).expect("progress-loop recovery is supported");

        assert_eq!(
            workflow_bootstrap_recovery_progress_loop_requests(&plan),
            vec![WorkflowExecutionSessionResumeRequest {
                session_id: "session-a".to_string(),
                workflow_run_id: "run-a".to_string(),
            }]
        );
    }

    #[test]
    fn run_terminal_event_includes_diagnostics_ledger_resource_rollup() {
        let service = WorkflowService::new().with_diagnostics_ledger(
            pantograph_diagnostics_ledger::SqliteDiagnosticsLedger::open_in_memory()
                .expect("diagnostics ledger opens"),
        );
        {
            let mut ledger = service
                .diagnostics_ledger_guard()
                .expect("diagnostics ledger guard");
            DiagnosticsLedgerRepository::append_diagnostic_event(
                &mut *ledger,
                sample_inference_resource_observation_event(),
            )
            .expect("inference resource diagnostic appends");
        }

        let session = WorkflowExecutionSessionSummary {
            session_id: "session-a".to_string(),
            workflow_id: "workflow-a".to_string(),
            session_kind: WorkflowExecutionSessionKind::Workflow,
            usage_profile: None,
            attribution: None,
            keep_alive: false,
            state: crate::scheduler::WorkflowExecutionSessionState::Running,
            queued_runs: 0,
            run_count: 1,
        };
        let response = Ok(WorkflowRunResponse {
            workflow_run_id: "run-a".to_string(),
            outputs: Vec::new(),
            timing_ms: 42,
        });

        service
            .record_run_terminal_event_if_configured(
                &session,
                None,
                "run-a",
                Some("1.0.0"),
                &response,
            )
            .expect("terminal event records");

        let terminal_payload = {
            let ledger = service
                .diagnostics_ledger_guard()
                .expect("diagnostics ledger guard");
            let events = DiagnosticsLedgerRepository::diagnostic_events_after(&*ledger, 0, 10)
                .expect("diagnostic events query succeeds");
            let terminal_event = events
                .iter()
                .find(|event| {
                    event.event_kind
                        == pantograph_diagnostics_ledger::DiagnosticEventKind::RunTerminal
                })
                .expect("terminal event exists");
            let payload: DiagnosticEventPayload =
                serde_json::from_str(&terminal_event.payload_json).expect("terminal payload json");
            let DiagnosticEventPayload::RunTerminal(payload) = payload else {
                panic!("expected run terminal payload");
            };
            payload
        };

        assert_eq!(terminal_payload.status, RunTerminalStatus::Completed);
        assert_eq!(terminal_payload.duration_ms, Some(42));
        assert_eq!(
            terminal_payload.resource_observation,
            Some(pantograph_diagnostics_ledger::RunResourceObservation {
                peak_ram_bytes: Some(4_096),
                peak_vram_bytes: Some(8_192),
                memory_failure_kind: Some(
                    pantograph_diagnostics_ledger::RunMemoryFailureKind::OutOfMemory,
                ),
            })
        );
    }

    fn sample_inference_resource_observation_event() -> DiagnosticEventAppendRequest {
        DiagnosticEventAppendRequest {
            source_component: DiagnosticEventSourceComponent::NodeExecution,
            source_instance_id: Some("python-runtime:pytorch:1".to_string()),
            occurred_at_ms: 20,
            workflow_run_id: Some(WorkflowRunId::try_from("run-a".to_string()).unwrap()),
            workflow_id: Some(WorkflowId::try_from("workflow-a".to_string()).unwrap()),
            workflow_version_id: None,
            workflow_semantic_version: Some("1.0.0".to_string()),
            node_id: Some("llm-node".to_string()),
            node_type: Some("llm-inference".to_string()),
            node_version: None,
            runtime_id: Some("pytorch.transformers".to_string()),
            runtime_version: None,
            model_id: Some("pumas://models/tiny-transformers".to_string()),
            model_version: None,
            client_id: None,
            client_session_id: None,
            bucket_id: None,
            scheduler_policy_id: None,
            retention_policy_id: None,
            privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
            retention_class: DiagnosticEventRetentionClass::AuditMetadata,
            payload_ref: None,
            payload: DiagnosticEventPayload::InferenceExecutionDiagnosticObserved(
                pantograph_diagnostics_ledger::InferenceExecutionDiagnosticObservedPayload {
                    request_id: "req-a".to_string(),
                    task_id: "image_generation".to_string(),
                    lifecycle_phase: Some("backend_execution".to_string()),
                    lifecycle_event_kind: Some("completed".to_string()),
                    duration_ms: Some(75),
                    selected_backend_key: Some("pytorch".to_string()),
                    selected_backend_family: Some("pytorch".to_string()),
                    selected_runtime_variant_id: Some("pytorch.cuda".to_string()),
                    selected_device_class: Some("cuda".to_string()),
                    selected_device_id: Some("cuda:0".to_string()),
                    selected_network_node_id: None,
                    resolved_artifact_kind: Some("diffusers_bundle".to_string()),
                    usage: None,
                    cache_handle_id: None,
                    artifact_refs: Vec::new(),
                    resource_observation: Some(
                        pantograph_diagnostics_ledger::InferenceResourceObservationDiagnosticSummary {
                            peak_ram_bytes: Some(4_096),
                            peak_vram_bytes: Some(8_192),
                            memory_failure_kind: Some(
                                pantograph_diagnostics_ledger::RunMemoryFailureKind::OutOfMemory,
                            ),
                            sources: Vec::new(),
                            availability: Vec::new(),
                        },
                    ),
                    kv_cache: None,
                    runtime_settings: None,
                    compatibility_report: None,
                    compatibility_issue_count: 0,
                    compatibility_issues: Vec::new(),
                    option_support_counts:
                        pantograph_diagnostics_ledger::InferenceOptionSupportCounts::default(),
                    option_diagnostics: Vec::new(),
                },
            ),
        }
    }
}
