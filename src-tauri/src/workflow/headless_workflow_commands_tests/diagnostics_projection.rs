use super::*;

#[test]
fn workflow_diagnostics_snapshot_projection_joins_backend_scheduler_and_runtime_data() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    let projection = workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "queue-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Running,
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
        Vec::new(),
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
        projection.scheduler.workflow_run_id.as_deref(),
        Some("run-1")
    );
    let trace = projection.runs_by_id.get("run-1").expect("joined trace");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.nodes.len(), 0);
}

#[test]
fn workflow_diagnostics_snapshot_projection_preserves_scheduler_runtime_registry_diagnostics() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    let projection = workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "queue-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: None,
                priority: 5,
                queue_position: Some(0),
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Pending,
            }],
            diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                loaded_session_count: 1,
                max_loaded_sessions: 2,
                reclaimable_loaded_session_count: 1,
                runtime_capacity_pressure:
                    pantograph_workflow_service::WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                active_run_blocks_admission: false,
                next_admission_workflow_run_id: Some("queue-1".to_string()),
                next_admission_bypassed_workflow_run_id: None,
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
        Vec::new(),
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
            next_warmup_decision: Some(WorkflowSchedulerRuntimeWarmupDecision::ReuseLoadedRuntime,),
            next_warmup_reason: Some(WorkflowSchedulerRuntimeWarmupReason::LoadedInstanceReady,),
        })
    );
}

#[test]
fn stored_runtime_trace_metrics_prefers_latest_recorded_trace() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
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
        Vec::new(),
        120,
    );

    let metrics = stored_runtime_trace_metrics(&diagnostics_store, Some("session-1"), Some("wf-1"))
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

    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
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
        Vec::new(),
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

    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
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
        Vec::new(),
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

    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
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
        Vec::new(),
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

    let projection = workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
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
        Vec::new(),
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
    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "queue-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Running,
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
        Vec::new(),
        120,
    );

    let projection = workflow_projection!(
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
        Vec::new(),
        130,
    );

    assert_eq!(projection.runtime.workflow_id, None);
    assert_eq!(projection.scheduler.session_id, None);
    assert_eq!(projection.scheduler.workflow_run_id, None);
    assert_eq!(projection.run_order, vec!["run-1".to_string()]);
}

#[test]
fn workflow_clear_diagnostics_history_response_preserves_backend_snapshots() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());

    workflow_projection!(
        &diagnostics_store,
        Some("session-1".to_string()),
        Some("wf-1".to_string()),
        Some("Workflow 1".to_string()),
        Some(Ok(WorkflowSchedulerSnapshotResponse {
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            workflow_run_id: Some("run-1".to_string()),
            session: running_session_summary(),
            items: vec![WorkflowExecutionSessionQueueItem {
                workflow_run_id: "queue-1".to_string(),

                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: WorkflowExecutionSessionQueueItemStatus::Running,
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
        Vec::new(),
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
        projection.scheduler.workflow_run_id.as_deref(),
        Some("run-1")
    );
}
