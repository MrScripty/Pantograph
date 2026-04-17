//! Runtime-registry inspection and targeted reclaim commands.

use crate::config::ServerModeInfo;
use crate::llm::commands::shared::synced_server_mode_info;
use crate::llm::health_monitor::{HealthCheckResult, SharedHealthMonitor};
use crate::llm::recovery::{RecoveryConfig, SharedRecoveryManager};
use crate::workflow::commands::{
    SharedExtensions, SharedWorkflowDiagnosticsStore, SharedWorkflowService,
};
use crate::workflow::diagnostics::{
    DiagnosticsRuntimeSnapshot, DiagnosticsSchedulerSnapshot, WorkflowDiagnosticsProjection,
    WorkflowDiagnosticsSnapshotRequest,
};
use crate::workflow::headless_diagnostics_transport::{
    workflow_diagnostics_snapshot_response, workflow_trace_snapshot_response,
};
use pantograph_workflow_service::{WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse};
use serde::{Deserialize, Serialize};
use tauri::{command, AppHandle, Manager, State};

use crate::llm::runtime_registry::{
    reclaim_runtime_and_sync_runtime_registry,
    runtime_registry_snapshot as synced_runtime_registry_snapshot,
};
use crate::llm::{SharedGateway, SharedRuntimeRegistry};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRegistryReclaimResponse {
    pub reclaim: pantograph_runtime_registry::RuntimeReclaimDisposition,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeRecoveryDebugState {
    pub in_progress: bool,
    pub attempt_count: u32,
    pub config: RecoveryConfig,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugSnapshot {
    pub mode_info: ServerModeInfo,
    pub snapshot: pantograph_runtime_registry::RuntimeRegistrySnapshot,
    pub health_monitor_running: bool,
    pub last_health_check: Option<HealthCheckResult>,
    pub recovery: Option<RuntimeRecoveryDebugState>,
    pub workflow_runtime_diagnostics: Option<DiagnosticsRuntimeSnapshot>,
    pub workflow_scheduler_diagnostics: Option<DiagnosticsSchedulerSnapshot>,
    pub workflow_trace: Option<WorkflowTraceSnapshotResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RuntimeDebugSnapshotRequest {
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_name: Option<String>,
    #[serde(default)]
    pub include_trace: Option<bool>,
    #[serde(default)]
    pub include_completed: Option<bool>,
}

async fn runtime_registry_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
) -> pantograph_runtime_registry::RuntimeRegistrySnapshot {
    synced_runtime_registry_snapshot(gateway.as_ref(), runtime_registry).await
}

async fn runtime_debug_snapshot_response(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    health_monitor: Option<&SharedHealthMonitor>,
    recovery_manager: Option<&SharedRecoveryManager>,
    workflow_diagnostics: Option<WorkflowDiagnosticsProjection>,
    workflow_trace: Option<WorkflowTraceSnapshotResponse>,
) -> RuntimeDebugSnapshot {
    let mode_info = synced_server_mode_info(gateway, runtime_registry).await;
    let health_monitor_running = health_monitor
        .map(|monitor| monitor.is_running())
        .unwrap_or(false);
    let last_health_check = match health_monitor {
        Some(monitor) => monitor.last_result().await,
        None => None,
    };
    let recovery = match recovery_manager {
        Some(manager) => Some(RuntimeRecoveryDebugState {
            in_progress: manager.is_recovering(),
            attempt_count: manager.attempt_count(),
            config: manager.config().clone(),
            last_error: manager.last_error().await,
        }),
        None => None,
    };
    let workflow_runtime_diagnostics = workflow_diagnostics
        .as_ref()
        .map(|projection| projection.runtime.clone());
    let workflow_scheduler_diagnostics = workflow_diagnostics
        .as_ref()
        .map(|projection| projection.scheduler.clone());

    RuntimeDebugSnapshot {
        mode_info,
        snapshot: runtime_registry.snapshot(),
        health_monitor_running,
        last_health_check,
        recovery,
        workflow_runtime_diagnostics,
        workflow_scheduler_diagnostics,
        workflow_trace,
    }
}

async fn reclaim_runtime(
    gateway: &SharedGateway,
    runtime_registry: &SharedRuntimeRegistry,
    runtime_id: &str,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    let reclaim = reclaim_runtime_and_sync_runtime_registry(gateway, runtime_registry, runtime_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(RuntimeRegistryReclaimResponse {
        reclaim,
        snapshot: runtime_registry.snapshot(),
    })
}

#[command]
pub async fn get_runtime_registry_snapshot(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<pantograph_runtime_registry::RuntimeRegistrySnapshot, String> {
    Ok(runtime_registry_snapshot_response(gateway.inner(), runtime_registry.inner()).await)
}

#[command]
pub async fn get_runtime_debug_snapshot(
    request: Option<RuntimeDebugSnapshotRequest>,
    app: AppHandle,
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
) -> Result<RuntimeDebugSnapshot, String> {
    let health_monitor = app
        .try_state::<SharedHealthMonitor>()
        .map(|monitor| (*monitor).clone());
    let recovery_manager = app
        .try_state::<SharedRecoveryManager>()
        .map(|manager| (*manager).clone());
    let workflow_diagnostics_store = app
        .try_state::<SharedWorkflowDiagnosticsStore>()
        .map(|store| (*store).clone());
    let workflow_service = app
        .try_state::<SharedWorkflowService>()
        .map(|service| (*service).clone());
    let extensions = app
        .try_state::<SharedExtensions>()
        .map(|extensions| (*extensions).clone());
    let workflow_request = request.unwrap_or_default();
    let execution_id_filter = workflow_request.execution_id.clone();
    let session_id_filter = workflow_request.session_id.clone();
    let workflow_id_filter = workflow_request.workflow_id.clone();
    let workflow_name_filter = workflow_request.workflow_name.clone();
    let include_trace = workflow_request.include_trace.unwrap_or(false);
    let include_completed = workflow_request.include_completed;
    let has_workflow_filter = session_id_filter.is_some()
        || workflow_id_filter.is_some()
        || workflow_name_filter.is_some();
    let workflow_diagnostics = match (
        workflow_diagnostics_store.clone(),
        workflow_service,
        extensions,
        has_workflow_filter,
    ) {
        (Some(store), Some(service), Some(extensions), true) => Some(
            workflow_diagnostics_snapshot_response(
                &app,
                gateway.inner(),
                runtime_registry.inner(),
                &extensions,
                &service,
                &store,
                WorkflowDiagnosticsSnapshotRequest {
                    session_id: session_id_filter.clone(),
                    workflow_id: workflow_id_filter.clone(),
                    workflow_name: workflow_name_filter.clone(),
                },
            )
            .await?,
        ),
        (Some(store), _, _, _) => Some(store.snapshot()),
        _ => None,
    };
    let workflow_trace = if include_trace {
        workflow_diagnostics_store
            .map(|store| {
                workflow_trace_snapshot_response(
                    &store,
                    WorkflowTraceSnapshotRequest {
                        execution_id: execution_id_filter,
                        session_id: session_id_filter,
                        workflow_id: workflow_id_filter,
                        include_completed,
                    },
                )
            })
            .transpose()?
    } else {
        None
    };

    Ok(runtime_debug_snapshot_response(
        gateway.inner(),
        runtime_registry.inner(),
        health_monitor.as_ref(),
        recovery_manager.as_ref(),
        workflow_diagnostics,
        workflow_trace,
    )
    .await)
}

#[command]
pub async fn reclaim_runtime_registry_runtime(
    gateway: State<'_, SharedGateway>,
    runtime_registry: State<'_, SharedRuntimeRegistry>,
    runtime_id: String,
) -> Result<RuntimeRegistryReclaimResponse, String> {
    reclaim_runtime(gateway.inner(), runtime_registry.inner(), &runtime_id).await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use inference::process::{ProcessEvent, ProcessHandle, ProcessSpawner};
    use inference::{EmbeddingMemoryMode, LlamaCppEmbeddingRuntime};
    use tokio::sync::mpsc;

    use super::{
        reclaim_runtime, runtime_debug_snapshot_response, runtime_registry_snapshot_response,
        RuntimeDebugSnapshotRequest,
    };
    use crate::llm::health_monitor::{
        HealthCheckResult, HealthMonitor, HealthMonitorConfig, HealthStatus, SharedHealthMonitor,
    };
    use crate::llm::recovery::{RecoveryManager, SharedRecoveryManager};
    use crate::llm::{InferenceGateway, RuntimeRegistry, SharedGateway, SharedRuntimeRegistry};
    use crate::workflow::diagnostics::SharedWorkflowDiagnosticsStore;
    use chrono::Utc;
    use pantograph_workflow_service::{
        graph::WorkflowSessionKind, WorkflowCapabilitiesResponse, WorkflowSessionState,
        WorkflowSessionSummary, WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest,
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
        workflow_diagnostics.record_runtime_snapshot(
            "workflow-debug".to_string(),
            "execution-debug".to_string(),
            123,
            Some(WorkflowCapabilitiesResponse {
                max_input_bindings: 1,
                max_output_targets: 1,
                max_value_bytes: 1024,
                runtime_requirements: Default::default(),
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            }),
            WorkflowTraceRuntimeMetrics {
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
            Some("/models/embed.gguf".to_string()),
            Some("/models/embed.gguf".to_string()),
            Some(inference::RuntimeLifecycleSnapshot {
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
            None,
            None,
        );
        let scheduler_projection = workflow_diagnostics.record_scheduler_snapshot(
            Some("workflow-debug".to_string()),
            "execution-debug".to_string(),
            "session-debug".to_string(),
            456,
            Some(WorkflowSessionSummary {
                session_id: "session-debug".to_string(),
                workflow_id: "workflow-debug".to_string(),
                session_kind: WorkflowSessionKind::Workflow,
                usage_profile: None,
                keep_alive: false,
                state: WorkflowSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            Vec::new(),
            None,
        );
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
        workflow_diagnostics.record_runtime_snapshot(
            "workflow-debug".to_string(),
            "execution-debug".to_string(),
            123,
            None,
            WorkflowTraceRuntimeMetrics {
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
            Some("/models/embed.gguf".to_string()),
            None,
            Some(inference::RuntimeLifecycleSnapshot {
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
            None,
            None,
        );
        workflow_diagnostics.record_scheduler_snapshot(
            Some("workflow-debug".to_string()),
            "execution-debug".to_string(),
            "session-debug".to_string(),
            456,
            Some(WorkflowSessionSummary {
                session_id: "session-debug".to_string(),
                workflow_id: "workflow-debug".to_string(),
                session_kind: WorkflowSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
                state: WorkflowSessionState::Running,
                queued_runs: 1,
                run_count: 1,
            }),
            vec![pantograph_workflow_service::WorkflowSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("execution-debug".to_string()),
                enqueued_at_ms: Some(400),
                dequeued_at_ms: Some(430),
                priority: 3,
                status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
            }],
            None,
        );
        let workflow_diagnostics_projection = workflow_diagnostics.snapshot();
        let workflow_trace = workflow_diagnostics
            .trace_snapshot(WorkflowTraceSnapshotRequest {
                execution_id: Some("execution-debug".to_string()),
                session_id: None,
                workflow_id: None,
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
}
