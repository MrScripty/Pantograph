use std::collections::HashMap;

use pantograph_workflow_service::graph::{WorkflowDerivedGraph, WorkflowExecutionSessionKind};

use super::*;

mod clear_history;

fn sample_graph() -> pantograph_workflow_service::WorkflowGraph {
    pantograph_workflow_service::WorkflowGraph {
        nodes: vec![pantograph_workflow_service::GraphNode {
            id: "llm-1".to_string(),
            node_type: "llm-inference".to_string(),
            position: pantograph_workflow_service::Position { x: 0.0, y: 0.0 },
            data: serde_json::json!({}),
        }],
        edges: Vec::new(),
        derived_graph: Some(WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "graph-123".to_string(),
            consumer_count_map: HashMap::new(),
        }),
    }
}

fn sample_parallel_graph() -> pantograph_workflow_service::WorkflowGraph {
    pantograph_workflow_service::WorkflowGraph {
        nodes: vec![
            pantograph_workflow_service::GraphNode {
                id: "left".to_string(),
                node_type: "llm-inference".to_string(),
                position: pantograph_workflow_service::Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            pantograph_workflow_service::GraphNode {
                id: "right".to_string(),
                node_type: "llm-inference".to_string(),
                position: pantograph_workflow_service::Position { x: 100.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: Vec::new(),
        derived_graph: Some(WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "graph-parallel".to_string(),
            consumer_count_map: HashMap::new(),
        }),
    }
}

fn diagnostics_overlay_event_for_node_engine_event(
    event: &node_engine::WorkflowEvent,
) -> crate::workflow::events::WorkflowEvent {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id,
            execution_id,
            ..
        } => crate::workflow::events::WorkflowEvent::Started {
            workflow_id: workflow_id.clone(),
            node_count: 0,
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id,
            execution_id,
            ..
        } => crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: workflow_id.clone(),
            outputs: HashMap::new(),
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id,
            execution_id,
            error,
            ..
        } => crate::workflow::events::WorkflowEvent::Failed {
            workflow_id: workflow_id.clone(),
            error: error.clone(),
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::WaitingForInput {
            workflow_id,
            execution_id,
            task_id,
            prompt,
            ..
        } => crate::workflow::events::WorkflowEvent::WaitingForInput {
            workflow_id: workflow_id.clone(),
            execution_id: execution_id.clone(),
            node_id: task_id.clone(),
            message: prompt.clone(),
        },
        node_engine::WorkflowEvent::TaskStarted {
            task_id,
            execution_id,
            ..
        } => crate::workflow::events::WorkflowEvent::NodeStarted {
            node_id: task_id.clone(),
            node_type: String::new(),
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::TaskCompleted {
            task_id,
            execution_id,
            output,
            ..
        } => crate::workflow::events::WorkflowEvent::NodeCompleted {
            node_id: task_id.clone(),
            outputs: output
                .as_ref()
                .and_then(|value| serde_json::from_value(value.clone()).ok())
                .unwrap_or_default(),
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::TaskFailed {
            task_id,
            execution_id,
            error,
            ..
        } => crate::workflow::events::WorkflowEvent::NodeError {
            node_id: task_id.clone(),
            error: error.clone(),
            execution_id: execution_id.clone(),
        },
        node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id,
            execution_id,
            tasks,
            ..
        } => crate::workflow::events::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: workflow_id.clone(),
            execution_id: execution_id.clone(),
            task_ids: tasks.clone(),
        },
        other => panic!("unsupported node-engine event in diagnostics test: {other:?}"),
    }
}

fn record_node_engine_event(
    store: &WorkflowDiagnosticsStore,
    event: &node_engine::WorkflowEvent,
) -> WorkflowDiagnosticsProjection {
    let (trace_event, occurred_at_ms) =
        node_engine_workflow_trace_event(event).expect("node-engine trace event");
    let overlay_event = diagnostics_overlay_event_for_node_engine_event(event);
    store.record_trace_event_with_overlay(&trace_event, &overlay_event, occurred_at_ms)
}

#[test]
fn workflow_diagnostics_snapshot_request_normalizes_trimmed_filters() {
    let normalized = WorkflowDiagnosticsSnapshotRequest {
        session_id: Some("  session-1  ".to_string()),
        workflow_id: Some("   ".to_string()),
        workflow_name: Some("\tWorkflow 1\t".to_string()),
        workflow_graph: None,
    }
    .normalized();

    assert_eq!(normalized.session_id.as_deref(), Some("session-1"));
    assert_eq!(normalized.workflow_id.as_deref(), Some(""));
    assert_eq!(normalized.workflow_name.as_deref(), Some("Workflow 1"));
}

#[test]
fn workflow_diagnostics_snapshot_request_rejects_blank_filters() {
    let request = WorkflowDiagnosticsSnapshotRequest {
        session_id: None,
        workflow_id: Some("   ".to_string()),
        workflow_name: None,
        workflow_graph: None,
    }
    .normalized();

    let error = request
        .validate()
        .expect_err("blank workflow_id should be rejected");

    assert!(
        matches!(
            error,
            pantograph_workflow_service::WorkflowServiceError::InvalidRequest(ref message)
                if message
                    == "workflow diagnostics snapshot request field 'workflow_id' must not be blank"
        ),
        "unexpected validation error: {:?}",
        error
    );
}

#[test]
fn record_workflow_event_tracks_run_and_node_timing() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-1",
        Some("wf-1".to_string()),
        Some("Test Workflow".to_string()),
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
            node_type: String::new(),
            execution_id: "exec-1".to_string(),
        },
        1_010,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeCompleted {
            node_id: "llm-1".to_string(),
            outputs: HashMap::new(),
            execution_id: "exec-1".to_string(),
        },
        1_050,
    );
    let snapshot = store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-1".to_string(),
            outputs: HashMap::new(),
            execution_id: "exec-1".to_string(),
        },
        1_100,
    );

    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    assert_eq!(run.workflow_name.as_deref(), Some("Test Workflow"));
    assert_eq!(run.graph_fingerprint_at_start.as_deref(), Some("graph-123"));
    assert_eq!(run.node_count_at_start, 1);
    assert_eq!(run.status, DiagnosticsRunStatus::Completed);
    assert_eq!(run.duration_ms, Some(100));
    assert_eq!(run.events.len(), 4);

    let node = run.nodes.get("llm-1").expect("node trace");
    assert_eq!(node.node_type.as_deref(), Some("llm-inference"));
    assert_eq!(node.status, DiagnosticsNodeStatus::Completed);
    assert_eq!(node.duration_ms, Some(40));
}

#[test]
fn node_engine_parallel_root_trace_projection_tracks_overlapping_node_timing() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-parallel",
        Some("wf-parallel".to_string()),
        Some("Parallel Workflow".to_string()),
    );
    store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            tasks: vec!["left".to_string(), "right".to_string()],
            occurred_at_ms: Some(1_000),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(1_010),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(1_012),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskCompleted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "left" })),
            occurred_at_ms: Some(1_040),
        },
    );
    let snapshot = record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskCompleted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "right" })),
            occurred_at_ms: Some(1_060),
        },
    );

    let run = snapshot
        .runs_by_id
        .get("exec-parallel")
        .expect("parallel run trace");
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
    assert_eq!(left.node_type.as_deref(), Some("llm-inference"));
    assert_eq!(left.status, DiagnosticsNodeStatus::Completed);
    assert_eq!(left.duration_ms, Some(30));

    let right = run.nodes.get("right").expect("right node trace");
    assert_eq!(right.node_type.as_deref(), Some("llm-inference"));
    assert_eq!(right.status, DiagnosticsNodeStatus::Completed);
    assert_eq!(right.duration_ms, Some(48));
}

#[test]
fn node_engine_parallel_waiting_trace_projection_tracks_waiting_state() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata(
        "exec-parallel",
        Some("wf-parallel".to_string()),
        Some("Parallel Workflow".to_string()),
    );
    store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            tasks: vec!["left".to_string(), "right".to_string()],
            occurred_at_ms: Some(2_000),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(2_010),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskStarted {
            task_id: "right".to_string(),
            execution_id: "exec-parallel".to_string(),
            occurred_at_ms: Some(2_012),
        },
    );
    record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::TaskCompleted {
            task_id: "left".to_string(),
            execution_id: "exec-parallel".to_string(),
            output: Some(serde_json::json!({ "out": "left" })),
            occurred_at_ms: Some(2_040),
        },
    );
    let snapshot = record_node_engine_event(
        &store,
        &node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: "wf-parallel".to_string(),
            execution_id: "exec-parallel".to_string(),
            task_id: "right".to_string(),
            prompt: Some("waiting at right".to_string()),
            occurred_at_ms: Some(2_060),
        },
    );

    let run = snapshot
        .runs_by_id
        .get("exec-parallel")
        .expect("parallel run trace");
    assert_eq!(run.status, DiagnosticsRunStatus::Waiting);
    assert!(run.waiting_for_input);
    assert_eq!(run.last_updated_at_ms, 2_060);

    let left = run.nodes.get("left").expect("left node trace");
    assert_eq!(left.status, DiagnosticsNodeStatus::Completed);
    assert_eq!(left.duration_ms, Some(30));

    let right = run.nodes.get("right").expect("right node trace");
    assert_eq!(right.status, DiagnosticsNodeStatus::Waiting);
    assert_eq!(right.duration_ms, Some(48));
}

mod runtime_projection;

mod overlay;

mod replay;

mod timing;

#[test]
fn cancelled_workflow_event_maps_to_cancelled_trace_status() {
    let store = WorkflowDiagnosticsStore::default();

    let snapshot = store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Cancelled {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            error: "workflow run cancelled during execution".to_string(),
        },
        200,
    );

    let trace = snapshot.runs_by_id.get("exec-1").expect("cancelled trace");
    assert_eq!(trace.status, DiagnosticsRunStatus::Cancelled);
    assert_eq!(
        trace.error.as_deref(),
        Some("workflow run cancelled during execution")
    );
}

#[test]
fn trace_snapshot_filters_runs_without_projection_overlay_rules() {
    let store = WorkflowDiagnosticsStore::default();
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            execution_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-1".to_string(),
            outputs: HashMap::new(),
            execution_id: "exec-1".to_string(),
        },
        1_100,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-2".to_string(),
            node_count: 1,
            execution_id: "exec-2".to_string(),
        },
        1_200,
    );

    let snapshot = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            execution_id: None,
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: Some(false),
        })
        .expect("trace snapshot");

    assert_eq!(snapshot.traces.len(), 1);
    assert_eq!(snapshot.traces[0].execution_id, "exec-2");
    assert_eq!(
        snapshot.traces[0].status,
        pantograph_workflow_service::WorkflowTraceStatus::Running
    );
}
