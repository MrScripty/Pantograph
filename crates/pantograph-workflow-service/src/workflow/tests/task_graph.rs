use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyReadinessDescriptorFingerprint,
    DependencyReadinessGraphRevision, DependencyReadinessValidationSessionId,
    DependencyRequirementsId, DependencyTaskId, DeviceIntentId, PumasModelRef, RuntimeIntentId,
};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{
    SchedulerEstimateHint, SchedulerEstimateHintKind, SchedulerNodeId,
    SchedulerRuntimeDeviceConstraints, SchedulerTraitId, SchedulerTraitSetting,
    SchedulerTraitValue,
};
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_task_graph, workflow_scheduler_task_graph_with_inference_projections,
    WorkflowSchedulerBlockedInferenceTaskProjection,
    WorkflowSchedulerBlockedInferenceTaskProjectionReason,
    WorkflowSchedulerDependencyReadinessSource, WorkflowSchedulerInferenceTaskProjection,
    WorkflowSchedulerInferenceTaskProjections, WorkflowSchedulerNonRuntimeTaskTemplate,
    WorkflowSchedulerReadyInferenceTaskProjection, WorkflowSchedulerSourceInputTemplate,
    WorkflowSchedulerTaskExecutionClass, WorkflowSchedulerTaskProjectionDiagnosticCode,
    WORKFLOW_SCHEDULER_TASK_GRAPH_SCHEMA_VERSION,
};

fn workflow_id() -> WorkflowId {
    WorkflowId::try_from("workflow-task-graph".to_string()).expect("workflow id")
}

fn workflow_run_id() -> WorkflowRunId {
    WorkflowRunId::try_from("run-task-graph".to_string()).expect("workflow run id")
}

#[test]
fn vector_output_lowers_to_optional_typed_scheduler_template() {
    for connected in [false, true] {
        let graph = WorkflowGraph {
            nodes: vec![
                GraphNode {
                    id: "source".into(),
                    node_type: "selection-input".into(),
                    position: Position { x: 0.0, y: 0.0 },
                    data: json!({}),
                },
                GraphNode {
                    id: "vector-out".into(),
                    node_type: "vector-output".into(),
                    position: Position { x: 200.0, y: 0.0 },
                    data: json!({"vector": "inert display data"}),
                },
            ],
            edges: if connected {
                vec![GraphEdge {
                    id: "source-vector".into(),
                    source: "source".into(),
                    target: "vector-out".into(),
                    source_handle: "value".into(),
                    target_handle: "vector".into(),
                }]
            } else {
                Vec::new()
            },
            derived_graph: None,
        };
        let tasks =
            workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).unwrap();
        let sink = tasks
            .tasks
            .iter()
            .find(|task| task.node_id.as_str() == "vector-out")
            .unwrap();
        assert_eq!(
            sink.execution_class,
            WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine
        );
        assert_eq!(
            sink.non_runtime_task_template,
            Some(WorkflowSchedulerNonRuntimeTaskTemplate::VectorOutput)
        );
        assert!(sink.diagnostics.is_empty());
        assert_eq!(sink.input_bindings.len(), usize::from(connected));
        if connected {
            assert_eq!(sink.input_bindings[0].source_node_id.as_str(), "source");
            assert_eq!(sink.input_bindings[0].source_port_id, "value");
            assert_eq!(sink.input_bindings[0].target_port_id, "vector");
        }
        let encoded = serde_json::to_value(&tasks).unwrap();
        let decoded: crate::workflow::WorkflowSchedulerTaskGraph =
            serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, tasks);
    }
}

fn inference_projection() -> WorkflowSchedulerInferenceTaskProjections {
    ready_inference_projection(resource_estimate_hints())
}

fn inference_projection_without_resource_estimates() -> WorkflowSchedulerInferenceTaskProjections {
    ready_inference_projection(Vec::new())
}

fn ready_inference_projection(
    estimate_hints: Vec<SchedulerEstimateHint>,
) -> WorkflowSchedulerInferenceTaskProjections {
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Ready(Box::new(
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
                runtime_source_context: runtime_source_context(),
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
                estimate_hints,
                dependency_readiness_source: dependency_readiness_source("iface.test.v1"),
            },
        )),
    ])
    .expect("projection")
}

fn runtime_source_context() -> crate::graph::WorkflowRuntimeSourceContext {
    crate::graph::WorkflowRuntimeSourceContext {
        operation_type: "image-generation.txt2img".to_string(),
        context_shape_key: "txt2img.1024x1024.steps30".to_string(),
        cancellation_mode: "per-run-fanout".to_string(),
    }
}

fn resource_estimate_hints() -> Vec<SchedulerEstimateHint> {
    vec![
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakRamBytes,
            value: 2_147_483_648,
        },
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakVramBytes,
            value: 4_294_967_296,
        },
    ]
}

fn dependency_readiness_source(
    descriptor_fingerprint: &str,
) -> WorkflowSchedulerDependencyReadinessSource {
    WorkflowSchedulerDependencyReadinessSource {
        graph_revision: DependencyReadinessGraphRevision::parse("graph.revision.001")
            .expect("graph revision"),
        validation_session_id: Some(
            DependencyReadinessValidationSessionId::parse("validation.session.001")
                .expect("validation session"),
        ),
        validation_snapshot_id: None,
        descriptor_fingerprint: DependencyReadinessDescriptorFingerprint::parse(
            descriptor_fingerprint,
        )
        .expect("descriptor fingerprint"),
        dependency_requirements_id: DependencyRequirementsId::parse(
            "requirements.image_generation.cuda0",
        )
        .expect("requirements id"),
        selected_binding_ids: vec![
            DependencyBindingId::parse("torch-diffusers").expect("binding id")
        ],
        dependency_override_fingerprint: DependencyOverrideFingerprint::parse("override.none")
            .expect("override fingerprint"),
    }
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
                    "runtime_source_context": {
                        "operation_type": "image-generation.txt2img",
                        "context_shape_key": "txt2img.1024x1024.steps30",
                        "cancellation_mode": "per-run-fanout"
                    },
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
fn ready_projection_preserves_owned_raw_record_and_compact_layout() {
    let projections = inference_projection();
    let projection = projections
        .get(&SchedulerNodeId::parse("infer").unwrap())
        .unwrap();
    let WorkflowSchedulerInferenceTaskProjection::Ready(borrowed) = projection else {
        panic!("expected ready projection");
    };
    let WorkflowSchedulerInferenceTaskProjection::Ready(owned) = projection.clone() else {
        panic!("expected cloned ready projection");
    };
    let raw: WorkflowSchedulerReadyInferenceTaskProjection = *owned;
    assert_eq!(&raw, borrowed.as_ref());
    assert!(
        std::mem::size_of::<WorkflowSchedulerInferenceTaskProjection>()
            < std::mem::size_of::<WorkflowSchedulerReadyInferenceTaskProjection>()
    );
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
    assert_eq!(intent.estimate_hints, resource_estimate_hints());
    assert_eq!(
        intent.trait_settings[0].trait_id.as_str(),
        "denoising_scheduler"
    );
    assert_eq!(
        intent.trait_settings[0].value,
        SchedulerTraitValue::String("euler_discrete".to_string())
    );

    let expected_intent = json!({
        "contract_version": 1,
        "workflow_id": "workflow-task-graph",
        "workflow_run_id": "run-task-graph",
        "node_id": "infer",
        "task_id": "infer",
        "task_type": "image_generation",
        "model_ref": {
            "model_id": "image/example/tiny-diffusion",
            "revision": "main",
            "selected_artifact_id": "diffusers-bundle"
        },
        "constraints": { "requested_runtime_id": "pytorch", "requested_device_id": "cuda:0" },
        "trait_settings": [{ "trait_id": "denoising_scheduler", "value": { "kind": "string", "value": "euler_discrete" } }],
        "estimate_hints": [
            { "kind": "peak_ram_bytes", "value": 2_147_483_648_u64 },
            { "kind": "peak_vram_bytes", "value": 4_294_967_296_u64 }
        ]
    });
    assert_eq!(serde_json::to_value(intent).unwrap(), expected_intent);
    assert_eq!(
        serde_json::from_value::<pantograph_scheduler::SchedulableTaskIntent>(expected_intent)
            .unwrap(),
        *intent
    );

    let encoded = serde_json::to_string(&graph).expect("encode task graph");
    assert!(!encoded.contains("model_path"));
    assert!(!encoded.contains("/tmp/legacy-model"));
}

#[test]
fn scheduler_task_graph_blocks_runtime_inference_without_resource_estimates() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_inline_inference_ref(),
        &inference_projection_without_resource_estimates(),
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
        Some("iface.test.v1")
    );
    assert!(inference_task.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingResourceEstimates
            && diagnostic.port_id.is_none()
            && diagnostic.message
                == "runtime inference scheduler tasks require validated resource estimate hints"
    }));
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
fn scheduler_image_output_retains_runtime_dependency_and_diagnoses_missing_image() {
    let mut graph = graph_with_inline_inference_ref();
    let task_graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph,
        &inference_projection(),
    )
    .unwrap();
    let output = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "image-output")
        .unwrap();
    assert_eq!(
        output.execution_class,
        WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine
    );
    assert_eq!(
        output.non_runtime_task_template,
        Some(WorkflowSchedulerNonRuntimeTaskTemplate::ImageOutput)
    );
    assert_eq!(
        output
            .dependency_task_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        ["infer"]
    );
    assert_eq!(output.input_bindings[0].source_port_id, "image");
    assert_eq!(
        serde_json::from_str::<crate::workflow::WorkflowSchedulerTaskGraph>(
            &serde_json::to_string(&task_graph).unwrap()
        )
        .unwrap(),
        task_graph
    );

    graph.edges.retain(|edge| edge.target != "image-output");
    let task_graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph,
        &inference_projection(),
    )
    .unwrap();
    let output = task_graph
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "image-output")
        .unwrap();
    assert!(output.non_runtime_task_template.is_none());
    assert!(output.diagnostics.iter().any(|diagnostic| diagnostic.code
        == WorkflowSchedulerTaskProjectionDiagnosticCode::MissingNonRuntimeTemplateValue));
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
        node_type: "audio-output".to_string(),
        position: Position { x: 100.0, y: 200.0 },
        data: json!({}),
    });
    graph.nodes.push(GraphNode {
        id: "deps".to_string(),
        node_type: "dependency-environment".to_string(),
        position: Position { x: 100.0, y: 300.0 },
        data: json!({"mode":"manual"}),
    });

    let task_graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph,
        &inference_projection(),
    )
    .expect("graph");
    assert!(!task_graph
        .tasks
        .iter()
        .any(|task| task.node_id.as_str() == "deps"));

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
fn scheduler_json_filter_captures_only_its_path_and_typed_selection_source() {
    let mut graph = WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "source".into(),
                node_type: "selection-input".into(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"value": "legacy source data"}),
            },
            GraphNode {
                id: "filter".into(),
                node_type: "json-filter".into(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({"path": "items[0].prompt", "label": "display data"}),
            },
        ],
        edges: vec![GraphEdge {
            id: "source-filter".into(),
            source: "source".into(),
            target: "filter".into(),
            source_handle: "value".into(),
            target_handle: "json".into(),
        }],
        derived_graph: None,
    };
    let tasks = workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).unwrap();
    let source = tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "source")
        .unwrap();
    assert_eq!(
        source.source_input_task_template,
        Some(WorkflowSchedulerSourceInputTemplate::Selection {
            port_id: "value".into()
        })
    );
    graph.nodes[1].data["path"] = json!("changed.after.submission");
    let filter = tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "filter")
        .unwrap();
    assert_eq!(
        filter.non_runtime_task_template,
        Some(WorkflowSchedulerNonRuntimeTaskTemplate::JsonFilter {
            path: "items[0].prompt".into()
        })
    );
    let encoded = serde_json::to_string(&tasks).unwrap();
    assert!(!encoded.contains("legacy source data"));
    assert!(!encoded.contains("display data"));
    assert_eq!(
        serde_json::from_str::<crate::workflow::WorkflowSchedulerTaskGraph>(&encoded).unwrap(),
        tasks
    );
    graph.nodes[1].data["path"] = json!(false);
    let invalid =
        workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).unwrap();
    let filter = invalid
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "filter")
        .unwrap();
    assert!(filter.non_runtime_task_template.is_none());
    assert_eq!(
        filter.diagnostics[0].code,
        WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidNonRuntimeTemplateValue
    );
}

#[test]
fn scheduler_task_graph_lowers_merge_with_all_dependencies_in_canonical_order() {
    let graph = WorkflowGraph {
        nodes: ["b", "a", "join"]
            .map(|id| GraphNode {
                id: id.into(),
                node_type: if id == "join" { "merge" } else { "text-input" }.into(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({}),
            })
            .to_vec(),
        edges: ["b", "a"]
            .map(|id| GraphEdge {
                id: format!("{id}-join"),
                source: id.into(),
                target: "join".into(),
                source_handle: "text".into(),
                target_handle: "inputs".into(),
            })
            .to_vec(),
        derived_graph: None,
    };
    let tasks = workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).unwrap();
    let merge = tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "join")
        .unwrap();
    assert_eq!(
        merge.execution_class,
        WorkflowSchedulerTaskExecutionClass::NonRuntimeNodeEngine
    );
    assert_eq!(
        merge.non_runtime_task_template,
        Some(WorkflowSchedulerNonRuntimeTaskTemplate::Merge)
    );
    assert_eq!(
        merge
            .dependency_task_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(merge.diagnostics.is_empty());
    let decoded: crate::workflow::WorkflowSchedulerTaskGraph =
        serde_json::from_str(&serde_json::to_string(&tasks).unwrap()).unwrap();
    assert_eq!(decoded, tasks);
}

#[test]
fn scheduler_task_graph_reports_invalid_merge_input_port() {
    let graph = WorkflowGraph {
        nodes: ["source", "join"]
            .map(|id| GraphNode {
                id: id.into(),
                node_type: if id == "join" { "merge" } else { "text-input" }.into(),
                position: Position { x: 0.0, y: 0.0 },
                data: json!({}),
            })
            .to_vec(),
        edges: vec![GraphEdge {
            id: "source-join".into(),
            source: "source".into(),
            target: "join".into(),
            source_handle: "text".into(),
            target_handle: "unexpected".into(),
        }],
        derived_graph: None,
    };
    let tasks = workflow_scheduler_task_graph(&workflow_id(), &workflow_run_id(), &graph).unwrap();
    let merge = tasks
        .tasks
        .iter()
        .find(|task| task.node_id.as_str() == "join")
        .unwrap();
    assert_eq!(merge.non_runtime_task_template, None);
    assert_eq!(merge.diagnostics.len(), 1);
    let diagnostic = &merge.diagnostics[0];
    assert_eq!(
        diagnostic.code,
        WorkflowSchedulerTaskProjectionDiagnosticCode::InvalidNonRuntimeTemplateValue
    );
    assert_eq!(diagnostic.port_id.as_deref(), Some("inputs"));
    assert_eq!(
        diagnostic.message,
        "merge accepts only bindings that target 'inputs'"
    );
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
