use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    SchedulerModelLifecycleChangedPayload, SchedulerModelLifecycleTransition,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunId, WorkflowRunSnapshotRecord,
};

use crate::scheduler::WorkflowExecutionSessionPreflightCache;
use crate::technical_fit::WorkflowTechnicalFitOverride;

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

pub(super) struct WorkflowSessionRuntimeAdmissionDiagnosticContext<'a> {
    pub(super) session: &'a WorkflowExecutionSessionSummary,
    pub(super) snapshot: Option<&'a WorkflowRunSnapshotRecord>,
    pub(super) workflow_run_id: &'a str,
    pub(super) workflow_semantic_version: &'a str,
}

struct CapacityRebalanceModelLifecycleEventRequest<'a> {
    context: &'a WorkflowSessionRuntimeAdmissionDiagnosticContext<'a>,
    candidate: &'a WorkflowExecutionSessionRuntimeUnloadCandidate,
    transition: SchedulerModelLifecycleTransition,
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
                    if let Some(context) = diagnostics_context.as_ref() {
                        self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                            CapacityRebalanceModelLifecycleEventRequest {
                                context,
                                candidate: &candidate,
                                transition: SchedulerModelLifecycleTransition::UnloadScheduled,
                                reason: "capacity rebalance selected loaded session",
                                duration_ms: None,
                                error: None,
                            },
                        )?;
                        self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                            CapacityRebalanceModelLifecycleEventRequest {
                                context,
                                candidate: &candidate,
                                transition: SchedulerModelLifecycleTransition::UnloadStarted,
                                reason: "capacity rebalance unloading selected session",
                                duration_ms: None,
                                error: None,
                            },
                        )?;
                    }
                    let unload_started_at_ms = crate::scheduler::unix_timestamp_ms();
                    let unload_result = host
                        .unload_session_runtime(
                            &candidate.session_id,
                            &candidate.workflow_id,
                            WorkflowExecutionSessionUnloadReason::CapacityRebalance,
                        )
                        .await;
                    let unload_duration_ms =
                        crate::scheduler::unix_timestamp_ms().saturating_sub(unload_started_at_ms);
                    if let Some(context) = diagnostics_context.as_ref() {
                        match &unload_result {
                            Ok(()) => {
                                self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                                    CapacityRebalanceModelLifecycleEventRequest {
                                        context,
                                        candidate: &candidate,
                                        transition:
                                            SchedulerModelLifecycleTransition::UnloadCompleted,
                                        reason: "capacity rebalance unloaded selected session",
                                        duration_ms: Some(unload_duration_ms),
                                        error: None,
                                    },
                                )?;
                            }
                            Err(error) => {
                                let error_text = error.to_string();
                                self.record_capacity_rebalance_model_lifecycle_events_if_configured(
                                    CapacityRebalanceModelLifecycleEventRequest {
                                        context,
                                        candidate: &candidate,
                                        transition: SchedulerModelLifecycleTransition::UnloadFailed,
                                        reason: "capacity rebalance failed to unload selected session",
                                        duration_ms: Some(unload_duration_ms),
                                        error: Some(error_text.as_str()),
                                    },
                                )?;
                            }
                        }
                    }
                    unload_result?;
                    if let Ok(mut store) = self.session_store.lock() {
                        let _ = store.mark_runtime_loaded(&candidate.session_id, false);
                    }
                }
                RuntimeDecision::LoadTarget {
                    workflow_id,
                    usage_profile,
                    retention_hint,
                } => {
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

        let workflow_run_id = WorkflowRunId::try_from(request.context.workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_runtime_admission_event(
            request.context.session,
            request.context.snapshot,
        )?;
        let runtime_id = request.candidate.required_backends.first().cloned();
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        for model_id in &request.candidate.required_models {
            DiagnosticsLedgerRepository::append_diagnostic_event(
                &mut *ledger,
                DiagnosticEventAppendRequest {
                    source_component: DiagnosticEventSourceComponent::Scheduler,
                    source_instance_id: Some("workflow-session-scheduler".to_string()),
                    occurred_at_ms: crate::scheduler::unix_timestamp_ms() as i64,
                    workflow_run_id: Some(workflow_run_id.clone()),
                    workflow_id: Some(workflow_id.clone()),
                    workflow_version_id: request
                        .context
                        .snapshot
                        .map(|snapshot| snapshot.workflow_version_id.clone()),
                    workflow_semantic_version: Some(
                        request
                            .context
                            .snapshot
                            .map(|snapshot| snapshot.workflow_semantic_version.clone())
                            .unwrap_or_else(|| {
                                request.context.workflow_semantic_version.to_string()
                            }),
                    ),
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
                            transition: request.transition,
                            reason: Some(request.reason.to_string()),
                            duration_ms: request.duration_ms,
                            error: request.error.map(str::to_string),
                        },
                    ),
                },
            )
            .map_err(WorkflowServiceError::from)?;
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
        self.ensure_session_runtime_loaded(host, session_id, None)
            .await
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
