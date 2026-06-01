use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyRequirementsId, PumasModelRef,
};
use pantograph_inference_interface_contracts::{
    AuthoredInferenceInterfaceSnapshot, DraftGraphValidationSessionId, DraftGraphValidationStatus,
    DraftGraphValidationSummary, InferenceAvailability, InferenceAvailabilityStatus,
    InferenceInterfaceDescriptor, InferenceInterfaceFingerprint, InferenceTaskKind,
    WorkflowGraphRevision, WorkflowNodeId, INFERENCE_INTERFACE_CONTRACT_VERSION,
};

use crate::graph::{
    InferenceInterfaceNodeProjectionRecord, WorkflowGraphInferenceValidationPublication,
    WorkflowGraphInferenceValidationSession,
};
use crate::{
    GraphEdge, GraphNode, Position, WorkflowGraph, WorkflowGraphCurrentValidationRefreshRequest,
};

use super::*;

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "input".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({"value": "first"}),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({"name": "Output"}),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge".to_string(),
            source: "input".to_string(),
            source_handle: "text".to_string(),
            target: "output".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    }
}

fn unresolved_inference_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![GraphNode {
            id: "inference".to_string(),
            node_type: "llm-inference".to_string(),
            position: Position { x: 0.0, y: 0.0 },
            data: serde_json::json!({
                "task_kind": "text_generation",
                "runtime": "cuda",
                "pumas_model_ref": {
                    "source": "puma-lib",
                    "status": "resolved",
                    "model_id": "family/model"
                }
            }),
        }],
        edges: Vec::new(),
        derived_graph: None,
    }
}

#[test]
fn resolve_workflow_graph_version_reuses_same_executable_fingerprint() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let first = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("first version");
    let second = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("reused version");

    assert_eq!(first.workflow_version_id, second.workflow_version_id);
    assert_eq!(first.semantic_version, "1.0.0");
    assert!(first
        .execution_fingerprint
        .starts_with("workflow-exec-blake3:"));
}

#[test]
fn resolve_workflow_graph_version_rejects_semantic_version_conflict() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("first version");

    let mut changed_graph = graph();
    changed_graph.edges[0].target_handle = "other-port".to_string();
    let err = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &changed_graph)
        .expect_err("semantic version conflict");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("semantic version"))
    );
}

#[test]
fn resolve_workflow_graph_presentation_revision_tracks_display_metadata_separately() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let version = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("version");
    let first = service
        .resolve_workflow_graph_presentation_revision(
            "workflow-versioned",
            version.workflow_version_id.as_str(),
            &graph(),
        )
        .expect("first presentation revision");

    let mut display_changed = graph();
    display_changed.nodes[0].position = Position { x: 50.0, y: 0.0 };
    let second = service
        .resolve_workflow_graph_presentation_revision(
            "workflow-versioned",
            version.workflow_version_id.as_str(),
            &display_changed,
        )
        .expect("second presentation revision");

    assert_ne!(
        first.workflow_presentation_revision_id,
        second.workflow_presentation_revision_id
    );
    assert_eq!(first.workflow_version_id, version.workflow_version_id);
    assert_eq!(second.workflow_version_id, version.workflow_version_id);
    assert!(first
        .presentation_fingerprint
        .starts_with("workflow-presentation-blake3:"));
}

#[test]
fn resolve_workflow_graph_presentation_revision_ignores_node_data_changes() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let version = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("version");
    let first = service
        .resolve_workflow_graph_presentation_revision(
            "workflow-versioned",
            version.workflow_version_id.as_str(),
            &graph(),
        )
        .expect("first presentation revision");

    let mut data_changed = graph();
    data_changed.nodes[0].data = serde_json::json!({"value": "changed"});
    let second = service
        .resolve_workflow_graph_presentation_revision(
            "workflow-versioned",
            version.workflow_version_id.as_str(),
            &data_changed,
        )
        .expect("reused presentation revision");

    assert_eq!(
        first.workflow_presentation_revision_id,
        second.workflow_presentation_revision_id
    );
}

#[test]
fn workflow_run_graph_query_returns_none_for_unknown_run() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");

    let response = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: "run_missing".to_string(),
        })
        .expect("query graph");

    assert_eq!(response.run_graph, None);
}

#[test]
fn workflow_executable_validation_snapshot_round_trips_through_attribution() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let version = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("version");
    let snapshot = executable_validation_snapshot(&version);

    let stored = service
        .store_workflow_executable_validation_snapshot(snapshot.clone())
        .expect("snapshot stored");
    let loaded = service
        .workflow_executable_validation_snapshot(
            WorkflowExecutableValidationSnapshotLookupRequest {
                workflow_version_id: version.workflow_version_id.clone(),
                workflow_execution_fingerprint: version.execution_fingerprint.clone(),
                descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            },
        )
        .expect("snapshot loaded");

    assert_eq!(stored.as_record(), &snapshot);
    assert_eq!(loaded.as_record(), &snapshot);
    assert!(loaded.scheduler_inference_task_projections().is_ok());
}

#[test]
fn workflow_executable_validation_snapshot_lookup_fails_closed_when_missing() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let version = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("version");

    let err = service
        .workflow_executable_validation_snapshot(
            WorkflowExecutableValidationSnapshotLookupRequest {
                workflow_version_id: version.workflow_version_id,
                workflow_execution_fingerprint: version.execution_fingerprint,
                descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            },
        )
        .expect_err("missing snapshot must fail closed");

    assert!(matches!(err, WorkflowServiceError::InvalidRequest(_)));
}

#[test]
fn workflow_executable_validation_snapshot_lookup_rejects_stale_fingerprint() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let version = service
        .resolve_workflow_graph_version("workflow-versioned", "1.0.0", &graph())
        .expect("version");
    service
        .store_workflow_executable_validation_snapshot(executable_validation_snapshot(&version))
        .expect("snapshot stored");

    let err = service
        .workflow_executable_validation_snapshot(
            WorkflowExecutableValidationSnapshotLookupRequest {
                workflow_version_id: version.workflow_version_id,
                workflow_execution_fingerprint: "workflow-exec-blake3:stale".to_string(),
                descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            },
        )
        .expect_err("stale fingerprint must fail closed");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("workflow fingerprint"))
    );
}

#[test]
fn publish_workflow_executable_validation_snapshot_rejects_runtime_publication() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let graph = graph();
    let validation_publication = executable_validation_publication(&graph);
    let err = service
        .publish_workflow_executable_validation_snapshot(
            WorkflowExecutableValidationSnapshotPublishRequest {
                workflow_id: "workflow-versioned".to_string(),
                workflow_semantic_version: "1.0.0".to_string(),
                graph: graph.clone(),
                validation_publication,
                validation_snapshot_id: Some(
                    WorkflowExecutableValidationSnapshotId::parse(
                        "wfvalsnap_00000000-0000-4000-8000-000000000011",
                    )
                    .expect("valid snapshot id"),
                ),
            },
        )
        .expect_err("caller-supplied runtime publication must fail closed");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("graph-session validation state"))
    );
}

#[test]
fn publish_workflow_executable_validation_snapshot_rejects_stale_runtime_publication_before_revision_check(
) {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let graph = graph();
    let mut validation_publication = executable_validation_publication(&graph);
    validation_publication.validation_session.graph_revision =
        WorkflowGraphRevision::parse("stale_revision").expect("valid stale revision");

    let err = service
        .publish_workflow_executable_validation_snapshot(
            WorkflowExecutableValidationSnapshotPublishRequest {
                workflow_id: "workflow-versioned".to_string(),
                workflow_semantic_version: "1.0.0".to_string(),
                graph,
                validation_publication,
                validation_snapshot_id: None,
            },
        )
        .expect_err("caller-supplied runtime publication should fail closed");

    assert!(
        matches!(err, WorkflowServiceError::InvalidRequest(message) if message.contains("graph-session validation state"))
    );
}

#[tokio::test]
async fn publish_graph_session_executable_validation_snapshot_rejects_non_executable_summary() {
    let service = WorkflowService::with_ephemeral_attribution_store().expect("service");
    let session = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: unresolved_inference_graph(),
            workflow_id: Some("workflow-versioned".to_string()),
        })
        .await
        .expect("create graph edit session");
    let graph_revision =
        WorkflowGraphRevision::parse(session.graph_revision).expect("valid graph revision");
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: session.session_id.clone(),
                graph_revision,
            },
        )
        .await
        .expect("refresh validation summary");

    assert!(
        !validation.summary.submit_gate.allowed,
        "unresolved inference graph must not be submittable"
    );

    let err = service
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: "workflow-versioned".to_string(),
                workflow_semantic_version: "1.0.0".to_string(),
                graph_session_id: session.session_id,
                validation_session_id: validation.summary.validation_session_id,
                validation_snapshot_id: Some(
                    WorkflowExecutableValidationSnapshotId::parse(
                        "wfvalsnap_00000000-0000-4000-8000-000000000012",
                    )
                    .expect("valid snapshot id"),
                ),
            },
        )
        .await
        .expect_err("non-executable validation summary must not publish");

    assert!(
        matches!(&err, WorkflowServiceError::InvalidRequest(message) if message.contains("not executable")),
        "unexpected error: {err}"
    );
}

fn executable_validation_snapshot(
    version: &pantograph_runtime_attribution::WorkflowVersionRecord,
) -> WorkflowExecutableValidationSnapshotRecord {
    WorkflowExecutableValidationSnapshotRecord {
        schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId::parse(
            "wfvalsnap_00000000-0000-4000-8000-000000000010",
        )
        .expect("valid snapshot id"),
        workflow_id: version.workflow_id.clone(),
        workflow_version_id: version.workflow_version_id.clone(),
        workflow_semantic_version: version.semantic_version.clone(),
        workflow_execution_fingerprint: version.execution_fingerprint.clone(),
        descriptor_contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: WorkflowGraphRevision::parse("revision_1").expect("valid graph revision"),
        validation_session_id: DraftGraphValidationSessionId::parse("validation_session_1")
            .expect("valid validation session id"),
        validation_summary: DraftGraphValidationSummary {
            status: DraftGraphValidationStatus::Executable,
            executable: true,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        },
        nodes: vec![WorkflowExecutableValidationSnapshotNode {
            node_id: WorkflowNodeId::parse("infer_node").expect("valid node id"),
            descriptor_fingerprint: InferenceInterfaceFingerprint::parse(
                "descriptor_fingerprint_1",
            )
            .expect("valid descriptor fingerprint"),
            task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
            model_ref: PumasModelRef {
                model_id: "pumas://model/stable-diffusion".to_string(),
                revision: Some("main".to_string()),
                selected_artifact_id: Some("artifact-diffusers".to_string()),
                selected_artifact_path: None,
                migration_diagnostics: Vec::new(),
            },
            constraints: Default::default(),
            availability_status: InferenceAvailabilityStatus::Available,
            validation_status: DraftGraphValidationStatus::Executable,
            trait_settings: Vec::new(),
            estimate_hints: Vec::new(),
            dependency_requirements_id: DependencyRequirementsId::parse(
                "requirements.image_generation.cuda0",
            )
            .expect("valid requirements id"),
            selected_binding_ids: vec![
                DependencyBindingId::parse("torch-diffusers").expect("valid binding id")
            ],
            dependency_override_fingerprint: DependencyOverrideFingerprint::parse("override.none")
                .expect("valid override fingerprint"),
            blocking_diagnostics: Vec::new(),
        }],
    }
}

fn executable_validation_publication(
    graph: &WorkflowGraph,
) -> WorkflowGraphInferenceValidationPublication {
    let graph_revision =
        WorkflowGraphRevision::parse(&graph.compute_fingerprint()).expect("valid graph revision");
    let validation_session_id = DraftGraphValidationSessionId::parse("validation_session_publish")
        .expect("valid validation session id");
    let summary = DraftGraphValidationSummary {
        status: DraftGraphValidationStatus::Executable,
        executable: true,
        enqueue_disabled_reasons: Vec::new(),
        diagnostics_count: 0,
        blocking_diagnostics_count: 0,
    };
    let descriptor = InferenceInterfaceDescriptor {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        model_ref: PumasModelRef {
            model_id: "pumas://model/stable-diffusion".to_string(),
            revision: Some("main".to_string()),
            selected_artifact_id: Some("artifact-diffusers".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
        task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
        descriptor_fingerprint: InferenceInterfaceFingerprint::parse("descriptor_fingerprint_1")
            .expect("valid descriptor fingerprint"),
        runtime_conditions: Vec::new(),
        inputs: Vec::new(),
        outputs: Vec::new(),
        availability: InferenceAvailability::available(),
        diagnostics: Vec::new(),
    };

    WorkflowGraphInferenceValidationPublication {
        validation_session: WorkflowGraphInferenceValidationSession {
            contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
            validation_session_id,
            graph_revision,
            latest_sequence: 0,
            summary: summary.clone(),
            events: Vec::new(),
        },
        node_projections: vec![InferenceInterfaceNodeProjectionRecord {
            node_id: WorkflowNodeId::parse("infer_node").expect("valid node id"),
            authored_snapshot: AuthoredInferenceInterfaceSnapshot {
                contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
                descriptor_fingerprint: descriptor.descriptor_fingerprint.clone(),
                task_kind: descriptor.task_kind.clone(),
                inputs: Vec::new(),
                outputs: Vec::new(),
            },
            descriptor,
            validation_summary: summary,
            drift_report: None,
            update_proposal: None,
            runtime_constraint: None,
            device_constraint: None,
            estimate_hints: Vec::new(),
        }],
        request_diagnostics: Vec::new(),
    }
}
