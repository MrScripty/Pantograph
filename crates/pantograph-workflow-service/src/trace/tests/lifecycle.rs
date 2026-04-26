use super::*;

#[test]
fn workflow_trace_store_resets_attempt_state_when_run_restarts_after_failure() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph_context(
        "exec-1",
        &WorkflowTraceGraphContext {
            graph_fingerprint: Some("graph-1".to_string()),
            node_count_at_start: 2,
            node_types_by_id: HashMap::from([
                ("node-1".to_string(), "llm-inference".to_string()),
                ("node-2".to_string(), "embedding".to_string()),
            ]),
        },
    );

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
            node_type: None,
        },
        110,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeFailed {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            error: "boom".to_string(),
        },
        120,
    );
    store.record_event(
        &WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            captured_at_ms: 125,
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("/models/restarted.gguf".to_string()),
                warmup_started_at_ms: Some(101),
                warmup_completed_at_ms: Some(109),
                warmup_duration_ms: Some(8),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("loaded_runtime".to_string()),
            },
            capabilities: None,
            error: None,
        },
        125,
    );
    store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 126,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: None,
                keep_alive: false,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 1,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: Some(90),
                dequeued_at_ms: Some(100),
                priority: 0,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        126,
    );
    store.record_event(
        &WorkflowTraceEvent::RunFailed {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            error: "boom".to_string(),
        },
        130,
    );

    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 2,
        },
        200,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-2".to_string(),
            node_type: None,
        },
        210,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.graph_fingerprint.as_deref(), Some("graph-1"));
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.started_at_ms, 200);
    assert_eq!(trace.ended_at_ms, None);
    assert_eq!(trace.duration_ms, None);
    assert_eq!(trace.last_error, None);
    assert_eq!(trace.node_count_at_start, 2);
    assert_eq!(trace.event_count, 2);
    assert_eq!(trace.stream_event_count, 0);
    assert_eq!(trace.queue, WorkflowTraceQueueMetrics::default());
    assert_eq!(trace.runtime, WorkflowTraceRuntimeMetrics::default());
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].node_id, "node-2");
    assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Running);
}

#[test]
fn workflow_trace_store_corrects_existing_trace_workflow_metadata() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("exec-1", Some("session-1".to_string()));
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("session-1".to_string()),
            node_count: 0,
        },
        100,
    );

    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    let snapshot = store
        .snapshot(&WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("exec-1".to_string()),
            session_id: None,
            workflow_id: None,
            include_completed: None,
        })
        .expect("trace snapshot");

    let trace = &snapshot.traces[0];
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
}

#[test]
fn workflow_trace_store_tracks_observed_runtime_ids_across_runtime_snapshots() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("exec-mixed", Some("wf-mixed".to_string()));
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-mixed".to_string(),
            workflow_id: Some("wf-mixed".to_string()),
            node_count: 0,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: "exec-mixed".to_string(),
            workflow_id: Some("wf-mixed".to_string()),
            captured_at_ms: 110,
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("pytorch".to_string()),
                observed_runtime_ids: vec!["pytorch".to_string()],
                runtime_instance_id: Some("python-runtime:pytorch:venv_a".to_string()),
                model_target: Some("/models/a".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            capabilities: None,
            error: None,
        },
        110,
    );
    store.record_event(
        &WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: "exec-mixed".to_string(),
            workflow_id: Some("wf-mixed".to_string()),
            captured_at_ms: 120,
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("onnx-runtime".to_string()),
                observed_runtime_ids: vec!["onnx-runtime".to_string()],
                runtime_instance_id: Some("python-runtime:onnx-runtime:venv_onnx".to_string()),
                model_target: Some("/models/b".to_string()),
                warmup_started_at_ms: None,
                warmup_completed_at_ms: None,
                warmup_duration_ms: None,
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            capabilities: None,
            error: None,
        },
        120,
    );

    let trace = store
        .snapshot(&crate::trace::WorkflowTraceSnapshotRequest {
            workflow_run_id: Some("exec-mixed".to_string()),
            session_id: None,
            workflow_id: None,
            include_completed: Some(true),
        })
        .expect("trace snapshot")
        .traces
        .into_iter()
        .next()
        .expect("mixed trace");

    assert_eq!(trace.runtime.runtime_id.as_deref(), Some("onnx-runtime"));
    assert_eq!(
        trace.runtime.observed_runtime_ids,
        vec!["pytorch".to_string(), "onnx-runtime".to_string()]
    );
}

#[test]
fn workflow_trace_store_keeps_inflight_state_on_duplicate_run_started() {
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
    let snapshot = store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 1,
        },
        120,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.started_at_ms, 100);
    assert_eq!(trace.node_count_at_start, 1);
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].node_id, "node-1");
    assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Running);
    assert_eq!(trace.event_count, 3);
}

#[test]
fn workflow_trace_store_records_cancelled_runs_and_marks_active_nodes_cancelled() {
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
    let snapshot = store.record_event(
        &WorkflowTraceEvent::RunCancelled {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            error: "workflow run cancelled during execution".to_string(),
        },
        140,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.status, WorkflowTraceStatus::Cancelled);
    assert_eq!(trace.ended_at_ms, Some(140));
    assert_eq!(trace.duration_ms, Some(40));
    assert_eq!(
        trace.last_error.as_deref(),
        Some("workflow run cancelled during execution")
    );
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Cancelled);
    assert_eq!(trace.nodes[0].ended_at_ms, Some(140));
    assert_eq!(trace.nodes[0].duration_ms, Some(30));
}

#[test]
fn workflow_trace_store_resets_attempt_state_when_run_restarts_after_cancellation() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph_context(
        "exec-1",
        &WorkflowTraceGraphContext {
            graph_fingerprint: Some("graph-1".to_string()),
            node_count_at_start: 2,
            node_types_by_id: HashMap::from([
                ("node-1".to_string(), "llm-inference".to_string()),
                ("node-2".to_string(), "embedding".to_string()),
            ]),
        },
    );

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
            node_type: None,
        },
        110,
    );
    store.record_event(
        &WorkflowTraceEvent::RunCancelled {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            error: "workflow run cancelled during execution".to_string(),
        },
        130,
    );

    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 2,
        },
        200,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-2".to_string(),
            node_type: None,
        },
        210,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.graph_fingerprint.as_deref(), Some("graph-1"));
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.started_at_ms, 200);
    assert_eq!(trace.ended_at_ms, None);
    assert_eq!(trace.duration_ms, None);
    assert_eq!(trace.last_error, None);
    assert_eq!(trace.node_count_at_start, 2);
    assert_eq!(trace.event_count, 2);
    assert_eq!(trace.stream_event_count, 0);
    assert_eq!(trace.queue, WorkflowTraceQueueMetrics::default());
    assert_eq!(trace.runtime, WorkflowTraceRuntimeMetrics::default());
    assert_eq!(trace.nodes.len(), 1);
    assert_eq!(trace.nodes[0].node_id, "node-2");
    assert_eq!(trace.nodes[0].status, WorkflowTraceNodeStatus::Running);
}

#[test]
fn workflow_trace_store_ignores_duplicate_run_completed_events() {
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
        &WorkflowTraceEvent::RunCompleted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
        },
        140,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::RunCompleted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
        },
        170,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.status, WorkflowTraceStatus::Completed);
    assert_eq!(trace.ended_at_ms, Some(140));
    assert_eq!(trace.duration_ms, Some(40));
    assert_eq!(trace.event_count, 2);
}

#[test]
fn workflow_trace_store_incremental_execution_started_resumes_waiting_runs() {
    let store = WorkflowTraceStore::new(10);

    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 2,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::WaitingForInput {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_id: "human-input-1".to_string(),
        },
        140,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::IncrementalExecutionStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            task_ids: vec!["resume-node".to_string()],
        },
        180,
    );

    let trace = snapshot.traces.first().expect("trace");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert!(!trace.waiting_for_input);
    assert_eq!(trace.started_at_ms, 100);
    assert_eq!(trace.ended_at_ms, None);
    assert_eq!(trace.duration_ms, None);
    assert_eq!(trace.last_error, None);
    assert_eq!(trace.event_count, 3);
    assert_eq!(
        trace.last_incremental_task_ids,
        vec!["resume-node".to_string()]
    );
}
