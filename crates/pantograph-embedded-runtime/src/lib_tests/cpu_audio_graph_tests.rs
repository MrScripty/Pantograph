use super::*;
use crate::runtime_host_audio_execution::tests::{
    fixture, gateway, input_values, parent_outputs, Capture,
};
use crate::runtime_host_embedding_execution::tests::{Package, Target, UnusedMediaSink};
use pantograph_inference_interface_contracts::InferenceInterfaceDescriptor;
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, WorkflowGraphLoadRequest, WorkflowGraphSaveRequest,
};

fn audio_graph(model_ref: &PumasModelRef) -> WorkflowGraph {
    let mut descriptor: InferenceInterfaceDescriptor = serde_json::from_str(include_str!("../../../pantograph-inference-interface-contracts/tests/fixtures/descriptor_audio_ready.json")).unwrap();
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
            serde_json::json!({"task_kind":"audio_transcription","runtime":"pytorch","device":"cpu","pumas_model_ref":model_ref,"inference_interface_snapshot":snapshot})
        } else {serde_json::json!({})},
    }).collect();
    let edges = [
        ("prompt", "text", "prompt"),
        ("guidance", "value", "audio"),
        ("count", "value", "chunk_length_s"),
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
fn install_audio_readiness(
    service: &WorkflowService,
    provider: &DependencyEnvironmentReadinessSnapshotProvider,
    graph: &WorkflowGraph,
    version: &WorkflowVersionRecord,
    model_ref: &PumasModelRef,
) {
    // Package, readiness and dispatch facts are controlled. The scheduler and
    // selected-load host/gateway are production; audio responses are synthetic.
    let mut snapshot =
        image_runtime_validation_snapshot(version, graph, &model_ref.model_id, "fixture");
    let node = &mut snapshot.nodes[0];
    node.task_kind = InferenceTaskKind::parse("audio_transcription").unwrap();
    node.model_ref = model_ref.clone();
    node.descriptor_fingerprint =
        InferenceInterfaceFingerprint::parse("iface.test.cpu_audio.v1").unwrap();
    node.constraints.requested_runtime_id = Some(RuntimeIntentId::parse("pytorch").unwrap());
    node.constraints.requested_device_id = Some(DeviceIntentId::parse("cpu").unwrap());
    node.runtime_source_context.operation_type = "audio.json".into();
    node.runtime_source_context.context_shape_key = "audio.query-documents".into();
    let mut planning = image_runtime_dependency_planning_request(
        version,
        model_ref,
        vec![DependencyBindingId::parse("pytorch-audio").unwrap()],
    );
    planning.task_id = DependencyTaskId::parse("audio_transcription").unwrap();
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
        "name": "pytorch-audio", "kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "pytorch", "feature_id": "audio_transcription", "runtime_variant_id": "pytorch.cpu"}
    }])).unwrap();
    result.bindings = serde_json::from_value(serde_json::json!([{
        "binding_id": "pytorch-audio", "requirement_name": "pytorch-audio", "environment_kind": "runtime_feature",
        "runtime_feature": {"runtime_id": "pytorch", "feature_id": "audio_transcription", "runtime_variant_id": "pytorch.cpu"}
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
async fn saved_cpu_audio_graph_reopens_and_matches_parent_outputs_and_selected_identity() {
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    for revision in ["synthetic-r1", "synthetic-r2"] {
        let (directory, request, mut package, mut target) = fixture();
        package.model_ref.revision = Some(revision.into());
        if revision == "synthetic-r2" {
            package.model_ref.model_id = "synthetic/asr-fixture-second".into();
        }
        target.model_ref = package.model_ref.clone();
        let model_ref: PumasModelRef =
            serde_json::from_value(serde_json::to_value(&package.model_ref).unwrap()).unwrap();
        let mut values = input_values(&request);
        values.remove("language");
        values.remove("asr_task");
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
        let graph = audio_graph(&model_ref);
        let saved = service
            .workflow_graph_save(
                &store,
                WorkflowGraphSaveRequest {
                    name: "CPU Audio Fixture".into(),
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
            .resolve_workflow_graph_version("wf-audio", "1.0.0", &restored)
            .unwrap();
        install_audio_readiness(&service, &provider, &restored, &version, &model_ref);
        let host = Arc::new(ImageRuntimeSessionHost::new(restored));
        let created = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-audio".into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .unwrap();
        let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(),host.clone())
            .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {session_id:created.session_id,workflow_semantic_version:"1.0.0".into(),inputs:vec![
                WorkflowPortBinding {node_id:"prompt".into(),port_id:"text".into(),value:values["prompt"].clone()},
                WorkflowPortBinding {node_id:"guidance".into(),port_id:"value".into(),value:values["audio"].clone()},
                WorkflowPortBinding {node_id:"count".into(),port_id:"value".into(),value:values["chunk_length_s"].clone()},
            ],output_targets:Some(["response","text","stream","language","duration_seconds","segments","metadata","diagnostics"].into_iter().map(|port| WorkflowOutputTarget {node_id:"infer".into(),port_id:port.into()}).collect()),override_selection:None,timeout_ms:None,priority:None}).await.expect("saved audio graph through scheduler");
        assert_eq!(response.outputs.len(), 8);
        for output in &response.outputs {
            assert!(
                output.value == parent[&output.port_id],
                "saved audio output must match the parent output"
            );
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
            "pytorch.cpu"
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

#[tokio::test]
async fn saved_owned_audio_reference_reopens_reuses_source_and_publishes_long_transcript() {
    let _owned_fixture = crate::runtime_host_owned_audio::tests::OWNED_AUDIO_TEST_LOCK
        .lock()
        .await;
    use crate::runtime_host_owned_audio::tests::wav;
    use pantograph_workflow_service::workflow::WorkflowSchedulerTaskResultValue;
    let (_model, _, package, target) = fixture();
    let model_ref: PumasModelRef =
        serde_json::from_value(serde_json::to_value(&package.model_ref).unwrap()).unwrap();
    let artifacts = TempDir::new().unwrap();
    let writer = test_artifact_writer(&artifacts);
    let audio_store = crate::OwnedAudioInputStore::new(writer.clone());
    let source = audio_store
        .import_wav("wf-audio", "original-recording-run", wav(32000, 16000, 1))
        .unwrap();
    let source_value = serde_json::to_value(WorkflowSchedulerTaskResultValue::MediaArtifactRef(
        source.clone(),
    ))
    .unwrap();
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    let port = Arc::new(
        EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
            Arc::new(Target(
                serde_json::from_value(serde_json::to_value(&target).unwrap()).unwrap(),
            )),
            Arc::new(Package(package.clone())),
            Arc::new(UnusedMediaSink),
            gateway,
        )
        .with_owned_audio_store(audio_store),
    );
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let lifecycle = Arc::new(TestReservationLifecyclePort::default());
    let service = Arc::new(
        WorkflowService::with_ephemeral_attribution_store()
            .unwrap()
            .with_artifact_writer(writer)
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
    let graph_dir = TempDir::new().unwrap();
    let graph_store = FileSystemWorkflowGraphStore::new(graph_dir.path());
    let mut graph = audio_graph(&model_ref);
    graph.nodes[1].data = serde_json::json!({"value":source_value});
    graph.nodes.push(GraphNode {
        id: "sink".into(),
        node_type: "text-output".into(),
        position: Position { x: 0.0, y: 0.0 },
        data: serde_json::json!({}),
    });
    graph.edges.push(GraphEdge {
        id: "infer-sink".into(),
        source: "infer".into(),
        source_handle: "text".into(),
        target: "sink".into(),
        target_handle: "text".into(),
    });
    let shape = pantograph_workflow_service::workflow::workflow_scheduler_task_graph(
        &"wf-audio".parse().unwrap(),
        &"shape-run".parse().unwrap(),
        &graph,
    )
    .unwrap();
    let sink = shape
        .tasks
        .iter()
        .find(|t| t.node_id.as_str() == "sink")
        .unwrap();
    assert!(
        sink.diagnostics.is_empty(),
        "sink projection must succeed without diagnostics"
    );
    let saved = service
        .workflow_graph_save(
            &graph_store,
            WorkflowGraphSaveRequest {
                name: "Owned Recording".into(),
                graph: graph.clone(),
            },
        )
        .unwrap();
    let restored = service
        .workflow_graph_load(&graph_store, WorkflowGraphLoadRequest { path: saved.path })
        .unwrap()
        .graph;
    assert!(restored.nodes[1].data == graph.nodes[1].data);
    assert!(restored.compute_fingerprint() == graph.compute_fingerprint());
    assert!(serde_json::to_string(&restored).unwrap().len() < 65536);
    let version = service
        .resolve_workflow_graph_version("wf-audio", "1.0.0", &restored)
        .unwrap();
    install_audio_readiness(&service, &provider, &restored, &version, &model_ref);
    let host = Arc::new(ImageRuntimeSessionHost::new(restored));
    for _ in 0..2 {
        let session = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-audio".into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .unwrap();
        let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(),host.clone()).run_workflow_execution_session(WorkflowExecutionSessionRunRequest{session_id:session.session_id,workflow_semantic_version:"1.0.0".into(),inputs:vec![WorkflowPortBinding{node_id:"prompt".into(),port_id:"text".into(),value:serde_json::json!("context")},WorkflowPortBinding{node_id:"guidance".into(),port_id:"value".into(),value:source_value.clone()},WorkflowPortBinding{node_id:"count".into(),port_id:"value".into(),value:serde_json::json!(0.5)}],output_targets:Some(["text","stream","language","duration_seconds","segments","metadata","diagnostics"].into_iter().map(|port|WorkflowOutputTarget{node_id:"infer".into(),port_id:port.into()}).chain([WorkflowOutputTarget{node_id:"sink".into(),port_id:"text".into()}]).collect()),override_selection:None,timeout_ms:None,priority:None}).await.expect("owned recording graph must complete");
        assert!(response.outputs.len() == 8);
        for output in &response.outputs {
            if matches!(output.port_id.as_str(), "response" | "text") {
                assert!(
                    output.value == serde_json::json!("x".repeat(3000)),
                    "complete bounded transcript equality"
                );
            }
        }
        assert!(lifecycle
            .events()
            .iter()
            .any(|e| e.workflow_run_id.as_str() == response.workflow_run_id));
    }
    let sources = capture.owned_sources.lock().unwrap();
    assert!(
        sources.len() == 2
            && sources.iter().all(|s| s.0 == source.artifact_id
                && s.2 == "wf-audio"
                && s.3 == "original-recording-run")
    );
    let ids = capture.ids.lock().unwrap();
    assert!(ids.len() == 2 && ids[0] != ids[1]);
    assert!(capture
        .loads
        .lock()
        .unwrap()
        .iter()
        .all(|l| l.1 == target && l.2.selected_runtime_variant_id.as_str() == "pytorch.cpu"));
    assert!(
        host.runtime_load_attempts.load(Ordering::SeqCst) == 0
            && host.run_attempts.load(Ordering::SeqCst) == 0
    );
}

// Current native Pumas targets omit fingerprints. This controlled authenticated
// producer supplies fixture identity while preserving the actual client/resolvers.
struct HostedAudioProducer(tokio::task::JoinHandle<()>);
impl Drop for HostedAudioProducer {
    fn drop(&mut self) {
        self.0.abort();
    }
}
async fn hosted_audio_producer(
    owner: Arc<pumas_library::PumasApi>,
) -> (Arc<pumas_library::PumasLocalClient>, HostedAudioProducer) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let library_path = owner.launcher_root().to_path_buf();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        while let Ok(length) = stream.read_u32().await {
            assert!(length <= 65536);
            let mut frame = vec![0; length as usize];
            stream.read_exact(&mut frame).await.unwrap();
            let request: serde_json::Value = serde_json::from_slice(&frame).unwrap();
            assert_eq!(
                request["params"]["connection_token"],
                "hosted-audio-fixture-token"
            );
            let result = match request["method"].as_str().unwrap() {
                "resolve_model_artifact_load_target" => {
                    let selected: pumas_library::models::ResolveModelArtifactLoadTargetRequest =
                        serde_json::from_value(request["params"]["request"].clone()).unwrap();
                    let model_id = selected.model_ref.model_id.clone();
                    let mut response = owner
                        .resolve_model_artifact_load_target(selected)
                        .await
                        .unwrap();
                    let target = response.target.as_mut().unwrap();
                    target.local_load_path = owner
                        .launcher_root()
                        .join("shared-resources/models")
                        .join(model_id)
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .into();
                    target.content_fingerprint = Some("synthetic-content-r1".into());
                    serde_json::to_value(response).unwrap()
                }
                "resolve_model_package_facts" => serde_json::to_value(
                    owner
                        .resolve_model_package_facts(
                            request["params"]["model_id"].as_str().unwrap(),
                        )
                        .await
                        .unwrap(),
                )
                .unwrap(),
                method => panic!("unexpected fixture owner method {method}"),
            };
            let response = serde_json::to_vec(
                &serde_json::json!({"jsonrpc":"2.0","id":request["id"],"result":result}),
            )
            .unwrap();
            stream.write_u32(response.len() as u32).await.unwrap();
            stream.write_all(&response).await.unwrap();
        }
    });
    let client = pumas_library::PumasLocalClient::connect(pumas_library::registry::InstanceEntry {
        library_path,
        pid: std::process::id(),
        port: address.port(),
        transport_kind: pumas_library::registry::LocalInstanceTransportKind::LoopbackTcp,
        endpoint: address.to_string(),
        connection_token: Some("hosted-audio-fixture-token".into()),
        started_at: "fixture".into(),
        version: None,
        status: pumas_library::registry::InstanceStatus::Ready,
    })
    .await
    .unwrap();
    (Arc::new(client), HostedAudioProducer(server))
}

#[tokio::test]
async fn hosted_startup_owned_audio_graph_reuses_source_and_publishes_long_transcript() {
    let _owned_fixture = crate::runtime_host_owned_audio::tests::OWNED_AUDIO_TEST_LOCK
        .lock()
        .await;
    use crate::runtime_host_owned_audio::tests::wav;
    use crate::workflow_service_composition::{
        EmbeddedHostedStartupCompositionInput, EmbeddedHostedStartupConfig,
        EmbeddedHostedStartupPumasSelectorSource, EmbeddedWorkflowServiceComposition,
    };
    use pantograph_workflow_service::workflow::WorkflowSchedulerTaskResultValue;
    let root = TempDir::new().unwrap();
    let model_id = "audio/fixture/hosted-asr";
    let model_dir = root.path().join("shared-resources/models").join(model_id);
    std::fs::create_dir_all(&model_dir).unwrap();
    // Local inspection fixtures only; the controlled backend never loads weights.
    std::fs::write(
        model_dir.join("config.json"),
        r#"{"model_type":"whisper","architectures":["WhisperForConditionalGeneration"]}"#,
    )
    .unwrap();
    std::fs::write(
        model_dir.join("preprocessor_config.json"),
        r#"{"feature_extractor_type":"WhisperFeatureExtractor","sampling_rate":16000}"#,
    )
    .unwrap();
    write_min_safetensors(&model_dir.join("model.safetensors"));
    std::fs::write(model_dir.join("metadata.json"), serde_json::json!({
        "model_id":model_id,"family":"fixture","model_type":"audio",
        "official_name":"Hosted ASR fixture","cleaned_name":"hosted-asr",
        "selected_artifact_id":"hf-main","entry_path":model_dir,
        "storage_kind":"library_owned","validation_state":"valid",
        "pipeline_tag":"automatic-speech-recognition","task_type_primary":"audio_transcription",
        "input_modalities":["audio"],"output_modalities":["text"],
        "runtime_engine_hints":["transformers"],
        "files":[{"name":"config.json"},{"name":"preprocessor_config.json"},{"name":"model.safetensors"}]
    }).to_string()).unwrap();
    let pumas = Arc::new(
        crate::pumas_test_support::builder(root.path())
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    pumas.rebuild_model_index().await.unwrap();
    let package = pumas.resolve_model_package_facts(model_id).await.unwrap();
    assert_eq!(
        serde_json::to_value(package.artifact.artifact_kind).unwrap(),
        serde_json::json!("hf_compatible_directory")
    );
    let model_ref: PumasModelRef = serde_json::from_value(serde_json::json!({
        "model_id": package.model_ref.model_id,
        "selected_artifact_id": package.model_ref.selected_artifact_id,
    }))
    .unwrap();
    let artifacts = TempDir::new().unwrap();
    let writer = test_artifact_writer(&artifacts);
    let source = crate::OwnedAudioInputStore::new(writer.clone())
        .import_wav("wf-audio", "original-recording-run", wav(32000, 16000, 1))
        .unwrap();
    let source_value = serde_json::to_value(WorkflowSchedulerTaskResultValue::MediaArtifactRef(
        source.clone(),
    ))
    .unwrap();
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    let (pumas_client, _producer) = hosted_audio_producer(pumas).await;
    let registry = Arc::new(pantograph_runtime_registry::RuntimeRegistry::new());
    let output = EmbeddedWorkflowServiceComposition::resource_backed_hosted_startup(
        EmbeddedHostedStartupCompositionInput::new(EmbeddedHostedStartupConfig {
            runtime_registry: registry,
            runtime_registry_controller: gateway.clone(),
            gateway,
            pumas_selector_source: Some(EmbeddedHostedStartupPumasSelectorSource::Provided(
                Arc::new(workflow_nodes::setup::PumasSelectorAccess::LocalClient(
                    pumas_client,
                )),
            )),
            project_root: root.path().to_path_buf(),
            kv_cache_dir: root.path().join("kv-cache"),
            dependency_readiness_runtime_handle: tokio::runtime::Handle::current(),
            max_loaded_sessions: Some(1),
            max_dispatch_source_snapshot_age_ms: 1000,
        })
        .with_workflow_service(
            WorkflowService::with_ephemeral_attribution_store()
                .unwrap()
                .with_artifact_writer(writer)
                .with_diagnostics_ledger(
                    pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
                ),
        ),
    )
    .await
    .unwrap();
    output
        .dependency_readiness_snapshot_producer
        .shutdown()
        .await;
    // Control readiness and scheduling without replacing either production host
    // execution port installed by hosted startup. Pumas and artifact custody stay real.
    struct ControlledSchedulerDiagnostics;
    impl pantograph_workflow_service::WorkflowSchedulerDiagnosticsProvider
        for ControlledSchedulerDiagnostics
    {
    }
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let lifecycle = Arc::new(TestReservationLifecyclePort::default());
    let service = Arc::new(
        Arc::try_unwrap(output.workflow_service)
            .ok()
            .expect("exclusive service after producer shutdown")
            .with_scheduler_diagnostics_provider(Arc::new(ControlledSchedulerDiagnostics))
            .with_dependency_environment_provider(Arc::new(provider.clone()))
            .with_runtime_dispatch_source_refresher(Arc::new(
                TestRuntimeDispatchSourceRefresher::default(),
            ))
            .with_runtime_dispatch_candidate_provider(Arc::new(
                TestRuntimeDispatchCandidateProvider,
            ))
            .with_reservation_lifecycle_port(lifecycle.clone()),
    );
    let graph_dir = TempDir::new().unwrap();
    let graph_store = FileSystemWorkflowGraphStore::new(graph_dir.path());
    let mut graph = audio_graph(&model_ref);
    graph.nodes[1].data = serde_json::json!({"value":source_value});
    graph.nodes.push(GraphNode {
        id: "sink".into(),
        node_type: "text-output".into(),
        position: Position { x: 0.0, y: 0.0 },
        data: serde_json::json!({}),
    });
    graph.edges.push(GraphEdge {
        id: "infer-sink".into(),
        source: "infer".into(),
        source_handle: "text".into(),
        target: "sink".into(),
        target_handle: "text".into(),
    });
    let shape = pantograph_workflow_service::workflow::workflow_scheduler_task_graph(
        &"wf-audio".parse().unwrap(),
        &"shape-run".parse().unwrap(),
        &graph,
    )
    .unwrap();
    let sink = shape
        .tasks
        .iter()
        .find(|t| t.node_id.as_str() == "sink")
        .unwrap();
    assert!(
        sink.diagnostics.is_empty(),
        "sink projection must succeed without diagnostics"
    );
    let saved = service
        .workflow_graph_save(
            &graph_store,
            WorkflowGraphSaveRequest {
                name: "Owned Recording".into(),
                graph: graph.clone(),
            },
        )
        .unwrap();
    let restored = service
        .workflow_graph_load(&graph_store, WorkflowGraphLoadRequest { path: saved.path })
        .unwrap()
        .graph;
    assert!(restored.nodes[1].data == graph.nodes[1].data);
    assert!(restored.compute_fingerprint() == graph.compute_fingerprint());
    assert!(serde_json::to_string(&restored).unwrap().len() < 65536);
    let version = service
        .resolve_workflow_graph_version("wf-audio", "1.0.0", &restored)
        .unwrap();
    install_audio_readiness(&service, &provider, &restored, &version, &model_ref);
    let host = Arc::new(ImageRuntimeSessionHost::new(restored));
    for _ in 0..2 {
        let session = service
            .create_workflow_execution_session(
                host.as_ref(),
                WorkflowExecutionSessionCreateRequest {
                    workflow_id: "wf-audio".into(),
                    usage_profile: None,
                    keep_alive: false,
                },
            )
            .await
            .unwrap();
        let response = pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(), host.clone())
            .run_workflow_execution_session(
                WorkflowExecutionSessionRunRequest {
                    session_id: session.session_id,
                    workflow_semantic_version: "1.0.0".into(),
                    inputs: vec![
                        WorkflowPortBinding {
                            node_id: "prompt".into(),
                            port_id: "text".into(),
                            value: serde_json::json!("context"),
                        },
                        WorkflowPortBinding {
                            node_id: "guidance".into(),
                            port_id: "value".into(),
                            value: source_value.clone(),
                        },
                        WorkflowPortBinding {
                            node_id: "count".into(),
                            port_id: "value".into(),
                            value: serde_json::json!(0.5),
                        },
                    ],
                    output_targets: Some(
                        [
                            "text",
                            "stream",
                            "language",
                            "duration_seconds",
                            "segments",
                            "metadata",
                            "diagnostics",
                        ]
                        .into_iter()
                        .map(|port| WorkflowOutputTarget {
                            node_id: "infer".into(),
                            port_id: port.into(),
                        })
                        .chain([WorkflowOutputTarget {
                            node_id: "sink".into(),
                            port_id: "text".into(),
                        }])
                        .collect(),
                    ),
                    override_selection: None,
                    timeout_ms: None,
                    priority: None,
                },
            )
            .await
            .expect("owned recording graph must complete");
        assert!(response.outputs.len() == 8);
        for output in &response.outputs {
            if matches!(output.port_id.as_str(), "response" | "text") {
                assert!(
                    output.value == serde_json::json!("x".repeat(3000)),
                    "complete bounded transcript equality"
                );
            }
        }
        assert!(lifecycle
            .events()
            .iter()
            .any(|e| e.workflow_run_id.as_str() == response.workflow_run_id));
    }
    let sources = capture.owned_sources.lock().unwrap();
    assert!(
        sources.len() == 2
            && sources.iter().all(|s| s.0 == source.artifact_id
                && s.2 == "wf-audio"
                && s.3 == "original-recording-run")
    );
    let ids = capture.ids.lock().unwrap();
    assert!(ids.len() == 2 && ids[0] != ids[1]);
    assert!(capture
        .loads
        .lock()
        .unwrap()
        .iter()
        .all(|l| l.1.local_load_path == model_dir.to_str().unwrap()
            && l.1.model_ref.model_id == model_id
            && l.2.selected_runtime_variant_id.as_str() == "pytorch.cpu"));
    assert!(
        host.runtime_load_attempts.load(Ordering::SeqCst) == 0
            && host.run_attempts.load(Ordering::SeqCst) == 0
    );
}
