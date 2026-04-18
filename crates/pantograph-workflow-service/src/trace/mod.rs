mod runtime;
mod scheduler;
mod store;
mod types;

pub use store::{WorkflowTraceRecordResult, WorkflowTraceStore};
pub use types::{
    WorkflowTraceEvent, WorkflowTraceGraphContext, WorkflowTraceNodeRecord,
    WorkflowTraceNodeStatus, WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics,
    WorkflowTraceRuntimeSelection, WorkflowTraceSnapshotRequest, WorkflowTraceSnapshotResponse,
    WorkflowTraceStatus, WorkflowTraceSummary,
};

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::runtime::{infer_runtime_id, runtime_lifecycle_reason};
    use super::*;
    use crate::workflow::{WorkflowSchedulerDecisionReason, WorkflowServiceError};
    use crate::{WorkflowSchedulerRuntimeCapacityPressure, WorkflowSchedulerSnapshotDiagnostics};

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
            execution_id: "exec-1".to_string(),
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            workflow_name: Some("Workflow".to_string()),
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
                    next_admission_queue_id: Some("queue-next".to_string()),
                    next_admission_bypassed_queue_id: None,
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
            }],
        })
        .expect("serialize trace summary");

        let expected = serde_json::json!({
            "execution_id": "exec-1",
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "workflow_name": "Workflow",
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
                    "next_admission_queue_id": "queue-next",
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
            execution_id: Some("exec-1".to_string()),
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            workflow_name: Some("Workflow 1".to_string()),
            include_completed: Some(true),
        };
        request.validate().expect("valid trace snapshot request");

        let value = serde_json::to_value(request).expect("serialize snapshot request");

        let expected = serde_json::json!({
            "execution_id": "exec-1",
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "workflow_name": "Workflow 1",
            "include_completed": true
        });

        assert_eq!(value, expected);
    }

    #[test]
    fn workflow_trace_snapshot_request_rejects_blank_filter_values() {
        let request = WorkflowTraceSnapshotRequest {
            execution_id: Some("   ".to_string()),
            session_id: None,
            workflow_id: None,
            workflow_name: None,
            include_completed: None,
        };

        let error = request
            .validate()
            .expect_err("blank execution_id should be rejected");
        assert!(
            matches!(
                error,
                WorkflowServiceError::InvalidRequest(ref message)
                    if message
                        == "workflow trace snapshot request field 'execution_id' must not be blank"
            ),
            "unexpected validation error: {:?}",
            error
        );
    }

    #[test]
    fn workflow_trace_snapshot_request_normalizes_trimmed_filters() {
        let normalized = WorkflowTraceSnapshotRequest {
            execution_id: Some("  exec-1  ".to_string()),
            session_id: Some("\tsession-1\t".to_string()),
            workflow_id: Some(" wf-1 ".to_string()),
            workflow_name: Some("  Workflow 1  ".to_string()),
            include_completed: Some(false),
        }
        .normalized();

        assert_eq!(normalized.execution_id.as_deref(), Some("exec-1"));
        assert_eq!(normalized.session_id.as_deref(), Some("session-1"));
        assert_eq!(normalized.workflow_id.as_deref(), Some("wf-1"));
        assert_eq!(normalized.workflow_name.as_deref(), Some("Workflow 1"));
        assert_eq!(normalized.include_completed, Some(false));
    }

    #[test]
    fn workflow_trace_store_records_run_and_node_timing() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Workflow".to_string()),
        );
        store.set_execution_graph_context(
            "exec-1",
            &WorkflowTraceGraphContext {
                graph_fingerprint: Some("graph-1".to_string()),
                node_count_at_start: 1,
                node_types_by_id: HashMap::from([(
                    "node-1".to_string(),
                    "llm-inference".to_string(),
                )]),
            },
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            1_000,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            1_010,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStream {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
            },
            1_030,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeCompleted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
            },
            1_050,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
            },
            1_100,
        );

        let trace = snapshot.traces.first().expect("trace summary");
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow"));
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
            },
            150,
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-2".to_string(),
                workflow_id: Some("wf-2".to_string()),
                node_count: 0,
            },
            200,
        );

        let filtered = store
            .snapshot(&WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: None,
                workflow_id: None,
                workflow_name: None,
                include_completed: Some(false),
            })
            .expect("filtered snapshot");

        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].execution_id, "exec-2");
        assert_eq!(filtered.traces[0].status, WorkflowTraceStatus::Running);
    }

    #[test]
    fn workflow_trace_store_filters_by_session_id_when_execution_differs() {
        let store = WorkflowTraceStore::new(10);
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "run-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "run-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 105,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("run-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(105),
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
                error: None,
            },
            105,
        );

        let filtered = store
            .snapshot(&WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: None,
                workflow_name: None,
                include_completed: None,
            })
            .expect("session-filtered snapshot");

        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].execution_id, "run-1");
        assert_eq!(filtered.traces[0].session_id.as_deref(), Some("session-1"));
    }

    #[test]
    fn workflow_trace_store_filters_by_workflow_name() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "run-1",
            Some("wf-1".to_string()),
            Some("Workflow 1".to_string()),
        );
        store.set_execution_metadata(
            "run-2",
            Some("wf-2".to_string()),
            Some("Workflow 2".to_string()),
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "run-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "run-2".to_string(),
                workflow_id: Some("wf-2".to_string()),
                node_count: 0,
            },
            120,
        );

        let filtered = store
            .snapshot(&WorkflowTraceSnapshotRequest {
                execution_id: None,
                session_id: None,
                workflow_id: None,
                workflow_name: Some("Workflow 1".to_string()),
                include_completed: None,
            })
            .expect("workflow-name-filtered snapshot");

        assert_eq!(filtered.traces.len(), 1);
        assert_eq!(filtered.traces[0].execution_id, "run-1");
        assert_eq!(
            filtered.traces[0].workflow_name.as_deref(),
            Some("Workflow 1")
        );
    }

    #[test]
    fn workflow_trace_store_enforces_retention_limit() {
        let store = WorkflowTraceStore::new(1);
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-2".to_string(),
                workflow_id: Some("wf-2".to_string()),
                node_count: 0,
            },
            200,
        );

        assert_eq!(snapshot.retained_trace_limit, 1);
        assert_eq!(snapshot.traces.len(), 1);
        assert_eq!(snapshot.traces[0].execution_id, "exec-2");
    }

    #[test]
    fn workflow_trace_store_records_queue_and_runtime_snapshot_metrics() {
        let store = WorkflowTraceStore::new(10);
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 90,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::IdleLoaded,
                    queued_runs: 1,
                    run_count: 2,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(80),
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Pending,
                }],
                diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                    loaded_session_count: 1,
                    max_loaded_sessions: 2,
                    reclaimable_loaded_session_count: 1,
                    runtime_capacity_pressure:
                        WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                    active_run_blocks_admission: true,
                    next_admission_queue_id: Some("queue-1".to_string()),
                    next_admission_bypassed_queue_id: None,
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-1".to_string(),
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
                        required_extensions: vec!["kv-cache".to_string()],
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 120,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 3,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(80),
                    dequeued_at_ms: Some(115),
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: Some(WorkflowSchedulerSnapshotDiagnostics {
                    loaded_session_count: 1,
                    max_loaded_sessions: 2,
                    reclaimable_loaded_session_count: 0,
                    runtime_capacity_pressure: WorkflowSchedulerRuntimeCapacityPressure::Available,
                    active_run_blocks_admission: false,
                    next_admission_queue_id: None,
                    next_admission_bypassed_queue_id: None,
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
                next_admission_queue_id: None,
                next_admission_bypassed_queue_id: None,
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

    #[test]
    fn workflow_trace_store_resets_attempt_state_when_run_restarts_after_failure() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Workflow".to_string()),
        );
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            110,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeFailed {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                error: "boom".to_string(),
            },
            120,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-1".to_string(),
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 126,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: false,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(90),
                    dequeued_at_ms: Some(100),
                    priority: 0,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
                error: None,
            },
            126,
        );
        store.record_event(
            &WorkflowTraceEvent::RunFailed {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                error: "boom".to_string(),
            },
            130,
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 2,
            },
            200,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-2".to_string(),
                node_type: None,
            },
            210,
        );

        let trace = snapshot.traces.first().expect("trace");
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow"));
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
    fn workflow_trace_store_tracks_observed_runtime_ids_across_runtime_snapshots() {
        let store = WorkflowTraceStore::new(10);
        store.set_execution_metadata(
            "exec-mixed",
            Some("wf-mixed".to_string()),
            Some("Workflow".to_string()),
        );
        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-mixed".to_string(),
                workflow_id: Some("wf-mixed".to_string()),
                node_count: 0,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-mixed".to_string(),
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
                execution_id: "exec-mixed".to_string(),
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
                execution_id: Some("exec-mixed".to_string()),
                session_id: None,
                workflow_id: None,
                workflow_name: None,
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
            },
            110,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
            },
            110,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunCancelled {
                execution_id: "exec-1".to_string(),
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
        store.set_execution_metadata(
            "exec-1",
            Some("wf-1".to_string()),
            Some("Workflow".to_string()),
        );
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: None,
            },
            110,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCancelled {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                error: "workflow run cancelled during execution".to_string(),
            },
            130,
        );

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 2,
            },
            200,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-2".to_string(),
                node_type: None,
            },
            210,
        );

        let trace = snapshot.traces.first().expect("trace");
        assert_eq!(trace.workflow_name.as_deref(), Some("Workflow"));
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
            },
            140,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::RunCompleted {
                execution_id: "exec-1".to_string(),
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
    fn workflow_trace_store_ignores_duplicate_node_failed_events() {
        let store = WorkflowTraceStore::new(10);

        store.record_event(
            &WorkflowTraceEvent::RunStarted {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeStarted {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                node_type: Some("llm-inference".to_string()),
            },
            110,
        );
        store.record_event(
            &WorkflowTraceEvent::NodeFailed {
                execution_id: "exec-1".to_string(),
                node_id: "node-1".to_string(),
                error: "boom".to_string(),
            },
            140,
        );
        let snapshot = store.record_event(
            &WorkflowTraceEvent::NodeFailed {
                execution_id: "exec-1".to_string(),
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
                execution_id: "exec-target".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 200,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: None,
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 2,
                    run_count: 3,
                }),
                items: vec![
                    crate::workflow::WorkflowSessionQueueItem {
                        queue_id: "queue-other".to_string(),
                        run_id: Some("other-run".to_string()),
                        enqueued_at_ms: Some(100),
                        dequeued_at_ms: Some(150),
                        priority: 10,
                        queue_position: None,
                        scheduler_admission_outcome: None,
                        scheduler_decision_reason: None,
                        status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                    },
                    crate::workflow::WorkflowSessionQueueItem {
                        queue_id: "queue-target".to_string(),
                        run_id: Some("exec-target".to_string()),
                        enqueued_at_ms: Some(180),
                        dequeued_at_ms: None,
                        priority: 5,
                        queue_position: None,
                        scheduler_admission_outcome: None,
                        scheduler_decision_reason: None,
                        status: crate::workflow::WorkflowSessionQueueItemStatus::Pending,
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
                execution_id: "edit-session-1".to_string(),
                workflow_id: None,
                session_id: "edit-session-1".to_string(),
                captured_at_ms: 5_000,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "edit-session-1".to_string(),
                    workflow_id: "edit-session-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Edit,
                    usage_profile: None,
                    keep_alive: false,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 1,
                    run_count: 2,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "edit-session-1".to_string(),
                    run_id: Some("edit-session-1".to_string()),
                    enqueued_at_ms: Some(4_750),
                    dequeued_at_ms: Some(4_750),
                    priority: 0,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 200,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: None,
                    dequeued_at_ms: None,
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
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
                execution_id: "exec-target".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 200,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 2,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "session-1".to_string(),
                    run_id: Some("other-run".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(120),
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: Some(
                        crate::workflow::WorkflowSchedulerDecisionReason::WarmSessionReused,
                    ),
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 200,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(120),
                    priority: 5,
                    queue_position: None,
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: Some(
                        crate::workflow::WorkflowSchedulerDecisionReason::WarmSessionReused,
                    ),
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
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
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                node_count: 1,
            },
            100,
        );
        store.record_event(
            &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                execution_id: "exec-1".to_string(),
                workflow_id: Some("wf-1".to_string()),
                session_id: "session-1".to_string(),
                captured_at_ms: 110,
                session: Some(crate::workflow::WorkflowSessionSummary {
                    session_id: "session-1".to_string(),
                    workflow_id: "wf-1".to_string(),
                    session_kind: crate::graph::WorkflowSessionKind::Workflow,
                    usage_profile: Some("interactive".to_string()),
                    keep_alive: true,
                    state: crate::workflow::WorkflowSessionState::Running,
                    queued_runs: 0,
                    run_count: 1,
                }),
                items: vec![crate::workflow::WorkflowSessionQueueItem {
                    queue_id: "queue-1".to_string(),
                    run_id: Some("exec-1".to_string()),
                    enqueued_at_ms: Some(100),
                    dequeued_at_ms: Some(110),
                    priority: 5,
                    queue_position: Some(0),
                    scheduler_admission_outcome: None,
                    scheduler_decision_reason: None,
                    status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                }],
                diagnostics: None,
                error: None,
            },
            110,
        );
        store.record_event(
            &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                execution_id: "exec-1".to_string(),
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
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: Some("wf-1".to_string()),
                workflow_name: None,
                include_completed: Some(true),
            })
            .expect("runtime selection");

        assert_eq!(selection.execution_id.as_deref(), Some("exec-1"));
        assert_eq!(selection.matched_execution_ids, vec!["exec-1".to_string()]);
        assert!(!selection.is_ambiguous());
        assert_eq!(
            selection.runtime.and_then(|runtime| runtime.runtime_id),
            Some("llama_cpp".to_string())
        );
    }

    #[test]
    fn workflow_trace_store_marks_runtime_metric_selection_ambiguous_for_multi_run_scope() {
        let store = WorkflowTraceStore::new(10);
        for (execution_id, runtime_id, captured_at_ms) in [
            ("exec-1", "llama_cpp", 120_u64),
            ("exec-2", "llama_cpp.embedding", 220_u64),
        ] {
            store.record_event(
                &WorkflowTraceEvent::RunStarted {
                    execution_id: execution_id.to_string(),
                    workflow_id: Some("wf-1".to_string()),
                    node_count: 1,
                },
                captured_at_ms.saturating_sub(20),
            );
            store.record_event(
                &WorkflowTraceEvent::SchedulerSnapshotCaptured {
                    execution_id: execution_id.to_string(),
                    workflow_id: Some("wf-1".to_string()),
                    session_id: "session-1".to_string(),
                    captured_at_ms: captured_at_ms.saturating_sub(10),
                    session: Some(crate::workflow::WorkflowSessionSummary {
                        session_id: "session-1".to_string(),
                        workflow_id: "wf-1".to_string(),
                        session_kind: crate::graph::WorkflowSessionKind::Workflow,
                        usage_profile: Some("interactive".to_string()),
                        keep_alive: true,
                        state: crate::workflow::WorkflowSessionState::Running,
                        queued_runs: 0,
                        run_count: 2,
                    }),
                    items: vec![crate::workflow::WorkflowSessionQueueItem {
                        queue_id: format!("queue-{execution_id}"),
                        run_id: Some(execution_id.to_string()),
                        enqueued_at_ms: Some(captured_at_ms.saturating_sub(20)),
                        dequeued_at_ms: Some(captured_at_ms.saturating_sub(10)),
                        priority: 5,
                        queue_position: Some(0),
                        scheduler_admission_outcome: None,
                        scheduler_decision_reason: None,
                        status: crate::workflow::WorkflowSessionQueueItemStatus::Running,
                    }],
                    diagnostics: None,
                    error: None,
                },
                captured_at_ms.saturating_sub(10),
            );
            store.record_event(
                &WorkflowTraceEvent::RuntimeSnapshotCaptured {
                    execution_id: execution_id.to_string(),
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
                execution_id: None,
                session_id: Some("session-1".to_string()),
                workflow_id: Some("wf-1".to_string()),
                workflow_name: None,
                include_completed: Some(true),
            })
            .expect("runtime selection");

        assert_eq!(selection.execution_id, None);
        assert_eq!(selection.runtime, None);
        assert!(selection.is_ambiguous());
        assert_eq!(
            selection.matched_execution_ids,
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
            execution_id: "exec-1".to_string(),
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
}
