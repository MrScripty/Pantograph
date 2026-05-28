use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyRequirementsId, DependencyTaskId,
    DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{
    SchedulerNodeId, SchedulerRuntimeDeviceConstraints, SchedulerTraitId, SchedulerTraitSetting,
    SchedulerTraitValue,
};
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_task_graph, workflow_scheduler_task_graph_with_inference_projections,
    WorkflowSchedulerBlockedInferenceTaskProjection,
    WorkflowSchedulerBlockedInferenceTaskProjectionReason,
    WorkflowSchedulerInferenceTaskProjection, WorkflowSchedulerInferenceTaskProjections,
    WorkflowSchedulerNonRuntimeTaskTemplate, WorkflowSchedulerReadyInferenceTaskProjection,
    WorkflowSchedulerSourceInputTemplate, WorkflowSchedulerTaskExecutionClass,
    WorkflowSchedulerTaskProjectionDiagnosticCode, WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-task-graph".to_string()).expect("workflow id")
}

fn workflow_run_id() -> WorkflowRunId {
    WorkflowRunId::try_from("run-task-graph".to_string()).expect("workflow run id")
}

fn inference_projection() -> WorkflowSchedulerInferenceTaskProjections {
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Ready(
            WorkflowSchedulerReadyInferenceTaskProjection {
                node_id: SchedulerNodeId::parse("infer").expect("node id"),
                descriptor_fingerprint: InferenceInterfaceFingerprint::parse("iface.test.v1")
                    .expect("fingerprint"),
                task_type: DependencyTaskId::parse("image_generation").expect("task kind"),
                model_ref: PumasModelRef {
                    model_id: "image/example/tiny-diffusion".to_string(),
                    revision: Some("main".to_string()),
                    selected_artifact_id: Some("diffusers-bundle".to_string()),
                    selected_artifact_path: None,
                    migration_diagnostics: Vec::new(),
                },
                constraints: SchedulerRuntimeDeviceConstraints {
                    requested_runtime_id: Some(
                        RuntimeIntentId::parse("pytorch").expect("runtime id"),
                    ),
                    requested_device_id: Some(DeviceIntentId::parse("cuda:0").expect("device id")),
                },
                trait_settings: vec![SchedulerTraitSetting {
                    trait_id: SchedulerTraitId::parse("denoising_scheduler").expect("trait id"),
                    value: SchedulerTraitValue::String("euler_discrete".to_string()),
                }],
                estimate_hints: Vec::new(),
                dependency_requirements_id: DependencyRequirementsId::parse(
                    "requirements.image_generation.cuda0",
                )
                .expect("requirements id"),
                selected_binding_ids: vec![
                    DependencyBindingId::parse("torch-diffusers").expect("binding id")
                ],
                dependency_override_fingerprint: DependencyOverrideFingerprint::parse(
                    "override.none",
                )
                .expect("override fingerprint"),
            },
        ),
    ])
    .expect("projection")
}

fn blocked_inference_projection(
    reason: WorkflowSchedulerBlockedInferenceTaskProjectionReason,
) -> WorkflowSchedulerInferenceTaskProjections {
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Blocked(
            WorkflowSchedulerBlockedInferenceTaskProjection {
                node_id: SchedulerNodeId::parse("infer").expect("node id"),
                descriptor_fingerprint: Some(
                    InferenceInterfaceFingerprint::parse("iface.blocked.v1").expect("fingerprint"),
                ),
                reason,
                message: "descriptor is not executable".to_string(),
            },
        ),
    ])
    .expect("projection")
}

fn graph_with_inline_inference_ref() -> WorkflowGraph {
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"text": "paint a red cube"}),
            },
            GraphNode {
                id: "infer".to_string(),
                node_type: "llm-inference".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: json!({
                    "task_kind": "text_generation",
                    "runtime": "wrong-runtime-is-resolver-only",
                    "device": "wrong-device-is-resolver-only",
                    "denoising_scheduler": "wrong-trait-is-resolver-only",
                    "pumas_model_ref": {
                        "model_id": "wrong/graph/ref",
                        "revision": "raw-graph-values-are-not-scheduler-authority",
                        "selected_artifact_id": "wrong-artifact"
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
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_inline_inference_ref(),
        &inference_projection(),
    )
    .expect("scheduler task graph");

    assert_eq!(
        graph.schema_version,
        WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION
    );
    assert_eq!(graph.tasks.len(), 3);

    let prompt_task = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "prompt")
        .expect("prompt task");
    assert_eq!(
        prompt_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::SourceInput
    );
    assert!(prompt_task.non_runtime_task_template.is_none());
    assert_eq!(
        prompt_task.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".to_string()
        })
    );

    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .expect("inference task");
    assert_eq!(
        inference_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::RuntimeInference
    );
    assert_eq!(inference_task.dependency_task_ids.len(), 1);
    assert_eq!(inference_task.dependency_task_ids[0].as_str(), "prompt");
    assert_eq!(
        inference_task.input_bindings[0].source_task_id.as_str(),
        "prompt"
    );
    assert_eq!(inference_task.input_bindings[0].target_port_id, "prompt");
    assert!(inference_task.diagnostics.is_empty());
    assert_eq!(
        inference_task
            .inference_descriptor_fingerprint
            .as_ref()
            .map(|fingerprint| fingerprint.as_str()),
        Some("iface.test.v1")
    );

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
    assert_eq!(
        intent
            .constraints
            .requested_device_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("cuda:0")
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
fn scheduler_task_graph_reports_missing_descriptor_projection() {
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
        diagnostic.code == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingInferenceDescriptor
            && diagnostic.port_id.is_none()
    }));
}

#[test]
fn scheduler_task_graph_uses_descriptor_projection_instead_of_raw_graph_task_kind() {
    let mut graph = graph_with_inline_inference_ref();
    let inference = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .expect("inference node");
    inference.data = json!({
        "task_kind": "text_generation",
        "runtime": "wrong-runtime",
        "device": "wrong-device",
        "pumas_model_ref": {
            "model_id": "wrong/raw-graph-model",
            "revision": "wrong",
            "selected_artifact_id": "wrong"
        }
    });

    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph,
        &inference_projection(),
    )
    .expect("graph");
    let intent = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .and_then(|task| task.schedulable_intent.as_ref())
        .expect("schedulable intent");

    assert_eq!(intent.task_type.as_str(), "image_generation");
    assert_eq!(intent.model_ref.model_id, "image/example/tiny-diffusion");
    assert_eq!(
        intent
            .constraints
            .requested_runtime_id
            .as_ref()
            .map(|runtime| runtime.as_str()),
        Some("pytorch")
    );
}

#[test]
fn scheduler_task_graph_reports_blocked_descriptor_projection() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_inline_inference_ref(),
        &blocked_inference_projection(WorkflowSchedulerBlockedInferenceTaskProjectionReason::Stale),
    )
    .expect("graph");
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "infer")
        .expect("inference task");

    assert!(inference_task.schedulable_intent.is_none());
    assert!(inference_task.schedulable_intent_template.is_none());
    assert_eq!(
        inference_task
            .inference_descriptor_fingerprint
            .as_ref()
            .map(|fingerprint| fingerprint.as_str()),
        Some("iface.blocked.v1")
    );
    assert!(inference_task.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowSchedulerTaskProjectionDiagnosticCode::StaleInferenceDescriptor
            && diagnostic.message == "descriptor is not executable"
    }));
}

#[test]
fn scheduler_task_graph_classifies_materialization_and_unsupported_tasks() {
    let mut graph = graph_with_inline_inference_ref();
    graph.nodes.push(GraphNode {
        id: "model".to_string(),
        node_type: "puma-lib".to_string(),
        position: Position { x: 100.0, y: 100.0 },
        data: json!({}),
    });
    graph.nodes.push(GraphNode {
        id: "settings".to_string(),
        node_type: "expand-settings".to_string(),
        position: Position { x: 100.0, y: 200.0 },
        data: json!({}),
    });

    let task_graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph,
        &inference_projection(),
    )
    .expect("graph");

    let model_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "model")
        .expect("model task");
    assert_eq!(
        model_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::PumasMaterialization
    );
    assert!(model_task.schedulable_intent.is_none());

    let settings_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "settings")
        .expect("settings task");
    assert_eq!(
        settings_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::Unsupported
    );
    assert!(settings_task.schedulable_intent.is_none());
}

#[test]
fn scheduler_task_graph_projects_source_input_and_non_runtime_templates() {
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"text": "describe the image"}),
            },
            GraphNode {
                id: "flag".to_string(),
                node_type: "boolean-input".to_string(),
                position: Position { x: 0.0, y: 100.0 },
                data: json!({"value": true}),
            },
            GraphNode {
                id: "out".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: json!({"ignored": "frontend display data"}),
            },
        ],
        edges: vec![GraphEdge {
            id: "edge-prompt-out".to_string(),
            source: "prompt".to_string(),
            source_handle: "text".to_string(),
            target: "out".to_string(),
            target_handle: "text".to_string(),
        }],
        derived_graph: None,
    };

    let task_graph =
        workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).expect("graph");

    let prompt_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "prompt")
        .expect("prompt task");
    assert_eq!(
        prompt_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::SourceInput
    );
    assert!(prompt_task.non_runtime_task_template.is_none());
    assert_eq!(
        prompt_task.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".to_string()
        })
    );

    let flag_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "flag")
        .expect("flag task");
    assert_eq!(
        flag_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::SourceInput
    );
    assert!(flag_task.non_runtime_task_template.is_none());
    assert_eq!(
        flag_task.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Boolean {
            port_id: "value".to_string()
        })
    );

    let output_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "out")
        .expect("output task");
    assert_eq!(
        output_task.non_runtime_task_template,
        Some(WorkflowSchedulerNonRuntimeTaskTemplate::TextOutput)
    );
    let encoded = serde_json::to_string(&task_graph).expect("encode task graph");
    assert!(!encoded.contains("frontend display data"));
    assert!(!encoded.contains("describe the image"));
}

#[test]
fn scheduler_task_graph_ignores_source_input_graph_data() {
    let graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".to_string(),
                node_type: "text-input".to_string(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"value": "legacy text field"}),
            },
            GraphNode {
                id: "flag".to_string(),
                node_type: "boolean-input".to_string(),
                position: Position { x: 0.0, y: 100.0 },
                data: json!({"value": "true"}),
            },
            GraphNode {
                id: "out".to_string(),
                node_type: "text-output".to_string(),
                position: Position { x: 200.0, y: 0.0 },
                data: json!({}),
            },
        ],
        edges: Vec::new(),
        derived_graph: None,
    };

    let task_graph =
        workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).expect("graph");

    let prompt_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "prompt")
        .expect("prompt task");
    assert_eq!(
        prompt_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::SourceInput
    );
    assert!(prompt_task.non_runtime_task_template.is_none());
    assert_eq!(
        prompt_task.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Text {
            port_id: "text".to_string()
        })
    );
    assert!(prompt_task.diagnostics.is_empty());

    let flag_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "flag")
        .expect("flag task");
    assert_eq!(
        flag_task.execution_class,
        WorkflowSchedulerTaskExecutionClass::SourceInput
    );
    assert!(flag_task.non_runtime_task_template.is_none());
    assert_eq!(
        flag_task.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Boolean {
            port_id: "value".to_string()
        })
    );
    assert!(flag_task.diagnostics.is_empty());

    let output_task = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "out")
        .expect("output task");
    assert!(output_task.non_runtime_task_template.is_none());
    assert!(output_task.diagnostics.iter().any(|diagnostic| {
        diagnostic.code
            == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingNonRuntimeTemplateValue
            && diagnostic.port_id.as_deref() == Some("text")
    }));
}
