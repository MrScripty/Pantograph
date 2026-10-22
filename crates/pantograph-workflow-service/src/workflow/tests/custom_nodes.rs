use super::*;
use crate::custom_nodes::tests::{admitted, Behavior, Executor};
use crate::custom_nodes::{CustomNodeCatalog, CustomNodeIdentity, CUSTOM_NODE_IDENTITY_FIELD};
use crate::graph::{FileSystemWorkflowGraphStore, WorkflowGraphStore};
use crate::{GraphEdge, GraphNode, Position};

struct SavedGraphHost {
    root: PathBuf,
}
#[async_trait]
impl WorkflowHost for SavedGraphHost {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        vec![self.root.join(".pantograph").join("workflows")]
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!(
            "scheduler-managed custom execution must never delegate to direct host graph execution"
        )
    }
}
fn catalog(executor: Arc<Executor>) -> Arc<CustomNodeCatalog> {
    Arc::new(CustomNodeCatalog::try_new(vec![admitted(executor)]).unwrap())
}
fn custom_graph(identity: &CustomNodeIdentity) -> WorkflowGraph {
    let nodes = [
        ("source", "text-input", serde_json::json!({})),
        (
            "custom",
            "custom-uppercase",
            serde_json::json!({CUSTOM_NODE_IDENTITY_FIELD: identity}),
        ),
        ("sink", "text-output", serde_json::json!({})),
    ]
    .map(|(id, node_type, data)| GraphNode {
        id: id.into(),
        node_type: node_type.into(),
        position: Position { x: 0.0, y: 0.0 },
        data,
    })
    .to_vec();
    let registry = crate::NodeRegistry::new();
    let nodes = nodes
        .into_iter()
        .map(|mut node| {
            if node.node_type != "custom-uppercase" {
                node.data["definition"] =
                    serde_json::to_value(registry.get_definition(&node.node_type).unwrap())
                        .unwrap();
            }
            node
        })
        .collect();
    WorkflowGraph {
        nodes,
        edges: vec![
            GraphEdge {
                id: "source-custom".into(),
                source: "source".into(),
                source_handle: "text".into(),
                target: "custom".into(),
                target_handle: "input".into(),
            },
            GraphEdge {
                id: "custom-sink".into(),
                source: "custom".into(),
                source_handle: "output".into(),
                target: "sink".into(),
                target_handle: "text".into(),
            },
        ],
        derived_graph: None,
    }
}
fn run_request(session_id: &str) -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: session_id.into(),
        workflow_semantic_version: "1.0.0".into(),
        inputs: vec![WorkflowPortBinding {
            node_id: "source".into(),
            port_id: "text".into(),
            value: serde_json::json!("Mixed case"),
        }],
        output_targets: Some(vec![WorkflowOutputTarget {
            node_id: "sink".into(),
            port_id: "text".into(),
        }]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    }
}
async fn session(service: &WorkflowService, host: &SavedGraphHost) -> String {
    service
        .create_workflow_execution_session(
            host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "custom-workflow".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .unwrap()
        .session_id
}
fn save(root: &std::path::Path, catalog: &CustomNodeCatalog) -> String {
    let service = WorkflowService::new();
    service
        .workflow_graph_save(
            &FileSystemWorkflowGraphStore::new(root),
            crate::WorkflowGraphSaveRequest {
                name: "custom-workflow".into(),
                graph: custom_graph(catalog.get("custom-uppercase").unwrap().identity()),
            },
        )
        .unwrap()
        .path
}

#[tokio::test]
async fn custom_nodes_saved_reopened_graph_executes_through_canonical_scheduler() {
    let root = tempfile::tempdir().unwrap();
    let initial = catalog(Arc::new(Executor::new(Behavior::Uppercase)));
    let path = save(root.path(), &initial);
    drop(initial);
    // A new service, graph-store object and callback instance reopen the file.
    let executor = Arc::new(Executor::new(Behavior::Uppercase));
    let catalog = catalog(executor.clone());
    let service = WorkflowService::with_trusted_custom_nodes(catalog)
        .with_attribution_store(SqliteAttributionStore::open_in_memory().unwrap())
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().unwrap());
    let reopened = service
        .workflow_graph_load(
            &FileSystemWorkflowGraphStore::new(root.path()),
            crate::WorkflowGraphLoadRequest { path: path.clone() },
        )
        .unwrap();
    assert_eq!(
        reopened.graph.nodes[1].data[CUSTOM_NODE_IDENTITY_FIELD]["implementation_digest"],
        serde_json::json!(
            admitted(Arc::new(Executor::new(Behavior::Uppercase)))
                .identity()
                .implementation_digest
        )
    );
    let inspected = service
        .workflow_graph_inspect(
            &FileSystemWorkflowGraphStore::new(root.path()),
            crate::WorkflowGraphInspectionRequest {
                path,
                selected_node_id: None,
            },
        )
        .unwrap();
    assert!(!inspected.diagnostics.iter().any(|d| d.blocking_submission));
    let version = service
        .resolve_workflow_graph_version("custom-workflow", "1.0.0", &reopened.graph)
        .unwrap();
    service.store_workflow_executable_validation_snapshot(WorkflowExecutableValidationSnapshotRecord {
        schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId::generate(),
        workflow_id: version.workflow_id.clone(), workflow_version_id: version.workflow_version_id.clone(),
        workflow_semantic_version: version.semantic_version.clone(), workflow_execution_fingerprint: version.execution_fingerprint.clone(),
        descriptor_contract_version: pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: pantograph_inference_interface_contracts::WorkflowGraphRevision::parse(reopened.graph.compute_fingerprint()).unwrap(),
        validation_session_id: pantograph_inference_interface_contracts::DraftGraphValidationSessionId::parse("validation.custom").unwrap(),
        validation_summary: pantograph_inference_interface_contracts::DraftGraphValidationSummary { status: pantograph_inference_interface_contracts::DraftGraphValidationStatus::Executable, executable: true, enqueue_disabled_reasons: vec![], diagnostics_count: 0, blocking_diagnostics_count: 0 },
        nodes: vec![],
    }).unwrap();
    let host = SavedGraphHost {
        root: root.path().to_path_buf(),
    };
    let session = session(&service, &host).await;
    let response = service
        .run_workflow_execution_session(&host, run_request(&session))
        .await
        .unwrap();
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].value, serde_json::json!("MIXED CASE"));
    let invocation = executor.invocation.lock().clone().unwrap();
    assert_eq!(invocation.task_id, "custom");
    assert!(!invocation.workflow_run_id.is_empty());
    assert!(!invocation.attempt_id.is_empty());
    assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
    let run = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: invocation.workflow_run_id,
        })
        .unwrap()
        .run_graph
        .unwrap();
    assert!(!run
        .graph_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.blocking_submission));
    assert_eq!(
        run.graph
            .nodes
            .iter()
            .find(|node| node.id == "custom")
            .unwrap()
            .data[CUSTOM_NODE_IDENTITY_FIELD],
        reopened.graph.nodes[1].data[CUSTOM_NODE_IDENTITY_FIELD]
    );
}

#[tokio::test]
async fn custom_nodes_missing_or_changed_registration_cannot_execute_saved_graph() {
    let root = tempfile::tempdir().unwrap();
    let c = catalog(Arc::new(Executor::new(Behavior::Uppercase)));
    let path = save(root.path(), &c);
    let host = SavedGraphHost {
        root: root.path().to_path_buf(),
    };
    let missing = WorkflowService::new();
    let session_id = session(&missing, &host).await;
    let error = missing
        .run_workflow_execution_session(&host, run_request(&session_id))
        .await
        .unwrap_err();
    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    let Some(WorkflowErrorDetails::Graph(details)) = error.details() else {
        panic!("expected structured stale-graph error: {error:?}")
    };
    assert!(details
        .graph_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code
            == crate::graph::WorkflowGraphDiagnosticCode::UnknownNodeType
            && diagnostic.node_id.as_deref() == Some("custom")));
    let executor = Arc::new(Executor::new(Behavior::Uppercase));
    let changed = crate::custom_nodes::TrustedCustomNode::admit(
        crate::custom_nodes::tests::contract("custom-uppercase"),
        crate::custom_nodes::tests::digest("different implementation"),
        crate::custom_nodes::tests::digest("trusted rust host v1"),
        executor.clone(),
    )
    .unwrap();
    let changed = WorkflowService::with_trusted_custom_nodes(Arc::new(
        CustomNodeCatalog::try_new(vec![changed]).unwrap(),
    ));
    let session_id = session(&changed, &host).await;
    let error = changed
        .run_workflow_execution_session(&host, run_request(&session_id))
        .await
        .unwrap_err();
    assert_eq!(error.code(), WorkflowErrorCode::InvalidRequest);
    let Some(WorkflowErrorDetails::Graph(details)) = error.details() else {
        panic!("expected structured stale-graph error: {error:?}")
    };
    assert!(details.graph_diagnostics.iter().any(|diagnostic| diagnostic.code == crate::graph::WorkflowGraphDiagnosticCode::InvalidDynamicDefinition && diagnostic.node_id.as_deref() == Some("custom") && diagnostic.message == "node 'custom' dynamic contract definition is invalid: custom-node identity does not match admitted implementation"));
    assert_eq!(executor.calls.load(Ordering::SeqCst), 0);
    let mut graph = FileSystemWorkflowGraphStore::new(root.path())
        .load_workflow(path)
        .unwrap()
        .graph;
    graph.nodes[1].data["definition"] = serde_json::json!({"inputs":[]});
    let configured = WorkflowService::with_trusted_custom_nodes(c);
    assert!(configured
        .node_registry
        .executable_topology(&graph)
        .is_err());
}

#[tokio::test]
async fn custom_nodes_active_scheduler_cancel_drops_callback_and_publishes_no_success() {
    let root = tempfile::tempdir().unwrap();
    let executor = Arc::new(Executor::new(Behavior::Pending));
    let c = catalog(executor.clone());
    save(root.path(), &c);
    let service = WorkflowService::with_trusted_custom_nodes(c);
    let host = SavedGraphHost {
        root: root.path().to_path_buf(),
    };
    let session_id = session(&service, &host).await;
    let (result, ()) = tokio::join!(
        service.run_workflow_execution_session(&host, run_request(&session_id)),
        async {
            executor.entered.notified().await;
            let i = executor.invocation.lock().clone().unwrap();
            let response = service
                .workflow_cancel_active_execution_session_task(
                    WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id: session_id.clone(),
                        workflow_run_id: i.workflow_run_id,
                        task_id: i.task_id,
                        reason: Some("test cancellation".into()),
                    },
                )
                .await
                .unwrap();
            assert_eq!(response.active_attempt_id, i.attempt_id);
        }
    );
    assert!(matches!(result, Err(WorkflowServiceError::Cancelled(_))));
    assert!(executor.dropped.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn custom_nodes_workflow_timeout_drops_callback_and_cleans_attempt() {
    let root = tempfile::tempdir().unwrap();
    let executor = Arc::new(Executor::new(Behavior::Pending));
    let c = catalog(executor.clone());
    save(root.path(), &c);
    let service = WorkflowService::with_trusted_custom_nodes(c);
    let host = SavedGraphHost {
        root: root.path().to_path_buf(),
    };
    let session_id = session(&service, &host).await;
    let mut request = run_request(&session_id);
    request.timeout_ms = Some(10);
    assert!(matches!(
        service.run_workflow_execution_session(&host, request).await,
        Err(WorkflowServiceError::RuntimeTimeout(_))
    ));
    assert!(executor.dropped.load(Ordering::SeqCst));
    let i = executor.invocation.lock().clone().unwrap();
    assert!(service
        .node_registry
        .custom_nodes
        .cancel(&i.workflow_run_id, &i.task_id, &i.attempt_id)
        .is_err());
}

#[tokio::test]
async fn custom_nodes_callback_failures_are_terminal_without_publishing_outputs() {
    for (behavior, expected) in [
        (Behavior::Fail, "custom-node callback failed"),
        (Behavior::Panic, "custom-node callback panicked"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let executor = Arc::new(Executor::new(behavior));
        let c = catalog(executor.clone());
        save(root.path(), &c);
        let service = WorkflowService::with_trusted_custom_nodes(c);
        let host = SavedGraphHost {
            root: root.path().to_path_buf(),
        };
        let session_id = session(&service, &host).await;
        let error = service
            .run_workflow_execution_session(&host, run_request(&session_id))
            .await
            .unwrap_err();
        assert!(
            matches!(error, WorkflowServiceError::InvalidRequest(ref message) if message == &format!("scheduler non-runtime task execution failed: {expected}"))
        );
        assert!(!error.to_string().contains("sensitive"));
        assert_eq!(executor.calls.load(Ordering::SeqCst), 1);
        assert!(executor.dropped.load(Ordering::SeqCst));
        let i = executor.invocation.lock().clone().unwrap();
        assert!(service
            .node_registry
            .custom_nodes
            .cancel(&i.workflow_run_id, &i.task_id, &i.attempt_id)
            .is_err());
    }
}

async fn authored_draft(service: &WorkflowService) -> crate::WorkflowGraphEditSessionGraphResponse {
    let identity = service.custom_node_authoring_catalog().nodes[0]
        .identity
        .clone();
    service
        .create_custom_node_draft(crate::custom_nodes::authoring::CustomNodeDraftRequest {
            identity,
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn custom_node_authoring_registry_survives_provider_builders_and_undo_redo() {
    for mode in 0..3 {
        let service = WorkflowService::with_trusted_custom_nodes(catalog(Arc::new(Executor::new(
            Behavior::Uppercase,
        ))));
        let service = match mode {
            1 => service.with_inference_interface_facts_provider(Arc::new(crate::graph::UnavailableInferenceInterfaceFactsProvider)),
            2 => service.with_graph_session_fact_providers(Arc::new(crate::graph::UnavailableInferenceInterfaceFactsProvider), Arc::new(pantograph_dependency_environment_service::NotImplementedDependencyEnvironmentProvider)),
            _ => service,
        };
        let draft = authored_draft(&service).await;
        assert!(
            draft
                .graph_diagnostics
                .iter()
                .all(|d| !d.blocking_submission),
            "{:?}",
            draft.graph_diagnostics
        );
        let pin = draft.graph.nodes[0].data[CUSTOM_NODE_IDENTITY_FIELD].clone();
        let updated = service
            .workflow_graph_update_node_data(crate::WorkflowGraphUpdateNodeDataRequest {
                session_id: draft.session_id.clone(),
                node_id: "input-0".into(),
                data: serde_json::json!({"text": "authored"}),
            })
            .await
            .unwrap();
        assert_eq!(
            updated.graph.find_node("input-0").unwrap().data["text"],
            "authored"
        );
        let request = crate::WorkflowGraphEditSessionGraphRequest {
            session_id: draft.session_id,
        };
        let undo = service.workflow_graph_undo(request.clone()).await.unwrap();
        let redo = service.workflow_graph_redo(request).await.unwrap();
        for snapshot in [undo, redo] {
            assert_eq!(
                snapshot.graph.find_node("custom").unwrap().data[CUSTOM_NODE_IDENTITY_FIELD],
                pin
            );
            assert!(snapshot
                .graph_diagnostics
                .iter()
                .all(|d| !d.blocking_submission));
        }
    }
}

#[tokio::test]
async fn custom_node_authoring_refuses_stale_selection_and_identity_patches_without_mutation() {
    let service = WorkflowService::with_trusted_custom_nodes(catalog(Arc::new(Executor::new(
        Behavior::Uppercase,
    ))));
    let entry = service.custom_node_authoring_catalog().nodes[0].clone();
    let mut stale = entry.identity.clone();
    stale.environment_digest = crate::custom_nodes::tests::digest("changed environment");
    let error = service
        .create_custom_node_draft(crate::custom_nodes::authoring::CustomNodeDraftRequest {
            identity: stale.clone(),
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("identity does not match"));
    assert!(
        serde_json::from_value::<crate::custom_nodes::authoring::CustomNodeDraftRequest>(
            serde_json::json!({"identity": entry.identity, "definition": {}})
        )
        .is_err()
    );
    let draft = authored_draft(&service).await;
    for patch in [
        serde_json::json!({CUSTOM_NODE_IDENTITY_FIELD: stale}),
        serde_json::json!({"definition": {}}),
        serde_json::json!({CUSTOM_NODE_IDENTITY_FIELD: null}),
    ] {
        let error = service
            .workflow_graph_update_node_data(crate::WorkflowGraphUpdateNodeDataRequest {
                session_id: draft.session_id.clone(),
                node_id: "custom".into(),
                data: patch,
            })
            .await
            .unwrap_err();
        assert!(error.to_string().contains("cannot be patched"));
        let after = service
            .workflow_graph_get_edit_session_graph(crate::WorkflowGraphEditSessionGraphRequest {
                session_id: draft.session_id.clone(),
            })
            .await
            .unwrap();
        assert_eq!(after.graph_revision, draft.graph_revision);
        assert_eq!(after.graph, draft.graph);
    }
    let mut unpinned = draft.graph.find_node("custom").unwrap().clone();
    unpinned.id = "bad".into();
    unpinned.data = serde_json::json!({});
    assert!(service
        .workflow_graph_add_node(crate::WorkflowGraphAddNodeRequest {
            session_id: draft.session_id,
            node: unpinned
        })
        .await
        .unwrap_err()
        .to_string()
        .contains("identity does not match"));
}

#[tokio::test]
async fn custom_node_authoring_candidates_and_commit_share_exact_registry() {
    let service = WorkflowService::with_trusted_custom_nodes(catalog(Arc::new(Executor::new(
        Behavior::Uppercase,
    ))));
    let draft = authored_draft(&service).await;
    let removed = service
        .workflow_graph_remove_edge(crate::WorkflowGraphRemoveEdgeRequest {
            session_id: draft.session_id.clone(),
            edge_id: "edge-input-0".into(),
        })
        .await
        .unwrap();
    let source = crate::ConnectionAnchor {
        node_id: "input-0".into(),
        port_id: "text".into(),
    };
    let candidates = service
        .workflow_graph_get_connection_candidates(
            crate::WorkflowGraphGetConnectionCandidatesRequest {
                session_id: draft.session_id.clone(),
                source_anchor: source.clone(),
                graph_revision: Some(removed.graph_revision.clone()),
            },
        )
        .await
        .unwrap();
    assert!(candidates
        .compatible_nodes
        .iter()
        .any(|node| node.node_id == "custom"));
    assert!(!candidates
        .insertable_node_types
        .iter()
        .any(|node| node.node_type == "custom-uppercase"));
    let rejected = service
        .workflow_graph_preview_node_insert_on_edge(
            crate::WorkflowGraphPreviewNodeInsertOnEdgeRequest {
                session_id: draft.session_id.clone(),
                edge_id: "edge-output-0".into(),
                node_type: "custom-uppercase".into(),
                graph_revision: removed.graph_revision.clone(),
            },
        )
        .await
        .unwrap();
    assert!(!rejected.accepted);
    assert!(rejected
        .rejection
        .unwrap()
        .message
        .contains("insertion by name is unsupported"));
    let committed = service
        .workflow_graph_connect(crate::WorkflowGraphConnectRequest {
            session_id: draft.session_id.clone(),
            source_anchor: source,
            target_anchor: crate::ConnectionAnchor {
                node_id: "custom".into(),
                port_id: "input".into(),
            },
            graph_revision: removed.graph_revision,
        })
        .await
        .unwrap();
    assert!(committed.accepted);
    let snapshot = service
        .workflow_graph_get_edit_session_graph(crate::WorkflowGraphEditSessionGraphRequest {
            session_id: draft.session_id,
        })
        .await
        .unwrap();
    assert!(snapshot
        .graph_diagnostics
        .iter()
        .all(|d| !d.blocking_submission));
}

#[tokio::test]
async fn custom_node_authoring_saved_reopen_preserves_pin_and_scheduler_execution() {
    let root = tempfile::tempdir().unwrap();
    let catalog = catalog(Arc::new(Executor::new(Behavior::Uppercase)));
    let service = WorkflowService::with_trusted_custom_nodes(catalog.clone());
    let draft = authored_draft(&service).await;
    let store = FileSystemWorkflowGraphStore::new(root.path());
    let saved = service
        .workflow_graph_save(
            &store,
            crate::WorkflowGraphSaveRequest {
                name: "custom-workflow".into(),
                graph: draft.graph.clone(),
            },
        )
        .unwrap();
    let reopened = WorkflowService::with_trusted_custom_nodes(catalog);
    let graph = reopened
        .workflow_graph_load(
            &store,
            crate::WorkflowGraphLoadRequest {
                path: saved.path.clone(),
            },
        )
        .unwrap()
        .graph;
    assert_eq!(
        graph.find_node("custom").unwrap().data[CUSTOM_NODE_IDENTITY_FIELD],
        draft.graph.find_node("custom").unwrap().data[CUSTOM_NODE_IDENTITY_FIELD]
    );
    for (service, blocking) in [(&reopened, false), (&WorkflowService::new(), true)] {
        let edit = service
            .workflow_graph_create_edit_session(crate::WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
                workflow_id: Some(saved.path.clone()),
            })
            .await
            .unwrap();
        let snapshot = service
            .workflow_graph_get_edit_session_graph(crate::WorkflowGraphEditSessionGraphRequest {
                session_id: edit.session_id,
            })
            .await
            .unwrap();
        assert_eq!(
            snapshot.graph.find_node("custom").unwrap().data,
            graph.find_node("custom").unwrap().data
        );
        assert_eq!(
            snapshot
                .graph_diagnostics
                .iter()
                .any(|d| d.blocking_submission),
            blocking
        );
    }
    let host = SavedGraphHost {
        root: root.path().into(),
    };
    let session_id = session(&reopened, &host).await;
    let mut request = run_request(&session_id);
    request.inputs[0].node_id = "input-0".into();
    request.output_targets.as_mut().unwrap()[0].node_id = "output-0".into();
    let result = reopened
        .run_workflow_execution_session(&host, request)
        .await
        .unwrap();
    assert_eq!(result.outputs[0].value, "MIXED CASE");
}

#[tokio::test]
async fn custom_node_edit_sessions_preserve_integrated_loop_admission() {
    use crate::tool_loops::tests::{catalog as loop_catalog, Executor as LoopExecutor, Mode};
    use crate::tool_loops::TOOL_LOOP_IDENTITY_FIELD;

    let controller = LoopExecutor::new(Mode::RoundTrip);
    let tool = LoopExecutor::new(Mode::Uppercase);
    let catalog = loop_catalog(controller.clone(), tool.clone(), 3);
    let mut graph = custom_graph(catalog.get("loop-controller").unwrap().identity());
    let node = graph.find_node_mut("custom").unwrap();
    node.node_type = "tool-loop".into();
    node.data = serde_json::json!({TOOL_LOOP_IDENTITY_FIELD: catalog.tool_loop_identity("pure-loop").unwrap()});
    graph.edges[0].target_handle = "prompt".into();
    graph.edges[1].source_handle = "response".into();
    let root = tempfile::tempdir().unwrap();
    let store = FileSystemWorkflowGraphStore::new(root.path());
    let service = WorkflowService::with_trusted_custom_nodes(catalog.clone());
    let saved = service
        .workflow_graph_save(
            &store,
            crate::WorkflowGraphSaveRequest {
                name: "integrated-loop".into(),
                graph,
            },
        )
        .unwrap();
    let graph = service
        .workflow_graph_load(
            &store,
            crate::WorkflowGraphLoadRequest {
                path: saved.path.clone(),
            },
        )
        .unwrap()
        .graph;
    let pin = graph.find_node("custom").unwrap().data[TOOL_LOOP_IDENTITY_FIELD].clone();
    let changed = loop_catalog(controller.clone(), tool.clone(), 4);
    for (service, blocking) in [
        (service, false),
        (WorkflowService::with_trusted_custom_nodes(changed), true),
        (WorkflowService::new(), true),
    ] {
        let edit = service
            .workflow_graph_create_edit_session(crate::WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
                workflow_id: Some(saved.path.clone()),
            })
            .await
            .unwrap();
        let request = crate::WorkflowGraphEditSessionGraphRequest {
            session_id: edit.session_id.clone(),
        };
        let snapshot = service
            .workflow_graph_get_edit_session_graph(request.clone())
            .await
            .unwrap();
        assert_eq!(
            snapshot.graph.find_node("custom").unwrap().data[TOOL_LOOP_IDENTITY_FIELD],
            pin
        );
        assert_eq!(
            snapshot
                .graph_diagnostics
                .iter()
                .any(|d| d.blocking_submission),
            blocking,
            "{:?}",
            snapshot.graph_diagnostics
        );
        let mut invalid = snapshot.graph.find_node("custom").unwrap().clone();
        invalid.id = "invalid-new-loop".into();
        invalid.data[TOOL_LOOP_IDENTITY_FIELD]["max_turns"] = serde_json::json!(5);
        assert!(service
            .workflow_graph_add_node(crate::WorkflowGraphAddNodeRequest {
                session_id: edit.session_id,
                node: invalid,
            })
            .await
            .is_err());
        let after = service
            .workflow_graph_get_edit_session_graph(request)
            .await
            .unwrap();
        assert_eq!(after.graph_revision, snapshot.graph_revision);
        assert_eq!(after.graph, snapshot.graph);
    }
    assert!(controller.calls.lock().is_empty());
    assert!(tool.calls.lock().is_empty());
}
