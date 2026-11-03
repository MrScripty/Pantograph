use super::*;
use inference::ResolvedModelPackageFacts;
use pantograph_inference_interface_contracts::InferenceInterfaceDescriptor;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionCancellationHandle, RuntimeHostExecutionInputValue,
    RuntimeHostExecutionPort, RuntimeHostExecutionRequest, RuntimeHostExecutionState,
    ValidatedRuntimeHostExecutionRequest,
};
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, WorkflowGraphLoadRequest, WorkflowGraphSaveRequest,
    WorkflowRunHandle,
};

// Expose every authored source port; no inference implementation lives in this host.
struct ChatSessionHost(ImageRuntimeSessionHost);
#[async_trait]
impl WorkflowHost for ChatSessionHost {
    async fn validate_workflow(&self, id: &str) -> Result<(), WorkflowServiceError> {
        self.0.validate_workflow(id).await
    }
    async fn workflow_graph_fingerprint(&self, id: &str) -> Result<String, WorkflowServiceError> {
        self.0.workflow_graph_fingerprint(id).await
    }
    async fn workflow_graph(&self, id: &str) -> Result<WorkflowGraph, WorkflowServiceError> {
        self.0.workflow_graph(id).await
    }
    async fn workflow_capabilities(
        &self,
        id: &str,
    ) -> Result<WorkflowHostCapabilities, WorkflowServiceError> {
        let mut limits = self.0.workflow_capabilities(id).await?;
        limits.max_input_bindings = 16;
        Ok(limits)
    }
    async fn workflow_io(&self, _: &str) -> Result<WorkflowIoResponse, WorkflowServiceError> {
        let io_node = |node: &GraphNode, port: &str, kind: &str| WorkflowIoNode {
            node_id: node.id.clone(),
            node_type: node.node_type.clone(),
            name: None,
            description: None,
            ports: vec![WorkflowIoPort {
                port_id: port.into(),
                name: None,
                description: None,
                data_type: Some(kind.into()),
                required: Some(true),
                multiple: Some(false),
            }],
        };
        Ok(WorkflowIoResponse {
            inputs: self
                .0
                .graph
                .nodes
                .iter()
                .filter_map(|node| match node.node_type.as_str() {
                    "text-input" => Some(io_node(node, "text", "string")),
                    "number-input" => Some(io_node(node, "value", "number")),
                    "selection-input" => Some(io_node(node, "value", "any")),
                    _ => None,
                })
                .collect(),
            outputs: self
                .0
                .graph
                .nodes
                .iter()
                .filter(|node| matches!(node.node_type.as_str(), "llm-inference" | "text-output"))
                .map(|node| io_node(node, "text", "string"))
                .collect(),
        })
    }
    async fn run_workflow(
        &self,
        _: &str,
        _: &[WorkflowPortBinding],
        _: Option<&[WorkflowOutputTarget]>,
        _: WorkflowRunOptions,
        _: WorkflowRunHandle,
    ) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
        panic!("public scheduler must use selected native owner, not alternate host execution")
    }
    async fn load_session_runtime(
        &self,
        _: &str,
        _: &str,
        _: Option<&str>,
        _: WorkflowExecutionSessionRetentionHint,
    ) -> Result<(), WorkflowServiceError> {
        panic!("public scheduler must use selected native loader, not alternate host loading")
    }
}

struct Target(pumas_library::models::PumasArtifactLoadTarget);
#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for Target {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        Ok(self.0.clone())
    }
}
struct Package(ResolvedModelPackageFacts);
#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for Package {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        Ok(self.0.clone())
    }
}
struct UnusedMediaSink;
impl crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSink for UnusedMediaSink {
    fn write_image_output(
        &self,
        _: crate::runtime_host_media_artifact_sink::RuntimeHostImageArtifactWriteRequest<'_>,
    ) -> Result<
        pantograph_runtime_host_contracts::RuntimeHostExecutionMediaArtifactRef,
        crate::runtime_host_media_artifact_sink::RuntimeHostMediaArtifactSinkError,
    > {
        panic!("text output must not require a media artifact");
    }
}

fn fixture(
    model_seed: u32,
    task: &str,
) -> (
    TempDir,
    RuntimeHostExecutionRequest,
    inference::ResolvedModelPackageFacts,
    inference::PumasArtifactLoadTarget,
) {
    let directory = TempDir::new().unwrap();
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../inference/tests/fixtures/tiny_chat_gpt2/model-{model_seed}"
    ));
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), directory.path().join(entry.file_name())).unwrap();
    }
    let mut package: serde_json::Value = serde_json::from_str(include_str!("../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json")).unwrap();
    let reference = serde_json::json!({"model_id":format!("chat/synthetic/GPT2-{model_seed}"), "revision":format!("untrained-seed-{model_seed}"), "selected_artifact_id":"main"});
    package["model_ref"] = reference.clone();
    package["artifact"]["entry_path"] = reference["model_id"].clone();
    package["transformers"] = serde_json::json!({"config_status":"present", "config_model_type":"gpt2", "architectures":["GPT2LMHeadModel"], "torch_dtype":"float32", "auto_map":[], "generation_config_status":"present"});
    package["custom_code"] = serde_json::json!({"requires_custom_code":false,"custom_code_sources":[],"auto_map_sources":[],"class_references":[],"dependency_manifests":[]});
    package["task"] = serde_json::json!({"task_type_primary":task,"input_modalities":["text"],"output_modalities":["text"]});
    package["generation_defaults"] =
        serde_json::json!({"status":"present", "source_path":"generation_config.json"});
    let package: inference::ResolvedModelPackageFacts = serde_json::from_value(package).unwrap();
    let mut request: serde_json::Value = serde_json::from_str(include_str!("../../../pantograph-runtime-host-contracts/tests/fixtures/runtime_host_execution_request_dispatch_selected.json")).unwrap();
    request["materialized_inputs"] = serde_json::json!([
        {"port_id":"prompt", "value":{"value_type":"string","value":"hello world"}},
        {"port_id":"system_prompt", "value":{"value_type":"string","value":"be brief"}},
        {"port_id":"max_new_tokens", "value":{"value_type":"u64","value":4}},
        {"port_id":"min_new_tokens", "value":{"value_type":"u64","value":4}},
        {"port_id":"temperature", "value":{"value_type":"f64","value":0.8}},
        {"port_id":"top_p", "value":{"value_type":"f64","value":1.0}},
        {"port_id":"top_k", "value":{"value_type":"u64","value":4}},
        {"port_id":"repetition_penalty", "value":{"value_type":"f64","value":1.0}},
        {"port_id":"seed", "value":{"value_type":"u64","value":42}},
        {"port_id":"stop", "value":{"value_type":"string","value":"NEVER_STOP"}}
    ]);
    for pointer in [
        "/handoff/task_intent",
        "/handoff/dispatch_decision/task_intent",
    ] {
        let intent = request.pointer_mut(pointer).unwrap();
        intent["task_type"] = serde_json::json!(task);
        intent["model_ref"] = reference.clone();
        intent["constraints"] =
            serde_json::json!({"requested_runtime_id":"pytorch","requested_device_id":"cpu"});
        intent["trait_settings"] = serde_json::json!([]);
    }
    for pointer in [
        "/handoff/readiness_proof/preflight_result/identity_key",
        "/handoff/dispatch_decision/readiness_proof/preflight_result/identity_key",
    ] {
        let key = request.pointer_mut(pointer).unwrap();
        key["task_id"] = serde_json::json!(task);
        key["model_ref"] = reference.clone();
        key["scheduler_intent"] =
            serde_json::json!({"requested_runtime_id":"pytorch","requested_device_id":"cpu"});
    }
    let decision = &mut request["handoff"]["dispatch_decision"];
    decision["selected_model_ref"] = reference.clone();
    decision["selected_runtime_id"] = serde_json::json!("pytorch");
    decision["selected_runtime_variant_id"] = serde_json::json!("pytorch.cpu");
    decision["selected_device_ids"] = serde_json::json!(["cpu"]);
    for reservation in decision["reservations"].as_array_mut().unwrap() {
        reservation["device_id"] = serde_json::json!("cpu");
    }
    let request: RuntimeHostExecutionRequest = serde_json::from_value(request).unwrap();
    request.validate().unwrap();
    let target = inference::PumasArtifactLoadTarget {
        model_ref: serde_json::from_value(reference).unwrap(),
        artifact_kind: inference::ModelArtifactKind::HfCompatibleDirectory,
        local_load_path: directory.path().to_str().unwrap().into(),
        load_path_kind: inference::PumasArtifactLoadPathKind::Directory,
        library_root_id: Some("synthetic-chat-root".into()),
        storage_kind: inference::ModelStorageKind::LibraryOwned,
        validation_state: inference::ModelValidationState::Valid,
        content_fingerprint: None,
        package_facts_contract_version: Some(package.package_facts_contract_version),
    };
    (directory, request, package, target)
}

fn graph(model_ref: &PumasModelRef, task: &str) -> WorkflowGraph {
    let mut descriptor: serde_json::Value=serde_json::from_str(include_str!("../../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_embedding_ready.json")).unwrap();
    descriptor["model_ref"] = serde_json::to_value(model_ref).unwrap();
    descriptor["task_kind"] = serde_json::json!(task);
    descriptor["descriptor_fingerprint"] = serde_json::json!("iface.test.cpu_chat.v1");
    descriptor["inputs"]=serde_json::from_str(include_str!("../../../pantograph-inference-interface-contracts/tests/fixtures/text_generation_system_prompt_inputs.json")).unwrap();
    descriptor["outputs"] = serde_json::json!([{"port_id":"text","label":"Text","direction":"output","requirement":"required","value_type":{"category":"scalar","kind":"string"},"options":{"kind":"none"},"availability":{"status":"available"}}]);
    let descriptor: InferenceInterfaceDescriptor = serde_json::from_value(descriptor).unwrap();
    descriptor.validate().unwrap();
    let snapshot =
        pantograph_workflow_service::graph::authored_snapshot_from_descriptor(&descriptor).unwrap();
    let mut nodes = vec![
        GraphNode {
            id: "prompt".into(),
            node_type: "text-input".into(),
            position: Position { x: 0.0, y: 0.0 },
            data: serde_json::json!({}),
        },
        GraphNode {
            id: "system".into(),
            node_type: "text-input".into(),
            position: Position { x: 0.0, y: 100.0 },
            data: serde_json::json!({}),
        },
        GraphNode {
            id: "infer".into(),
            node_type: "llm-inference".into(),
            position: Position { x: 200.0, y: 0.0 },
            data: serde_json::json!({"task_kind":task,"runtime":"pytorch","device":"cpu","pumas_model_ref":model_ref,"inference_interface_snapshot":snapshot}),
        },
        GraphNode {
            id: "output".into(),
            node_type: "text-output".into(),
            position: Position { x: 400.0, y: 0.0 },
            data: serde_json::json!({}),
        },
        GraphNode {
            id: "stop".into(),
            node_type: "text-input".into(),
            position: Position { x: 0.0, y: 200.0 },
            data: serde_json::json!({"text":"NEVER_STOP"}),
        },
    ];
    let mut edges = vec![
        GraphEdge {
            id: "prompt-edge".into(),
            source: "prompt".into(),
            source_handle: "text".into(),
            target: "infer".into(),
            target_handle: "prompt".into(),
        },
        GraphEdge {
            id: "system-edge".into(),
            source: "system".into(),
            source_handle: "text".into(),
            target: "infer".into(),
            target_handle: "system_prompt".into(),
        },
        GraphEdge {
            id: "stop-edge".into(),
            source: "stop".into(),
            source_handle: "text".into(),
            target: "infer".into(),
            target_handle: "stop".into(),
        },
        GraphEdge {
            id: "output-edge".into(),
            source: "infer".into(),
            source_handle: "text".into(),
            target: "output".into(),
            target_handle: "text".into(),
        },
    ];
    for (port, value) in [
        ("max_new_tokens", serde_json::json!(4)),
        ("min_new_tokens", serde_json::json!(4)),
        ("temperature", serde_json::json!(0.8)),
        ("top_p", serde_json::json!(1.0)),
        ("top_k", serde_json::json!(4)),
        ("repetition_penalty", serde_json::json!(1.0)),
        ("seed", serde_json::json!(42)),
    ] {
        nodes.push(GraphNode {
            id: port.into(),
            node_type: if value.is_f64() {
                "selection-input"
            } else {
                "number-input"
            }
            .into(),
            position: Position { x: 0.0, y: 300.0 },
            data: serde_json::json!({"value":value}),
        });
        edges.push(GraphEdge {
            id: format!("{port}-edge"),
            source: port.into(),
            source_handle: "value".into(),
            target: "infer".into(),
            target_handle: port.into(),
        });
    }
    WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    }
}

fn install_chat_readiness(
    service: &WorkflowService,
    provider: &DependencyEnvironmentReadinessSnapshotProvider,
    graph: &WorkflowGraph,
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
    task: &str,
) {
    // Only package/readiness/dispatch facts are controlled. Inference below uses
    // the production selected-model owner and committed untrained GPT-2 CPU model.
    let mut snapshot =
        image_runtime_validation_snapshot(version, graph, &model_ref.model_id, "fixture");
    let node = &mut snapshot.nodes[0];
    node.task_kind = InferenceTaskKind::parse(task).unwrap();
    node.model_ref = model_ref.clone();
    node.descriptor_fingerprint =
        InferenceInterfaceFingerprint::parse("iface.test.cpu_chat.v1").unwrap();
    node.constraints.requested_runtime_id = Some(RuntimeIntentId::parse("pytorch").unwrap());
    node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    node.runtime_source_context.operation_type = format!("{task}.prompt");
    node.runtime_source_context.context_shape_key = "chat.single-prompt".into();
    let mut planning = image_runtime_dependency_planning_request(
        version,
        model_ref,
        vec![DependencyBindingId::parse("pytorch-chat").unwrap()],
    );
    planning.task_id = DependencyTaskId::parse(task).unwrap();
    planning.task_type = None;
    planning.scheduler_intent.requested_runtime_id =
        Some(RuntimeIntentId::parse("pytorch").unwrap());
    planning.scheduler_intent.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    let proof = produce_dependency_requirements_proof(
        &ValidatedDependencyPlanningRequest::try_from(planning.clone()).unwrap(),
        None,
    )
    .unwrap();
    node.dependency_requirements_id = proof.dependency_requirements_id.clone();
    node.selected_binding_ids = proof.identity_key.selected_binding_ids.clone();
    node.dependency_override_fingerprint = proof.dependency_override_fingerprint.clone();
    let request = ValidatedDependencyEnvironmentRequest::try_from(DependencyEnvironmentRequest {
        contract_version: 1,
        action: DependencyEnvironmentAction::Resolve,
        identity_key: proof.identity_key,
        planning_request: planning,
        dependency_requirements_id: Some(proof.dependency_requirements_id),
        environment_ref: None,
    })
    .unwrap();
    let mut result = ready_image_dependency_environment_result(&request);
    result.requirements = serde_json::from_value(serde_json::json!([{
        "name": "pytorch-chat", "kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "pytorch", "feature_id": task, "runtime_variant_id": "pytorch.cpu"}
    }])).unwrap();
    result.bindings = serde_json::from_value(serde_json::json!([{
        "binding_id": "pytorch-chat", "requirement_name": "pytorch-chat", "environment_kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "pytorch", "feature_id": task, "runtime_variant_id": "pytorch.cpu"}
    }])).unwrap();
    provider
        .insert_snapshot(
            DependencyEnvironmentReadinessSnapshot::for_request(
                &request,
                result,
                DependencyEnvironmentReadinessSnapshotStatus::Fresh,
            )
            .unwrap(),
        )
        .unwrap();
    service
        .store_workflow_executable_validation_snapshot(snapshot)
        .unwrap();
}

#[test]
fn chat_projection_preserves_canonical_task_controls_and_original_revision() {
    let (_directory, request, package, target) = fixture(11, "chat_completion");
    let validated = ValidatedRuntimeHostExecutionRequest::try_from(request).unwrap();
    let projection = crate::runtime_host_text_execution::project_runtime_host_text_generation(
        &validated,
        package,
        serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        projection.request().task_id,
        inference::InferenceTaskId::ChatCompletion
    );
    assert_eq!(
        projection.backend_decision().selected_task_id,
        Some(inference::InferenceTaskId::ChatCompletion)
    );
    let options = projection.request().generation_options.as_ref().unwrap();
    assert_eq!(options.sampling.seed, Some(42));
    assert_eq!(options.length.max_new_tokens, Some(4));
    assert_eq!(options.stopping.stop_strings, ["NEVER_STOP"]);
}

fn requested_revision(request: &mut RuntimeHostExecutionRequest, revision: Option<&str>) {
    request.handoff.task_intent.model_ref.revision = revision.map(str::to_owned);
    request
        .handoff
        .readiness_proof
        .preflight_result
        .identity_key
        .model_ref
        .revision = revision.map(str::to_owned);
    let decision = request.handoff.dispatch_decision.as_mut().unwrap();
    decision.task_intent = request.handoff.task_intent.clone();
    decision.readiness_proof = request.handoff.readiness_proof.clone();
    request.validate().unwrap();
}

struct NeverResolve;
#[async_trait]
impl crate::runtime_host_load_target::RuntimeHostLoadTargetResolver for NeverResolve {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        pumas_library::models::PumasArtifactLoadTarget,
        crate::runtime_host_load_target::RuntimeHostPumasLoadTargetError,
    > {
        panic!("invalid chat must reject before target resolution")
    }
}
#[async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for NeverResolve {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        inference::ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        panic!("invalid chat must reject before package resolution")
    }
}
struct Cancelled(String);
impl pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSignal for Cancelled {
    fn snapshot(
        &self,
    ) -> pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
        pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot{cancellation_context_id:self.0.clone(),state:pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState::CancellationRequested,reason:Some("chat fixture precancellation".into())}
    }
}

#[tokio::test]
async fn canonical_chat_rejects_invalid_shape_revision_and_precancellation_before_resolvers() {
    for fault in [
        "revision",
        "missing_selected_revision",
        "blank",
        "messages",
        "cancel",
    ] {
        let (_directory, mut request, _package, _target) = fixture(11, "chat_completion");
        match fault {
            "revision" => requested_revision(&mut request, Some("different-requested-revision")),
            "missing_selected_revision" => {
                request
                    .handoff
                    .dispatch_decision
                    .as_mut()
                    .unwrap()
                    .selected_model_ref
                    .revision = None
            }
            "blank" => {
                request.materialized_inputs[0].value =
                    pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue::String(
                        " ".into(),
                    )
            }
            "messages" => request.materialized_inputs[0].port_id = "messages".into(),
            _ => {}
        }
        request.validate().unwrap();
        let gateway = Arc::new(inference::InferenceGateway::with_backend(
            Box::new(inference::backend::pytorch::PyTorchBackend::new()),
            "PyTorch",
        ));
        let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(NeverResolve),
            Arc::new(NeverResolve),
            Arc::new(UnusedMediaSink),
            gateway.clone(),
        );
        let cancellation = if fault == "cancel" {
            RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Cancelled(
                request.cancellation_context.cancellation_context_id.clone(),
            )))
        } else {
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone())
        };
        let response = port
            .execute_runtime_host_request(request, cancellation)
            .await
            .unwrap();
        response.validate().unwrap();
        assert_eq!(response.state, RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
        assert!(!gateway.is_ready().await);
    }
}

#[tokio::test]
async fn canonical_chat_package_task_mismatch_refuses_before_native_loading() {
    let (_directory, request, mut package, target) = fixture(11, "chat_completion");
    package.task.task_type_primary = Some("text_generation".into());
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::pytorch::PyTorchBackend::new()),
        "PyTorch",
    ));
    let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        Arc::new(Target(
            serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
        )),
        Arc::new(Package(package)),
        Arc::new(UnusedMediaSink),
        gateway.clone(),
    );
    let response = port
        .execute_runtime_host_request(
            request.clone(),
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context),
        )
        .await
        .unwrap();
    response.validate().unwrap();
    assert_eq!(response.state, RuntimeHostExecutionState::Failed);
    assert!(response.outputs.is_empty());
    assert!(!gateway.is_ready().await);
}

#[test]
fn canonical_chat_projection_preserves_omitted_revision_refinement_and_single_prompt_shape() {
    let (_directory, mut request, package, target) = fixture(11, "chat_completion");
    requested_revision(&mut request, None);
    let validated = ValidatedRuntimeHostExecutionRequest::try_from(request).unwrap();
    let projection = crate::runtime_host_text_execution::project_runtime_host_text_generation(
        &validated,
        package,
        serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        projection.request().model_ref.as_ref().unwrap().revision,
        None
    );
    assert_eq!(
        projection
            .backend_decision()
            .selected_model_ref
            .as_ref()
            .unwrap()
            .revision
            .as_deref(),
        Some("untrained-seed-11")
    );
    assert!(
        matches!(&projection.request().input,inference::InferenceExecutionInput::TextGeneration{prompt:Some(prompt),system_prompt:Some(system),messages,stream:false} if prompt=="hello world" && system=="be brief" && messages.is_empty())
    );
}

fn chat_batch_request(
    request: RuntimeHostExecutionRequest,
) -> pantograph_runtime_host_contracts::RuntimeHostBatchExecutionRequest {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionMemberRequest, RuntimeHostBatchExecutionRequest,
        RuntimeHostBatchMemberFailurePolicy, RuntimeHostBatchMemberReservationPolicy,
    };
    let members = ["hello world", "hello"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| {
            let mut request = request.clone();
            request.execution_request_id = format!("chat.execution.{index}");
            request.handoff.workflow_id = format!("workflow.chat.{index}").parse().unwrap();
            request.handoff.workflow_run_id = format!("run.chat.{index}").parse().unwrap();
            request.handoff.node_id = format!("node.chat.{index}").parse().unwrap();
            request.handoff.task_id = format!("task.chat.{index}").parse().unwrap();
            request.handoff.task_intent.workflow_id = request.handoff.workflow_id.clone();
            request.handoff.task_intent.workflow_run_id = request.handoff.workflow_run_id.clone();
            request.handoff.task_intent.node_id = request.handoff.node_id.clone();
            request.handoff.task_intent.task_id = request.handoff.task_id.clone();
            let context = &mut request.handoff.readiness_proof.execution_context;
            context.workflow_id = request.handoff.workflow_id.as_str().parse().unwrap();
            context.workflow_run_id = request.handoff.workflow_run_id.as_str().parse().unwrap();
            context.node_id = request.handoff.node_id.as_str().parse().unwrap();
            context.scheduler_task_id = request.handoff.task_id.as_str().parse().unwrap();
            let decision = request.handoff.dispatch_decision.as_mut().unwrap();
            decision.workflow_id = request.handoff.workflow_id.clone();
            decision.workflow_run_id = request.handoff.workflow_run_id.clone();
            decision.node_id = request.handoff.node_id.clone();
            decision.task_id = request.handoff.task_id.clone();
            decision.task_intent = request.handoff.task_intent.clone();
            decision.readiness_proof = request.handoff.readiness_proof.clone();
            for reservation in &mut decision.reservations {
                reservation.workflow_run_id = decision.workflow_run_id.clone();
                reservation.task_id = decision.task_id.clone();
            }
            request.materialized_inputs[0].value =
                RuntimeHostExecutionInputValue::String(text.into());
            RuntimeHostBatchExecutionMemberRequest {
                execution_request_id: request.execution_request_id,
                assignment_id: format!("assignment.chat.{index}"),
                handoff: request.handoff,
                materialized_inputs: request.materialized_inputs,
                timeout_ms: None,
                failure_policy: RuntimeHostBatchMemberFailurePolicy::TerminalOnly,
                reservation_policy: RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal,
            }
        })
        .collect();
    let batch = RuntimeHostBatchExecutionRequest {
        contract_version: pantograph_runtime_host_contracts::RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: "chat.batch.001".into(), anchor_execution_request_id: "chat.execution.0".into(),
        cancellation_context: pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationContext::workflow_service("chat.batch.001"),
        members,
    };
    batch.validate().unwrap();
    batch
}

#[tokio::test]
async fn canonical_chat_envelope_revision_mismatches_reject_before_resolvers() {
    use pantograph_runtime_host_contracts::{
        RuntimeHostBatchExecutionPort, RuntimeHostBatchExecutionState,
    };
    let (_directory, mut request, _package, _target) = fixture(11, "chat_completion");
    requested_revision(&mut request, Some("requested-other"));
    let batch = chat_batch_request(request);
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::pytorch::PyTorchBackend::new()),
        "PyTorch",
    ));
    let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        Arc::new(NeverResolve),
        Arc::new(NeverResolve),
        Arc::new(UnusedMediaSink),
        gateway.clone(),
    );
    let cancellation =
        RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone());
    let response = port
        .execute_runtime_host_batch_request(batch, cancellation)
        .await
        .unwrap();
    response.validate().unwrap();
    assert_eq!(response.state, RuntimeHostBatchExecutionState::Rejected);
    assert!(response
        .members
        .iter()
        .all(|member| member.outputs.is_empty()));
    assert!(!gateway.is_ready().await);
}

#[tokio::test]
#[ignore = "explicit native qualification requires real CPU Torch, Transformers, Accelerate and SoundFile"]
async fn saved_cpu_chat_graph_runs_native_owner_with_template_controls_and_scope_isolation() {
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::pytorch::PyTorchBackend::new()),
        "PyTorch",
    ));
    let mut previous_runs = std::collections::BTreeSet::new();
    for (sequence, (model_seed, task)) in [
        (11, "chat_completion"),
        (37, "chat_completion"),
        (11, "text_generation"),
        (11, "chat_completion"),
    ]
    .into_iter()
    .enumerate()
    {
        let (directory, request, package, target) = fixture(model_seed, task);
        let model_ref = request
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .selected_model_ref
            .clone();
        let artifact_root = TempDir::new().unwrap();
        let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
        let lifecycle = Arc::new(TestReservationLifecyclePort::default());
        let port = Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(
                serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
            )),
            Arc::new(Package(package)),
            Arc::new(UnusedMediaSink),
            gateway.clone(),
        ));
        let service = Arc::new(
            WorkflowService::with_ephemeral_attribution_store()
                .unwrap()
                .with_artifact_writer(test_artifact_writer(&artifact_root))
                .with_diagnostics_ledger(
                    pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
                )
                .with_dependency_environment_provider(Arc::new(provider.clone()))
                .with_dependency_readiness_work_queue(Arc::new(DependencyReadinessWorkQueue::new()))
                .with_runtime_dispatch_source_refresher(Arc::new(
                    TestRuntimeDispatchSourceRefresher::default(),
                ))
                .with_runtime_dispatch_candidate_provider(Arc::new(
                    TestRuntimeDispatchCandidateProvider,
                ))
                .with_runtime_host_execution_port(port.clone())
                .with_runtime_host_batch_execution_port(port.clone())
                .with_reservation_lifecycle_port(lifecycle.clone()),
        );
        let store = FileSystemWorkflowGraphStore::new(directory.path());
        let authored = graph(&model_ref, task);
        let saved = service
            .workflow_graph_save(
                &store,
                WorkflowGraphSaveRequest {
                    name: "Synthetic CPU Chat".into(),
                    graph: authored.clone(),
                },
            )
            .unwrap();
        let restored = service
            .workflow_graph_load(&store, WorkflowGraphLoadRequest { path: saved.path })
            .unwrap()
            .graph;
        assert_eq!(
            restored.compute_fingerprint(),
            authored.compute_fingerprint()
        );
        assert_eq!(restored.nodes[2].data["task_kind"], task);
        let workflow_id = format!("wf-chat-synthetic-{sequence}");
        let version = service
            .resolve_workflow_graph_version(&workflow_id, "1.0.0", &restored)
            .unwrap();
        install_chat_readiness(&service, &provider, &restored, &version, &model_ref, task);
        let control_bindings: Vec<_> = restored
            .nodes
            .iter()
            .filter(|node| {
                matches!(node.node_type.as_str(), "number-input" | "selection-input")
                    && node.id != "seed"
            })
            .map(|node| WorkflowPortBinding {
                node_id: node.id.clone(),
                port_id: "value".into(),
                value: node.data["value"].clone(),
            })
            .chain(std::iter::once(WorkflowPortBinding {
                node_id: "stop".into(),
                port_id: "text".into(),
                value: serde_json::json!("NEVER_STOP"),
            }))
            .collect();
        let host = Arc::new(ChatSessionHost(ImageRuntimeSessionHost::new(restored)));
        let cases: Vec<serde_json::Value> =
            serde_json::from_slice(&std::fs::read(directory.path().join("golden.json")).unwrap())
                .unwrap();
        // Distinct envelope members traverse the same native owner sequentially.
        // Results and terminal attribution must remain attached to their own scopes.
        use pantograph_runtime_host_contracts::{
            RuntimeHostBatchExecutionMemberState, RuntimeHostBatchExecutionPort,
            RuntimeHostBatchExecutionState, RuntimeHostExecutionOutputValue,
        };
        let mut batch = chat_batch_request(request.clone());
        for (member, case_index) in batch.members.iter_mut().zip([1, 6]) {
            let case = &cases[case_index];
            for input in &mut member.materialized_inputs {
                input.value = match input.port_id.as_str() {
                    "prompt" => RuntimeHostExecutionInputValue::String(
                        case["prompt"].as_str().unwrap().into(),
                    ),
                    "system_prompt" => RuntimeHostExecutionInputValue::String(
                        case["system_prompt"].as_str().unwrap_or("").into(),
                    ),
                    "seed" => RuntimeHostExecutionInputValue::U64(case["seed"].as_u64().unwrap()),
                    _ => input.value.clone(),
                };
            }
        }
        batch.validate().unwrap();
        let completed = port
            .execute_runtime_host_batch_request(
                batch.clone(),
                RuntimeHostExecutionCancellationHandle::running(batch.cancellation_context.clone()),
            )
            .await
            .unwrap();
        completed.validate().unwrap();
        assert_eq!(completed.state, RuntimeHostBatchExecutionState::Completed);
        for ((member, authored), case_index) in
            completed.members.iter().zip(&batch.members).zip([1, 6])
        {
            assert_eq!(
                member.state,
                RuntimeHostBatchExecutionMemberState::Completed
            );
            assert_eq!(member.execution_request_id, authored.execution_request_id);
            assert_eq!(member.assignment_id, authored.assignment_id);
            assert_eq!(member.workflow_id, authored.handoff.workflow_id);
            assert_eq!(member.workflow_run_id, authored.handoff.workflow_run_id);
            assert_eq!(member.node_id, authored.handoff.node_id);
            assert_eq!(member.task_id, authored.handoff.task_id);
            assert_eq!(member.outputs.len(), 1);
            assert_eq!(
                member.outputs[0].value,
                RuntimeHostExecutionOutputValue::String(
                    cases[case_index]["text"].as_str().unwrap().into()
                )
            );
        }
        let cancelled = port
            .execute_runtime_host_batch_request(
                batch.clone(),
                RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Cancelled(
                    batch.cancellation_context.cancellation_context_id.clone(),
                ))),
            )
            .await
            .unwrap();
        cancelled.validate().unwrap();
        assert_eq!(cancelled.state, RuntimeHostBatchExecutionState::Cancelled);
        assert!(cancelled.members.iter().all(|member| member.state
            == RuntimeHostBatchExecutionMemberState::Cancelled
            && member.outputs.is_empty()));
        let cancelled = port
            .execute_runtime_host_request(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Cancelled(
                    request.cancellation_context.cancellation_context_id.clone(),
                ))),
            )
            .await
            .unwrap();
        cancelled.validate().unwrap();
        assert_eq!(cancelled.state, RuntimeHostExecutionState::Rejected);
        assert!(cancelled.outputs.is_empty());
        // Healthy public graph replays below follow cancelled single/envelope requests.
        // Interleave seeds and replay 42 after another request; never reuse a run's output.
        for case_index in [1, 0, 2, 1, 3, 5, 7] {
            let case = &cases[case_index];
            let created = service
                .create_workflow_execution_session(
                    host.as_ref(),
                    WorkflowExecutionSessionCreateRequest {
                        workflow_id: workflow_id.clone(),
                        usage_profile: None,
                        keep_alive: false,
                    },
                )
                .await
                .unwrap();
            let mut inputs = control_bindings.clone();
            inputs.extend([
                WorkflowPortBinding {
                    node_id: "prompt".into(),
                    port_id: "text".into(),
                    value: case["prompt"].clone(),
                },
                WorkflowPortBinding {
                    node_id: "system".into(),
                    port_id: "text".into(),
                    value: case["system_prompt"]
                        .as_str()
                        .map(serde_json::Value::from)
                        .unwrap_or_else(|| serde_json::json!("")),
                },
                WorkflowPortBinding {
                    node_id: "seed".into(),
                    port_id: "value".into(),
                    value: case["seed"].clone(),
                },
            ]);
            let response=pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(),host.clone())
                .run_workflow_execution_session(WorkflowExecutionSessionRunRequest{session_id:created.session_id,workflow_semantic_version:"1.0.0".into(),
                    inputs,
                    output_targets:Some(vec![WorkflowOutputTarget{node_id:"output".into(),port_id:"text".into()},WorkflowOutputTarget{node_id:"infer".into(),port_id:"text".into()}]),
                    override_selection:None,timeout_ms:None,priority:None}).await.expect("actual synthetic chat through saved public scheduler graph");
            assert!(previous_runs.insert(response.workflow_run_id.clone()));
            assert_eq!(response.outputs.len(), 2);
            for output in &response.outputs {
                assert!(
                    output.value == case["text"],
                    "model={model_seed}, case={case_index}"
                );
            }
            assert!(lifecycle
                .events()
                .iter()
                .any(|event| event.workflow_run_id.as_str() == response.workflow_run_id));
        }
        println!("model={model_seed}, task={task}: 7 saved public graph replays + 2 separately attributed native envelope members, with pre-cancelled single/envelope isolation");
        assert_eq!(host.0.runtime_load_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(host.0.run_attempts.load(Ordering::SeqCst), 0);
    }
    gateway.stop().await.unwrap();
}
