use pantograph_dependency_planning::{
    DependencyBindingId, DependencyOverrideFingerprint, DependencyReadinessDescriptorFingerprint,
    DependencyReadinessGraphRevision, DependencyReadinessValidationSessionId,
    DependencyRequirementsId, DependencyTaskId, PumasModelRef, RuntimeIntentId,
};
use pantograph_inference_interface_contracts::InferenceInterfaceFingerprint;
use pantograph_runtime_attribution::{WorkflowId, WorkflowRunId};
use pantograph_scheduler::{
    SchedulerEstimateHint, SchedulerEstimateHintKind, SchedulerNodeId,
    SchedulerRuntimeDeviceConstraints,
};
use serde_json::json;

use crate::graph::{GraphEdge, GraphNode, Position, WorkflowGraph};
use crate::workflow::{
    workflow_scheduler_resolve_task_intent,
    workflow_scheduler_task_graph_with_inference_projections,
    WorkflowSchedulerDependencyReadinessSource, WorkflowSchedulerInferenceTaskProjection,
    WorkflowSchedulerInferenceTaskProjections, WorkflowSchedulerReadyInferenceTaskProjection,
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

fn inference_projection() -> WorkflowSchedulerInferenceTaskProjections {
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Ready(Box::new(
            WorkflowSchedulerReadyInferenceTaskProjection {
                node_id: SchedulerNodeId::parse("infer").expect("node id"),
                descriptor_fingerprint: InferenceInterfaceFingerprint::parse("iface.binding.v1")
                    .expect("fingerprint"),
                task_type: DependencyTaskId::parse("image_generation").expect("task kind"),
                model_ref: pumas_model_ref(),
                runtime_source_context: runtime_source_context(),
                constraints: SchedulerRuntimeDeviceConstraints {
                    requested_runtime_id: Some(
                        RuntimeIntentId::parse("pytorch").expect("runtime id"),
                    ),
                    requested_device_id: None,
                },
                trait_settings: Vec::new(),
                estimate_hints: resource_estimate_hints(),
                dependency_readiness_source: dependency_readiness_source("iface.binding.v1"),
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
                    "runtime_source_context": {
                        "operation_type": "image-generation.txt2img",
                        "context_shape_key": "txt2img.1024x1024.steps30",
                        "cancellation_mode": "per-run-fanout"
                    },
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

fn graph_with_bound_model_ref_prompt_and_dependency_sidecar() -> WorkflowGraph {
    let mut graph = graph_with_bound_model_ref_and_prompt();
    graph.nodes.push(GraphNode {
        id: "dep-env".to_string(),
        node_type: "dependency-environment".to_string(),
        position: Position { x: 400.0, y: 0.0 },
        data: json!({ "mode": "manual" }),
    });
    graph.edges.push(GraphEdge {
        id: "edge-dep-env-infer".to_string(),
        source: "dep-env".to_string(),
        source_handle: "dependency_environment_sidecar".to_string(),
        target: "infer".to_string(),
        target_handle: "dependency_environment_sidecar".to_string(),
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
fn sidecar_association_is_not_materialized_as_scheduler_input() {
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &graph_with_bound_model_ref_prompt_and_dependency_sidecar(),
        &inference_projection(),
    )
    .expect("scheduler task graph");
    assert!(!graph
        .tasks
        .iter()
        .any(|task| task.task_id.as_str() == "dep-env"));
    let inference_task = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("inference task");

    assert!(!inference_task
        .dependency_task_ids
        .iter()
        .any(|task_id| task_id.as_str() == "dep-env"));
    assert!(!inference_task.input_bindings.iter().any(|binding| {
        binding.source_port_id == "dependency_environment_sidecar"
            || binding.target_port_id == "dependency_environment_sidecar"
    }));

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

fn dependency_control_embedding_graph() -> WorkflowGraph {
    let mut graph = graph_with_bound_model_ref_prompt_and_dependency_sidecar();
    graph.nodes.retain(|node| node.id != "model-selector");
    graph.edges.retain(|edge| edge.source != "model-selector");
    let infer = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "infer")
        .expect("infer");
    infer.data = json!({"task_kind": "embedding", "pumas_model_ref": pumas_model_ref()});
    graph
        .edges
        .iter_mut()
        .find(|edge| edge.target == "infer" && edge.source == "prompt")
        .expect("text edge")
        .target_handle = "text".to_string();
    graph.nodes.push(GraphNode {
        id: "vectors".to_string(),
        node_type: "vector-output".to_string(),
        position: Position { x: 600.0, y: 0.0 },
        data: json!({}),
    });
    graph.edges.push(GraphEdge {
        id: "infer-vectors".to_string(),
        source: "infer".to_string(),
        source_handle: "embedding".to_string(),
        target: "vectors".to_string(),
        target_handle: "vector".to_string(),
    });
    graph
}

fn embedding_projection() -> WorkflowSchedulerInferenceTaskProjections {
    let original = inference_projection();
    let Some(WorkflowSchedulerInferenceTaskProjection::Ready(projection)) =
        original.get(&SchedulerNodeId::parse("infer").expect("node id"))
    else {
        panic!("ready projection")
    };
    let mut projection = (**projection).clone();
    projection.task_type = DependencyTaskId::parse("embedding").expect("task kind");
    projection.constraints.requested_runtime_id =
        Some(RuntimeIntentId::parse("candle").expect("runtime"));
    projection.constraints.requested_device_id = Some("cpu".parse().expect("device"));
    projection.runtime_source_context = crate::graph::WorkflowRuntimeSourceContext {
        operation_type: "embedding.text".to_string(),
        context_shape_key: "embedding.one-text".to_string(),
        cancellation_mode: "run_scoped".to_string(),
    };
    WorkflowSchedulerInferenceTaskProjections::from_records(vec![
        WorkflowSchedulerInferenceTaskProjection::Ready(Box::new(projection)),
    ])
    .expect("embedding projection")
}

#[test]
fn control_only_dependency_sidecar_projects_only_data_tasks_without_changing_graph() {
    let source = dependency_control_embedding_graph();
    let before = source.clone();
    let fingerprint = source.compute_fingerprint();
    let topology = crate::graph::workflow_executable_topology(&source).expect("topology");
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &source,
        &embedding_projection(),
    )
    .expect("task graph");
    assert_eq!(
        graph
            .tasks
            .iter()
            .map(|task| task.task_id.as_str())
            .collect::<Vec<_>>(),
        vec!["infer", "prompt", "vectors"]
    );
    let infer = graph
        .tasks
        .iter()
        .find(|task| task.task_id.as_str() == "infer")
        .expect("infer task");
    assert_eq!(
        infer
            .dependency_task_ids
            .iter()
            .map(|id| id.as_str())
            .collect::<Vec<_>>(),
        vec!["prompt"]
    );
    assert_eq!(
        infer
            .schedulable_intent_template
            .as_ref()
            .expect("intent template")
            .dependency_readiness_source,
        dependency_readiness_source("iface.binding.v1")
    );
    let service = crate::workflow::WorkflowService::new();
    let records = service
        .scheduler_task_orchestrator
        .initial_task_state_records(&graph)
        .expect("states");
    assert!(
        records
            .iter()
            .all(|record| record.state.kind()
                != pantograph_scheduler::SchedulerTaskStateKind::Invalid)
    );
    assert_eq!(
        records
            .iter()
            .find(|record| record.task_id.as_str() == "infer")
            .expect("infer state")
            .state
            .kind(),
        pantograph_scheduler::SchedulerTaskStateKind::AwaitingInputs
    );
    assert_eq!(source, before);
    assert_eq!(source.compute_fingerprint(), fingerprint);
    assert_eq!(
        crate::graph::workflow_executable_topology(&source).expect("topology"),
        topology
    );
}

#[tokio::test]
async fn native_control_graph_requires_request_text_then_waits_for_dependency_readiness() {
    use crate::workflow::session_scheduler_runner::WorkflowPreDispatchPreparationBoundary;
    use crate::workflow::{
        WorkflowExecutionSessionRunRequest, WorkflowPortBinding, WorkflowService,
    };
    use pantograph_scheduler::SchedulerTaskStateKind;

    let mut source = dependency_control_embedding_graph();
    source
        .nodes
        .iter_mut()
        .find(|node| node.id == "prompt")
        .unwrap()
        .data = json!({"text": "hello world"});
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &source,
        &embedding_projection(),
    )
    .expect("projected native graph");
    // Every projected dependency names a retained execution task; control metadata
    // is not the missing prerequisite in this second native failure.
    for task in &graph.tasks {
        for dependency in &task.dependency_task_ids {
            assert!(graph
                .tasks
                .iter()
                .any(|upstream| upstream.task_id == *dependency));
        }
        assert!(task
            .input_bindings
            .iter()
            .all(|binding| binding.source_task_id.as_str() != "dep-env"));
    }
    let service = WorkflowService::new();
    let run_id = workflow_run_id().as_str().to_owned();
    let session_id = {
        let mut store = service.session_store_guard().expect("store");
        let session_id = store
            .create_session(
                workflow_id().as_str().to_owned(),
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
            )
            .expect("session");
        store
            .enqueue_run_with_id(
                &session_id,
                &WorkflowExecutionSessionRunRequest {
                    session_id: session_id.clone(),
                    workflow_semantic_version: "0.1.0".into(),
                    inputs: Vec::new(),
                    output_targets: None,
                    override_selection: None,
                    timeout_ms: None,
                    priority: None,
                },
                run_id.clone(),
            )
            .expect("enqueue");
        store
            .begin_queued_run(&session_id, &run_id)
            .expect("begin")
            .expect("active run");
        service
            .scheduler_task_orchestrator
            .initialize_active_run_task_state(&mut store, &session_id, &run_id, graph)
            .expect("initial task state");
        session_id
    };
    let boundary = WorkflowPreDispatchPreparationBoundary::new(&service);
    boundary
        .materialize_external_inputs(&session_id, &run_id, &[])
        .expect("empty inputs");
    let error = boundary
        .prepare_runtime_dispatch(&session_id, &run_id)
        .await
        .err()
        .expect("empty GUI inputs reproduce the native readiness guard");
    assert!(error
        .to_string()
        .contains("runtime scheduler graph has no ready task and is not complete"));
    {
        let mut store = service.session_store_guard().expect("store");
        let (_, states) = store
            .active_run_scheduler_task_state(&session_id, &run_id)
            .expect("states")
            .expect("active graph");
        assert!(states
            .iter()
            .all(|state| state.state.kind() == SchedulerTaskStateKind::AwaitingInputs));
        assert!(store
            .active_run_scheduler_task_results(&session_id, &run_id)
            .expect("results")
            .is_empty());
    }
    let wrong = WorkflowPortBinding {
        node_id: "prompt".into(),
        port_id: "text".into(),
        value: json!(7),
    };
    assert!(boundary
        .materialize_external_inputs(&session_id, &run_id, &[wrong])
        .is_err());
    boundary
        .materialize_external_inputs(
            &session_id,
            &run_id,
            &[WorkflowPortBinding {
                node_id: "prompt".into(),
                port_id: "text".into(),
                value: json!("hello world"),
            }],
        )
        .expect("typed Submit input");
    boundary
        .run_progress_loop(&session_id, &run_id)
        .await
        .expect("progress");
    let mut store = service.session_store_guard().expect("store");
    let (_, states) = store
        .active_run_scheduler_task_state(&session_id, &run_id)
        .expect("states")
        .expect("active graph");
    for (id, expected) in [
        ("prompt", SchedulerTaskStateKind::Completed),
        ("infer", SchedulerTaskStateKind::WaitingDependencyReadiness),
        ("vectors", SchedulerTaskStateKind::AwaitingInputs),
    ] {
        assert_eq!(
            states
                .iter()
                .find(|state| state.task_id.as_str() == id)
                .expect("state")
                .state
                .kind(),
            expected
        );
    }
    let results = store
        .active_run_scheduler_task_results(&session_id, &run_id)
        .expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].workflow_run_id, run_id);
    assert_eq!(results[0].node_id, "prompt");
    assert_eq!(
        results[0].outputs[0].value,
        WorkflowSchedulerTaskResultValue::String("hello world".into())
    );
}

#[test]
fn malformed_or_dataflow_bound_dependency_controls_remain_invalid_at_initialization() {
    for case in [
        "missing",
        "duplicate",
        "wrong_source",
        "wrong_target",
        "non_inference",
        "dangling",
        "incoming_data",
        "outgoing_data",
    ] {
        let mut source = dependency_control_embedding_graph();
        let sidecar_index = source
            .edges
            .iter()
            .position(|edge| edge.source == "dep-env")
            .expect("sidecar");
        match case {
            "missing" => {
                source.edges.remove(sidecar_index);
            }
            "duplicate" => {
                let mut duplicate = source.edges[sidecar_index].clone();
                duplicate.id = "duplicate".into();
                source.edges.push(duplicate);
            }
            "wrong_source" => source.edges[sidecar_index].source_handle = "mode".into(),
            "wrong_target" => source.edges[sidecar_index].target_handle = "text".into(),
            "non_inference" => source.edges[sidecar_index].target = "prompt".into(),
            "dangling" => source.edges[sidecar_index].target = "missing-infer".into(),
            "incoming_data" => source.edges.push(GraphEdge {
                id: "data-in".into(),
                source: "prompt".into(),
                source_handle: "text".into(),
                target: "dep-env".into(),
                target_handle: "mode".into(),
            }),
            "outgoing_data" => source.edges.push(GraphEdge {
                id: "data-out".into(),
                source: "dep-env".into(),
                source_handle: "value".into(),
                target: "vectors".into(),
                target_handle: "vector".into(),
            }),
            _ => unreachable!(),
        }
        let graph = workflow_scheduler_task_graph_with_inference_projections(
            &workflow_id(),
            &workflow_run_id(),
            &source,
            &embedding_projection(),
        )
        .expect("task graph");
        let control = graph
            .tasks
            .iter()
            .find(|task| task.task_id.as_str() == "dep-env")
            .expect("unsupported control retained");
        assert_eq!(
            control.execution_class,
            crate::workflow::WorkflowSchedulerTaskExecutionClass::Unsupported,
            "{case}"
        );
        let service = crate::workflow::WorkflowService::new();
        let states = service
            .scheduler_task_orchestrator
            .initial_task_state_records(&graph)
            .expect("states");
        let control = states
            .iter()
            .find(|record| record.task_id.as_str() == "dep-env")
            .expect("control state");
        assert_eq!(
            control.state.kind(),
            pantograph_scheduler::SchedulerTaskStateKind::Invalid,
            "{case}"
        );
        assert_eq!(control.state_version, 1);
        assert_eq!(control.last_transition_id.as_str(), "initial:dep-env");
    }
}

#[test]
fn control_projection_preserves_missing_inference_facts_and_other_unsupported_failures() {
    let mut source = dependency_control_embedding_graph();
    source.nodes.push(GraphNode {
        id: "unsupported".into(),
        node_type: "model-provider".into(),
        position: Position { x: 0.0, y: 0.0 },
        data: json!({}),
    });
    let graph = workflow_scheduler_task_graph_with_inference_projections(
        &workflow_id(),
        &workflow_run_id(),
        &source,
        &WorkflowSchedulerInferenceTaskProjections::default(),
    )
    .expect("task graph");
    assert!(!graph
        .tasks
        .iter()
        .any(|task| task.task_id.as_str() == "dep-env"));
    let service = crate::workflow::WorkflowService::new();
    let states = service
        .scheduler_task_orchestrator
        .initial_task_state_records(&graph)
        .expect("states");
    for id in ["infer", "unsupported"] {
        assert_eq!(
            states
                .iter()
                .find(|record| record.task_id.as_str() == id)
                .expect("retained state")
                .state
                .kind(),
            pantograph_scheduler::SchedulerTaskStateKind::Invalid,
            "{id}"
        );
    }
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
