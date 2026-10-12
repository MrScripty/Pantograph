//! Actual workflow worker → serial adapter → native receipt → registry cleanup.
//! Only readiness/candidate metadata and RAM declarations are fixture-controlled.
use super::cpu_embedding_graph_tests::{embedding_graph, install_embedding_readiness};
use super::*;
use crate::serial_cpu_port::tests::fixture;
use pantograph_runtime_host_contracts::*;
use pantograph_runtime_registry::*;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerSerialBoundDispatch, SchedulerSerialDrainedDispatch,
    SchedulerSerialOwnerSnapshot,
};
use pantograph_workflow_service::workflow::WorkflowSerialReadyConfig;
use std::sync::{atomic::AtomicUsize, Weak};

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

/// This wrapper cancels only after the real native call returns its drained
/// result. It never substitutes output, owner evidence, drain proof or cleanup.
/// The optional receipt loss is an explicit collector fault after that drain.
struct CancelAfterNativeDrain {
    inner: Arc<crate::EmbeddedRetainedCpuSerialPort>,
    cancel_target: Mutex<Option<(Weak<WorkflowService>, String)>>,
    lose_cancelled_receipt: bool,
    forward_calls: AtomicUsize,
    snapshot_cleanup_calls: AtomicUsize,
    completed: Mutex<Vec<(RuntimeReservationLease, RuntimeHostBatchExecutionResponse)>>,
    cleanups: Arc<Mutex<Vec<(ReservationLifecycleEvent, ReservationLifecycleApplication)>>>,
}

struct ObserveNativeCleanup {
    inner: Box<dyn SerialRuntimeHostCleanupReceipt>,
    applications: Arc<Mutex<Vec<(ReservationLifecycleEvent, ReservationLifecycleApplication)>>>,
}

#[async_trait::async_trait]
impl SerialRuntimeHostCleanupReceipt for ObserveNativeCleanup {
    fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.inner.previous_owner()
    }

    fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.inner.current_owner()
    }

    async fn apply(
        self: Box<Self>,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let application = self.inner.apply(event.clone()).await?;
        self.applications
            .lock()
            .unwrap()
            .push((event, application.clone()));
        Ok(application)
    }
}

#[async_trait::async_trait]
impl RuntimeHostBatchExecutionPort for CancelAfterNativeDrain {
    async fn execute_runtime_host_batch_request(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        panic!("native serial workflow must not use an unowned batch fallback")
    }
}

#[async_trait::async_trait]
impl ReservationLifecyclePort for CancelAfterNativeDrain {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.inner.apply_reservation_lifecycle(event).await
    }
}

#[async_trait::async_trait]
impl SerialRuntimeHostBatchExecutionPort for CancelAfterNativeDrain {
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        self.inner.resident_serial_cpu_owner(intent)
    }

    async fn execute_serial_singleton(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
        _: SchedulerSerialBoundDispatch,
    ) -> Result<
        (
            RuntimeHostBatchExecutionResponse,
            SchedulerSerialDrainedDispatch,
            Option<SerialRuntimeHostCpuOwnerEvidence>,
        ),
        RuntimeHostExecutionPortError,
    > {
        panic!("native serial workflow must collect the linear cleanup receipt")
    }

    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected_owner: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.snapshot_cleanup_calls.fetch_add(1, Ordering::SeqCst);
        self.inner.apply_serial_cleanup(event, expected_owner).await
    }

    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        ownership: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        let member = &request.members[0];
        let workflow_run_id = member.handoff.workflow_run_id.to_string();
        let task_id = member.handoff.task_id.to_string();
        let lease_id: u64 = member
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .reservation_lease_id
            .as_str()
            .strip_prefix("runtime-registry.")
            .unwrap()
            .parse()
            .unwrap();
        self.forward_calls.fetch_add(1, Ordering::SeqCst);
        let mut execution = self
            .inner
            .execute_serial_singleton_with_cleanup(request, cancellation.clone(), ownership)
            .await?;
        assert!(execution.cleanup.is_some(), "actual native linear receipt");
        let charged = self.inner.registry.reservation_lease(lease_id).unwrap();
        self.completed
            .lock()
            .unwrap()
            .push((charged, execution.response.clone()));

        // Drop the target lock before awaiting the public cancellation API.
        let target = self.cancel_target.lock().unwrap().take();
        let cancelled = target.is_some();
        if let Some((service, session_id)) = target {
            let response = service
                .upgrade()
                .unwrap()
                .workflow_cancel_active_execution_session_task(
                    pantograph_workflow_service::WorkflowExecutionSessionActiveTaskCancelRequest {
                        session_id,
                        workflow_run_id,
                        task_id,
                        reason: Some("test cancellation after actual native drain".into()),
                    },
                )
                .await
                .unwrap();
            assert!(response.ok);
            assert_eq!(
                response.status,
                pantograph_workflow_service::WorkflowExecutionSessionActiveTaskCancelStatus::CancellationRequested
            );
            assert_eq!(
                cancellation.snapshot().state,
                RuntimeHostExecutionCancellationState::CancellationRequested
            );
        }
        let receipt = execution.cleanup.take().unwrap();
        if cancelled && self.lose_cancelled_receipt {
            // Deliberate collector loss, never a forged cleanup acknowledgement.
            drop(receipt);
        } else {
            execution.cleanup = Some(Box::new(ObserveNativeCleanup {
                inner: receipt,
                applications: self.cleanups.clone(),
            }));
        }
        Ok(execution)
    }
}

fn assert_actual_native_embedding(
    response: &RuntimeHostBatchExecutionResponse,
    golden: &serde_json::Value,
) {
    assert_eq!(response.state, RuntimeHostBatchExecutionState::Completed);
    assert_eq!(response.members.len(), 1);
    let member = &response.members[0];
    assert_eq!(
        member.state,
        RuntimeHostBatchExecutionMemberState::Completed
    );
    let output = member
        .outputs
        .iter()
        .find(|value| value.port_id == "embedding")
        .unwrap();
    let RuntimeHostExecutionOutputValue::Json(vector) = &output.value else {
        panic!("actual native structured embedding output missing")
    };
    let actual = vector.as_array().unwrap();
    let expected = golden["single_vectors"][0].as_array().unwrap();
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!((actual.as_f64().unwrap() - expected.as_f64().unwrap()).abs() < 1e-5);
    }
    let output = member
        .outputs
        .iter()
        .find(|value| value.port_id == "usage")
        .unwrap();
    let RuntimeHostExecutionOutputValue::Json(usage) = &output.value else {
        panic!("actual native token usage missing")
    };
    assert_eq!(usage["total_tokens"], 4);
}

async fn native_post_drain_cancellation_case(lose_receipt: bool) {
    // C4b lifecycle evidence only: readiness/candidate metadata are controlled.
    // This is separate from C2's authoritative provider qualification and from
    // the gateway test that requests cancellation during actual native drain.
    let f = fixture().await;
    let model_ref = f.request.handoff.task_intent.model_ref.clone();
    let port = Arc::new(CancelAfterNativeDrain {
        inner: f.port.clone(),
        cancel_target: Mutex::new(None),
        lose_cancelled_receipt: lose_receipt,
        forward_calls: AtomicUsize::new(0),
        snapshot_cleanup_calls: AtomicUsize::new(0),
        completed: Mutex::new(Vec::new()),
        cleanups: Arc::new(Mutex::new(Vec::new())),
    });
    let artifact_root = TempDir::new().unwrap();
    let provider = DependencyEnvironmentReadinessSnapshotProvider::new();
    let service = Arc::new(
        WorkflowService::new_serial_ready_cpu(port.clone(), WorkflowSerialReadyConfig::default())
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
    let workflow_id = "wf-native-post-drain-cancel";
    let version = service
        .resolve_workflow_graph_version(workflow_id, "1.0.0", &graph)
        .unwrap();
    install_embedding_readiness(&service, &provider, &graph, &version, &model_ref);
    let host = Arc::new(ImageRuntimeSessionHost::new(graph));
    let golden: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.directory.path().join("golden.json")).unwrap())
            .unwrap();
    let mut cancelled_lease = None;
    for attempt in 0..2 {
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
        if attempt == 0 {
            *port.cancel_target.lock().unwrap() =
                Some((Arc::downgrade(&service), created.session_id.clone()));
        }
        let before = f
            .port
            .gateway
            .resident_cpu_serial_owner(&f.target.model_ref)
            .unwrap()
            .serial_facts();
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            pantograph_workflow_service::workflow::WorkflowSessionExecutionRuntime::from_shared_service(service.clone(), host.clone())
                .run_workflow_execution_session(WorkflowExecutionSessionRunRequest {
                    session_id: created.session_id,
                    workflow_semantic_version: "1.0.0".into(),
                    inputs: vec![WorkflowPortBinding { node_id: "prompt".into(), port_id: "text".into(), value: serde_json::json!("hello world") }],
                    output_targets: Some(vec![WorkflowOutputTarget { node_id: "vectors".into(), port_id: "vector".into() }, WorkflowOutputTarget { node_id: "infer".into(), port_id: "usage".into() }]),
                    override_selection: None,
                    timeout_ms: None,
                    priority: None,
                }),
        ).await.expect("bounded native cancellation regression");
        let after = f
            .port
            .gateway
            .resident_cpu_serial_owner(&f.target.model_ref)
            .unwrap()
            .serial_facts();
        assert_eq!(before.loaded_instance, after.loaded_instance);
        let completed = port.completed.lock().unwrap().clone();
        let cleanups = port.cleanups.lock().unwrap().clone();
        if attempt == 0 {
            assert_eq!(before.generation + 2, after.generation);
            assert_eq!(port.forward_calls.load(Ordering::SeqCst), 1);
            assert_eq!(completed.len(), 1);
            assert_actual_native_embedding(&completed[0].1, &golden);
            cancelled_lease = Some(completed[0].0.clone());
            assert!(
                service.serial_ready_observations_for_test().is_empty(),
                "cancelled collection must never train"
            );
        }
        if lose_receipt {
            assert!(
                response.is_err(),
                "no receipt means no acknowledged cleanup or next dispatch"
            );
            assert!(cleanups.is_empty());
            assert!(port.snapshot_cleanup_calls.load(Ordering::SeqCst) >= 1);
            assert_eq!(port.forward_calls.load(Ordering::SeqCst), 1);
            assert_eq!(completed.len(), 1);
            if attempt == 1 {
                assert_eq!(
                    before.generation, after.generation,
                    "next attempt must not forward while fenced"
                );
            }
            let charged = cancelled_lease.as_ref().unwrap();
            assert_eq!(
                f.port
                    .registry
                    .reservation_lease(charged.reservation_id)
                    .as_ref(),
                Some(charged)
            );
            assert!(
                f.port
                    .registry
                    .release_reservation(charged.reservation_id)
                    .is_err(),
                "physical drain alone cannot release execution custody"
            );
            assert!(service.serial_ready_observations_for_test().is_empty());
            assert!(f
                .port
                .registry
                .snapshot()
                .reservations
                .contains(&f.successor));
        } else {
            // Cancellation arrived after physical completion: do not fabricate
            // a Cancelled native response or imply cancellation during compute.
            response.expect("completed output may win late cancellation only with matched cleanup");
            assert_eq!(port.snapshot_cleanup_calls.load(Ordering::SeqCst), 0);
            assert_eq!(port.forward_calls.load(Ordering::SeqCst), attempt + 1);
            assert_eq!(completed.len(), attempt + 1);
            assert_eq!(cleanups.len(), attempt + 1);
            for ((event, application), (lease, response)) in cleanups.iter().zip(&completed) {
                assert_actual_native_embedding(response, &golden);
                assert_eq!(event.outcome, ReservationLifecycleOutcome::RetryDeferred);
                assert_eq!(
                    application.state,
                    ReservationLifecycleApplicationState::Applied
                );
                assert_eq!(application.lifecycle_event_id, event.lifecycle_event_id);
                assert_eq!(application.reservation_lease_id, event.reservation_lease_id);
                assert_eq!(
                    event.reservation_lease_id.as_str(),
                    format!("runtime-registry.{}", lease.reservation_id)
                );
                assert!(f
                    .port
                    .registry
                    .reservation_lease(lease.reservation_id)
                    .is_none());
            }
            assert_eq!(
                f.port.registry.snapshot().reservations,
                vec![f.successor.clone()]
            );
            if attempt == 1 {
                assert_eq!(before.generation + 2, after.generation);
                assert_eq!(
                    service.serial_ready_observations_for_test(),
                    vec![(after.generation, 1)],
                    "only the subsequent uncancelled attempt trains"
                );
            }
        }
        assert_eq!(host.runtime_load_attempts.load(Ordering::SeqCst), 0);
        assert_eq!(host.run_attempts.load(Ordering::SeqCst), 0);
    }
    f.port.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn actual_cpu_post_drain_cancellation_skips_training_and_matched_cleanup_allows_next_attempt()
{
    native_post_drain_cancellation_case(false).await;
}

#[tokio::test]
async fn actual_cpu_post_drain_cancellation_with_lost_receipt_fences_next_attempt() {
    native_post_drain_cancellation_case(true).await;
}
