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

#[test]
fn workflow_diagnostics_snapshot_request_normalizes_blank_filters() {
    let normalized = WorkflowDiagnosticsSnapshotRequest {
        session_id: Some("  session-1  ".to_string()),
        workflow_id: Some("   ".to_string()),
        workflow_name: Some("\tWorkflow 1\t".to_string()),
    }
    .normalized();

    assert_eq!(normalized.session_id.as_deref(), Some("session-1"));
    assert_eq!(normalized.workflow_id, None);
    assert_eq!(normalized.workflow_name.as_deref(), Some("Workflow 1"));
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
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
        None,
        6_000,
    );

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
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
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
            status: pantograph_workflow_service::WorkflowSessionQueueItemStatus::Running,
        }],
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
        2_100,
    );

    let snapshot = store.clear_history();

    assert!(snapshot.runs_by_id.is_empty());
    assert!(snapshot.run_order.is_empty());
    assert_eq!(snapshot.runtime.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(snapshot.scheduler.session_id.as_deref(), Some("exec-1"));
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
