use std::{collections::HashMap, time::Duration};

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    IoArtifactObservedPayload, IoArtifactRole, LibraryAssetAccessedPayload, LibraryAssetOperation,
    RunResourceObservationRollupQuery, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload,
    RunStartedPayload, RunTerminalPayload, RunTerminalStatus, SchedulerCandidateSetSummary,
    SchedulerEstimateBlockingCondition, SchedulerEstimateProducedPayload,
    SchedulerExecutionPlanSummary, SchedulerModelCacheState, SchedulerModelLifecycleChangedPayload,
    SchedulerModelLifecycleTransition, SchedulerQueuePlacementPayload,
    SchedulerReservationChangedPayload, SchedulerReservationResourceKind,
    SchedulerReservationTransition, SchedulerRunAdmittedPayload, SchedulerRunDelayedPayload,
    SchedulerSelectionDecisionCode, SchedulerSelectionHistoryThresholdState,
    SchedulerSelectionPolicyPhase, SchedulerSelectionPolicyTrace,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttributionResolveRequest,
    WorkflowRunId, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
};
use pantograph_scheduler::SchedulerTaskStateKind;
use pantograph_timing_contracts::{checked_timing_duration_ms, WorkflowTimingAttemptId};

use crate::graph::{
    workflow_executable_topology, workflow_graph_run_settings, workflow_graph_run_settings_json,
    WorkflowExecutionSessionKind, WorkflowGraph, WorkflowGraphRunSettings,
};
use crate::scheduler::{unix_timestamp_ms, WORKFLOW_SESSION_QUEUE_POLL_MS};
use crate::technical_fit::{
    WorkflowTechnicalFitDecision, WorkflowTechnicalFitDecisionCode,
    WorkflowTechnicalFitHistoryThresholdState, WorkflowTechnicalFitOverride,
    WorkflowTechnicalFitPolicyPhase, WorkflowTechnicalFitResourceEstimateKind,
    WorkflowTechnicalFitResourceEstimateState,
};

use super::diagnostic_errors::{
    WorkflowDiagnosticErrorRecordRequest, WorkflowDiagnosticRunContext, WorkflowDiagnosticRunScope,
    WorkflowDiagnosticRuntimeModelScope, WorkflowDiagnosticSchedulerScope,
};
use super::io_contract::validate_workflow_io;
use super::session_io_artifacts::workflow_io_artifact_metadata;
use super::session_runtime::WorkflowSessionRuntimeAdmissionDiagnosticContext;
use super::session_runtime_load_lifecycle::{
    WorkflowRuntimeLoadLifecycleContext, WorkflowRuntimeLoadLifecycleEvent,
};
use super::validation::{
    validate_bindings, validate_host_output_bindings, validate_output_targets,
    validate_output_targets_against_io, validate_requested_outputs_produced, validate_timeout_ms,
    validate_workflow_graph_submit_readiness, validate_workflow_id,
    validate_workflow_semantic_version,
};
use super::{
    build_workflow_execution_plan_from_admission, project_scheduler_task_results_to_outputs,
    workflow_scheduler_task_graph, workflow_scheduler_task_run_summary, AttributionRepository,
    WorkflowCapabilityModel, WorkflowErrorDiagnosticsLink, WorkflowExecutionPlan,
    WorkflowExecutionSessionAttributedCreateRequest, WorkflowExecutionSessionAttributionContext,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionCreateResponse,
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRunRequest, WorkflowExecutionSessionSummary,
    WorkflowExecutionSessionUnloadReason, WorkflowHost, WorkflowOutputTarget, WorkflowPortBinding,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowRuntimeCapability,
    WorkflowRuntimeDiagnosticPhaseHint, WorkflowRuntimeRequirements,
    WorkflowSchedulerDecisionReason, WorkflowSchedulerTaskGraph, WorkflowSchedulerTaskRunSummary,
    WorkflowService, WorkflowServiceError, WORKFLOW_EXECUTION_PLAN_MAX_POLICY_TRACE_IDS,
};

const WORKFLOW_SESSION_SCHEDULER_POLICY: &str = "priority_then_fifo";
const WORKFLOW_SESSION_RETENTION_KEEP_ALIVE: &str = "keep_alive";
const WORKFLOW_SESSION_RETENTION_EPHEMERAL: &str = "ephemeral";

pub(super) struct SchedulerModelLifecycleEventRequest<'a> {
    pub(super) session: &'a WorkflowExecutionSessionSummary,
    pub(super) snapshot: Option<&'a WorkflowRunSnapshotRecord>,
    pub(super) workflow_run_id: &'a str,
    pub(super) workflow_semantic_version: &'a str,
    pub(super) selected_runtime_id: Option<&'a str>,
    pub(super) selected_runtime_variant_id: Option<&'a str>,
    pub(super) execution_plan_summary: Option<&'a SchedulerExecutionPlanSummary>,
    pub(super) required_backends: &'a [String],
    pub(super) required_models: &'a [String],
    pub(super) transition: SchedulerModelLifecycleTransition,
    pub(super) timing_attempt_id: Option<&'a str>,
    pub(super) reason: Option<&'a str>,
    pub(super) duration_ms: Option<u64>,
    pub(super) error: Option<&'a str>,
    pub(super) canonical_error_event_id: Option<&'a str>,
}

struct SchedulerReservationContext {
    selected_runtime_id: Option<String>,
    selected_runtime_variant_id: Option<String>,
    selected_device_class: Option<String>,
    selected_device_id: Option<String>,
    reserved_model_ids: Vec<String>,
}

fn workflow_execution_plan_diagnostic_summary(
    execution_plan: &WorkflowExecutionPlan,
) -> SchedulerExecutionPlanSummary {
    let mut policy_trace_ids = Vec::new();
    for decision in execution_plan.node_decisions().values() {
        for trace_id in decision.policy_trace_ids() {
            if !policy_trace_ids.contains(trace_id)
                && policy_trace_ids.len() < WORKFLOW_EXECUTION_PLAN_MAX_POLICY_TRACE_IDS
            {
                policy_trace_ids.push(trace_id.clone());
            }
        }
    }

    SchedulerExecutionPlanSummary {
        schema_version: execution_plan.schema_version(),
        node_decision_count: u32::try_from(execution_plan.node_decisions().len())
            .expect("workflow execution plan node decisions are bounded below u32::MAX"),
        policy_trace_ids,
    }
}

impl WorkflowService {
    pub fn workflow_execution_session_active_execution_plan(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<Option<super::WorkflowExecutionPlan>, WorkflowServiceError> {
        let store = self.session_store_guard()?;
        store.active_run_execution_plan(session_id, workflow_run_id)
    }

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

        let scheduler_task_graph = match self
            .scheduler_task_graph_for_session_run(host, &session.workflow_id, &workflow_run_id)
            .await
        {
            Ok(task_graph) => task_graph,
            Err(error) => {
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
        };
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

        let mut runtime_admission_delay_recorded = false;
        let queued_run = loop {
            let session_ready_to_load = {
                let mut store = self.session_store_guard()?;
                if scheduler_task_run_summary.is_non_runtime_only()
                    || scheduler_task_run_summary.has_runtime_inference()
                    || !store.queued_run_is_admission_candidate(&session_id, &workflow_run_id)?
                {
                    None
                } else {
                    Some(store.session_summary(&session_id)?)
                }
            };
            if let Some(session) = session_ready_to_load {
                let retention_hint = if session.keep_alive {
                    WorkflowExecutionSessionRetentionHint::KeepAlive
                } else {
                    WorkflowExecutionSessionRetentionHint::Ephemeral
                };
                if !host
                    .can_load_session_runtime(
                        &session.session_id,
                        &session.workflow_id,
                        session.usage_profile.as_deref(),
                        retention_hint,
                    )
                    .await?
                {
                    if let Ok(mut store) = self.session_store.lock() {
                        let _ = store.set_queue_decision_reason_if_present(
                            &session_id,
                            &workflow_run_id,
                            WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission,
                        );
                    }
                    if !runtime_admission_delay_recorded {
                        let delayed_until_ms = scheduler_delay_until_ms(unix_timestamp_ms())?;
                        self.record_scheduler_delay_event_if_configured(
                            &session,
                            run_snapshot.as_ref(),
                            &workflow_run_id,
                            &request.workflow_semantic_version,
                            WorkflowSchedulerDecisionReason::WaitingForRuntimeAdmission,
                            Some(delayed_until_ms),
                            Some("runtime admission retry scheduled"),
                        )?;
                        runtime_admission_delay_recorded = true;
                    }
                    tokio::time::sleep(Duration::from_millis(WORKFLOW_SESSION_QUEUE_POLL_MS)).await;
                    continue;
                }
            }

            let maybe_queued = {
                let mut store = self.session_store_guard()?;
                store.begin_queued_run(&session_id, &workflow_run_id)?
            };
            if let Some(queued) = maybe_queued {
                break queued;
            }
            tokio::time::sleep(Duration::from_millis(WORKFLOW_SESSION_QUEUE_POLL_MS)).await;
        };
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        let queued_workflow_inputs = queued_run.queued.inputs.clone();
        let queued_graph_run_settings = decode_queued_graph_run_settings(run_snapshot.as_ref())?;
        if let Err(error) = self.set_scheduler_task_state_for_admitted_run(
            scheduler_task_graph.clone(),
            initial_scheduler_task_records,
            &session_id,
            &workflow_run_id,
        ) {
            self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
            let terminal_result = Err(error);
            self.record_run_terminal_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&queued_workflow_semantic_version),
                &terminal_result,
            )?;
            return terminal_result;
        }

        if scheduler_task_run_summary.is_non_runtime_only() {
            self.record_run_started_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &queued_run,
            )?;
            let run_started_at = std::time::Instant::now();
            let run_result = self
                .run_non_runtime_only_scheduler_session(
                    host,
                    &session_id,
                    &workflow_run_id,
                    &queued_run.workflow_id,
                    &queued_run.queued.inputs,
                    queued_run.queued.output_targets.as_deref(),
                    &scheduler_task_run_summary,
                    run_started_at,
                )
                .await;
            let finish_state = {
                let mut store = self.session_store_guard()?;
                store.finish_run(&session_id, &workflow_run_id)?
            };
            if let Err(record_error) = self.record_run_terminal_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&queued_workflow_semantic_version),
                &run_result,
            ) {
                if let Err(error) = run_result {
                    return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                        workflow_run_id: Some(workflow_run_id),
                        diagnostic_event_id: None,
                        diagnostics_unavailable: Some(record_error.message().to_string()),
                    }));
                }
                return Err(record_error);
            }
            if let Ok(response) = run_result.as_ref() {
                self.record_workflow_io_artifact_events_if_configured(
                    &session,
                    run_snapshot.as_ref(),
                    &workflow_run_id,
                    &queued_workflow_semantic_version,
                    &queued_workflow_inputs,
                    &response.outputs,
                )?;
            }
            debug_assert!(!finish_state.unload_runtime);
            return run_result;
        }

        if scheduler_task_run_summary.has_runtime_inference() {
            self.record_run_started_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &queued_run,
            )?;
            let run_result = self.fail_runtime_scheduler_session_not_wired(
                &session_id,
                &workflow_run_id,
                &scheduler_task_run_summary,
            );
            self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
            if let Err(record_error) = self.record_run_terminal_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&queued_workflow_semantic_version),
                &run_result,
            ) {
                if let Err(error) = run_result {
                    return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                        workflow_run_id: Some(workflow_run_id),
                        diagnostic_event_id: None,
                        diagnostics_unavailable: Some(record_error.message().to_string()),
                    }));
                }
                return Err(record_error);
            }
            return run_result;
        }
        let mut reservation_context = scheduler_reservation_context(
            run_snapshot.as_ref(),
            &queued_run.required_backends,
            &queued_run.required_models,
        )?;

        let preflight_cache = match self
            .ensure_session_runtime_preflight(
                host,
                &session_id,
                &queued_run.workflow_id,
                queued_run.queued.override_selection.clone(),
            )
            .await
        {
            Ok(cache) => cache,
            Err(error) => {
                let diagnostic_outcome = self.record_workflow_diagnostic_error_if_configured(
                    WorkflowDiagnosticErrorRecordRequest::runtime_preflight_failed(
                        workflow_runtime_model_error_scope(
                            &session,
                            run_snapshot.as_ref(),
                            &workflow_run_id,
                            &queued_workflow_semantic_version,
                            &[],
                            &[],
                        )?,
                        &error,
                    )
                    .with_source_instance_id("workflow-session-scheduler")
                    .with_cause("runtime admission preflight failed before model load"),
                )?;
                self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
                let terminal_error = error
                    .with_diagnostics(diagnostic_outcome.into_error_link(Some(&workflow_run_id)));
                let terminal_result = Err(terminal_error);
                self.record_run_terminal_event_if_configured(
                    &session,
                    run_snapshot.as_ref(),
                    &workflow_run_id,
                    Some(&queued_workflow_semantic_version),
                    &terminal_result,
                )?;
                return terminal_result;
            }
        };
        let execution_plan = match build_workflow_execution_plan_from_admission(
            &workflow_run_id,
            &queued_run.workflow_id,
            &preflight_cache.capability_models,
            preflight_cache.technical_fit_decision.as_ref(),
        ) {
            Ok(execution_plan) => execution_plan,
            Err(error) => {
                let error = WorkflowServiceError::CapabilityViolation(format!(
                    "workflow execution-plan production failed: {error}"
                ));
                let diagnostic_outcome = self.record_workflow_diagnostic_error_if_configured(
                    WorkflowDiagnosticErrorRecordRequest::runtime_preflight_failed(
                        workflow_runtime_model_error_scope(
                            &session,
                            run_snapshot.as_ref(),
                            &workflow_run_id,
                            &queued_workflow_semantic_version,
                            &preflight_cache.required_backends,
                            &preflight_cache.required_models,
                        )?,
                        &error,
                    )
                    .with_source_instance_id("workflow-session-scheduler")
                    .with_cause("execution-plan production failed after runtime admission"),
                )?;
                self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
                let terminal_error = error
                    .with_diagnostics(diagnostic_outcome.into_error_link(Some(&workflow_run_id)));
                let terminal_result = Err(terminal_error);
                self.record_run_terminal_event_if_configured(
                    &session,
                    run_snapshot.as_ref(),
                    &workflow_run_id,
                    Some(&queued_workflow_semantic_version),
                    &terminal_result,
                )?;
                return terminal_result;
            }
        };
        let execution_plan_summary = execution_plan
            .as_ref()
            .map(workflow_execution_plan_diagnostic_summary);
        if let Some(execution_plan) = execution_plan {
            let mut store = self.session_store_guard()?;
            store.set_active_run_execution_plan(&session_id, &workflow_run_id, execution_plan)?;
        }
        apply_technical_fit_to_reservation_context(
            &mut reservation_context,
            preflight_cache.technical_fit_decision.as_ref(),
        );
        self.record_scheduler_run_admitted_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &queued_run,
            &reservation_context,
            preflight_cache.technical_fit_decision.as_ref(),
            execution_plan_summary.as_ref(),
        )?;
        self.record_scheduler_reservation_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &workflow_run_id,
            &queued_workflow_semantic_version,
            &reservation_context,
            SchedulerReservationTransition::Created,
            Some("local runtime slot admitted"),
        )?;
        self.record_run_started_event_if_configured(&session, run_snapshot.as_ref(), &queued_run)?;
        let required_backends = preflight_cache.required_backends.clone();
        let required_models = preflight_cache.required_models.clone();
        let runtime_load_timing_attempt_id = WorkflowTimingAttemptId::generate();
        let runtime_load_lifecycle_context = WorkflowRuntimeLoadLifecycleContext {
            session: &session,
            snapshot: run_snapshot.as_ref(),
            workflow_run_id: &workflow_run_id,
            workflow_semantic_version: &queued_workflow_semantic_version,
            timing_attempt_id: runtime_load_timing_attempt_id.as_str(),
            selected_runtime_id: reservation_context.selected_runtime_id.as_deref(),
            selected_runtime_variant_id: reservation_context.selected_runtime_variant_id.as_deref(),
            execution_plan_summary: execution_plan_summary.as_ref(),
            required_backends: &required_backends,
            required_models: &required_models,
        };

        let runtime_load_started_at_ms = unix_timestamp_ms();
        self.record_runtime_load_lifecycle_event_if_configured(
            runtime_load_lifecycle_context,
            WorkflowRuntimeLoadLifecycleEvent::Requested,
        )?;
        let runtime_load_result = self
            .ensure_session_runtime_loaded(
                host,
                &session_id,
                Some(WorkflowSessionRuntimeAdmissionDiagnosticContext {
                    session: &session,
                    snapshot: run_snapshot.as_ref(),
                    workflow_run_id: &workflow_run_id,
                    workflow_semantic_version: &queued_workflow_semantic_version,
                }),
            )
            .await;
        let runtime_load_duration_ms = workflow_timing_duration_ms(
            &runtime_load_timing_attempt_id,
            runtime_load_started_at_ms,
            unix_timestamp_ms(),
        )?;
        let runtime_load_result = match runtime_load_result {
            Ok(()) => {
                host.session_runtime_load_proof(&session_id, &session.workflow_id)
                    .await
            }
            Err(error) => Err(error),
        };
        match &runtime_load_result {
            Ok(proof) => {
                self.record_runtime_load_lifecycle_event_if_configured(
                    runtime_load_lifecycle_context,
                    WorkflowRuntimeLoadLifecycleEvent::DependencyResolved {
                        duration_ms: runtime_load_duration_ms,
                    },
                )?;
                if proof
                    .as_ref()
                    .map(|proof| proof.requested_model_active)
                    .unwrap_or(false)
                {
                    self.record_runtime_load_lifecycle_event_if_configured(
                        runtime_load_lifecycle_context,
                        WorkflowRuntimeLoadLifecycleEvent::Completed {
                            duration_ms: runtime_load_duration_ms,
                        },
                    )?;
                }
            }
            Err(_) => {}
        }
        if let Err(error) = runtime_load_result {
            let diagnostic_request = workflow_runtime_load_error_record_request(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                &queued_workflow_semantic_version,
                &required_backends,
                &required_models,
                &error,
            )?;
            let diagnostic_outcome = match self
                .record_workflow_diagnostic_error_if_configured(diagnostic_request)
            {
                Ok(outcome) => outcome,
                Err(record_error) => {
                    self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
                    let terminal_result =
                        Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                            workflow_run_id: Some(workflow_run_id.clone()),
                            diagnostic_event_id: None,
                            diagnostics_unavailable: Some(record_error.message().to_string()),
                        }));
                    let _terminal_record_result = self.record_run_terminal_event_if_configured(
                        &session,
                        run_snapshot.as_ref(),
                        &workflow_run_id,
                        Some(&queued_workflow_semantic_version),
                        &terminal_result,
                    );
                    let _reservation_record_result = self
                        .record_scheduler_reservation_event_if_configured(
                            &session,
                            run_snapshot.as_ref(),
                            &workflow_run_id,
                            &queued_workflow_semantic_version,
                            &reservation_context,
                            SchedulerReservationTransition::Released,
                            Some("runtime load failed after admission"),
                        );
                    return terminal_result;
                }
            };
            let canonical_error_event_id = diagnostic_outcome.event_id.as_deref();
            let error_text = sanitize_diagnostic_error_text(&error.to_string());
            self.record_runtime_load_lifecycle_event_if_configured(
                runtime_load_lifecycle_context,
                WorkflowRuntimeLoadLifecycleEvent::Failed {
                    duration_ms: runtime_load_duration_ms,
                    error: error_text.as_str(),
                    canonical_error_event_id,
                },
            )?;
            self.finish_failed_workflow_run_after_admission(&session_id, &workflow_run_id)?;
            let terminal_error =
                error.with_diagnostics(diagnostic_outcome.into_error_link(Some(&workflow_run_id)));
            let terminal_result = Err(terminal_error);
            self.record_run_terminal_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                Some(&queued_workflow_semantic_version),
                &terminal_result,
            )?;
            self.record_scheduler_reservation_event_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                &queued_workflow_semantic_version,
                &reservation_context,
                SchedulerReservationTransition::Released,
                Some("runtime load failed after admission"),
            )?;
            return terminal_result;
        }

        let run_result = self
            .workflow_run_internal(
                host,
                WorkflowRunRequest {
                    workflow_id: queued_run.workflow_id,
                    workflow_semantic_version: queued_run.queued.workflow_semantic_version,
                    inputs: queued_run.queued.inputs,
                    output_targets: queued_run.queued.output_targets,
                    override_selection: queued_run.queued.override_selection,
                    timeout_ms: queued_run.queued.timeout_ms,
                },
                Some(preflight_cache),
                Some(session_id.clone()),
                Some(queued_run.queued.workflow_run_id.clone()),
                queued_graph_run_settings,
            )
            .await;

        let finish_state = {
            let mut store = self.session_store_guard()?;
            store.finish_run(&session_id, &workflow_run_id)?
        };
        if let Err(record_error) = self.record_run_terminal_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &workflow_run_id,
            Some(&queued_workflow_semantic_version),
            &run_result,
        ) {
            if let Err(error) = run_result {
                return Err(error.with_diagnostics(WorkflowErrorDiagnosticsLink {
                    workflow_run_id: Some(workflow_run_id),
                    diagnostic_event_id: None,
                    diagnostics_unavailable: Some(record_error.message().to_string()),
                }));
            }
            return Err(record_error);
        }
        self.record_scheduler_reservation_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &workflow_run_id,
            &queued_workflow_semantic_version,
            &reservation_context,
            SchedulerReservationTransition::Released,
            Some("workflow run finished"),
        )?;
        if let Ok(response) = run_result.as_ref() {
            self.record_workflow_io_artifact_events_if_configured(
                &session,
                run_snapshot.as_ref(),
                &workflow_run_id,
                &queued_workflow_semantic_version,
                &queued_workflow_inputs,
                &response.outputs,
            )?;
        }
        if finish_state.unload_runtime {
            let runtime_unload_timing_attempt_id = WorkflowTimingAttemptId::generate();
            self.record_scheduler_model_lifecycle_events_if_configured(
                SchedulerModelLifecycleEventRequest {
                    session: &session,
                    snapshot: run_snapshot.as_ref(),
                    workflow_run_id: &workflow_run_id,
                    workflow_semantic_version: &queued_workflow_semantic_version,
                    selected_runtime_id: reservation_context.selected_runtime_id.as_deref(),
                    selected_runtime_variant_id: reservation_context
                        .selected_runtime_variant_id
                        .as_deref(),
                    execution_plan_summary: execution_plan_summary.as_ref(),
                    required_backends: &required_backends,
                    required_models: &required_models,
                    transition: SchedulerModelLifecycleTransition::UnloadScheduled,
                    timing_attempt_id: Some(runtime_unload_timing_attempt_id.as_str()),
                    reason: Some("keep-alive disabled after run completion"),
                    duration_ms: None,
                    error: None,
                    canonical_error_event_id: None,
                },
            )?;
            let runtime_unload_started_at_ms = unix_timestamp_ms();
            self.record_scheduler_model_lifecycle_events_if_configured(
                SchedulerModelLifecycleEventRequest {
                    session: &session,
                    snapshot: run_snapshot.as_ref(),
                    workflow_run_id: &workflow_run_id,
                    workflow_semantic_version: &queued_workflow_semantic_version,
                    selected_runtime_id: reservation_context.selected_runtime_id.as_deref(),
                    selected_runtime_variant_id: reservation_context
                        .selected_runtime_variant_id
                        .as_deref(),
                    execution_plan_summary: execution_plan_summary.as_ref(),
                    required_backends: &required_backends,
                    required_models: &required_models,
                    transition: SchedulerModelLifecycleTransition::UnloadStarted,
                    timing_attempt_id: Some(runtime_unload_timing_attempt_id.as_str()),
                    reason: Some("keep-alive disabled after run completion"),
                    duration_ms: None,
                    error: None,
                    canonical_error_event_id: None,
                },
            )?;
            let runtime_unload_result = host
                .unload_session_runtime(
                    &session_id,
                    &finish_state.workflow_id,
                    WorkflowExecutionSessionUnloadReason::KeepAliveDisabled,
                )
                .await;
            let runtime_unload_duration_ms = workflow_timing_duration_ms(
                &runtime_unload_timing_attempt_id,
                runtime_unload_started_at_ms,
                unix_timestamp_ms(),
            )?;
            match &runtime_unload_result {
                Ok(()) => self.record_scheduler_model_lifecycle_events_if_configured(
                    SchedulerModelLifecycleEventRequest {
                        session: &session,
                        snapshot: run_snapshot.as_ref(),
                        workflow_run_id: &workflow_run_id,
                        workflow_semantic_version: &queued_workflow_semantic_version,
                        selected_runtime_id: reservation_context.selected_runtime_id.as_deref(),
                        selected_runtime_variant_id: reservation_context
                            .selected_runtime_variant_id
                            .as_deref(),
                        execution_plan_summary: execution_plan_summary.as_ref(),
                        required_backends: &required_backends,
                        required_models: &required_models,
                        transition: SchedulerModelLifecycleTransition::UnloadCompleted,
                        timing_attempt_id: Some(runtime_unload_timing_attempt_id.as_str()),
                        reason: Some("keep-alive disabled after run completion"),
                        duration_ms: Some(runtime_unload_duration_ms),
                        error: None,
                        canonical_error_event_id: None,
                    },
                )?,
                Err(error) => {
                    let error_text = sanitize_diagnostic_error_text(&error.to_string());
                    let _diagnostic_result = self
                        .record_scheduler_model_lifecycle_events_if_configured(
                            SchedulerModelLifecycleEventRequest {
                                session: &session,
                                snapshot: run_snapshot.as_ref(),
                                workflow_run_id: &workflow_run_id,
                                workflow_semantic_version: &queued_workflow_semantic_version,
                                selected_runtime_id: reservation_context
                                    .selected_runtime_id
                                    .as_deref(),
                                selected_runtime_variant_id: reservation_context
                                    .selected_runtime_variant_id
                                    .as_deref(),
                                execution_plan_summary: execution_plan_summary.as_ref(),
                                required_backends: &required_backends,
                                required_models: &required_models,
                                transition: SchedulerModelLifecycleTransition::UnloadFailed,
                                timing_attempt_id: Some(runtime_unload_timing_attempt_id.as_str()),
                                reason: Some("keep-alive disabled after run completion"),
                                duration_ms: Some(runtime_unload_duration_ms),
                                error: Some(error_text.as_str()),
                                canonical_error_event_id: None,
                            },
                        );
                }
            }
            runtime_unload_result?;
        }

        run_result
    }

    fn finish_failed_workflow_run_after_admission(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let mut store = self.session_store_guard()?;
        store.finish_run(session_id, workflow_run_id)?;
        Ok(())
    }

    async fn scheduler_task_graph_for_session_run<H: WorkflowHost>(
        &self,
        host: &H,
        workflow_id: &str,
        workflow_run_id: &str,
    ) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
        let graph = host.workflow_graph(workflow_id).await?;
        validate_workflow_graph_submit_readiness(&graph)?;
        let workflow_id = WorkflowId::try_from(workflow_id.to_string())?;
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        workflow_scheduler_task_graph(&workflow_id, &workflow_run_id, &graph)
    }

    fn set_scheduler_task_state_for_admitted_run(
        &self,
        task_graph: WorkflowSchedulerTaskGraph,
        records: Vec<pantograph_scheduler::SchedulerTaskStateRecord>,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let mut store = self.session_store_guard()?;
        store.set_active_run_scheduler_task_state(session_id, workflow_run_id, task_graph, records)
    }

    async fn run_non_runtime_only_scheduler_session<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_run_id: &str,
        workflow_id: &str,
        inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        summary: &WorkflowSchedulerTaskRunSummary,
        started_at: std::time::Instant,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if !summary.is_non_runtime_only() || summary.has_runtime_inference() {
            return Err(WorkflowServiceError::Internal(
                "scheduler session runner received a runtime-containing run".to_string(),
            ));
        }
        {
            let mut store = self.session_store_guard()?;
            self.scheduler_task_orchestrator
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
        }

        let mut progressed = true;
        while progressed {
            progressed = false;
            let (task_graph, records) =
                self.active_run_scheduler_task_state_required(session_id, workflow_run_id)?;
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
                if task.execution_class
                    != super::WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine
                {
                    continue;
                }
                let advanced = {
                    let mut store = self.session_store_guard()?;
                    self.scheduler_task_orchestrator
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
                };
                progressed |= advanced.is_some();
            }

            let (_task_graph, records) =
                self.active_run_scheduler_task_state_required(session_id, workflow_run_id)?;
            let ready_task_ids = records
                .iter()
                .filter(|record| record.state.kind() == SchedulerTaskStateKind::Ready)
                .map(|record| record.task_id.as_str().to_string())
                .collect::<Vec<_>>();
            for task_id in ready_task_ids {
                let started = {
                    let mut store = self.session_store_guard()?;
                    self.scheduler_task_orchestrator
                        .start_ready_non_runtime_task(
                            &mut store,
                            session_id,
                            workflow_run_id,
                            &task_id,
                        )
                        .map_err(|error| {
                            WorkflowServiceError::InvalidRequest(format!(
                                "scheduler non-runtime task start failed: {error}"
                            ))
                        })?
                };
                let execution_result = self
                    .scheduler_task_orchestrator
                    .execute_started_non_runtime_task(&started)
                    .await;
                match execution_result {
                    Ok(result) => {
                        let mut store = self.session_store_guard()?;
                        self.scheduler_task_orchestrator
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
                        let mut store = self.session_store_guard()?;
                        let _ = self.scheduler_task_orchestrator.fail_started_non_runtime_task(
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
        }

        let (task_graph, records) =
            self.active_run_scheduler_task_state_required(session_id, workflow_run_id)?;
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
        let results = {
            let mut store = self.session_store_guard()?;
            store.active_run_scheduler_task_results(session_id, workflow_run_id)?
        };
        let targets = self
            .scheduler_output_targets_for_run(host, workflow_id, output_targets)
            .await?;
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

    fn fail_runtime_scheduler_session_not_wired(
        &self,
        session_id: &str,
        workflow_run_id: &str,
        summary: &WorkflowSchedulerTaskRunSummary,
    ) -> Result<WorkflowRunResponse, WorkflowServiceError> {
        if !summary.has_runtime_inference() {
            return Err(WorkflowServiceError::Internal(
                "scheduler runtime fail-closed path received a run without runtime inference"
                    .to_string(),
            ));
        }
        {
            let mut store = self.session_store_guard()?;
            self.scheduler_task_orchestrator
                .fail_runtime_dispatch_not_wired_for_active_run(
                    &mut store,
                    session_id,
                    workflow_run_id,
                )
                .map_err(|error| {
                    WorkflowServiceError::InvalidRequest(format!(
                        "scheduler runtime dispatch fail-closed transition failed: {error}"
                    ))
                })?;
        }
        Err(WorkflowServiceError::CapabilityViolation(format!(
            "runtime scheduler dispatch is not wired for {count} runtime inference task(s); runtime tasks must execute only through dispatch-selected scheduler runtime-host handoff",
            count = summary.runtime_inference_tasks
        )))
    }

    fn active_run_scheduler_task_state_required(
        &self,
        session_id: &str,
        workflow_run_id: &str,
    ) -> Result<
        (
            WorkflowSchedulerTaskGraph,
            Vec<pantograph_scheduler::SchedulerTaskStateRecord>,
        ),
        WorkflowServiceError,
    > {
        let store = self.session_store_guard()?;
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
        &self,
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

    async fn create_queued_run_snapshot_if_configured<H: WorkflowHost>(
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

    #[allow(clippy::too_many_arguments)]
    fn record_scheduler_delay_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        workflow_semantic_version: &str,
        reason: WorkflowSchedulerDecisionReason,
        delayed_until_ms: Option<u64>,
        fairness_context: Option<&str>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = unix_timestamp_ms() as i64;
        let delayed_until_ms =
            delayed_until_ms.map(|value| i64::try_from(value).unwrap_or(i64::MAX));

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
                        .unwrap_or_else(|| workflow_semantic_version.to_string()),
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
                payload: DiagnosticEventPayload::SchedulerRunDelayed(SchedulerRunDelayedPayload {
                    reason: reason.as_str().to_string(),
                    delayed_until_ms,
                    fairness_context: fairness_context.map(str::to_string),
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn record_run_started_event_if_configured(
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

    pub(super) fn record_scheduler_model_lifecycle_events_if_configured(
        &self,
        request: SchedulerModelLifecycleEventRequest<'_>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        if request.required_models.is_empty() {
            return Ok(());
        }
        let workflow_run_id = WorkflowRunId::try_from(request.workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(request.session, request.snapshot)?;
        let occurred_at_ms = unix_timestamp_ms() as i64;
        let runtime_id = request
            .selected_runtime_id
            .map(str::to_string)
            .or_else(|| request.required_backends.first().cloned());

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        for model_id in request.required_models {
            self.append_diagnostic_event_and_request_projection_refresh(
                &mut *ledger,
                DiagnosticEventAppendRequest {
                    source_component: DiagnosticEventSourceComponent::Scheduler,
                    source_instance_id: Some("workflow-session-scheduler".to_string()),
                    occurred_at_ms,
                    workflow_run_id: Some(workflow_run_id.clone()),
                    workflow_id: Some(workflow_id.clone()),
                    workflow_version_id: request
                        .snapshot
                        .map(|snapshot| snapshot.workflow_version_id.clone()),
                    workflow_semantic_version: Some(
                        request
                            .snapshot
                            .map(|snapshot| snapshot.workflow_semantic_version.clone())
                            .unwrap_or_else(|| request.workflow_semantic_version.to_string()),
                    ),
                    node_id: None,
                    node_type: None,
                    node_version: None,
                    runtime_id: runtime_id.clone(),
                    runtime_version: None,
                    model_id: Some(model_id.clone()),
                    model_version: None,
                    client_id: event_client_id(request.session, request.snapshot)?,
                    client_session_id: event_client_session_id(request.session, request.snapshot)?,
                    bucket_id: event_bucket_id(request.session, request.snapshot)?,
                    scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                    retention_policy_id: request
                        .snapshot
                        .map(|snapshot| snapshot.retention_policy.clone()),
                    privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                    retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                    payload_ref: None,
                    payload: DiagnosticEventPayload::SchedulerModelLifecycleChanged(
                        SchedulerModelLifecycleChangedPayload {
                            transition: request.transition,
                            cache_state: Some(SchedulerModelCacheState::for_lifecycle_transition(
                                request.transition,
                            )),
                            execution_plan_summary: request.execution_plan_summary.cloned(),
                            timing_attempt_id: request.timing_attempt_id.map(str::to_string),
                            selected_runtime_variant_id: request
                                .selected_runtime_variant_id
                                .map(str::to_string),
                            reason: request.reason.map(str::to_string),
                            duration_ms: request.duration_ms,
                            error: request.error.map(str::to_string),
                            canonical_error_event_id: request
                                .canonical_error_event_id
                                .map(str::to_string),
                        },
                    ),
                },
            )
            .map_err(WorkflowServiceError::from)?;
        }
        Ok(())
    }

    fn record_scheduler_run_admitted_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        queued_run: &crate::scheduler::WorkflowExecutionSessionDequeuedRun,
        reservation_context: &SchedulerReservationContext,
        technical_fit_decision: Option<&WorkflowTechnicalFitDecision>,
        execution_plan_summary: Option<&SchedulerExecutionPlanSummary>,
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
                runtime_id: reservation_context.selected_runtime_id.clone(),
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
                payload: DiagnosticEventPayload::SchedulerRunAdmitted(
                    SchedulerRunAdmittedPayload {
                        queue_wait_ms,
                        decision_reason: queued_run.scheduler_decision_reason.as_str().to_string(),
                        execution_plan_summary: execution_plan_summary.cloned(),
                        selected_runtime_id: reservation_context.selected_runtime_id.clone(),
                        selected_runtime_variant_id: reservation_context
                            .selected_runtime_variant_id
                            .clone(),
                        selected_backend_key: technical_fit_decision
                            .and_then(|decision| decision.selected_backend_key.clone()),
                        selected_device_class: reservation_context.selected_device_class.clone(),
                        selected_device_id: reservation_context.selected_device_id.clone(),
                        selected_network_node_id: None,
                        reserved_model_ids: reservation_context.reserved_model_ids.clone(),
                        technical_fit_selection_policy_trace: technical_fit_decision
                            .and_then(scheduler_selection_policy_trace),
                    },
                ),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    #[allow(clippy::too_many_arguments)]
    fn record_scheduler_reservation_event_if_configured(
        &self,
        session: &WorkflowExecutionSessionSummary,
        snapshot: Option<&WorkflowRunSnapshotRecord>,
        workflow_run_id: &str,
        workflow_semantic_version: &str,
        reservation_context: &SchedulerReservationContext,
        transition: SchedulerReservationTransition,
        reason: Option<&str>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        self.append_diagnostic_event_and_request_projection_refresh(
            &mut *ledger,
            DiagnosticEventAppendRequest {
                source_component: DiagnosticEventSourceComponent::Scheduler,
                source_instance_id: Some("workflow-session-scheduler".to_string()),
                occurred_at_ms: unix_timestamp_ms() as i64,
                workflow_run_id: Some(workflow_run_id.clone()),
                workflow_id: Some(workflow_id),
                workflow_version_id: snapshot.map(|snapshot| snapshot.workflow_version_id.clone()),
                workflow_semantic_version: Some(
                    snapshot
                        .map(|snapshot| snapshot.workflow_semantic_version.clone())
                        .unwrap_or_else(|| workflow_semantic_version.to_string()),
                ),
                node_id: None,
                node_type: None,
                node_version: None,
                runtime_id: reservation_context.selected_runtime_id.clone(),
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
                payload: DiagnosticEventPayload::SchedulerReservationChanged(
                    SchedulerReservationChangedPayload {
                        transition,
                        reservation_id: scheduler_runtime_slot_reservation_id(&workflow_run_id),
                        resource_kind: SchedulerReservationResourceKind::RuntimeSlot,
                        selected_runtime_id: reservation_context.selected_runtime_id.clone(),
                        selected_runtime_variant_id: reservation_context
                            .selected_runtime_variant_id
                            .clone(),
                        selected_device_class: reservation_context.selected_device_class.clone(),
                        selected_device_id: reservation_context.selected_device_id.clone(),
                        selected_network_node_id: None,
                        reserved_model_ids: reservation_context.reserved_model_ids.clone(),
                        reason: reason.map(str::to_string),
                    },
                ),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn record_workflow_io_artifact_events_if_configured(
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

    fn record_run_terminal_event_if_configured(
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

fn scheduler_delay_until_ms(now_ms: u64) -> Result<u64, WorkflowServiceError> {
    now_ms
        .checked_add(WORKFLOW_SESSION_QUEUE_POLL_MS)
        .ok_or_else(|| {
            WorkflowServiceError::Internal(format!(
                "scheduler admission retry timestamp overflowed: now_ms={now_ms}, poll_ms={WORKFLOW_SESSION_QUEUE_POLL_MS}"
            ))
        })
}

fn workflow_timing_duration_ms(
    attempt_id: &WorkflowTimingAttemptId,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<u64, WorkflowServiceError> {
    checked_timing_duration_ms(attempt_id, started_at_ms, completed_at_ms)
        .map_err(|error| WorkflowServiceError::Internal(error.to_string()))
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

fn workflow_runtime_model_error_scope(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
    workflow_run_id: &str,
    workflow_semantic_version: &str,
    required_backends: &[String],
    required_models: &[String],
) -> Result<WorkflowDiagnosticRuntimeModelScope, WorkflowServiceError> {
    Ok(WorkflowDiagnosticRuntimeModelScope {
        run: workflow_diagnostic_run_context(
            session,
            snapshot,
            workflow_run_id,
            Some(workflow_semantic_version),
        )?,
        runtime_id: required_backends
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown_runtime".to_string()),
        runtime_version: None,
        model_id: required_models.first().cloned(),
        model_version: None,
    })
}

fn workflow_runtime_load_error_record_request(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
    workflow_run_id: &str,
    workflow_semantic_version: &str,
    required_backends: &[String],
    required_models: &[String],
    error: &WorkflowServiceError,
) -> Result<WorkflowDiagnosticErrorRecordRequest, WorkflowServiceError> {
    let scope = workflow_runtime_model_error_scope(
        session,
        snapshot,
        workflow_run_id,
        workflow_semantic_version,
        required_backends,
        required_models,
    )?;
    let request = match error
        .runtime_diagnostic_phase_hint()
        .unwrap_or(WorkflowRuntimeDiagnosticPhaseHint::RuntimeModelLoad)
    {
        WorkflowRuntimeDiagnosticPhaseHint::RuntimeModelLoad => {
            WorkflowDiagnosticErrorRecordRequest::runtime_model_load_failed(scope, error)
        }
        WorkflowRuntimeDiagnosticPhaseHint::RuntimeLaunch => {
            WorkflowDiagnosticErrorRecordRequest::runtime_launch_failed(scope, error)
        }
        WorkflowRuntimeDiagnosticPhaseHint::ModelDependency => {
            WorkflowDiagnosticErrorRecordRequest::model_dependency_failed(scope, error)
        }
        WorkflowRuntimeDiagnosticPhaseHint::ManagedBinary => {
            WorkflowDiagnosticErrorRecordRequest::managed_binary_failed(scope, error)
        }
    };

    Ok(request
        .with_source_instance_id("workflow-session-scheduler")
        .with_cause("runtime admission failed to load required models"))
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

fn scheduler_reservation_context(
    snapshot: Option<&WorkflowRunSnapshotRecord>,
    required_backends: &[String],
    required_models: &[String],
) -> Result<SchedulerReservationContext, WorkflowServiceError> {
    let snapshot_runtime_requirements = snapshot
        .map(workflow_run_snapshot_runtime_requirements)
        .transpose()?;
    let selected_runtime_id = snapshot_runtime_requirements
        .as_ref()
        .and_then(|requirements| requirements.required_backends.first().cloned())
        .or_else(|| required_backends.first().cloned());
    let reserved_model_ids = snapshot_runtime_requirements
        .as_ref()
        .map(|requirements| requirements.required_models.clone())
        .filter(|models| !models.is_empty())
        .unwrap_or_else(|| required_models.to_vec());

    Ok(SchedulerReservationContext {
        selected_runtime_id,
        selected_runtime_variant_id: None,
        selected_device_class: None,
        selected_device_id: None,
        reserved_model_ids,
    })
}

fn apply_technical_fit_to_reservation_context(
    context: &mut SchedulerReservationContext,
    technical_fit_decision: Option<&WorkflowTechnicalFitDecision>,
) {
    if let Some(selected_runtime_id) = technical_fit_decision
        .and_then(|decision| decision.selected_runtime_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        context.selected_runtime_id = Some(selected_runtime_id.to_string());
    }

    if let Some(selected_runtime_variant_id) = technical_fit_decision
        .and_then(|decision| decision.selected_runtime_variant_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        context.selected_runtime_variant_id = Some(selected_runtime_variant_id.to_string());
    }

    if let Some(selected_device_class) =
        technical_fit_decision.and_then(|decision| decision.selected_device_class)
    {
        context.selected_device_class =
            Some(workflow_technical_fit_device_class_key(selected_device_class).to_string());
    }

    if let Some(selected_device_id) = technical_fit_decision
        .and_then(|decision| decision.selected_device_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        context.selected_device_id = Some(selected_device_id.to_string());
    }
}

fn workflow_technical_fit_device_class_key(
    device_class: crate::technical_fit::WorkflowTechnicalFitDeviceClass,
) -> &'static str {
    match device_class {
        crate::technical_fit::WorkflowTechnicalFitDeviceClass::Cpu => "cpu",
        crate::technical_fit::WorkflowTechnicalFitDeviceClass::Cuda => "cuda",
        crate::technical_fit::WorkflowTechnicalFitDeviceClass::Metal => "metal",
        crate::technical_fit::WorkflowTechnicalFitDeviceClass::Mps => "mps",
    }
}

fn scheduler_selection_policy_trace(
    decision: &WorkflowTechnicalFitDecision,
) -> Option<SchedulerSelectionPolicyTrace> {
    let trace = decision.selection_policy_trace.as_ref()?;
    Some(SchedulerSelectionPolicyTrace {
        policy_version: trace.policy_version,
        policy_phase: trace.policy_phase.map(scheduler_selection_policy_phase),
        decision_code: trace.decision_code.map(scheduler_selection_decision_code),
        history_threshold_state: trace
            .history_threshold_state
            .map(scheduler_selection_history_threshold_state),
        candidate_set_summary: trace.candidate_set_summary.as_ref().map(|summary| {
            SchedulerCandidateSetSummary {
                total_candidate_count: summary.total_candidate_count,
                eligible_candidate_count: summary.eligible_candidate_count,
                rejected_candidate_count: summary.rejected_candidate_count,
                eligible_candidate_ids: summary.eligible_candidate_ids.clone(),
            }
        }),
        ranking_reason: trace.ranking_reason.clone(),
        exploration_reason: trace.exploration_reason.clone(),
        seed_basis: trace.seed_basis.clone(),
    })
}

fn scheduler_selection_policy_phase(
    phase: WorkflowTechnicalFitPolicyPhase,
) -> SchedulerSelectionPolicyPhase {
    match phase {
        WorkflowTechnicalFitPolicyPhase::CandidateRanking => {
            SchedulerSelectionPolicyPhase::CandidateRanking
        }
    }
}

fn scheduler_selection_decision_code(
    code: WorkflowTechnicalFitDecisionCode,
) -> SchedulerSelectionDecisionCode {
    match code {
        WorkflowTechnicalFitDecisionCode::SelectedCandidate => {
            SchedulerSelectionDecisionCode::SelectedCandidate
        }
    }
}

fn scheduler_selection_history_threshold_state(
    state: WorkflowTechnicalFitHistoryThresholdState,
) -> SchedulerSelectionHistoryThresholdState {
    match state {
        WorkflowTechnicalFitHistoryThresholdState::NotEvaluated => {
            SchedulerSelectionHistoryThresholdState::NotEvaluated
        }
        WorkflowTechnicalFitHistoryThresholdState::InsufficientSamples => {
            SchedulerSelectionHistoryThresholdState::InsufficientSamples
        }
        WorkflowTechnicalFitHistoryThresholdState::Evaluated => {
            SchedulerSelectionHistoryThresholdState::Evaluated
        }
    }
}

fn scheduler_runtime_slot_reservation_id(workflow_run_id: &WorkflowRunId) -> String {
    format!("reservation_{}", workflow_run_id.as_str())
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

fn decode_queued_graph_run_settings(
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<WorkflowGraphRunSettings>, WorkflowServiceError> {
    snapshot
        .map(|snapshot| {
            serde_json::from_str(&snapshot.graph_settings_json).map_err(|error| {
                WorkflowServiceError::CapabilityViolation(format!(
                    "failed to decode workflow run snapshot graph settings: {error}"
                ))
            })
        })
        .transpose()
}

fn workflow_execution_session_kind_label(kind: &WorkflowExecutionSessionKind) -> &'static str {
    match kind {
        WorkflowExecutionSessionKind::Edit => "edit",
        WorkflowExecutionSessionKind::Workflow => "workflow",
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_runtime_requirements() -> WorkflowRuntimeRequirements {
        WorkflowRuntimeRequirements {
            resource_estimates: Vec::new(),
            required_models: Vec::new(),
            required_backends: Vec::new(),
            required_extensions: Vec::new(),
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
    fn scheduler_delay_until_rejects_timestamp_overflow() {
        let error = scheduler_delay_until_ms(u64::MAX)
            .expect_err("scheduler retry timestamp overflow should fail");

        assert!(matches!(
            error,
            WorkflowServiceError::Internal(message)
                if message.contains("scheduler admission retry timestamp overflowed")
        ));
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
