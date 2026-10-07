use super::*;
use crate::runtime_host_embedding_execution::tests::{Package, Target, UnusedMediaSink};
use crate::runtime_host_rerank_execution::tests::{
    fixture, gateway, input_values, parent_outputs, Capture,
};
use pantograph_inference_interface_contracts::InferenceInterfaceDescriptor;
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, WorkflowGraphLoadRequest, WorkflowGraphSaveRequest,
};

fn rerank_graph(model_ref: &PumasModelRef) -> WorkflowGraph {
    let mut descriptor: InferenceInterfaceDescriptor = serde_json::from_str(include_str!("../../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_rerank_ready.json")).unwrap();
    descriptor.model_ref = model_ref.clone();
    let snapshot =
        pantograph_workflow_service::graph::authored_snapshot_from_descriptor(&descriptor).unwrap();
    let nodes = [
        ("prompt", "text-input"),
        ("guidance", "selection-input"),
        ("count", "selection-input"),
        ("infer", "llm-inference"),
    ].into_iter().map(|(id,node_type)| GraphNode {
        id:id.into(), node_type:node_type.into(), position:Position {x:0.0,y:0.0},
        data:if id=="infer" {
            serde_json::json!({"task_kind":"rerank","runtime":"llamacpp","device":"cpu","pumas_model_ref":model_ref,"inference_interface_snapshot":snapshot})
        } else {serde_json::json!({})},
    }).collect();
    let edges = [
        ("prompt", "text", "query"),
        ("guidance", "value", "documents"),
        ("count", "value", "task_options"),
    ]
    .into_iter()
    .map(|(source, source_handle, target_handle)| GraphEdge {
        id: format!("{source}-infer"),
        source: source.into(),
        source_handle: source_handle.into(),
        target: "infer".into(),
        target_handle: target_handle.into(),
    })
    .collect();
    WorkflowGraph {
        nodes,
        edges,
        derived_graph: None,
    }
}
fn install_rerank_readiness(
    service: &WorkflowService,
    provider: &DependencyEnvironmentReadinessSnapshotProvider,
    graph: &WorkflowGraph,
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
) {
    // Package, readiness and dispatch facts are controlled. The scheduler and
    // selected-load host/gateway are production; rerank responses are synthetic.
    let mut snapshot =
        image_runtime_validation_snapshot(version, graph, &model_ref.model_id, "fixture");
    let node = &mut snapshot.nodes[0];
    node.task_kind = InferenceTaskKind::parse("rerank").unwrap();
    node.model_ref = model_ref.clone();
    node.descriptor_fingerprint =
        InferenceInterfaceFingerprint::parse("iface.test.cpu_rerank.v1").unwrap();
    node.constraints.requested_runtime_id = Some(RuntimeIntentId::parse("llamacpp").unwrap());
    node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    node.runtime_source_context.operation_type = "rerank.json".into();
    node.runtime_source_context.context_shape_key = "rerank.query-documents".into();
    let mut planning = image_runtime_dependency_planning_request(
        version,
        model_ref,
        vec![DependencyBindingId::parse("llamacpp-rerank").unwrap()],
    );
    planning.task_id = DependencyTaskId::parse("rerank").unwrap();
    planning.task_type = Some(planning.task_id.clone());
    planning.scheduler_intent.requested_runtime_id =
        Some(RuntimeIntentId::parse("llamacpp").unwrap());
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
        "name": "llamacpp-rerank", "kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "llamacpp", "feature_id": "rerank", "runtime_variant_id": "llama_cpp.cpu"}
    }])).unwrap();
    result.bindings = serde_json::from_value(serde_json::json!([{
        "binding_id": "llamacpp-rerank", "requirement_name": "llamacpp-rerank", "environment_kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "llamacpp", "feature_id": "rerank", "runtime_variant_id": "llama_cpp.cpu"}
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
async fn saved_cpu_rerank_graph_reopens_and_matches_parent_outputs_and_selected_identity() {
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    for revision in ["synthetic-r1", "synthetic-r2"] {
        let (directory, request, mut package, mut target) = fixture();
        package.model_ref.revision = Some(revision.into());
        if revision == "synthetic-r2" {
            package.model_ref.model_id = "rerank/test/second-owner".into();
        }
        target.model_ref = package.model_ref.clone();
        let model_ref: PumasModelRef =
            serde_json::from_value(serde_json::to_value(&package.model_ref).unwrap()).unwrap();
        let values = input_values(&request);
        let parent = parent_outputs(gateway.clone(), &package.model_ref, values.clone()).await;
        let artifact_root = TempDir::new().unwrap();
        let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
        let lifecycle = Arc::new(TestReservationLifecyclePort::default());
        let port = Arc::new(EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(
                serde_json::from_value(serde_json::to_value(&target).unwrap()).unwrap(),
            )),
            Arc::new(Package(package.clone())),
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
        let graph = rerank_graph(&model_ref);
        let saved = service
            .workflow_graph_save(
                &store,
                WorkflowGraphSaveRequest {
                    name: "CPU Rerank Fixture".into(),
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
            restored.nodes[3].data["inference_interface_snapshot"],
            graph.nodes[3].data["inference_interface_snapshot"]
        );
        let version = service
            .resolve_workflow_graph_version("wf-rerank", "1.0.0", &restored)
            .unwrap();
        install_rerank_readiness(&service, &provider, &restored, &version, &model_ref);
        let host = Arc::new(ImageRuntimeSessionHost::new(restored));
        let created = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-rerank".into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .unwrap();
        let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(),host.clone())
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {session_id:created.session_id,workflow_semantic_version:"1.0.0".into(),inputs:vec![
                WorkflowPortBinding {node_id:"prompt".into(),port_id:"text".into(),value:values["query"].clone()},
                WorkflowPortBinding {node_id:"guidance".into(),port_id:"value".into(),value:values["documents"].clone()},
                WorkflowPortBinding {node_id:"count".into(),port_id:"value".into(),value:values["task_options"].clone()},
            ],output_targets:Some(["results","scores","top_document","top_score","diagnostics"].into_iter().map(|port| WorkflowOutputTarget {node_id:"infer".into(),port_id:port.into()}).collect()),override_selection:None,timeout_ms:None,priority:None}).await.expect("saved rerank graph through scheduler");
        assert_eq!(response.outputs.len(), 5);
        for output in &response.outputs {
            assert_eq!(output.value, parent[&output.port_id], "{}", output.port_id);
        }
        let loads = capture.loads.lock().unwrap();
        let selected = loads.last().unwrap();
        assert_eq!(selected.1, target);
        assert_eq!(
            selected.2.selected_model_ref.as_ref(),
            Some(&package.model_ref)
        );
        assert_eq!(
            selected.2.selected_runtime_variant_id.as_str(),
            "llama_cpp.cpu"
        );
        assert_eq!(
            selected.2.selected_device_id.as_ref().unwrap().as_str(),
            "cpu"
        );
        assert!(lifecycle
            .events()
            .iter()
            .all(|event| event.workflow_run_id.as_str() == response.workflow_run_id));
        assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    }
    assert_eq!(capture.loads.lock().unwrap().len(), 2);
}
