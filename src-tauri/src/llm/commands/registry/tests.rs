use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
use inference::{EmbeddingMemoryMode, LlamaCppEmbeddingRuntime};
use tokio::sync::mpsc;

use super::{
    reclaim_runtime, resolve_runtime_debug_trace_scope, runtime_debug_snapshot_response,
    runtime_registry_snapshot_response, RuntimeDebugSnapshotRequest,
};
use crate::llm::health_monitor::{
    HealthCheckResult, HealthMonitor, HealthMonitorConfig, HealthStatus, SharedHealthMonitor,
};
use crate::llm::recovery::{RecoveryManager, SharedRecoveryManager};
use crate::llm::{InferenceGateway, RuntimeRegistry, SharedGateway, SharedRuntimeRegistry};
use crate::workflow::diagnostics::{
    SharedWorkflowDiagnosticsStore, WorkflowRuntimeSnapshotRecord, WorkflowSchedulerSnapshotRecord,
};
use pantograph_workflow_service::{
    graph::WorkflowExecutionSessionKind, WorkflowCapabilitiesResponse,
    WorkflowExecutionSessionState, WorkflowExecutionSessionSummary, WorkflowServiceError,
    WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest,
};

struct MockProcessHandle;

impl ProcessHandle for MockProcessHandle {
    fn pid(&self) -> u32 {
        21
    }

    fn kill(&self) -> Result<(), String> {
        Ok(())
    }
}

struct MockProcessSpawner;

#[async_trait]
impl ProcessSpawner for MockProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        _args: &[&str],
    ) -> Result<(mpsc::Receiver<ProcessEvent>, Box<dyn ProcessHandle>), String> {
        Err("spawn not used in runtime registry command tests".to_string())
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }
}

#[tokio::test]
async fn runtime_registry_snapshot_syncs_embedding_runtime_observation() {
    let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
    gateway.init().await;

    let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
    server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
    server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-10".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    });
    gateway.set_test_embedding_server(server).await;

    let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
    let snapshot = runtime_registry_snapshot_response(&gateway, &runtime_registry).await;

    let runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-cpp-embedding-10")
    );
    assert_eq!(
        runtime.status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Ready
    );
}

#[tokio::test]
async fn reclaim_runtime_returns_updated_registry_snapshot() {
    let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
    gateway.init().await;

    let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
    server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
    server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-11".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    });
    gateway.set_test_embedding_server(server).await;

    let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
    let response = reclaim_runtime(&gateway, &runtime_registry, "llama_cpp_embedding")
        .await
        .expect("reclaim should succeed");

    assert_eq!(
        response.reclaim,
        pantograph_runtime_registry::RuntimeReclaimDisposition::stop_producer(
            "llama.cpp.embedding",
            pantograph_runtime_registry::RuntimeRegistryStatus::Stopping,
        )
    );
    let runtime = response
        .snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(
        runtime.status,
        pantograph_runtime_registry::RuntimeRegistryStatus::Stopped
    );
    assert!(runtime.runtime_instance_id.is_none());
}

#[tokio::test]
async fn runtime_debug_snapshot_includes_synced_runtime_and_recovery_state() {
    let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
    gateway.init().await;

    let mut server = LlamaCppEmbeddingRuntime::new(EmbeddingMemoryMode::CpuParallel);
    server.set_test_ready_state(Box::new(MockProcessHandle), "/models/embed.gguf");
    server.set_test_runtime_lifecycle_snapshot(inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-12".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    });
    gateway.set_test_embedding_server(server).await;

    let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
    let health_monitor: SharedHealthMonitor =
        Arc::new(HealthMonitor::new(HealthMonitorConfig::default()));
    health_monitor
        .set_test_last_result(Some(HealthCheckResult {
            healthy: true,
            status: HealthStatus::Healthy,
            response_time_ms: Some(25),
            error: None,
            timestamp: Utc::now(),
            consecutive_failures: 0,
        }))
        .await;
    let recovery_manager: SharedRecoveryManager = Arc::new(RecoveryManager::default());
    let workflow_diagnostics: SharedWorkflowDiagnosticsStore = Arc::new(Default::default());
    workflow_diagnostics.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "workflow-debug".to_string(),
        execution_id: "execution-debug".to_string(),
        captured_at_ms: 123,
        capabilities: Some(WorkflowCapabilitiesResponse {
            max_input_bindings: 1,
            max_output_targets: 1,
            max_value_bytes: 1024,
            runtime_requirements: Default::default(),
            models: Vec::new(),
            runtime_capabilities: Vec::new(),
        }),
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            observed_runtime_ids: vec!["llama.cpp.embedding".to_string()],
            runtime_instance_id: Some("llama-cpp-embedding-12".to_string()),
            model_target: Some("/models/embed.gguf".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
        },
        active_model_target: Some("/models/embed.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-12".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        error: None,
    });
    let scheduler_projection =
        workflow_diagnostics.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
            workflow_id: Some("workflow-debug".to_string()),
            execution_id: "execution-debug".to_string(),
            session_id: "session-debug".to_string(),
            captured_at_ms: 456,
            session: Some(WorkflowExecutionSessionSummary {
                session_id: "session-debug".to_string(),
                workflow_id: "workflow-debug".to_string(),
                session_kind: WorkflowExecutionSessionKind::Workflow,
                usage_profile: None,
                keep_alive: false,
                state: WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            items: Vec::new(),
            diagnostics: None,
            error: None,
        });
    assert_eq!(
        scheduler_projection.scheduler.workflow_id.as_deref(),
        Some("workflow-debug")
    );
    let workflow_diagnostics_projection = workflow_diagnostics.snapshot();
    let workflow_trace = workflow_diagnostics
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            execution_id: Some("execution-debug".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(true),
        })
        .expect("workflow trace snapshot");

    let response = runtime_debug_snapshot_response(
        &gateway,
        &runtime_registry,
        Some(&health_monitor),
        Some(&recovery_manager),
        Some(workflow_diagnostics_projection),
        Some(workflow_trace),
        None,
    )
    .await;

    let runtime = response
        .snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama.cpp.embedding")
        .expect("embedding runtime snapshot");
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("llama-cpp-embedding-12")
    );
    assert_eq!(
        response.mode_info.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert!(!response.health_monitor_running);
    assert_eq!(
        response
            .last_health_check
            .as_ref()
            .and_then(|result| result.response_time_ms),
        Some(25)
    );
    assert_eq!(
        response
            .recovery
            .as_ref()
            .map(|recovery| recovery.attempt_count),
        Some(0)
    );
    assert_eq!(
        response
            .recovery
            .as_ref()
            .and_then(|recovery| recovery.last_error.as_deref()),
        None
    );
    assert_eq!(
        response
            .workflow_runtime_diagnostics
            .as_ref()
            .and_then(|runtime| runtime.workflow_id.as_deref()),
        Some("workflow-debug")
    );
    assert_eq!(
        response
            .workflow_runtime_diagnostics
            .as_ref()
            .and_then(|runtime| runtime.active_runtime.as_ref())
            .and_then(|runtime| runtime.lifecycle_decision_reason.as_deref()),
        Some("runtime_ready")
    );
    assert_eq!(
        response
            .workflow_scheduler_diagnostics
            .as_ref()
            .and_then(|scheduler| scheduler.session_id.as_deref()),
        Some("session-debug")
    );
    assert_eq!(
        response
            .workflow_trace
            .as_ref()
            .map(|trace| trace.traces.len()),
        Some(1)
    );
}

#[tokio::test]
async fn runtime_debug_snapshot_preserves_backend_trace_and_scheduler_contracts() {
    let gateway: SharedGateway = Arc::new(InferenceGateway::new(Arc::new(MockProcessSpawner)));
    gateway.init().await;

    let runtime_registry: SharedRuntimeRegistry = Arc::new(RuntimeRegistry::new());
    let workflow_diagnostics: SharedWorkflowDiagnosticsStore = Arc::new(Default::default());
    workflow_diagnostics.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "workflow-debug".to_string(),
        execution_id: "execution-debug".to_string(),
        captured_at_ms: 123,
        capabilities: None,
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            observed_runtime_ids: vec![
                "llama.cpp.embedding".to_string(),
                "llama_cpp_embedding".to_string(),
            ],
            runtime_instance_id: Some("llama-cpp-embedding-13".to_string()),
            model_target: Some("/models/embed.gguf".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
        },
        active_model_target: Some("/models/embed.gguf".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-13".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        error: None,
    });
    workflow_diagnostics.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
        workflow_id: Some("workflow-debug".to_string()),
        execution_id: "execution-debug".to_string(),
        session_id: "session-debug".to_string(),
        captured_at_ms: 456,
        session: Some(WorkflowExecutionSessionSummary {
            session_id: "session-debug".to_string(),
            workflow_id: "workflow-debug".to_string(),
            session_kind: WorkflowExecutionSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: WorkflowExecutionSessionState::Running,
            queued_runs: 1,
            run_count: 1,
        }),
        items: vec![
            pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("execution-debug".to_string()),
                enqueued_at_ms: Some(400),
                dequeued_at_ms: Some(430),
                priority: 3,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status:
                    pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
            },
        ],
        diagnostics: None,
        error: None,
    });
    let workflow_diagnostics_projection = workflow_diagnostics.snapshot();
    let workflow_trace = workflow_diagnostics
        .trace_snapshot(WorkflowTraceSnapshotRequest {
            execution_id: Some("execution-debug".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(true),
        })
        .expect("workflow trace snapshot");

    let response = runtime_debug_snapshot_response(
        &gateway,
        &runtime_registry,
        None,
        None,
        Some(workflow_diagnostics_projection),
        Some(workflow_trace),
        None,
    )
    .await;

    let scheduler = response
        .workflow_scheduler_diagnostics
        .as_ref()
        .expect("scheduler diagnostics");
    assert_eq!(scheduler.session_id.as_deref(), Some("session-debug"));
    assert_eq!(
        scheduler.trace_execution_id.as_deref(),
        Some("execution-debug")
    );

    let runtime = response
        .workflow_runtime_diagnostics
        .as_ref()
        .expect("runtime diagnostics");
    assert_eq!(runtime.workflow_id.as_deref(), Some("workflow-debug"));
    assert_eq!(
        runtime.active_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        runtime
            .active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.as_deref()),
        Some("llama.cpp.embedding")
    );

    let trace = response
        .workflow_trace
        .as_ref()
        .and_then(|trace| trace.traces.first())
        .expect("workflow trace");
    assert_eq!(trace.execution_id, "execution-debug");
    assert_eq!(trace.session_id.as_deref(), Some("session-debug"));
    assert_eq!(trace.workflow_id.as_deref(), Some("workflow-debug"));
    assert_eq!(trace.queue.enqueued_at_ms, Some(400));
    assert_eq!(trace.queue.dequeued_at_ms, Some(430));
    assert_eq!(trace.queue.queue_wait_ms, Some(30));
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_running_item")
    );
    assert_eq!(
        trace.runtime.observed_runtime_ids,
        vec![
            "llama.cpp.embedding".to_string(),
            "llama_cpp_embedding".to_string(),
        ]
    );
}

#[test]
fn runtime_debug_snapshot_request_serializes_optional_workflow_filters() {
    let request = RuntimeDebugSnapshotRequest {
        execution_id: Some("execution-1".to_string()),
        session_id: Some("session-1".to_string()),
        workflow_id: Some("workflow-1".to_string()),
        workflow_name: Some("Workflow 1".to_string()),
        include_trace: Some(true),
        include_completed: Some(false),
    };

    let value = serde_json::to_value(request).expect("serialize runtime debug request");
    assert_eq!(
        value,
        serde_json::json!({
            "execution_id": "execution-1",
            "session_id": "session-1",
            "workflow_id": "workflow-1",
            "workflow_name": "Workflow 1",
            "include_trace": true,
            "include_completed": false
        })
    );
}

#[test]
fn runtime_debug_snapshot_request_normalizes_and_rejects_blank_filters() {
    let normalized = RuntimeDebugSnapshotRequest {
        execution_id: Some("  execution-1  ".to_string()),
        session_id: Some("  ".to_string()),
        workflow_id: Some("\tworkflow-1\t".to_string()),
        workflow_name: Some("  Workflow 1  ".to_string()),
        include_trace: Some(true),
        include_completed: Some(false),
    }
    .normalized();

    assert_eq!(normalized.execution_id.as_deref(), Some("execution-1"));
    assert_eq!(normalized.session_id.as_deref(), Some(""));
    assert_eq!(normalized.workflow_id.as_deref(), Some("workflow-1"));
    assert_eq!(normalized.workflow_name.as_deref(), Some("Workflow 1"));

    let error = normalized
        .validate()
        .expect_err("blank session_id should be rejected");
    assert!(
        matches!(
            error,
            WorkflowServiceError::InvalidRequest(ref message)
                if message
                    == "runtime debug snapshot request field 'session_id' must not be blank"
        ),
        "unexpected validation error: {:?}",
        error
    );
}

#[test]
fn resolve_runtime_debug_trace_scope_uses_unique_execution_match() {
    let diagnostics_store: SharedWorkflowDiagnosticsStore = Arc::new(Default::default());
    diagnostics_store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "workflow-debug".to_string(),
        execution_id: "execution-debug".to_string(),
        captured_at_ms: 123,
        capabilities: None,
        trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            observed_runtime_ids: vec!["llama.cpp.embedding".to_string()],
            runtime_instance_id: Some("llama-cpp-embedding-21".to_string()),
            model_target: Some("/models/embed.gguf".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
        },
        active_model_target: Some("/models/embed.gguf".to_string()),
        ..Default::default()
    });

    let (request, selection) = resolve_runtime_debug_trace_scope(
        Some(&diagnostics_store),
        &WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: None,
            workflow_id: Some("workflow-debug".to_string()),
            workflow_name: None,
            include_completed: Some(true),
        },
    )
    .expect("trace selection should succeed")
    .expect("trace selection should exist");

    assert_eq!(request.execution_id.as_deref(), Some("execution-debug"));
    assert_eq!(selection.execution_id.as_deref(), Some("execution-debug"));
    assert!(!selection.ambiguous);
    assert_eq!(
        selection.matched_execution_ids,
        vec!["execution-debug".to_string()]
    );
}

#[test]
fn resolve_runtime_debug_trace_scope_marks_multi_run_scope_ambiguous() {
    let diagnostics_store: SharedWorkflowDiagnosticsStore = Arc::new(Default::default());
    for execution_id in ["execution-a", "execution-b"] {
        diagnostics_store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
            workflow_id: "workflow-debug".to_string(),
            execution_id: execution_id.to_string(),
            captured_at_ms: 123,
            capabilities: None,
            trace_runtime_metrics: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama.cpp.embedding".to_string()),
                observed_runtime_ids: vec!["llama.cpp.embedding".to_string()],
                runtime_instance_id: Some(format!("runtime-{}", execution_id)),
                model_target: Some("/models/embed.gguf".to_string()),
                warmup_started_at_ms: Some(10),
                warmup_completed_at_ms: Some(20),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            active_model_target: Some("/models/embed.gguf".to_string()),
            ..Default::default()
        });
    }

    let (request, selection) = resolve_runtime_debug_trace_scope(
        Some(&diagnostics_store),
        &WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: None,
            workflow_id: Some("workflow-debug".to_string()),
            workflow_name: None,
            include_completed: Some(true),
        },
    )
    .expect("trace selection should succeed")
    .expect("trace selection should exist");

    assert!(request.execution_id.is_none());
    assert!(selection.execution_id.is_none());
    assert!(selection.ambiguous);
    assert_eq!(
        selection.matched_execution_ids,
        vec!["execution-b".to_string(), "execution-a".to_string()]
    );
}
