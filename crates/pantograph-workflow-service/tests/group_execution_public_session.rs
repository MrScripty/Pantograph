//! Public saved-group CPU execution. The host never lowers or executes graphs.
use async_trait::async_trait;
use pantograph_node_contracts::ComposedTracePolicy;
use pantograph_runtime_attribution::{
    BucketSelection, ClientRegistrationRequest, ClientSessionOpenRequest, WorkflowId, WorkflowRunId,
};
use pantograph_workflow_service::graph::{lower_groups, GraphSessionStore};
use pantograph_workflow_service::workflow::workflow_scheduler_task_graph;
use pantograph_workflow_service::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

struct FixtureDirectory {
    path: PathBuf,
    keep: bool,
}
impl FixtureDirectory {
    fn new(name: &str) -> Self {
        let (path, keep) = match std::env::var("GROUP_EXECUTION_EVIDENCE_DIR") {
            Ok(root) => (PathBuf::from(root).join(name), true),
            Err(_) => (
                std::env::temp_dir().join(format!("pantograph-{name}-{}", uuid::Uuid::new_v4())),
                false,
            ),
        };
        std::fs::create_dir_all(&path).unwrap();
        Self { path, keep }
    }
    fn evidence(&self, name: &str, value: &Value) {
        if self.keep {
            std::fs::write(
                self.path.join(name),
                serde_json::to_vec_pretty(value).unwrap(),
            )
            .unwrap();
        }
    }
}
impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

struct DiskHost {
    root: PathBuf,
    legacy_calls: Arc<AtomicUsize>,
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
        self.legacy_calls.fetch_add(1, Ordering::SeqCst);
        panic!("public grouped runs must use existing scheduler CPU tasks, not legacy execution");
    }
}

fn node(id: &str, kind: &str, mut data: Value) -> GraphNode {
    // These are actual public authoring definitions, not a host I/O override.
    // Keep default-host behavior, including its existing definition requirement.
    if matches!(
        kind,
        "selection-input" | "text-input" | "text-output" | "vector-output"
    ) {
        data["definition"] =
            serde_json::to_value(NodeRegistry::new().get_definition(kind).unwrap()).unwrap();
    }
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
fn json_fixture() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            node("source", "selection-input", json!({})),
            node("pass", "json-filter", json!({"path":""})),
            node(
                "extract",
                "json-filter",
                json!({"path":"results[0].text", "opaque":{"keep":[null, 7, "🐾"]}}),
            ),
            node("sink", "text-output", json!({})),
        ],
        edges: vec![
            edge("json-edge", "source", "value", "pass", "json"),
            edge("pass-edge", "pass", "value", "extract", "json"),
            edge("text-edge", "extract", "value", "sink", "text"),
        ],
        ..WorkflowGraph::default()
    }
}
async fn group(graph: WorkflowGraph, ids: &[&str]) -> WorkflowGraph {
    let sessions = GraphSessionStore::new();
    let session = sessions.create_session(graph, None).await;
    sessions
        .create_group(WorkflowGraphCreateGroupRequest {
            session_id: session.session_id,
            name: "Processing".into(),
            selected_node_ids: ids.iter().map(|id| id.to_string()).collect(),
        })
        .await
        .unwrap()
        .graph
}
fn group_value(graph: &mut WorkflowGraph) -> &mut Value {
    &mut graph
        .nodes
        .iter_mut()
        .find(|node| node.node_type == "node-group")
        .unwrap()
        .data["group"]
}

async fn submit(
    dir: &FixtureDirectory,
    workflow: &str,
    input_node: &str,
    input_port: &str,
    value: Value,
    sink: &str,
    sink_port: &str,
) -> Result<WorkflowRunResponse, WorkflowServiceError> {
    let legacy = Arc::new(AtomicUsize::new(0));
    let host = DiskHost {
        root: dir.path.clone(),
        legacy_calls: legacy.clone(),
    };
    let service = WorkflowService::new();
    let created = service
        .create_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow.into(),
                usage_profile: None,
                keep_alive: false,
            },
        )
        .await?;
    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: input_node.into(),
                    port_id: input_port.into(),
                    value,
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: sink.into(),
                    port_id: sink_port.into(),
                }]),
                override_selection: None,
                timeout_ms: Some(5000),
                priority: None,
            },
        )
        .await;
    let status = service
        .workflow_get_execution_session_status(WorkflowExecutionSessionStatusRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .unwrap();
    let queue = service
        .workflow_list_execution_session_queue(WorkflowExecutionSessionQueueListRequest {
            session_id: created.session_id.clone(),
        })
        .await
        .unwrap();
    assert!(queue.items.is_empty());
    assert_eq!(status.session.run_count, u64::from(response.is_ok()));
    let closed = service
        .close_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .unwrap();
    assert!(closed.ok);
    assert_eq!(legacy.load(Ordering::SeqCst), 0);
    let result = match &response {
        Ok(result) => json!({"ok":result}),
        Err(error) => {
            json!({"error":serde_json::from_str::<Value>(&error.to_envelope_json()).unwrap()})
        }
    };
    dir.evidence(&format!("{workflow}-session.json"), &json!({"created":created,"status":status,"queue":queue,"closed":closed,"result":result,"legacy_host_calls":0,
        "host_overrides":["workflow_roots via public default_workflow_roots","legacy run_workflow panic guard"],
        "qualification":"existing public CPU session integration; no research profile, native GUI or admitted-snapshot isolation claim"}));
    response
}

fn input() -> Value {
    json!({"results":[{"text":"  BEFORE\n🐾  "},{"text":"  AFTER\n🐾  "}]})
}

#[tokio::test]
async fn persisted_group_path_edit_executes_after_reopen_without_ungroup() {
    let dir = FixtureDirectory::new("path-reopen");
    let store = FileSystemWorkflowGraphStore::new(&dir.path);
    let sessions = GraphSessionStore::new();
    let edit = sessions.create_session(json_fixture(), None).await;
    let grouped = sessions
        .create_group(WorkflowGraphCreateGroupRequest {
            session_id: edit.session_id.clone(),
            name: "Extraction".into(),
            selected_node_ids: vec!["pass".into(), "extract".into()],
        })
        .await
        .unwrap();
    let original = grouped.graph.clone();
    let path = store
        .save_workflow("group-before".into(), original.clone())
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = submit(
        &dir,
        "group-before",
        "source",
        "value",
        input(),
        "sink",
        "text",
    )
    .await
    .unwrap();
    assert_eq!(
        before.outputs,
        vec![WorkflowPortBinding {
            node_id: "sink".into(),
            port_id: "text".into(),
            value: json!("  BEFORE\n🐾  ")
        }]
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let wrapper = original
        .nodes
        .iter()
        .find(|node| node.node_type == "node-group")
        .unwrap();
    let inner = wrapper.data["group"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "extract")
        .unwrap();
    let changed = sessions
        .update_group_node_data(WorkflowGraphUpdateGroupNodeDataRequest {
            session_id: edit.session_id.clone(),
            group_id: wrapper.id.clone(),
            node_id: "extract".into(),
            expected_node_type: "json-filter".into(),
            expected_node_data: inner["data"].clone(),
            data: json!({"path":"results[1].text"}),
        })
        .await
        .unwrap();
    let after_path = store
        .save_workflow("group-after".into(), changed.graph.clone())
        .unwrap();
    sessions.close_session(&edit.session_id).await.unwrap();
    drop(sessions);
    drop(store);
    let reopened_store = FileSystemWorkflowGraphStore::new(&dir.path);
    let reopened = reopened_store.load_workflow(after_path.clone()).unwrap();
    assert_eq!(reopened.graph, changed.graph);
    let saved_bytes = std::fs::read(&after_path).unwrap();
    let projection = lower_groups(&reopened.graph, &NodeRegistry::new()).unwrap();
    assert_eq!(
        projection
            .executable_graph
            .find_node("extract")
            .unwrap()
            .data,
        json!({"path":"results[1].text","opaque":{"keep":[null,7,"🐾"]}})
    );
    assert_eq!(
        lower_groups(&projection.executable_graph, &NodeRegistry::new())
            .unwrap()
            .executable_graph,
        projection.executable_graph
    );
    assert_eq!(
        projection.compositions[&wrapper.id].trace_policy,
        ComposedTracePolicy::PreservePrimitiveFacts
    );
    assert_eq!(projection.parent_by_node["extract"], wrapper.id);
    assert_eq!(
        projection
            .executable_graph
            .edges
            .iter()
            .map(|edge| edge.id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["json-edge", "pass-edge", "text-edge"])
    );
    let after = submit(
        &dir,
        "group-after",
        "source",
        "value",
        input(),
        "sink",
        "text",
    )
    .await
    .unwrap();
    assert_eq!(
        after.outputs,
        vec![WorkflowPortBinding {
            node_id: "sink".into(),
            port_id: "text".into(),
            value: json!("  AFTER\n🐾  ")
        }]
    );
    assert_ne!(before.workflow_run_id, after.workflow_run_id);
    assert_eq!(std::fs::read(&after_path).unwrap(), saved_bytes);
    let tasks = workflow_scheduler_task_graph(
        &WorkflowId::try_from("group-after".to_string()).unwrap(),
        &WorkflowRunId::try_from(after.workflow_run_id.clone()).unwrap(),
        &reopened.graph,
    )
    .unwrap();
    assert_eq!(
        tasks
            .tasks
            .iter()
            .map(|task| task.task_id.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["source", "pass", "extract", "sink"])
    );
    assert!(tasks
        .tasks
        .iter()
        .all(|task| task.task_id.as_str() == task.node_id.as_str()
            && task.workflow_run_id.as_str() == after.workflow_run_id));
    assert_eq!(
        tasks
            .tasks
            .iter()
            .find(|task| task.node_id.as_str() == "extract")
            .unwrap()
            .non_runtime_task_template,
        Some(WorkflowSchedulerNonRuntimeTaskTemplate::JsonFilter {
            path: "results[1].text".into()
        })
    );
    dir.evidence("projection-and-run.json",&json!({"parent_base":"30e2c0ace28ee0626dd807e730d7dc4a24bde950","authored_graph":reopened.graph,"primitive_graph":projection.executable_graph,"compositions":projection.compositions,"parent_by_node":projection.parent_by_node,"projected_tasks_for_actual_run":tasks,"before":before,"after":after,"ungroup_calls":0,"direct_executor_calls":0,"file_unchanged":true}));
}

#[tokio::test]
async fn flat_graph_and_identity_contract_remain_unchanged() {
    let graph = json_fixture();
    let projection = lower_groups(&graph, &NodeRegistry::new()).unwrap();
    assert_eq!(projection.executable_graph, graph);
    assert!(projection.compositions.is_empty());
    assert!(projection.parent_by_node.is_empty());
    let grouped = group(graph.clone(), &["pass", "extract"]).await;
    assert_eq!(
        workflow_executable_topology(&grouped).unwrap(),
        workflow_executable_topology(&graph).unwrap()
    );
    assert_eq!(
        workflow_execution_fingerprint(&grouped).unwrap(),
        workflow_execution_fingerprint(&graph).unwrap()
    );
    let dir = FixtureDirectory::new("flat-control");
    FileSystemWorkflowGraphStore::new(&dir.path)
        .save_workflow("flat".into(), graph)
        .unwrap();
    let response = submit(&dir, "flat", "source", "value", input(), "sink", "text")
        .await
        .unwrap();
    assert_eq!(response.outputs[0].value, json!("  BEFORE\n🐾  "));
}

#[tokio::test]
async fn group_to_group_edges_rewrite_both_ends_and_execute() {
    let mut graph = json_fixture();
    graph
        .nodes
        .push(node("pass2", "json-filter", json!({"path":""})));
    graph
        .nodes
        .push(node("pass3", "json-filter", json!({"path":""})));
    graph.edges = vec![
        edge("in", "source", "value", "pass", "json"),
        edge("one", "pass", "value", "pass2", "json"),
        edge("cross", "pass2", "value", "pass3", "json"),
        edge("two", "pass3", "value", "extract", "json"),
        edge("out", "extract", "value", "sink", "text"),
    ];
    let grouped = group(
        group(graph.clone(), &["pass", "pass2"]).await,
        &["pass3", "extract"],
    )
    .await;
    let projection = lower_groups(&grouped, &NodeRegistry::new()).unwrap();
    assert_eq!(
        projection
            .executable_graph
            .edges
            .iter()
            .find(|edge| edge.id == "cross")
            .unwrap(),
        graph.edges.iter().find(|edge| edge.id == "cross").unwrap()
    );
    assert_eq!(projection.compositions.len(), 2);
    let dir = FixtureDirectory::new("two-groups");
    FileSystemWorkflowGraphStore::new(&dir.path)
        .save_workflow("two-groups".into(), grouped)
        .unwrap();
    assert_eq!(
        submit(
            &dir,
            "two-groups",
            "source",
            "value",
            input(),
            "sink",
            "text"
        )
        .await
        .unwrap()
        .outputs[0]
            .value,
        json!("  BEFORE\n🐾  ")
    );
}

#[tokio::test]
async fn merge_group_and_identical_output_fanout_mappings_execute() {
    let graph = WorkflowGraph {
        nodes: vec![
            node("source", "text-input", json!({})),
            node("a", "merge", json!({})),
            node("b", "merge", json!({})),
            node("sink", "text-output", json!({})),
            node("sink2", "text-output", json!({})),
        ],
        edges: vec![
            edge("in", "source", "text", "a", "inputs"),
            edge("middle", "a", "merged", "b", "inputs"),
            edge("out", "b", "merged", "sink", "text"),
            edge("fan", "b", "merged", "sink2", "text"),
        ],
        ..WorkflowGraph::default()
    };
    let grouped = group(graph, &["a", "b"]).await;
    let wrapper = grouped
        .nodes
        .iter()
        .find(|node| node.node_type == "node-group")
        .unwrap();
    assert_eq!(
        wrapper.data["group"]["exposed_outputs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let projection = lower_groups(&grouped, &NodeRegistry::new()).unwrap();
    assert_eq!(
        projection.compositions[&wrapper.id]
            .port_mappings
            .outputs
            .len(),
        1
    );
    let dir = FixtureDirectory::new("merge-fanout");
    FileSystemWorkflowGraphStore::new(&dir.path)
        .save_workflow("merge".into(), grouped)
        .unwrap();
    let response = submit(
        &dir,
        "merge",
        "source",
        "text",
        json!("  merge\n🐾  "),
        "sink",
        "text",
    )
    .await
    .unwrap();
    // Existing merge drops empty entries and preserves the remaining strings.
    assert_eq!(response.outputs[0].value, json!("  merge\n🐾  "));
}

#[tokio::test]
async fn malformed_groups_refuse_before_queue_and_do_not_rewrite_files() {
    let valid = group(json_fixture(), &["pass", "extract"]).await;
    let mut cases = Vec::new();
    let mut graph = valid.clone();
    group_value(&mut graph)["id"] = json!("wrong");
    cases.push(("wrong-identity", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["id"] = json!("pass ");
    cases.push(("inner-id-alias", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["exposed_outputs"][0]["group_port_id"] = json!(" out-extract-value ");
    cases.push(("port-id-alias", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["exposed_outputs"][0]["internal_port_id"] = json!("json");
    cases.push(("wrong-direction", graph));
    let mut graph = valid.clone();
    let mut mapping = group_value(&mut graph)["exposed_outputs"][0].clone();
    mapping["internal_node_id"] = json!("pass");
    group_value(&mut graph)["exposed_outputs"]
        .as_array_mut()
        .unwrap()
        .push(mapping);
    cases.push(("conflicting-mapping", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["id"] = json!("source");
    cases.push(("global-collision", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["node_type"] = json!("node-group");
    cases.push(("nested", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["node_type"] = json!("llm-inference");
    cases.push(("runtime", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0] = json!(["pass","json-filter",{"x":0,"y":0},{}]);
    cases.push(("array-node", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["edges"][0]["id"] = json!("json-edge");
    cases.push(("edge-collision", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["exposed_inputs"][0]["group_port_id"] = json!("unused");
    cases.push(("missing-boundary", graph));
    let mut graph = valid.clone();
    let mut mapping = group_value(&mut graph)["exposed_inputs"][0].clone();
    mapping["group_port_id"] = json!("unused");
    mapping["internal_node_id"] = json!("missing");
    group_value(&mut graph)["exposed_inputs"]
        .as_array_mut()
        .unwrap()
        .push(mapping);
    cases.push(("unused-dangling-map", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["data"]["definition"] = json!({"inputs":[{"id":"json","label":"JSON","data_type":"any","required":true,"multiple":true}]});
    cases.push(("unsupported-cpu-cardinality", graph));
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][0]["data"]["definition"] = json!({"outputs":[{"id":"found","label":"Found","data_type":"image","required":false,"multiple":false}]});
    cases.push(("unsupported-cpu-output-type", graph));
    let dir = FixtureDirectory::new("refusals");
    let store = FileSystemWorkflowGraphStore::new(&dir.path);
    for (name, graph) in cases {
        assert!(
            lower_groups(&graph, &NodeRegistry::new()).is_err(),
            "{name}"
        );
        let path = store.save_workflow(name.into(), graph).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            submit(&dir, name, "source", "value", input(), "sink", "text")
                .await
                .is_err(),
            "{name}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name}");
    }
    let mut graph = valid.clone();
    group_value(&mut graph)["nodes"][1]["data"]["path"] = json!(42);
    // The existing typed CPU template owns path validation; lowering retains
    // the exact data and task materialization refuses it before admission.
    assert_eq!(
        lower_groups(&graph, &NodeRegistry::new())
            .unwrap()
            .executable_graph
            .find_node("extract")
            .unwrap()
            .data["path"],
        42
    );
    let path = store.save_workflow("invalid-path".into(), graph).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(submit(
        &dir,
        "invalid-path",
        "source",
        "value",
        input(),
        "sink",
        "text"
    )
    .await
    .is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

fn assert_historic_graph(
    actual: &WorkflowRunGraphProjection,
    stored: &pantograph_runtime_attribution::WorkflowRunVersionProjection,
    authored: &WorkflowGraph,
) {
    assert_eq!(
        actual.workflow_run_id,
        stored.snapshot.workflow_run_id.as_str()
    );
    assert_eq!(actual.workflow_id, stored.snapshot.workflow_id.as_str());
    assert_eq!(
        actual.workflow_version_id,
        stored.snapshot.workflow_version_id.as_str()
    );
    assert_eq!(
        actual.workflow_presentation_revision_id,
        stored.snapshot.workflow_presentation_revision_id.as_str()
    );
    assert_eq!(
        actual.workflow_semantic_version,
        stored.snapshot.workflow_semantic_version
    );
    assert_eq!(
        actual.workflow_execution_fingerprint,
        stored.snapshot.workflow_execution_fingerprint
    );
    assert_eq!(actual.snapshot_created_at_ms, stored.snapshot.created_at_ms);
    assert_eq!(
        actual.workflow_version_created_at_ms,
        stored.workflow_version.created_at_ms
    );
    assert_eq!(
        actual.presentation_revision_created_at_ms,
        stored.presentation_revision.created_at_ms
    );
    assert_eq!(
        actual.executable_topology,
        serde_json::from_str::<WorkflowExecutableTopology>(
            &stored.workflow_version.executable_topology_json
        )
        .unwrap()
    );
    assert_eq!(
        actual.presentation_metadata,
        serde_json::from_str::<WorkflowPresentationMetadata>(
            &stored.presentation_revision.presentation_metadata_json
        )
        .unwrap()
    );
    assert_eq!(
        actual.graph_settings,
        serde_json::from_str::<WorkflowGraphRunSettings>(&stored.snapshot.graph_settings_json)
            .unwrap()
    );
    assert_eq!(actual.graph.nodes.len(), authored.nodes.len());
    assert_eq!(actual.graph.edges.len(), authored.edges.len());
    for node in &authored.nodes {
        assert_eq!(actual.graph.find_node(&node.id), Some(node));
    }
    for edge in &authored.edges {
        assert_eq!(
            actual.graph.edges.iter().find(|e| e.id == edge.id),
            Some(edge)
        );
    }
    assert!(actual.graph.derived_graph.is_none());
}

#[tokio::test]
async fn attributed_run_graph_query_preserves_authored_group_and_primitive_facts() {
    let dir = FixtureDirectory::new("attribution-query");
    let graph = group(json_fixture(), &["pass", "extract"]).await;
    FileSystemWorkflowGraphStore::new(&dir.path)
        .save_workflow("attributed-group".into(), graph.clone())
        .unwrap();
    let service = WorkflowService::with_ephemeral_attribution_store().unwrap();
    // Publish the actual grouped graph through the existing graph-session owner.
    // Inference submit readiness and raw graph contract diagnostics are distinct.
    let edit = service
        .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
            graph: graph.clone(),
            workflow_id: Some("attributed-group".into()),
        })
        .await
        .unwrap();
    let validation = service
        .workflow_graph_refresh_current_validation_summary(
            WorkflowGraphCurrentValidationRefreshRequest {
                graph_session_id: edit.session_id.clone(),
                graph_revision: edit.graph_revision.parse().unwrap(),
            },
        )
        .await
        .unwrap();
    assert!(validation.summary.submit_gate.allowed);
    let published = service
        .publish_graph_session_executable_validation_snapshot(
            WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                workflow_id: "attributed-group".into(),
                workflow_semantic_version: "0.1.0".into(),
                graph_session_id: edit.session_id.clone(),
                validation_session_id: validation.summary.validation_session_id.clone(),
                validation_snapshot_id: None,
            },
        )
        .await
        .unwrap();
    service
        .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
            session_id: edit.session_id,
        })
        .await
        .unwrap();
    let raw_diagnostics =
        validate_workflow_graph_contract_diagnostics(&graph, &NodeRegistry::new());
    assert!(raw_diagnostics
        .iter()
        .any(|d| d.node_type.as_deref() == Some("node-group") && d.blocking_submission));
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("group boundary fixture".into()),
            metadata_json: None,
        })
        .unwrap();
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("local fixture".into()),
        })
        .unwrap();
    let legacy = Arc::new(AtomicUsize::new(0));
    let host = DiskHost {
        root: dir.path.clone(),
        legacy_calls: legacy.clone(),
    };
    let created = service
        .create_attributed_workflow_execution_session(
            &host,
            WorkflowExecutionSessionAttributedCreateRequest {
                workflow_id: "attributed-group".into(),
                usage_profile: None,
                keep_alive: false,
                attribution: WorkflowExecutionSessionAttributionRequest {
                    credential: registered.credential_proof_request(),
                    client_session_id: opened.session.client_session_id.as_str().into(),
                    bucket_selection: BucketSelection::Default,
                },
            },
        )
        .await
        .unwrap();
    let response = service
        .run_workflow_execution_session(
            &host,
            WorkflowExecutionSessionRunRequest {
                session_id: created.session_id.clone(),
                workflow_semantic_version: "0.1.0".into(),
                inputs: vec![WorkflowPortBinding {
                    node_id: "source".into(),
                    port_id: "value".into(),
                    value: input(),
                }],
                output_targets: Some(vec![WorkflowOutputTarget {
                    node_id: "sink".into(),
                    port_id: "text".into(),
                }]),
                override_selection: None,
                timeout_ms: Some(5000),
                priority: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(response.outputs[0].value, json!("  BEFORE\n🐾  "));
    let snapshot = service
        .workflow_run_snapshot(&response.workflow_run_id)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.workflow_run_id.as_str(), response.workflow_run_id);
    let stored = service
        .workflow_run_version_projection(&response.workflow_run_id)
        .unwrap()
        .unwrap();
    let queried = service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
        })
        .unwrap()
        .run_graph
        .unwrap();
    assert_historic_graph(&queried, &stored, &graph);
    assert_eq!(queried.graph_diagnostics.len(), raw_diagnostics.len());
    for diagnostic in &raw_diagnostics {
        assert!(queried.graph_diagnostics.contains(diagnostic));
    }
    let closed = service
        .close_workflow_execution_session(
            &host,
            WorkflowExecutionSessionCloseRequest {
                session_id: created.session_id.clone(),
            },
        )
        .await
        .unwrap();
    assert!(closed.ok);
    assert_eq!(legacy.load(Ordering::SeqCst), 0);
    dir.evidence("attributed-query.json",&json!({"session":created,"response":response,"snapshot":snapshot,"grouped_validation_snapshot":published.as_record(),"inference_validation":validation,"raw_group_contract_diagnostics":raw_diagnostics,"run_graph_query":queried,"closed":closed,"qualification":"actual public grouped query succeeds; authored diagnostics remain unchanged; accepted source isolation remains separate"}));
}

#[tokio::test]
async fn lowered_ports_capacity_duplicate_connections_and_cycles_are_checked() {
    let valid = group(json_fixture(), &["pass", "extract"]).await;
    let mut incompatible = valid.clone();
    group_value(&mut incompatible)["exposed_outputs"][0]["internal_port_id"] = json!("found");
    let sink = incompatible.find_node_mut("sink").unwrap();
    *sink = node("sink", "image-output", json!({}));
    incompatible
        .edges
        .iter_mut()
        .find(|edge| edge.target == "sink")
        .unwrap()
        .target_handle = "image".into();
    assert!(
        matches!(lower_groups(&incompatible, &NodeRegistry::new()), Err(WorkflowServiceError::StaleWorkflowGraph {diagnostics,..}) if diagnostics.iter().any(|d| d.code == graph::WorkflowGraphDiagnosticCode::IncompatiblePortTypes))
    );
    let mut capacity = valid.clone();
    let mut duplicate = capacity.edges[0].clone();
    duplicate.id = "extra".into();
    capacity.edges.push(duplicate);
    assert!(lower_groups(&capacity, &NodeRegistry::new()).is_err());
    let mut capacity = valid.clone();
    capacity
        .nodes
        .push(node("source2", "selection-input", json!({})));
    let mut additional = capacity.edges[0].clone();
    additional.id = "second-input".into();
    additional.source = "source2".into();
    capacity.edges.push(additional);
    assert!(
        matches!(lower_groups(&capacity, &NodeRegistry::new()), Err(WorkflowServiceError::StaleWorkflowGraph {diagnostics,..}) if diagnostics.iter().any(|d| d.code == graph::WorkflowGraphDiagnosticCode::TargetInputCapacityReached))
    );
    let mut cycle = valid.clone();
    let group_id = cycle
        .nodes
        .iter()
        .find(|node| node.node_type == "node-group")
        .unwrap()
        .id
        .clone();
    let raw = group_value(&mut cycle);
    raw["exposed_inputs"].as_array_mut().unwrap().push(json!({"internal_node_id":"pass","internal_port_id":"json","group_port_id":"cycle-in","group_port_label":"Cycle","data_type":"any"}));
    cycle.edges.push(edge(
        "cycle",
        &group_id,
        "out-extract-value",
        &group_id,
        "cycle-in",
    ));
    assert!(
        matches!(lower_groups(&cycle, &NodeRegistry::new()), Err(WorkflowServiceError::StaleWorkflowGraph {diagnostics,..}) if diagnostics.iter().any(|d| d.code == graph::WorkflowGraphDiagnosticCode::CycleDetected))
    );
    let mut bypass = valid.clone();
    bypass.edges[0].target = "pass".into();
    bypass.edges[0].target_handle = "json".into();
    assert!(lower_groups(&bypass, &NodeRegistry::new()).is_err());
}

/// Query real stored history across edits/reopen, independently of current files.
#[tokio::test]
async fn attributed_group_history_queries_original_authored_graph_after_store_reopen() {
    use pantograph_runtime_attribution::SqliteAttributionStore;

    let dir = FixtureDirectory::new("historic-reopen");
    let db = dir.path.join("attribution.db");
    let mut authoring = json_fixture();
    for (index, node) in authoring.nodes.iter_mut().enumerate() {
        node.position = Position {
            x: 31.5 + index as f64 * 110.25,
            y: -22.75 + index as f64 * 50.5,
        };
    }
    let original = group(authoring, &["pass", "extract"]).await;
    let store = FileSystemWorkflowGraphStore::new(&dir.path);
    let workflow_path = store
        .save_workflow("historic-group".into(), original.clone())
        .unwrap();
    let original_bytes = std::fs::read(&workflow_path).unwrap();
    drop(store);
    let legacy = Arc::new(AtomicUsize::new(0));
    let host = DiskHost {
        root: dir.path.clone(),
        legacy_calls: legacy.clone(),
    };
    let mut service =
        WorkflowService::new().with_attribution_store(SqliteAttributionStore::open(&db).unwrap());
    let registered = service
        .register_attribution_client(ClientRegistrationRequest {
            display_name: Some("historic query reproduction".into()),
            metadata_json: None,
        })
        .unwrap();
    let opened = service
        .open_client_session(ClientSessionOpenRequest {
            credential: registered.credential_proof_request(),
            takeover: false,
            reason: Some("persistent local fixture".into()),
        })
        .unwrap();
    let mut graphs = vec![original.clone()];
    let mut runs = Vec::new();
    let mut records = Vec::new();
    let mut queried_graphs = Vec::new();
    let mut receipts = Vec::new();
    let mut changed_graph: Option<WorkflowGraph> = None;
    for phase in 0..2 {
        let workflow = if phase == 1 {
            "historic-flat"
        } else {
            "historic-group"
        };
        if phase == 1 {
            let edited = service
                .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                    graph: changed_graph.as_ref().unwrap().clone(),
                    workflow_id: Some("historic-group".into()),
                })
                .await
                .unwrap();
            let validation = service
                .workflow_graph_refresh_current_validation_summary(
                    WorkflowGraphCurrentValidationRefreshRequest {
                        graph_session_id: edited.session_id.clone(),
                        graph_revision: edited.graph_revision.parse().unwrap(),
                    },
                )
                .await
                .unwrap();
            assert!(validation.summary.submit_gate.allowed);
            let refusal = service
                .publish_graph_session_executable_validation_snapshot(
                    WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                        workflow_id: "historic-group".into(),
                        workflow_semantic_version: "0.1.0".into(),
                        graph_session_id: edited.session_id.clone(),
                        validation_session_id: validation.summary.validation_session_id.clone(),
                        validation_snapshot_id: None,
                    },
                )
                .await
                .unwrap_err();
            assert!(
                matches!(&refusal, WorkflowServiceError::InvalidRequest(message)
                if message.contains("freshness field 'graph_revision' changed"))
            );
            dir.evidence("edited-group-publication-refusal.json", &json!({
                "error":serde_json::from_str::<Value>(&refusal.to_envelope_json()).unwrap(),
                "qualification":"separate existing same-version validation freshness refusal; no second attributed group run claimed"}));
            service
                .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                    session_id: edited.session_id,
                })
                .await
                .unwrap();
        }
        let graph = graphs[phase].clone();
        let edit = service
            .workflow_graph_create_edit_session(WorkflowGraphEditSessionCreateRequest {
                graph: graph.clone(),
                workflow_id: Some(workflow.into()),
            })
            .await
            .unwrap();
        let validation = service
            .workflow_graph_refresh_current_validation_summary(
                WorkflowGraphCurrentValidationRefreshRequest {
                    graph_session_id: edit.session_id.clone(),
                    graph_revision: edit.graph_revision.parse().unwrap(),
                },
            )
            .await
            .unwrap();
        assert!(validation.summary.submit_gate.allowed);
        let published = service
            .publish_graph_session_executable_validation_snapshot(
                WorkflowGraphSessionExecutableValidationSnapshotPublishRequest {
                    workflow_id: workflow.into(),
                    workflow_semantic_version: "0.1.0".into(),
                    graph_session_id: edit.session_id.clone(),
                    validation_session_id: validation.summary.validation_session_id.clone(),
                    validation_snapshot_id: None,
                },
            )
            .await
            .unwrap();
        let created = service
            .create_attributed_workflow_execution_session(
                &host,
                WorkflowExecutionSessionAttributedCreateRequest {
                    workflow_id: workflow.into(),
                    usage_profile: None,
                    keep_alive: false,
                    attribution: WorkflowExecutionSessionAttributionRequest {
                        credential: registered.credential_proof_request(),
                        client_session_id: opened.session.client_session_id.as_str().into(),
                        bucket_selection: BucketSelection::Default,
                    },
                },
            )
            .await
            .unwrap();
        let response = service
            .run_workflow_execution_session(
                &host,
                WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id.clone(),
                    workflow_semantic_version: "0.1.0".into(),
                    inputs: vec![WorkflowPortBinding {
                        node_id: "source".into(),
                        port_id: "value".into(),
                        value: input(),
                    }],
                    output_targets: Some(vec![WorkflowOutputTarget {
                        node_id: "sink".into(),
                        port_id: "text".into(),
                    }]),
                    override_selection: None,
                    timeout_ms: Some(5000),
                    priority: None,
                },
            )
            .await
            .unwrap();
        let expected = json!("  BEFORE\n🐾  ");
        assert_eq!(response.outputs[0].value, expected);
        let version = service
            .workflow_run_version_projection(&response.workflow_run_id)
            .unwrap()
            .unwrap();
        let settings: WorkflowGraphRunSettings =
            serde_json::from_str(&version.snapshot.graph_settings_json).unwrap();
        let presentation: WorkflowPresentationMetadata =
            serde_json::from_str(&version.presentation_revision.presentation_metadata_json)
                .unwrap();
        let topology: WorkflowExecutableTopology =
            serde_json::from_str(&version.workflow_version.executable_topology_json).unwrap();
        assert_eq!(settings, workflow_graph_run_settings(&graph));
        assert_eq!(presentation, workflow_presentation_metadata(&graph));
        assert_eq!(topology, workflow_executable_topology(&graph).unwrap());
        let query = service.workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: response.workflow_run_id.clone(),
        });
        let queried = query.unwrap().run_graph.unwrap();
        assert_historic_graph(&queried, &version, &graph);
        if phase == 0 {
            assert!(!settings.nodes.iter().any(|n| n.node_id == "extract"));
            assert!(!presentation.nodes.iter().any(|n| n.node_id == "extract"));
            assert!(topology.nodes.iter().any(|n| n.node_id == "extract"));
            assert!(queried
                .graph_diagnostics
                .iter()
                .any(|d| d.node_type.as_deref() == Some("node-group") && d.blocking_submission));
        }
        let query_receipt = json!({"ok":queried});
        queried_graphs.push(queried);
        let closed = service
            .close_workflow_execution_session(
                &host,
                WorkflowExecutionSessionCloseRequest {
                    session_id: created.session_id.clone(),
                },
            )
            .await
            .unwrap();
        assert!(closed.ok);
        if phase == 0 {
            assert_eq!(std::fs::read(&workflow_path).unwrap(), original_bytes);
            let wrapper = graph
                .nodes
                .iter()
                .find(|n| n.node_type == "node-group")
                .unwrap();
            let inner = wrapper.data["group"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == "extract")
                .unwrap();
            let changed = service
                .workflow_graph_update_group_node_data(WorkflowGraphUpdateGroupNodeDataRequest {
                    session_id: edit.session_id.clone(),
                    group_id: wrapper.id.clone(),
                    node_id: "extract".into(),
                    expected_node_type: "json-filter".into(),
                    expected_node_data: inner["data"].clone(),
                    data: json!({"path":"results[1].text"}),
                })
                .await
                .unwrap();
            FileSystemWorkflowGraphStore::new(&dir.path)
                .save_workflow("historic-group".into(), changed.graph.clone())
                .unwrap();
            changed_graph = Some(changed.graph);
            let flat = json_fixture();
            FileSystemWorkflowGraphStore::new(&dir.path)
                .save_workflow("historic-flat".into(), flat.clone())
                .unwrap();
            graphs.push(flat);
        }
        service
            .workflow_graph_close_edit_session(WorkflowGraphEditSessionCloseRequest {
                session_id: edit.session_id,
            })
            .await
            .unwrap();
        runs.push(response.workflow_run_id.clone());
        records.push(version.clone());
        receipts.push(json!({"phase":phase,"session":created,"response":response,
            "publication":published.as_record(),"closed":closed,"query":query_receipt,
            "stored_version_projection":version,"authored_graph":graph}));
        // Close/reopen both the SQLite and workflow owners; never carry an in-memory
        // snapshot across this boundary as the authoritative query source.
        drop(service);
        service = WorkflowService::new()
            .with_attribution_store(SqliteAttributionStore::open(&db).unwrap());
        let current = FileSystemWorkflowGraphStore::new(&dir.path)
            .load_workflow(workflow_path.clone())
            .unwrap();
        assert_eq!(&current.graph, changed_graph.as_ref().unwrap());
        for (index, run_id) in runs.iter().enumerate() {
            assert_eq!(
                service
                    .workflow_run_version_projection(run_id)
                    .unwrap()
                    .as_ref(),
                Some(&records[index])
            );
            let reopened_query = service.workflow_run_graph_query(WorkflowRunGraphQueryRequest {
                workflow_run_id: run_id.clone(),
            });
            let reopened = reopened_query.unwrap().run_graph.unwrap();
            assert_historic_graph(&reopened, &records[index], &graphs[index]);
            assert_eq!(reopened, queried_graphs[index]);
        }
    }
    assert_ne!(runs[0], runs[1]);
    assert_ne!(
        records[0].snapshot.workflow_execution_session_id,
        records[1].snapshot.workflow_execution_session_id
    );
    assert_eq!(
        records[0].workflow_version.execution_fingerprint,
        records[1].workflow_version.execution_fingerprint
    );
    assert_ne!(
        records[0].snapshot.graph_settings_json,
        records[1].snapshot.graph_settings_json
    );
    assert_eq!(legacy.load(Ordering::SeqCst), 0);
    assert!(service
        .workflow_run_graph_query(WorkflowRunGraphQueryRequest {
            workflow_run_id: "run-unknown".into(),
        })
        .unwrap()
        .run_graph
        .is_none());
    dir.evidence("persistent-reopen-query-reproduction.json", &json!({
        "base_commit":"13226769558634bfec7747f735b83a2391f8fae4",
        "receipts":receipts,"workflow_store_reopens":2,"sqlite_owner_reopens":2,
        "historic_snapshots_unchanged_after_current_file_edit":true,"legacy_host_calls":0,
        "qualification":"actual public grouped and flat historic queries succeed after current-file edit and persistent owner reopen in same process; original authored data and stored primitive facts preserved"}));
}
