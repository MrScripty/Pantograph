//! Actual workflow worker → serial adapter → native receipt → registry cleanup.
//! Only readiness/candidate metadata and RAM declarations are fixture-controlled.
#[path = "native_session_composition_tests.rs"]
mod native_session_composition_tests;
use super::cpu_embedding_graph_tests::{embedding_graph, install_embedding_readiness};
use super::*;
use crate::serial_cpu_port::tests::fixture;
use pantograph_runtime_registry::*;
use pantograph_workflow_service::workflow::WorkflowSerialReadyConfig;

struct AdmittedCpuCandidates(SharedRuntimeRegistry);
impl WorkflowRuntimeDispatchCandidateProvider for AdmittedCpuCandidates {
    fn runtime_dispatch_candidates(
        &self,
        task: &WorkflowSchedulerTask,
        ready: &SchedulerTaskStateRecord,
        proof: &DependencyReadinessProofEnvelope,
    ) -> Result<WorkflowRuntimeDispatchCandidateSet, WorkflowRuntimeDispatchCandidateProviderError>
    {
        let set =
            TestRuntimeDispatchCandidateProvider.runtime_dispatch_candidates(task, ready, proof)?;
        let mut facts = Vec::new();
        for candidate in &set.candidates {
            let mut fact = set
                .candidate_evidence_context
                .candidate_fact(&candidate.candidate_id)
                .unwrap()
                .clone();
            let lease = self
                .0
                .acquire_reservation(RuntimeReservationRequest {
                    runtime_id: "candle".into(),
                    workflow_id: task.workflow_id.to_string(),
                    reservation_owner_id: Some(format!(
                        "{}:{}",
                        task.workflow_run_id, task.task_id
                    )),
                    usage_profile: None,
                    model_id: Some(fact.selected_model_ref.model_id.clone()),
                    pin_runtime: false,
                    requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                        RuntimeReservationResourceClaim::ram_bytes(256),
                    ])),
                    retention_hint: RuntimeRetentionHint::Ephemeral,
                })
                .map_err(
                    |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                        message: error.to_string(),
                    },
                )?;
            fact.reservations[0].reservation_lease_id = SchedulerReservationLeaseId::parse(
                format!("runtime-registry.{}", lease.reservation_id),
            )
            .unwrap();
            fact.reservations[0].resource_kind = SchedulerResourceKind::SystemRam;
            fact.reservations[0].reserved_bytes = 256;
            facts.push(fact);
        }
        let bundle = ValidatedWorkflowRuntimeDispatchCandidateFactBundle::try_from(
            WorkflowRuntimeDispatchCandidateFactBundle {
                contract_version: WORKFLOW_RUNTIME_DISPATCH_CANDIDATE_FACT_BUNDLE_CONTRACT_VERSION,
                facts,
                diagnostics: Vec::new(),
            },
        )
        .map_err(
            |error| WorkflowRuntimeDispatchCandidateProviderError::Failed {
                message: error.to_string(),
            },
        )?;
        Ok(WorkflowRuntimeDispatchCandidateSet::from_candidate_fact_bundle(bundle))
    }
}
#[tokio::test]
async fn actual_cpu_worker_consumes_linear_warm_receipt_and_preserves_session_successor() {
    let f = fixture().await;
    let model_ref = f.request.handoff.task_intent.model_ref.clone();
    let artifact_root = TempDir::new().unwrap();
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let service = Arc::new(
        WorkflowService::new_serial_ready_cpu(f.port.clone(), WorkflowSerialReadyConfig::default())
            .with_attribution_store(
                pantograph_workflow_service::workflow::SqliteAttributionStore::open_in_memory()
                    .unwrap(),
            )
            .with_artifact_writer(test_artifact_writer(&artifact_root))
            .with_diagnostics_ledger(
                pantograph_workflow_service::SqliteDiagnosticsLedger::open_in_memory().unwrap(),
            )
            .with_dependency_environment_provider(Arc::new(provider.clone()))
            .with_dependency_readiness_work_queue(Arc::new(DependencyReadinessWorkQueue::new()))
            .with_runtime_dispatch_source_refresher(Arc::new(
                TestRuntimeDispatchSourceRefresher::default(),
            ))
            .with_runtime_dispatch_candidate_provider(Arc::new(AdmittedCpuCandidates(
                f.port.registry.clone(),
            ))),
    );
    let graph = embedding_graph(&model_ref);
    let workflow_id = "wf-native-serial-cpu";
    let version = service
        .resolve_workflow_graph_version(workflow_id, "1.0.0", &graph)
        .unwrap();
    install_embedding_readiness(&service, &provider, &graph, &version, &model_ref);
    let host = Arc::new(ImageRuntimeSessionHost::new(graph));
    for i in 0..4 {
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
        let before = f
            .port
            .gateway
            .resident_cpu_serial_owner(&f.target.model_ref)
            .unwrap()
            .serial_facts();
        let response = tokio::time::timeout(std::time::Duration::from_secs(5),
        pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(), host.clone())
        .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
            session_id: created.session_id, workflow_semantic_version: "1.0.0".into(),
            inputs: vec![WorkflowPortBinding { node_id: "prompt".into(), port_id: "text".into(), value: serde_json::json!("hello world") }],
            output_targets: Some(vec![WorkflowOutputTarget { node_id: "vectors".into(), port_id: "vector".into() },
                WorkflowOutputTarget { node_id: "infer".into(), port_id: "usage".into() }]),
            override_selection: None, timeout_ms: None, priority: None,
        })).await.unwrap().expect("actual owned CPU serial worker");
        let vector = &response
            .outputs
            .iter()
            .find(|output| output.node_id == "vectors")
            .unwrap()
            .value;
        let golden: serde_json::Value =
            serde_json::from_slice(&std::fs::read(f.directory.path().join("golden.json")).unwrap())
                .unwrap();
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
                .find(|output| output.port_id == "usage")
                .unwrap()
                .value["total_tokens"],
            4
        );
        let after = f
            .port
            .gateway
            .resident_cpu_serial_owner(&f.target.model_ref)
            .unwrap()
            .serial_facts();
        assert_eq!(before.loaded_instance, after.loaded_instance);
        assert_eq!(before.generation + 2, after.generation);
        let snapshot = f.port.registry.snapshot();
        assert_eq!(snapshot.reservations, vec![f.successor.clone()]);
        assert!(snapshot.runtimes[0]
            .model_resource_residency
            .as_ref()
            .unwrap()
            .requirements
            .is_some());
        assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(
            service.serial_ready_observations_for_test(),
            vec![(after.generation, i + 1)]
        );
    }
    f.port.gateway.stop().await.unwrap();
}
