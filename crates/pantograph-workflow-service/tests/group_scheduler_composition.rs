//! Combined group lowering and dependency-control projection regression.
//! This uses public authoring/projection APIs; no inference forward is claimed.
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_workflow_service::graph::{lower_groups, GraphSessionStore};
use pantograph_workflow_service::workflow::{
    workflow_scheduler_task_graph, WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskGraph,
    WorkflowSchedulerTaskProjectionDiagnosticCode,
};
use pantograph_workflow_service::*;
use serde_json::{json, Value};

const SIDECAR: &str = "dependency_environment_sidecar";

fn node(id: &str, node_type: &str, data: Value) -> GraphNode {
    GraphNode {
        id: id.into(),
        node_type: node_type.into(),
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

fn fixture() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            node("source", "selection-input", json!({})),
            node("pass", "json-filter", json!({"path":""})),
            node("extract", "json-filter", json!({"path":"message"})),
            node("sink", "text-output", json!({})),
            node("dep", "dependency-environment", json!({})),
            // No resolver/readiness evidence is supplied for this root node.
            // It must remain blocked even while its control is excluded.
            node("infer", "llm-inference", json!({})),
        ],
        edges: vec![
            edge("input", "source", "value", "pass", "json"),
            edge("inside", "pass", "value", "extract", "json"),
            edge("output", "extract", "value", "sink", "text"),
            edge("control", "dep", SIDECAR, "infer", SIDECAR),
        ],
        ..WorkflowGraph::default()
    }
}

async fn grouped(graph: WorkflowGraph) -> Result<WorkflowGraph, WorkflowServiceError> {
    let store = GraphSessionStore::new();
    let session = store.create_session(graph, None).await;
    store
        .create_group(WorkflowGraphCreateGroupRequest {
            session_id: session.session_id,
            name: "CPU extraction".into(),
            selected_node_ids: vec!["pass".into(), "extract".into()],
        })
        .await
        .map(|response| response.graph)
}

fn tasks(graph: &WorkflowGraph) -> Result<WorkflowSchedulerTaskGraph, WorkflowServiceError> {
    workflow_scheduler_task_graph(
        &WorkflowId::try_from("combined-projection".to_string()).unwrap(),
        &WorkflowRunId::try_from("combined-run".to_string()).unwrap(),
        graph,
    )
}

#[tokio::test]
async fn grouped_cpu_projection_preserves_root_control_exclusion_and_inference_refusal() {
    let flat = fixture();
    let authored = grouped(flat.clone()).await.unwrap();
    let unchanged = authored.clone();
    let projection = lower_groups(&authored, &NodeRegistry::new()).unwrap();
    assert_eq!(projection.parent_by_node.len(), 2);
    assert!(projection.parent_by_node.contains_key("extract"));
    assert_eq!(
        workflow_executable_topology(&authored).unwrap(),
        workflow_executable_topology(&flat).unwrap()
    );
    let mut flat_tasks = tasks(&flat).unwrap();
    let mut group_tasks = tasks(&authored).unwrap();
    flat_tasks.tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    group_tasks.tasks.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    assert_eq!(group_tasks, flat_tasks);
    assert_eq!(group_tasks.tasks.len(), 5);
    assert!(group_tasks
        .tasks
        .iter()
        .all(|task| task.node_id.as_str() != "dep"));
    let infer = group_tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .unwrap();
    assert!(infer.input_bindings.is_empty());
    assert!(infer.dependency_task_ids.is_empty());
    assert!(infer.schedulable_intent_template.is_none());
    assert!(infer.schedulable_intent.is_none());
    assert!(infer.diagnostics.iter().any(|diagnostic| diagnostic.code
        == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingInferenceDescriptor));
    let extract = group_tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "extract")
        .unwrap();
    assert!(extract.diagnostics.is_empty());
    assert_eq!(extract.input_bindings[0].source_node_id.as_str(), "pass");
    assert_eq!(extract.input_bindings[0].source_port_id, "value");
    assert!(projection
        .executable_graph
        .edges
        .iter()
        .any(|edge| edge.id == "input" && edge.source == "source" && edge.target == "pass"));
    assert_eq!(authored, unchanged);
}

#[tokio::test]
async fn grouping_does_not_hide_missing_or_malformed_root_control_associations() {
    for kind in ["missing", "wrong-handle", "wrong-target", "bound"] {
        let mut graph = fixture();
        match kind {
            "missing" => graph.edges.retain(|edge| edge.id != "control"),
            "wrong-handle" => graph.edges[3].target_handle = "prompt".into(),
            "wrong-target" => graph.edges[3].target = "extract".into(),
            "bound" => graph.edges.push(edge(
                "bound",
                "source",
                "value",
                "dep",
                "selected_binding_ids",
            )),
            _ => unreachable!(),
        }
        let flat = tasks(&graph).unwrap();
        let dep = flat
            .tasks
            .iter()
            .find(|task| task.node_id.as_str() == "dep")
            .unwrap();
        assert_eq!(
            dep.execution_class,
            WorkflowSchedulerTaskExecutionClass::Unsupported,
            "{kind}"
        );
        assert!(dep.schedulable_intent.is_none(), "{kind}");
        let authored = match grouped(graph).await {
            Ok(graph) => graph,
            Err(error) => {
                assert!(
                    matches!(
                        error,
                        WorkflowServiceError::InvalidRequest(_)
                            | WorkflowServiceError::StaleWorkflowGraph { .. }
                    ),
                    "{kind}: {error}"
                );
                continue;
            }
        };
        let before = authored.clone();
        match tasks(&authored) {
            Ok(tasks) => {
                let dep = tasks
                    .tasks
                    .iter()
                    .find(|task| task.node_id.as_str() == "dep")
                    .unwrap();
                assert_eq!(
                    dep.execution_class,
                    WorkflowSchedulerTaskExecutionClass::Unsupported,
                    "{kind}"
                );
                assert!(dep.schedulable_intent.is_none(), "{kind}");
            }
            Err(WorkflowServiceError::InvalidRequest(message)) => {
                assert_eq!(kind, "wrong-target");
                assert!(
                    message.contains("mapping references unknown Input"),
                    "{message}"
                );
            }
            Err(error) => assert!(
                matches!(error, WorkflowServiceError::StaleWorkflowGraph { .. }),
                "{kind}: {error}"
            ),
        }
        assert_eq!(authored, before);
    }
}
