use pantograph_dependency_planning::{DependencyTaskId, PumasModelRef, RuntimeIntentId};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{SchedulerNodeId, SchedulerRuntimeDeviceConstraints};
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_resolve_task_intent,
    workflow_scheduler_task_graph_with_inference_projections,
    WorkflowSchedulerInferenceTaskProjection, WorkflowSchedulerInferenceTaskProjections,
    WorkflowSchedulerReadyInferenceTaskProjection, WorkflowSchedulerTaskBindingDiagnosticCode,
    WorkflowSchedulerTaskBindingResolutionStatus, WorkflowSchedulerTaskResult,
    WorkflowSchedulerTaskResultOutput, WorkflowSchedulerTaskResultStatus,
    WorkflowSchedulerTaskResultValue, WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
};

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-task-binding".to_string()).expect("workflow id")
}

fn workflow_run_id() -> WorkflowRunId {
    WorkflowRunId::try_from("run-task-binding".to_string()).expect("workflow run id")
}

fn inference_projection() -> WorkflowSchedulerInferenceTaskProjections {
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Ready(
            WorkflowSchedulerReadyInferenceTaskProjection {
                node_id: SchedulerNodeId::parse("infer").expect("node id"),
                descriptor_fingerprint: InferenceInterfaceFingerprint::parse("iface.binding.v1")
                    .expect("fingerprint"),
                task_type: DependencyTaskId::parse("image_generation").expect("task kind"),
                model_ref: pumas_model_ref(),
                constraints: SchedulerRuntimeDeviceConstraints {
                    requested_runtime_id: Some(
                        RuntimeIntentId::parse("pytorch").expect("runtime id"),
                    ),
                    requested_device_id: None,
                },
                trait_settings: Vec::new(),
                estimate_hints: Vec::new(),
            },
        ),
    ])
    .expect("projection")
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

fn graph_with_bound_model_ref_and_prompt() -> WorkflowGraph {
    let mut graph = graph_with_bound_model_ref();
    graph.nodes.insert(
        0,
        GraphNode {
            id: "prompt".to_string(),
            node_type: "text-input".to_string(),
            position: Position { x: -200.0, y: 0.0 },
            data: json!({"value": "paint a red cube"}),
        },
    );
    graph.edges.push(GraphEdge {
        id: "edge-prompt-infer".to_string(),
        source: "prompt".to_string(),
        source_handle: "text".to_string(),
        target: "infer".to_string(),
        target_handle: "prompt".to_string(),
    });
    graph
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

fn materialized_prompt_result(
    workflow_run_id: &str,
    status: WorkflowSchedulerTaskResultStatus,
) -> WorkflowSchedulerTaskResult {
    WorkflowSchedulerTaskResult {
        schema_version: WORKFLOW_SCHEDULER_TASK_RESULT_SCHEMA_VERSION,
        workflow_id: "workflow-task-binding".to_string(),
        workflow_run_id: workflow_run_id.to_string(),
        node_id: "prompt".to_string(),
        task_id: "prompt".to_string(),
        status,
        outputs: vec![WorkflowSchedulerTaskResultOutput {
            port_id: "text".to_string(),
            value: WorkflowSchedulerTaskResultValue::String("paint a red cube".to_string()),
        }],
        diagnostics: Vec::new(),
        terminal_metadata: None,
    }
}

fn pumas_model_ref_value() -> WorkflowSchedulerTaskResultValue {
    WorkflowSchedulerTaskResultValue::PumasModelRef(pumas_model_ref())
}

fn pumas_model_ref() -> PumasModelRef {
    PumasModelRef {
        model_id: "image/example/tiny-diffusion".to_string(),
        revision: Some("main".to_string()),
        selected_artifact_id: Some("diffusers-bundle".to_string()),
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    }
}

#[test]
fn binding_resolution_readies_descriptor_intent_after_upstream_model_ref_materializes() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
        &inference_projection(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    assert!(inference_task.schedulable_intent.is_some());
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
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
        &inference_projection(),
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
fn binding_resolution_uses_descriptor_model_ref_not_materialized_model_ref_value() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref(),
        &inference_projection(),
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
            WorkflowSchedulerTaskResultValue::String("completed-upstream-node".to_string()),
        )],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Ready
    );
    assert_eq!(
        resolution
            .schedulable_intent
            .expect("intent")
            .model_ref
            .model_id,
        "image/example/tiny-diffusion"
    );
}

#[test]
fn binding_resolution_blocks_until_every_connected_input_is_materialized() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref_and_prompt(),
        &inference_projection(),
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
            pumas_model_ref_value(),
        )],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Blocked
    );
    assert_eq!(resolution.diagnostics[0].port_id.as_deref(), Some("prompt"));
    assert_eq!(
        resolution.diagnostics[0].code,
        WorkflowSchedulerTaskBindingDiagnosticCode::MissingMaterializedInput
    );
    assert!(resolution.schedulable_intent.is_none());
}

#[test]
fn binding_resolution_readies_after_every_connected_input_materializes() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref_and_prompt(),
        &inference_projection(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    let resolution = workflow_scheduler_resolve_task_intent(
        inference_task,
        &[
            materialized_model_ref_result(graph.workflow_run_id.as_str(), pumas_model_ref_value()),
            materialized_prompt_result(
                graph.workflow_run_id.as_str(),
                WorkflowSchedulerTaskResultStatus::Completed,
            ),
        ],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Ready
    );
    assert!(resolution.schedulable_intent.is_some());
    assert!(resolution.diagnostics.is_empty());
}

#[test]
fn binding_resolution_propagates_unavailable_connected_input() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref_and_prompt(),
        &inference_projection(),
    )
    .expect("scheduler task graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    let resolution = workflow_scheduler_resolve_task_intent(
        inference_task,
        &[
            materialized_model_ref_result(graph.workflow_run_id.as_str(), pumas_model_ref_value()),
            materialized_prompt_result(
                graph.workflow_run_id.as_str(),
                WorkflowSchedulerTaskResultStatus::Unavailable,
            ),
        ],
    );

    assert_eq!(
        resolution.status,
        WorkflowSchedulerTaskBindingResolutionStatus::Unavailable
    );
    assert_eq!(resolution.diagnostics[0].port_id.as_deref(), Some("prompt"));
    assert_eq!(
        resolution.diagnostics[0].code,
        WorkflowSchedulerTaskBindingDiagnosticCode::UpstreamTaskUnavailable
    );
    assert!(resolution.schedulable_intent.is_none());
}
