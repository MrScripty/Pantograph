use super::*;
use crate::runtime_host_embedding_execution::tests::{fixture as model_fixture, Package, Target};
use pantograph_runtime_registry::*;
use std::time::Duration;

struct PausedPackage {
    package: inference::ResolvedModelPackageFacts,
    entered: Arc<tokio::sync::Notify>,
    proceed: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl RuntimeHostPackageFactsResolver for PausedPackage {
    async fn resolve(
        &self,
        _: &ValidatedRuntimeHostExecutionRequest,
    ) -> Result<
        inference::ResolvedModelPackageFacts,
        crate::runtime_host_package_facts::RuntimeHostPumasPackageFactsError,
    > {
        self.entered.notify_one();
        self.proceed.notified().await;
        Ok(self.package.clone())
    }
}

fn shrink_task_claim(f: &Fixture, provisional: bool) -> RuntimeReservationLease {
    let replacement = RuntimeReservationRequest {
        runtime_id: "candle".into(),
        workflow_id: f.request.handoff.workflow_id.to_string(),
        reservation_owner_id: Some("native.aba".into()),
        usage_profile: None,
        model_id: Some(f.target.model_ref.model_id.clone()),
        pin_runtime: false,
        requirements: Some(RuntimeReservationRequirements::from_claims(vec![
            RuntimeReservationResourceClaim::ram_bytes(1),
        ])),
        retention_hint: RuntimeRetentionHint::Ephemeral,
    };
    if provisional {
        let expected = f
            .port
            .registry
            .evaluate_reservation(replacement.clone())
            .unwrap()
            .observation()
            .clone();
        let (lease, custody) = f
            .port
            .registry
            .acquire_reservation_provisional(replacement, &expected, |lease| {
                Ok::<_, ()>(lease.clone())
            })
            .unwrap();
        custody.transfer().unwrap();
        lease
    } else {
        f.port.registry.acquire_reservation(replacement).unwrap()
    }
}

#[tokio::test]
async fn selected_incarnation_replaced_before_host_entry_refuses_native_forward() {
    for provisional in [false, true] {
        let f = fixture().await;
        let old = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.aba");
        let request = batch(&f, &old);
        let owner_before = f
            .port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        let admission = SchedulerSerialAdmission::new();
        let bound = bind(&admission, &request, None);
        let new = shrink_task_claim(&f, provisional);
        let result = f
            .port
            .execute_serial_singleton_with_cleanup(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                ),
                bound,
            )
            .await;
        eprintln!(
            "before_entry provisional={provisional} leases_equal={} dispatch_accepted={}",
            old == new,
            result.is_ok()
        );
        assert!(
            result.is_err(),
            "selected admission was replaced before host entry"
        );
        assert_eq!(
            f.port
                .resident_serial_cpu_owner(&f.request.handoff.task_intent)
                .unwrap()
                .snapshot,
            owner_before
        );
        assert_eq!(
            f.port.registry.reservation_lease(new.reservation_id),
            Some(new)
        );
    }
}

#[tokio::test]
async fn selected_incarnation_replaced_during_awaited_resolver_refuses_native_forward() {
    for provisional in [false, true] {
        let f = fixture().await;
        let old = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.aba");
        let request = batch(&f, &old);
        let owner_before = f
            .port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        let admission = SchedulerSerialAdmission::new();
        let bound = bind(&admission, &request, None);
        let entered = Arc::new(tokio::sync::Notify::new());
        let proceed = Arc::new(tokio::sync::Notify::new());
        let port = Arc::new(EmbeddedRetainedCpuSerialPort {
            gateway: f.port.gateway.clone(),
            registry: f.port.registry.clone(),
            package: Arc::new(PausedPackage {
                package: f.package.clone(),
                entered: entered.clone(),
                proceed: proceed.clone(),
            }),
            target: f.port.target.clone(),
            session_reservations: None,
            #[cfg(feature = "native-task-release")]
            release_observer: None,
        });
        let execution = tokio::spawn(async move {
            port.execute_serial_singleton_with_cleanup(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                ),
                bound,
            )
            .await
        });
        tokio::time::timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        let new = shrink_task_claim(&f, provisional);
        proceed.notify_one();
        let result = tokio::time::timeout(Duration::from_secs(5), execution)
            .await
            .unwrap()
            .unwrap();
        eprintln!(
            "during_resolver provisional={provisional} leases_equal={} dispatch_accepted={}",
            old == new,
            result.is_ok()
        );
        assert!(
            result.is_err(),
            "admitted incarnation changed while resolver was suspended"
        );
        assert_eq!(
            f.port
                .resident_serial_cpu_owner(&f.request.handoff.task_intent)
                .unwrap()
                .snapshot,
            owner_before
        );
        assert_eq!(
            f.port.registry.reservation_lease(new.reservation_id),
            Some(new)
        );
    }
}

pub(crate) struct Fixture {
    pub directory: tempfile::TempDir,
    pub port: Arc<EmbeddedRetainedCpuSerialPort>,
    pub request: RuntimeHostExecutionRequest,
    pub package: inference::ResolvedModelPackageFacts,
    pub target: inference::PumasArtifactLoadTarget,
    pub successor: RuntimeReservationLease,
}
pub(crate) async fn fixture() -> Fixture {
    let (directory, request, package, target) = model_fixture(8);
    let gateway = Arc::new(
        inference::InferenceGateway::new_calibrated_candle_cpu(
            inference::CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap(),
        )
        .unwrap(),
    );
    let projection = crate::runtime_host_embedding_execution::project_runtime_host_embedding(
        &ValidatedRuntimeHostExecutionRequest::try_from(request.clone()).unwrap(),
        package.clone(),
        target.clone(),
    )
    .unwrap();
    gateway
        .execute_selected_embedding_with_cancellation(
            projection.request,
            projection.target,
            projection.decision,
            inference::InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let registry = Arc::new(RuntimeRegistry::new());
    registry
        .configure_model_resident_estimates(vec![RuntimeModelResidentEstimate {
            runtime_id: "candle".into(),
            model_id: target.local_load_path.clone(),
            requirements: RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(32768),
            ]),
        }])
        .unwrap();
    let successor = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: request.handoff.workflow_id.to_string(),
            reservation_owner_id: Some("session.successor".into()),
            usage_profile: None,
            model_id: Some(target.model_ref.model_id.clone()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(256),
            ])),
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .unwrap();
    gateway
        .publish_resident_cpu_cleanup_owner(&registry)
        .await
        .unwrap();
    let port = Arc::new(EmbeddedRetainedCpuSerialPort {
        gateway,
        registry,
        package: Arc::new(Package(package.clone())),
        target: Arc::new(Target(
            serde_json::from_value(serde_json::to_value(&target).unwrap()).unwrap(),
        )),
        session_reservations: None,
        #[cfg(feature = "native-task-release")]
        release_observer: None,
    });
    Fixture {
        directory,
        port,
        request,
        package,
        target,
        successor,
    }
}
pub(crate) fn task_lease(f: &Fixture, workflow: &str, owner: &str) -> RuntimeReservationLease {
    f.port
        .registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: workflow.into(),
            reservation_owner_id: Some(owner.into()),
            usage_profile: None,
            model_id: Some(f.target.model_ref.model_id.clone()),
            pin_runtime: false,
            requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(256),
            ])),
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .unwrap()
}
fn batch(f: &Fixture, lease: &RuntimeReservationLease) -> RuntimeHostBatchExecutionRequest {
    let mut handoff = f.request.handoff.clone();
    let decision = handoff.dispatch_decision.as_mut().unwrap();
    let lease_id =
        SchedulerReservationLeaseId::parse(format!("runtime-registry.{}", lease.reservation_id))
            .unwrap();
    decision.reservation_lease_id = lease_id.clone();
    for reservation in &mut decision.reservations {
        reservation.reservation_lease_id = lease_id.clone();
    }
    RuntimeHostBatchExecutionRequest {
        contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
        batch_execution_request_id: "native.batch".into(),
        anchor_execution_request_id: f.request.execution_request_id.clone(),
        cancellation_context: f.request.cancellation_context.clone(),
        members: vec![RuntimeHostBatchExecutionMemberRequest {
            execution_request_id: f.request.execution_request_id.clone(),
            assignment_id: "native.assignment".into(),
            handoff,
            materialized_inputs: f.request.materialized_inputs.clone(),
            timeout_ms: None,
            failure_policy: RuntimeHostBatchMemberFailurePolicy::TerminalOnly,
            reservation_policy: RuntimeHostBatchMemberReservationPolicy::ReleaseOnTerminal,
        }],
    }
}
fn bind(
    admission: &SchedulerSerialAdmission,
    request: &RuntimeHostBatchExecutionRequest,
    owner: Option<SchedulerSerialOwnerSnapshot>,
) -> SchedulerSerialBoundDispatch {
    let m = &request.members[0];
    let id = SchedulerSerialAttemptIdentity {
        workflow_id: m.handoff.workflow_id.as_str(),
        workflow_run_id: m.handoff.workflow_run_id.as_str(),
        node_id: m.handoff.node_id.as_str(),
        task_id: m.handoff.task_id.as_str(),
        attempt_id: "native.attempt",
        execution_request_id: &m.execution_request_id,
        candidate_id: "native.candidate",
        reservation_lease_id: m
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .reservation_lease_id
            .as_str(),
    };
    let dispatch = admission.try_prepare().unwrap().begin_dispatch();
    match owner {
        Some(owner) => dispatch.bind_attempt(id, owner),
        None => dispatch.bind_unranked_attempt(id),
    }
    .unwrap()
}
fn cleanup_event(request: &RuntimeHostBatchExecutionRequest) -> ReservationLifecycleEvent {
    let m = &request.members[0];
    ReservationLifecycleEvent {
        contract_version: 1,
        lifecycle_event_id: "native.cleanup".into(),
        reservation_lease_id: m
            .handoff
            .dispatch_decision
            .as_ref()
            .unwrap()
            .reservation_lease_id
            .clone(),
        workflow_id: m.handoff.workflow_id.clone(),
        workflow_run_id: m.handoff.workflow_run_id.clone(),
        node_id: m.handoff.node_id.clone(),
        task_id: m.handoff.task_id.clone(),
        outcome: ReservationLifecycleOutcome::RuntimeHostCompleted,
        candidate_id: Some(SchedulerDispatchCandidateId::parse("native.candidate").unwrap()),
        diagnostics: Vec::new(),
    }
}
#[tokio::test]
async fn actual_native_serial_dispatch_consumes_receipt_and_only_completed_lease() {
    let f = fixture().await;
    let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.task");
    let request = batch(&f, &lease);
    let old = f
        .port
        .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
        .unwrap();
    let admission = SchedulerSerialAdmission::new();
    let bound = bind(&admission, &request, Some(old.snapshot));
    let execution = tokio::time::timeout(
        Duration::from_secs(5),
        f.port.execute_serial_singleton_with_cleanup(
            request.clone(),
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone()),
            bound,
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        execution.response.state,
        RuntimeHostBatchExecutionState::Completed
    );
    let actual = execution.owner.unwrap();
    assert_eq!(old.snapshot.generation + 2, actual.snapshot.generation);
    assert_eq!(
        old.snapshot.loaded_instance,
        actual.snapshot.loaded_instance
    );
    assert!(f
        .port
        .registry
        .release_reservation(lease.reservation_id)
        .is_err());
    let outputs = &execution.response.members[0].outputs;
    let RuntimeHostExecutionOutputValue::Json(vector) = &outputs[0].value else {
        panic!("vector")
    };
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
    assert!(outputs.iter().any(|o| o.port_id == "usage"));
    let event = cleanup_event(&request);
    let cleanup_id = SchedulerSerialCleanupEvent {
        identity: SchedulerSerialAttemptIdentity {
            workflow_id: event.workflow_id.as_str(),
            workflow_run_id: event.workflow_run_id.as_str(),
            node_id: event.node_id.as_str(),
            task_id: event.task_id.as_str(),
            attempt_id: "native.attempt",
            execution_request_id: &request.members[0].execution_request_id,
            candidate_id: "native.candidate",
            reservation_lease_id: event.reservation_lease_id.as_str(),
        },
        lifecycle_event_id: &event.lifecycle_event_id,
        releases_reservation: true,
    };
    let pending = execution.drained.expect_cleanup(cleanup_id).unwrap();
    let applied = execution.cleanup.unwrap().apply(event).await.unwrap();
    pending
        .acknowledge_cleanup(
            &applied.lifecycle_event_id,
            applied.reservation_lease_id.as_str(),
            SchedulerSerialCleanupState::Applied,
        )
        .unwrap();
    assert!(admission.try_prepare().is_ok());
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_none());
    assert_eq!(
        f.port
            .registry
            .reservation_lease(f.successor.reservation_id),
        Some(f.successor)
    );
    assert_eq!(
        f.port.registry.snapshot().runtimes[0]
            .model_resource_residency
            .as_ref()
            .unwrap()
            .requirements
            .as_ref()
            .unwrap()
            .claims
            .len(),
        1
    );
    f.port.gateway.stop().await.unwrap();
}
#[tokio::test]
async fn unknown_or_changed_native_owner_withholds_fallback_and_serial_reopens_never() {
    let f = fixture().await;
    let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.task");
    let request = batch(&f, &lease);
    let old = f
        .port
        .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
        .unwrap();
    let admission = SchedulerSerialAdmission::new();
    let bound = bind(&admission, &request, Some(old.snapshot));
    f.port.gateway.stop().await.unwrap();
    assert!(f
        .port
        .execute_serial_singleton_with_cleanup(
            request.clone(),
            RuntimeHostExecutionCancellationHandle::running(request.cancellation_context.clone()),
            bound
        )
        .await
        .is_err());
    assert!(admission.try_prepare().is_err());
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease)
    );
    assert!(!f.port.gateway.is_ready().await);
}
#[tokio::test]
async fn abandoned_receipt_foreign_cleanup_and_intervening_reload_leave_selected_charge_fenced() {
    for fault in ["drop", "foreign", "reload", "last"] {
        let f = fixture().await;
        let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "native.task");
        let request = batch(&f, &lease);
        let admission = SchedulerSerialAdmission::new();
        let execution = f
            .port
            .execute_serial_singleton_with_cleanup(
                request.clone(),
                RuntimeHostExecutionCancellationHandle::running(
                    request.cancellation_context.clone(),
                ),
                bind(&admission, &request, None),
            )
            .await
            .unwrap();
        let receipt = execution.cleanup.unwrap();
        if fault == "drop" {
            drop(receipt);
        } else {
            let mut event = cleanup_event(&request);
            if fault == "foreign" {
                event.candidate_id = Some(SchedulerDispatchCandidateId::parse("foreign").unwrap());
            }
            if fault == "last" {
                f.port
                    .registry
                    .release_reservation(f.successor.reservation_id)
                    .unwrap();
            }
            if fault == "reload" {
                let projection =
                    crate::runtime_host_embedding_execution::project_runtime_host_embedding(
                        &ValidatedRuntimeHostExecutionRequest::try_from(f.request.clone()).unwrap(),
                        f.package.clone(),
                        f.target.clone(),
                    )
                    .unwrap();
                f.port
                    .gateway
                    .execute_selected_embedding_with_cancellation(
                        projection.request,
                        projection.target,
                        projection.decision,
                        inference::InferenceExecutionCancellationHandle::running(),
                    )
                    .await
                    .unwrap();
            }
            assert!(receipt.apply(event).await.is_err());
        }
        assert!(f
            .port
            .registry
            .release_reservation(lease.reservation_id)
            .is_err());
        assert_eq!(
            f.port.registry.reservation_lease(lease.reservation_id),
            Some(lease)
        );
        assert!(admission.try_prepare().is_err());
        f.port.gateway.stop().await.unwrap();
    }
}

#[cfg(feature = "native-task-release")]
#[path = "native_task_release_tests.rs"]
mod native_task_release_tests;

#[cfg(feature = "native-task-release")]
#[path = "native_task_start_tests.rs"]
mod native_task_start_tests;
