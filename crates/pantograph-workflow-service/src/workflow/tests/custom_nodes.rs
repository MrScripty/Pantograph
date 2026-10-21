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
