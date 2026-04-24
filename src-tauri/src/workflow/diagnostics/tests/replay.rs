use super::*;

#[test]
fn clear_history_preserves_runtime_and_scheduler_snapshots() {
    let store = WorkflowDiagnosticsStore::default();
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-1".to_string()),
        captured_at_ms: 2_000,
        ..Default::default()
    });
    store.update_scheduler_snapshot(WorkflowSchedulerSnapshotUpdate {
        workflow_id: Some("wf-1".to_string()),
        session_id: Some("exec-1".to_string()),
        captured_at_ms: 2_100,
        ..Default::default()
    });

    let snapshot = store.clear_history();

    assert!(snapshot.runs_by_id.is_empty());
    assert!(snapshot.run_order.is_empty());
    assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("exec-1"));
}

#[test]
fn clear_history_reconciles_restarted_backend_trace_and_runtime_snapshots() {
    let store = WorkflowDiagnosticsStore::default();
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "stale-exec".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeStarted {
            node_id: "llm-1".to_string(),
            node_type: "llm-inference".to_string(),
            execution_id: "stale-exec".to_string(),
        },
        1_010,
    );

    let cleared = store.clear_history();
    assert!(cleared.runs_by_id.is_empty());
    assert!(cleared.run_order.is_empty());

    let projection =
        store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
            workflow_id: Some("wf-1".to_string()),
            execution_id: "restored-exec".to_string(),
            session_id: "session-1".to_string(),
            captured_at_ms: 2_000,
            session: Some(
                pantograph_workflow_service::WorkflowExecutionSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: WorkflowExecutionSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                    queued_runs: 1,
                    run_count: 1,
                },
            ),
            items: vec![pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("restored-exec".to_string()),
            enqueued_at_ms: Some(1_950),
            dequeued_at_ms: Some(1_980),
            priority: 5,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowExecutionSessionQueueItemStatus::Running,
        }],
            diagnostics: None,
            error: None,
        });

    assert_eq!(projection.run_order, vec!["restored-exec".to_string()]);
    assert!(!projection.runs_by_id.contains_key("stale-exec"));
    assert_eq!(
        projection.scheduler.trace_execution_id.as_deref(),
        Some("restored-exec")
    );

    let runtime_projection = store.update_runtime_snapshot(WorkflowRuntimeSnapshotUpdate {
        workflow_id: Some("wf-1".to_string()),
        active_model_target: Some("/models/restarted.gguf".to_string()),
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-restored".to_string()),
            warmup_started_at_ms: Some(1_900),
            warmup_completed_at_ms: Some(1_940),
            warmup_duration_ms: Some(40),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
            active: true,
            last_error: None,
        }),
        captured_at_ms: 2_010,
        ..Default::default()
    });

    assert_eq!(
        runtime_projection.run_order,
        vec!["restored-exec".to_string()]
    );
    assert_eq!(
        runtime_projection.runtime.active_model_target.as_deref(),
        Some("/models/restarted.gguf")
    );
    assert_eq!(
        runtime_projection
            .runtime
            .active_runtime
            .as_ref()
            .and_then(|runtime| runtime.runtime_id.as_deref()),
        Some("llama_cpp")
    );

    let trace = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            execution_id: Some("restored-exec".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(true),
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("restored trace");

    assert_eq!(trace.execution_id, "restored-exec");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.queue.enqueued_at_ms, Some(1_950));
    assert_eq!(trace.queue.dequeued_at_ms, Some(1_980));
}

#[test]
fn restarted_run_clears_stale_overlay_history_and_node_state() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-1",
        Some("wf-1".to_string()),
        Some("Retry Workflow".to_string()),
    );
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeStarted {
            node_id: "llm-1".to_string(),
            node_type: "llm-inference".to_string(),
            execution_id: "exec-1".to_string(),
        },
        1_010,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeProgress {
            node_id: "llm-1".to_string(),
            progress: 0.5,
            message: Some("halfway".to_string()),
            detail: None,
            execution_id: "exec-1".to_string(),
        },
        1_020,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-1".to_string(),
            outputs: HashMap::new(),
            execution_id: "exec-1".to_string(),
        },
        1_100,
    );

    let restarted = store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        2_000,
    );

    let run = restarted.runs_by_id.get("exec-1").expect("restarted run");
    assert_eq!(run.status, DiagnosticsRunStatus::Running);
    assert_eq!(run.started_at_ms, 2_000);
    assert_eq!(run.event_count, 1);
    assert_eq!(run.events.len(), 1);
    assert_eq!(run.events[0].event_type, "Started");
    assert!(run.nodes.is_empty());
}

#[test]
fn restarted_cancelled_run_clears_stale_overlay_history_and_node_state() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-1",
        Some("wf-1".to_string()),
        Some("Retry Workflow".to_string()),
    );
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeProgress {
            node_id: "llm-1".to_string(),
            progress: 0.5,
            message: Some("halfway".to_string()),
            detail: None,
            execution_id: "exec-1".to_string(),
        },
        1_020,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Cancelled {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            error: "workflow run cancelled during execution".to_string(),
        },
        1_100,
    );

    let restarted = store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        2_000,
    );

    let run = restarted.runs_by_id.get("exec-1").expect("restarted run");
    assert_eq!(run.status, DiagnosticsRunStatus::Running);
    assert_eq!(run.started_at_ms, 2_000);
    assert_eq!(run.event_count, 1);
    assert_eq!(run.events.len(), 1);
    assert_eq!(run.events[0].event_type, "Started");
    assert!(run.nodes.is_empty());
}

#[test]
fn node_progress_detail_is_exposed_in_diagnostics_snapshot() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-1",
        Some("wf-1".to_string()),
        Some("KV Workflow".to_string()),
    );
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeProgress {
            node_id: "llm-1".to_string(),
            progress: 0.0,
            message: Some("kv cache restored".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::RestoreInput,
                    outcome: node_engine::KvCacheEventOutcome::Hit,
                    cache_id: Some("cache-1".to_string()),
                    backend_key: Some("llamacpp".to_string()),
                    reuse_source: Some("llamacpp_slot".to_string()),
                    token_count: Some(48),
                    reason: Some("restored_input_handle".to_string()),
                },
            )),
            execution_id: "exec-1".to_string(),
        },
        1_020,
    );

    let snapshot = store.snapshot();
    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    let node = run.nodes.get("llm-1").expect("node trace");
    match node.last_progress_detail.as_ref() {
        Some(node_engine::TaskProgressDetail::KvCache(detail)) => {
            assert_eq!(detail.outcome, node_engine::KvCacheEventOutcome::Hit);
            assert_eq!(detail.cache_id.as_deref(), Some("cache-1"));
        }
        other => panic!("unexpected progress detail: {other:?}"),
    }
    assert_eq!(node.last_progress, None);
}

#[test]
fn restarted_run_clears_stale_graph_mutation_overlay_state() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-1",
        Some("wf-1".to_string()),
        Some("Retry Workflow".to_string()),
    );
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 2,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::GraphModified {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            graph: None,
            dirty_tasks: vec!["llm-1".to_string()],
            memory_impact: Some(
                node_engine::GraphMemoryImpactSummary::fallback_full_invalidation(
                    ["llm-1"],
                    "graph_changed",
                ),
            ),
        },
        1_020,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            task_ids: vec!["llm-1".to_string()],
        },
        1_040,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-1".to_string(),
            outputs: HashMap::new(),
            execution_id: "exec-1".to_string(),
        },
        1_100,
    );

    let restarted = store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 2,
            execution_id: "exec-1".to_string(),
        },
        2_000,
    );

    let run = restarted.runs_by_id.get("exec-1").expect("restarted run");
    assert_eq!(run.status, DiagnosticsRunStatus::Running);
    assert_eq!(run.started_at_ms, 2_000);
    assert_eq!(run.event_count, 1);
    assert_eq!(run.events.len(), 1);
    assert_eq!(run.events[0].event_type, "Started");
    assert!(run.nodes.is_empty());
    assert!(run.last_dirty_tasks.is_empty());
    assert!(run.last_incremental_task_ids.is_empty());
    assert_eq!(run.last_graph_memory_impact, None);
}

#[test]
fn replayed_backend_scheduler_and_runtime_snapshots_do_not_duplicate_trace() {
    let store = WorkflowDiagnosticsStore::default();

    store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
        workflow_id: Some("wf-1".to_string()),
        execution_id: "exec-1".to_string(),
        session_id: "session-1".to_string(),
        captured_at_ms: 1_000,
        session: Some(
            pantograph_workflow_service::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
                state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                queued_runs: 1,
                run_count: 1,
            },
        ),
        items: vec![
            pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("exec-1".to_string()),
                enqueued_at_ms: Some(900),
                dequeued_at_ms: Some(930),
                priority: 1,
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
    store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        captured_at_ms: 1_010,
        capabilities: None,
        trace_runtime_metrics: pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp".to_string()),
            observed_runtime_ids: vec!["llama.cpp".to_string()],
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            model_target: Some("/models/first.gguf".to_string()),
            warmup_started_at_ms: Some(880),
            warmup_completed_at_ms: Some(890),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("runtime_ready".to_string()),
        },
        active_model_target: Some("/models/first.gguf".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: Some(880),
            warmup_completed_at_ms: Some(890),
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

    store.record_scheduler_snapshot(WorkflowSchedulerSnapshotRecord {
        workflow_id: Some("wf-1".to_string()),
        execution_id: "exec-1".to_string(),
        session_id: "session-1".to_string(),
        captured_at_ms: 1_100,
        session: Some(
            pantograph_workflow_service::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                keep_alive: true,
                state: pantograph_workflow_service::WorkflowExecutionSessionState::Running,
                queued_runs: 1,
                run_count: 1,
            },
        ),
        items: vec![
            pantograph_workflow_service::WorkflowExecutionSessionQueueItem {
                queue_id: "queue-1".to_string(),
                run_id: Some("exec-1".to_string()),
                enqueued_at_ms: Some(900),
                dequeued_at_ms: Some(940),
                priority: 1,
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

    store.record_runtime_snapshot(WorkflowRuntimeSnapshotRecord {
        workflow_id: "wf-1".to_string(),
        execution_id: "exec-1".to_string(),
        captured_at_ms: 1_120,
        capabilities: None,
        trace_runtime_metrics: pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama.cpp".to_string()),
            observed_runtime_ids: vec!["llama.cpp".to_string(), "llama_cpp".to_string()],
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            model_target: Some("/models/replayed.gguf".to_string()),
            warmup_started_at_ms: Some(880),
            warmup_completed_at_ms: Some(890),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
        },
        active_model_target: Some("/models/replayed.gguf".to_string()),
        embedding_model_target: None,
        active_runtime_snapshot: Some(inference::RuntimeLifecycleSnapshot {
            runtime_id: Some("llama.cpp".to_string()),
            runtime_instance_id: Some("llama-cpp-1".to_string()),
            warmup_started_at_ms: Some(880),
            warmup_completed_at_ms: Some(890),
            warmup_duration_ms: Some(10),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("runtime_reused".to_string()),
            active: true,
            last_error: None,
        }),
        embedding_runtime_snapshot: None,
        managed_runtimes: Vec::new(),
        error: None,
    });

    let trace_snapshot = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            execution_id: Some("exec-1".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(true),
        })
        .expect("trace snapshot");

    assert_eq!(trace_snapshot.traces.len(), 1);
    let trace = &trace_snapshot.traces[0];
    assert_eq!(trace.execution_id, "exec-1");
    assert_eq!(trace.session_id.as_deref(), Some("session-1"));
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.queue.dequeued_at_ms, Some(930));
    assert_eq!(
        trace.runtime.observed_runtime_ids,
        vec!["llama.cpp".to_string(), "llama_cpp".to_string()]
    );
    assert_eq!(
        trace.runtime.model_target.as_deref(),
        Some("/models/replayed.gguf")
    );
}
