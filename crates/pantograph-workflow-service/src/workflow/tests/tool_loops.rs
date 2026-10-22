use super::*;
use crate::custom_nodes::CustomNodeCatalog;
use crate::graph::{FileSystemWorkflowGraphStore, WorkflowGraphStore};
use crate::tool_loops::tests::{catalog, Executor, Mode};
use crate::tool_loops::{ToolLoopIdentity, TOOL_LOOP_IDENTITY_FIELD};
use crate::{GraphEdge, GraphNode, Position, WorkflowId, WorkflowRunId};

struct Host {
    root: PathBuf,
}
#[async_trait]
impl WorkflowHost for Host {
    fn workflow_roots(&self) -> Vec<PathBuf> {
        vec![self.root.join(".pantograph/workflows")]
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!("tool-loop must use scheduler ownership, never direct host execution")
    }
}

fn graph(identity: &ToolLoopIdentity) -> WorkflowGraph {
    WorkflowGraph {
        nodes: [
            ("source", "text-input", serde_json::json!({})),
            (
                "loop",
                "tool-loop",
                serde_json::json!({TOOL_LOOP_IDENTITY_FIELD: identity}),
            ),
            ("sink", "text-output", serde_json::json!({})),
        ]
        .into_iter()
        .map(|(id, node_type, mut data)| {
            if node_type != "tool-loop" {
                data["definition"] = serde_json::to_value(
                    crate::NodeRegistry::new()
                        .get_definition(node_type)
                        .unwrap(),
                )
                .unwrap();
            }
            GraphNode {
                id: id.into(),
                node_type: node_type.into(),
                position: Position { x: 0.0, y: 0.0 },
                data,
            }
        })
        .collect(),
        edges: [
            ("source", "text", "loop", "prompt"),
            ("loop", "response", "sink", "text"),
        ]
        .into_iter()
        .enumerate()
        .map(
            |(i, (source, source_handle, target, target_handle))| GraphEdge {
                id: format!("edge-{i}"),
                source: source.into(),
                source_handle: source_handle.into(),
                target: target.into(),
                target_handle: target_handle.into(),
            },
        )
        .collect(),
        derived_graph: None,
    }
}

fn request(session_id: &str) -> WorkflowExecutionSessionRunRequest {
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

async fn session(service: &WorkflowService, host: &Host) -> String {
    service
        .create_workflow_execution_session(
            host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: "tool-loop-workflow".into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await
        .unwrap()
        .session_id
}

fn save(root: &std::path::Path, catalog: &CustomNodeCatalog) -> String {
    WorkflowService::new()
        .workflow_graph_save(
            &FileSystemWorkflowGraphStore::new(root),
            crate::WorkflowGraphSaveRequest {
                name: "tool-loop-workflow".into(),
                graph: graph(catalog.tool_loop_identity("pure-loop").unwrap()),
            },
        )
        .unwrap()
        .path
}

#[tokio::test]
async fn tool_loop_saved_reopened_graph_runs_authorized_primitives_and_sink() {
    let root = tempfile::tempdir().unwrap();
    let initial = catalog(
        Executor::new(Mode::RoundTrip),
        Executor::new(Mode::Uppercase),
        3,
    );
    let path = save(root.path(), &initial);
    drop(initial);
    let controller = Executor::new(Mode::RoundTrip);
    let tool = Executor::new(Mode::Uppercase);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    let service = WorkflowService::with_trusted_custom_nodes(catalog.clone())
        .with_attribution_store(SqliteAttributionStore::open_in_memory().unwrap())
        .with_diagnostics_ledger(SqliteDiagnosticsLedger::open_in_memory().unwrap());
    let reopened = service
        .workflow_graph_load(
            &FileSystemWorkflowGraphStore::new(root.path()),
            crate::WorkflowGraphLoadRequest { path: path.clone() },
        )
        .unwrap();
    assert_eq!(
        reopened.graph.nodes[1].data[TOOL_LOOP_IDENTITY_FIELD],
        serde_json::to_value(catalog.tool_loop_identity("pure-loop").unwrap()).unwrap()
    );
    let inspection = service
        .workflow_graph_inspect(
            &FileSystemWorkflowGraphStore::new(root.path()),
            crate::WorkflowGraphInspectionRequest {
                path,
                selected_node_id: None,
            },
        )
        .unwrap();
    assert!(
        !inspection.diagnostics.iter().any(|d| d.blocking_submission),
        "{:?}",
        inspection.diagnostics
    );
    let version = service
        .resolve_workflow_graph_version("tool-loop-workflow", "1.0.0", &reopened.graph)
        .unwrap();
    // A zero-inference validation fixture does not assert real model readiness.
    service.store_workflow_executable_validation_snapshot(WorkflowExecutableValidationSnapshotRecord {
        schema_version: WORKFLOW_EXECUTABLE_VALIDATION_SNAPSHOT_SCHEMA_VERSION,
        validation_snapshot_id: WorkflowExecutableValidationSnapshotId::generate(),
        workflow_id: version.workflow_id.clone(), workflow_version_id: version.workflow_version_id.clone(), workflow_semantic_version: version.semantic_version.clone(), workflow_execution_fingerprint: version.execution_fingerprint.clone(),
        descriptor_contract_version: pantograph_inference_interface_contracts::INFERENCE_INTERFACE_CONTRACT_VERSION,
        graph_revision: pantograph_inference_interface_contracts::WorkflowGraphRevision::parse(reopened.graph.compute_fingerprint()).unwrap(),
        validation_session_id: pantograph_inference_interface_contracts::DraftGraphValidationSessionId::parse("validation.tool-loop").unwrap(),
        validation_summary: pantograph_inference_interface_contracts::DraftGraphValidationSummary { status: pantograph_inference_interface_contracts::DraftGraphValidationStatus::Executable, executable: true, enqueue_disabled_reasons: vec![], diagnostics_count: 0, blocking_diagnostics_count: 0 }, nodes: vec![],
    }).unwrap();
    let host = Host {
        root: root.path().into(),
    };
    let session_id = session(&service, &host).await;
    let response = service
        .run_workflow_execution_session(&host, request(&session_id))
        .await
        .unwrap();
    assert_eq!(response.outputs.len(), 1);
    assert_eq!(response.outputs[0].value, serde_json::json!("MIXED CASE"));
    assert_eq!(controller.calls.lock().len(), 2);
    let parent = tool.calls.lock()[0].0.clone();
    assert_eq!(parent.task_id, "loop");
    assert!(!parent.attempt_id.is_empty());
    let run = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: parent.workflow_run_id,
        })
        .unwrap()
        .run_graph
        .unwrap();
    assert_eq!(
        run.graph
            .nodes
            .iter()
            .find(|n| n.id == "loop")
            .unwrap()
            .data[TOOL_LOOP_IDENTITY_FIELD],
        reopened.graph.nodes[1].data[TOOL_LOOP_IDENTITY_FIELD]
    );
    assert!(!run.graph_diagnostics.iter().any(|d| d.blocking_submission));
    // A late cancellation cannot claim ownership after commitment.
    let workflow_run_id = tool.calls.lock()[0].0.workflow_run_id.clone();
    assert!(service
        .workflow_cancel_active_execution_session_task(
            WorkflowExecutionSessionActiveTaskCancelRequest {
                session_id,
                workflow_run_id,
                task_id: "loop".into(),
                reason: None
            }
        )
        .await
        .is_err());
}

#[tokio::test]
async fn tool_loop_saved_graph_rejects_missing_stale_forged_and_unsupported_authority() {
    let root = tempfile::tempdir().unwrap();
    let controller = Executor::new(Mode::RoundTrip);
    let tool = Executor::new(Mode::Uppercase);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    let path = save(root.path(), &catalog);
    let saved = FileSystemWorkflowGraphStore::new(root.path())
        .load_workflow(path)
        .unwrap()
        .graph;
    assert!(WorkflowService::new()
        .node_registry
        .executable_topology(&saved)
        .is_err());
    let service = WorkflowService::with_trusted_custom_nodes(catalog);
    let baseline = service.node_registry.executable_topology(&saved).unwrap();
    for port in ["tools", "max_turns", "system_prompt", "context"] {
        let mut connected = saved.clone();
        connected.edges.push(GraphEdge {
            id: format!("unsupported-{port}"),
            source: "source".into(),
            source_handle: "text".into(),
            target: "loop".into(),
            target_handle: port.into(),
        });
        let error = crate::workflow::task_graph::workflow_scheduler_task_graph_with_registry(
            &WorkflowId::try_from("workflow".to_string()).unwrap(),
            &WorkflowRunId::try_from("run".to_string()).unwrap(),
            &connected,
            &Default::default(),
            &service.node_registry,
        )
        .unwrap_err();
        assert!(
            matches!(error, WorkflowServiceError::InvalidRequest(ref message)
            if message == "pure tool-loop requires exactly one prompt binding; optional authoring ports are unsupported"),
            "{port}: {error}"
        );
    }
    for field in [
        "max_turns",
        "tools",
        "definition",
        "context",
        "system_prompt",
    ] {
        let mut changed = saved.clone();
        changed.nodes[1].data[field] = serde_json::json!({});
        assert!(
            service.node_registry.executable_topology(&changed).is_err(),
            "{field}"
        );
    }
    let mut forged = saved.clone();
    forged.nodes[1].data[TOOL_LOOP_IDENTITY_FIELD]["controller"]["implementation_digest"] =
        serde_json::json!("forged");
    assert!(service.node_registry.executable_topology(&forged).is_err());
    let changed_catalog = crate::tool_loops::tests::catalog(controller.clone(), tool.clone(), 4);
    let changed = WorkflowService::with_trusted_custom_nodes(changed_catalog.clone());
    assert!(changed.node_registry.executable_topology(&saved).is_err());
    let changed_graph = graph(changed_catalog.tool_loop_identity("pure-loop").unwrap());
    assert_ne!(
        baseline,
        changed
            .node_registry
            .executable_topology(&changed_graph)
            .unwrap()
    );
    assert!(controller.calls.lock().is_empty());
    assert!(tool.calls.lock().is_empty());
}

#[tokio::test]
async fn tool_loop_active_scheduler_cancellation_drops_tool_and_prevents_sink() {
    let root = tempfile::tempdir().unwrap();
    let controller = Executor::new(Mode::RoundTrip);
    let tool = Executor::new(Mode::Pending);
    let catalog = catalog(controller.clone(), tool.clone(), 3);
    save(root.path(), &catalog);
    let service = WorkflowService::with_trusted_custom_nodes(catalog.clone());
    let host = Host {
        root: root.path().into(),
    };
    let session_id = session(&service, &host).await;
    let (result, ()) = tokio::join!(
        service.run_workflow_execution_session(&host, request(&session_id)),
        async {
            tool.entered.notified().await;
            let parent = tool.calls.lock()[0].0.clone();
            let response = service
                .workflow_cancel_active_execution_session_task(
                    WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id: session_id.clone(),
                        workflow_run_id: parent.workflow_run_id,
                        task_id: parent.task_id,
                        reason: Some("cancel owned tool".into()),
                    },
                )
                .await
                .unwrap();
            assert_eq!(response.active_attempt_id, parent.attempt_id);
        }
    );
    assert!(matches!(result, Err(WorkflowServiceError::Cancelled(_))));
    assert!(tool.dropped.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(controller.calls.lock().len(), 1);
    let parent = tool.calls.lock()[0].0.clone();
    assert!(catalog
        .cancel(&parent.workflow_run_id, &parent.task_id, &parent.attempt_id)
        .is_err());
}

#[tokio::test(start_paused = true)]
async fn tool_loop_outer_workflow_timeout_disposes_primitive_and_registration() {
    let root = tempfile::tempdir().unwrap();
    let controller = Executor::new(Mode::Pending);
    let catalog = catalog(controller.clone(), Executor::new(Mode::Uppercase), 3);
    save(root.path(), &catalog);
    let service = WorkflowService::with_trusted_custom_nodes(catalog.clone());
    let host = Host {
        root: root.path().into(),
    };
    let session_id = session(&service, &host).await;
    let mut run_request = request(&session_id);
    run_request.timeout_ms = Some(10);
    assert!(matches!(
        service
            .run_workflow_execution_session(&host, run_request)
            .await,
        Err(WorkflowServiceError::RuntimeTimeout(_))
    ));
    assert!(controller.dropped.load(std::sync::atomic::Ordering::SeqCst));
    let parent = controller.calls.lock()[0].0.clone();
    assert!(catalog
        .cancel(&parent.workflow_run_id, &parent.task_id, &parent.attempt_id)
        .is_err());
}

#[tokio::test]
async fn tool_loop_scheduler_failures_are_terminal_and_do_not_return_partial_outputs() {
    for (mode, tool_mode, expected) in [
        (
            Mode::Unauthorized,
            Mode::Uppercase,
            "outside its host grant",
        ),
        (Mode::Repeat, Mode::Uppercase, "turn budget"),
        (Mode::RoundTrip, Mode::Fail, "callback failed"),
        (Mode::RoundTrip, Mode::Panic, "callback panicked"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let controller = Executor::new(mode);
        let tool = Executor::new(tool_mode);
        let catalog = catalog(controller.clone(), tool, 2);
        save(root.path(), &catalog);
        let service = WorkflowService::with_trusted_custom_nodes(catalog.clone());
        let host = Host {
            root: root.path().into(),
        };
        let session_id = session(&service, &host).await;
        let error = service
            .run_workflow_execution_session(&host, request(&session_id))
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{error}");
        assert!(!error.to_string().contains("sensitive"));
        let parent = controller.calls.lock()[0].0.clone();
        assert!(catalog
            .cancel(&parent.workflow_run_id, &parent.task_id, &parent.attempt_id)
            .is_err());
    }
}
