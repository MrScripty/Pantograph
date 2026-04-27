use std::time::Duration;

use pantograph_diagnostics_ledger::{
    DiagnosticEventAppendRequest, DiagnosticEventPayload, DiagnosticEventPrivacyClass,
    DiagnosticEventRetentionClass, DiagnosticEventSourceComponent, DiagnosticsLedgerRepository,
    IoArtifactObservedPayload, IoArtifactRetentionState, LibraryAssetAccessedPayload,
    LibraryAssetOperation, RunSnapshotAcceptedPayload, RunSnapshotNodeVersionPayload,
    RunStartedPayload, RunTerminalPayload, RunTerminalStatus, SchedulerEstimateProducedPayload,
    SchedulerQueuePlacementPayload, SchedulerRunAdmittedPayload, SchedulerRunDelayedPayload,
};
use pantograph_runtime_attribution::{
    BucketId, ClientId, ClientSessionId, WorkflowId, WorkflowRunAttributionResolveRequest,
    WorkflowRunId, WorkflowRunSnapshotRecord, WorkflowRunSnapshotRequest,
};

use crate::graph::{
    workflow_executable_topology, workflow_graph_run_settings, workflow_graph_run_settings_json,
    WorkflowExecutionSessionKind, WorkflowGraph,
};
use crate::scheduler::{unix_timestamp_ms, WORKFLOW_SESSION_QUEUE_POLL_MS};
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::validation::{
    validate_bindings, validate_output_targets, validate_timeout_ms, validate_workflow_id,
    validate_workflow_semantic_version,
};
use super::{
    AttributionRepository, WorkflowCapabilityModel,
    WorkflowExecutionSessionAttributedCreateRequest, WorkflowExecutionSessionAttributionContext,
    WorkflowExecutionSessionCreateRequest, WorkflowExecutionSessionCreateResponse,
    WorkflowExecutionSessionQueueItem, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRunRequest, WorkflowExecutionSessionSummary,
    WorkflowExecutionSessionUnloadReason, WorkflowHost, WorkflowPortBinding, WorkflowRunRequest,
    WorkflowRunResponse, WorkflowSchedulerDecisionReason, WorkflowService, WorkflowServiceError,
};

const WORKFLOW_SESSION_SCHEDULER_POLICY: &str = "priority_then_fifo";
const WORKFLOW_SESSION_RETENTION_KEEP_ALIVE: &str = "keep_alive";
const WORKFLOW_SESSION_RETENTION_EPHEMERAL: &str = "ephemeral";

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

        let mut runtime_admission_delay_recorded = false;
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
                    if !runtime_admission_delay_recorded {
                        let delayed_until_ms = unix_timestamp_ms()
                            .saturating_add(WORKFLOW_SESSION_QUEUE_POLL_MS as u64);
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
        self.record_scheduler_run_admitted_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &queued_run,
        )?;
        self.record_run_started_event_if_configured(&session, run_snapshot.as_ref(), &queued_run)?;
        let queued_workflow_semantic_version = queued_run.queued.workflow_semantic_version.clone();
        let queued_workflow_inputs = queued_run.queued.inputs.clone();

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
        };

        if let Err(error) = self.ensure_session_runtime_loaded(host, &session_id).await {
            if let Ok(mut store) = self.session_store.lock() {
                let _ = store.finish_run(&session_id, &workflow_run_id);
            }
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
        self.record_run_terminal_event_if_configured(
            &session,
            run_snapshot.as_ref(),
            &workflow_run_id,
            Some(&queued_workflow_semantic_version),
            &run_result,
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
            DiagnosticsLedgerRepository::append_diagnostic_event(
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

    fn record_scheduler_run_admitted_event_if_configured(
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
                payload: DiagnosticEventPayload::SchedulerRunAdmitted(
                    SchedulerRunAdmittedPayload {
                        queue_wait_ms,
                        decision_reason: queued_run.scheduler_decision_reason.as_str().to_string(),
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
        let Some(ledger) = self.diagnostics_ledger.as_ref() else {
            return Ok(());
        };
        let workflow_run_id = WorkflowRunId::try_from(workflow_run_id.to_string())?;
        let workflow_id = workflow_id_for_scheduler_event(session, snapshot)?;
        let occurred_at_ms = unix_timestamp_ms() as i64;
        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;

        for (role, binding) in inputs
            .iter()
            .map(|binding| ("workflow_input", binding))
            .chain(outputs.iter().map(|binding| ("workflow_output", binding)))
        {
            let value_json = serde_json::to_vec(&binding.value).map_err(|error| {
                WorkflowServiceError::CapabilityViolation(format!(
                    "failed to encode workflow {role} metadata: {error}"
                ))
            })?;
            DiagnosticsLedgerRepository::append_diagnostic_event(
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
                    privacy_class: DiagnosticEventPrivacyClass::UserMetadata,
                    retention_class: DiagnosticEventRetentionClass::AuditMetadata,
                    payload_ref: None,
                    payload: DiagnosticEventPayload::IoArtifactObserved(
                        IoArtifactObservedPayload {
                            artifact_id: workflow_io_artifact_id(
                                workflow_run_id.as_str(),
                                role,
                                &binding.node_id,
                                &binding.port_id,
                            ),
                            artifact_role: role.to_string(),
                            media_type: Some("application/json".to_string()),
                            size_bytes: Some(value_json.len() as u64),
                            content_hash: Some(format!("blake3:{}", blake3::hash(&value_json))),
                            retention_state: Some(IoArtifactRetentionState::MetadataOnly),
                            retention_reason: Some(
                                "workflow value body is not retained in the I/O artifact ledger"
                                    .to_string(),
                            ),
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
        let (status, duration_ms, error) = match run_result {
            Ok(response) => (
                RunTerminalStatus::Completed,
                Some(response.timing_ms.min(u128::from(u64::MAX)) as u64),
                None,
            ),
            Err(WorkflowServiceError::Cancelled(message)) => {
                (RunTerminalStatus::Cancelled, None, Some(message.clone()))
            }
            Err(error) => (RunTerminalStatus::Failed, None, Some(error.to_string())),
        };

        let mut ledger = ledger.lock().map_err(|_| {
            WorkflowServiceError::Internal("diagnostics ledger lock poisoned".to_string())
        })?;
        DiagnosticsLedgerRepository::append_diagnostic_event(
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
                }),
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

fn workflow_io_artifact_id(
    workflow_run_id: &str,
    artifact_role: &str,
    node_id: &str,
    port_id: &str,
) -> String {
    let hash =
        blake3::hash(format!("{workflow_run_id}:{artifact_role}:{node_id}:{port_id}").as_bytes());
    format!("workflow-io-{hash}")
}

fn pumas_model_asset_id(model_id: &str) -> String {
    format!("pumas://models/{model_id}")
}

fn single_model_node_id(model: &WorkflowCapabilityModel) -> Option<String> {
    (model.node_ids.len() == 1).then(|| model.node_ids[0].clone())
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
