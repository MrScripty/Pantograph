//! Real backend preflight through the existing public current-validation API.
//! These tests do not compose the separate group UI successor.
use async_trait::async_trait;
use pantograph_inference_interface_contracts::{
    DraftGraphValidationSessionId, DraftGraphValidationStatus, InferenceDiagnosticCode as Code,
};
use pantograph_workflow_service::graph::{
    GraphSessionStore, InferenceInterfaceFactsProvider, InferenceInterfaceFactsProviderError,
    InferenceInterfaceGraphResolutionInput, InferenceInterfaceResolverFacts,
};
use pantograph_workflow_service::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Semaphore;

#[derive(Debug)]
struct Facts {
    calls: AtomicUsize,
    block: Vec<usize>,
    entered: Semaphore,
    release: Semaphore,
}
impl Facts {
    fn new(block: Vec<usize>) -> Arc<Self> {
        Arc::new(Self {
            calls: AtomicUsize::new(0),
            block,
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
        })
    }
    async fn wait_entered(&self) {
        tokio::time::timeout(Duration::from_secs(5), self.entered.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    }
}
#[async_trait]
impl InferenceInterfaceFactsProvider for Facts {
    async fn facts_for_resolution_inputs(
        &self,
        _: &[InferenceInterfaceGraphResolutionInput],
    ) -> Result<
        BTreeMap<String, InferenceInterfaceResolverFacts>,
        InferenceInterfaceFactsProviderError,
    > {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if self.block.contains(&call) {
            self.entered.add_permits(1);
            self.release.acquire().await.unwrap().forget();
        }
        Ok(BTreeMap::new())
    }
}

fn node(id: &str, kind: &str) -> GraphNode {
    let data = if matches!(kind, "text-input" | "text-output") {
        json!({"definition":NodeRegistry::new().get_definition(kind).unwrap()})
    } else {
        json!({})
    };
    GraphNode {
        id: id.into(),
        node_type: kind.into(),
        position: Position::default(),
        data,
    }
}
fn edge(id: &str, source: &str, source_port: &str, target: &str, target_port: &str) -> GraphEdge {
    GraphEdge {
        id: id.into(),
        source: source.into(),
        source_handle: source_port.into(),
        target: target.into(),
        target_handle: target_port.into(),
    }
}
async fn authored() -> WorkflowGraph {
    let graph = WorkflowGraph {
        nodes: vec![
            node("source", "text-input"),
            node("a", "merge"),
            node("b", "merge"),
            node("sink", "text-output"),
            node("sink2", "text-output"),
        ],
        edges: vec![
            edge("in", "source", "text", "a", "inputs"),
            edge("middle", "a", "merged", "b", "inputs"),
            edge("out", "b", "merged", "sink", "text"),
            edge("fan", "b", "merged", "sink2", "text"),
        ],
        ..WorkflowGraph::default()
    };
    let sessions = GraphSessionStore::new();
    let edit = sessions.create_session(graph, None).await;
    let grouped = sessions
        .create_group(WorkflowGraphCreateGroupRequest {
            session_id: edit.session_id.clone(),
            name: "Public preflight".into(),
            selected_node_ids: vec!["a".into(), "b".into()],
        })
        .await
        .unwrap()
        .graph;
    sessions.close_session(&edit.session_id).await.unwrap();
    grouped
}
fn group(graph: &mut WorkflowGraph) -> &mut Value {
    &mut graph
        .nodes
        .iter_mut()
        .find(|n| n.node_type == "node-group")
        .unwrap()
        .data["group"]
}
async fn open(
    service: &WorkflowService,
    graph: WorkflowGraph,
) -> WorkflowGraphEditSessionCreateResponse {
    service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph,
            workflow_id: Some("preflight".into()),
        })
        .await
        .unwrap()
}
async fn refresh(
    service: &WorkflowService,
    edit: &WorkflowGraphEditSessionCreateResponse,
) -> WorkflowGraphCurrentValidationRefreshResponse {
    service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: edit.session_id.clone(),
                graph_revision: edit.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap()
}
fn publish_request(
    edit: &WorkflowGraphEditSessionCreateResponse,
    generation: Option<DraftGraphValidationSessionId>,
) -> WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
    WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
        workflow_id: "preflight".into(),
        workflow_semantic_version: "0.1.0".into(),
        graph_session_id: edit.session_id.clone(),
        validation_session_id: generation,
        validation_snapshot_id: None,
    }
}
fn evidence(name: &str, value: &Value) {
    if let Ok(root) = std::env::var("GROUP_PREFLIGHT_EVIDENCE_DIR") {
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            PathBuf::from(root).join(format!("{name}.json")),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
}

#[tokio::test]
async fn valid_saved_fanout_has_bound_current_preflight_and_preserves_authored_bytes() {
    let graph = authored().await;
    let dir = std::env::temp_dir().join(format!(
        "pantograph-group-preflight-{}",
        uuid::Uuid::new_v4()
    ));
    let store = FileSystemWorkflowGraphStore::new(&dir);
    let path = store
        .save_workflow("preflight".into(), graph.clone())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let reopened = store.load_workflow(path.clone()).unwrap().graph;
    assert_eq!(reopened, graph);
    let service = WorkflowService::with_ephemeral_attribution_store().unwrap();
    let edit = open(&service, reopened).await;
    let before = service
        .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap()
        .graph;
    let result = refresh(&service, &edit).await;
    assert!(result.summary.submit_gate.allowed);
    assert_eq!(
        result.summary.state,
        WorkflowGraphCurrentValidationSummaryState::Current
    );
    let facts = result.summary.group_preflight.as_ref().unwrap();
    assert_eq!(facts.graph_session_id.as_str(), edit.session_id);
    assert_eq!(facts.graph_revision.as_str(), edit.graph_revision);
    assert_eq!(
        Some(&facts.validation_session_id),
        result.summary.validation_session_id.as_ref()
    );
    assert_eq!(facts.group_count, 1);
    assert!(facts.failures.is_empty());
    let snapshot = service
        .publish_graph_session_executable_validation_snapshot(publish_request(
            &edit,
            result.summary.validation_session_id.clone(),
        ))
        .await
        .unwrap();
    let after = service
        .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap()
        .graph;
    assert_eq!(before, after);
    let wrapper = after
        .nodes
        .iter()
        .find(|n| n.node_type == "node-group")
        .unwrap();
    assert_eq!(
        wrapper.data["group"]["exposed_outputs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    evidence(
        "valid-fanout",
        &json!({"summary":result.summary,"authored":after,"snapshot":snapshot.as_record(),"saved_bytes_unchanged":true}),
    );
    service
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: edit.session_id,
        })
        .await
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn public_preflight_reports_typed_malformed_unsupported_and_mapping_failures() {
    let valid = authored().await;
    let mut cases = Vec::new();
    let mut graph = valid.clone();
    group(&mut graph)["nodes"] = json!({});
    cases.push((
        "schema",
        graph,
        Code::GroupSchemaInvalid,
        GroupRejectionKind::Malformed,
        None,
        "group.nodes",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["id"] = json!("foreign-group");
    cases.push((
        "identity",
        graph,
        Code::GroupIdentityInvalid,
        GroupRejectionKind::Invalid,
        None,
        "group.id",
    ));
    for kind in ["llm-inference", "node-group", "future-processing"] {
        let mut graph = valid.clone();
        group(&mut graph)["nodes"][0]["node_type"] = json!(kind);
        cases.push((
            kind,
            graph,
            Code::GroupChildUnsupported,
            GroupRejectionKind::Unsupported,
            Some("a"),
            "group.nodes.node_type",
        ));
    }
    let mut graph = valid.clone();
    group(&mut graph)["exposed_inputs"][0]["internal_port_id"] = json!("missing");
    cases.push((
        "mapping",
        graph,
        Code::GroupMappingInvalid,
        GroupRejectionKind::Invalid,
        Some("a"),
        "group.exposed_inputs",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["nodes"][0]["data"]["definition"] = json!({"inputs":[{"id":"inputs","label":"Inputs","data_type":"string","required":false,"multiple":false}]});
    cases.push((
        "cpu-ports",
        graph,
        Code::GroupEffectivePortsUnsupported,
        GroupRejectionKind::Unsupported,
        Some("a"),
        "group.nodes.data.definition",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["nodes"][0]["id"] = json!("bad child ID!");
    cases.push((
        "child-id",
        graph,
        Code::GroupIdentityInvalid,
        GroupRejectionKind::Invalid,
        Some("bad child ID!"),
        "group.nodes.id",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["exposed_outputs"][1]["internal_node_id"] = json!("a");
    cases.push((
        "conflicting-mapping",
        graph,
        Code::GroupMappingInvalid,
        GroupRejectionKind::Invalid,
        Some("a"),
        "group.exposed_outputs",
    ));
    let mut graph = valid.clone();
    graph.edges[0].target_handle = "missing-boundary".into();
    cases.push((
        "missing-boundary",
        graph,
        Code::GroupMappingInvalid,
        GroupRejectionKind::Invalid,
        None,
        "edges.boundary_port",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["edges"][0]["target"] = json!("escaped-child");
    cases.push((
        "escaped-edge",
        graph,
        Code::GroupCompositionInvalid,
        GroupRejectionKind::Invalid,
        Some("escaped-child"),
        "group.edges",
    ));
    let mut graph = valid.clone();
    group(&mut graph)["nodes"][0]["id"] = json!("x".repeat(1024 * 1024));
    group(&mut graph)["nodes"][0]["node_type"] = json!("llm-inference");
    cases.push((
        "overlong-child-id",
        graph,
        Code::GroupChildUnsupported,
        GroupRejectionKind::Unsupported,
        None,
        "group.nodes.node_type",
    ));
    for (name, graph, code, kind, child, field) in cases {
        let facts = Facts::new(vec![]);
        let service = WorkflowService::with_ephemeral_attribution_store()
            .unwrap()
            .with_inference_interface_facts_provider(facts.clone());
        let authored_nodes = graph.nodes.clone();
        let edit = open(&service, graph).await;
        let result = refresh(&service, &edit).await;
        assert_eq!(
            service
                .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
                    session_id: edit.session_id.clone()
                })
                .await
                .unwrap()
                .graph
                .nodes,
            authored_nodes,
            "{name}: preflight must preserve authored nodes"
        );
        assert_eq!(
            result.summary.state,
            WorkflowGraphCurrentValidationSummaryState::Invalid,
            "{name}"
        );
        assert!(!result.summary.submit_gate.allowed, "{name}");
        assert_eq!(
            result.summary.submit_gate.reason_code,
            Some(WorkflowGraphValidationSubmitGateReason::BlockingDiagnostics)
        );
        assert_eq!(
            result.summary.summary.as_ref().unwrap().status,
            DraftGraphValidationStatus::Blocked
        );
        assert!(
            result
                .summary
                .summary
                .as_ref()
                .unwrap()
                .blocking_diagnostics_count
                > 0
        );
        let captured = result.summary.group_preflight.as_ref().unwrap();
        assert_eq!(captured.graph_revision.as_str(), edit.graph_revision);
        assert!(
            serde_json::to_vec(captured).unwrap().len() <= 16_384,
            "{name}: typed facts must remain bounded independently of raw IDs"
        );
        let failure = &captured.failures[0];
        assert_eq!(failure.code, code, "{name}");
        assert_eq!(failure.rejection_kind, kind, "{name}");
        assert!(failure.group_id.is_some());
        assert_eq!(failure.child_id.as_deref(), child);
        assert_eq!(failure.field, field, "{name}");
        assert!(failure.message.chars().count() <= 2048);
        assert!(
            !failure.field.is_empty()
                && !failure.message.is_empty()
                && !failure.repair_hint.is_empty()
        );
        assert!(failure.blocking_submission);
        assert_eq!(result.summary.diagnostics[0].code, code);
        assert_eq!(
            facts.calls.load(Ordering::SeqCst),
            0,
            "invalid groups must refuse before external facts lookup"
        );
        assert!(service
            .publish_graph_session_executable_validation_snapshot(publish_request(
                &edit,
                result.summary.validation_session_id.clone()
            ))
            .await
            .is_err());
        let current = service
            .workflow_graph_current_validation_summary(
                WorkflowGraphCurrentValidationSummaryRequest {
                    graph_session_id: edit.session_id.clone(),
                    graph_revision: edit.graph_revision.parse().unwrap(),
                },
            )
            .await
            .unwrap();
        assert_eq!(current, result.summary);
        let encoded = serde_json::to_value(&current).unwrap();
        assert_eq!(
            serde_json::from_value::<WorkflowGraphCurrentValidationSummaryResponse>(
                encoded.clone()
            )
            .unwrap(),
            current
        );
        evidence(name, &encoded);
        service
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit.session_id,
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn starting_background_generation_synchronously_revokes_previous_allowed_summary() {
    let facts = Facts::new(vec![1]);
    let service = WorkflowService::new().with_inference_interface_facts_provider(facts.clone());
    let edit = open(&service, authored().await).await;
    let previous = refresh(&service, &edit).await;
    assert!(previous.summary.submit_gate.allowed);
    let next = service
        .workflow_graph_start_current_validation_task(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: edit.session_id.clone(),
                graph_revision: edit.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert_ne!(Some(&next), previous.summary.validation_session_id.as_ref());
    // No wait/yield before the public read: Start has already invalidated the old generation.
    let current = service
        .workflow_graph_current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: edit.session_id.clone(),
            graph_revision: edit.graph_revision.parse().unwrap(),
        })
        .await
        .unwrap();
    assert!(!current.submit_gate.allowed);
    facts.wait_entered().await;
    service.workflow_graph_shutdown_validation_tasks().await;
    let canceled = service
        .workflow_graph_current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: edit.session_id.clone(),
            graph_revision: edit.graph_revision.parse().unwrap(),
        })
        .await
        .unwrap();
    assert!(!canceled.submit_gate.allowed);
    evidence(
        "start-and-cancel",
        &json!({"previous":previous.summary,"pending":current,"canceled":canceled}),
    );
}

#[tokio::test]
async fn superseded_refresh_never_inherits_newer_executable_generation() {
    let facts = Facts::new(vec![0]);
    let service =
        Arc::new(WorkflowService::new().with_inference_interface_facts_provider(facts.clone()));
    let edit = open(&service, authored().await).await;
    let first_service = service.clone();
    let first_edit = edit.clone();
    let first = tokio::spawn(async move { refresh(&first_service, &first_edit).await });
    facts.wait_entered().await;
    let newer = refresh(&service, &edit).await;
    assert!(newer.summary.submit_gate.allowed);
    let superseded = tokio::time::timeout(Duration::from_secs(5), first)
        .await
        .unwrap()
        .unwrap();
    assert!(!superseded.summary.submit_gate.allowed);
    assert!(superseded.summary.validation_session_id.is_none());
    assert!(superseded.node_projections.is_empty());
    assert_eq!(
        superseded.summary.diagnostics[0].code,
        Code::ValidationSessionSuperseded
    );
    let current = service
        .workflow_graph_current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: edit.session_id.clone(),
            graph_revision: edit.graph_revision.parse().unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(current, newer.summary);
    evidence(
        "superseded",
        &json!({"superseded":superseded.summary,"newer":current}),
    );
}

#[tokio::test]
async fn semantic_group_edit_blocks_stale_summary_and_old_snapshot_generation() {
    let service = WorkflowService::with_ephemeral_attribution_store().unwrap();
    let edit = open(&service, authored().await).await;
    let previous = refresh(&service, &edit).await;
    let before = service
        .workflow_graph_get_edit_session_graph(WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap()
        .graph;
    let wrapper = before
        .nodes
        .iter()
        .find(|n| n.node_type == "node-group")
        .unwrap();
    let child = &wrapper.data["group"]["nodes"][0];
    let changed = service
        .workflow_graph_update_group_node_data(WorkflowGraphUpdateGroupNodeDataRequest {
            session_id: edit.session_id.clone(),
            group_id: wrapper.id.clone(),
            node_id: "a".into(),
            expected_node_type: "merge".into(),
            expected_node_data: child["data"].clone(),
            data: json!({"separator":"semantic revision change"}),
        })
        .await
        .unwrap();
    assert_ne!(changed.graph_revision, edit.graph_revision);
    let stale = service
        .workflow_graph_current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: edit.session_id.clone(),
            graph_revision: edit.graph_revision.parse().unwrap(),
        })
        .await
        .unwrap();
    assert_eq!(
        stale.state,
        WorkflowGraphCurrentValidationSummaryState::Stale
    );
    assert!(!stale.submit_gate.allowed);
    assert!(stale.group_preflight.is_none());
    assert!(service
        .publish_graph_session_executable_validation_snapshot(publish_request(
            &edit,
            previous.summary.validation_session_id
        ))
        .await
        .is_err());
    evidence(
        "semantic-edit",
        &json!({"stale":stale,"current_authored":changed.graph}),
    );
    service.workflow_graph_shutdown_validation_tasks().await;
}

#[tokio::test]
async fn closing_session_during_fact_lookup_cannot_publish_or_enable_submission() {
    let facts = Facts::new(vec![0]);
    let service = Arc::new(
        WorkflowService::with_ephemeral_attribution_store()
            .unwrap()
            .with_inference_interface_facts_provider(facts.clone()),
    );
    let edit = open(&service, authored().await).await;
    let task_service = service.clone();
    let task_edit = edit.clone();
    let task = tokio::spawn(async move {
        task_service
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: task_edit.session_id,
                    graph_revision: task_edit.graph_revision.parse().unwrap(),
                },
            )
            .await
    });
    facts.wait_entered().await;
    service
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: edit.session_id.clone(),
        })
        .await
        .unwrap();
    assert!(tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(service
        .publish_graph_session_executable_validation_snapshot(publish_request(&edit, None))
        .await
        .is_err());
    assert!(service
        .workflow_graph_current_validation_summary(WorkflowGraphCurrentValidationSummaryRequest {
            graph_session_id: edit.session_id,
            graph_revision: edit.graph_revision.parse().unwrap()
        })
        .await
        .is_err());
}

struct DiskHost {
    root: PathBuf,
    loads: AtomicUsize,
}
#[async_trait]
impl WorkflowHost for DiskHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        capabilities::default_workflow_roots(&self.root)
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!("refused group must not start runtime")
    }
    async fn load_session_runtime(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        panic!("refused group must not acquire KeepAlive")
    }
}

#[tokio::test]
async fn actual_saved_unsupported_group_refuses_before_keepalive_acquisition() {
    let mut graph = authored().await;
    group(&mut graph)["nodes"][0]["node_type"] = json!("llm-inference");
    let root = std::env::temp_dir().join(format!(
        "pantograph-group-preflight-acquire-{}",
        uuid::Uuid::new_v4()
    ));
    let path = FileSystemWorkflowGraphStore::new(&root)
        .save_workflow("unsupported".into(), graph)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let host = DiskHost {
        root: root.clone(),
        loads: AtomicUsize::new(0),
    };
    let service = WorkflowService::new();
    assert!(service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "unsupported".into(),
                usage_profile: None,
                keep_alive: true
            }
        )
        .await
        .is_err());
    assert_eq!(host.loads.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    evidence(
        "keepalive-refusal",
        &json!({"runtime_load_calls":0,"saved_bytes_unchanged":true}),
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn scheduler_projection_refuses_superseded_and_canceled_completed_generation() {
    let sessions = GraphSessionStore::new();
    let edit = sessions.create_session(authored().await, None).await;
    let request = WorkflowGraphCurrentValidationRefreshRequest {
        graph_session_id: edit.session_id.clone(),
        graph_revision: edit.graph_revision.parse().unwrap(),
    };
    let previous = sessions
        .refresh_current_validation_summary(request.clone())
        .await
        .unwrap();
    let previous_id = previous.summary.validation_session_id.unwrap();
    let current_id = sessions
        .start_current_validation_task(request)
        .await
        .unwrap();
    let key = WorkflowGraphCurrentValidationSummaryRequest {
        graph_session_id: edit.session_id.clone(),
        graph_revision: edit.graph_revision.parse().unwrap(),
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if sessions
                .current_validation_summary(key.clone())
                .await
                .unwrap()
                .submit_gate
                .allowed
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(sessions
        .scheduler_inference_task_projections_for_session(&edit.session_id, Some(previous_id))
        .await
        .is_err());
    assert!(sessions
        .scheduler_inference_task_projections_for_session(
            &edit.session_id,
            Some(current_id.clone())
        )
        .await
        .is_ok());
    sessions.shutdown_validation_tasks().await;
    let canceled = sessions.current_validation_summary(key).await.unwrap();
    assert!(!canceled.submit_gate.allowed);
    assert_eq!(
        canceled.diagnostics[0].code,
        Code::ValidationSessionCancelled
    );
    assert!(sessions
        .scheduler_inference_task_projections_for_session(&edit.session_id, Some(current_id))
        .await
        .is_err());
    assert!(sessions
        .scheduler_inference_task_projections_for_session(&edit.session_id, None)
        .await
        .is_err());
    evidence(
        "canceled-scheduler-projections",
        &json!({"summary":canceled,"projection_refused":true}),
    );
}

#[tokio::test]
async fn forged_positive_publication_cannot_store_unsupported_authored_graph() {
    let sessions = GraphSessionStore::new();
    let graph = authored().await;
    let edit = sessions.create_session(graph.clone(), None).await;
    let mut publication = sessions
        .publish_inference_validation_session(
            &edit.session_id,
            "validation.forged".parse().unwrap(),
        )
        .await
        .unwrap();
    assert!(publication.validation_session.summary.executable);
    let mut invalid = graph;
    group(&mut invalid)["nodes"][0]["node_type"] = json!("llm-inference");
    publication.validation_session.graph_revision = invalid.compute_fingerprint().parse().unwrap();
    publication.validation_session.events.clear();
    publication.validation_session.latest_sequence = 0;
    publication.validation_session.validate().unwrap();
    let service = WorkflowService::with_ephemeral_attribution_store().unwrap();
    let result = service.publish_workflow_executable_validation_snapshot(
        WorkflowExecutableValidationSnapshotPublishRequest {
            workflow_id: "forged".into(),
            workflow_semantic_version: "0.1.0".into(),
            graph: invalid,
            validation_publication: publication,
            validation_snapshot_id: None,
        },
    );
    assert!(result.is_err());
    evidence(
        "forged-publication-refused",
        &json!({"positive_validation_was_structurally_valid":true,"snapshot_refused":true}),
    );
}

struct RebalanceHost {
    root: PathBuf,
    replacement: WorkflowGraph,
    loads: AtomicUsize,
    unloads: AtomicUsize,
}
#[async_trait]
impl WorkflowHost for RebalanceHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        capabilities::default_workflow_roots(&self.root)
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!("capacity fixture does not execute workflows")
    }
    async fn load_session_runtime(
        &self,
        _: &str,
        workflow_id: &str,
        _: Option<&str>,
        _: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        assert_eq!(
            workflow_id, "resident",
            "changed target must be refused before target load"
        );
        self.loads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    async fn unload_session_runtime(
        &self,
        _: &str,
        _: &str,
        reason: WorkflowExecutionSessionUnloadReason,
    ) -> Result<(), WorkflowServiceError> {
        if reason == WorkflowExecutionSessionUnloadReason::CapacityRebalance {
            FileSystemWorkflowGraphStore::new(&self.root)
                .save_workflow("target".into(), self.replacement.clone())?;
            self.unloads.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

#[tokio::test]
async fn capacity_rebalance_rechecks_actual_graph_before_loading_changed_target() {
    let valid = authored().await;
    let mut invalid = valid.clone();
    group(&mut invalid)["nodes"][0]["node_type"] = json!("llm-inference");
    let root = std::env::temp_dir().join(format!(
        "pantograph-group-preflight-rebalance-{}",
        uuid::Uuid::new_v4()
    ));
    let store = FileSystemWorkflowGraphStore::new(&root);
    store
        .save_workflow("resident".into(), valid.clone())
        .unwrap();
    let target_path = store.save_workflow("target".into(), valid).unwrap();
    let host = RebalanceHost {
        root: root.clone(),
        replacement: invalid.clone(),
        loads: AtomicUsize::new(0),
        unloads: AtomicUsize::new(0),
    };
    let service = WorkflowService::with_capacity_limits(4, 1);
    service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "resident".into(),
                usage_profile: None,
                keep_alive: true,
            },
        )
        .await
        .unwrap();
    assert!(service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "target".into(),
                usage_profile: None,
                keep_alive: true
            }
        )
        .await
        .is_err());
    assert_eq!(host.loads.load(Ordering::SeqCst), 1);
    assert_eq!(host.unloads.load(Ordering::SeqCst), 1);
    let mut reopened = store.load_workflow(target_path).unwrap().graph;
    // The save API recomputes derived metadata; the authored semantic bytes matter.
    assert_eq!(
        reopened.compute_fingerprint(),
        invalid.compute_fingerprint()
    );
    assert_eq!(group(&mut reopened), group(&mut invalid));
    evidence(
        "capacity-boundary-refusal",
        &json!({"resident_load_calls":1,"capacity_unload_calls":1,"changed_target_load_calls":0}),
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn manual_publication_cannot_reactivate_canceled_or_superseded_generation_id() {
    let sessions = GraphSessionStore::new();
    let edit = sessions.create_session(authored().await, None).await;
    let first = sessions
        .publish_inference_validation_session(
            &edit.session_id,
            "validation.manual-first".parse().unwrap(),
        )
        .await
        .unwrap();
    let second = sessions
        .publish_inference_validation_session(
            &edit.session_id,
            "validation.manual-second".parse().unwrap(),
        )
        .await
        .unwrap();
    assert!(sessions
        .record_inference_validation_session(&edit.session_id, first.validation_session)
        .await
        .is_err());
    let key = WorkflowGraphCurrentValidationSummaryRequest {
        graph_session_id: edit.session_id.clone(),
        graph_revision: edit.graph_revision.parse().unwrap(),
    };
    let before = sessions
        .current_validation_summary(key.clone())
        .await
        .unwrap();
    assert!(before.submit_gate.allowed);
    assert_eq!(
        before.validation_session_id.as_ref(),
        Some(&second.validation_session.validation_session_id)
    );
    sessions.shutdown_validation_tasks().await;
    assert!(sessions
        .record_inference_validation_session(&edit.session_id, second.validation_session)
        .await
        .is_err());
    let after = sessions.current_validation_summary(key).await.unwrap();
    assert!(!after.submit_gate.allowed);
    evidence(
        "manual-generation-replay",
        &json!({"before_shutdown":before,"after_refused_replay":after}),
    );
}

#[tokio::test]
async fn valid_group_facts_cannot_make_missing_root_inference_executable() {
    let mut graph = authored().await;
    graph
        .nodes
        .push(node("missing-root-model", "llm-inference"));
    let service = WorkflowService::with_ephemeral_attribution_store().unwrap();
    let edit = open(&service, graph).await;
    let result = refresh(&service, &edit).await;
    assert!(result
        .summary
        .group_preflight
        .as_ref()
        .unwrap()
        .failures
        .is_empty());
    assert!(!result.summary.submit_gate.allowed);
    assert!(!result.summary.summary.as_ref().unwrap().executable);
    assert!(service
        .publish_graph_session_executable_validation_snapshot(publish_request(
            &edit,
            result.summary.validation_session_id.clone()
        ))
        .await
        .is_err());
    evidence(
        "group-success-inference-blocked",
        &serde_json::to_value(result.summary).unwrap(),
    );
}

#[tokio::test]
async fn ephemeral_metadata_does_not_authorize_unsupported_group_retention_or_run() {
    let mut graph = authored().await;
    group(&mut graph)["nodes"][0]["node_type"] = json!("llm-inference");
    let root = std::env::temp_dir().join(format!(
        "pantograph-group-preflight-ephemeral-{}",
        uuid::Uuid::new_v4()
    ));
    let path = FileSystemWorkflowGraphStore::new(&root)
        .save_workflow("unsupported".into(), graph)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let host = DiskHost {
        root: root.clone(),
        loads: AtomicUsize::new(0),
    };
    let service = WorkflowService::new();
    let metadata = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "unsupported".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .unwrap();
    assert_eq!(host.loads.load(Ordering::SeqCst), 0);
    assert!(service
        .workflow_set_execution_session_keep_alive(
            &host,
            WorkflowExecutionSessionKeepAliveRequest {
                session_id: metadata.session_id.clone(),
                keep_alive: true
            }
        )
        .await
        .is_err());
    assert!(service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: metadata.session_id.clone(),
                workflow_semantic_version: "0.1.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "source".into(),
                    port_id: "text".into(),
                    value: json!("preflight")
                }],
                output_targets: None,
                override_selection: None,
                timeout_ms: None,
                priority: None,
            }
        )
        .await
        .is_err());
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: metadata.session_id.clone(),
        })
        .await
        .unwrap();
    assert!(!status.session.keep_alive);
    assert_eq!(
        status.session.state,
        WorkflowExecutionSessionState::IdleUnloaded
    );
    assert!(service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: metadata.session_id
        })
        .await
        .unwrap()
        .items
        .is_empty());
    assert_eq!(host.loads.load(Ordering::SeqCst), 0);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    evidence(
        "ephemeral-metadata-refusal",
        &json!({"metadata_created":true,"status_after_refused_retention_and_run":status,"runtime_load_calls":0,"queue_items":0,"saved_bytes_unchanged":true}),
    );
    std::fs::remove_dir_all(root).unwrap();
}
