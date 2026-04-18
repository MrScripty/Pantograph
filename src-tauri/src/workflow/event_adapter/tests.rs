use std::sync::{Arc, Mutex};

use node_engine::EventSink;
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use super::diagnostics_bridge::translate_node_event_with_diagnostics;
use super::translation::translated_execution_id;
use crate::workflow::WorkflowDiagnosticsStore;

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
            occurred_at_ms: Some(44),
        },
    );

    match &event {
        super::TauriWorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        } => {
            assert_eq!(workflow_id, "wf-1");
            assert_eq!(execution_id, "exec-graph");
            assert_eq!(dirty_tasks, &vec!["node-a".to_string(), "node-b".to_string()]);
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
