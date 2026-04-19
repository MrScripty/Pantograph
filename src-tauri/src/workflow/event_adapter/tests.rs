use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use node_engine::EventSink;
use node_engine::{
    Context, ExecutorExtensions, GraphNode as EngineGraphNode, TaskExecutor, WorkflowExecutor,
};
use pantograph_workflow_service::{
    graph::WorkflowDerivedGraph, GraphNode, Position, WorkflowGraph,
};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use super::diagnostics_bridge::translate_node_event_with_diagnostics;
use super::translation::translated_execution_id;
use crate::workflow::WorkflowDiagnosticsStore;

fn sample_parallel_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "left".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "right".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 100.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: Vec::new(),
        derived_graph: Some(WorkflowDerivedGraph {
            schema_version: 1,
            graph_fingerprint: "graph-parallel".to_string(),
            consumer_count_map: std::collections::HashMap::new(),
        }),
    }
}

fn make_parallel_roots_graph() -> node_engine::WorkflowGraph {
    let mut graph = node_engine::WorkflowGraph::new("parallel", "Parallel");
    for (id, x) in [("left", 0.0), ("right", 100.0)] {
        graph.nodes.push(EngineGraphNode {
            id: id.to_string(),
            node_type: "process".to_string(),
            data: serde_json::json!({"node": id}),
            position: (x, 0.0),
        });
    }
    graph
}

struct YieldingAcceptanceExecutor {
    current_in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
}

impl YieldingAcceptanceExecutor {
    fn new() -> Self {
        Self {
            current_in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }

    fn record_max_in_flight(&self, observed: usize) {
        let mut current_max = self.max_in_flight.load(Ordering::SeqCst);
        while observed > current_max {
            match self.max_in_flight.compare_exchange(
                current_max,
                observed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }
}

#[async_trait]
impl TaskExecutor for YieldingAcceptanceExecutor {
    async fn execute_task(
        &self,
        task_id: &str,
        _inputs: HashMap<String, serde_json::Value>,
        _context: &Context,
        _extensions: &ExecutorExtensions,
    ) -> node_engine::Result<HashMap<String, serde_json::Value>> {
        let observed = self.current_in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.record_max_in_flight(observed);
        tokio::task::yield_now().await;
        self.current_in_flight.fetch_sub(1, Ordering::SeqCst);

        Ok(HashMap::from([(
            "out".to_string(),
            serde_json::json!({ "task": task_id }),
        )]))
    }
}

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
        super::TauriWorkflowEvent::Started {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot {
            execution_id,
            snapshot,
        } => {
            assert_eq!(execution_id, "exec-1");
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
            occurred_at_ms: Some(25),
        },
    );

    match diagnostics_event {
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
            let trace = snapshot.runs_by_id.get("exec-1").expect("trace");
            let node = trace.nodes.get("node-a").expect("node overlay");
            assert_eq!(node.last_progress, Some(0.5));
            assert_eq!(node.last_message.as_deref(), Some("working"));
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
        super::TauriWorkflowEvent::Cancelled {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::Failed {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::GraphModified {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot {
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
        super::TauriWorkflowEvent::IncrementalExecutionStarted {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot {
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
        super::TauriWorkflowEvent::WaitingForInput {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
        super::TauriWorkflowEvent::DiagnosticsSnapshot { snapshot, .. } => {
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
fn adapter_send_emits_primary_and_diagnostics_events() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = emitted.clone();
    let channel: Channel<super::TauriWorkflowEvent> = Channel::new(move |body| {
        let value = match body {
            InvokeResponseBody::Json(json) => {
                serde_json::from_str::<Value>(&json).expect("channel event json")
            }
            InvokeResponseBody::Raw(bytes) => {
                serde_json::from_slice::<Value>(&bytes).expect("channel event raw json")
            }
        };
        captured.lock().expect("captured events lock").push(value);
        Ok(())
    });
    let adapter = super::TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store);

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(55),
        },
    )
    .expect("send should succeed");

    let events = emitted.lock().expect("captured events lock");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["type"], "Started");
    assert_eq!(events[0]["data"]["execution_id"], "exec-1");
    assert_eq!(events[1]["type"], "DiagnosticsSnapshot");
    assert_eq!(events[1]["data"]["execution_id"], "exec-1");
    assert_eq!(events[1]["data"]["snapshot"]["runOrder"][0], "exec-1");
}

#[tokio::test]
async fn workflow_executor_parallel_run_emits_consumer_visible_events_through_adapter() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    diagnostics_store.set_execution_metadata(
        "exec-parallel",
        Some("parallel".to_string()),
        Some("Parallel Workflow".to_string()),
    );
    diagnostics_store.set_execution_graph("exec-parallel", &sample_parallel_graph());

    let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = emitted.clone();
    let channel: Channel<super::TauriWorkflowEvent> = Channel::new(move |body| {
        let value = match body {
            InvokeResponseBody::Json(json) => {
                serde_json::from_str::<Value>(&json).expect("channel event json")
            }
            InvokeResponseBody::Raw(bytes) => {
                serde_json::from_slice::<Value>(&bytes).expect("channel event raw json")
            }
        };
        captured.lock().expect("captured events lock").push(value);
        Ok(())
    });
    let adapter = Arc::new(super::TauriEventAdapter::new(
        channel,
        "parallel",
        diagnostics_store,
    ));
    let workflow_executor = WorkflowExecutor::new(
        "exec-parallel",
        make_parallel_roots_graph(),
        adapter.clone(),
    );
    let executor_impl = YieldingAcceptanceExecutor::new();

    let outputs = workflow_executor
        .demand_multiple(&["left".to_string(), "right".to_string()], &executor_impl)
        .await
        .expect("parallel workflow should succeed");

    assert_eq!(executor_impl.max_in_flight(), 2);
    assert_eq!(outputs.len(), 2);

    let events = emitted.lock().expect("captured events lock");
    let event_types = events
        .iter()
        .map(|event| event["type"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert!(event_types.contains(&"IncrementalExecutionStarted".to_string()));
    assert!(event_types.contains(&"NodeStarted".to_string()));
    assert!(event_types.contains(&"NodeCompleted".to_string()));
    assert!(event_types.contains(&"DiagnosticsSnapshot".to_string()));

    let snapshot = events
        .iter()
        .rev()
        .find(|event| event["type"] == "DiagnosticsSnapshot")
        .expect("final diagnostics snapshot");
    let run = &snapshot["data"]["snapshot"]["runsById"]["exec-parallel"];
    assert_eq!(run["workflowName"], "Parallel Workflow");
    assert_eq!(run["graphFingerprintAtStart"], "graph-parallel");
    assert_eq!(run["lastIncrementalTaskIds"][0], "left");
    assert_eq!(run["lastIncrementalTaskIds"][1], "right");
    assert_eq!(run["nodes"]["left"]["status"], "completed");
    assert_eq!(run["nodes"]["right"]["status"], "completed");
}
