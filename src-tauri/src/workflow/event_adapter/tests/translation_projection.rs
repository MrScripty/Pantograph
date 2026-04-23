use super::*;

#[test]
fn translated_workflow_started_event_preserves_engine_execution_id() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(1_717_171_001),
        },
    );

    match &event {
        TauriWorkflowEvent::Started {
            workflow_id,
            execution_id,
            ..
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-1");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(translated_execution_id(&event), "exec-1");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot {
            execution_id,
            snapshot,
        } => {
            assert_eq!(execution_id, "exec-1");
            assert_eq!(
                snapshot.context.source_execution_id.as_deref(),
                Some("exec-1")
            );
            assert_eq!(
                snapshot.context.relevant_execution_id.as_deref(),
                Some("exec-1")
            );
            assert!(snapshot.context.relevant);
            assert_eq!(snapshot.run_order, vec!["exec-1".to_string()]);
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(trace.started_at_ms, 1_717_171_001);
            assert_eq!(trace.events[0].timestamp_ms, 1_717_171_001);
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_task_progress_event_updates_backend_diagnostics_projection() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(10),
        },
    );

    let (_, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "exec-1".to_string(),
            progress: 0.5,
            message: Some("working".to_string()),
            detail: None,
            occurred_at_ms: Some(25),
        },
    );

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            let node = trace.nodes.get("node-a").expect("node overlay");
            assert_eq!(node.last_progress, Some(0.5));
            assert_eq!(node.last_message.as_deref(), Some("working"));
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_task_progress_detail_updates_backend_diagnostics_projection() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(10),
        },
    );

    let (_, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "exec-1".to_string(),
            progress: 0.0,
            message: Some("kv cache restored".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::RestoreInput,
                    outcome: node_engine::KvCacheEventOutcome::Hit,
                    cache_id: Some("cache-1".to_string()),
                    backend_key: Some("llamacpp".to_string()),
                    reuse_source: Some("llamacpp_slot".to_string()),
                    token_count: Some(32),
                    reason: Some("restored_input_handle".to_string()),
                },
            )),
            occurred_at_ms: Some(25),
        },
    );

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            let node = trace.nodes.get("node-a").expect("node overlay");
            match node.last_progress_detail.as_ref() {
                Some(node_engine::TaskProgressDetail::KvCache(detail)) => {
                    assert_eq!(detail.outcome, node_engine::KvCacheEventOutcome::Hit);
                    assert_eq!(detail.cache_id.as_deref(), Some("cache-1"));
                }
                other => panic!("unexpected progress detail: {other:?}"),
            }
            assert_eq!(node.last_progress, None);
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_workflow_cancelled_event_maps_directly_to_cancelled_event() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            error: "workflow run cancelled during execution".to_string(),
            occurred_at_ms: Some(33),
        },
    );

    match event {
        TauriWorkflowEvent::Cancelled {
            workflow_id,
            execution_id,
            error,
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-1");
            assert!(error.contains("cancelled"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Cancelled
            );
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_workflow_failed_event_stays_failed() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            error: "runtime unavailable".to_string(),
            occurred_at_ms: Some(33),
        },
    );

    match event {
        TauriWorkflowEvent::Failed {
            workflow_id,
            execution_id,
            error,
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-1");
            assert_eq!(error, "runtime unavailable");
        }
        other => panic!("unexpected event: {other:?}"),
    }

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Failed
            );
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_graph_modified_event_preserves_engine_execution_id() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::GraphModified {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-graph".to_string(),
            dirty_tasks: vec!["node-a".to_string(), "node-b".to_string()],
            memory_impact: Some(
                node_engine::GraphMemoryImpactSummary::fallback_full_invalidation(
                    ["node-a", "node-b"],
                    "graph_changed",
                ),
            ),
            occurred_at_ms: Some(44),
        },
    );

    match &event {
        TauriWorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            memory_impact,
            ..
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-graph");
            assert_eq!(
                dirty_tasks,
                &vec!["node-a".to_string(), "node-b".to_string()]
            );
            assert_eq!(
                memory_impact,
                &Some(
                    node_engine::GraphMemoryImpactSummary::fallback_full_invalidation(
                        ["node-a", "node-b"],
                        "graph_changed",
                    )
                )
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(translated_execution_id(&event), "exec-graph");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot {
            execution_id,
            snapshot,
        } => {
            assert_eq!(execution_id, "exec-graph");
            assert_eq!(snapshot.run_order, vec!["exec-graph".to_string()]);
            let trace = snapshot.runs_by_id.get("exec-graph").expect("trace");
            assert_eq!(
                trace.last_dirty_tasks,
                vec!["node-a".to_string(), "node-b".to_string()]
            );
            assert!(trace.last_incremental_task_ids.is_empty());
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_incremental_execution_started_event_preserves_resume_task_ids() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-inc".to_string(),
            task_id: "human-input-1".to_string(),
            prompt: Some("Need approval".to_string()),
            occurred_at_ms: Some(50),
        },
    );

    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-inc".to_string(),
            tasks: vec!["node-a".to_string(), "node-b".to_string()],
            occurred_at_ms: Some(61),
        },
    );

    match &event {
        TauriWorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            task_ids,
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-inc");
            assert_eq!(task_ids, &vec!["node-a".to_string(), "node-b".to_string()]);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(translated_execution_id(&event), "exec-inc");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot {
            execution_id,
            snapshot,
        } => {
            assert_eq!(execution_id, "exec-inc");
            let trace = snapshot.runs_by_id.get("exec-inc").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Running
            );
            assert!(!trace.waiting_for_input);
            assert_eq!(
                trace.last_incremental_task_ids,
                vec!["node-a".to_string(), "node-b".to_string()]
            );
            assert!(trace.last_dirty_tasks.is_empty());
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_waiting_for_input_event_preserves_backend_contract_and_waiting_status() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-wait".to_string(),
            task_id: "human-input-1".to_string(),
            prompt: Some("Need approval".to_string()),
            occurred_at_ms: Some(52),
        },
    );

    match &event {
        TauriWorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            node_id,
            message,
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-wait");
            assert_eq!(node_id, "human-input-1");
            assert_eq!(message.as_deref(), Some("Need approval"));
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert_eq!(translated_execution_id(&event), "exec-wait");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot {
            execution_id,
            snapshot,
        } => {
            assert_eq!(execution_id, "exec-wait");
            let trace = snapshot.runs_by_id.get("exec-wait").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Waiting
            );
            let node = trace.nodes.get("human-input-1").expect("node overlay");
            assert_eq!(node.last_message.as_deref(), Some("Need approval"));
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_parallel_root_events_preserve_overlapping_trace_timing() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    diagnostics_store.set_execution_metadata(
        "exec-parallel",
        Some("wf-parallel".to_string()),
        Some("Parallel Workflow".to_string()),
    );
    diagnostics_store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            tasks: vec!["left".to_string(), "right".to_string()],
            occurred_at_ms: Some(1_000),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskStarted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(1_010),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskStarted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(1_012),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskCompleted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "left" })),
            occurred_at_ms: Some(1_040),
        },
    );
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskCompleted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "right" })),
            occurred_at_ms: Some(1_060),
        },
    );

    assert_eq!(translated_execution_id(&event), "exec-parallel");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let run = snapshot.runs_by_id.get("exec-parallel").expect("trace");
            assert_eq!(run.workflow_name.as_deref(), Some("Parallel Workflow"));
            assert_eq!(
                run.graph_fingerprint_at_start.as_deref(),
                Some("graph-parallel")
            );
            assert_eq!(
                run.last_incremental_task_ids,
                vec!["left".to_string(), "right".to_string()]
            );
            assert_eq!(run.event_count, 5);
            assert_eq!(run.last_updated_at_ms, 1_060);

            let left = run.nodes.get("left").expect("left node trace");
            assert_eq!(
                left.status,
                crate::workflow::diagnostics::DiagnosticsNodeStatus::Completed
            );
            assert_eq!(left.duration_ms, Some(30));

            let right = run.nodes.get("right").expect("right node trace");
            assert_eq!(
                right.status,
                crate::workflow::diagnostics::DiagnosticsNodeStatus::Completed
            );
            assert_eq!(right.duration_ms, Some(48));
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_parallel_waiting_event_preserves_waiting_pause_duration() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    diagnostics_store.set_execution_metadata(
        "exec-parallel",
        Some("wf-parallel".to_string()),
        Some("Parallel Workflow".to_string()),
    );
    diagnostics_store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            tasks: vec!["left".to_string(), "right".to_string()],
            occurred_at_ms: Some(2_000),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskStarted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(2_010),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskStarted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(2_012),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskCompleted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "left" })),
            occurred_at_ms: Some(2_040),
        },
    );
    let (event, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            task_id: "right".to_string(),
            prompt: Some("waiting at right".to_string()),
            occurred_at_ms: Some(2_060),
        },
    );

    assert_eq!(translated_execution_id(&event), "exec-parallel");
    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let run = snapshot.runs_by_id.get("exec-parallel").expect("trace");
            assert_eq!(
                run.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Waiting
            );
            assert!(run.waiting_for_input);
            assert_eq!(run.last_updated_at_ms, 2_060);

            let left = run.nodes.get("left").expect("left node trace");
            assert_eq!(left.duration_ms, Some(30));

            let right = run.nodes.get("right").expect("right node trace");
            assert_eq!(
                right.status,
                crate::workflow::diagnostics::DiagnosticsNodeStatus::Waiting
            );
            assert_eq!(right.duration_ms, Some(48));
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_duplicate_terminal_events_preserve_backend_trace_timing() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(100),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(140),
        },
    );
    let (_, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(170),
        },
    );

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Completed
            );
            assert_eq!(trace.ended_at_ms, Some(140));
            assert_eq!(trace.duration_ms, Some(40));
            assert_eq!(trace.events.len(), 2);
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_restarted_execution_resets_diagnostics_overlay_state() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(100),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "exec-1".to_string(),
            progress: 0.5,
            message: Some("halfway".to_string()),
            detail: None,
            occurred_at_ms: Some(120),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(140),
        },
    );

    let (_, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(200),
        },
    );

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Running
            );
            assert_eq!(trace.started_at_ms, 200);
            assert_eq!(trace.event_count, 1);
            assert_eq!(trace.events.len(), 1);
            assert_eq!(trace.events[0].event_type, "Started");
            assert!(trace.nodes.is_empty());
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}

#[test]
fn translated_cancelled_then_restarted_execution_resets_diagnostics_overlay_state() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(100),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::TaskProgress {
            task_id: "node-a".to_string(),
            execution_id: "exec-1".to_string(),
            progress: 0.5,
            message: Some("halfway".to_string()),
            detail: None,
            occurred_at_ms: Some(120),
        },
    );
    let _ = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            error: "workflow run cancelled during execution".to_string(),
            occurred_at_ms: Some(140),
        },
    );

    let (_, diagnostics_event) = translate_node_event_with_diagnostics(
        &diagnostics_store,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(200),
        },
    );

    match diagnostics_event {
        TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            assert_eq!(
                trace.status,
                crate::workflow::diagnostics::DiagnosticsRunStatus::Running
            );
            assert_eq!(trace.started_at_ms, 200);
            assert_eq!(trace.event_count, 1);
            assert_eq!(trace.events.len(), 1);
            assert_eq!(trace.events[0].event_type, "Started");
            assert!(trace.nodes.is_empty());
        }
        other => panic!("unexpected diagnostics event: {other:?}"),
    }
}
