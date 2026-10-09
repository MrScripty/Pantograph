//! Public real-host session acquisition and declared native Worker path. Only
//! package/target/readiness/candidate facts and RAM declarations are controlled.
use super::*;
use crate::retained_cpu_session::*;
use pantograph_workflow_service::{
    FileSystemWorkflowGraphStore, WorkflowGraphLoadRequest, WorkflowGraphSaveRequest,
};

#[derive(Debug)]
struct Ceiling(std::sync::atomic::AtomicU64);
impl RuntimeHostRamCapacitySource for Ceiling {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        let n = self.0.load(Ordering::Acquire);
        (n != u64::MAX).then_some(n)
    }
}
struct PausedPackage {
    package: inference::ResolvedModelPackageFacts,
    entered: Arc<tokio::sync::Notify>,
    proceed: Arc<tokio::sync::Notify>,
}
#[async_trait::async_trait]
impl crate::runtime_host_package_facts::RuntimeHostPackageFactsResolver for PausedPackage {
    async fn resolve(
        &self,
        _: &pantograph_runtime_host_contracts::ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        inference::ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        self.entered.notify_one();
        self.proceed.notified().await;
        Ok(self.package.clone())
    }
}
struct Episode {
    runtime: EmbeddedRetainedCpuSessionRuntime,
    registry: SharedRuntimeRegistry,
    gateway: Arc<inference::InferenceGateway>,
    model: inference::PumasModelRef,
    model_dir: TempDir,
    _root: TempDir,
    _artifacts: TempDir,
    workflow_id: String,
    ceiling: Arc<Ceiling>,
}
async fn episode(pause: Option<(Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>)>) -> Episode {
    let f = fixture().await;
    let gateway = f.port.gateway.clone();
    let registry = f.port.registry.clone();
    registry
        .release_reservation(f.successor.reservation_id)
        .unwrap();
    registry
        .configure_resource_domain(RuntimeResourceDomain {
            domain_id: "host".into(),
            total_bytes: 131072,
            safety_margin_bytes: 0,
            bindings: vec![RuntimeResourceDomainBinding {
                runtime_id: "candle".into(),
                resource_kind: RuntimeAdmissionResourceKind::RamBytes,
            }],
        })
        .unwrap();
    let ceiling = Arc::new(Ceiling(std::sync::atomic::AtomicU64::new(131072)));
    registry.bind_host_ram_capacity_source(ceiling.clone());
    let model = f.target.model_ref.clone();
    assert_ne!(
        model.model_id, f.target.local_load_path,
        "logical/path identity mismatch must be exercised"
    );
    let mut port = Arc::try_unwrap(f.port).ok().expect("unique fixture port");
    if let Some((entered, proceed)) = pause {
        port = port.with_test_package(Arc::new(PausedPackage {
            package: f.package,
            entered,
            proceed,
        }));
    }
    let root = TempDir::new().unwrap();
    let artifacts = TempDir::new().unwrap();
    // Real host technical-fit requires package facts. Seed a local owner API
    // with committed tiny files; native target remains separately controlled.
    std::fs::create_dir_all(root.path().join("pumas")).unwrap();
    let pumas = Arc::new(
        crate::pumas_test_support::builder(root.path().join("pumas"))
            .with_hf_client(false)
            .with_process_manager(false)
            .build()
            .await
            .unwrap(),
    );
    // This fixture already has an exact case-sensitive logical owner ID. The
    // convenience name builder normalizes case; seed its exact library ID.
    let model_path = pumas.model_library().library_root().join(&model.model_id);
    assert_eq!(
        pumas.model_library().get_model_id(&model_path).as_deref(),
        Some(model.model_id.as_str())
    );
    std::fs::create_dir_all(&model_path).unwrap();
    for entry in std::fs::read_dir(f.directory.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), model_path.join(entry.file_name())).unwrap();
        }
    }
    let metadata = ModelMetadata {
        schema_version: Some(2),
        model_id: Some(model.model_id.clone()),
        family: Some("bert".into()),
        model_type: Some("embedding".into()),
        official_name: Some("Synthetic-BERT-8".into()),
        cleaned_name: Some("Synthetic-BERT-8".into()),
        selected_artifact_id: model.selected_artifact_id.clone(),
        upstream_revision: model.revision.clone(),
        storage_kind: Some(StorageKind::LibraryOwned),
        import_state: Some(ImportState::Ready),
        validation_state: Some(AssetValidationState::Valid),
        task_type_primary: Some("embedding".into()),
        pipeline_tag: Some("feature-extraction".into()),
        input_modalities: Some(vec!["text".into()]),
        output_modalities: Some(vec!["embedding".into()]),
        runtime_engine_hints: Some(vec!["candle".into()]),
        ..Default::default()
    };
    pumas
        .model_library()
        .save_metadata(&model_path, &metadata)
        .await
        .unwrap();
    pumas
        .model_library()
        .index_model_dir(&model_path)
        .await
        .unwrap();
    pumas
        .resolve_model_package_facts(&model.model_id)
        .await
        .unwrap();
    let mut extensions = ExecutorExtensions::new();
    extensions.set(node_engine::extension_keys::PUMAS_API, pumas.clone());
    extensions.set(
        workflow_nodes::setup::PUMAS_SELECTOR_ACCESS,
        Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(pumas)),
    );
    let graph = session_graph(&model);
    let store = FileSystemWorkflowGraphStore::new(root.path());
    let saved = WorkflowService::new()
        .workflow_graph_save(
            &store,
            WorkflowGraphSaveRequest {
                name: "Retained CPU Session".into(),
                graph: graph.clone(),
            },
        )
        .unwrap();
    let workflow_id = Path::new(&saved.path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let restored = WorkflowService::new()
        .workflow_graph_load(
            &store,
            WorkflowGraphLoadRequest {
                path: saved.path.clone(),
            },
        )
        .unwrap()
        .graph;
    assert_eq!(restored.compute_fingerprint(), graph.compute_fingerprint());
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let config = EmbeddedRetainedCpuSessionConfig::new(
        model.clone(),
        1024,
        WorkflowSerialReadyConfig::default(),
    )
    .unwrap();
    let composition = EmbeddedRetainedCpuSessionComposition::from_port(port, config)
        .unwrap()
        .configure_service(|service| {
            service
                .with_attribution_store(
                    pantograph_workflow_service::workflow::SqliteAttributionStore::open_in_memory()
                        .unwrap(),
                )
                .with_artifact_writer(test_artifact_writer(&artifacts))
                .with_diagnostics_ledger(
                    pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
                )
                .with_dependency_environment_provider(Arc::new(provider.clone()))
                .with_dependency_readiness_work_queue(Arc::new(DependencyReadinessWorkQueue::new()))
                .with_runtime_dispatch_source_refresher(Arc::new(
                    TestRuntimeDispatchSourceRefresher::default(),
                ))
                .with_runtime_dispatch_candidate_provider(Arc::new(AdmittedCpuCandidates(
                    registry.clone(),
                )))
        })
        .unwrap();
    let mut config = EmbeddedRuntimeConfig::new(root.path().join("app"), root.path().to_path_buf());
    config.workflow_roots = vec![Path::new(&saved.path).parent().unwrap().to_path_buf()];
    let runtime = composition
        .into_runtime(config, Arc::new(RwLock::new(extensions)))
        .unwrap();
    let service = runtime.workflow_service();
    let version = service
        .resolve_workflow_graph_version(&workflow_id, "1.0.0", &restored)
        .unwrap();
    install_embedding_readiness(service, &provider, &restored, &version, &model);
    Episode {
        runtime,
        registry,
        gateway,
        model,
        model_dir: f.directory,
        _root: root,
        _artifacts: artifacts,
        workflow_id,
        ceiling,
    }
}
fn session_graph(model: &inference::PumasModelRef) -> WorkflowGraph {
    let mut graph = embedding_graph(model);
    for (id, category, port, data_type) in [
        ("prompt", "input", "text", "string"),
        ("vectors", "output", "vector", "embedding"),
    ] {
        let node = graph.nodes.iter_mut().find(|node| node.id == id).unwrap();
        node.data["definition"] = serde_json::json!({
            "node_type": node.node_type, "category": category,
            "io_binding_origin": "client_session",
            "inputs": [{"id":port,"label":port,"data_type":data_type,"required":false}],
            "outputs": [{"id":port,"label":port,"data_type":data_type,"required":false}],
        });
    }
    graph.nodes.push(GraphNode {
        id: "usage-out".into(),
        node_type: "json-filter".into(),
        position: Position { x: 400.0, y: 200.0 },
        data: serde_json::json!({"path":"total_tokens", "definition": {
            "node_type":"json-filter", "category":"output", "io_binding_origin":"client_session",
            "inputs":[{"id":"json","label":"JSON","data_type":"json","required":true}],
            "outputs":[{"id":"value","label":"Value","data_type":"any","required":false},
                {"id":"found","label":"Found","data_type":"boolean","required":false}]
        }}),
    });
    graph.edges.push(GraphEdge {
        id: "usage-to-tokens".into(),
        source: "infer".into(),
        source_handle: "usage".into(),
        target: "usage-out".into(),
        target_handle: "json".into(),
    });
    graph
}
fn create(e: &Episode) -> WorkflowExecutionSessionCreateRequest {
    WorkflowExecutionSessionCreateRequest {
        workflow_id: e.workflow_id.clone(),
        usage_profile: None,
        keep_alive: true,
    }
}
fn run(session: &str) -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: session.into(),
        workflow_semantic_version: "1.0.0".into(),
        inputs: vec![WorkflowPortBinding {
            node_id: "prompt".into(),
            port_id: "text".into(),
            value: serde_json::json!("hello world"),
        }],
        output_targets: Some(vec![
            WorkflowOutputTarget {
                node_id: "vectors".into(),
                port_id: "vector".into(),
            },
            WorkflowOutputTarget {
                node_id: "usage-out".into(),
                port_id: "value".into(),
            },
            WorkflowOutputTarget {
                node_id: "usage-out".into(),
                port_id: "found".into(),
            },
        ]),
        override_selection: None,
        timeout_ms: None,
        priority: None,
    }
}
fn mapped(e: &Episode, session: &str) -> RuntimeReservationLease {
    let id = e
        .runtime
        .runtime
        .session_runtime_reservations
        .lock()
        .unwrap()[session];
    e.registry.reservation_lease(id).unwrap()
}
fn io_query(run_id: String) -> WorkflowIoArtifactQueryRequest {
    WorkflowIoArtifactQueryRequest {
        workflow_run_id: Some(run_id),
        node_id: None,
        producer_node_id: None,
        consumer_node_id: None,
        artifact_role: None,
        media_type: None,
        retention_state: None,
        retention_policy_id: None,
        runtime_id: None,
        selected_backend_key: None,
        model_id: None,
        after_event_seq: None,
        limit: Some(50),
        projection_batch_size: Some(50),
    }
}
#[tokio::test]
async fn real_keep_alive_session_public_worker_uses_declared_envelope_and_releases_outputs() {
    let e = episode(None).await;
    let created = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let lease = mapped(&e, &created.session_id);
    assert_eq!(lease.model_id.as_deref(), Some(e.model.model_id.as_str()));
    assert_eq!(
        lease.reservation_owner_id.as_deref(),
        Some(created.session_id.as_str())
    );
    assert_eq!(lease.retention_hint, RuntimeRetentionHint::KeepAlive);
    assert_eq!(
        e.registry
            .snapshot()
            .runtimes
            .iter()
            .flat_map(|r| &r.active_reservation_claims)
            .find(|c| c.reservation_id == lease.reservation_id)
            .unwrap()
            .claims,
        vec![RuntimeReservationResourceClaim::ram_bytes(1024)]
    );
    let golden: serde_json::Value =
        serde_json::from_slice(&std::fs::read(e.model_dir.path().join("golden.json")).unwrap())
            .unwrap();
    for i in 0..4 {
        let before = e
            .gateway
            .resident_cpu_serial_owner(&e.model)
            .unwrap()
            .serial_facts();
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            e.runtime
                .run_workflow_execution_session(run(&created.session_id)),
        )
        .await
        .unwrap()
        .unwrap();
        let vector = &response
            .outputs
            .iter()
            .find(|x| x.node_id == "vectors")
            .unwrap()
            .value;
        assert_eq!(
            vector.as_array().unwrap().len(),
            golden["single_vectors"][0].as_array().unwrap().len()
        );
        for (actual, expected) in vector
            .as_array()
            .unwrap()
            .iter()
            .zip(golden["single_vectors"][0].as_array().unwrap())
        {
            assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-5);
        }
        assert_eq!(
            response
                .outputs
                .iter()
                .find(|o| o.node_id == "usage-out" && o.port_id == "value")
                .unwrap()
                .value,
            4
        );
        assert_eq!(
            response
                .outputs
                .iter()
                .find(|o| o.node_id == "usage-out" && o.port_id == "found")
                .unwrap()
                .value,
            true
        );
        let service = e.runtime.workflow_service();
        let refresh = service
            .workflow_diagnostics_projection_refresh(
                pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshRequest {
                    projections: vec![
                        pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::RunDetail,
                        pantograph_workflow_service::WorkflowDiagnosticsProjectionKind::IoArtifact,
                    ],
                    workflow_run_id: Some(response.workflow_run_id.clone()),
                    workflow_id: Some(e.workflow_id.clone()),
                    reason: pantograph_workflow_service::WorkflowDiagnosticsProjectionRefreshReason::ExplicitRefresh,
                    batch_size: 50,
                },
            )
            .unwrap();
        assert!(refresh.failed.is_empty());
        assert_eq!(
            service
                .workflow_run_detail_query(
                    pantograph_workflow_service::WorkflowRunDetailQueryRequest {
                        workflow_run_id: response.workflow_run_id.clone(),
                        projection_batch_size: Some(50),
                    }
                )
                .unwrap()
                .run
                .unwrap()
                .status,
            pantograph_workflow_service::RunListProjectionStatus::Completed
        );
        let artifacts = e
            .runtime
            .workflow_service()
            .workflow_io_artifact_query(io_query(response.workflow_run_id))
            .unwrap()
            .artifacts;
        // The existing archive retains requested public I/O bindings. Internal
        // producer acceptance is required by both real downstream branches and
        // same-store final projection; do not invent internal artifact capture.
        let tokens = serde_json::json!(4);
        let found = serde_json::json!(true);
        for (node, port, expected) in [
            ("vectors", "vector", vector),
            ("usage-out", "value", &tokens),
            ("usage-out", "found", &found),
        ] {
            let artifact = artifacts
                .iter()
                .find(|x| {
                    x.producer_node_id.as_deref() == Some(node)
                        && x.producer_port_id.as_deref() == Some(port)
                })
                .expect("accepted requested workflow output persisted artifact");
            let body = e
                .runtime
                .workflow_service()
                .read_artifact_body(pantograph_workflow_service::ArtifactReadRequest {
                    artifact_id: artifact.artifact_id.clone(),
                    byte_range_start: None,
                    byte_range_end_exclusive: None,
                })
                .unwrap()
                .body;
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                *expected
            );
        }
        let after = e
            .gateway
            .resident_cpu_serial_owner(&e.model)
            .unwrap()
            .serial_facts();
        assert_eq!(before.loaded_instance, after.loaded_instance);
        assert_eq!(before.generation + 2, after.generation);
        assert_eq!(e.registry.snapshot().reservations, vec![lease.clone()]);
        assert_eq!(mapped(&e, &created.session_id), lease);
        assert_eq!(
            e.runtime
                .workflow_service()
                .serial_ready_observations_for_test(),
            vec![(after.generation, i + 1)]
        );
    }
    let resident_before_close = e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
        .model_resource_residency
        .expect("actual owner published a resident charge");
    assert!(resident_before_close.requirements.is_some());
    e.runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: created.session_id,
        })
        .await
        .unwrap();
    assert!(e.registry.snapshot().reservations.is_empty());
    assert!(e
        .runtime
        .runtime
        .session_runtime_reservations
        .lock()
        .unwrap()
        .is_empty());
    assert!(e.gateway.resident_cpu_serial_owner(&e.model).is_none());
    // Actual native stop and the immediate accepted exact-owner producer ACK
    // retire the configured resident envelope through public session close.
    assert_eq!(
        e.registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|r| r.runtime_id == "candle")
            .unwrap()
            .model_resource_residency,
        None
    );
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
}
#[tokio::test]
async fn ephemeral_and_foreign_session_map_refuse_before_claims() {
    let e = episode(None).await;
    let mut request = create(&e);
    request.keep_alive = false;
    assert!(e
        .runtime
        .create_workflow_execution_session(request)
        .await
        .is_err());
    assert!(e.registry.snapshot().reservations.is_empty());
    let a = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let b = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let original = mapped(&e, &a.session_id);
    let foreign = mapped(&e, &b.session_id);
    e.runtime
        .runtime
        .session_runtime_reservations
        .lock()
        .unwrap()
        .insert(a.session_id.clone(), foreign.reservation_id);
    let before = e.registry.snapshot();
    let owner = e
        .gateway
        .resident_cpu_serial_owner(&e.model)
        .unwrap()
        .serial_facts();
    assert!(e
        .runtime
        .run_workflow_execution_session(run(&a.session_id))
        .await
        .is_err());
    assert_eq!(e.registry.snapshot().reservations, before.reservations);
    assert_eq!(
        owner.generation,
        e.gateway
            .resident_cpu_serial_owner(&e.model)
            .unwrap()
            .serial_facts()
            .generation
    );
    e.runtime
        .runtime
        .session_runtime_reservations
        .lock()
        .unwrap()
        .insert(a.session_id.clone(), original.reservation_id);
    for session_id in [a.session_id, b.session_id] {
        e.runtime
            .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest { session_id })
            .await
            .unwrap();
    }
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
}
#[tokio::test]
async fn captured_session_replacement_or_capacity_loss_during_resolver_refuses_forward_and_ack() {
    for failure in [0, 1, 2] {
        let entered = Arc::new(tokio::sync::Notify::new());
        let proceed = Arc::new(tokio::sync::Notify::new());
        let e = Arc::new(episode(Some((entered.clone(), proceed.clone()))).await);
        let created = e
            .runtime
            .create_workflow_execution_session(create(&e))
            .await
            .unwrap();
        let original = mapped(&e, &created.session_id);
        let owner = e
            .gateway
            .resident_cpu_serial_owner(&e.model)
            .unwrap()
            .serial_facts();
        let worker = {
            let e = e.clone();
            let request = run(&created.session_id);
            tokio::spawn(async move { e.runtime.run_workflow_execution_session(request).await })
        };
        tokio::time::timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        if failure == 2 {
            e.runtime
                .runtime
                .host()
                .reserve_loaded_session_runtime(
                    &created.session_id,
                    &e.workflow_id,
                    None,
                    WorkflowExecutionSessionRetentionHint::KeepAlive,
                )
                .await
                .unwrap();
            assert_ne!(
                mapped(&e, &created.session_id).reservation_id,
                original.reservation_id
            );
        } else {
            e.ceiling.0.store(
                if failure == 0 { 32768 } else { u64::MAX },
                Ordering::Release,
            );
        }
        proceed.notify_one();
        assert!(tokio::time::timeout(Duration::from_secs(5), worker)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        assert_eq!(
            owner.generation,
            e.gateway
                .resident_cpu_serial_owner(&e.model)
                .unwrap()
                .serial_facts()
                .generation
        );
        assert_eq!(
            e.registry.snapshot().reservations.len(),
            2,
            "no fabricated generic cleanup ACK"
        );
        assert!(e
            .runtime
            .workflow_service()
            .serial_ready_observations_for_test()
            .is_empty());
        assert!(e.runtime.workflow_service().serial_ready_cpu_is_poisoned());
        assert!(
            tokio::time::timeout(
                Duration::from_secs(5),
                e.runtime
                    .run_workflow_execution_session(run(&created.session_id))
            )
            .await
            .unwrap()
            .is_err(),
            "no reopening after absent native drain"
        );
        assert_eq!(e.registry.snapshot().reservations.len(), 2);
        // Failure deliberately retains charged uncertainty; no fake cleanup.
    }
}
#[tokio::test]
async fn missing_warm_owner_wrong_graph_and_replaced_service_are_refused() {
    let e = episode(None).await;
    let store = FileSystemWorkflowGraphStore::new(e._root.path());
    let mut other = e.model.clone();
    other.model_id.push_str("-foreign");
    let saved = e
        .runtime
        .workflow_service()
        .workflow_graph_save(
            &store,
            WorkflowGraphSaveRequest {
                name: "Wrong Retained Model".into(),
                graph: session_graph(&other),
            },
        )
        .unwrap();
    let mut request = create(&e);
    request.workflow_id = Path::new(&saved.path)
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .into();
    assert!(e
        .runtime
        .create_workflow_execution_session(request)
        .await
        .is_err());
    assert!(e.registry.snapshot().reservations.is_empty());
    let protected_resident = e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
        .model_resource_residency;
    assert!(protected_resident.is_some());
    e.gateway.stop().await.unwrap();
    assert!(e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .is_err());
    assert!(e.registry.snapshot().reservations.is_empty());
    // Generic stop did not publish the linear native proof. Shutdown cannot
    // reconstruct it from a missing physical model or fabricate a release ACK.
    let refused = e
        .runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .expect_err("generic stop without native ACK stays charged");
    assert!(refused
        .to_string()
        .contains("exact native CPU retirement acknowledgement required"));
    assert!(!e.gateway.is_ready().await);
    assert!(e.registry.snapshot().reservations.is_empty());
    assert_eq!(
        e.registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|r| r.runtime_id == "candle")
            .unwrap()
            .model_resource_residency,
        protected_resident
    );
    let f = fixture().await;
    let gateway = f.port.gateway.clone();
    let model = f.target.model_ref;
    assert!(EmbeddedRetainedCpuSessionConfig::new(
        model.clone(),
        0,
        WorkflowSerialReadyConfig::default()
    )
    .is_err());
    let config =
        EmbeddedRetainedCpuSessionConfig::new(model, 1024, WorkflowSerialReadyConfig::default())
            .unwrap();
    let port = Arc::try_unwrap(f.port).ok().unwrap();
    assert!(
        EmbeddedRetainedCpuSessionComposition::from_port(port, config)
            .unwrap()
            .configure_service(|_| WorkflowService::new())
            .is_err()
    );
    gateway.stop().await.unwrap();
}

#[tokio::test]
async fn declared_floor_preserves_larger_ram_and_vram_and_mixed_graph_refuses() {
    let e = episode(None).await;
    let host = e.runtime.runtime.host();
    let profile = e.runtime.runtime.retained_cpu_session.as_ref().unwrap();
    let mut request = RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: e.workflow_id.clone(),
        reservation_owner_id: Some("probe".into()),
        usage_profile: None,
        model_id: Some("physical/path".into()),
        pin_runtime: false,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(4096),
            RuntimeReservationResourceClaim::vram_bytes(512),
        ])),
        retention_hint: RuntimeRetentionHint::KeepAlive,
    };
    profile
        .apply_request(&host, &e.workflow_id, &mut request)
        .await
        .unwrap();
    assert_eq!(request.model_id.as_deref(), Some(e.model.model_id.as_str()));
    assert_eq!(
        request.requirements.unwrap().claims,
        vec![
            RuntimeReservationResourceClaim::ram_bytes(4096),
            RuntimeReservationResourceClaim::vram_bytes(512),
        ]
    );
    let store = FileSystemWorkflowGraphStore::new(e._root.path());
    for node_type in ["puma-lib", "audio-transcription", "unknown-runtime"] {
        let mut graph = session_graph(&e.model);
        let mut node = graph.nodes[0].clone();
        node.id = "unsupported".into();
        node.node_type = node_type.into();
        graph.nodes.push(node);
        let saved = e
            .runtime
            .workflow_service()
            .workflow_graph_save(
                &store,
                WorkflowGraphSaveRequest {
                    name: format!("Mixed {node_type}"),
                    graph,
                },
            )
            .unwrap();
        let workflow_id = Path::new(&saved.path)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let mut request = RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: workflow_id.into(),
            reservation_owner_id: Some("probe".into()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::KeepAlive,
        };
        assert!(profile
            .apply_request(&host, workflow_id, &mut request)
            .await
            .is_err());
        assert!(e.registry.snapshot().reservations.is_empty());
        assert!(e
            .runtime
            .create_workflow_execution_session(WorkflowExecutionSessionCreateRequest {
                workflow_id: workflow_id.into(),
                usage_profile: None,
                keep_alive: true,
            })
            .await
            .is_err());
        assert!(e.registry.snapshot().reservations.is_empty());
    }
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
}
#[tokio::test]
async fn closed_session_refuses_while_other_session_keeps_actual_owner_warm() {
    let e = episode(None).await;
    let a = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let b = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let retained = mapped(&e, &b.session_id);
    let resident_before = e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
        .model_resource_residency;
    assert!(resident_before.is_some());
    e.runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: a.session_id.clone(),
        })
        .await
        .unwrap();
    let owner = e
        .gateway
        .resident_cpu_serial_owner(&e.model)
        .unwrap()
        .serial_facts();
    assert!(e
        .runtime
        .run_workflow_execution_session(run(&a.session_id))
        .await
        .is_err());
    assert_eq!(e.registry.snapshot().reservations, vec![retained]);
    assert_eq!(
        e.registry
            .snapshot()
            .runtimes
            .into_iter()
            .find(|r| r.runtime_id == "candle")
            .unwrap()
            .model_resource_residency,
        resident_before
    );
    assert_eq!(
        e.gateway
            .resident_cpu_serial_owner(&e.model)
            .unwrap()
            .serial_facts()
            .generation,
        owner.generation
    );
    e.runtime
        .close_workflow_execution_session(WorkflowExecutionSessionCloseRequest {
            session_id: b.session_id,
        })
        .await
        .unwrap();
    assert!(e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
        .model_resource_residency
        .is_none());
    assert!(!e.gateway.is_ready().await);
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
}
#[tokio::test]
async fn callback_replacement_with_different_serial_owner_is_refused() {
    let f = fixture().await;
    let replacement = fixture().await;
    let config = EmbeddedRetainedCpuSessionConfig::new(
        f.target.model_ref.clone(),
        1024,
        WorkflowSerialReadyConfig::default(),
    )
    .unwrap();
    let gateway = f.port.gateway.clone();
    let other_gateway = replacement.port.gateway.clone();
    let composition = EmbeddedRetainedCpuSessionComposition::from_port(
        Arc::try_unwrap(f.port).ok().unwrap(),
        config,
    )
    .unwrap();
    assert!(composition
        .configure_service(|_| WorkflowService::new_serial_ready_cpu(
            replacement.port,
            WorkflowSerialReadyConfig::default()
        ))
        .is_err());
    gateway.stop().await.unwrap();
    other_gateway.stop().await.unwrap();
}

#[tokio::test]
async fn public_constructor_propagates_same_pumas_owner_and_refuses_conflicts_or_busy_extensions() {
    let e = episode(None).await;
    let (selector, api) = {
        let guard = e.runtime.runtime.extensions.read().await;
        (
            guard
                .get::<Arc<workflow_nodes::setup::PumasSelectorAccess>>(
                    workflow_nodes::setup::PUMAS_SELECTOR_ACCESS,
                )
                .unwrap()
                .clone(),
            guard
                .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .unwrap()
                .clone(),
        )
    };
    let make = || {
        EmbeddedRetainedCpuSessionComposition::new(
            e.gateway.clone(),
            e.registry.clone(),
            selector.clone(),
            EmbeddedRetainedCpuSessionConfig::new(
                e.model.clone(),
                1024,
                WorkflowSerialReadyConfig::default(),
            )
            .unwrap(),
        )
        .unwrap()
    };
    let config =
        || EmbeddedRuntimeConfig::new(e._root.path().join("app"), e._root.path().to_path_buf());
    for api_conflict in [false, true] {
        let mut ext = ExecutorExtensions::new();
        if api_conflict {
            ext.set(
                node_engine::extension_keys::PUMAS_API,
                "wrong typed existing owner".to_string(),
            );
        } else {
            ext.set(
                workflow_nodes::setup::PUMAS_SELECTOR_ACCESS,
                Arc::new(workflow_nodes::setup::PumasSelectorAccess::Owner(
                    api.clone(),
                )),
            );
        }
        let shared = Arc::new(RwLock::new(ext));
        assert!(make().into_runtime(config(), shared.clone()).is_err());
        let guard = shared.read().await;
        assert_eq!(
            guard.has(workflow_nodes::setup::PUMAS_SELECTOR_ACCESS),
            !api_conflict
        );
        assert_eq!(
            guard.has(node_engine::extension_keys::PUMAS_API),
            api_conflict,
            "refusal leaves existing extensions intact"
        );
        assert!(e.registry.snapshot().reservations.is_empty());
    }
    let shared = Arc::new(RwLock::new(ExecutorExtensions::new()));
    let held = shared.write().await;
    assert!(make().into_runtime(config(), shared.clone()).is_err());
    drop(held);
    let constructed = make().into_runtime(config(), shared.clone()).unwrap();
    {
        let guard = shared.read().await;
        assert!(Arc::ptr_eq(
            guard
                .get::<Arc<workflow_nodes::setup::PumasSelectorAccess>>(
                    workflow_nodes::setup::PUMAS_SELECTOR_ACCESS
                )
                .unwrap(),
            &selector
        ));
        assert!(Arc::ptr_eq(
            guard
                .get::<Arc<pumas_library::PumasApi>>(node_engine::extension_keys::PUMAS_API)
                .unwrap(),
            &api
        ));
    }
    // Constructor source ownership only; do not run the unqualified native
    // production HF target resolver or claim production target/preload success.
    constructed
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
}

#[tokio::test]
async fn public_worker_shutdown_retires_actual_resident_after_completed_run() {
    let e = episode(None).await;
    let created = e
        .runtime
        .create_workflow_execution_session(create(&e))
        .await
        .unwrap();
    let response = e
        .runtime
        .run_workflow_execution_session(run(&created.session_id))
        .await
        .unwrap();
    assert_eq!(
        response
            .outputs
            .iter()
            .find(|o| o.node_id == "usage-out" && o.port_id == "value")
            .unwrap()
            .value,
        4
    );
    assert!(e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap()
        .model_resource_residency
        .is_some());
    e.runtime
        .shutdown(Duration::from_secs(1), Duration::from_secs(1))
        .await
        .unwrap();
    assert!(!e.gateway.is_ready().await);
    let stopped = e
        .registry
        .snapshot()
        .runtimes
        .into_iter()
        .find(|r| r.runtime_id == "candle")
        .unwrap();
    assert!(
        stopped.model_resource_residency.is_none()
            && !stopped.resident_resources_uncertain
            && stopped.models.is_empty()
    );
}
