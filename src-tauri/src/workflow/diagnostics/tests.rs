use std::collections::HashMap;

use pantograph_workflow_service::graph::{WorkflowDerivedGraph, WorkflowSessionKind};

use super::*;

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

#[test]
fn runtime_and_scheduler_snapshots_are_backend_owned() {
    let store = WorkflowDiagnosticsStore::default();
    store.update_runtime_snapshot(
        Some("wf-runtime".to_string()),
        Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                required_extensions: vec!["kv-cache".to_string()],
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
        None,
        Some("/models/main.gguf".to_string()),
        Some("/models/embed.gguf".to_string()),
        Some(inference::RuntimeLifecycleSnapshot {
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
        Some(inference::RuntimeLifecycleSnapshot {
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
        5_000,
    );
    let snapshot = store.update_scheduler_snapshot(
        Some("wf-runtime".to_string()),
        Some("session-1".to_string()),
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-runtime".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: None,
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 3,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("run-1".to_string()),
            enqueued_at_ms: None,
            dequeued_at_ms: None,
            priority: 10,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
        6_000,
    );

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
        Some(WorkflowSessionKind::Workflow)
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
        next_admission_queue_id: Some("queue-1".to_string()),
        next_admission_bypassed_queue_id: None,
        next_admission_after_runs: Some(1),
        next_admission_wait_ms: None,
        next_admission_not_before_ms: None,
        next_admission_reason: Some(
            pantograph_workflow_service::WorkflowSchedulerDecisionReason::WarmSessionReused,
        ),
        runtime_registry: None,
    };

    let snapshot = store.update_scheduler_snapshot(
        Some("wf-runtime".to_string()),
        Some("session-1".to_string()),
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-runtime".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 3,
        }),
        Vec::new(),
        Some(diagnostics.clone()),
        None,
        6_000,
    );

    assert_eq!(snapshot.scheduler.diagnostics, Some(diagnostics));
}

#[test]
fn runtime_snapshot_falls_back_to_selected_capability_when_lifecycle_is_absent() {
    let store = WorkflowDiagnosticsStore::default();
    let snapshot = store.update_runtime_snapshot(
        Some("wf-runtime".to_string()),
        Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                supports_external_connection: false,
                backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        None,
        Some("black-forest-labs/flux.1-schnell".to_string()),
        None,
        None,
        None,
        5_000,
    );

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
    let snapshot = store.update_runtime_snapshot(
        Some("wf-onnx".to_string()),
        Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                supports_external_connection: false,
                backend_keys: vec!["ONNX Runtime".to_string(), "onnx-runtime".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        None,
        Some("kitten-tts".to_string()),
        None,
        None,
        None,
        5_000,
    );

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
    let snapshot = store.update_runtime_snapshot(
        Some("wf-runtime".to_string()),
        Some(pantograph_workflow_service::WorkflowCapabilitiesResponse {
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
                supports_external_connection: false,
                backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        }),
        None,
        Some("black-forest-labs/flux.1-schnell".to_string()),
        None,
        None,
        None,
        5_000,
    );

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
    let snapshot = store.record_runtime_snapshot(
        "wf-runtime".to_string(),
        "exec-runtime".to_string(),
        5_000,
        None,
        pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
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
        Some("/models/main.gguf".to_string()),
        Some("/models/embed.gguf".to_string()),
        Some(inference::RuntimeLifecycleSnapshot {
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
        Some(inference::RuntimeLifecycleSnapshot {
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
        None,
    );

    let trace = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            execution_id: Some("exec-runtime".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
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
    let projection = store.record_scheduler_snapshot(
        None,
        "edit-session-1".to_string(),
        "edit-session-1".to_string(),
        5_000,
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "edit-session-1".to_string(),
            workflow_id: "edit-session-1".to_string(),
            session_kind: WorkflowSessionKind::Edit,
            usage_profile: None,
            keep_alive: false,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 2,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "edit-session-1".to_string(),
            run_id: Some("edit-session-1".to_string()),
            enqueued_at_ms: Some(4_750),
            dequeued_at_ms: Some(4_750),
            priority: 0,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
    );
    assert_eq!(
        projection.scheduler.trace_execution_id.as_deref(),
        Some("edit-session-1")
    );

    let trace = store
        .trace_snapshot(pantograph_workflow_service::WorkflowTraceSnapshotRequest {
            execution_id: Some("edit-session-1".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
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
fn scheduler_snapshot_event_carries_trace_execution_id_into_projection() {
    let store = WorkflowDiagnosticsStore::default();
    let projection = store.record_scheduler_snapshot(
        Some("wf-1".to_string()),
        "run-1".to_string(),
        "session-1".to_string(),
        5_000,
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: None,
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 2,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("run-1".to_string()),
            enqueued_at_ms: Some(100),
            dequeued_at_ms: Some(110),
            priority: 5,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
    );

    assert_eq!(projection.scheduler.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(
        projection.scheduler.session_id.as_deref(),
        Some("session-1")
    );
    assert_eq!(
        projection.scheduler.trace_execution_id.as_deref(),
        Some("run-1")
    );
    let run = projection.runs_by_id.get("run-1").expect("run trace");
    assert_eq!(run.session_id.as_deref(), Some("session-1"));
}

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
    store.update_runtime_snapshot(
        Some("wf-1".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        2_000,
    );
    store.update_scheduler_snapshot(
        Some("wf-1".to_string()),
        Some("exec-1".to_string()),
        None,
        Vec::new(),
        None,
        None,
        2_100,
    );

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

    let projection = store.record_scheduler_snapshot(
        Some("wf-1".to_string()),
        "restored-exec".to_string(),
        "session-1".to_string(),
        2_000,
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 1,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("restored-exec".to_string()),
            enqueued_at_ms: Some(1_950),
            dequeued_at_ms: Some(1_980),
            priority: 5,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
    );

    assert_eq!(projection.run_order, vec!["restored-exec".to_string()]);
    assert!(!projection.runs_by_id.contains_key("stale-exec"));
    assert_eq!(
        projection.scheduler.trace_execution_id.as_deref(),
        Some("restored-exec")
    );

    let runtime_projection = store.update_runtime_snapshot(
        Some("wf-1".to_string()),
        None,
        None,
        Some("/models/restarted.gguf".to_string()),
        None,
        Some(inference::RuntimeLifecycleSnapshot {
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
        None,
        2_010,
    );

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

    store.record_scheduler_snapshot(
        Some("wf-1".to_string()),
        "exec-1".to_string(),
        "session-1".to_string(),
        1_000,
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 1,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("exec-1".to_string()),
            enqueued_at_ms: Some(900),
            dequeued_at_ms: Some(930),
            priority: 1,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
    );
    store.record_runtime_snapshot(
        "wf-1".to_string(),
        "exec-1".to_string(),
        1_010,
        None,
        pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
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
        Some("/models/first.gguf".to_string()),
        None,
        Some(inference::RuntimeLifecycleSnapshot {
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
        None,
        None,
    );

    store.record_scheduler_snapshot(
        Some("wf-1".to_string()),
        "exec-1".to_string(),
        "session-1".to_string(),
        1_100,
        Some(pantograph_workflow_service::WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: pantograph_workflow_service::WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 1,
        }),
        vec![pantograph_workflow_service::WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("exec-1".to_string()),
            enqueued_at_ms: Some(900),
            dequeued_at_ms: Some(940),
            priority: 1,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        None,
    );

    store.record_runtime_snapshot(
        "wf-1".to_string(),
        "exec-1".to_string(),
        1_120,
        None,
        pantograph_workflow_service::WorkflowTraceRuntimeMetrics {
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
        Some("/models/replayed.gguf".to_string()),
        None,
        Some(inference::RuntimeLifecycleSnapshot {
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
        None,
        None,
    );

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
