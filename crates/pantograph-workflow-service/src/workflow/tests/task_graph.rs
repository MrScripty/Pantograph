use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::SchedulerTraitValue;
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_task_graph, WorkflowSchedulerTaskProjectionDiagnosticCode,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-task-graph".to_string()).expect("workflow id")
}

fn workflow_run_id() -> WorkflowRunId {
    WorkflowRunId::try_from("run-task-graph".to_string()).expect("workflow run id")
}

fn graph_with_inline_inference_ref() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"value": "paint a red cube"}),
            },
            GraphNode {
                id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch",
                    "denoising_scheduler": "euler_discrete",
                    "pumas_model_ref": {
                        "model_id": "image/example/tiny-diffusion",
                        "revision": "main",
                        "selected_artifact_id": "diffusers-bundle"
                    },
                    "model_path": "/tmp/legacy-model"
                }),
            },
            GraphNode {
                id: "image-output".to_string(),
                node_type: "image-output".to_string(),
                position: Position { x: 400.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "edge-prompt-infer".to_string(),
                source: "prompt".to_string(),
                source_handle: "text".to_string(),
                target: "infer".to_string(),
                target_handle: "prompt".to_string(),
            },
            GraphEdge {
                id: "edge-infer-output".to_string(),
                source: "infer".to_string(),
                source_handle: "image".to_string(),
                target: "image-output".to_string(),
                target_handle: "image".to_string(),
            },
        ],
        derived_graph: None,
    }
}

#[test]
fn scheduler_task_graph_projects_path_free_inference_intent() {
    let graph = workflow_scheduler_task_graph(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_inline_inference_ref(),
    )
    .expect("scheduler task graph");

    assert_eq!(
        graph.schema_version,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION
    );
    assert_eq!(graph.tasks.len(), 3);

    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .expect("inference task");
    assert_eq!(inference_task.dependency_task_ids.len(), 1);
    assert_eq!(inference_task.dependency_task_ids[0].as_str(), "prompt");
    assert_eq!(
        inference_task.input_bindings[0].source_task_id.as_str(),
        "prompt"
    );
    assert_eq!(inference_task.input_bindings[0].target_port_id, "prompt");
    assert!(inference_task.diagnostics.is_empty());

    let intent = inference_task
        .schedulable_intent
        .as_ref()
        .expect("schedulable intent");
    assert_eq!(intent.workflow_id.as_str(), "workflow-task-graph");
    assert_eq!(intent.workflow_run_id.as_str(), "run-task-graph");
    assert_eq!(intent.node_id.as_str(), "infer");
    assert_eq!(intent.task_id.as_str(), "infer");
    assert_eq!(intent.task_type.as_str(), "image_generation");
    assert_eq!(intent.model_ref.model_id, "image/example/tiny-diffusion");
    assert_eq!(
        intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("pytorch")
    );
    assert_eq!(intent.trait_settings.len(), 1);
    assert_eq!(
        intent.trait_settings[0].trait_id.as_str(),
        "denoising_scheduler"
    );
    assert_eq!(
        intent.trait_settings[0].value,
        SchedulerTraitValue::String("euler_discrete".to_string())
    );

    let encoded = serde_json::to_string(&graph).expect("encode task graph");
    assert!(!encoded.contains("model_path"));
    assert!(!encoded.contains("/tmp/legacy-model"));
}

#[test]
fn scheduler_task_graph_reports_missing_canonical_inference_inputs() {
    let mut graph = graph_with_inline_inference_ref();
    let inference = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .expect("inference node");
    inference.data = json!({
        "model_ref": "pumas://models/image/example/tiny-diffusion",
        "model_path": "/tmp/legacy-model"
    });

    let graph =
        workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).expect("graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .expect("inference task");

    assert!(inference_task.schedulable_intent.is_none());
    assert!(inference_task.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingPumasModelRef
            && diagnostic.port_id.as_deref() == Some("pumas_model_ref")
    }));
    assert!(inference_task.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingTaskKind
            && diagnostic.port_id.as_deref() == Some("task_kind")
    }));
}
