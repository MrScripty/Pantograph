use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::runtime::{infer_runtime_id, runtime_lifecycle_reason};
use super::*;
use crate::workflow::{WorkflowSchedulerDecisionReason, WorkflowServiceError};
use crate::{WorkflowSchedulerRuntimeCapacityPressure, WorkflowSchedulerSnapshotDiagnostics};

mod lifecycle;
mod scheduler_runtime;
mod timing;

fn workflow_capabilities_with_runtimes(
    runtime_requirements: crate::workflow::WorkflowRuntimeRequirements,
    runtime_capabilities: Vec<crate::workflow::WorkflowRuntimeCapability>,
) -> crate::workflow::WorkflowCapabilitiesResponse {
    crate::workflow::WorkflowCapabilitiesResponse {
        max_input_bindings: 4,
        max_output_targets: 2,
        max_value_bytes: 2_048,
        runtime_requirements,
        models: Vec::new(),
        runtime_capabilities,
    }
}

#[test]
fn workflow_trace_summary_serializes_with_snake_case_contract() {
    let value = serde_json::to_value(WorkflowTraceSummary {
        workflow_run_id: "exec-1".to_string(),
        session_id: Some("session-1".to_string()),
        workflow_id: Some("wf-1".to_string()),
        graph_fingerprint: Some("graph-1".to_string()),
        status: WorkflowTraceStatus::Running,
        started_at_ms: 100,
        ended_at_ms: Some(200),
        duration_ms: Some(100),
        queue: WorkflowTraceQueueMetrics {
            enqueued_at_ms: Some(80),
            dequeued_at_ms: Some(100),
            queue_wait_ms: Some(20),
            scheduler_admission_outcome: Some("admitted".to_string()),
            scheduler_decision_reason: Some("warm_session_reused".to_string()),
            scheduler_snapshot_diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                loaded_session_count: 1,
                max_loaded_sessions: 2,
                reclaimable_loaded_session_count: 1,
                runtime_capacity_pressure:
                    WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                active_run_blocks_admission: false,
                next_admission_workflow_run_id: Some("queue-next".to_string()),
                next_admission_bypassed_workflow_run_id: None,
                next_admission_after_runs: Some(0),
                next_admission_wait_ms: Some(0),
                next_admission_not_before_ms: Some(100),
                next_admission_reason: Some(WorkflowSchedulerDecisionReason::WarmSessionReused),
                runtime_registry: None,
            }),
        },
        runtime: WorkflowTraceRuntimeMetrics {
            runtime_id: Some("llama_cpp".to_string()),
            observed_runtime_ids: vec!["llama_cpp".to_string()],
            runtime_instance_id: Some("runtime-1".to_string()),
            model_target: Some("llava:13b".to_string()),
            warmup_started_at_ms: Some(90),
            warmup_completed_at_ms: Some(99),
            warmup_duration_ms: Some(9),
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("already_ready".to_string()),
        },
        node_count_at_start: 2,
        event_count: 3,
        stream_event_count: 1,
        last_dirty_tasks: vec!["merge".to_string(), "output".to_string()],
        last_incremental_task_ids: vec!["output".to_string()],
        last_graph_memory_impact: Some(node_engine::GraphMemoryImpactSummary {
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
        }),
        waiting_for_input: false,
        last_error: None,
        nodes: vec![WorkflowTraceNodeRecord {
            node_id: "node-1".to_string(),
            node_type: Some("llm-inference".to_string()),
            status: WorkflowTraceNodeStatus::Completed,
            started_at_ms: Some(110),
            ended_at_ms: Some(180),
            duration_ms: Some(70),
            event_count: 2,
            stream_event_count: 1,
            last_error: None,
            last_progress_detail: None,
            timing_expectation: None,
        }],
        timing_expectation: None,
    })
    .expect("serialize trace summary");

    let expected = serde_json::json!({
        "workflow_run_id": "exec-1",
        "session_id": "session-1",
        "workflow_id": "wf-1",
        "graph_fingerprint": "graph-1",
        "status": "running",
        "started_at_ms": 100,
        "ended_at_ms": 200,
        "duration_ms": 100,
        "queue": {
            "enqueued_at_ms": 80,
            "dequeued_at_ms": 100,
            "queue_wait_ms": 20,
            "scheduler_admission_outcome": "admitted",
            "scheduler_decision_reason": "warm_session_reused",
            "scheduler_snapshot_diagnostics": {
                "loaded_session_count": 1,
                "max_loaded_sessions": 2,
                "reclaimable_loaded_session_count": 1,
                "runtime_capacity_pressure": "rebalance_required",
                "active_run_blocks_admission": false,
                "next_admission_workflow_run_id": "queue-next",
                "next_admission_after_runs": 0,
                "next_admission_wait_ms": 0,
                "next_admission_not_before_ms": 100,
                "next_admission_reason": "warm_session_reused"
            }
        },
        "runtime": {
            "runtime_id": "llama_cpp",
            "observed_runtime_ids": ["llama_cpp"],
            "runtime_instance_id": "runtime-1",
            "model_target": "llava:13b",
            "warmup_started_at_ms": 90,
            "warmup_completed_at_ms": 99,
            "warmup_duration_ms": 9,
            "runtime_reused": true,
            "lifecycle_decision_reason": "already_ready"
        },
        "node_count_at_start": 2,
        "event_count": 3,
        "stream_event_count": 1,
        "last_dirty_tasks": ["merge", "output"],
        "last_incremental_task_ids": ["output"],
        "last_graph_memory_impact": {
            "node_decisions": [
                {
                    "node_id": "merge",
                    "compatibility": "preserve_with_input_refresh",
                    "reason": "upstream input updated"
                },
                {
                    "node_id": "output",
                    "compatibility": "fallback_full_invalidation",
                    "reason": "compatibility unknown"
                }
            ],
            "fallback_to_full_invalidation": true
        },
        "waiting_for_input": false,
        "last_error": null,
        "nodes": [{
            "node_id": "node-1",
            "node_type": "llm-inference",
            "status": "completed",
            "started_at_ms": 110,
            "ended_at_ms": 180,
            "duration_ms": 70,
            "event_count": 2,
            "stream_event_count": 1,
            "last_error": null
        }]
    });

    assert_eq!(value, expected);
}

#[test]
fn infer_runtime_id_prefers_selected_runtime_capability() {
    let capabilities = workflow_capabilities_with_runtimes(
        crate::workflow::WorkflowRuntimeRequirements {
            required_backends: vec!["llama_cpp".to_string()],
            ..crate::workflow::WorkflowRuntimeRequirements::default()
        },
        vec![
            crate::workflow::WorkflowRuntimeCapability {
                runtime_id: "llama_cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                install_state: crate::workflow::WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: crate::workflow::WorkflowRuntimeSourceKind::Managed,
                selected: false,
                readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["llama_cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
            crate::workflow::WorkflowRuntimeCapability {
                runtime_id: "pytorch".to_string(),
                display_name: "PyTorch (Python sidecar)".to_string(),
                install_state: crate::workflow::WorkflowRuntimeInstallState::SystemProvided,
                available: true,
                configured: true,
                can_install: false,
                can_remove: false,
                source_kind: crate::workflow::WorkflowRuntimeSourceKind::System,
                selected: true,
                readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
                selected_version: None,
                supports_external_connection: false,
                backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            },
        ],
    );

    assert_eq!(infer_runtime_id(&capabilities).as_deref(), Some("pytorch"));
}

#[test]
fn infer_runtime_id_normalizes_selected_runtime_display_name() {
    let capabilities = workflow_capabilities_with_runtimes(
        crate::workflow::WorkflowRuntimeRequirements::default(),
        vec![crate::workflow::WorkflowRuntimeCapability {
            runtime_id: "PyTorch".to_string(),
            display_name: "PyTorch".to_string(),
            install_state: crate::workflow::WorkflowRuntimeInstallState::SystemProvided,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: crate::workflow::WorkflowRuntimeSourceKind::System,
            selected: true,
            readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
            selected_version: None,
            supports_external_connection: false,
            backend_keys: vec!["pytorch".to_string(), "torch".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }],
    );

    assert_eq!(infer_runtime_id(&capabilities).as_deref(), Some("pytorch"));
}

#[test]
fn infer_runtime_id_matches_single_required_backend_alias_to_runtime_capability() {
    let capabilities = workflow_capabilities_with_runtimes(
        crate::workflow::WorkflowRuntimeRequirements {
            required_backends: vec!["onnxruntime".to_string()],
            ..crate::workflow::WorkflowRuntimeRequirements::default()
        },
        vec![crate::workflow::WorkflowRuntimeCapability {
            runtime_id: "onnx-runtime".to_string(),
            display_name: "ONNX Runtime (Python sidecar)".to_string(),
            install_state: crate::workflow::WorkflowRuntimeInstallState::SystemProvided,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: crate::workflow::WorkflowRuntimeSourceKind::System,
            selected: false,
            readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
            selected_version: None,
            supports_external_connection: false,
            backend_keys: vec!["ONNX Runtime".to_string(), "onnx-runtime".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }],
    );

    assert_eq!(
        infer_runtime_id(&capabilities).as_deref(),
        Some("onnx-runtime")
    );
    assert_eq!(
        runtime_lifecycle_reason(&capabilities),
        "required_runtime_reported"
    );
}

#[test]
fn infer_runtime_id_normalizes_required_backend_fallback_alias() {
    let capabilities = workflow_capabilities_with_runtimes(
        crate::workflow::WorkflowRuntimeRequirements {
            required_backends: vec!["onnxruntime".to_string()],
            ..crate::workflow::WorkflowRuntimeRequirements::default()
        },
        Vec::new(),
    );

    assert_eq!(
        infer_runtime_id(&capabilities).as_deref(),
        Some("onnx-runtime")
    );
}

#[test]
fn runtime_lifecycle_reason_prefers_selected_runtime_metadata() {
    let capabilities = workflow_capabilities_with_runtimes(
        crate::workflow::WorkflowRuntimeRequirements::default(),
        vec![crate::workflow::WorkflowRuntimeCapability {
            runtime_id: "remote-llama".to_string(),
            display_name: "Remote llama.cpp".to_string(),
            install_state: crate::workflow::WorkflowRuntimeInstallState::Installed,
            available: true,
            configured: true,
            can_install: false,
            can_remove: false,
            source_kind: crate::workflow::WorkflowRuntimeSourceKind::Host,
            selected: true,
            readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
            selected_version: None,
            supports_external_connection: false,
            backend_keys: vec!["llama_cpp".to_string()],
            missing_files: Vec::new(),
            unavailable_reason: None,
        }],
    );

    assert_eq!(
        runtime_lifecycle_reason(&capabilities),
        "selected_runtime_reported"
    );
}

#[test]
fn workflow_trace_snapshot_request_serializes_optional_filters() {
    let request = WorkflowTraceSnapshotRequest {
        workflow_run_id: Some("exec-1".to_string()),
        session_id: Some("session-1".to_string()),
        workflow_id: Some("wf-1".to_string()),
        include_completed: Some(true),
    };
    request.validate().expect("valid trace snapshot request");

    let value = serde_json::to_value(request).expect("serialize snapshot request");

    let expected = serde_json::json!({
        "workflow_run_id": "exec-1",
        "session_id": "session-1",
        "workflow_id": "wf-1",
        "include_completed": true
    });

    assert_eq!(value, expected);
}

#[test]
fn workflow_trace_snapshot_request_rejects_blank_filter_values() {
    let request = WorkflowTraceSnapshotRequest {
        workflow_run_id: Some("   ".to_string()),
        session_id: None,
        workflow_id: None,
        include_completed: None,
    };

    let error = request
        .validate()
        .expect_err("blank workflow_run_id should be rejected");
    assert!(
        matches!(
            error,
            WorkflowServiceError::InvalidRequest(ref message)
                if message
                    == "workflow trace snapshot request field 'workflow_run_id' must not be blank"
        ),
        "unexpected validation error: {:?}",
        error
    );
}

#[test]
fn workflow_trace_snapshot_request_normalizes_trimmed_filters() {
    let normalized = WorkflowTraceSnapshotRequest {
        workflow_run_id: Some("  exec-1  ".to_string()),
        session_id: Some("\tsession-1\t".to_string()),
        workflow_id: Some(" wf-1 ".to_string()),
        include_completed: Some(false),
    }
    .normalized();

    assert_eq!(normalized.workflow_run_id.as_deref(), Some("exec-1"));
    assert_eq!(normalized.session_id.as_deref(), Some("session-1"));
    assert_eq!(normalized.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(normalized.include_completed, Some(false));
}

#[test]
fn workflow_trace_store_records_run_and_node_timing() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph_context(
        "exec-1",
        &WorkflowTraceGraphContext {
            graph_fingerprint: Some("graph-1".to_string()),
            node_count_at_start: 1,
            node_types_by_id: HashMap::from([("node-1".to_string(), "llm-inference".to_string())]),
        },
    );

    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 1,
        },
        1_000,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStarted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
            node_type: None,
        },
        1_010,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeStream {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
        },
        1_030,
    );
    store.record_event(
        &WorkflowTraceEvent::NodeCompleted {
            workflow_run_id: "exec-1".to_string(),
            node_id: "node-1".to_string(),
        },
        1_050,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::RunCompleted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
        },
        1_100,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.workflow_id.as_deref(), Some("wf-1"));
    assert_eq!(trace.graph_fingerprint.as_deref(), Some("graph-1"));
    assert_eq!(trace.status, WorkflowTraceStatus::Completed);
    assert_eq!(trace.duration_ms, Some(100));
    assert_eq!(trace.event_count, 5);
    assert_eq!(trace.stream_event_count, 1);

    let node = trace.nodes.first().expect("node summary");
    assert_eq!(node.node_type.as_deref(), Some("llm-inference"));
    assert_eq!(node.status, WorkflowTraceNodeStatus::Completed);
    assert_eq!(node.duration_ms, Some(40));
    assert_eq!(node.stream_event_count, 1);
}

#[test]
fn workflow_trace_store_filters_completed_runs() {
    let store = WorkflowTraceStore::new(10);
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 0,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::RunCompleted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
        },
        150,
    );
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-2".to_string(),
            workflow_id: Some("wf-2".to_string()),
            node_count: 0,
        },
        200,
    );

    let filtered = store
        .snapshot(&WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: None,
            workflow_id: None,
            include_completed: Some(false),
        })
        .expect("filtered snapshot");

    assert_eq!(filtered.traces.len(), 1);
    assert_eq!(filtered.traces[0].workflow_run_id, "exec-2");
    assert_eq!(filtered.traces[0].status, WorkflowTraceStatus::Running);
}

#[test]
fn workflow_trace_store_filters_by_session_id_when_execution_differs() {
    let store = WorkflowTraceStore::new(10);
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "run-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 0,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "run-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 105,
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
                workflow_run_id: "run-1".to_string(),
                enqueued_at_ms: Some(100),
                dequeued_at_ms: Some(105),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: None,
            error: None,
        },
        105,
    );

    let filtered = store
        .snapshot(&WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: Some("session-1".to_string()),
            workflow_id: None,
            include_completed: None,
        })
        .expect("session-filtered snapshot");

    assert_eq!(filtered.traces.len(), 1);
    assert_eq!(filtered.traces[0].workflow_run_id, "run-1");
    assert_eq!(filtered.traces[0].session_id.as_deref(), Some("session-1"));
}

#[test]
fn workflow_trace_store_filters_by_workflow_id() {
    let store = WorkflowTraceStore::new(10);
    store.set_execution_metadata("run-1", Some("wf-1".to_string()));
    store.set_execution_metadata("run-2", Some("wf-2".to_string()));
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "run-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 0,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "run-2".to_string(),
            workflow_id: Some("wf-2".to_string()),
            node_count: 0,
        },
        120,
    );

    let filtered = store
        .snapshot(&WorkflowTraceSnapshotRequest {
            workflow_run_id: None,
            session_id: None,
            workflow_id: Some("wf-1".to_string()),
            include_completed: None,
        })
        .expect("workflow-id-filtered snapshot");

    assert_eq!(filtered.traces.len(), 1);
    assert_eq!(filtered.traces[0].workflow_run_id, "run-1");
    assert_eq!(filtered.traces[0].workflow_id.as_deref(), Some("wf-1"));
}

#[test]
fn workflow_trace_store_enforces_retention_limit() {
    let store = WorkflowTraceStore::new(1);
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 0,
        },
        100,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-2".to_string(),
            workflow_id: Some("wf-2".to_string()),
            node_count: 0,
        },
        200,
    );

    assert_eq!(snapshot.retained_trace_limit, 1);
    assert_eq!(snapshot.traces.len(), 1);
    assert_eq!(snapshot.traces[0].workflow_run_id, "exec-2");
}

#[test]
fn workflow_trace_store_records_queue_and_runtime_snapshot_metrics() {
    let store = WorkflowTraceStore::new(10);
    store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 90,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: None,
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::IdleLoaded,
                queued_runs: 1,
                run_count: 2,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: Some(80),
                dequeued_at_ms: None,
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Pending,
            }],
            diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                loaded_session_count: 1,
                max_loaded_sessions: 2,
                reclaimable_loaded_session_count: 1,
                runtime_capacity_pressure:
                    WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                active_run_blocks_admission: true,
                next_admission_workflow_run_id: Some("queue-1".to_string()),
                next_admission_bypassed_workflow_run_id: None,
                next_admission_after_runs: Some(1),
                next_admission_wait_ms: None,
                next_admission_not_before_ms: None,
                next_admission_reason: Some(WorkflowSchedulerDecisionReason::WarmSessionReused),
                runtime_registry: None,
            }),
            error: None,
        },
        90,
    );
    store.record_event(
        &WorkflowTraceEvent::RunStarted {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            node_count: 0,
        },
        100,
    );
    store.record_event(
        &WorkflowTraceEvent::RuntimeSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            captured_at_ms: 110,
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("llama_cpp-1".to_string()),
                model_target: Some("/models/demo.gguf".to_string()),
                warmup_started_at_ms: Some(100),
                warmup_completed_at_ms: Some(110),
                warmup_duration_ms: Some(10),
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("runtime_ready".to_string()),
            },
            capabilities: Some(crate::workflow::WorkflowCapabilitiesResponse {
                max_input_bindings: 4,
                max_output_targets: 2,
                max_value_bytes: 2_048,
                runtime_requirements: crate::workflow::WorkflowRuntimeRequirements {
                    estimated_peak_vram_mb: None,
                    estimated_peak_ram_mb: None,
                    estimated_min_vram_mb: None,
                    estimated_min_ram_mb: None,
                    estimation_confidence: "high".to_string(),
                    required_models: vec!["model-a".to_string()],
                    required_backends: vec!["llama_cpp".to_string()],
                    required_extensions: vec!["kv_cache".to_string()],
                },
                models: Vec::new(),
                runtime_capabilities: vec![crate::workflow::WorkflowRuntimeCapability {
                    runtime_id: "llama_cpp".to_string(),
                    display_name: "llama.cpp".to_string(),
                    install_state: crate::workflow::WorkflowRuntimeInstallState::Installed,
                    available: true,
                    configured: true,
                    can_install: false,
                    can_remove: false,
                    source_kind: crate::workflow::WorkflowRuntimeSourceKind::Managed,
                    selected: true,
                    readiness_state: Some(crate::workflow::WorkflowRuntimeReadinessState::Ready),
                    selected_version: None,
                    supports_external_connection: true,
                    backend_keys: vec!["llama_cpp".to_string()],
                    missing_files: Vec::new(),
                    unavailable_reason: None,
                }],
            }),
            error: None,
        },
        110,
    );
    let snapshot = store.record_event(
        &WorkflowTraceEvent::SchedulerSnapshotCaptured {
            workflow_run_id: "exec-1".to_string(),
            workflow_id: Some("wf-1".to_string()),
            session_id: "session-1".to_string(),
            captured_at_ms: 120,
            session: Some(crate::workflow::WorkflowExecutionSessionSummary {
                session_id: "session-1".to_string(),
                workflow_id: "wf-1".to_string(),
                session_kind: crate::graph::WorkflowExecutionSessionKind::Workflow,
                usage_profile: None,
                attribution: None,
                keep_alive: true,
                state: crate::workflow::WorkflowExecutionSessionState::Running,
                queued_runs: 0,
                run_count: 3,
            }),
            items: vec![crate::workflow::WorkflowExecutionSessionQueueItem {
                workflow_run_id: "exec-1".to_string(),
                enqueued_at_ms: Some(80),
                dequeued_at_ms: Some(115),
                priority: 5,
                queue_position: None,
                scheduler_admission_outcome: None,
                scheduler_decision_reason: None,
                status: crate::workflow::WorkflowExecutionSessionQueueItemStatus::Running,
            }],
            diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                loaded_session_count: 1,
                max_loaded_sessions: 2,
                reclaimable_loaded_session_count: 0,
                runtime_capacity_pressure: WorkflowSchedulerRuntimeCapacityPressure::Available,
                active_run_blocks_admission: false,
                next_admission_workflow_run_id: None,
                next_admission_bypassed_workflow_run_id: None,
                next_admission_after_runs: None,
                next_admission_wait_ms: None,
                next_admission_not_before_ms: None,
                next_admission_reason: None,
                runtime_registry: None,
            }),
            error: None,
        },
        120,
    );

    let trace = snapshot.traces.first().expect("trace summary");
    assert_eq!(trace.status, WorkflowTraceStatus::Running);
    assert_eq!(trace.queue.enqueued_at_ms, Some(80));
    assert_eq!(trace.queue.dequeued_at_ms, Some(115));
    assert_eq!(trace.queue.queue_wait_ms, Some(35));
    assert_eq!(
        trace.queue.scheduler_admission_outcome.as_deref(),
        Some("admitted")
    );
    assert_eq!(
        trace.queue.scheduler_decision_reason.as_deref(),
        Some("matched_running_item")
    );
    assert_eq!(
        trace.queue.scheduler_snapshot_diagnostics,
        Some(WorkflowSchedulerSnapshotDiagnostics {
            loaded_session_count: 1,
            max_loaded_sessions: 2,
            reclaimable_loaded_session_count: 0,
            runtime_capacity_pressure: WorkflowSchedulerRuntimeCapacityPressure::Available,
            active_run_blocks_admission: false,
            next_admission_workflow_run_id: None,
            next_admission_bypassed_workflow_run_id: None,
            next_admission_after_runs: None,
            next_admission_wait_ms: None,
            next_admission_not_before_ms: None,
            next_admission_reason: None,
            runtime_registry: None,
        })
    );
    assert_eq!(trace.runtime.runtime_id.as_deref(), Some("llama_cpp"));
    assert_eq!(
        trace.runtime.runtime_instance_id.as_deref(),
        Some("llama_cpp-1")
    );
    assert_eq!(
        trace.runtime.model_target.as_deref(),
        Some("/models/demo.gguf")
    );
    assert_eq!(trace.runtime.warmup_started_at_ms, Some(100));
    assert_eq!(trace.runtime.warmup_completed_at_ms, Some(110));
    assert_eq!(trace.runtime.warmup_duration_ms, Some(10));
    assert_eq!(trace.runtime.runtime_reused, Some(false));
    assert_eq!(
        trace.runtime.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}
