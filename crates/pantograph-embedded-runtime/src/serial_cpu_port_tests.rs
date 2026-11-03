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
            residency: None,
            after_load_gate: std::sync::Mutex::new(None),
            after_drain_gate: std::sync::Mutex::new(None),
            gateway: f.port.gateway.clone(),
            registry: f.port.registry.clone(),
            package: Arc::new(PausedPackage {
                package: f.package.clone(),
                entered: entered.clone(),
                proceed: proceed.clone(),
            }),
            target: f.port.target.clone(),
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
        residency: None,
        after_load_gate: std::sync::Mutex::new(None),
        after_drain_gate: std::sync::Mutex::new(None),
        gateway,
        registry,
        package: Arc::new(Package(package.clone())),
        target: Arc::new(Target(
            serde_json::from_value(serde_json::to_value(&target).unwrap()).unwrap(),
        )),
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

/// Controlled admission metadata; this test does not qualify physical memory.
#[tokio::test]
async fn cold_residency_headroom_requires_both_additive_kinds_and_no_old_lease() {
    let f = fixture().await;
    let declaration = CpuResidencyDeclaration {
        target: f.target.clone(),
        resident_ram_bytes: 100,
        load_peak_ram_bytes: 40,
        load_peak_vram_bytes: 0,
    };
    let registry = Arc::new(RuntimeRegistry::new());
    let register = |ram, vram| {
        let mut resources = vec![RuntimeAdmissionResourceBudget::ram_bytes(ram)];
        if let Some(vram) = vram {
            resources.push(RuntimeAdmissionResourceBudget::vram_bytes(Some(vram)));
        }
        registry.register_runtime(
            RuntimeRegistration::new("candle", "controlled CPU")
                .with_admission_budget(RuntimeAdmissionBudget::from_resources(resources)),
        );
    };
    register(Some(150), Some(10));
    assert!(validate_residency_headroom(&registry, &declaration, (10, 10)).is_ok());
    register(Some(149), Some(10));
    assert!(validate_residency_headroom(&registry, &declaration, (10, 10)).is_err());
    register(Some(150), None);
    assert!(validate_residency_headroom(&registry, &declaration, (10, 10)).is_err());
    register(Some(150), Some(9));
    assert!(validate_residency_headroom(&registry, &declaration, (10, 10)).is_err());
    register(Some(150), Some(10));
    assert!(validate_residency_headroom(&registry, &declaration, (u64::MAX, u64::MAX)).is_err());
    let old = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "candle".into(),
            workflow_id: "controlled".into(),
            reservation_owner_id: None,
            usage_profile: None,
            model_id: None,
            pin_runtime: true,
            requirements: Some(RuntimeReservationRequirements::from_claims(vec![
                RuntimeReservationResourceClaim::ram_bytes(1),
                RuntimeReservationResourceClaim::vram_bytes(0),
            ])),
            retention_hint: RuntimeRetentionHint::KeepAlive,
        })
        .unwrap();
    assert!(validate_residency_headroom(&registry, &declaration, (10, 10)).is_err());
    assert_eq!(registry.reservation_lease(old.reservation_id).unwrap(), old);
    f.port.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn preparation_uses_actual_per_kind_maxima_and_refuses_unknown_or_shrunk_claims() {
    let f = fixture().await;
    let mut package = f.package.clone();
    package.artifact.logical_size = Some(inference::PackageLogicalSizeFacts {
        total_size_bytes: Some(1024),
        files: Vec::new(),
        diagnostics: Vec::new(),
        value_source: inference::PackageFactValueSource::FilesystemMetadata,
    });
    let floor =
        crate::inference_resource_estimator::conservative_loaded_runtime_memory_estimate_bytes(
            package.artifact.logical_size.as_ref().unwrap(),
        )
        .unwrap()
            + 512 * 1024 * 1024;
    let mut intent = f.request.handoff.task_intent.clone();
    intent.estimate_hints = vec![
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakRamBytes,
            value: floor + 111,
        },
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakRamBytes,
            value: floor + 222,
        },
        SchedulerEstimateHint {
            kind: SchedulerEstimateHintKind::PeakVramBytes,
            value: floor + 333,
        },
    ];
    assert_eq!(
        cpu_task_claims(&intent, &package).unwrap(),
        (floor + 222, floor + 333)
    );
    intent.estimate_hints.pop();
    assert!(cpu_task_claims(&intent, &package).is_err());
    intent.estimate_hints.push(SchedulerEstimateHint {
        kind: SchedulerEstimateHintKind::PeakVramBytes,
        value: floor - 1,
    });
    assert!(cpu_task_claims(&intent, &package).is_err());
    f.port.gateway.stop().await.unwrap();
}

#[tokio::test]
async fn residency_started_ack_does_not_release_and_unstarted_refusal_cannot_stop_or_release_guard()
{
    let f = fixture().await;
    let task = &f.request.handoff;
    let owner = format!("{}:{}", task.workflow_run_id, task.task_id);
    let lease = task_lease(&f, task.workflow_id.as_str(), &owner);
    let request = batch(&f, &lease);
    let before = f
        .port
        .gateway
        .resident_cpu_serial_owner(&f.target.model_ref)
        .unwrap()
        .serial_facts();
    let mut event = cleanup_event(&request);
    event.outcome = ReservationLifecycleOutcome::DispatchStarted;
    assert_eq!(
        f.port
            .apply_residency_task_lifecycle(event.clone())
            .unwrap()
            .state,
        ReservationLifecycleApplicationState::Applied
    );
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease.clone())
    );
    event.outcome = ReservationLifecycleOutcome::DuplicateReplay;
    assert_eq!(
        f.port
            .apply_residency_task_lifecycle(event.clone())
            .unwrap()
            .state,
        ReservationLifecycleApplicationState::AlreadyApplied
    );
    event.outcome = ReservationLifecycleOutcome::RuntimeHostCompleted;
    assert!(f
        .port
        .apply_residency_task_lifecycle(event.clone())
        .is_err());
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease.clone())
    );
    event.outcome = ReservationLifecycleOutcome::RuntimeHostDispatchRejected;
    event.diagnostics.push(ReservationLifecycleDiagnostic {
        severity: ReservationLifecycleDiagnosticSeverity::Info,
        code: ReservationLifecycleDiagnosticCode::RuntimeHostRejected,
        message: "controlled unstarted host refusal".into(),
        hint: None,
    });
    let mut foreign = event.clone();
    foreign.reservation_lease_id = SchedulerReservationLeaseId::parse(format!(
        "runtime-registry.{}",
        f.successor.reservation_id
    ))
    .unwrap();
    assert!(f.port.apply_residency_task_lifecycle(foreign).is_err());
    assert!(f
        .port
        .registry
        .reservation_lease(f.successor.reservation_id)
        .is_some());
    f.port.apply_residency_task_lifecycle(event).unwrap();
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_none());
    let after = f
        .port
        .gateway
        .resident_cpu_serial_owner(&f.target.model_ref)
        .unwrap()
        .serial_facts();
    assert_eq!(before.loaded_instance, after.loaded_instance);
    assert_eq!(before.generation, after.generation);
    assert!(f.port.registry.snapshot().runtimes[0]
        .model_resource_residency
        .is_some());
    f.port.gateway.stop().await.unwrap();
}
