use async_trait::async_trait;
use pantograph_workflow_service::graph::WorkflowSessionKind;
use pantograph_workflow_service::{
    WorkflowCapabilitiesRequest, WorkflowCapabilityModel, WorkflowHost, WorkflowHostCapabilities,
    WorkflowIoNode, WorkflowIoPort, WorkflowIoRequest, WorkflowIoResponse, WorkflowOutputTarget,
    WorkflowPortBinding, WorkflowPreflightRequest, WorkflowRunRequest, WorkflowRuntimeCapability,
    WorkflowRuntimeInstallState, WorkflowRuntimeRequirements, WorkflowRuntimeSourceKind,
    WorkflowSchedulerSnapshotResponse, WorkflowService, WorkflowServiceError,
    WorkflowSessionQueueItem, WorkflowSessionQueueItemStatus, WorkflowSessionState,
    WorkflowSessionSummary, WorkflowTraceNodeRecord, WorkflowTraceNodeStatus,
    WorkflowTraceQueueMetrics, WorkflowTraceRuntimeMetrics, WorkflowTraceSnapshotRequest,
    WorkflowTraceSnapshotResponse, WorkflowTraceStatus, WorkflowTraceSummary,
};

struct ContractHost;

#[async_trait]
impl WorkflowHost for ContractHost {
    async fn validate_workflow(&self, _workflow_id: &str) -> Result<(), WorkflowServiceError> {
        Ok(())
    }

    async fn workflow_capabilities(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        Ok(WorkflowHostCapabilities {
            max_input_bindings: 32,
            max_output_targets: 8,
            max_value_bytes: 4096,
            runtime_requirements: WorkflowRuntimeRequirements {
                estimated_peak_vram_mb: Some(1536),
                estimated_peak_ram_mb: Some(3072),
                estimated_min_vram_mb: Some(1024),
                estimated_min_ram_mb: Some(2048),
                estimation_confidence: "estimated".to_string(),
                required_models: vec!["model-a".to_string()],
                required_backends: vec!["llama_cpp".to_string()],
                required_extensions: vec!["inference_gateway".to_string()],
            },
            models: vec![WorkflowCapabilityModel {
                model_id: "model-a".to_string(),
                model_revision_or_hash: Some("sha256:model-a-hash".to_string()),
                model_type: Some("embedding".to_string()),
                node_ids: vec!["node-embed".to_string()],
                roles: vec!["embedding".to_string(), "inference".to_string()],
            }],
            runtime_capabilities: vec![WorkflowRuntimeCapability {
                runtime_id: "llama_cpp".to_string(),
                display_name: "llama.cpp".to_string(),
                install_state: WorkflowRuntimeInstallState::Installed,
                available: true,
                configured: true,
                can_install: false,
                can_remove: true,
                source_kind: WorkflowRuntimeSourceKind::Managed,
                selected: true,
                readiness_state: Some(
                    pantograph_workflow_service::WorkflowRuntimeReadinessState::Ready,
                ),
                selected_version: None,
                supports_external_connection: true,
                backend_keys: vec!["llamacpp".to_string(), "llama.cpp".to_string()],
                missing_files: Vec::new(),
                unavailable_reason: None,
            }],
        })
    }

    async fn workflow_graph_fingerprint(
        &self,
        _workflow_id: &str,
    ) -> Result<String, WorkflowServiceError> {
        Ok("contract-graph".to_string())
    }

    async fn run_workflow(
        &self,
        _workflow_id: &str,
        _inputs: &[WorkflowPortBinding],
        output_targets: Option<&[WorkflowOutputTarget]>,
        _run_options: pantograph_workflow_service::WorkflowRunOptions,
        _run_handle: pantograph_workflow_service::WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        if let Some(targets) = output_targets {
            return Ok(targets
                .iter()
                .map(|target| WorkflowPortBinding {
                    node_id: target.node_id.clone(),
                    port_id: target.port_id.clone(),
                    value: serde_json::json!([0.1, 0.2, 0.3]),
                })
                .collect());
        }

        Ok(vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.1, 0.2, 0.3]),
        }])
    }

    async fn workflow_io(
        &self,
        _workflow_id: &str,
    ) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        Ok(WorkflowIoResponse {
            inputs: vec![WorkflowIoNode {
                node_id: "text-input-1".to_string(),
                node_type: "text-input".to_string(),
                name: Some("Prompt".to_string()),
                description: Some("Prompt text input".to_string()),
                ports: vec![WorkflowIoPort {
                    port_id: "text".to_string(),
                    name: Some("Text".to_string()),
                    description: None,
                    data_type: Some("string".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
            outputs: vec![WorkflowIoNode {
                node_id: "vector-output-1".to_string(),
                node_type: "vector-output".to_string(),
                name: Some("Embedding Vector".to_string()),
                description: Some("Vector result".to_string()),
                ports: vec![WorkflowIoPort {
                    port_id: "vector".to_string(),
                    name: Some("Vector".to_string()),
                    description: None,
                    data_type: Some("embedding".to_string()),
                    required: Some(false),
                    multiple: Some(false),
                }],
            }],
        })
    }
}

#[tokio::test]
async fn workflow_run_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: Some("run-123".to_string()),
            },
        )
        .await
        .expect("workflow_run response");

    let value = serde_json::to_value(response).expect("serialize response");
    let expected = serde_json::json!({
        "run_id": "run-123",
        "outputs": [
            {
                "node_id": "vector-output-1",
                "port_id": "vector",
                "value": [0.1, 0.2, 0.3]
            }
        ],
        "timing_ms": value["timing_ms"]
    });

    assert_eq!(value, expected);
}

#[tokio::test]
async fn workflow_capabilities_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_get_capabilities(
            &host,
            WorkflowCapabilitiesRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("capabilities response");

    let value = serde_json::to_value(response).expect("serialize capabilities");
    let expected = serde_json::json!({
        "max_input_bindings": 32,
        "max_output_targets": 8,
        "max_value_bytes": 4096,
        "runtime_requirements": {
            "estimated_peak_vram_mb": 1536,
            "estimated_peak_ram_mb": 3072,
            "estimated_min_vram_mb": 1024,
            "estimated_min_ram_mb": 2048,
            "estimation_confidence": "estimated",
            "required_models": ["model-a"],
            "required_backends": ["llama_cpp"],
            "required_extensions": ["inference_gateway"]
        },
        "models": [{
            "model_id": "model-a",
            "model_revision_or_hash": "sha256:model-a-hash",
            "model_type": "embedding",
            "node_ids": ["node-embed"],
            "roles": ["embedding", "inference"]
        }],
        "runtime_capabilities": [{
            "runtime_id": "llama_cpp",
            "display_name": "llama.cpp",
            "install_state": "installed",
            "available": true,
            "configured": true,
            "can_install": false,
            "can_remove": true,
            "source_kind": "managed",
            "selected": true,
            "readiness_state": "ready",
            "supports_external_connection": true,
            "backend_keys": ["llamacpp", "llama.cpp"],
            "missing_files": [],
            "unavailable_reason": null
        }]
    });

    assert_eq!(value, expected);
}

#[tokio::test]
async fn workflow_io_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_get_io(
            &host,
            WorkflowIoRequest {
                workflow_id: "wf-1".to_string(),
            },
        )
        .await
        .expect("workflow io response");

    let value = serde_json::to_value(response).expect("serialize workflow io");
    let expected = serde_json::json!({
        "inputs": [{
            "node_id": "text-input-1",
            "node_type": "text-input",
            "name": "Prompt",
            "description": "Prompt text input",
            "ports": [{
                "port_id": "text",
                "name": "Text",
                "description": null,
                "data_type": "string",
                "required": false,
                "multiple": false
            }]
        }],
        "outputs": [{
            "node_id": "vector-output-1",
            "node_type": "vector-output",
            "name": "Embedding Vector",
            "description": "Vector result",
            "ports": [{
                "port_id": "vector",
                "name": "Vector",
                "description": null,
                "data_type": "embedding",
                "required": false,
                "multiple": false
            }]
        }]
    });

    assert_eq!(value, expected);
}

#[test]
fn workflow_trace_contract_snapshot() {
    let response = WorkflowTraceSnapshotResponse {
        traces: vec![WorkflowTraceSummary {
            execution_id: "exec-1".to_string(),
            session_id: Some("session-1".to_string()),
            workflow_id: Some("wf-1".to_string()),
            workflow_name: Some("Workflow 1".to_string()),
            graph_fingerprint: Some("graph-1".to_string()),
            status: WorkflowTraceStatus::Completed,
            started_at_ms: 100,
            ended_at_ms: Some(180),
            duration_ms: Some(80),
            queue: WorkflowTraceQueueMetrics {
                enqueued_at_ms: Some(90),
                dequeued_at_ms: Some(100),
                queue_wait_ms: Some(10),
                scheduler_admission_outcome: Some("admitted".to_string()),
                scheduler_decision_reason: Some("dequeued_fifo".to_string()),
                scheduler_snapshot_diagnostics: Some(
                    pantograph_workflow_service::WorkflowSchedulerSnapshotDiagnostics {
                        loaded_session_count: 1,
                        max_loaded_sessions: 2,
                        reclaimable_loaded_session_count: 1,
                        runtime_capacity_pressure: pantograph_workflow_service::WorkflowSchedulerRuntimeCapacityPressure::RebalanceRequired,
                        active_run_blocks_admission: false,
                        next_admission_queue_id: Some("queue-next".to_string()),
                        next_admission_bypassed_queue_id: None,
                        next_admission_after_runs: Some(0),
                        next_admission_wait_ms: Some(0),
                        next_admission_not_before_ms: Some(100),
                        next_admission_reason: Some(
                            pantograph_workflow_service::WorkflowSchedulerDecisionReason::WarmSessionReused,
                        ),
                        runtime_registry: None,
                    },
                ),
            },
            runtime: WorkflowTraceRuntimeMetrics {
                runtime_id: Some("llama_cpp".to_string()),
                observed_runtime_ids: vec!["llama_cpp".to_string()],
                runtime_instance_id: Some("runtime-1".to_string()),
                model_target: Some("llava:13b".to_string()),
                warmup_started_at_ms: Some(95),
                warmup_completed_at_ms: Some(99),
                warmup_duration_ms: Some(4),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("already_ready".to_string()),
            },
            node_count_at_start: 2,
            event_count: 4,
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
                started_at_ms: Some(101),
                ended_at_ms: Some(170),
                duration_ms: Some(69),
                event_count: 2,
                stream_event_count: 1,
                last_error: None,
                last_progress_detail: None,
            }],
        }],
        retained_trace_limit: 200,
    };

    let value = serde_json::to_value(response).expect("serialize trace response");
    let expected = serde_json::json!({
        "traces": [{
            "execution_id": "exec-1",
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "workflow_name": "Workflow 1",
            "graph_fingerprint": "graph-1",
            "status": "completed",
            "started_at_ms": 100,
            "ended_at_ms": 180,
            "duration_ms": 80,
            "queue": {
                "enqueued_at_ms": 90,
                "dequeued_at_ms": 100,
                "queue_wait_ms": 10,
                "scheduler_admission_outcome": "admitted",
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
                },
                "scheduler_decision_reason": "dequeued_fifo"
            },
            "runtime": {
                "runtime_id": "llama_cpp",
                "observed_runtime_ids": ["llama_cpp"],
                "runtime_instance_id": "runtime-1",
                "model_target": "llava:13b",
                "warmup_started_at_ms": 95,
                "warmup_completed_at_ms": 99,
                "warmup_duration_ms": 4,
                "runtime_reused": true,
                "lifecycle_decision_reason": "already_ready"
            },
            "node_count_at_start": 2,
            "event_count": 4,
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
                "started_at_ms": 101,
                "ended_at_ms": 170,
                "duration_ms": 69,
                "event_count": 2,
                "stream_event_count": 1,
                "last_error": null
            }]
        }],
        "retained_trace_limit": 200
    });

    assert_eq!(value, expected);
}

#[test]
fn workflow_scheduler_snapshot_response_contract_snapshot() {
    let response = WorkflowSchedulerSnapshotResponse {
        workflow_id: Some("wf-1".to_string()),
        session_id: "session-1".to_string(),
        trace_execution_id: Some("run-1".to_string()),
        session: WorkflowSessionSummary {
            session_id: "session-1".to_string(),
            workflow_id: "wf-1".to_string(),
            session_kind: WorkflowSessionKind::Workflow,
            usage_profile: Some("interactive".to_string()),
            keep_alive: true,
            state: WorkflowSessionState::Running,
            queued_runs: 1,
            run_count: 2,
        },
        items: vec![WorkflowSessionQueueItem {
            queue_id: "queue-1".to_string(),
            run_id: Some("run-1".to_string()),
            enqueued_at_ms: Some(100),
            dequeued_at_ms: Some(110),
            priority: 5,
            queue_position: None,
            scheduler_admission_outcome: None,
            scheduler_decision_reason: None,
            status: WorkflowSessionQueueItemStatus::Running,
        }],
        diagnostics: None,
    };

    let value = serde_json::to_value(response).expect("serialize scheduler snapshot");
    let expected = serde_json::json!({
        "workflow_id": "wf-1",
        "session_id": "session-1",
        "trace_execution_id": "run-1",
        "session": {
            "session_id": "session-1",
            "workflow_id": "wf-1",
            "session_kind": "workflow",
            "usage_profile": "interactive",
            "keep_alive": true,
            "state": "running",
            "queued_runs": 1,
            "run_count": 2
        },
        "items": [{
            "queue_id": "queue-1",
            "run_id": "run-1",
            "enqueued_at_ms": 100,
            "dequeued_at_ms": 110,
            "priority": 5,
            "status": "running"
        }]
    });

    assert_eq!(value, expected);
}

#[test]
fn workflow_trace_snapshot_request_contract_snapshot() {
    let request = WorkflowTraceSnapshotRequest {
        execution_id: Some("exec-1".to_string()),
        session_id: Some("session-1".to_string()),
        workflow_id: Some("wf-1".to_string()),
        workflow_name: Some("Workflow 1".to_string()),
        include_completed: Some(true),
    };
    request
        .validate()
        .expect("trace snapshot request contract should remain valid");

    let value = serde_json::to_value(request).expect("serialize trace request");
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
fn workflow_trace_snapshot_request_rejects_blank_contract_filter() {
    let request = WorkflowTraceSnapshotRequest {
        execution_id: Some(String::new()),
        session_id: None,
        workflow_id: None,
        workflow_name: None,
        include_completed: None,
    };

    let error = request
        .validate()
        .expect_err("blank contract filter should be rejected");
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

#[tokio::test]
async fn workflow_run_rejects_non_discovered_output_target_contract() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let err = service
        .workflow_run(
            &host,
            WorkflowRunRequest {
                workflow_id: "wf-1".to_string(),
                inputs: Vec::new(),
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "stream".to_string(),
                }]),
                override_selection: None,
                timeout_ms: None,
                run_id: None,
            },
        )
        .await
        .expect_err("expected invalid request for non-discovered target");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[tokio::test]
async fn workflow_preflight_contract_snapshot() {
    let service = WorkflowService::new();
    let host = ContractHost;

    let response = service
        .workflow_preflight(
            &host,
            WorkflowPreflightRequest {
                workflow_id: "wf-1".to_string(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "text-input-1".to_string(),
                    port_id: "text".to_string(),
                    value: serde_json::json!("hello world"),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "vector-output-1".to_string(),
                    port_id: "vector".to_string(),
                }]),
                override_selection: None,
            },
        )
        .await
        .expect("preflight response");

    let value = serde_json::to_value(response).expect("serialize preflight");
    let expected = serde_json::json!({
        "missing_required_inputs": [],
        "invalid_targets": [],
        "warnings": [],
        "graph_fingerprint": "contract-graph",
        "runtime_warnings": [],
        "blocking_runtime_issues": [],
        "can_run": true
    });
    assert_eq!(value, expected);
}
