use super::*;

#[test]
fn runtime_and_scheduler_snapshots_are_backend_owned() {
    let store = WorkflowDiagnosticsStore::default();
    store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-runtime".to_string()),
        capabilities: Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                required_backends: vec!["llama-cpp".to_string()],
                required_extensions: vec!["kv_cache".to_string()],
            },
            models: vec![pantograph_workflow_service::WorkflowCapabilityModel {
                model_id: "model-a".to_string(),
                model_revision_or_hash: None,
                model_type: None,
                node_ids: vec!["node-a".to_string()],
                roles: vec!["generation".to_string()],
            }],
            runtime_capabilities: Vec::new(),
        }),
        last_error: None,
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: Some(4_900),
            warmup_completed_at_ms: Some(5_000),
            warmup_duration_ms: Some(100),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-2".to_string()),
            warmup_started_at_ms: Some(4_800),
            warmup_completed_at_ms: Some(4_850),
            warmup_duration_ms: Some(50),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        }),
        managed_runtimes: Vec::new(),
        captured_at_ms: 5_000,
    });
    let snapshot =
        store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
            workflow_id: Some("wf-runtime".to_string()),
            session_id: Some("session-1".to_string()),
            session: Some(
                pantograph_workflow_service::WorkflowExecutionSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-runtime".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                    queued_runs: 1,
                    run_count: 3,
                },
            ),
            items: vec![pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
            workflow_run_id: "queue-1".to_string(),

            enqueued_at_ms: None,
            dequeued_at_ms: None,
            priority: 10,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
        }],
            diagnostics: None,
            last_error: None,
            captured_at_ms: 6_000,
        });

    assert!(snapshot.runs_by_id.is_empty());
    assert!(snapshot.run_order.is_empty());
    assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-runtime"));
    assert_eq!(snapshot.runtime.max_input_bindings, Some(4));
    assert_eq!(
        snapshot.runtime.active_model_target.as_deref(),
        Some("/models/main.gguf")
    );
    assert_eq!(
        snapshot.runtime.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("llama_cpp")
    );
    assert_eq!(
        snapshot
            .runtime
            .embedding_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("llama.cpp.embedding")
    );
    assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("session-1"));
    assert_eq!(
        snapshot
            .scheduler
            .session
            .as_ref()
            .map(|session| session.session_kind.clone()),
        Some(WorkflowExecutionSessionKind::Workflow)
    );
    assert_eq!(snapshot.scheduler.items.len(), 1);
}

#[test]
fn workflow_diagnostics_projection_preserves_scheduler_snapshot_diagnostics() {
    let store = WorkflowDiagnosticsStore::default();
    let diagnostics = pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics {
        loaded_session_count: 1,
        max_loaded_sessions: 2,
        reclaimable_loaded_session_count: 1,
        runtime_capacity_pressure:
            pantograph_workflow_service::WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
        active_run_blocks_admission: true,
        next_admission_workflow_run_id: Some("queue-1".to_string()),
        next_admission_bypassed_workflow_run_id: None,
        next_admission_after_runs: Some(1),
        next_admission_wait_ms: None,
        next_admission_not_before_ms: None,
        next_admission_reason: Some(
            pantograph_workflow_service::WorkflowSchedulerDecisionReason::WarmSessionReused,
        ),
        runtime_registry: None,
    };

    let snapshot = store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
        workflow_id: Some("wf-runtime".to_string()),
        session_id: Some("session-1".to_string()),
        session: Some(
            pantograph_workflow_service::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-runtime".to_string(),
                session_kind: WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
                state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                queued_runs: 1,
                run_count: 3,
            },
        ),
        items: Vec::new(),
        diagnostics: Some(diagnostics.clone()),
        last_error: None,
        captured_at_ms: 6_000,
    });

    assert_eq!(snapshot.scheduler.diagnostics, Some(diagnostics));
}

#[test]
fn runtime_snapshot_preserves_managed_runtime_views() {
    let store = WorkflowDiagnosticsStore::default();
    let managed_runtime = pantograph_embedded_runtime::ManagedRuntimeManagerRuntimeView {
        id: inference::ManagedBinaryId::LlamaCpp,
        display_name: "llama.cpp".to_string(),
        install_state: inference::ManagedBinaryInstallState::Missing,
        readiness_state: inference::ManagedRuntimeReadinessState::Downloading,
        available: false,
        can_install: true,
        can_remove: false,
        missing_files: vec!["llama-server-x86_64-unknown-linux-gnu".to_string()],
        unavailable_reason: Some("binary download still in progress".to_string()),
        versions: Vec::new(),
        selection: inference::ManagedRuntimeSelectionState {
            selected_version: Some("b8248".to_string()),
            active_version: None,
            default_version: Some("b8248".to_string()),
        },
        active_job: Some(inference::ManagedRuntimeJobStatus {
            state: inference::ManagedRuntimeJobState::Downloading,
            status: "downloading".to_string(),
            current: 128,
            total: 512,
            resumable: true,
            cancellable: true,
            error: None,
        }),
        job_artifact: None,
        install_history: Vec::new(),
    };

    let snapshot = store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-runtime".to_string()),
        capabilities: None,
        last_error: Some("runtime not ready".to_string()),
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: None,
        embedding_runtime_snapshot: None,
        managed_runtimes: vec![managed_runtime],
        captured_at_ms: 5_000,
    });

    assert_eq!(snapshot.runtime.managed_runtimes.len(), 1);
    let runtime = &snapshot.runtime.managed_runtimes[0];
    assert_eq!(runtime.id, inference::ManagedBinaryId::LlamaCpp);
    assert_eq!(
        runtime.readiness_state,
        inference::ManagedRuntimeReadinessState::Downloading
    );
    assert_eq!(
        runtime.active_job.as_ref().map(|job| job.state),
        Some(inference::ManagedRuntimeJobState::Downloading)
    );
    assert_eq!(
        runtime.missing_files,
        vec!["llama-server-x86_64-unknown-linux-gnu".to_string()]
    );
}

#[test]
fn runtime_snapshot_falls_back_to_selected_capability_when_lifecycle_is_absent() {
    let store = WorkflowDiagnosticsStore::default();
    let snapshot = store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-runtime".to_string()),
        capabilities: Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "pytorch".to_string(),
                display_name: "PyTorch (Python sidecar)".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: true,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        last_error: None,
        active_model_target: Some("black-forest-labs/flux.1-schnell".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: None,
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        captured_at_ms: 5_000,
    });

    assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-runtime"));
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("pytorch")
    );
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.lifecycle_decision_reason.as_deref()),
        Some("selected_runtime_reported")
    );
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .map(|runtime| runtime.active),
        Some(false)
    );
}

#[test]
fn runtime_snapshot_matches_required_backend_alias_when_selected_runtime_is_absent() {
    let store = WorkflowDiagnosticsStore::default();
    let snapshot = store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-onnx".to_string()),
        capabilities: Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                required_backends: vec!["onnxruntime".to_string()],
                required_extensions: Vec::new(),
            },
            models: Vec::new(),
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "onnx-runtime".to_string(),
                display_name: "ONNX Runtime (Python sidecar)".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: false,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["ONNX Runtime".to_string(), "onnx-runtime".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        last_error: None,
        active_model_target: Some("kitten-tts".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: None,
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        captured_at_ms: 5_000,
    });

    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("onnx-runtime")
    );
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.lifecycle_decision_reason.as_deref()),
        Some("required_runtime_reported")
    );
}

#[test]
fn runtime_snapshot_normalizes_selected_capability_runtime_id_when_lifecycle_is_absent() {
    let store = WorkflowDiagnosticsStore::default();
    let snapshot = store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-runtime".to_string()),
        capabilities: Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
            runtime_capabilities: vec![pantograph_workflow_service::WorkflowRuntimeCapability {
                runtime_id: "PyTorch".to_string(),
                display_name: "PyTorch".to_string(),
                install_state:
                    pantograph_workflow_service::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: pantograph_workflow_service::WorkflowRuntimeSourceKind::System,
                selected: true,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        last_error: None,
        active_model_target: Some("black-forest-labs/flux.1-schnell".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: None,
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        captured_at_ms: 5_000,
    });

    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("pytorch")
    );
}

#[test]
fn runtime_snapshot_event_carries_runtime_lifecycle_into_trace_store() {
    let store = WorkflowDiagnosticsStore::default();
    let snapshot = store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "wf-runtime".to_string(),
        workflow_run_id: "exec-runtime".to_string(),
        captured_at_ms: 5_000,
        capabilities: None,
        trace_runtime_metrics: pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp".to_string()),
            observed_runtime_ids: vec!["llama.cpp".to_string()],
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            model_target: Some("/models/main.gguf".to_string()),
            warmup_started_at_ms: Some(4_900),
            warmup_completed_at_ms: Some(5_000),
            warmup_duration_ms: Some(100),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
        },
        active_model_target: Some("/models/main.gguf".to_string()),
        embedding_model_target: Some("/models/embed.gguf".to_string()),
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: Some(4_900),
            warmup_completed_at_ms: Some(5_000),
            warmup_duration_ms: Some(100),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp.embedding".to_string()),
            runtime_instance_id: Some("llama-cpp-embedding-7".to_string()),
            warmup_started_at_ms: Some(4_700),
            warmup_completed_at_ms: Some(4_760),
            warmup_duration_ms: Some(60),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        }),
        managed_runtimes: Vec::new(),
        error: None,
    });

    let trace = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("exec-runtime".to_string()),
            session_id: None,
            workflow_id: None,

            include_completed: None,
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("runtime trace");

    assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama.cpp"));
    assert_eq!(
        trace.runtime.observed_runtime_ids,
        vec!["llama.cpp".to_string()]
    );
    assert_eq!(
        trace.runtime.runtime_instance_id.as_deref(),
        Some("llama-cpp-1")
    );
    assert_eq!(
        trace.runtime.model_target.as_deref(),
        Some("/models/main.gguf")
    );
    assert_eq!(trace.runtime.warmup_started_at_ms, Some(4_900));
    assert_eq!(trace.runtime.warmup_completed_at_ms, Some(5_000));
    assert_eq!(trace.runtime.warmup_duration_ms, Some(100));
    assert_eq!(trace.runtime.runtime_reused, Some(false));
    assert_eq!(
        trace.runtime.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
    assert_eq!(
        snapshot
            .runs_by_id
            .get("exec-runtime")
            .and_then(|run| run.runtime.model_target.as_deref()),
        Some("/models/main.gguf")
    );
    assert_eq!(
        snapshot.runtime.active_model_target.as_deref(),
        Some("/models/main.gguf")
    );
    assert_eq!(
        snapshot.runtime.embedding_model_target.as_deref(),
        Some("/models/embed.gguf")
    );
    assert_eq!(
        snapshot
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_instance_id.as_deref()),
        Some("llama-cpp-1")
    );
    assert_eq!(
        snapshot
            .runtime
            .embedding_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_instance_id.as_deref()),
        Some("llama-cpp-embedding-7")
    );
}

#[test]
fn diagnostics_runtime_lifecycle_snapshot_normalizes_known_runtime_aliases() {
    let snapshot =
        DiagnosticsRuntimeLifecycleSnapshot::from(&inference::RuntimeLifecycleSnapshot {
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
}

#[test]
fn diagnostics_runtime_lifecycle_snapshot_infers_default_lifecycle_reason() {
    let snapshot =
        DiagnosticsRuntimeLifecycleSnapshot::from(&inference::RuntimeLifecycleSnapshot {
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

    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[test]
fn diagnostics_runtime_lifecycle_snapshot_infers_start_failure_reason() {
    let snapshot =
        DiagnosticsRuntimeLifecycleSnapshot::from(&inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: None,
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(25),
            warmup_duration_ms: Some(15),
            runtime_reused: None,
            lifecycle_decision_reason: None,
            active: false,
            last_error: Some("failed".to_string()),
        });

    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
}

#[test]
fn inference_runtime_lifecycle_snapshot_from_diagnostics_infers_default_reason() {
    let snapshot =
        inference::RuntimeLifecycleSnapshot::from(&DiagnosticsRuntimeLifecycleSnapshot {
            runtime_id: Some("llama_cpp".to_string()),
            runtime_instance_id: Some("runtime-1".to_string()),
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: None,
            active: true,
            last_error: None,
        });

    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[test]
fn inference_runtime_lifecycle_snapshot_from_diagnostics_infers_start_failure_reason() {
    let snapshot =
        inference::RuntimeLifecycleSnapshot::from(&DiagnosticsRuntimeLifecycleSnapshot {
            runtime_id: Some("llama_cpp".to_string()),
            runtime_instance_id: None,
            warmup_started_at_ms: Some(10),
            warmup_completed_at_ms: Some(20),
            warmup_duration_ms: Some(10),
            runtime_reused: None,
            lifecycle_decision_reason: None,
            active: false,
            last_error: Some("failed".to_string()),
        });

    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
}

#[test]
fn inference_runtime_lifecycle_snapshot_from_diagnostics_canonicalizes_runtime_aliases() {
    let snapshot =
        inference::RuntimeLifecycleSnapshot::from(&DiagnosticsRuntimeLifecycleSnapshot {
            runtime_id: Some("PyTorch".to_string()),
            runtime_instance_id: Some("runtime-1".to_string()),
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

#[test]
fn scheduler_snapshot_event_carries_authoritative_queue_metrics_into_trace_store() {
    let store = WorkflowDiagnosticsStore::default();
    let projection =
        store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
            workflow_id: None,
            workflow_run_id: "edit-session-1".to_string(),
            session_id: "edit-session-1".to_string(),
            captured_at_ms: 5_000,
            session: Some(
                pantograph_workflow_service::WorkflowExecutionSessionSummary {
                    session_id: "edit-session-1".to_string(),
                    workflow_id: "edit-session-1".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Edit,
                    usage_profile: None,
                    keep_alive: false,
                    state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                    queued_runs: 1,
                    run_count: 2,
                },
            ),
            items: vec![pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
            workflow_run_id: "edit-session-1".to_string(),

            enqueued_at_ms: Some(4_750),
            dequeued_at_ms: Some(4_750),
            priority: 0,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
        }],
            diagnostics: None,
            error: None,
        });
    assert_eq!(
        projection.scheduler.workflow_run_id.as_deref(),
        Some("edit-session-1")
    );

    let trace = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("edit-session-1".to_string()),
            session_id: None,
            workflow_id: None,

            include_completed: None,
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("scheduler trace");

    assert_eq!(trace.session_id.as_deref(), Some("edit-session-1"));
    assert_eq!(
        trace.status,
        pantograph_workflow_service::WorkflowTraceStatus::Running
    );
    assert_eq!(trace.queue.enqueued_at_ms, Some(4_750));
    assert_eq!(trace.queue.dequeued_at_ms, Some(4_750));
    assert_eq!(trace.queue.queue_wait_ms, Some(0));
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_running_item")
    );
}

#[test]
fn scheduler_snapshot_event_carries_workflow_run_id_into_projection() {
    let store = WorkflowDiagnosticsStore::default();
    let projection =
        store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
            workflow_id: Some("wf-1".to_string()),
            workflow_run_id: "run-1".to_string(),
            session_id: "session-1".to_string(),
            captured_at_ms: 5_000,
            session: Some(
                pantograph_workflow_service::WorkflowExecutionSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                    queued_runs: 1,
                    run_count: 2,
                },
            ),
            items: vec![pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
            workflow_run_id: "queue-1".to_string(),

            enqueued_at_ms: Some(100),
            dequeued_at_ms: Some(110),
            priority: 5,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
        }],
            diagnostics: None,
            error: None,
        });

    assert_eq!(projection.scheduler.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(
        projection.scheduler.session_id.as_deref(),
        Some("session-1")
    );
    assert_eq!(
        projection.scheduler.workflow_run_id.as_deref(),
        Some("run-1")
    );
    let run = projection.runs_by_id.get("run-1").expect("run trace");
    assert_eq!(run.session_id.as_deref(), Some("session-1"));
}
