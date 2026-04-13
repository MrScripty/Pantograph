//! Headless workflow API adapter for Tauri transport.
//!
//! This module now acts as a thin transport wrapper over the backend-owned
//! Pantograph embedded runtime.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_embedded_runtime::{
    EmbeddedRuntime, EmbeddedRuntimeConfig, RagBackend, RagDocument,
};
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilitiesResponse, WorkflowIoRequest,
    WorkflowIoResponse, WorkflowPreflightRequest, WorkflowPreflightResponse, WorkflowRunRequest,
    WorkflowRunResponse, WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse,
    WorkflowServiceError, WorkflowSessionCloseRequest, WorkflowSessionCloseResponse,
    WorkflowSessionCreateRequest, WorkflowSessionCreateResponse, WorkflowSessionKeepAliveRequest,
    WorkflowSessionKeepAliveResponse, WorkflowSessionQueueCancelRequest,
    WorkflowSessionQueueCancelResponse, WorkflowSessionQueueListRequest,
    WorkflowSessionQueueListResponse, WorkflowSessionQueueReprioritizeRequest,
    WorkflowSessionQueueReprioritizeResponse, WorkflowSessionRunRequest,
    WorkflowSessionStatusRequest, WorkflowSessionStatusResponse, WorkflowTraceRuntimeMetrics,
    WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
};
use tauri::{AppHandle, Manager, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::SharedGateway;
use crate::project_root::resolve_project_root;

use super::commands::{SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService};
use super::diagnostics::{
    WorkflowDiagnosticsProjection, WorkflowDiagnosticsSnapshotRequest, WorkflowDiagnosticsStore,
};

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

struct TauriRagBackend {
    rag_manager: SharedRagManager,
}

#[async_trait]
impl RagBackend for TauriRagBackend {
    async fn search_as_docs(&self, query: &str, limit: usize) -> Result<Vec<RagDocument>, String> {
        let guard = self.rag_manager.read().await;
        let docs = guard
            .search_as_docs(query, limit)
            .await
            .map_err(|err| err.to_string())?;
        Ok(docs
            .into_iter()
            .map(|doc| RagDocument {
                id: doc.id,
                title: doc.title,
                section: doc.section,
                summary: doc.summary,
                content: doc.content,
            })
            .collect())
    }
}

pub(super) fn build_runtime(
    app: &AppHandle,
    gateway: &SharedGateway,
    extensions: &SharedExtensions,
    workflow_service: &SharedWorkflowService,
    rag_manager: Option<&SharedRagManager>,
) -> Result<EmbeddedRuntime, String> {
    let config = EmbeddedRuntimeConfig::new(app_data_dir(app)?, resolve_project_root()?);
    let rag_backend = rag_manager.cloned().map(|manager| {
        Arc::new(TauriRagBackend {
            rag_manager: manager,
        }) as Arc<dyn RagBackend>
    });
    Ok(EmbeddedRuntime::with_default_python_runtime(
        config,
        gateway.inner_arc(),
        extensions.clone(),
        workflow_service.clone(),
        rag_backend,
    ))
}

fn record_headless_scheduler_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    requested_session_id: &str,
    requested_workflow_id: Option<String>,
    requested_workflow_name: Option<String>,
    snapshot_result: Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>,
    captured_at_ms: u64,
) -> String {
    diagnostics_store.set_execution_metadata(
        requested_session_id,
        requested_workflow_id.clone(),
        requested_workflow_name.clone(),
    );

    match snapshot_result {
        Ok(snapshot) => {
            let observed_execution_id = snapshot
                .trace_execution_id
                .clone()
                .unwrap_or_else(|| requested_session_id.to_string());
            if observed_execution_id != requested_session_id {
                diagnostics_store.set_execution_metadata(
                    &observed_execution_id,
                    snapshot
                        .workflow_id
                        .clone()
                        .or_else(|| requested_workflow_id.clone()),
                    requested_workflow_name,
                );
            }
            diagnostics_store.record_scheduler_snapshot(
                snapshot.workflow_id,
                observed_execution_id.clone(),
                snapshot.session_id,
                captured_at_ms,
                Some(snapshot.session),
                snapshot.items,
                None,
            );
            observed_execution_id
        }
        Err(error) => {
            diagnostics_store.record_scheduler_snapshot(
                requested_workflow_id,
                requested_session_id.to_string(),
                requested_session_id.to_string(),
                captured_at_ms,
                None,
                Vec::new(),
                Some(error.to_envelope_json()),
            );
            requested_session_id.to_string()
        }
    }
}

fn record_headless_runtime_snapshot(
    diagnostics_store: &WorkflowDiagnosticsStore,
    workflow_id: String,
    trace_execution_id: Option<&str>,
    capabilities_result: Result<WorkflowCapabilitiesResponse, WorkflowServiceError>,
    trace_runtime_metrics: WorkflowTraceRuntimeMetrics,
    active_model_target: Option<String>,
    embedding_model_target: Option<String>,
    active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    captured_at_ms: u64,
) {
    match (trace_execution_id, capabilities_result) {
        (Some(trace_execution_id), Ok(capabilities)) => {
            diagnostics_store.record_runtime_snapshot(
                workflow_id,
                trace_execution_id.to_string(),
                captured_at_ms,
                Some(capabilities),
                trace_runtime_metrics,
                active_model_target.clone(),
                embedding_model_target.clone(),
                active_runtime_snapshot.clone(),
                embedding_runtime_snapshot.clone(),
                None,
            );
        }
        (Some(trace_execution_id), Err(error)) => {
            diagnostics_store.record_runtime_snapshot(
                workflow_id,
                trace_execution_id.to_string(),
                captured_at_ms,
                None,
                trace_runtime_metrics,
                active_model_target.clone(),
                embedding_model_target.clone(),
                active_runtime_snapshot.clone(),
                embedding_runtime_snapshot.clone(),
                Some(error.to_envelope_json()),
            );
        }
        (None, Ok(capabilities)) => {
            diagnostics_store.update_runtime_snapshot(
                Some(workflow_id),
                Some(capabilities),
                None,
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot,
                embedding_runtime_snapshot,
                captured_at_ms,
            );
        }
        (None, Err(error)) => {
            diagnostics_store.update_runtime_snapshot(
                Some(workflow_id),
                None,
                Some(error.to_envelope_json()),
                active_model_target,
                embedding_model_target,
                active_runtime_snapshot,
                embedding_runtime_snapshot,
                captured_at_ms,
            );
        }
    }
}

async fn workflow_scheduler_snapshot_response(
    workflow_service: &SharedWorkflowService,
    request: WorkflowSchedulerSnapshotRequest,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_service
        .workflow_get_scheduler_snapshot(request)
        .await
        .map_err(workflow_error_json)
}

fn workflow_trace_snapshot_response(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    request: WorkflowTraceSnapshotRequest,
) -> Result<WorkflowTraceSnapshotResponse, String> {
    diagnostics_store
        .trace_snapshot(request)
        .map_err(workflow_error_json)
}

fn workflow_clear_diagnostics_history_response(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
) -> WorkflowDiagnosticsProjection {
    diagnostics_store.clear_history()
}

fn normalize_optional_request_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn stored_runtime_trace_metrics(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    session_id: Option<&str>,
    workflow_id: Option<&str>,
) -> Option<WorkflowTraceRuntimeMetrics> {
    diagnostics_store
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: session_id.map(ToOwned::to_owned),
            workflow_id: workflow_id.map(ToOwned::to_owned),
            include_completed: Some(true),
        })
        .ok()?
        .traces
        .into_iter()
        .next()
        .map(|trace| trace.runtime)
}

fn workflow_diagnostics_snapshot_projection(
    diagnostics_store: &SharedWorkflowDiagnosticsStore,
    session_id: Option<String>,
    workflow_id: Option<String>,
    workflow_name: Option<String>,
    scheduler_snapshot_result: Option<
        Result<WorkflowSchedulerSnapshotResponse, WorkflowServiceError>,
    >,
    capabilities_result: Option<Result<WorkflowCapabilitiesResponse, WorkflowServiceError>>,
    runtime_trace_metrics: WorkflowTraceRuntimeMetrics,
    active_model_target: Option<String>,
    embedding_model_target: Option<String>,
    active_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
    captured_at_ms: u64,
) -> WorkflowDiagnosticsProjection {
    let mut trace_execution_id = session_id.clone();

    if let Some(session_id) = session_id.as_deref() {
        trace_execution_id = Some(record_headless_scheduler_snapshot(
            diagnostics_store.as_ref(),
            session_id,
            workflow_id.clone(),
            workflow_name.clone(),
            scheduler_snapshot_result.unwrap_or_else(|| {
                Err(WorkflowServiceError::InvalidRequest(
                    "scheduler snapshot unavailable".to_string(),
                ))
            }),
            captured_at_ms,
        ));
    } else {
        diagnostics_store.update_scheduler_snapshot(
            None,
            None,
            None,
            Vec::new(),
            None,
            captured_at_ms,
        );
    }

    if let Some(workflow_id) = workflow_id {
        record_headless_runtime_snapshot(
            diagnostics_store.as_ref(),
            workflow_id,
            trace_execution_id.as_deref(),
            capabilities_result.unwrap_or_else(|| {
                Err(WorkflowServiceError::InvalidRequest(
                    "workflow capabilities unavailable".to_string(),
                ))
            }),
            runtime_trace_metrics,
            active_model_target,
            embedding_model_target,
            active_runtime_snapshot,
            embedding_runtime_snapshot,
            captured_at_ms,
        );
    } else {
        diagnostics_store.update_runtime_snapshot(
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            captured_at_ms,
        );
    }

    diagnostics_store.snapshot()
}

pub async fn workflow_run(
    request: WorkflowRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )?;
    runtime
        .workflow_run(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_get_capabilities(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_io(
    request: WorkflowIoRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_get_io(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_preflight(
    request: WorkflowPreflightRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowPreflightResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_preflight(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_create_session(
    request: WorkflowSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCreateResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .create_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_session(
    request: WorkflowSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )?;
    runtime
        .run_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_session(
    request: WorkflowSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCloseResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .close_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_session_status(
    request: WorkflowSessionStatusRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionStatusResponse, String> {
    workflow_service
        .workflow_get_session_status(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_list_session_queue(
    request: WorkflowSessionQueueListRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueListResponse, String> {
    workflow_service
        .workflow_list_session_queue(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_scheduler_snapshot(
    request: WorkflowSchedulerSnapshotRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSchedulerSnapshotResponse, String> {
    workflow_scheduler_snapshot_response(workflow_service.inner(), request).await
}

pub async fn workflow_cancel_session_queue_item(
    request: WorkflowSessionQueueCancelRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueCancelResponse, String> {
    workflow_service
        .workflow_cancel_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_reprioritize_session_queue_item(
    request: WorkflowSessionQueueReprioritizeRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionQueueReprioritizeResponse, String> {
    workflow_service
        .workflow_reprioritize_session_queue_item(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_set_session_keep_alive(
    request: WorkflowSessionKeepAliveRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionKeepAliveResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )?;
    runtime
        .workflow_set_session_keep_alive(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_diagnostics_snapshot(
    request: WorkflowDiagnosticsSnapshotRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    let captured_at_ms = super::workflow_execution_commands::unix_timestamp_ms();
    let session_id = normalize_optional_request_value(request.session_id.as_deref());
    let workflow_id = normalize_optional_request_value(request.workflow_id.as_deref());
    let workflow_name = normalize_optional_request_value(request.workflow_name.as_deref());
    let scheduler_snapshot_result = if let Some(session_id) = session_id.as_deref() {
        Some(
            workflow_service
                .workflow_get_scheduler_snapshot(WorkflowSchedulerSnapshotRequest {
                    session_id: session_id.to_string(),
                })
                .await,
        )
    } else {
        None
    };
    let capabilities_result = if let Some(workflow_id) = workflow_id.as_ref() {
        let runtime = build_runtime(
            &app,
            gateway.inner(),
            extensions.inner(),
            workflow_service.inner(),
            None,
        )?;
        Some(
            runtime
                .workflow_get_capabilities(WorkflowCapabilitiesRequest {
                    workflow_id: workflow_id.clone(),
                })
                .await,
        )
    } else {
        None
    };
    let runtime_trace_metrics = if let Some(metrics) = stored_runtime_trace_metrics(
        diagnostics_store.inner(),
        session_id.as_deref(),
        workflow_id.as_deref(),
    ) {
        metrics
    } else {
        let runtime_snapshot = gateway.runtime_lifecycle_snapshot().await;
        let mode_info = gateway.mode_info().await;
        super::workflow_execution_commands::trace_runtime_metrics(
            &runtime_snapshot,
            super::workflow_execution_commands::resolve_runtime_model_target(
                &mode_info,
                &runtime_snapshot,
            )
            .as_deref(),
        )
    };
    let mode_info = gateway.mode_info().await;
    let active_runtime_snapshot = Some(gateway.runtime_lifecycle_snapshot().await);
    let embedding_runtime_snapshot = gateway.embedding_runtime_lifecycle_snapshot().await;

    Ok(workflow_diagnostics_snapshot_projection(
        diagnostics_store.inner(),
        session_id,
        workflow_id,
        workflow_name,
        scheduler_snapshot_result,
        capabilities_result,
        runtime_trace_metrics,
        mode_info.active_model_target,
        mode_info.embedding_model_target,
        active_runtime_snapshot,
        embedding_runtime_snapshot,
        captured_at_ms,
    ))
}

pub async fn workflow_get_trace_snapshot(
    request: WorkflowTraceSnapshotRequest,
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowTraceSnapshotResponse, String> {
    workflow_trace_snapshot_response(diagnostics_store.inner(), request)
}

pub async fn workflow_clear_diagnostics_history(
    diagnostics_store: State<'_, SharedWorkflowDiagnosticsStore>,
) -> Result<WorkflowDiagnosticsProjection, String> {
    Ok(workflow_clear_diagnostics_history_response(
        diagnostics_store.inner(),
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        record_headless_runtime_snapshot, record_headless_scheduler_snapshot,
        stored_runtime_trace_metrics, workflow_clear_diagnostics_history_response,
        workflow_diagnostics_snapshot_projection, workflow_scheduler_snapshot_response,
        workflow_trace_snapshot_response,
    };
    use crate::workflow::diagnostics::{
        WorkflowDiagnosticsSnapshotRequest, WorkflowDiagnosticsStore,
    };
    use pantograph_workflow_service::graph::WorkflowSessionKind;
    use pantograph_workflow_service::{
        WorkflowCapabilitiesResponse, WorkflowCapabilityModel, WorkflowGraph,
        WorkflowGraphEditSessionCreateRequest, WorkflowRuntimeRequirements,
        WorkflowSchedulerSnapshotRequest, WorkflowSchedulerSnapshotResponse, WorkflowService,
        WorkflowServiceError, WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus,
        WorkflowSessionState, WorkflowSessionSummary, WorkflowTraceRuntimeMetrics,
        WorkflowTraceSnapshotRequest,
    };

    fn running_session_summary() -> WorkflowSessionSummary {
        WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 2,
        }
    }

    fn capability_response() -> WorkflowCapabilitiesResponse {
        WorkflowCapabilitiesResponse {
            max_input_bindings: 4,
            max_output_targets: 2,
            max_value_bytes: 2_048,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: None,
                estimated_peak_ram_mb: None,
                estimated_min_vram_mb: None,
                estimated_min_ram_mb: None,
                estimation_confidence: "high".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: vec!["kv-cache".to_string()],
            },
            models: vec![WorkflowCapabilityModel {
                model_id: "model-a".to_string(),
                model_revision_or_hash: None,
                model_type: Some("embedding".to_string()),
                node_ids: vec!["node-a".to_string()],
                roles: vec!["embedding".to_string()],
            }],
            runtime_capabilities: Vec::new(),
        }
    }

    #[test]
    fn headless_scheduler_snapshot_helper_uses_trace_execution_identity() {
        let diagnostics_store = WorkflowDiagnosticsStore::default();

        let execution_id = record_headless_scheduler_snapshot(
            &diagnostics_store,
            "session-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            }),
            120,
        );

        assert_eq!(execution_id, "run-1");
        let trace = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            })
            .expect("trace snapshot")
            .traces
            .into_iter()
            .next()
            .expect("scheduler trace");
        assert_eq!(trace.execution_id, "run-1");
        assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow 1"));
        assert_eq!(trace.queue.enqueued_at_ms, Some(100));
        assert_eq!(trace.queue.dequeued_at_ms, Some(110));
    }

    #[test]
    fn headless_scheduler_snapshot_helper_falls_back_to_session_identity_on_error() {
        let diagnostics_store = WorkflowDiagnosticsStore::default();

        let execution_id = record_headless_scheduler_snapshot(
            &diagnostics_store,
            "session-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Err(WorkflowServiceError::InvalidRequest(
                "session missing".to_string(),
            )),
            120,
        );

        assert_eq!(execution_id, "session-1");
        let trace = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("session-1".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            })
            .expect("trace snapshot")
            .traces
            .into_iter()
            .next()
            .expect("scheduler trace");
        assert_eq!(trace.execution_id, "session-1");
        assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(
            trace.queue.scheduler_decision_reason.as_deref(),
            Some("scheduler_snapshot_failed")
        );
    }

    #[test]
    fn headless_runtime_snapshot_helper_records_trace_for_identified_execution() {
        let diagnostics_store = WorkflowDiagnosticsStore::default();

        record_headless_runtime_snapshot(
            &diagnostics_store,
            "wf-1".to_string(),
            Some("run-1"),
            Ok(capability_response()),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:13b".to_string()),
                warmup_started_at_ms: Some(100),
                warmup_completed_at_ms: Some(110),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            Some("llava:13b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        let trace = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            })
            .expect("trace snapshot")
            .traces
            .into_iter()
            .next()
            .expect("runtime trace");
        assert_eq!(trace.execution_id, "run-1");
        assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            trace.runtime.runtime_instance_id.as_deref(),
            Some("runtime-1")
        );
        assert_eq!(trace.runtime.model_target.as_deref(), Some("llava:13b"));
        assert_eq!(
            trace.runtime.lifecycle_decision_reason.as_deref(),
            Some("runtime_ready")
        );
        let projection = diagnostics_store.snapshot();
        assert_eq!(
            projection.runtime.active_model_target.as_deref(),
            Some("llava:13b")
        );
        assert_eq!(
            projection.runtime.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
    }

    #[test]
    fn headless_runtime_snapshot_helper_keeps_trace_store_empty_without_execution_identity() {
        let diagnostics_store = WorkflowDiagnosticsStore::default();

        record_headless_runtime_snapshot(
            &diagnostics_store,
            "wf-1".to_string(),
            None,
            Ok(capability_response()),
            WorkflowTraceRuntimeMetrics::default(),
            Some("llava:7b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        let projection = diagnostics_store.snapshot();
        assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(
            projection.runtime.active_model_target.as_deref(),
            Some("llava:7b")
        );
        assert_eq!(
            projection.runtime.embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
        let trace_snapshot = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: None,
                workflow_id: Some("wf-1".to_string()),
                include_completed: None,
            })
            .expect("trace snapshot");
        assert!(trace_snapshot.traces.is_empty());
    }

    #[test]
    fn headless_scheduler_and_runtime_helpers_join_on_trace_execution_identity() {
        let diagnostics_store = WorkflowDiagnosticsStore::default();

        let execution_id = record_headless_scheduler_snapshot(
            &diagnostics_store,
            "session-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            }),
            120,
        );
        assert_eq!(execution_id, "run-1");

        record_headless_runtime_snapshot(
            &diagnostics_store,
            "wf-1".to_string(),
            Some("run-1"),
            Ok(capability_response()),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:34b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            130,
        );

        let trace = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            })
            .expect("trace snapshot")
            .traces
            .into_iter()
            .next()
            .expect("joined trace");
        assert_eq!(trace.execution_id, "run-1");
        assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow 1"));
        assert_eq!(trace.queue.enqueued_at_ms, Some(100));
        assert_eq!(trace.queue.dequeued_at_ms, Some(110));
        assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
        assert_eq!(
            trace.runtime.runtime_instance_id.as_deref(),
            Some("runtime-1")
        );
        assert_eq!(trace.runtime.model_target.as_deref(), Some("llava:34b"));
        assert_eq!(
            trace.runtime.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
    }

    #[test]
    fn diagnostics_snapshot_request_still_allows_optional_scheduler_context() {
        let request = WorkflowDiagnosticsSnapshotRequest {
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            workflow_name: Some("Workflow 1".to_string()),
        };

        let value = serde_json::to_value(request).expect("serialize diagnostics request");
        assert_eq!(
            value,
            serde_json::json!({
                "session_id": "session-1",
                "workflow_id": "wf-1",
                "workflow_name": "Workflow 1"
            })
        );
    }

    #[tokio::test]
    async fn workflow_scheduler_snapshot_response_reads_backend_owned_service_snapshot() {
        let workflow_service = Arc::new(WorkflowService::new());
        let created = workflow_service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: WorkflowGraph::new(),
            })
            .await
            .expect("create edit session");

        let snapshot = workflow_scheduler_snapshot_response(
            &workflow_service,
            WorkflowSchedulerSnapshotRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .expect("scheduler snapshot");

        assert_eq!(snapshot.session_id, created.session_id);
        assert_eq!(snapshot.workflow_id, None);
        assert_eq!(snapshot.session.session_kind, WorkflowSessionKind::Edit);
        assert_eq!(snapshot.session.state, WorkflowSessionState::IdleLoaded);
        assert!(snapshot.items.is_empty());
    }

    #[test]
    fn workflow_trace_snapshot_response_reads_backend_owned_trace_snapshot() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let execution_id = record_headless_scheduler_snapshot(
            diagnostics_store.as_ref(),
            "session-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            }),
            120,
        );
        assert_eq!(execution_id, "run-1");

        let snapshot = workflow_trace_snapshot_response(
            &diagnostics_store,
            WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            },
        )
        .expect("trace snapshot");

        assert_eq!(snapshot.traces.len(), 1);
        let trace = &snapshot.traces[0];
        assert_eq!(trace.execution_id, "run-1");
        assert_eq!(trace.session_id.as_deref(), Some("session-1"));
        assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow 1"));
        assert_eq!(trace.queue.enqueued_at_ms, Some(100));
        assert_eq!(trace.queue.dequeued_at_ms, Some(110));
    }

    #[test]
    fn workflow_trace_snapshot_response_filters_by_backend_session_id() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        let execution_id = record_headless_scheduler_snapshot(
            diagnostics_store.as_ref(),
            "session-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            }),
            120,
        );
        assert_eq!(execution_id, "run-1");

        let snapshot = workflow_trace_snapshot_response(
            &diagnostics_store,
            WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: None,
                include_completed: None,
            },
        )
        .expect("session-filtered trace snapshot");

        assert_eq!(snapshot.traces.len(), 1);
        let trace = &snapshot.traces[0];
        assert_eq!(trace.execution_id, "run-1");
        assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    }

    #[test]
    fn workflow_trace_snapshot_response_returns_backend_validation_error() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        let error = workflow_trace_snapshot_response(
            &diagnostics_store,
            WorkflowTraceSnapshotRequest {
                execution_id: Some("   ".to_string()),
                session_id: None,
                workflow_id: None,
                include_completed: None,
            },
        )
        .expect_err("blank execution id should be rejected");

        assert!(error.contains("\"code\":\"invalid_request\""));
        assert!(
            error
                .contains("workflow trace snapshot request field 'execution_id' must not be blank")
        );
    }

    #[test]
    fn workflow_diagnostics_snapshot_projection_joins_backend_scheduler_and_runtime_data() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        let projection = workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            Some("session-1".to_string()),
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            })),
            Some(Ok(capability_response())),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:34b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        assert_eq!(projection.run_order, vec!["run-1".to_string()]);
        assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(projection.runtime.max_input_bindings, Some(4));
        assert_eq!(
            projection.scheduler.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            projection.scheduler.trace_execution_id.as_deref(),
            Some("run-1")
        );
        let trace = projection.runs_by_id.get("run-1").expect("joined trace");
        assert_eq!(trace.session_id.as_deref(), Some("session-1"));
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow 1"));
        assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(trace.nodes.len(), 0);
    }

    #[test]
    fn stored_runtime_trace_metrics_prefers_latest_recorded_trace() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            Some("session-1".to_string()),
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![],
            })),
            Some(Ok(capability_response())),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                runtime_instance_id: Some("embed-7".to_string()),
                model_target: Some("/models/embed.gguf".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("reused_embedding_runtime".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        let metrics =
            stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
                .expect("stored trace metrics should exist");

        assert_eq!(metrics.runtime_id.as_deref(), Some("llama.cpp.embedding"));
        assert_eq!(metrics.runtime_instance_id.as_deref(), Some("embed-7"));
        assert_eq!(metrics.model_target.as_deref(), Some("/models/embed.gguf"));
        assert_eq!(metrics.runtime_reused, Some(true));
        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("reused_embedding_runtime")
        );
    }

    #[test]
    fn workflow_diagnostics_snapshot_projection_clears_scheduler_and_runtime_without_context() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
        workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            Some("session-1".to_string()),
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            })),
            Some(Ok(capability_response())),
            WorkflowTraceRuntimeMetrics::default(),
            None,
            None,
            None,
            None,
            120,
        );

        let projection = workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            None,
            None,
            None,
            None,
            None,
            WorkflowTraceRuntimeMetrics::default(),
            None,
            None,
            None,
            None,
            130,
        );

        assert_eq!(projection.runtime.workflow_id, None);
        assert_eq!(projection.scheduler.session_id, None);
        assert_eq!(projection.scheduler.trace_execution_id, None);
        assert_eq!(projection.run_order, vec!["run-1".to_string()]);
    }

    #[test]
    fn workflow_clear_diagnostics_history_response_preserves_backend_snapshots() {
        let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

        workflow_diagnostics_snapshot_projection(
            &diagnostics_store,
            Some("session-1".to_string()),
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
            Some(Ok(WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                trace_execution_id: Some("run-1".to_string()),
                session: running_session_summary(),
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
            })),
            Some(Ok(capability_response())),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:34b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        let projection = workflow_clear_diagnostics_history_response(&diagnostics_store);

        assert!(projection.runs_by_id.is_empty());
        assert!(projection.run_order.is_empty());
        assert_eq!(projection.runtime.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(
            projection.scheduler.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            projection.scheduler.trace_execution_id.as_deref(),
            Some("run-1")
        );
    }
}
