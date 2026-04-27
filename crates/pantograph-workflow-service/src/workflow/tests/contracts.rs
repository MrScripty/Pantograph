use super::*;
use crate::{
    GraphEdge, GraphNode, Position, WorkflowExecutableTopology, WorkflowExecutableTopologyEdge,
    WorkflowExecutableTopologyNode, WorkflowGraph, WorkflowGraphRunSettings,
    WorkflowGraphRunSettingsNode, WorkflowPresentationEdge, WorkflowPresentationMetadata,
    WorkflowPresentationNode,
};

#[test]
fn request_roundtrip_uses_snake_case() {
    let req = WorkflowRunRequest {
        workflow_id: "wf-1".to_string(),
        workflow_semantic_version: "0.1.0".to_string(),
        inputs: vec![WorkflowPortBinding {
            node_id: "input-1".to_string(),
            port_id: "text".to_string(),
            value: serde_json::json!("hello"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "text-output-1".to_string(),
            port_id: "text".to_string(),
        }]),
        override_selection: Some(WorkflowTechnicalFitOverride {
            model_id: Some("model-a".to_string()),
            backend_key: Some("llama.cpp".to_string()),
        }),
        timeout_ms: None,
    };

    let json = serde_json::to_value(&req).expect("serialize request");
    assert_eq!(json["workflow_id"], "wf-1");
    assert_eq!(json["inputs"][0]["node_id"], "input-1");
    assert_eq!(json["output_targets"][0]["port_id"], "text");
    assert_eq!(json["override_selection"]["model_id"], "model-a");
    assert_eq!(json["override_selection"]["backend_key"], "llama.cpp");
}

#[test]
fn request_rejects_caller_authored_run_id() {
    let payload = serde_json::json!({
        "workflow_id": "wf-1",
        "inputs": [],
        "output_targets": null,
        "override_selection": null,
        "timeout_ms": null,
        "run_id": "caller-run-1"
    });

    let err = serde_json::from_value::<WorkflowRunRequest>(payload)
        .expect_err("old run_id field must be rejected");
    assert!(err.to_string().contains("unknown field `run_id`"));
}

#[test]
fn response_roundtrip_preserves_outputs() {
    let res = WorkflowRunResponse {
        workflow_run_id: "run-1".to_string(),
        outputs: vec![WorkflowPortBinding {
            node_id: "vector-output-1".to_string(),
            port_id: "vector".to_string(),
            value: serde_json::json!([0.1, 0.2, 0.3]),
        }],
        timing_ms: 5,
    };

    let json = serde_json::to_string(&res).expect("serialize response");
    let parsed: WorkflowRunResponse = serde_json::from_str(&json).expect("parse response");
    assert_eq!(parsed.workflow_run_id, "run-1");
    assert_eq!(parsed.outputs[0].node_id, "vector-output-1");
}

#[test]
fn workflow_io_roundtrip_uses_snake_case() {
    let response = WorkflowIoResponse {
        inputs: vec![WorkflowIoNode {
            node_id: "text-input-1".to_string(),
            node_type: "text-input".to_string(),
            name: Some("Prompt".to_string()),
            description: Some("Prompt input".to_string()),
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
            node_id: "text-output-1".to_string(),
            node_type: "text-output".to_string(),
            name: Some("Answer".to_string()),
            description: None,
            ports: vec![WorkflowIoPort {
                port_id: "text".to_string(),
                name: Some("Text".to_string()),
                description: None,
                data_type: Some("string".to_string()),
                required: Some(false),
                multiple: Some(false),
            }],
        }],
    };

    let json = serde_json::to_value(&response).expect("serialize workflow io");
    assert_eq!(json["inputs"][0]["node_id"], "text-input-1");
    assert_eq!(json["outputs"][0]["ports"][0]["port_id"], "text");

    let parsed: WorkflowIoResponse =
        serde_json::from_value(json).expect("parse workflow io response");
    assert_eq!(parsed.inputs[0].name.as_deref(), Some("Prompt"));
    assert_eq!(
        parsed.outputs[0].ports[0].data_type.as_deref(),
        Some("string")
    );
}

#[test]
fn workflow_run_graph_query_roundtrip_uses_snake_case() {
    let response = WorkflowRunGraphQueryResponse {
        run_graph: Some(WorkflowRunGraphProjection {
            workflow_run_id: "run-1".to_string(),
            workflow_id: "workflow-a".to_string(),
            workflow_version_id: "wfver-1".to_string(),
            workflow_presentation_revision_id: "wfpres-1".to_string(),
            workflow_semantic_version: "1.2.3".to_string(),
            workflow_execution_fingerprint: "workflow-exec-blake3:abc".to_string(),
            snapshot_created_at_ms: 10,
            workflow_version_created_at_ms: 11,
            presentation_revision_created_at_ms: 12,
            graph: WorkflowGraph {
                nodes: vec![GraphNode {
                    id: "node-1".to_string(),
                    node_type: "text-input".to_string(),
                    position: Position { x: 1.0, y: 2.0 },
                    data: serde_json::json!({"value": "hello"}),
                }],
                edges: vec![GraphEdge {
                    id: "edge-1".to_string(),
                    source: "node-1".to_string(),
                    source_handle: "text".to_string(),
                    target: "node-2".to_string(),
                    target_handle: "text".to_string(),
                }],
                derived_graph: None,
            },
            executable_topology: WorkflowExecutableTopology {
                schema_version: 1,
                nodes: vec![WorkflowExecutableTopologyNode {
                    node_id: "node-1".to_string(),
                    node_type: "text-input".to_string(),
                    contract_version: "0.1.0".to_string(),
                    behavior_digest: "digest".to_string(),
                }],
                edges: vec![WorkflowExecutableTopologyEdge {
                    source_node_id: "node-1".to_string(),
                    source_port_id: "text".to_string(),
                    target_node_id: "node-2".to_string(),
                    target_port_id: "text".to_string(),
                }],
            },
            presentation_metadata: WorkflowPresentationMetadata {
                schema_version: 1,
                nodes: vec![WorkflowPresentationNode {
                    node_id: "node-1".to_string(),
                    position: Position { x: 1.0, y: 2.0 },
                }],
                edges: vec![WorkflowPresentationEdge {
                    edge_id: "edge-1".to_string(),
                    source_node_id: "node-1".to_string(),
                    source_port_id: "text".to_string(),
                    target_node_id: "node-2".to_string(),
                    target_port_id: "text".to_string(),
                }],
            },
            graph_settings: WorkflowGraphRunSettings {
                schema_version: 1,
                nodes: vec![WorkflowGraphRunSettingsNode {
                    node_id: "node-1".to_string(),
                    node_type: "text-input".to_string(),
                    data: serde_json::json!({"value": "hello"}),
                }],
            },
        }),
    };

    let json = serde_json::to_value(&response).expect("serialize run graph response");
    let run_graph = &json["run_graph"];
    assert_eq!(run_graph["workflow_run_id"], "run-1");
    assert_eq!(run_graph["workflow_version_id"], "wfver-1");
    assert_eq!(run_graph["graph"]["nodes"][0]["node_type"], "text-input");
    assert_eq!(
        run_graph["executable_topology"]["nodes"][0]["contract_version"],
        "0.1.0"
    );

    let parsed: WorkflowRunGraphQueryResponse =
        serde_json::from_value(json).expect("parse run graph response");
    assert_eq!(
        parsed
            .run_graph
            .expect("run graph")
            .workflow_semantic_version,
        "1.2.3"
    );
}

#[test]
fn workflow_local_network_status_roundtrip_uses_snake_case() {
    let response = WorkflowLocalNetworkStatusQueryResponse {
        local_node: WorkflowLocalNetworkNodeStatus {
            node_id: "local".to_string(),
            display_name: "Local Pantograph".to_string(),
            captured_at_ms: 42,
            transport_state: WorkflowNetworkTransportState::LocalOnly,
            system: WorkflowLocalSystemMetrics {
                hostname: Some("host-a".to_string()),
                os_name: Some("Linux".to_string()),
                os_version: None,
                kernel_version: None,
                cpu: WorkflowLocalCpuMetrics {
                    logical_core_count: 8,
                    average_usage_percent: Some(12.5),
                },
                memory: WorkflowLocalMemoryMetrics {
                    total_bytes: 100,
                    used_bytes: 40,
                    available_bytes: 60,
                },
                disks: vec![WorkflowLocalDiskMetrics {
                    name: "disk-a".to_string(),
                    mount_point: "/".to_string(),
                    total_bytes: 1000,
                    available_bytes: 500,
                }],
                network_interfaces: vec![WorkflowLocalNetworkInterfaceMetrics {
                    name: "eth0".to_string(),
                    total_received_bytes: 10,
                    total_transmitted_bytes: 20,
                }],
                gpu: WorkflowLocalGpuMetrics {
                    available: false,
                    reason: Some("not implemented".to_string()),
                },
            },
            scheduler_load: WorkflowLocalSchedulerLoad {
                max_sessions: 4,
                active_session_count: 1,
                max_loaded_sessions: 2,
                loaded_session_count: 1,
                active_run_count: 0,
                queued_run_count: 3,
            },
            degradation_warnings: vec!["not implemented".to_string()],
        },
        peer_nodes: vec![WorkflowPeerNetworkNodeStatus {
            node_id: "peer-a".to_string(),
            display_name: "Peer A".to_string(),
            transport_state: WorkflowNetworkTransportState::PairingRequired,
            last_seen_at_ms: None,
        }],
    };

    let json = serde_json::to_value(&response).expect("serialize network response");
    assert_eq!(json["local_node"]["transport_state"], "local_only");
    assert_eq!(json["local_node"]["scheduler_load"]["queued_run_count"], 3);
    assert_eq!(
        json["local_node"]["system"]["network_interfaces"][0]["total_received_bytes"],
        10
    );
    assert_eq!(json["peer_nodes"][0]["transport_state"], "pairing_required");

    let parsed: WorkflowLocalNetworkStatusQueryResponse =
        serde_json::from_value(json).expect("parse network response");
    assert_eq!(parsed.local_node.node_id, "local");
    assert_eq!(parsed.peer_nodes[0].node_id, "peer-a");
}

#[test]
fn workflow_retention_policy_update_request_uses_snake_case() {
    let request = WorkflowRetentionPolicyUpdateRequest {
        retention_days: 120,
        explanation: "Keep diagnostics for development audit".to_string(),
        reason: "GUI settings update".to_string(),
    };

    let json = serde_json::to_value(&request).expect("serialize retention update request");
    assert_eq!(json["retention_days"], 120);
    assert_eq!(
        json["explanation"],
        "Keep diagnostics for development audit"
    );
    assert_eq!(json["reason"], "GUI settings update");

    let parsed: WorkflowRetentionPolicyUpdateRequest =
        serde_json::from_value(json).expect("parse retention update request");
    assert_eq!(parsed.retention_days, 120);
}

#[test]
fn workflow_retention_cleanup_request_uses_snake_case() {
    let request = WorkflowRetentionCleanupRequest {
        limit: Some(250),
        reason: "GUI cleanup request".to_string(),
    };

    let json = serde_json::to_value(&request).expect("serialize retention cleanup request");
    assert_eq!(json["limit"], 250);
    assert_eq!(json["reason"], "GUI cleanup request");

    let parsed: WorkflowRetentionCleanupRequest =
        serde_json::from_value(json).expect("parse retention cleanup request");
    assert_eq!(parsed.limit, Some(250));
    assert_eq!(parsed.reason, "GUI cleanup request");
}

#[test]
fn workflow_service_error_envelope_roundtrip() {
    let err = WorkflowServiceError::OutputNotProduced(
        "requested output target 'vector-output-1.vector' was not produced".to_string(),
    );

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::OutputNotProduced);
    assert!(envelope.message.contains("vector-output-1.vector"));
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::OutputNotProduced);
    assert!(parsed.message.contains("vector-output-1.vector"));
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_cancelled_envelope_roundtrip() {
    let err = WorkflowServiceError::Cancelled("workflow run cancelled".to_string());

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::Cancelled);
    assert_eq!(envelope.message, "workflow run cancelled");
    assert_eq!(envelope.details, None);

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(parsed.code, WorkflowErrorCode::Cancelled);
    assert_eq!(parsed.message, "workflow run cancelled");
    assert_eq!(parsed.details, None);
}

#[test]
fn workflow_service_scheduler_busy_envelope_includes_structured_details() {
    let err = WorkflowServiceError::scheduler_runtime_capacity_exhausted(2, 2, 0);

    let envelope = err.to_envelope();
    assert_eq!(envelope.code, WorkflowErrorCode::SchedulerBusy);
    assert_eq!(
        envelope.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );

    let json = err.to_envelope_json();
    let parsed: WorkflowErrorEnvelope =
        serde_json::from_str(&json).expect("parse workflow error envelope");
    assert_eq!(
        parsed.details,
        Some(WorkflowErrorDetails::Scheduler(
            WorkflowSchedulerErrorDetails::runtime_capacity_exhausted(2, 2, 0),
        ))
    );
}
