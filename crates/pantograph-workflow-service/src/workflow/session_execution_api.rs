use std::time::Duration;

use pantograph_runtime_attribution::{WorkflowRunId, WorkflowRunSnapshotRequest};

use crate::graph::{
    workflow_graph_run_settings, workflow_graph_run_settings_json, WorkflowExecutionSessionKind,
};
use crate::scheduler::WORKFLOW_SESSION_QUEUE_POLL_MS;
use crate::technical_fit::WorkflowTechnicalFitOverride;

use super::validation::{
    validate_bindings, validate_output_targets, validate_timeout_ms, validate_workflow_id,
    validate_workflow_semantic_version,
};
use super::{
    AttributionRepository, WorkflowExecutionSessionCreateRequest,
    WorkflowExecutionSessionCreateResponse, WorkflowExecutionSessionRetentionHint,
    WorkflowExecutionSessionRunRequest, WorkflowExecutionSessionSummary,
    WorkflowExecutionSessionUnloadReason, WorkflowHost, WorkflowRunRequest, WorkflowRunResponse,
    WorkflowSchedulerDecisionReason, WorkflowService, WorkflowServiceError,
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
        self.create_queued_run_snapshot_if_configured(host, &session, &workflow_run_id, &request)
            .await?;
        {
            let mut store = self.session_store_guard()?;
            store.enqueue_run_with_id(&session_id, &request, workflow_run_id.clone())?
        };

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
    ) -> Result<(), WorkflowServiceError> {
        if self.attribution_store.is_none() {
            return Ok(());
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
        store
            .create_workflow_run_snapshot(snapshot)
            .map_err(WorkflowServiceError::from)?;
        Ok(())
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
