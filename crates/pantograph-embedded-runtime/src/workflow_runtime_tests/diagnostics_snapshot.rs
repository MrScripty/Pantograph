use super::*;

#[tokio::test]
async fn diagnostics_snapshot_with_registry_sync_reconciles_execution_runtime() {
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
                workflow_run_id: Some("exec-456".to_string()),
                session: WorkflowExecutionSessionSummary {
                    session_id: "session-123".to_string(),
                    workflow_id: "wf-123".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                    state: WorkflowExecutionSessionState::Running,
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
fn build_workflow_execution_diagnostics_snapshot_uses_backend_owned_scheduler_and_runtime_facts() {
    let registry = RuntimeRegistry::new();
    let snapshot =
        build_workflow_execution_diagnostics_snapshot(WorkflowExecutionDiagnosticsInput {
            runtime_registry: Some(&registry),
            scheduler_snapshot: &WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-123".to_string()),
                session_id: "session-123".to_string(),
                workflow_run_id: Some("exec-456".to_string()),
                session: WorkflowExecutionSessionSummary {
                    session_id: "session-123".to_string(),
                    workflow_id: "wf-123".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                    state: WorkflowExecutionSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                },
                items: vec![WorkflowExecutionSessionQueueItem {
                    workflow_run_id: "exec-456".to_string(),
                    enqueued_at_ms: Some(11),
                    dequeued_at_ms: Some(12),
                    priority: 0,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
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
    assert_eq!(
        snapshot.scheduler.workflow_run_id.as_deref(),
        Some("exec-456")
    );
    assert_eq!(snapshot.scheduler.session_id, "session-123");
    assert_eq!(snapshot.scheduler.captured_at_ms, 999);
    assert_eq!(snapshot.runtime.workflow_id, "wf-123");
    assert_eq!(
        snapshot.runtime.workflow_run_id.as_deref(),
        Some("exec-456")
    );
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
fn build_workflow_execution_diagnostics_snapshot_preserves_idle_no_run_state() {
    let snapshot =
        build_workflow_execution_diagnostics_snapshot(WorkflowExecutionDiagnosticsInput {
            runtime_registry: None,
            scheduler_snapshot: &WorkflowSchedulerSnapshotResponse {
                workflow_id: Some("wf-idle".to_string()),
                session_id: "session-idle".to_string(),
                workflow_run_id: None,
                session: WorkflowExecutionSessionSummary {
                    session_id: "session-idle".to_string(),
                    workflow_id: "wf-idle".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: false,
                    state: WorkflowExecutionSessionState::IdleLoaded,
                    queued_runs: 0,
                    run_count: 1,
                },
                items: Vec::new(),
                diagnostics: None,
            },
            captured_at_ms: 1_234,
            runtime_capabilities: None,
            runtime_error: None,
            trace_runtime_metrics_override: None,
            runtime_snapshot_override: None,
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
            embedding_runtime_snapshot: None,
            gateway_mode_info: &HostRuntimeModeSnapshot {
                backend_name: Some("llama.cpp".to_string()),
                backend_key: Some("llama_cpp".to_string()),
                active_model_target: Some("/models/main.gguf".to_string()),
                embedding_model_target: None,
                active_runtime: None,
                embedding_runtime: None,
            },
            runtime_model_target_override: None,
        });

    assert_eq!(snapshot.scheduler.workflow_run_id, None);
    assert_eq!(snapshot.scheduler.session_id, "session-idle");
    assert_eq!(snapshot.runtime.workflow_run_id, None);
    assert_eq!(snapshot.runtime.workflow_id, "wf-idle");
}
