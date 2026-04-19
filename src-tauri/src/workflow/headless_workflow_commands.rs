//! Headless workflow API adapter for Tauri transport.
//!
//! This module now acts as a thin transport wrapper over the backend-owned
//! Pantograph embedded runtime.

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
    WorkflowSessionStaleCleanupRequest, WorkflowSessionStaleCleanupResponse,
    WorkflowSessionStatusRequest, WorkflowSessionStatusResponse,
};
use tauri::{AppHandle, State};

use crate::agent::rag::SharedRagManager;
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

use super::commands::{SharedExtensions, SharedWorkflowService};
use super::headless_diagnostics::workflow_scheduler_snapshot_response;
pub(crate) use super::headless_runtime::build_runtime;

fn workflow_error_json(error: WorkflowServiceError) -> String {
    error.to_envelope_json()
}

pub async fn workflow_run(
    request: WorkflowRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    runtime
        .workflow_run(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_capabilities(
    request: WorkflowCapabilitiesRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowCapabilitiesResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_get_capabilities(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_get_io(
    request: WorkflowIoRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowIoResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_get_io(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_preflight(
    request: WorkflowPreflightRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowPreflightResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_preflight(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_create_session(
    request: WorkflowSessionCreateRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCreateResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .create_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_run_session(
    request: WorkflowSessionRunRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    rag_manager: State<'_, SharedRagManager>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowRunResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        Some(rag_manager.inner()),
    )
    .await?;
    runtime
        .run_workflow_session(request)
        .await
        .map_err(workflow_error_json)
}

pub async fn workflow_close_session(
    request: WorkflowSessionCloseRequest,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionCloseResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
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

pub async fn workflow_cleanup_stale_sessions(
    request: WorkflowSessionStaleCleanupRequest,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionStaleCleanupResponse, String> {
    workflow_service
        .workflow_cleanup_stale_sessions(request)
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
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    extensions: State<'_, SharedExtensions>,
    workflow_service: State<'_, SharedWorkflowService>,
) -> Result<WorkflowSessionKeepAliveResponse, String> {
    let runtime = build_runtime(
        &app,
        gateway.inner(),
        runtime_registry.inner(),
        extensions.inner(),
        workflow_service.inner(),
        None,
    )
    .await?;
    runtime
        .workflow_set_session_keep_alive(request)
        .await
        .map_err(workflow_error_json)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::workflow::diagnostics::{
        DiagnosticsRuntimeLifecycleSnapshot, WorkflowDiagnosticsSnapshotRequest,
        WorkflowDiagnosticsStore,
    };
    use crate::workflow::headless_diagnostics::{
        record_headless_runtime_snapshot, record_headless_scheduler_snapshot,
        stored_runtime_model_targets, stored_runtime_snapshots, stored_runtime_trace_metrics,
        workflow_clear_diagnostics_history_response, workflow_diagnostics_snapshot_projection,
        workflow_scheduler_snapshot_response, workflow_trace_snapshot_response,
    };
    use pantograph_workflow_service::graph::WorkflowSessionKind;
    use pantograph_workflow_service::{
        WorkflowCapabilitiesResponse, WorkflowCapabilityModel, WorkflowErrorCode,
        WorkflowErrorDetails, WorkflowErrorEnvelope, WorkflowGraph,
        WorkflowGraphEditSessionCreateRequest, WorkflowRuntimeRequirements,
        WorkflowSchedulerErrorDetails, WorkflowSchedulerRuntimeRegistryDiagnostics,
        WorkflowSchedulerRuntimeWarmupDecision, WorkflowSchedulerRuntimeWarmupReason,
        WorkflowSchedulerSnapshotDiagnostics, WorkflowSchedulerSnapshotRequest,
        WorkflowSchedulerSnapshotResponse, WorkflowService, WorkflowServiceError,
        WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus, WorkflowSessionState,
        WorkflowSessionSummary, WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest,
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            }),
            120,
        );

        assert_eq!(execution_id.as_deref(), Some("run-1"));
        let trace = diagnostics_store
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                workflow_name: None,
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
    fn headless_scheduler_snapshot_helper_keeps_error_overlay_without_invented_run_identity() {
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

        assert_eq!(execution_id, None);
        let projection = diagnostics_store.snapshot();
        assert_eq!(projection.scheduler.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(
            projection.scheduler.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(projection.scheduler.trace_execution_id, None);
        assert_eq!(
            projection.scheduler.last_error.as_deref(),
            Some("{\"code\":\"invalid_request\",\"message\":\"session missing\"}")
        );
        assert!(projection.run_order.is_empty());
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
                observed_runtime_ids: vec!["llama_cpp".to_string()],
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
                workflow_name: None,
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
                workflow_name: None,
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            }),
            120,
        );
        assert_eq!(execution_id.as_deref(), Some("run-1"));

        record_headless_runtime_snapshot(
            &diagnostics_store,
            "wf-1".to_string(),
            Some("run-1"),
            Ok(capability_response()),
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
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
                workflow_name: None,
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            }),
            120,
        );
        assert_eq!(execution_id.as_deref(), Some("run-1"));

        let snapshot = workflow_trace_snapshot_response(
            &diagnostics_store,
            WorkflowTraceSnapshotRequest {
                execution_id: Some("run-1".to_string()),
                session_id: None,
                workflow_id: None,
                workflow_name: None,
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            }),
            120,
        );
        assert_eq!(execution_id.as_deref(), Some("run-1"));

        let snapshot = workflow_trace_snapshot_response(
            &diagnostics_store,
            WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: None,
                workflow_name: None,
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
                workflow_name: None,
                include_completed: None,
            },
        )
        .expect_err("blank execution id should be rejected");

        assert!(error.contains("\"code\":\"invalid_request\""));
        assert!(error
            .contains("workflow trace snapshot request field 'execution_id' must not be blank"));
    }

    #[test]
    fn workflow_transport_error_json_preserves_backend_error_envelopes() {
        let cases = [
            (
                WorkflowServiceError::InvalidRequest(
                    "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'"
                        .to_string(),
                ),
                WorkflowErrorCode::InvalidRequest,
                "workflow 'interactive-human-input' requires interactive input at node 'human-input-1'",
                None,
            ),
            (
                WorkflowServiceError::RuntimeNotReady("runtime unavailable".to_string()),
                WorkflowErrorCode::RuntimeNotReady,
                "runtime unavailable",
                None,
            ),
            (
                WorkflowServiceError::CapabilityViolation("runtime admission rejected".to_string()),
                WorkflowErrorCode::CapabilityViolation,
                "runtime admission rejected",
                None,
            ),
            (
                WorkflowServiceError::Cancelled("workflow run cancelled".to_string()),
                WorkflowErrorCode::Cancelled,
                "workflow run cancelled",
                None,
            ),
            (
                WorkflowServiceError::scheduler_runtime_capacity_exhausted(1, 1, 0),
                WorkflowErrorCode::SchedulerBusy,
                "runtime capacity exhausted; no idle session runtime available for unload",
                Some(WorkflowErrorDetails::Scheduler(
                    WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(1, 1, 0),
                )),
            ),
        ];

        for (error, expected_code, expected_message, expected_details) in cases {
            let envelope: WorkflowErrorEnvelope =
                serde_json::from_str(&super::workflow_error_json(error))
                    .expect("parse error envelope");

            assert_eq!(envelope.code, expected_code);
            assert_eq!(envelope.message, expected_message);
            assert_eq!(envelope.details, expected_details);
        }
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
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
    fn workflow_diagnostics_snapshot_projection_preserves_scheduler_runtime_registry_diagnostics() {
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
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: Some(0),
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Pending,
                }],
                diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                    loaded_session_count: 1,
                    max_loaded_sessions: 2,
                    reclaimable_loaded_session_count: 1,
                    runtime_capacity_pressure:
                        pantograph_workflow_service::WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                    active_run_blocks_admission: false,
                    next_admission_queue_id: Some("queue-1".to_string()),
                    next_admission_bypassed_queue_id: None,
                    next_admission_after_runs: Some(0),
                    next_admission_wait_ms: Some(0),
                    next_admission_not_before_ms: Some(120),
                    next_admission_reason: Some(
                        pantograph_workflow_service::WorkflowSchedulerDecisionReason::WarmSessionReused,
                    ),
                    runtime_registry: Some(WorkflowSchedulerRuntimeRegistryDiagnostics {
                        target_runtime_id: Some("llama_cpp".to_string()),
                        reclaim_candidate_session_id: Some("session-loaded".to_string()),
                        reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                        next_warmup_decision: Some(
                            WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                        ),
                        next_warmup_reason: Some(
                            WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,
                        ),
                    }),
                }),
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:34b".to_string()),
                warmup_started_at_ms: Some(90),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(9),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
            },
            Some("llava:34b".to_string()),
            None,
            None,
            None,
            120,
        );

        assert_eq!(
            projection
                .scheduler
                .diagnostics
                .as_ref()
                .and_then(|diagnostics| diagnostics.runtime_registry.clone()),
            Some(WorkflowSchedulerRuntimeRegistryDiagnostics {
                target_runtime_id: Some("llama_cpp".to_string()),
                reclaim_candidate_session_id: Some("session-loaded".to_string()),
                reclaim_candidate_runtime_id: Some("llama_cpp".to_string()),
                next_warmup_decision: Some(
                    WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,
                ),
                next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,),
            })
        );
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
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                observed_runtime_ids: vec!["llama.cpp.embedding".to_string()],
                runtime_instance_id: Some("embed-7".to_string()),
                model_target: Some("/models/embed.gguf".to_string()),
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

        let metrics =
            stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
                .expect("stored trace metrics should exist");

        assert_eq!(metrics.runtime_id.as_deref(), Some("llama.cpp.embedding"));
        assert_eq!(metrics.runtime_instance_id.as_deref(), Some("embed-7"));
        assert_eq!(metrics.model_target.as_deref(), Some("/models/embed.gguf"));
        assert_eq!(metrics.runtime_reused, Some(true));
        assert_eq!(
            metrics.lifecycle_decision_reason.as_deref(),
            Some("runtime_reused")
        );
    }

    #[test]
    fn stored_runtime_snapshots_return_recorded_active_runtime() {
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
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("onnx-runtime".to_string()),
                observed_runtime_ids: vec!["onnx-runtime".to_string()],
                runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                model_target: Some("/tmp/model.onnx".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            Some(inference::RuntimeLifecycleSnapshot::from(
                &DiagnosticsRuntimeLifecycleSnapshot {
                    runtime_id: Some("onnx-runtime".to_string()),
                    runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                    warmup_started_at_ms: None,
                    warmup_completed_at_ms: None,
                    warmup_duration_ms: None,
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: true,
                    last_error: None,
                },
            )),
            None,
            120,
        );

        let (active_runtime, embedding_runtime) =
            stored_runtime_snapshots(&diagnostics_store, Some("wf-1"))
                .expect("stored runtime snapshots should exist");

        assert_eq!(
            active_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_id.as_deref()),
            Some("onnx-runtime")
        );
        assert_eq!(
            active_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.runtime_instance_id.as_deref()),
            Some("python-runtime:onnx-runtime:venv_onnx")
        );
        assert!(embedding_runtime.is_none());
    }

    #[test]
    fn stored_runtime_snapshots_normalize_missing_lifecycle_reason_from_diagnostics_projection() {
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
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:13b".to_string()),
                warmup_started_at_ms: Some(100),
                warmup_completed_at_ms: Some(110),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            Some("llava:13b".to_string()),
            None,
            Some(inference::RuntimeLifecycleSnapshot::from(
                &DiagnosticsRuntimeLifecycleSnapshot {
                    runtime_id: Some("llama_cpp".to_string()),
                    runtime_instance_id: Some("runtime-1".to_string()),
                    warmup_started_at_ms: Some(100),
                    warmup_completed_at_ms: Some(110),
                    warmup_duration_ms: Some(10),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: None,
                    active: true,
                    last_error: None,
                },
            )),
            None,
            120,
        );

        let (active_runtime, _) = stored_runtime_snapshots(&diagnostics_store, Some("wf-1"))
            .expect("stored runtime snapshots should exist");

        assert_eq!(
            active_runtime
                .as_ref()
                .and_then(|snapshot| snapshot.lifecycle_decision_reason.as_deref()),
            Some("runtime_ready")
        );
    }

    #[test]
    fn stored_runtime_model_targets_return_recorded_runtime_targets() {
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
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("onnx-runtime".to_string()),
                observed_runtime_ids: vec!["onnx-runtime".to_string()],
                runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                model_target: Some("/tmp/model.onnx".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            Some("/tmp/model.onnx".to_string()),
            Some("/models/embed.gguf".to_string()),
            Some(inference::RuntimeLifecycleSnapshot::from(
                &DiagnosticsRuntimeLifecycleSnapshot {
                    runtime_id: Some("onnx-runtime".to_string()),
                    runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                    warmup_started_at_ms: None,
                    warmup_completed_at_ms: None,
                    warmup_duration_ms: None,
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                    active: false,
                    last_error: None,
                },
            )),
            None,
            120,
        );

        let (active_model_target, embedding_model_target) =
            stored_runtime_model_targets(&diagnostics_store, Some("wf-1"))
                .expect("stored runtime model targets should exist");

        assert_eq!(active_model_target.as_deref(), Some("/tmp/model.onnx"));
        assert_eq!(
            embedding_model_target.as_deref(),
            Some("/models/embed.gguf")
        );
    }

    #[test]
    fn workflow_diagnostics_snapshot_projection_preserves_observed_runtime_ids() {
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
                items: vec![],
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("onnx-runtime".to_string()),
                observed_runtime_ids: vec!["pytorch".to_string(), "onnx-runtime".to_string()],
                runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                model_target: Some("/tmp/model.onnx".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            Some("llava:34b".to_string()),
            Some("/models/embed.gguf".to_string()),
            None,
            None,
            120,
        );

        let trace = projection.runs_by_id.get("run-1").expect("joined trace");
        assert_eq!(
            trace.runtime.observed_runtime_ids,
            vec!["pytorch".to_string(), "onnx-runtime".to_string()]
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
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
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            })),
            Some(Ok(capability_response())),
            None,
            WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
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
