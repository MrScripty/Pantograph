use super::*;

#[test]
fn workflow_trace_store_records_graph_reconciliation_facts() {
    let store = WorkflowTraceStore::new(10);

    let snapshot = store.record_event(
        &WorkflowTraceEvent::GraphModified {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            dirty_tasks: vec!["merge".to_string(), "output".to_string()],
            memory_impact: Some(node_engine::GraphMemoryImpactSummary {
                node_decisions: vec![
                    node_engine::NodeMemoryCompatibilitySnapshot {
                        node_id: "merge".to_string(),
                        compatibility:
                            node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh,
                        reason: Some("upstream input updated".to_string()),
                    },
                    node_engine::NodeMemoryCompatibilitySnapshot {
                        node_id: "output".to_string(),
                        compatibility:
                            node_engine::NodeMemoryCompatibility::FallbackFullInvalidation,
                        reason: Some("compatibility unknown".to_string()),
                    },
                ],
                fallback_to_full_invalidation: true,
            }),
        },
        180,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(
        trace.last_dirty_tasks,
        vec!["merge".to_string(), "output".to_string()]
    );
    assert_eq!(
        trace.last_graph_memory_impact,
        Some(node_engine::GraphMemoryImpactSummary {
            node_decisions: vec![
                node_engine::NodeMemoryCompatibilitySnapshot {
                    node_id: "merge".to_string(),
                    compatibility: node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh,
                    reason: Some("upstream input updated".to_string()),
                },
                node_engine::NodeMemoryCompatibilitySnapshot {
                    node_id: "output".to_string(),
                    compatibility: node_engine::NodeMemoryCompatibility::FallbackFullInvalidation,
                    reason: Some("compatibility unknown".to_string()),
                },
            ],
            fallback_to_full_invalidation: true,
        })
    );
}

#[test]
fn workflow_trace_store_waiting_nodes_capture_pause_duration() {
    let store = WorkflowTraceStore::new(10);

    store.record_event(
        &WorkflowTraceEvent::IncrementalExecutionStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            task_ids: vec!["node-1".to_string()],
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            node_type: Some("llm-inference".to_string()),
        },
        110,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::WaitingForInput {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_id: "node-1".to_string(),
        },
        140,
    );

    let trace = snapshot.traces.first().expect("trace");
    let node = trace.nodes.first().expect("node");
    assert_eq!(trace.status, WorkflowTraceStatus::Waiting);
    assert!(trace.waiting_for_input);
    assert_eq!(node.status, WorkflowTraceNodeStatus::Waiting);
    assert_eq!(node.ended_at_ms, Some(140));
    assert_eq!(node.duration_ms, Some(30));
}

#[test]
fn workflow_trace_store_ignores_duplicate_node_failed_events() {
    let store = WorkflowTraceStore::new(10);

    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 1,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            node_type: Some("llm-inference".to_string()),
        },
        110,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeFailed {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            error: "boom".to_string(),
        },
        140,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::NodeFailed {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            error: "boom".to_string(),
        },
        170,
    );

    let trace = snapshot.traces.first().expect("trace");
    let node = trace.nodes.first().expect("node");
    assert_eq!(trace.event_count, 3);
    assert_eq!(node.event_count, 2);
    assert_eq!(node.status, WorkflowTraceNodeStatus::Failed);
    assert_eq!(node.ended_at_ms, Some(140));
    assert_eq!(node.duration_ms, Some(30));
    assert_eq!(node.last_error.as_deref(), Some("boom"));
}

#[test]
fn workflow_trace_store_prefers_matching_queue_items_over_session_backlog() {
    let store = WorkflowTraceStore::new(10);
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-target".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 200,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: None,
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 2,
                run_count: 3,
            }),
            items: vec![
                crate::workflow::WorkflowExecutionSessionQueueItem {
                    workflow_run_id: "other-run".to_string(),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(150),
                    priority: 10,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
                },
                crate::workflow::WorkflowExecutionSessionQueueItem {
                    workflow_run_id: "exec-target".to_string(),
                    enqueued_at_ms: Some(180),
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Pending,
                },
            ],
            diagnostics: None,
            error: None,
        },
        200,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.status, WorkflowTraceStatus::Queued);
    assert_eq!(trace.queue.enqueued_at_ms, Some(180));
    assert_eq!(trace.queue.dequeued_at_ms, None);
    assert_eq!(trace.queue.queue_wait_ms, None);
    assert_eq!(
        trace.queue.scheduler_admission_outcome.as_deref(),
        Some("queued")
    );
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_pending_item")
    );
}

#[test]
fn workflow_trace_store_preserves_enqueue_time_when_first_snapshot_is_running() {
    let store = WorkflowTraceStore::new(10);
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "edit-session-1".to_string(),
            workflow_id: None,
            session_id: "edit-session-1".to_string(),
            captured_at_ms: 5_000,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "edit-session-1".to_string(),
                workflow_id: "edit-session-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Edit,
                usage_profile: None,
                attribution: None,
                keep_alive: false,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 1,
                run_count: 2,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "edit-session-1".to_string(),
                enqueued_at_ms: Some(4_750),
                dequeued_at_ms: Some(4_750),
                priority: 0,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        5_000,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.queue.enqueued_at_ms, Some(4_750));
    assert_eq!(trace.queue.dequeued_at_ms, Some(4_750));
    assert_eq!(trace.queue.queue_wait_ms, Some(0));
    assert_eq!(
        trace.queue.scheduler_admission_outcome.as_deref(),
        Some("admitted")
    );
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_running_item")
    );
}

#[test]
fn workflow_trace_store_does_not_synthesize_queue_timing_from_snapshot_capture_time() {
    let store = WorkflowTraceStore::new(10);
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 200,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: None,
                dequeued_at_ms: None,
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        200,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.queue.enqueued_at_ms, None);
    assert_eq!(trace.queue.dequeued_at_ms, None);
    assert_eq!(trace.queue.queue_wait_ms, None);
    assert_eq!(
        trace.queue.scheduler_admission_outcome.as_deref(),
        Some("admitted")
    );
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_running_item")
    );
}

#[test]
fn workflow_trace_store_does_not_match_unrelated_queue_item_by_session_id() {
    let store = WorkflowTraceStore::new(10);
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-target".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 200,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 2,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "other-run".to_string(),
                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(120),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: Some(
                    crate::workflow::WorkflowSchedulerDecisionReason::WarmSessionReused,
                ),
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        200,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.queue.enqueued_at_ms, None);
    assert_eq!(trace.queue.dequeued_at_ms, None);
    assert_eq!(trace.queue.queue_wait_ms, None);
    assert_eq!(
        trace.queue.scheduler_admission_outcome.as_deref(),
        Some("admitted")
    );
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("session_running")
    );
}

#[test]
fn workflow_trace_store_prefers_backend_scheduler_decision_reason_from_queue_item() {
    let store = WorkflowTraceStore::new(10);
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 200,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(120),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: Some(
                    crate::workflow::WorkflowSchedulerDecisionReason::WarmSessionReused,
                ),
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        200,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("warm_session_reused")
    );
}

#[test]
fn workflow_trace_store_selects_runtime_metrics_when_trace_match_is_unique() {
    let store = WorkflowTraceStore::new(10);
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 1,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 110,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: Some("interactive".to_string()),
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(110),
                priority: 5,
                queue_position: Some(0),
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        110,
    );
    store.record_event(
        &WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            captured_at_ms: 120,
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("/models/one.gguf".to_string()),
                warmup_started_at_ms: Some(111),
                warmup_completed_at_ms: Some(119),
                warmup_duration_ms: Some(8),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            capabilities: None,
            error: None,
        },
        120,
    );

    let selection = store
        .select_runtime_metrics(&WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            include_completed: Some(true),
        })
        .expect("runtime selection");

    assert_eq!(selection.workflow_run_id.as_deref(), Some("exec-1"));
    assert_eq!(
        selection.matched_workflow_run_ids,
        vec!["exec-1".to_string()]
    );
    assert!(!selection.is_ambiguous());
    assert_eq!(
        selection.runtime.and_then(|runtime| runtime.runtime_id),
        Some("llama_cpp".to_string())
    );
}

#[test]
fn workflow_trace_store_marks_runtime_metric_selection_ambiguous_for_multi_run_scope() {
    let store = WorkflowTraceStore::new(10);
    for (workflow_run_id, runtime_id, captured_at_ms) in [
        ("exec-1", "llama_cpp", 120_u64),
        ("exec-2", "llama_cpp.embedding", 220_u64),
    ] {
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                workflow_run_id: workflow_run_id.to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            captured_at_ms.saturating_sub(20),
        );
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                workflow_run_id: workflow_run_id.to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: captured_at_ms.saturating_sub(10),
                session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    attribution: None,
                    keep_alive: true,
                    state: crate::workflow::WorkflowExecutionSessionState::Running,
                    queued_runs: 0,
                    run_count: 2,
                }),
                items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                    workflow_run_id: workflow_run_id.to_string(),
                    enqueued_at_ms: Some(captured_at_ms.saturating_sub(20)),
                    dequeued_at_ms: Some(captured_at_ms.saturating_sub(10)),
                    priority: 5,
                    queue_position: Some(0),
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
                error: None,
            },
            captured_at_ms.saturating_sub(10),
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                workflow_run_id: workflow_run_id.to_string(),
                workflow_id: Some("wf-1".to_string()),
                captured_at_ms,
                runtime: WorkflowTraceRuntimeMetrics {
                    runtime_id: Some(runtime_id.to_string()),
                    observed_runtime_ids: vec![runtime_id.to_string()],
                    runtime_instance_id: Some(format!("{runtime_id}-instance")),
                    model_target: Some(format!("/models/{runtime_id}.gguf")),
                    warmup_started_at_ms: Some(captured_at_ms.saturating_sub(9)),
                    warmup_completed_at_ms: Some(captured_at_ms),
                    warmup_duration_ms: Some(9),
                    runtime_reused: Some(false),
                    lifecycle_decision_reason: Some("runtime_ready".to_string()),
                },
                capabilities: None,
                error: None,
            },
            captured_at_ms,
        );
    }

    let selection = store
        .select_runtime_metrics(&WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            include_completed: Some(true),
        })
        .expect("runtime selection");

    assert_eq!(selection.workflow_run_id, None);
    assert_eq!(selection.runtime, None);
    assert!(selection.is_ambiguous());
    assert_eq!(
        selection.matched_workflow_run_ids,
        vec!["exec-2".to_string(), "exec-1".to_string()]
    );
}

#[test]
fn workflow_trace_store_record_event_now_uses_backend_timestamp_capture() {
    let store = WorkflowTraceStore::new(10);
    let before_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let result = store.record_event_now(&WorkflowTraceEvent::RunStarted {
        workflow_run_id: "exec-1".to_string(),
        workflow_id: Some("wf-1".to_string()),
        node_count: 2,
    });
    let after_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before epoch")
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;

    assert!(result.recorded_at_ms >= before_ms);
    assert!(result.recorded_at_ms <= after_ms);
    let trace = result.snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.started_at_ms, result.recorded_at_ms);
    assert_eq!(trace.node_count_at_start, 2);
}
