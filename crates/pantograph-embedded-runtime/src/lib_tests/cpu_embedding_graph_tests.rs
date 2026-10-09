use super::*;
use crate::runtime_host_embedding_execution::tests::{fixture, Package, Target, UnusedMediaSink};
use pantograph_inference_interface_contracts::InferenceInterfaceDescriptor;
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, WorkflowGraphLoadRequest, WorkflowGraphSaveRequest,
};

fn embedding_graph(model_ref: &PumasModelRef) -> WorkflowGraph {
    let mut descriptor: InferenceInterfaceDescriptor = serde_json::from_str(include_str!(
        "../../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_embedding_ready.json"
    ))
    .unwrap();
    descriptor.model_ref = model_ref.clone();
    let snapshot =
        pantograph_workflow_service::graph::authored_snapshot_from_descriptor(&descriptor).unwrap();
    WorkflowGraph {
        nodes: vec![
            GraphNode {
                id: "prompt".into(),
                node_type: "text-input".into(),
                position: Position { x: 0.0, y: 0.0 },
                data: serde_json::json!({}),
            },
            GraphNode {
                id: "infer".into(),
                node_type: "llm-inference".into(),
                position: Position { x: 200.0, y: 0.0 },
                data: serde_json::json!({
                    "task_kind": "embedding", "runtime": "candle", "device": "cpu",
                    "pumas_model_ref": model_ref, "inference_interface_snapshot": snapshot,
                }),
            },
            GraphNode {
                id: "vectors".into(),
                node_type: "vector-output".into(),
                position: Position { x: 400.0, y: 0.0 },
                data: serde_json::json!({}),
            },
        ],
        edges: vec![
            GraphEdge {
                id: "text-to-embedding".into(),
                source: "prompt".into(),
                source_handle: "text".into(),
                target: "infer".into(),
                target_handle: "text".into(),
            },
            GraphEdge {
                id: "embedding-to-vector".into(),
                source: "infer".into(),
                source_handle: "embedding".into(),
                target: "vectors".into(),
                target_handle: "vector".into(),
            },
        ],
        derived_graph: None,
    }
}

fn install_embedding_readiness(
    service: &WorkflowService,
    provider: &DependencyEnvironmentReadinessSnapshotProvider,
    graph: &WorkflowGraph,
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
) {
    // Only package/readiness/dispatch facts are controlled. Inference below uses
    // the production selected-model owner and the committed actual Candle model.
    let mut snapshot =
        image_runtime_validation_snapshot(version, graph, &model_ref.model_id, "fixture");
    let node = &mut snapshot.nodes[0];
    node.task_kind = InferenceTaskKind::parse("embedding").unwrap();
    node.model_ref = model_ref.clone();
    node.descriptor_fingerprint =
        InferenceInterfaceFingerprint::parse("iface.test.cpu_embedding.v1").unwrap();
    node.constraints.requested_runtime_id = Some(RuntimeIntentId::parse("candle").unwrap());
    node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    node.runtime_source_context.operation_type = "embedding.text".into();
    node.runtime_source_context.context_shape_key = "embedding.one-text".into();
    let mut planning = image_runtime_dependency_planning_request(
        version,
        model_ref,
        vec![DependencyBindingId::parse("candle-embedding").unwrap()],
    );
    planning.task_id = DependencyTaskId::parse("embedding").unwrap();
    planning.task_type = Some(planning.task_id.clone());
    planning.scheduler_intent.requested_runtime_id =
        Some(RuntimeIntentId::parse("candle").unwrap());
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
        "name": "candle-embedding", "kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "candle", "feature_id": "embedding", "runtime_variant_id": "candle.cpu"}
    }])).unwrap();
    result.bindings = serde_json::from_value(serde_json::json!([{
        "binding_id": "candle-embedding", "requirement_name": "candle-embedding", "environment_kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "candle", "feature_id": "embedding", "runtime_variant_id": "candle.cpu"}
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

#[tokio::test]
async fn saved_cpu_embedding_graph_runs_public_scheduler_and_preserves_vectors_metadata_usage() {
    let gateway = Arc::new(inference::InferenceGateway::with_backend(
        Box::new(inference::backend::candle::CandleBackend::new()),
        "Candle",
    ));
    for width in [8, 12] {
        let (directory, request, package, target) = fixture(width);
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
                .with_runtime_host_batch_execution_port(port)
                .with_reservation_lifecycle_port(lifecycle.clone()),
        );
        let store = FileSystemWorkflowGraphStore::new(directory.path());
        let graph = embedding_graph(&model_ref);
        let saved = service
            .workflow_graph_save(
                &store,
                WorkflowGraphSaveRequest {
                    name: "CPU Embedding Fixture".into(),
                    graph: graph.clone(),
                },
            )
            .unwrap();
        let restored = service
            .workflow_graph_load(&store, WorkflowGraphLoadRequest { path: saved.path })
            .unwrap()
            .graph;
        assert_eq!(restored.compute_fingerprint(), graph.compute_fingerprint());
        assert_eq!(
            restored.nodes[1].data["inference_interface_snapshot"],
            graph.nodes[1].data["inference_interface_snapshot"]
        );
        let workflow_id = "wf-cpu-embedding-fixture";
        let version = service
            .resolve_workflow_graph_version(workflow_id, "1.0.0", &restored)
            .unwrap();
        install_embedding_readiness(&service, &provider, &restored, &version, &model_ref);
        let host = Arc::new(ImageRuntimeSessionHost::new(restored));
        let created = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: workflow_id.into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .unwrap();
        let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(), host.clone())
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                session_id: created.session_id, workflow_semantic_version: "1.0.0".into(),
                inputs: vec![WorkflowPortBinding { node_id: "prompt".into(), port_id: "text".into(), value: serde_json::json!("hello world") }],
                output_targets: Some(vec![
                    WorkflowOutputTarget { node_id: "vectors".into(), port_id: "vector".into() },
                    WorkflowOutputTarget { node_id: "infer".into(), port_id: "embedding".into() },
                    WorkflowOutputTarget { node_id: "infer".into(), port_id: "metadata".into() },
                    WorkflowOutputTarget { node_id: "infer".into(), port_id: "usage".into() },
                ]), override_selection: None, timeout_ms: None, priority: None,
            }).await.expect("actual Candle embedding graph through public scheduler");
        let output = |node: &str, port: &str| {
            &response
                .outputs
                .iter()
                .find(|item| item.node_id == node && item.port_id == port)
                .unwrap()
                .value
        };
        let vector = output("vectors", "vector");
        assert_eq!(vector, output("infer", "embedding"));
        assert_eq!(vector.as_array().unwrap().len(), width);
        let golden: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                    "../inference/tests/fixtures/candle_bert/bert-{width}/golden.json"
                )),
            )
            .unwrap(),
        )
        .unwrap();
        for (actual, expected) in vector
            .as_array()
            .unwrap()
            .iter()
            .zip(golden["single_vectors"][0].as_array().unwrap())
        {
            assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-5);
        }
        let metadata = output("infer", "metadata");
        assert_eq!(
            metadata["model_ref"],
            serde_json::to_value(&model_ref).unwrap()
        );
        assert_eq!(metadata["vector_length"], width);
        assert_eq!(metadata["index"], 0);
        assert_eq!(metadata["token_count"], 4);
        assert_eq!(metadata["runtime_variant_id"], "candle.cpu");
        assert_eq!(metadata["device_ids"], serde_json::json!(["cpu"]));
        assert_eq!(output("infer", "usage")["prompt_tokens"], 4);
        assert_eq!(output("infer", "usage")["total_tokens"], 4);
        assert!(lifecycle
            .events()
            .iter()
            .all(|event| event.workflow_run_id.as_str() == response.workflow_run_id));
        assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    }
    gateway.stop().await.unwrap();
}
