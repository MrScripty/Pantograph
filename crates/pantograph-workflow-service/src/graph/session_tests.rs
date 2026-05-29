use super::super::types::InsertNodePositionHint;
use super::*;
use crate::graph::types::{ConnectionAnchor, GraphNode, Position};
use crate::graph::{
    InferenceCapabilityFacts, InferenceInterfaceFactsProvider,
    InferenceInterfaceFactsProviderError, InferenceInterfaceGraphResolutionInput,
    InferenceInterfaceResolverFacts, InferenceModelResolutionFacts, InferenceModelResolutionState,
    InferenceRuntimeAvailabilityFact, InferenceRuntimeAvailabilityState,
    WorkflowGraphCurrentValidationRefreshRequest, WorkflowGraphCurrentValidationSummaryRequest,
    WorkflowGraphCurrentValidationSummaryState, WorkflowGraphDeleteSelectionRequest,
    WorkflowGraphEditSessionGraphRequest, WorkflowGraphInferenceValidationSession,
    WorkflowGraphRemoveEdgeRequest, WorkflowGraphRemoveEdgesRequest,
    WorkflowGraphValidationSubmitGateReason,
};
use crate::{
    workflow::WorkflowSchedulerInferenceTaskProjection, WorkflowExecutionSessionQueueItemStatus,
    WorkflowGraphAddNodeRequest, WorkflowGraphRemoveNodeRequest,
    WorkflowGraphUpdateNodeDataRequest, WorkflowGraphUpdateNodePositionRequest,
};
use async_trait::async_trait;
use pantograph_dependency_environment_service::{
    DependencyEnvironmentProvider, NotImplementedDependencyEnvironmentProvider,
};
use pantograph_dependency_planning::{
    DependencyEnvironmentResult, ValidatedDependencyEnvironmentRequest,
};
use pantograph_inference_interface_contracts::{
    DependencyEnvironmentAction, DependencyEnvironmentActionIntent,
    DependencyEnvironmentActionIntentStatus, DraftGraphValidationSessionId,
    DraftGraphValidationStatus, DraftGraphValidationSummary, InferenceAvailability,
    InferenceDiagnosticCode, InferencePortDescriptor, InferencePortDirection, InferencePortId,
    InferencePortOptions, InferencePortRequirement, InferenceScalarType, InferenceTaskKind,
    InferenceValueType, RuntimeIntentId, WorkflowGraphRevision,
    INFERENCE_INTERFACE_CONTRACT_VERSION,
};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Notify;

fn sample_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "text-input".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({
                    "label": "Text Input",
                    "text": "hello",
                    "definition": {
                        "node_type": "text-input"
                    }
                }),
            },
            GraphNode {
                id: "text-output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 120.0, y: 0.0 },
                data: serde_json::json!({
                    "label": "Text Output"
                }),
            },
        ],
        edges: vec![GraphEdge {
            id: "text-input-text-text-output-text".to_string(),
            source: "text-input".to_string(),
            source_handle: "text".to_string(),
            target: "text-output".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    }
}

fn disconnected_graph() -> WorkflowGraph {
    let mut graph = sample_graph();
    graph.edges.clear();
    graph
}

fn inference_to_output_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "llm".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "text_generation",
                    "backend_key": "llama_cpp",
                    "pumas_model_ref": {
                        "source": "puma-lib",
                        "status": "resolved",
                        "model_id": "family/model",
                        "model_path": "/models/model.gguf"
                    }
                }),
            },
            GraphNode {
                id: "output".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 120.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    }
}

fn branching_graph() -> WorkflowGraph {
    let mut graph = sample_graph();
    graph.nodes.push(GraphNode {
        id: "text-copy".to_string(),
        node_type: "text-output".to_string(),
        position: Position { x: 120.0, y: 80.0 },
        data: serde_json::json!({
            "label": "Text Copy"
        }),
    });
    graph.edges.push(GraphEdge {
        id: "text-input-text-text-copy-text".to_string(),
        source: "text-input".to_string(),
        source_handle: "text".to_string(),
        target: "text-copy".to_string(),
        target_handle: "text".to_string(),
    });
    graph
}

#[tokio::test]
async fn create_session_returns_backend_owned_edit_kind() {
    let store = GraphSessionStore::new();

    let session = store.create_session(sample_graph(), None).await;

    assert_eq!(session.session_kind, WorkflowExecutionSessionKind::Edit);
    assert!(!session.session_id.is_empty());
    assert!(!session.graph_revision.is_empty());
}

#[tokio::test]
async fn scheduler_snapshot_preserves_source_workflow_id_for_loaded_edit_session() {
    let store = GraphSessionStore::new();

    let session = store
        .create_session(sample_graph(), Some("saved-flow".to_string()))
        .await;

    let snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("scheduler snapshot");

    assert_eq!(snapshot.workflow_id, None);
    assert_eq!(snapshot.session.session_id, session.session_id);
    assert_eq!(snapshot.session.workflow_id, "saved-flow");
}

#[tokio::test]
async fn scheduler_snapshot_tracks_running_edit_session_queue_item() {
    let store = GraphSessionStore::new();

    let session = store.create_session(sample_graph(), None).await;
    store
        .mark_running(&session.session_id, "run-1")
        .await
        .expect("mark running");

    let running_snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("running scheduler snapshot");

    assert_eq!(running_snapshot.session.queued_runs, 1);
    assert_eq!(running_snapshot.items.len(), 1);
    assert_eq!(running_snapshot.items[0].workflow_run_id, "run-1");
    assert_eq!(running_snapshot.workflow_run_id.as_deref(), Some("run-1"));
    assert_eq!(
        running_snapshot.items[0].status,
        WorkflowExecutionSessionQueueItemStatus::Running
    );

    store
        .finish_run(&session.session_id)
        .await
        .expect("finish run");
    let finished_snapshot = store
        .get_scheduler_snapshot(&session.session_id)
        .await
        .expect("finished scheduler snapshot");

    assert_eq!(finished_snapshot.session.queued_runs, 0);
    assert_eq!(finished_snapshot.session.run_count, 1);
    assert!(finished_snapshot.items.is_empty());
}

#[tokio::test]
async fn dependency_environment_action_intent_fails_closed_without_validation_summary() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;

    let result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: None,
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        result.diagnostics[0].code,
        InferenceDiagnosticCode::ValidationSummaryMissing
    );
}

#[tokio::test]
async fn dependency_environment_action_intent_consumes_current_validation_summary() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Pending,
                false,
            ),
        )
        .await
        .expect("record current validation summary");

    let result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.1"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        result.diagnostics[0].code,
        InferenceDiagnosticCode::GraphValidationPending
    );
}

#[tokio::test]
async fn current_validation_summary_fails_closed_without_published_summary() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;

    let response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        response.state,
        WorkflowGraphCurrentValidationSummaryState::Missing
    );
    assert!(!response.submit_gate.allowed);
    assert_eq!(
        response.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::ValidationSummaryMissing)
    );
    assert_eq!(
        response.diagnostics[0].code,
        InferenceDiagnosticCode::ValidationSummaryMissing
    );
}

#[tokio::test]
async fn current_validation_summary_reports_stale_graph_revision() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Executable,
                true,
            ),
        )
        .await
        .expect("record current validation summary");

    let response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: "stale-revision".parse().expect("valid stale revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        response.state,
        WorkflowGraphCurrentValidationSummaryState::Stale
    );
    assert!(!response.submit_gate.allowed);
    assert_eq!(
        response.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::GraphRevisionStale)
    );
    assert_eq!(
        response.diagnostics[0].code,
        InferenceDiagnosticCode::GraphRevisionMismatch
    );
}

#[tokio::test]
async fn current_validation_summary_allows_executable_summary() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Executable,
                true,
            ),
        )
        .await
        .expect("record current validation summary");

    let response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        response.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );
    assert!(response.submit_gate.allowed);
    assert_eq!(
        response
            .validation_session_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("validation.session.1")
    );
}

#[tokio::test]
async fn current_validation_summary_marks_semantic_node_data_edit_stale() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Executable,
                true,
            ),
        )
        .await
        .expect("record current validation summary");

    let updated = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "infer".to_string(),
            data: serde_json::json!({
                "runtime": "cuda"
            }),
        })
        .await
        .expect("update semantic inference node data");

    assert_ne!(updated.graph_revision, session.graph_revision);

    let stale_response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid original graph revision"),
        })
        .await
        .expect("stale validation summary response");

    assert_eq!(
        stale_response.state,
        WorkflowGraphCurrentValidationSummaryState::Stale
    );
    assert!(!stale_response.submit_gate.allowed);
    assert_eq!(
        stale_response.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::GraphRevisionStale)
    );

    let current_response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id,
            graph_revision: updated
                .graph_revision
                .parse()
                .expect("valid updated graph revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        current_response.state,
        WorkflowGraphCurrentValidationSummaryState::Missing
    );
    assert!(!current_response.submit_gate.allowed);
    assert_eq!(
        current_response.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::ValidationSummaryMissing)
    );
}

#[tokio::test]
async fn current_validation_summary_survives_layout_only_position_edit() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Executable,
                true,
            ),
        )
        .await
        .expect("record current validation summary");

    let updated = store
        .update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: session.session_id.clone(),
            node_id: "infer".to_string(),
            position: Position { x: 320.0, y: 64.0 },
        })
        .await
        .expect("update inference node position");

    assert_eq!(updated.graph_revision, session.graph_revision);

    let response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id,
            graph_revision: updated
                .graph_revision
                .parse()
                .expect("valid updated graph revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        response.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );
    assert!(response.submit_gate.allowed);
}

#[tokio::test]
async fn close_session_clears_current_validation_state() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Executable,
                true,
            ),
        )
        .await
        .expect("record current validation summary");

    assert_eq!(
        store
            .validation_state_record_count_for_session(&session.session_id)
            .await
            .expect("validation state record count"),
        1
    );

    store
        .close_session(&session.session_id)
        .await
        .expect("close graph session");

    assert_eq!(
        store
            .validation_state_record_count_for_session(&session.session_id)
            .await
            .expect("validation state record count"),
        0
    );
}

#[tokio::test]
async fn current_validation_summary_blocks_pending_summary() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .record_inference_validation_session(
            &session.session_id,
            validation_session(
                &session.graph_revision,
                DraftGraphValidationStatus::Pending,
                false,
            ),
        )
        .await
        .expect("record current validation summary");

    let response = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
        })
        .await
        .expect("current validation summary response");

    assert_eq!(
        response.state,
        WorkflowGraphCurrentValidationSummaryState::Pending
    );
    assert!(!response.submit_gate.allowed);
    assert_eq!(
        response.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::ValidationPending)
    );
}

#[tokio::test]
async fn refresh_current_validation_summary_generates_backend_validation_session() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;

    let response = store
        .refresh_current_validation_summary(WorkflowGraphCurrentValidationRefreshRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
        })
        .await
        .expect("refresh current validation summary");

    assert_eq!(
        response.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );
    assert!(response.summary.submit_gate.allowed);
    let validation_session_id = response
        .summary
        .validation_session_id
        .as_ref()
        .expect("backend generated validation session id");
    assert!(validation_session_id
        .as_str()
        .starts_with("validation.session."));
    assert_ne!(validation_session_id.as_str(), "validation.session.1");
    assert_eq!(response.node_projections.len(), 1);
    assert_eq!(response.node_projections[0].node_id.as_str(), "infer");
}

#[tokio::test]
async fn refresh_current_validation_summary_rejects_stale_requested_revision() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;

    let response = store
        .refresh_current_validation_summary(WorkflowGraphCurrentValidationRefreshRequest {
            graph_session_id: session.session_id.clone(),
            graph_revision: "stale-revision".parse().expect("valid stale revision"),
        })
        .await
        .expect("refresh current validation summary");

    assert_eq!(
        response.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Stale
    );
    assert!(!response.summary.submit_gate.allowed);
    assert!(response.summary.validation_session_id.is_none());
    assert!(response.node_projections.is_empty());
    assert_eq!(
        response.summary.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::GraphRevisionStale)
    );
}

#[tokio::test]
async fn refresh_current_validation_summary_rejects_revision_changed_during_fact_lookup() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(BlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let requested_revision: WorkflowGraphRevision = session
        .graph_revision
        .parse()
        .expect("valid graph revision");
    let refresh_store = Arc::clone(&store);
    let session_id = session.session_id.clone();
    let refresh = tokio::spawn(async move {
        refresh_store
            .refresh_current_validation_summary(WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: session_id,
                graph_revision: requested_revision,
            })
            .await
    });
    entered.notified().await;

    let updated = store
        .remove_edge(WorkflowGraphRemoveEdgeRequest {
            session_id: session.session_id.clone(),
            edge_id: "dep-env-to-infer".to_string(),
        })
        .await
        .expect("mutate graph while validation facts are pending");
    release.notify_one();
    let response = refresh
        .await
        .expect("refresh task should not panic")
        .expect("refresh response");

    assert_eq!(
        response.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Stale
    );
    assert!(response.node_projections.is_empty());
    assert!(!response.summary.submit_gate.allowed);
    assert_eq!(
        response.summary.submit_gate.reason_code,
        Some(WorkflowGraphValidationSubmitGateReason::GraphRevisionStale)
    );

    let current = store
        .current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: session.session_id,
            graph_revision: updated
                .graph_revision
                .parse()
                .expect("valid updated graph revision"),
        })
        .await
        .expect("current validation summary response");
    assert_eq!(
        current.state,
        WorkflowGraphCurrentValidationSummaryState::Missing
    );
}

#[tokio::test]
async fn refresh_current_validation_summary_rejects_superseded_validation_session() {
    let first_entered = Arc::new(Notify::new());
    let first_release = Arc::new(Notify::new());
    let second_entered = Arc::new(Notify::new());
    let second_release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(SequencedBlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            calls: std::sync::Mutex::new(0),
            first_entered: Arc::clone(&first_entered),
            first_release: Arc::clone(&first_release),
            second_entered: Arc::clone(&second_entered),
            second_release: Arc::clone(&second_release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let requested_revision: WorkflowGraphRevision = session
        .graph_revision
        .parse()
        .expect("valid graph revision");
    let first_revision = requested_revision.clone();
    let second_revision = requested_revision;
    let first_store = Arc::clone(&store);
    let first_session_id = session.session_id.clone();
    let first = tokio::spawn(async move {
        first_store
            .refresh_current_validation_summary(WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: first_session_id,
                graph_revision: first_revision,
            })
            .await
    });
    first_entered.notified().await;

    let second_store = Arc::clone(&store);
    let second_session_id = session.session_id.clone();
    let second = tokio::spawn(async move {
        second_store
            .refresh_current_validation_summary(WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: second_session_id,
                graph_revision: second_revision,
            })
            .await
    });
    second_entered.notified().await;
    second_release.notify_one();
    let second_response = second
        .await
        .expect("second refresh task should not panic")
        .expect("second refresh response");
    assert_eq!(
        second_response.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );

    first_release.notify_one();
    let first_response = first
        .await
        .expect("first refresh task should not panic")
        .expect("first refresh response");
    assert_eq!(
        first_response.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );
    assert!(first_response.node_projections.is_empty());
    assert_eq!(
        first_response.summary.validation_session_id,
        second_response.summary.validation_session_id
    );
}

#[tokio::test]
async fn publish_inference_validation_session_records_current_summary() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let publication = store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.2")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    assert_eq!(publication.node_projections.len(), 1);
    assert_eq!(
        publication.validation_session.summary.status,
        DraftGraphValidationStatus::Executable
    );
    assert!(publication.validation_session.summary.executable);

    let missing_check = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.2"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Check,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        missing_check.diagnostics[0].code,
        InferenceDiagnosticCode::DependencyRequirementsMissing
    );

    let resolved = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.2"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        resolved.status,
        DependencyEnvironmentActionIntentStatus::RequestReady
    );

    let ready_check = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.2"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Check,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        ready_check.status,
        DependencyEnvironmentActionIntentStatus::RequestReady
    );

    let projections = store
        .scheduler_inference_task_projections_for_session(
            &session.session_id,
            Some(
                "validation.session.2"
                    .parse()
                    .expect("valid validation session id"),
            ),
        )
        .await
        .expect("scheduler inference projections");
    let projection = projections
        .get(&pantograph_scheduler::SchedulerNodeId::parse("infer").expect("node id"))
        .expect("projection");
    let WorkflowSchedulerInferenceTaskProjection::Ready(projection) = projection else {
        panic!("expected ready scheduler projection");
    };
    assert_eq!(projection.task_type.as_str(), "image_generation");
    assert_eq!(projection.model_ref.model_id, "image/example/tiny");
}

#[tokio::test]
async fn scheduler_inference_task_projections_reject_semantic_node_data_stale_summary() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.scheduler.stale")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    let updated = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "infer".to_string(),
            data: serde_json::json!({
                "runtime": "cuda"
            }),
        })
        .await
        .expect("update semantic inference node data");

    assert_ne!(updated.graph_revision, session.graph_revision);

    let error = store
        .scheduler_inference_task_projections_for_session(
            &session.session_id,
            Some(
                "validation.session.scheduler.stale"
                    .parse()
                    .expect("valid validation session id"),
            ),
        )
        .await
        .expect_err("stale validation summary should not project scheduler tasks");

    assert!(
        error
            .message()
            .contains("validation summary is missing for the current graph revision"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn executable_validation_snapshot_source_rejects_semantic_node_data_stale_summary() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.snapshot.stale")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    let updated = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "infer".to_string(),
            data: serde_json::json!({
                "runtime": "cuda"
            }),
        })
        .await
        .expect("update semantic inference node data");

    assert_ne!(updated.graph_revision, session.graph_revision);

    let error = store
        .executable_validation_snapshot_source_for_session(
            &session.session_id,
            Some(
                "validation.session.snapshot.stale"
                    .parse()
                    .expect("valid validation session id"),
            ),
        )
        .await
        .expect_err("stale validation summary should not produce snapshot source");

    assert!(
        error
            .message()
            .contains("validation summary is missing for the current graph revision"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn publish_inference_validation_session_rejects_graph_changed_during_fact_lookup() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(BlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let publish_store = Arc::clone(&store);
    let session_id = session.session_id.clone();
    let publish = tokio::spawn(async move {
        publish_store
            .publish_inference_validation_session(
                &session_id,
                DraftGraphValidationSessionId::parse("validation.session.stale.publish")
                    .expect("valid validation session id"),
            )
            .await
    });
    entered.notified().await;

    store
        .remove_edge(WorkflowGraphRemoveEdgeRequest {
            session_id: session.session_id.clone(),
            edge_id: "dep-env-to-infer".to_string(),
        })
        .await
        .expect("mutate graph while validation facts are pending");
    release.notify_one();

    let error = publish
        .await
        .expect("publish task should not panic")
        .expect_err("publish should reject cancelled validation session");
    assert!(
        error
            .to_string()
            .contains("validation publication cancelled: graph revision changed"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn publish_inference_validation_session_rejects_remove_edges_changed_during_fact_lookup() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(BlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let publish_store = Arc::clone(&store);
    let session_id = session.session_id.clone();
    let publish = tokio::spawn(async move {
        publish_store
            .publish_inference_validation_session(
                &session_id,
                DraftGraphValidationSessionId::parse("validation.session.remove.edges.changed")
                    .expect("valid validation session id"),
            )
            .await
    });
    entered.notified().await;

    store
        .remove_edges(WorkflowGraphRemoveEdgesRequest {
            session_id: session.session_id,
            edge_ids: vec!["dep-env-to-infer".to_string()],
        })
        .await
        .expect("mutate graph while validation facts are pending");
    release.notify_one();

    let error = publish
        .await
        .expect("publish task should not panic")
        .expect_err("publish should reject cancelled validation session");
    assert!(
        error
            .to_string()
            .contains("validation publication cancelled: graph revision changed"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn publish_inference_validation_session_rejects_add_node_changed_during_fact_lookup() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(BlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let publish_store = Arc::clone(&store);
    let session_id = session.session_id.clone();
    let publish = tokio::spawn(async move {
        publish_store
            .publish_inference_validation_session(
                &session_id,
                DraftGraphValidationSessionId::parse("validation.session.add.node.changed")
                    .expect("valid validation session id"),
            )
            .await
    });
    entered.notified().await;

    store
        .add_node(WorkflowGraphAddNodeRequest {
            session_id: session.session_id,
            node: GraphNode {
                id: "notes".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 40.0, y: 240.0 },
                data: serde_json::json!({
                    "text": "notes"
                }),
            },
        })
        .await
        .expect("mutate graph while validation facts are pending");
    release.notify_one();

    let error = publish
        .await
        .expect("publish task should not panic")
        .expect_err("publish should reject cancelled validation session");
    assert!(
        error
            .to_string()
            .contains("validation publication cancelled: graph revision changed"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn publish_inference_validation_session_rejects_node_data_changed_during_fact_lookup() {
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(BlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            entered: Arc::clone(&entered),
            release: Arc::clone(&release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    let publish_store = Arc::clone(&store);
    let session_id = session.session_id.clone();
    let publish = tokio::spawn(async move {
        publish_store
            .publish_inference_validation_session(
                &session_id,
                DraftGraphValidationSessionId::parse("validation.session.node.data.changed")
                    .expect("valid validation session id"),
            )
            .await
    });
    entered.notified().await;

    store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id,
            node_id: "infer".to_string(),
            data: serde_json::json!({
                "label": "Infer Changed"
            }),
        })
        .await
        .expect("mutate node data while validation facts are pending");
    release.notify_one();

    let error = publish
        .await
        .expect("publish task should not panic")
        .expect_err("publish should reject cancelled validation session");
    assert!(
        error
            .to_string()
            .contains("validation publication cancelled: graph revision changed"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn publish_inference_validation_session_rejects_superseded_validation_session() {
    let first_entered = Arc::new(Notify::new());
    let first_release = Arc::new(Notify::new());
    let second_entered = Arc::new(Notify::new());
    let second_release = Arc::new(Notify::new());
    let store = Arc::new(GraphSessionStore::with_inference_interface_facts_provider(
        Arc::new(SequencedBlockingInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
            calls: std::sync::Mutex::new(0),
            first_entered: Arc::clone(&first_entered),
            first_release: Arc::clone(&first_release),
            second_entered: Arc::clone(&second_entered),
            second_release: Arc::clone(&second_release),
        }),
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;

    let first_store = Arc::clone(&store);
    let first_session_id = session.session_id.clone();
    let first = tokio::spawn(async move {
        first_store
            .publish_inference_validation_session(
                &first_session_id,
                DraftGraphValidationSessionId::parse("validation.session.superseded.publish")
                    .expect("valid validation session id"),
            )
            .await
    });
    first_entered.notified().await;

    let second_store = Arc::clone(&store);
    let second_session_id = session.session_id.clone();
    let second = tokio::spawn(async move {
        second_store
            .publish_inference_validation_session(
                &second_session_id,
                DraftGraphValidationSessionId::parse("validation.session.current.publish")
                    .expect("valid validation session id"),
            )
            .await
    });
    second_entered.notified().await;
    second_release.notify_one();

    let second_publication = second
        .await
        .expect("second publish task should not panic")
        .expect("second publish response");
    assert_eq!(
        second_publication.validation_session.summary.status,
        DraftGraphValidationStatus::Executable
    );

    first_release.notify_one();
    let error = first
        .await
        .expect("first publish task should not panic")
        .expect_err("first publish should reject superseded validation session");
    assert!(
        error
            .to_string()
            .contains("validation publication cancelled: validation session was superseded"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn dependency_environment_action_intent_blocks_invalid_service_output() {
    let store = GraphSessionStore::with_timeout_and_providers(
        std::time::Duration::from_secs(5 * 60),
        Arc::new(StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        }),
        Arc::new(InvalidDependencyEnvironmentProvider),
    );
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.invalid.provider")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    let result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.invalid.provider"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        result.diagnostics[0].code,
        InferenceDiagnosticCode::DependencySidecarDescriptorInvalid
    );
}

#[tokio::test]
async fn publish_inference_validation_session_defaults_to_unavailable_facts() {
    let store = GraphSessionStore::new();
    let session = store.create_session(inference_graph(), None).await;
    let publication = store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.3")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    assert_eq!(
        publication.validation_session.summary.status,
        DraftGraphValidationStatus::Blocked
    );
    assert!(!publication.validation_session.summary.executable);
    assert_eq!(publication.node_projections.len(), 1);

    let error = store
        .scheduler_inference_task_projections_for_session(
            &session.session_id,
            Some(
                "validation.session.3"
                    .parse()
                    .expect("valid validation session id"),
            ),
        )
        .await
        .expect_err("non-executable validation should not project scheduler tasks");
    assert!(error.message().contains("not executable"));
}

#[tokio::test]
async fn dependency_environment_action_intent_rejects_stale_graph_revision() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: "0000000000000000".parse().expect("valid graph revision"),
            validation_session_id: None,
            target_node_id: "text-input".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Check,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        result.diagnostics[0].code,
        InferenceDiagnosticCode::GraphRevisionMismatch
    );
}

#[tokio::test]
async fn dependency_environment_action_intent_rejects_semantic_node_data_stale_summary() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let session = store
        .create_session(dependency_inference_graph(), None)
        .await;
    store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.dependency.stale")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    let updated = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "infer".to_string(),
            data: serde_json::json!({
                "runtime": "cuda"
            }),
        })
        .await
        .expect("update semantic inference node data");

    assert_ne!(updated.graph_revision, session.graph_revision);

    let stale_result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid original graph revision"),
            validation_session_id: Some(
                "validation.session.dependency.stale"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed stale result");

    assert_eq!(
        stale_result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        stale_result.diagnostics[0].code,
        InferenceDiagnosticCode::GraphRevisionMismatch
    );

    let current_result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: updated
                .graph_revision
                .parse()
                .expect("valid updated graph revision"),
            validation_session_id: Some(
                "validation.session.dependency.stale"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed missing result");

    assert_eq!(
        current_result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        current_result.diagnostics[0].code,
        InferenceDiagnosticCode::ValidationSummaryMissing
    );
}

#[tokio::test]
async fn dependency_environment_action_intent_blocks_invalid_sidecar_choices() {
    let store = GraphSessionStore::with_inference_interface_facts_provider(Arc::new(
        StaticInferenceFactsProvider {
            facts: BTreeMap::from([("infer".to_string(), ready_inference_facts())]),
        },
    ));
    let mut graph = dependency_inference_graph();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "dep-env")
        .expect("dependency environment node exists")
        .data = serde_json::json!({
        "selected_binding_ids": "binding-a"
    });
    let session = store.create_session(graph, None).await;
    store
        .publish_inference_validation_session(
            &session.session_id,
            DraftGraphValidationSessionId::parse("validation.session.2")
                .expect("valid validation session id"),
        )
        .await
        .expect("publish inference validation session");

    let result = store
        .resolve_dependency_environment_action_intent(DependencyEnvironmentActionIntent {
            contract_version: 1,
            graph_session_id: session.session_id.parse().expect("valid graph session id"),
            graph_revision: session
                .graph_revision
                .parse()
                .expect("valid graph revision"),
            validation_session_id: Some(
                "validation.session.2"
                    .parse()
                    .expect("valid validation session id"),
            ),
            target_node_id: "dep-env".parse().expect("valid target node id"),
            action: DependencyEnvironmentAction::Resolve,
        })
        .await
        .expect("intent resolution should return typed result");

    assert_eq!(
        result.status,
        DependencyEnvironmentActionIntentStatus::Blocked
    );
    assert_eq!(
        result.diagnostics[0].code,
        InferenceDiagnosticCode::InvalidOption
    );
    assert_eq!(
        result.diagnostics[0]
            .port_id
            .as_ref()
            .map(|port_id| port_id.as_str()),
        Some("selected_binding_ids")
    );
}

fn inference_graph() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({
                    "pumas_model_ref": {
                        "model_id": "image/example/tiny",
                        "selected_artifact_id": "diffusers"
                    }
                }),
            },
            GraphNode {
                id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch"
                }),
            },
        ],
        edges: vec![GraphEdge {
            id: "model-to-infer".to_string(),
            source: "model".to_string(),
            source_handle: "pumas_model_ref".to_string(),
            target: "infer".to_string(),
            target_handle: "pumas_model_ref".to_string(),
        }],
        derived_graph: None,
    }
}

fn dependency_inference_graph() -> WorkflowGraph {
    let mut graph = inference_graph();
    graph.nodes.push(GraphNode {
        id: "dep-env".to_string(),
        node_type: "dependency-environment".to_string(),
        position: Position { x: 400.0, y: 0.0 },
        data: serde_json::json!({
            "mode": "manual"
        }),
    });
    graph.edges.push(GraphEdge {
        id: "dep-env-to-infer".to_string(),
        source: "dep-env".to_string(),
        source_handle: "dependency_environment_sidecar".to_string(),
        target: "infer".to_string(),
        target_handle: "dependency_environment_sidecar".to_string(),
    });
    graph
}

#[derive(Debug)]
struct StaticInferenceFactsProvider {
    facts: BTreeMap<String, InferenceInterfaceResolverFacts>,
}

#[async_trait]
impl InferenceInterfaceFactsProvider for StaticInferenceFactsProvider {
    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        Ok(inputs
            .iter()
            .filter_map(|input| {
                self.facts
                    .get(&input.node_id)
                    .cloned()
                    .map(|facts| (input.node_id.clone(), facts))
            })
            .collect())
    }
}

#[derive(Debug)]
struct BlockingInferenceFactsProvider {
    facts: BTreeMap<String, InferenceInterfaceResolverFacts>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}

#[async_trait]
impl InferenceInterfaceFactsProvider for BlockingInferenceFactsProvider {
    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        self.entered.notify_one();
        self.release.notified().await;
        Ok(inputs
            .iter()
            .filter_map(|input| {
                self.facts
                    .get(&input.node_id)
                    .cloned()
                    .map(|facts| (input.node_id.clone(), facts))
            })
            .collect())
    }
}

#[derive(Debug)]
struct SequencedBlockingInferenceFactsProvider {
    facts: BTreeMap<String, InferenceInterfaceResolverFacts>,
    calls: std::sync::Mutex<usize>,
    first_entered: Arc<Notify>,
    first_release: Arc<Notify>,
    second_entered: Arc<Notify>,
    second_release: Arc<Notify>,
}

#[async_trait]
impl InferenceInterfaceFactsProvider for SequencedBlockingInferenceFactsProvider {
    async fn facts_for_resolution_inputs(
        &self,
        inputs: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        let call_index = {
            let mut calls = self.calls.lock().expect("call counter lock");
            *calls += 1;
            *calls
        };
        match call_index {
            1 => {
                self.first_entered.notify_one();
                self.first_release.notified().await;
            }
            2 => {
                self.second_entered.notify_one();
                self.second_release.notified().await;
            }
            _ => {
                return Err(InferenceInterfaceFactsProviderError::Resolve(
                    "unexpected validation facts call".to_string(),
                ));
            }
        }
        Ok(inputs
            .iter()
            .filter_map(|input| {
                self.facts
                    .get(&input.node_id)
                    .cloned()
                    .map(|facts| (input.node_id.clone(), facts))
            })
            .collect())
    }
}

fn ready_inference_facts() -> InferenceInterfaceResolverFacts {
    let runtime = RuntimeIntentId::parse("pytorch").expect("valid runtime id");
    InferenceInterfaceResolverFacts {
        model: InferenceModelResolutionFacts {
            state: InferenceModelResolutionState::Ready,
        },
        capability: Some(InferenceCapabilityFacts {
            task_kind: InferenceTaskKind::parse("image_generation").expect("valid task kind"),
            inputs: vec![InferencePortDescriptor {
                port_id: InferencePortId::parse("prompt").expect("valid port id"),
                label: "Prompt".to_string(),
                direction: InferencePortDirection::Input,
                requirement: InferencePortRequirement::Required,
                value_type: InferenceValueType::Scalar(InferenceScalarType::String),
                default: None,
                options: InferencePortOptions::None,
                availability: InferenceAvailability::available(),
                runtime_conditions: Vec::new(),
                diagnostics: Vec::new(),
            }],
            outputs: Vec::new(),
            runtime_conditions: Vec::new(),
            supported_runtime_ids: vec![runtime.clone()],
        }),
        runtimes: vec![InferenceRuntimeAvailabilityFact {
            runtime_id: runtime,
            state: InferenceRuntimeAvailabilityState::Available,
            device_ids: Vec::new(),
        }],
    }
}

fn validation_session(
    graph_revision: &str,
    status: DraftGraphValidationStatus,
    executable: bool,
) -> WorkflowGraphInferenceValidationSession {
    WorkflowGraphInferenceValidationSession {
        contract_version: INFERENCE_INTERFACE_CONTRACT_VERSION,
        validation_session_id: "validation.session.1"
            .parse()
            .expect("valid validation session id"),
        graph_revision: graph_revision.parse().expect("valid graph revision"),
        latest_sequence: 0,
        summary: DraftGraphValidationSummary {
            status,
            executable,
            enqueue_disabled_reasons: Vec::new(),
            diagnostics_count: 0,
            blocking_diagnostics_count: 0,
        },
        events: Vec::new(),
    }
}

#[tokio::test]
async fn update_node_data_merges_patch_into_existing_data() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated",
                "placeholder": "Prompt"
            }),
        })
        .await
        .expect("update node data");

    let node = response
        .graph
        .find_node("text-input")
        .expect("text-input node");
    assert_eq!(node.data["text"], "updated");
    assert_eq!(node.data["placeholder"], "Prompt");
    assert_eq!(node.data["label"], "Text Input");
    assert!(node.data.get("definition").is_some());
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-input".to_string(), "text-output".to_string()]
    ));
    let memory_impact = response
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(!memory_impact.fallback_to_full_invalidation);
    assert_eq!(memory_impact.node_decisions.len(), 2);
    assert!(matches!(
        memory_impact.node_decisions.as_slice(),
        [
            node_engine::NodeMemoryCompatibilitySnapshot {
                node_id,
                compatibility,
                reason: Some(reason),
            },
            node_engine::NodeMemoryCompatibilitySnapshot {
                node_id: dependent_node_id,
                compatibility: dependent_compatibility,
                reason: Some(dependent_reason),
            }
        ] if node_id == "text-input"
            && *compatibility == node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
            && reason == "node_data_changed"
            && dependent_node_id == "text-output"
            && *dependent_compatibility
                == node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
            && dependent_reason == "upstream_dependency_changed"
    ));
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            memory_impact: Some(memory_impact),
            ..
        }) if memory_impact.node_decisions.len() == 2
    ));
}

#[tokio::test]
async fn update_node_position_updates_session_graph() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
            position: Position { x: 320.0, y: 48.0 },
        })
        .await
        .expect("update node position");

    let node = response
        .graph
        .find_node("text-output")
        .expect("text-output node");
    assert_eq!(node.position, Position { x: 320.0, y: 48.0 });
    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == session.session_id
            && execution_id == session.session_id
            && dirty_tasks.is_empty()
    ));
    assert_eq!(
        response
            .workflow_execution_session_state
            .expect("workflow execution session state")
            .memory_impact,
        None
    );
}

#[tokio::test]
async fn remove_node_prunes_attached_edges() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    let response = store
        .remove_node(WorkflowGraphRemoveNodeRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
        })
        .await
        .expect("remove node");

    assert!(response.graph.find_node("text-output").is_none());
    assert!(response.graph.edges.is_empty());
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-output".to_string()]
    ));
    let memory_impact = response
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 1);
    assert_eq!(
        memory_impact.node_decisions[0].compatibility,
        node_engine::NodeMemoryCompatibility::DropOnIdentityChange
    );
    assert_eq!(
        memory_impact.node_decisions[0].reason.as_deref(),
        Some("node_removed")
    );
}

#[tokio::test]
async fn remove_edges_removes_multiple_edges_with_one_undo_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(branching_graph(), None).await;

    let response = store
        .remove_edges(WorkflowGraphRemoveEdgesRequest {
            session_id: session.session_id.clone(),
            edge_ids: vec![
                "text-input-text-text-output-text".to_string(),
                "text-input-text-text-copy-text".to_string(),
            ],
        })
        .await
        .expect("remove edges");

    assert!(response.graph.edges.is_empty());
    let undo_state = store
        .get_undo_redo_state(&session.session_id)
        .await
        .expect("undo state");
    assert_eq!(undo_state.undo_count, 1);

    let undo_response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo remove edges");
    assert_eq!(undo_response.graph.edges.len(), 2);
}

#[tokio::test]
async fn delete_selection_removes_mixed_selection_with_one_undo_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(branching_graph(), None).await;

    let response = store
        .delete_selection(WorkflowGraphDeleteSelectionRequest {
            session_id: session.session_id.clone(),
            node_ids: vec!["text-copy".to_string()],
            edge_ids: vec!["text-input-text-text-output-text".to_string()],
        })
        .await
        .expect("delete selection");

    assert!(response.graph.find_node("text-copy").is_none());
    assert!(response.graph.edges.is_empty());
    let undo_state = store
        .get_undo_redo_state(&session.session_id)
        .await
        .expect("undo state");
    assert_eq!(undo_state.undo_count, 1);

    let undo_response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo delete selection");
    assert!(undo_response.graph.find_node("text-copy").is_some());
    assert_eq!(undo_response.graph.edges.len(), 2);
}

#[tokio::test]
async fn undo_response_carries_backend_owned_graph_modified_event() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated"
            }),
        })
        .await
        .expect("update node data");

    let response = store
        .undo(WorkflowGraphEditSessionGraphRequest {
            session_id: session.session_id.clone(),
        })
        .await
        .expect("undo graph edit");

    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == session.session_id
            && execution_id == session.session_id
            && dirty_tasks == vec!["text-input".to_string(), "text-output".to_string()]
    ));
}

#[tokio::test]
async fn get_session_graph_replays_last_memory_impact_until_a_non_invalidating_edit_clears_it() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;

    store
        .update_node_data(WorkflowGraphUpdateNodeDataRequest {
            session_id: session.session_id.clone(),
            node_id: "text-input".to_string(),
            data: serde_json::json!({
                "text": "updated"
            }),
        })
        .await
        .expect("update node data");

    let after_data_edit = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after data edit");
    let memory_impact = after_data_edit
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 2);
    assert!(!memory_impact.fallback_to_full_invalidation);

    store
        .update_node_position(WorkflowGraphUpdateNodePositionRequest {
            session_id: session.session_id.clone(),
            node_id: "text-output".to_string(),
            position: Position { x: 240.0, y: 32.0 },
        })
        .await
        .expect("update node position");

    let after_position_edit = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after position edit");
    assert_eq!(
        after_position_edit
            .workflow_execution_session_state
            .expect("workflow execution session state")
            .memory_impact,
        None
    );
}

#[tokio::test]
async fn insert_node_on_edge_replaces_original_edge_in_session_graph() {
    let store = GraphSessionStore::new();
    let session = store.create_session(sample_graph(), None).await;
    let session_id = session.session_id.clone();

    let response = store
        .insert_node_on_edge(WorkflowGraphInsertNodeOnEdgeRequest {
            session_id: session_id.clone(),
            edge_id: "text-input-text-text-output-text".to_string(),
            node_type: "llm-inference".to_string(),
            graph_revision: session.graph_revision,
            position_hint: InsertNodePositionHint {
                position: Position { x: 80.0, y: 24.0 },
            },
        })
        .await
        .expect("insert node on edge");

    assert!(response.accepted);
    let graph = response.graph.expect("updated graph");
    assert_eq!(graph.edges.len(), 2);
    assert!(graph
        .edges
        .iter()
        .all(|edge| edge.id != "text-input-text-text-output-text"));
    let inserted_node_id = response.inserted_node_id.expect("inserted node id");
    assert!(graph.find_node(&inserted_node_id).is_some());
    assert!(matches!(
        response.workflow_event,
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            ..
        }) if workflow_id == session_id && execution_id == session_id
    ));
    let response_memory_impact = response
        .workflow_execution_session_state
        .clone()
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(response_memory_impact
        .node_decisions
        .iter()
        .any(|decision| decision.node_id == inserted_node_id));

    let snapshot = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after insert");
    let memory_impact = snapshot
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert!(!memory_impact.node_decisions.is_empty());
    assert!(memory_impact
        .node_decisions
        .iter()
        .any(|decision| decision.node_id == inserted_node_id));
}

#[tokio::test]
async fn connect_persists_memory_impact_for_later_session_snapshot() {
    let store = GraphSessionStore::new();
    let session = store.create_session(disconnected_graph(), None).await;

    let response = store
        .connect(WorkflowGraphConnectRequest {
            session_id: session.session_id.clone(),
            graph_revision: session.graph_revision,
            source_anchor: ConnectionAnchor {
                node_id: "text-input".to_string(),
                port_id: "text".to_string(),
            },
            target_anchor: ConnectionAnchor {
                node_id: "text-output".to_string(),
                port_id: "text".to_string(),
            },
        })
        .await
        .expect("connect nodes");
    assert!(response.accepted);
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            workflow_id,
            execution_id,
            dirty_tasks,
            ..
        }) if workflow_id == &session.session_id
            && execution_id == &session.session_id
            && dirty_tasks == &vec!["text-output".to_string()]
    ));
    assert!(matches!(
        response.workflow_event.as_ref(),
        Some(node_engine::WorkflowEvent::GraphModified {
            memory_impact: Some(memory_impact),
            ..
        }) if !memory_impact.node_decisions.is_empty()
    ));
    let response_memory_impact = response
        .workflow_execution_session_state
        .clone()
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(response_memory_impact.node_decisions.len(), 1);
    assert_eq!(
        response_memory_impact.node_decisions[0].node_id,
        "text-output"
    );

    let snapshot = store
        .get_session_graph(&session.session_id)
        .await
        .expect("get session graph after connect");
    let memory_impact = snapshot
        .workflow_execution_session_state
        .expect("workflow execution session state")
        .memory_impact
        .expect("memory impact");
    assert_eq!(memory_impact.node_decisions.len(), 1);
    assert_eq!(memory_impact.node_decisions[0].node_id, "text-output");
    assert_eq!(
        memory_impact.node_decisions[0].compatibility,
        node_engine::NodeMemoryCompatibility::PreserveWithInputRefresh
    );
    assert_eq!(
        memory_impact.node_decisions[0].reason.as_deref(),
        Some("edge_topology_changed")
    );
}

#[tokio::test]
async fn connect_canonicalizes_llm_stream_drop_to_text_output_response_edge() {
    let store = GraphSessionStore::new();
    let session = store
        .create_session(inference_to_output_graph(), None)
        .await;

    let response = store
        .connect(WorkflowGraphConnectRequest {
            session_id: session.session_id.clone(),
            graph_revision: session.graph_revision,
            source_anchor: ConnectionAnchor {
                node_id: "llm".to_string(),
                port_id: "stream".to_string(),
            },
            target_anchor: ConnectionAnchor {
                node_id: "output".to_string(),
                port_id: "stream".to_string(),
            },
        })
        .await
        .expect("connect stream edge");

    assert!(response.accepted);
    let graph = response.graph.expect("updated graph");
    assert_eq!(graph.edges.len(), 1);
    let edge = &graph.edges[0];
    assert_eq!(edge.id, "llm-response-output-text");
    assert_eq!(edge.source, "llm");
    assert_eq!(edge.source_handle, "response");
    assert_eq!(edge.target, "output");
    assert_eq!(edge.target_handle, "text");
    assert!(!graph.edges.iter().any(|edge| {
        edge.source == "llm"
            && edge.source_handle == "stream"
            && edge.target == "output"
            && edge.target_handle == "stream"
    }));
}

#[derive(Debug, Clone, Copy)]
struct InvalidDependencyEnvironmentProvider;

impl DependencyEnvironmentProvider for InvalidDependencyEnvironmentProvider {
    fn resolve(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        let mut result = NotImplementedDependencyEnvironmentProvider.resolve(request);
        result.diagnostics.clear();
        result
    }

    fn check(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve(request)
    }

    fn install(
        &self,
        request: &ValidatedDependencyEnvironmentRequest,
    ) -> DependencyEnvironmentResult {
        self.resolve(request)
    }
}
