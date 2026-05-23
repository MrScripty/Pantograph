use pantograph_dependency_planning::PumasModelRef;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_resolve_task_intent, workflow_scheduler_task_graph,
    WorkflowSchedulerTaskBindingDiagnosticCode, WorkflowSchedulerTaskBindingResolutionStatus,
    WorkflowSchedulerTaskResult, WorkflowSchedulerTaskResultOutput,
    WorkflowSchedulerTaskResultStatus, WorkflowSchedulerTaskResultValue,
    WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-task-binding".to_string()).expect("workflow id")
}

fn workflow_run_id() -> WorkflowRunId {
    WorkflowRunId::try_from("run-task-binding".to_string()).expect("workflow run id")
}

fn graph_with_bound_model_ref() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "model-selector".to_string(),
                node_type: "puma-lib".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({}),
            },
            GraphNode {
                id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: json!({
                    "task_kind": "image_generation",
                    "runtime": "pytorch",
                    "device": "cuda:0",
                    "denoising_scheduler": "euler_discrete",
                    "model_path": "/tmp/legacy-model"
                }),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge-model-infer".to_string(),
            source: "model-selector".to_string(),
            source_handle: "pumas_model_ref".to_string(),
            target: "infer".to_string(),
            target_handle: "pumas_model_ref".to_string(),
        }],
        derived_graph: None,
    }
}

fn materialized_model_ref_result(
    workflow_run_id: &str,
    value: WorkflowSchedulerTaskResultValue,
) -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: "workflow-task-binding".to_string(),
        workflow_run_id: workflow_run_id.to_string(),
        node_id: "model-selector".to_string(),
        task_id: "model-selector".to_string(),
        status: WorkflowSchedulerTaskResultStatus::Completed,
        outputs: vec![WorkflowSchedulerTaskResultOutput {
            port_id: "pumas_model_ref".to_string(),
            value,
        }],
        diagnostics: Vec::new(),
        terminal_metadata: None,
    }
}

fn pumas_model_ref_value() -> WorkflowSchedulerTaskResultValue {
    WorkflowSchedulerTaskResultValue::PumasModelRef(PumasModelRef {
        model_id: "image/example/tiny-diffusion".to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some("diffusers-bundle".to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    })
}

#[test]
fn binding_resolution_materializes_model_ref_into_schedulable_intent() {
    let graph = workflow_scheduler_task_graph(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    assert!(inference_task.schedulable_intent.is_none());
    assert!(inference_task.schedulable_intent_template.is_some());
    assert!(inference_task.diagnostics.is_empty());

    let resolution = workflow_scheduler_resolve_task_intent(
        inference_task,
        &[materialized_model_ref_result(
            graph.workflow_run_id.as_str(),
            pumas_model_ref_value(),
        )],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Ready
    );
    let intent = resolution.schedulable_intent.expect("ready intent");
    assert_eq!(intent.model_ref.model_id, "image/example/tiny-diffusion");
    assert_eq!(intent.task_type.as_str(), "image_generation");
    assert_eq!(
        intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("pytorch")
    );
    let encoded = serde_json::to_string(&intent).expect("encode intent");
    assert!(!encoded.contains("model_path"));
    assert!(!encoded.contains("/tmp/legacy-model"));
}

#[test]
fn binding_resolution_blocks_until_materialized_model_ref_exists() {
    let graph = workflow_scheduler_task_graph(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    let resolution = workflow_scheduler_resolve_task_intent(inference_task, &[]);

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Blocked
    );
    assert_eq!(
        resolution.diagnostics[0].code,
        WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput
    );
    assert!(resolution.schedulable_intent.is_none());
}

#[test]
fn binding_resolution_rejects_wrong_materialized_value_type() {
    let graph = workflow_scheduler_task_graph(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    let resolution = workflow_scheduler_resolve_task_intent(
        inference_task,
        &[materialized_model_ref_result(
            graph.workflow_run_id.as_str(),
            WorkflowSchedulerTaskResultValue::String("not-a-model-ref".to_string()),
        )],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Invalid
    );
    assert_eq!(
        resolution.diagnostics[0].code,
        WorkflowSchedulerTaskBindingDiagnosticCode::WrongMaterializedValueType
    );
    assert!(resolution.schedulable_intent.is_none());
}
