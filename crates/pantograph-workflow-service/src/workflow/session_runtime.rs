use pantograph_diagnostics_ledger::{
    sanitize_diagnostic_error_text, DiagnosticEventAppendRequest, DiagnosticEventPayload,
    DiagnosticEventPrivacyClass, DiagnosticEventRetentionClass, DiagnosticEventSourceComponent,
    SchedulerModelCacheState, SchedulerModelLifecycleChangedPayload,
    SchedulerModelLifecycleTransition, MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunSnapshotRecord,
};
use pantograph_timing_contracts::{checked_timing_duration_ms, WorkflowTimingAttemptId};

use crate::scheduler::WorkflowExecutionSessionPreflightCache;
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::diagnostic_errors::{
    WorkflowDiagnosticErrorRecordRequest, WorkflowDiagnosticSessionRuntimeScope,
};
use super::{
    WorkflowExecutionSessionRetentionHint, WorkflowExecutionSessionRuntimeSelectionTarget,
    WorkflowExecutionSessionRuntimeUnloadCandidate, WorkflowExecutionSessionSummary,
    WorkflowExecutionSessionUnloadReason, WorkflowHost, WorkflowRuntimeCapability, WorkflowService,
    WorkflowServiceError,
};

fn compute_runtime_capability_fingerprint(
    runtime_capabilities: &[WorkflowRuntimeCapability],
) -> String {
    let mut normalized = runtime_capabilities.to_vec();
    normalized.sort_by(|a, b| a.runtime_id.cmp(&b.runtime_id));
    for capability in &mut normalized {
        capability.backend_keys.sort();
        capability.missing_files.sort();
    }

    let encoded = serde_json::to_string(&normalized).unwrap_or_default();
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in encoded.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn workflow_timing_duration_ms(
    attempt_id: &WorkflowTimingAttemptId,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<u64, WorkflowServiceError> {
    checked_timing_duration_ms(attempt_id, started_at_ms, completed_at_ms)
        .map_err(|error| WorkflowServiceError::Internal(error.to_string()))
}

pub(super) struct WorkflowSessionRuntimeAdmissionDiagnosticContext<'a> {
    pub(super) session: &'a WorkflowExecutionSessionSummary,
    pub(super) snapshot: Option<&'a WorkflowRunSnapshotRecord>,
}

struct CapacityRebalanceModelLifecycleEventRequest<'a> {
    context: &'a WorkflowSessionRuntimeAdmissionDiagnosticContext<'a>,
    candidate: &'a WorkflowExecutionSessionRuntimeUnloadCandidate,
    transition: SchedulerModelLifecycleTransition,
    timing_attempt_id: Option<&'a str>,
    reason: &'a str,
    duration_ms: Option<u64>,
    error: Option<&'a str>,
}

impl WorkflowService {
    pub fn invalidate_all_session_runtimes(&self) -> Result<Vec<String>, WorkflowServiceError> {
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        Ok(store.invalidate_all_loaded_session_runtimes())
    }

    pub(super) async fn ensure_session_runtime_loaded<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        diagnostics_context: Option<WorkflowSessionRuntimeAdmissionDiagnosticContext<'_>>,
    ) -> Result<(), WorkflowServiceError> {
        let workflow_id = self
            .session_store_guard()?
            .session_summary(session_id)?
            .workflow_id;
        let authored_graph = host.workflow_graph(&workflow_id).await?;
        crate::graph::lower_groups(&authored_graph, &crate::graph::NodeRegistry::new())?;
        enum RuntimeDecision {
            Ready,
            SelectUnloadCandidate {
                target: WorkflowExecutionSessionRuntimeSelectionTarget,
                candidates: Vec<WorkflowExecutionSessionRuntimeUnloadCandidate>,
                loaded_session_count: usize,
                max_loaded_sessions: usize,
            },
            LoadTarget {
                workflow_id: String,
                usage_profile: Option<String>,
                retention_hint: WorkflowExecutionSessionRetentionHint,
            },
        }

        loop {
            let decision = {
                let store = self.session_store.lock().map_err(|_| {
                    WorkflowServiceError::Internal("session store lock poisoned".to_string())
                })?;
                let target = store.active.get(session_id).ok_or_else(|| {
                    WorkflowServiceError::SessionNotFound(format!(
                        "session '{}' not found",
                        session_id
                    ))
                })?;
                if target.runtime_loaded {
                    RuntimeDecision::Ready
                } else if store.loaded_session_count() >= store.max_loaded_sessions {
                    let loaded_session_count = store.loaded_session_count();
                    RuntimeDecision::SelectUnloadCandidate {
                        target: WorkflowExecutionSessionRuntimeSelectionTarget {
                            session_id: session_id.to_string(),
                            workflow_id: target.workflow_id.clone(),
                            usage_profile: target.usage_profile.clone(),
                            required_backends: target.required_backends.clone(),
                            required_models: target.required_models.clone(),
                        },
                        candidates: store.runtime_unload_candidates(session_id),
                        loaded_session_count,
                        max_loaded_sessions: store.max_loaded_sessions,
                    }
                } else {
                    RuntimeDecision::LoadTarget {
                        workflow_id: target.workflow_id.clone(),
                        usage_profile: target.usage_profile.clone(),
                        retention_hint: if target.keep_alive {
                            WorkflowExecutionSessionRetentionHint::KeepAlive
                        } else {
                            WorkflowExecutionSessionRetentionHint::Ephemeral
                        },
                    }
                }
            };

            match decision {
                RuntimeDecision::Ready => return Ok(()),
                RuntimeDecision::SelectUnloadCandidate {
                    target,
                    candidates,
                    loaded_session_count,
                    max_loaded_sessions,
                } => {
                    let Some(candidate) = host
                        .select_runtime_unload_candidate(&target, &candidates)
                        .await?
                    else {
                        return Err(WorkflowServiceError::scheduler_runtime_capacity_exhausted(
                            loaded_session_count,
                            max_loaded_sessions,
                            candidates.len(),
                        ));
                    };
                    let unload_timing_attempt_id = WorkflowTimingAttemptId::generate();
                    if let Some(context) = diagnostics_context.as_ref() {
                        self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                            CapacityRebalanceModelLifecycleEventRequest {
                                context,
                                candidate: &candidate,
                                transition: SchedulerModelLifecycleTransition::UnloadScheduled,
                                timing_attempt_id: Some(unload_timing_attempt_id.as_str()),
                                reason: "capacity rebalance selected loaded session",
                                duration_ms: None,
                                error: None,
                            },
                        )
                        .map_err(lifecycle_diagnostic_failure)?;
                        self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                            CapacityRebalanceModelLifecycleEventRequest {
                                context,
                                candidate: &candidate,
                                transition: SchedulerModelLifecycleTransition::UnloadStarted,
                                timing_attempt_id: Some(unload_timing_attempt_id.as_str()),
                                reason: "capacity rebalance unloading selected session",
                                duration_ms: None,
                                error: None,
                            },
                        )
                        .map_err(lifecycle_diagnostic_failure)?;
                    }
                    let unload_started_at_ms = crate::scheduler::unix_timestamp_ms();
                    let unload_result = host
                        .unload_session_runtime(
                            &candidate.session_id,
                            &candidate.workflow_id,
                            WorkflowExecutionSessionUnloadReason::CapacityRebalance,
                        )
                        .await;
                    // Host residency is authoritative even if subsequent telemetry fails.
                    if unload_result.is_ok() {
                        self.session_store_guard()?
                            .mark_runtime_loaded(&candidate.session_id, false)?;
                    }
                    let unload_duration_ms = workflow_timing_duration_ms(
                        &unload_timing_attempt_id,
                        unload_started_at_ms,
                        crate::scheduler::unix_timestamp_ms(),
                    )?;
                    let terminal_diagnostics = if let Some(context) = diagnostics_context.as_ref() {
                        let error_text = unload_result.as_ref().err().map(|error| {
                            sanitize_diagnostic_error_text(
                                &error.to_string(),
                                MAX_DIAGNOSTIC_ERROR_TEXT_LEN,
                            )
                        });
                        self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                            CapacityRebalanceModelLifecycleEventRequest {
                                context,
                                candidate: &candidate,
                                transition: if unload_result.is_ok() {
                                    SchedulerModelLifecycleTransition::UnloadCompleted
                                } else {
                                    SchedulerModelLifecycleTransition::UnloadFailed
                                },
                                timing_attempt_id: Some(unload_timing_attempt_id.as_str()),
                                reason: if unload_result.is_ok() {
                                    "capacity rebalance unloaded selected session"
                                } else {
                                    "capacity rebalance failed to unload selected session"
                                },
                                duration_ms: Some(unload_duration_ms),
                                error: error_text.as_deref(),
                            },
                        )
                    } else {
                        Ok(())
                    };
                    match unload_result {
                        Ok(()) => terminal_diagnostics.map_err(lifecycle_diagnostic_failure)?,
                        Err(error) => {
                            return Err(match terminal_diagnostics {
                                Ok(()) => error,
                                Err(recording_error) => error
                                    .with_diagnostics(lifecycle_diagnostic_link(&recording_error)),
                            })
                        }
                    }
                }
                RuntimeDecision::LoadTarget {
                    workflow_id,
                    usage_profile,
                    retention_hint,
                } => {
                    // Capacity selection and unloading await host callbacks. Re-read the
                    // authored graph at the target load boundary after those awaits.
                    let authored_graph = host.workflow_graph(&workflow_id).await?;
                    crate::graph::lower_groups(
                        &authored_graph,
                        &crate::graph::NodeRegistry::new(),
                    )?;
                    host.load_session_runtime(
                        session_id,
                        &workflow_id,
                        usage_profile.as_deref(),
                        retention_hint,
                    )
                    .await?;
                    let mut store = self.session_store.lock().map_err(|_| {
                        WorkflowServiceError::Internal("session store lock poisoned".to_string())
                    })?;
                    store.mark_runtime_loaded(session_id, true)?;
                    return Ok(());
                }
            }
        }
    }

    fn record_capacity_rebalance_model_lifecycle_events_if_configured(
        &self,
        request: CapacityRebalanceModelLifecycleEventRequest<'_>,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        if request.candidate.required_models.is_empty() {
            return Ok(());
        }

        let workflow_run_id = request
            .context
            .snapshot
            .map(|snapshot| snapshot.workflow_run_id.clone());
        let workflow_id = workflow_id_for_runtime_admission_event(
            request.context.session,
            request.context.snapshot,
        )?;
        let runtime_id = request.candidate.required_backends.first().cloned();
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        for model_id in &request.candidate.required_models {
            self.append_diagnostic_event_and_request_projection_refresh(
                &mut *ledger,
                DiagnosticEventAppendRequest {
                    source_component: DiagnosticEventSourceComponent::Scheduler,
                    source_instance_id: Some("workflow-session-scheduler".to_string()),
                    occurred_at_ms: crate::scheduler::unix_timestamp_ms() as i64,
                    workflow_run_id: workflow_run_id.clone(),
                    workflow_id: Some(workflow_id.clone()),
                    workflow_version_id: request
                        .context
                        .snapshot
                        .map(|snapshot| snapshot.workflow_version_id.clone()),
                    workflow_semantic_version: request
                        .context
                        .snapshot
                        .map(|snapshot| snapshot.workflow_semantic_version.clone()),
                    node_id: None,
                    node_type: None,
                    node_version: None,
                    runtime_id: runtime_id.clone(),
                    runtime_version: None,
                    model_id: Some(model_id.clone()),
                    model_version: None,
                    client_id: runtime_event_client_id(
                        request.context.session,
                        request.context.snapshot,
                    )?,
                    client_session_id: runtime_event_client_session_id(
                        request.context.session,
                        request.context.snapshot,
                    )?,
                    bucket_id: runtime_event_bucket_id(
                        request.context.session,
                        request.context.snapshot,
                    )?,
                    scheduler_policy_id: Some("priority_then_fifo".to_string()),
                    retention_policy_id: request
                        .context
                        .snapshot
                        .map(|snapshot| snapshot.retention_policy.clone()),
                    privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                    retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                    payload_ref: None,
                    payload: DiagnosticEventPayload::SchedulerModelLifecycleChanged(
                        SchedulerModelLifecycleChangedPayload {
                            workflow_execution_session_id: Some(
                                request.context.session.session_id.clone(),
                            ),
                            unloaded_workflow_execution_session_id: Some(
                                request.candidate.session_id.clone(),
                            ),
                            transition: request.transition,
                            cache_state: Some(SchedulerModelCacheState::for_lifecycle_transition(
                                request.transition,
                            )),
                            execution_plan_summary: None,
                            timing_attempt_id: request.timing_attempt_id.map(str::to_string),
                            selected_runtime_variant_id: None,
                            reason: Some(request.reason.to_string()),
                            duration_ms: request.duration_ms,
                            error: request.error.map(str::to_string),
                            canonical_error_event_id: None,
                        },
                    ),
                },
            )?;
        }
        Ok(())
    }

    pub(super) async fn ensure_session_runtime_preflight<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
        override_selection: Option<WorkflowTechnicalFitOverride>,
    ) -> Result<WorkflowExecutionSessionPreflightCache, WorkflowServiceError> {
        let graph_fingerprint = host.workflow_graph_fingerprint(workflow_id).await?;
        let runtime_capabilities = host.runtime_capabilities().await?;
        let runtime_capability_fingerprint =
            compute_runtime_capability_fingerprint(&runtime_capabilities);

        {
            let store = self.session_store.lock().map_err(|_| {
                WorkflowServiceError::Internal("session store lock poisoned".to_string())
            })?;
            if let Some(cached) = store.cached_preflight(session_id)? {
                if cached.graph_fingerprint == graph_fingerprint
                    && cached.runtime_capability_fingerprint == runtime_capability_fingerprint
                    && cached.override_selection == override_selection
                {
                    return Ok(cached);
                }
            }
        }

        let capabilities = host.workflow_capabilities(workflow_id).await?;
        let runtime_preflight = self
            .workflow_execution_session_runtime_preflight_assessment(
                host,
                session_id,
                &capabilities,
                override_selection.clone(),
            )
            .await?;
        let cache = WorkflowExecutionSessionPreflightCache {
            graph_fingerprint,
            runtime_capability_fingerprint,
            override_selection,
            required_backends: capabilities.runtime_requirements.required_backends.clone(),
            required_models: capabilities.runtime_requirements.required_models.clone(),
            blocking_runtime_issues: runtime_preflight.blocking_runtime_issues,
        };

        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.cache_preflight(session_id, cache.clone())?;
        Ok(cache)
    }

    pub(super) async fn ensure_keep_alive_session_runtime_ready<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        self.refresh_session_runtime_affinity_basis(host, session_id, workflow_id)
            .await?;
        let cache = self
            .ensure_session_runtime_preflight(host, session_id, workflow_id, None)
            .await?;
        if !cache.blocking_runtime_issues.is_empty() {
            return Err(WorkflowServiceError::RuntimeNotReady(
                super::format_runtime_not_ready_message(&cache.blocking_runtime_issues),
            ));
        }
        let session = self.session_store_guard()?.session_summary(session_id)?;
        let result = self
            .ensure_session_runtime_loaded(
                host,
                session_id,
                Some(WorkflowSessionRuntimeAdmissionDiagnosticContext {
                    session: &session,
                    snapshot: None,
                }),
            )
            .await;
        match result {
            Ok(()) => Ok(()),
            Err(error) => {
                let outcome = self.record_workflow_diagnostic_error_if_configured(
                    WorkflowDiagnosticErrorRecordRequest::session_runtime_admission_failed(
                        WorkflowDiagnosticSessionRuntimeScope {
                            session_id: session.session_id.clone(),
                            workflow_id: WorkflowId::try_from(session.workflow_id.clone())?,
                            workflow_run_id: None,
                        },
                        &error,
                    ),
                );
                let prior_unavailable = error
                    .diagnostics()
                    .and_then(|link| link.diagnostics_unavailable.clone());
                let mut diagnostics = match outcome {
                    Ok(outcome) => outcome.into_error_link(None::<String>),
                    Err(recording_error) => super::WorkflowErrorDiagnosticsLink {
                        workflow_run_id: None,
                        diagnostic_event_id: None,
                        diagnostics_unavailable: Some(recording_error.message().to_string()),
                    },
                };
                if let Some(prior) = prior_unavailable {
                    diagnostics.diagnostics_unavailable =
                        Some(match diagnostics.diagnostics_unavailable {
                            Some(current) if current != prior => format!("{prior}; {current}"),
                            _ => prior,
                        });
                }
                Err(error.with_diagnostics(diagnostics))
            }
        }
    }

    pub(super) async fn refresh_session_runtime_affinity_basis<H: WorkflowHost>(
        &self,
        host: &H,
        session_id: &str,
        workflow_id: &str,
    ) -> Result<(), WorkflowServiceError> {
        let capabilities = host.workflow_capabilities(workflow_id).await?;
        let mut store = self.session_store.lock().map_err(|_| {
            WorkflowServiceError::Internal("session store lock poisoned".to_string())
        })?;
        store.update_runtime_affinity_basis(
            session_id,
            capabilities.runtime_requirements.required_backends,
            capabilities.runtime_requirements.required_models,
        )?;
        Ok(())
    }
}

fn workflow_id_for_runtime_admission_event(
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

fn runtime_event_client_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<ClientId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.client_id.clone()) {
        Some(client_id) => Ok(Some(client_id)),
        None => session
            .attribution
            .as_ref()
            .map(|context| ClientId::try_from(context.client_id.clone()))
            .transpose()
            .map_err(WorkflowServiceError::from),
    }
}

fn runtime_event_client_session_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<ClientSessionId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.client_session_id.clone()) {
        Some(client_session_id) => Ok(Some(client_session_id)),
        None => session
            .attribution
            .as_ref()
            .map(|context| ClientSessionId::try_from(context.client_session_id.clone()))
            .transpose()
            .map_err(WorkflowServiceError::from),
    }
}

fn runtime_event_bucket_id(
    session: &WorkflowExecutionSessionSummary,
    snapshot: Option<&WorkflowRunSnapshotRecord>,
) -> Result<Option<BucketId>, WorkflowServiceError> {
    match snapshot.and_then(|snapshot| snapshot.bucket_id.clone()) {
        Some(bucket_id) => Ok(Some(bucket_id)),
        None => session
            .attribution
            .as_ref()
            .map(|context| BucketId::try_from(context.bucket_id.clone()))
            .transpose()
            .map_err(WorkflowServiceError::from),
    }
}

fn lifecycle_diagnostic_link(error: &WorkflowServiceError) -> super::WorkflowErrorDiagnosticsLink {
    super::WorkflowErrorDiagnosticsLink {
        workflow_run_id: None,
        diagnostic_event_id: None,
        diagnostics_unavailable: Some(format!(
            "capacity lifecycle event unavailable: {}",
            sanitize_diagnostic_error_text(error.message(), MAX_DIAGNOSTIC_ERROR_TEXT_LEN)
        )),
    }
}

fn lifecycle_diagnostic_failure(error: WorkflowServiceError) -> WorkflowServiceError {
    let link = lifecycle_diagnostic_link(&error);
    error.with_diagnostics(link)
}
