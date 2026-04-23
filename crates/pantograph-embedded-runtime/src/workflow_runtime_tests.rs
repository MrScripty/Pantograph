use crate::HostRuntimeModeSnapshot;
use crate::runtime_health::RuntimeHealthAssessmentSnapshot;
use async_trait::async_trait;
use pantograph_runtime_registry::RuntimeRegistry;
use pantograph_workflow_service::graph::WorkflowSessionKind;
use pantograph_workflow_service::{
    WorkflowCapabilitiesResponse, WorkflowSchedulerSnapshotResponse, WorkflowSessionQueueItem,
    WorkflowSessionState, WorkflowSessionSummary, WorkflowTraceRuntimeMetrics,
};

use super::{
    WorkflowExecutionDiagnosticsController, WorkflowExecutionDiagnosticsInput,
    WorkflowExecutionDiagnosticsSyncInput, build_runtime_diagnostics_projection,
    build_runtime_event_projection, build_runtime_event_projection_with_registry_override,
    build_runtime_event_projection_with_registry_reconciliation,
    build_runtime_event_projection_with_registry_sync,
    build_workflow_execution_diagnostics_snapshot,
    build_workflow_execution_diagnostics_snapshot_with_registry_sync,
    normalized_runtime_lifecycle_snapshot, reconcile_runtime_registry_stored_projection_overrides,
    resolve_runtime_model_target, trace_runtime_metrics,
    trace_runtime_metrics_with_observed_runtime_ids,
};

struct MockRuntimeRegistryController {
    mode_info: HostRuntimeModeSnapshot,
    active_runtime_snapshot: inference::RuntimeLifecycleSnapshot,
    embedding_runtime_snapshot: Option<inference::RuntimeLifecycleSnapshot>,
}

#[async_trait]
impl crate::runtime_registry::HostRuntimeRegistryController for MockRuntimeRegistryController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        self.mode_info.clone()
    }

    async fn stop_runtime_producer(&self, _producer: crate::runtime_registry::HostRuntimeProducer) {
    }

    async fn runtime_health_assessment_snapshot(&self) -> RuntimeHealthAssessmentSnapshot {
        RuntimeHealthAssessmentSnapshot::default()
    }
}

#[async_trait]
impl WorkflowExecutionDiagnosticsController for MockRuntimeRegistryController {
    async fn active_runtime_lifecycle_snapshot(&self) -> inference::RuntimeLifecycleSnapshot {
        self.active_runtime_snapshot.clone()
    }

    async fn embedding_runtime_lifecycle_snapshot(
        &self,
    ) -> Option<inference::RuntimeLifecycleSnapshot> {
        self.embedding_runtime_snapshot.clone()
    }
}

#[test]
fn trace_runtime_metrics_keeps_canonical_backend_lifecycle_reason() {
    let metrics = trace_runtime_metrics(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("pytorch".to_string()),
            runtime_instance_id: Some("pytorch-1".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/demo"),
    );

    assert_eq!(
        metrics.lifecycle_decision_reason.as_deref(),
        Some("runtime_reused")
    );
    assert_eq!(metrics.model_target.as_deref(), Some("/models/demo"));
}

#[test]
fn trace_runtime_metrics_normalizes_known_runtime_aliases() {
    let metrics = trace_runtime_metrics(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
        Some("/models/main.gguf"),
    );

    assert_eq!(metrics.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(metrics.observed_runtime_ids, vec!["llama_cpp".to_string()]);
}

#[test]
fn normalized_runtime_lifecycle_snapshot_canonicalizes_runtime_aliases() {
    let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("PyTorch".to_string()),
        runtime_instance_id: Some("pytorch-1".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    });

    assert_eq!(snapshot.runtime_id.as_deref(), Some("pytorch"));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_reused")
    );
}

#[tokio::test]
async fn build_runtime_event_projection_with_registry_sync_reconciles_live_and_stored_runtimes() {
    let registry = RuntimeRegistry::new();
    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: None,
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-live".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: None,
    };

    let projection = build_runtime_event_projection_with_registry_sync(
        &MockRuntimeRegistryController {
            mode_info: gateway_mode_info.clone(),
            active_runtime_snapshot: gateway_mode_info
                .active_runtime
                .clone()
                .expect("gateway runtime snapshot"),
            embedding_runtime_snapshot: None,
        },
        &registry,
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:restored".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/restored.safetensors"),
        None,
        None,
        None,
        gateway_mode_info
            .active_runtime
            .as_ref()
            .expect("gateway runtime snapshot"),
        None,
        &gateway_mode_info,
        None,
    )
    .await;

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("PyTorch")
    );
    let snapshot = registry.snapshot();
    assert!(
        snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama_cpp")
    );
    let stored_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("stored python runtime should be replayed");
    assert_eq!(
        stored_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:restored")
    );
}

#[tokio::test]
async fn build_workflow_execution_diagnostics_snapshot_with_registry_sync_reconciles_execution_runtime()
 {
    let registry = RuntimeRegistry::new();
    let active_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-main-live".to_string()),
        warmup_started_at_ms: Some(1),
        warmup_completed_at_ms: Some(2),
        warmup_duration_ms: Some(1),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama_cpp_embedding".to_string()),
        runtime_instance_id: Some("embed-live".to_string()),
        warmup_started_at_ms: Some(3),
        warmup_completed_at_ms: Some(4),
        warmup_duration_ms: Some(1),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };

    let snapshot = build_workflow_execution_diagnostics_snapshot_with_registry_sync(
        &MockRuntimeRegistryController {
            mode_info: HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/main.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: Some(active_runtime_snapshot.clone()),
                embedding_runtime: Some(embedding_runtime_snapshot.clone()),
            },
            active_runtime_snapshot: active_runtime_snapshot.clone(),
            embedding_runtime_snapshot: Some(embedding_runtime_snapshot.clone()),
        },
        WorkflowExecutionDiagnosticsSyncInput {
            runtime_registry: Some(&registry),
            scheduler_snapshot: &WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-123".to_string()),
                session_id: "session-123".to_string(),
                trace_execution_id: Some("exec-456".to_string()),
                session: WorkflowSessionSummary {
                    session_id: "session-123".to_string(),
                    workflow_id: "wf-123".to_string(),
                    session_kind: WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                    state: WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                },
                items: Vec::new(),
                diagnostics: None,
            },
            captured_at_ms: 999,
            runtime_capabilities: None,
            runtime_error: None,
            trace_runtime_metrics_override: Some(WorkflowTraceRuntimeMetrics {
                runtime_id: Some("pytorch".to_string()),
                observed_runtime_ids: vec!["pytorch".to_string()],
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                model_target: Some("/models/sidecar.safetensors".to_string()),
                warmup_started_at_ms: Some(5),
                warmup_completed_at_ms: Some(9),
                warmup_duration_ms: Some(4),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            }),
            runtime_snapshot_override: Some(&inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                warmup_started_at_ms: Some(5),
                warmup_completed_at_ms: Some(9),
                warmup_duration_ms: Some(4),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            runtime_model_target_override: Some("/models/sidecar.safetensors"),
        },
    )
    .await;

    assert_eq!(snapshot.runtime.workflow_id, "wf-123");
    assert_eq!(
        snapshot
            .runtime
            .active_runtime_snapshot
            .runtime_id
            .as_deref(),
        Some("PyTorch")
    );
    assert_eq!(
        snapshot
            .runtime
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|runtime| runtime.runtime_instance_id.as_deref()),
        Some("embed-live")
    );

    let registry_snapshot = registry.snapshot();
    assert!(
        registry_snapshot
            .runtimes
            .iter()
            .any(|runtime| runtime.runtime_id == "llama_cpp")
    );
    let execution_runtime = registry_snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("execution runtime should reconcile into registry");
    assert_eq!(
        execution_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:default")
    );
}

#[test]
fn normalized_runtime_lifecycle_snapshot_infers_backend_owned_default_reason() {
    let snapshot = normalized_runtime_lifecycle_snapshot(&inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-1".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(false),
        lifecycle_decision_reason: None,
        active: true,
        last_error: None,
    });

    assert_eq!(snapshot.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[test]
fn trace_runtime_metrics_with_observed_runtime_ids_preserves_all_runtime_ids() {
    let metrics = trace_runtime_metrics_with_observed_runtime_ids(
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("onnxruntime".to_string()),
            runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: false,
            last_error: None,
        },
        Some("/tmp/model.onnx"),
        &[
            "diffusers".to_string(),
            "onnx-runtime".to_string(),
            "diffusers".to_string(),
        ],
    );

    assert_eq!(metrics.runtime_id.as_deref(), Some("onnx-runtime"));
    assert_eq!(
        metrics.observed_runtime_ids,
        vec!["onnx-runtime".to_string(), "diffusers".to_string()]
    );
    assert_eq!(metrics.model_target.as_deref(), Some("/tmp/model.onnx"));
}

#[test]
fn resolve_runtime_model_target_prefers_embedding_target_for_embedding_alias() {
    let mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };
    let snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama_cpp_embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };

    assert_eq!(
        resolve_runtime_model_target(&mode_info, &snapshot).as_deref(),
        Some("/models/embed.gguf")
    );
}

#[test]
fn build_runtime_diagnostics_projection_prefers_execution_snapshot_override() {
    let execution_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
        warmup_started_at_ms: Some(100),
        warmup_completed_at_ms: Some(110),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let restored_gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-restore-9".to_string()),
        warmup_started_at_ms: Some(200),
        warmup_completed_at_ms: Some(240),
        warmup_duration_ms: Some(40),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/restore.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_diagnostics_projection(
        Some(&execution_snapshot),
        &restored_gateway_snapshot,
        &mode_info,
        Some("/models/embed.gguf"),
    );

    assert_eq!(
        projection
            .active_runtime_snapshot
            .runtime_instance_id
            .as_deref(),
        Some("llama-cpp-embedding-2")
    );
    assert_eq!(
        projection.runtime_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection.trace_runtime_metrics.runtime_id.as_deref(),
        Some("llama.cpp.embedding")
    );
    assert_eq!(
        projection.trace_runtime_metrics.observed_runtime_ids,
        vec!["llama.cpp.embedding".to_string()]
    );
    assert_eq!(
        projection
            .trace_runtime_metrics
            .lifecycle_decision_reason
            .as_deref(),
        Some("runtime_reused")
    );
}

#[test]
fn build_runtime_event_projection_prefers_stored_runtime_over_gateway_snapshot() {
    let stored_active_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("onnx-runtime".to_string()),
        runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
        warmup_started_at_ms: None,
        warmup_completed_at_ms: None,
        warmup_duration_ms: None,
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: false,
        last_error: None,
    };
    let stored_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp.embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-3".to_string()),
        warmup_started_at_ms: Some(10),
        warmup_completed_at_ms: Some(20),
        warmup_duration_ms: Some(10),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-main-1".to_string()),
        warmup_started_at_ms: Some(1),
        warmup_completed_at_ms: Some(2),
        warmup_duration_ms: Some(1),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_event_projection(
        Some(&stored_active_runtime_snapshot),
        Some(&stored_embedding_runtime_snapshot),
        Some("/models/sidecar.onnx"),
        Some("/models/embed.gguf"),
        None,
        None,
        &gateway_snapshot,
        None,
        &gateway_mode_info,
        None,
    );

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        projection
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.as_deref()),
        Some("llama.cpp.embedding")
    );
    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/sidecar.onnx")
    );
    assert_eq!(
        projection.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection.trace_runtime_metrics.runtime_id.as_deref(),
        Some("onnx-runtime")
    );
}

#[test]
fn build_runtime_event_projection_preserves_live_embedding_snapshot_without_stored_override() {
    let gateway_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama.cpp".to_string()),
        runtime_instance_id: Some("llama-cpp-main-2".to_string()),
        warmup_started_at_ms: Some(1),
        warmup_completed_at_ms: Some(3),
        warmup_duration_ms: Some(2),
        runtime_reused: Some(false),
        lifecycle_decision_reason: Some("runtime_ready".to_string()),
        active: true,
        last_error: None,
    };
    let live_embedding_runtime_snapshot = inference::RuntimeLifecycleSnapshot {
        runtime_id: Some("llama_cpp_embedding".to_string()),
        runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
        warmup_started_at_ms: Some(4),
        warmup_completed_at_ms: Some(7),
        warmup_duration_ms: Some(3),
        runtime_reused: Some(true),
        lifecycle_decision_reason: Some("runtime_reused".to_string()),
        active: true,
        last_error: None,
    };
    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime: None,
        embedding_runtime: None,
    };

    let projection = build_runtime_event_projection(
        None,
        None,
        None,
        None,
        None,
        None,
        &gateway_snapshot,
        Some(&live_embedding_runtime_snapshot),
        &gateway_mode_info,
        None,
    );

    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/main.gguf")
    );
    assert_eq!(
        projection.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        projection
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_instance_id.as_deref()),
        Some("llama-cpp-embedding-7")
    );
}

#[test]
fn build_workflow_execution_diagnostics_snapshot_uses_backend_owned_scheduler_and_runtime_facts() {
    let registry = RuntimeRegistry::new();
    let snapshot =
        build_workflow_execution_diagnostics_snapshot(WorkflowExecutionDiagnosticsInput {
            runtime_registry: Some(&registry),
            scheduler_snapshot: &WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-123".to_string()),
                session_id: "session-123".to_string(),
                trace_execution_id: Some("exec-456".to_string()),
                session: WorkflowSessionSummary {
                    session_id: "session-123".to_string(),
                    workflow_id: "wf-123".to_string(),
                    session_kind: WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                    state: WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                },
                items: vec![WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-456".to_string()),
                    enqueued_at_ms: Some(11),
                    dequeued_at_ms: Some(12),
                    priority: 0,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
            },
            captured_at_ms: 999,
            runtime_capabilities: Some(WorkflowCapabilitiesResponse {
                max_input_bindings: 4,
                max_output_targets: 2,
                max_value_bytes: 1000,
                runtime_requirements: pantograph_workflow_service::WorkflowRuntimeRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: None,
                    estimation_confidence: "high".to_string(),
                    required_models: vec!["model-a".to_string()],
                    required_backends: vec!["pytorch".to_string()],
                    required_extensions: Vec::new(),
                },
                models: Vec::new(),
                runtime_capabilities: Vec::new(),
            }),
            runtime_error: Some("runtime capability probe failed".to_string()),
            trace_runtime_metrics_override: Some(WorkflowTraceRuntimeMetrics {
                runtime_id: Some("pytorch".to_string()),
                observed_runtime_ids: vec!["pytorch".to_string(), "diffusers".to_string()],
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                model_target: Some("/models/sidecar.safetensors".to_string()),
                warmup_started_at_ms: Some(5),
                warmup_completed_at_ms: Some(9),
                warmup_duration_ms: Some(4),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            }),
            runtime_snapshot_override: Some(&inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("PyTorch".to_string()),
                runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
                warmup_started_at_ms: Some(5),
                warmup_completed_at_ms: Some(9),
                warmup_duration_ms: Some(4),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            gateway_snapshot: &inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
                active: true,
                last_error: None,
            },
            embedding_runtime_snapshot: Some(&inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama_cpp_embedding".to_string()),
                runtime_instance_id: Some("embed-1".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("runtime_reused".to_string()),
                active: true,
                last_error: None,
            }),
            gateway_mode_info: &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/main.gguf".to_string()),
                embedding_model_target: Some("/models/embed.gguf".to_string()),
                active_runtime: None,
                embedding_runtime: None,
            },
            runtime_model_target_override: Some("/models/sidecar.safetensors"),
        });

    assert_eq!(snapshot.scheduler.workflow_id.as_deref(), Some("wf-123"));
    assert_eq!(snapshot.scheduler.trace_execution_id, "exec-456");
    assert_eq!(snapshot.scheduler.session_id, "session-123");
    assert_eq!(snapshot.scheduler.captured_at_ms, 999);
    assert_eq!(snapshot.runtime.workflow_id, "wf-123");
    assert_eq!(snapshot.runtime.trace_execution_id, "exec-456");
    assert_eq!(snapshot.runtime.captured_at_ms, 999);
    assert_eq!(
        snapshot.runtime.error.as_deref(),
        Some("runtime capability probe failed")
    );
    assert_eq!(
        snapshot.runtime.active_model_target.as_deref(),
        Some("/models/sidecar.safetensors")
    );
    assert_eq!(
        snapshot
            .runtime
            .embedding_runtime_snapshot
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("llama_cpp_embedding")
    );
    assert_eq!(
        snapshot.runtime.trace_runtime_metrics.observed_runtime_ids,
        vec!["pytorch".to_string(), "diffusers".to_string()]
    );

    let registry_runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("execution override should reconcile into registry");
    assert_eq!(
        registry_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:default")
    );
}

#[test]
fn build_runtime_event_projection_with_registry_override_reconciles_execution_runtime() {
    let registry = RuntimeRegistry::new();
    let projection = build_runtime_event_projection_with_registry_override(
        Some(&registry),
        None,
        None,
        None,
        None,
        None,
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:default".to_string()),
            warmup_started_at_ms: None,
            warmup_completed_at_ms: None,
            warmup_duration_ms: None,
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        &inference::RuntimeLifecycleSnapshot::default(),
        None,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: None,
            embedding_runtime: None,
        },
        Some("/models/sidecar.safetensors"),
    );

    let runtime = registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("execution runtime should be reconciled into the registry");

    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/sidecar.safetensors")
    );
    assert_eq!(runtime.models.len(), 1);
    assert_eq!(runtime.models[0].model_id, "/models/sidecar.safetensors");
    assert_eq!(
        runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:default")
    );
}

#[test]
fn build_runtime_event_projection_with_registry_reconciliation_replays_stored_runtime_into_registry()
 {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    let projection = build_runtime_event_projection_with_registry_reconciliation(
        Some(&registry),
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:restored".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/restored.safetensors"),
        None,
        None,
        None,
        &inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-live".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        },
        None,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
        None,
    );

    assert_eq!(
        projection.active_runtime_snapshot.runtime_id.as_deref(),
        Some("PyTorch")
    );
    assert_eq!(
        projection.active_model_target.as_deref(),
        Some("/models/restored.safetensors")
    );

    let snapshot = registry.snapshot();
    let gateway_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("gateway runtime should remain present");
    assert_eq!(
        gateway_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );

    let restored_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("stored sidecar runtime should be replayed");
    assert_eq!(
        restored_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:restored")
    );
    assert_eq!(restored_runtime.models.len(), 1);
    assert_eq!(
        restored_runtime.models[0].model_id,
        "/models/restored.safetensors"
    );
}

#[test]
fn reconcile_runtime_registry_stored_projection_overrides_replays_non_live_runtime_snapshot() {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: Some("/models/embed.gguf".to_string()),
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: None,
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-live".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: None,
    };

    reconcile_runtime_registry_stored_projection_overrides(
        Some(&registry),
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("python-runtime:pytorch:restored".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/restored.safetensors"),
        None,
        &gateway_mode_info,
    );

    let snapshot = registry.snapshot();
    let gateway_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("gateway runtime should remain present");
    assert_eq!(
        gateway_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );

    let restored_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "pytorch")
        .expect("stored sidecar runtime should be replayed");
    assert_eq!(
        restored_runtime.runtime_instance_id.as_deref(),
        Some("python-runtime:pytorch:restored")
    );
    assert_eq!(restored_runtime.models.len(), 1);
    assert_eq!(
        restored_runtime.models[0].model_id,
        "/models/restored.safetensors"
    );
}

#[test]
fn reconcile_runtime_registry_stored_projection_overrides_skips_live_host_runtime_ids() {
    let registry = RuntimeRegistry::new();
    crate::runtime_registry::reconcile_runtime_registry_mode_info(
        &registry,
        &HostRuntimeModeSnapshot {
            backend_name: Some("llama.cpp".to_string()),
            backend_key: Some("llama_cpp".to_string()),
            active_model_target: Some("/models/main.gguf".to_string()),
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some("llama.cpp".to_string()),
                runtime_instance_id: Some("llama-main-live".to_string()),
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        },
    );

    let gateway_mode_info = HostRuntimeModeSnapshot {
        backend_name: Some("llama.cpp".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: None,
        active_runtime: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-live".to_string()),
            warmup_started_at_ms: Some(1),
            warmup_completed_at_ms: Some(2),
            warmup_duration_ms: Some(1),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime: None,
    };

    reconcile_runtime_registry_stored_projection_overrides(
        Some(&registry),
        Some(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-main-stale".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(12),
            warmup_duration_ms: Some(2),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        None,
        Some("/models/stale.gguf"),
        None,
        &gateway_mode_info,
    );

    let snapshot = registry.snapshot();
    let live_runtime = snapshot
        .runtimes
        .iter()
        .find(|runtime| runtime.runtime_id == "llama_cpp")
        .expect("live gateway runtime should remain present");
    assert_eq!(
        live_runtime.runtime_instance_id.as_deref(),
        Some("llama-main-live")
    );
    assert_eq!(live_runtime.models.len(), 1);
    assert_eq!(live_runtime.models[0].model_id, "/models/main.gguf");
}
