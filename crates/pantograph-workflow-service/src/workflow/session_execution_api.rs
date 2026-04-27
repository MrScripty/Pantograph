use std::time::Duration;

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    RunSnapshotAcceptedPayload, SchedulerEstimateProducedPayload, SchedulerQueuePlacementPayload,
};
use pantograph_runtime_attribution::{
    WorkflowId, WorkflowRunId, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
};

use crate::graph::{
    workflow_graph_run_settings, workflow_graph_run_settings_json, WorkflowExecutionSessionKind,
};
use crate::scheduler::{unix_timestamp_ms, WORKFLOW_SESSION_QUEUE_POLL_MS};
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::validation::{
    validate_bindings, validate_output_targets, validate_timeout_ms, validate_workflow_id,
    validate_workflow_semantic_version,
};
use super::{
    AttributionRepository, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionCreateResponse, WorkflowExecutionSessionQueueItem,
    WorkflowExecutionSessionRetentionHint, WorkflowExecutionSessionRunRequest,
    WorkflowExecutionSessionSummary, WorkflowExecutionSessionUnloadReason, WorkflowHost,
    WorkflowRunRequest, WorkflowRunResponse, WorkflowSchedulerDecisionReason, WorkflowService,
    WorkflowServiceError,
};

const WORKFLOW_SESSION_SCHEDULER_POLICY: &str = "priority_then_fifo";
const WORKFLOW_SESSION_RETENTION_KEEP_ALIVE: &str = "keep_alive";
const WORKFLOW_SESSION_RETENTION_EPHEMERAL: &str = "ephemeral";

impl WorkflowService {
    pub async fn create_workflow_execution_session<H: WorkflowHost>(
        &self,
        host: &H,
        request: WorkflowExecutionSessionCreateRequest,
    ) -> Result<WorkflowExecutionSessionCreateResponse, WorkflowServiceError> {
        validate_workflow_id(&request.workflow_id)?;
        host.validate_workflow(&request.workflow_id).await?;

        let session_id = {
            let mut store = self.session_store_guard()?;
            store.create_session(
                request.workflow_id.clone(),
                request
                    .usage_profile
                    .clone()
                    .map(|v| v.trim().to_string())
                    .filter(|v| !v.is_empty()),
                Vec::new(),
                Vec::new(),
                request.keep_alive,
            )?
        };

        if request.keep_alive {
            if let Err(error) = self
                .ensure_keep_alive_session_runtime_ready(host, &session_id, &request.workflow_id)
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
        let run_snapshot = self
            .create_queued_run_snapshot_if_configured(host, &session, &workflow_run_id, &request)
            .await?;
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
                &request,
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
            return Err(error);
        }

        let queued_run = loop {
            let session_ready_to_load = {
                let mut store = self.session_store_guard()?;
                if !store.queued_run_is_admission_candidate(&session_id, &workflow_run_id)? {
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
                if let Ok(mut store) = self.session_store.lock() {
                    let _ = store.finish_run(&session_id, &workflow_run_id);
                }
                return Err(error);
            }
        };

        if let Err(error) = self.ensure_session_runtime_loaded(host, &session_id).await {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.finish_run(&session_id, &workflow_run_id);
            }
            return Err(error);
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
            )
            .await;

        let finish_state = {
            let mut store = self.session_store_guard()?;
            store.finish_run(&session_id, &workflow_run_id)?
        };
        if finish_state.unload_runtime {
            host.unload_session_runtime(
                &session_id,
                &finish_state.workflow_id,
                WorkflowExecutionSessionUnloadReason::KeepAliveDisabled,
            )
            .await?;
        }

        run_result
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
        self.record_run_snapshot_accepted_event_if_configured(&snapshot)?;
        Ok(Some(snapshot))
    }

    fn record_run_snapshot_accepted_event_if_configured(
        &self,
        snapshot: &WorkflowRunSnapshotRecord,
    ) -> Result<(), WorkflowServiceError> {
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        DiagnosticsLedgerRepository::append_diagnostic_event(
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
                client_id: None,
                client_session_id: None,
                bucket_id: None,
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
                }),
            },
        )
        .map(|_| ())
        .map_err(WorkflowServiceError::from)
    }

    fn record_scheduler_estimate_event_if_configured(
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
        let reason = if queue_position == 0 {
            "next admission candidate pending runtime readiness".to_string()
        } else {
            format!("{queue_position} run(s) ahead in session queue")
        };

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        DiagnosticsLedgerRepository::append_diagnostic_event(
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
                client_id: None,
                client_session_id: None,
                bucket_id: None,
                scheduler_policy_id: Some(WORKFLOW_SESSION_SCHEDULER_POLICY.to_string()),
                retention_policy_id: snapshot.map(|snapshot| snapshot.retention_policy.clone()),
                privacy_class: DiagnosticEventPrivacyClass::SystemMetadata,
                retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                payload_ref: None,
                payload: DiagnosticEventPayload::SchedulerEstimateProduced(
                    SchedulerEstimateProducedPayload {
                        estimate_version: "session-scheduler-v1".to_string(),
                        confidence: "low".to_string(),
                        estimated_queue_wait_ms: None,
                        estimated_duration_ms: None,
                        reasons: vec![reason],
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
        DiagnosticsLedgerRepository::append_diagnostic_event(
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
                client_id: None,
                client_session_id: None,
                bucket_id: None,
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
