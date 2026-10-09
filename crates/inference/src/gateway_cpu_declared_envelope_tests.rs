use super::*;
use pantograph_runtime_host_contracts::{
    RuntimeHostTaskStartAuthority, RuntimeHostTaskStartDisposition,
};
use pantograph_runtime_registry::{
    RuntimeAdmissionResourceKind, RuntimeHostRamCapacitySource, RuntimeResourceDomain,
    RuntimeResourceDomainBinding,
};
#[derive(Debug)]
struct Ceiling(std::sync::atomic::AtomicU64);
impl RuntimeHostRamCapacitySource for Ceiling {
    fn capacity_ceiling_bytes(&self) -> Option<u64> {
        match self.0.load(Ordering::Acquire) {
            u64::MAX => None,
            n => Some(n),
        }
    }
}
fn configure(f: &Fixture) -> Arc<Ceiling> {
    f.registry
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
    f.registry.bind_host_ram_capacity_source(ceiling.clone());
    ceiling
}
fn handle(
    a: &RuntimeHostTaskStartAuthority,
    cancel: &Arc<AtomicBool>,
) -> InferenceExecutionCancellationHandle {
    InferenceExecutionCancellationHandle::with_signal(Arc::new(AuthorizedCancelSignal {
        cancellation: CancelSignal(cancel.clone()),
        authority: a.clone(),
    }))
}
struct OpenPhaseGateOnDrop(Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>);
impl OpenPhaseGateOnDrop {
    fn open(&self) {
        let (flag, changed) = self.0.as_ref();
        let mut open = flag.lock().unwrap_or_else(|error| error.into_inner());
        *open = true;
        drop(open);
        changed.notify_all();
    }
}
impl Drop for OpenPhaseGateOnDrop {
    fn drop(&mut self) {
        self.open();
    }
}
async fn declared_boundary(phase: &'static str, mode: &str) {
    let f = fixture().await;
    let ceiling = configure(&f);
    let id = identity(&f.task);
    let a = RuntimeHostTaskStartAuthority::new(id.task_id, id.attempt_id).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let entered = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new()));
    // A failed assertion or timeout must not strand the blocking supervisor.
    let gate_opener = OpenPhaseGateOnDrop(gate.clone());
    let g = gate.clone();
    let e = entered.clone();
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = Some(Arc::new(move |current| {
        if current == phase {
            e.notify_one();
            let mut done = g.0.lock().unwrap();
            while !*done {
                done = g.1.wait(done).unwrap();
            }
        }
    }));
    let before = state(&f.registry);
    let owner = f.owner.serial_facts();
    let gateway = f.gateway.clone();
    let registry = f.registry.clone();
    let request = f.request.clone();
    let target = f.target.clone();
    let decision = f.decision.clone();
    let task_lease = f.task.clone();
    let retaining = f.successor.clone();
    let stamp = f.owner.clone();
    let h = handle(&a, &cancel);
    let job = tokio::spawn(async move {
        gateway
            .execute_declared_envelope_cpu_warm_attempt(
                registry,
                CandleCpuWarmAttemptRequest {
                    request,
                    target,
                    decision,
                    identity: identity(&task_lease),
                    lease: &task_lease,
                    expected_owner: &stamp,
                },
                &retaining,
                h,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert!(f.gateway.backend.try_write().is_err());
    assert_eq!(
        a.disposition(),
        if phase == "declared_prestart" {
            RuntimeHostTaskStartDisposition::Prepared
        } else {
            RuntimeHostTaskStartDisposition::StartedOrUncertain
        }
    );
    assert!(f
        .registry
        .release_reservation(f.task.reservation_id)
        .is_err());
    assert!(f
        .registry
        .release_reservation(f.successor.reservation_id)
        .is_err());
    assert!(f.registry.acquire_execution_custody(&f.task).is_err());
    assert!(f.registry.acquire_execution_custody(&f.successor).is_err());
    match mode {
        "cancel" => {
            a.revoke();
            cancel.store(true, Ordering::Release);
        }
        "stale_owner" => {
            a.revoke();
        }
        "abort" => job.abort(),
        "capacity_lost" => ceiling.0.store(33279, Ordering::Release),
        "capacity_unknown" => ceiling.0.store(u64::MAX, Ordering::Release),
        "success" => {}
        _ => panic!("unknown fixture mode"),
    };
    assert_eq!(before, state(&f.registry));
    let result = if mode == "abort" {
        // abort() only requests cancellation. Join while the supervisor remains
        // behind its barrier, so collector Drop revokes the SAME start cell first.
        let result = tokio::time::timeout(Duration::from_secs(5), job)
            .await
            .expect("collector cancellation completes before opening phase barrier");
        assert!(matches!(&result, Err(error) if error.is_cancelled()));
        assert!(!*gate.0.lock().unwrap());
        assert!(f.gateway.backend.try_write().is_err());
        assert_eq!(
            a.disposition(),
            if phase == "declared_prestart" {
                RuntimeHostTaskStartDisposition::NoStartAuthorized
            } else {
                RuntimeHostTaskStartDisposition::StartedOrUncertain
            }
        );
        if phase == "declared_prestart" {
            assert!(!a.try_start(id.task_id, id.attempt_id));
        }
        assert_eq!(before, state(&f.registry));
        gate_opener.open();
        result
    } else {
        gate_opener.open();
        job.await
    };
    if mode == "abort" {
        assert!(matches!(result, Err(error) if error.is_cancelled()));
        let backend = tokio::time::timeout(Duration::from_secs(5), f.gateway.backend.write())
            .await
            .unwrap();
        drop(backend);
        assert_eq!(
            a.disposition(),
            if phase == "declared_prestart" {
                RuntimeHostTaskStartDisposition::NoStartAuthorized
            } else {
                RuntimeHostTaskStartDisposition::StartedOrUncertain
            }
        );
        assert_eq!(before, state(&f.registry));
        if phase == "declared_prestart" {
            let fresh = f
                .gateway
                .resident_cpu_serial_owner(f.owner.model_ref())
                .unwrap()
                .serial_facts();
            assert_eq!(fresh.generation, owner.generation);
            assert_eq!(fresh.loaded_instance, owner.loaded_instance);
            // Writer idleness does not attest supervisor Drop completion or settlement.
            // Original claims remain charged; no collector has an ACK.
            assert_eq!(
                f.registry.reservation_lease(f.task.reservation_id),
                Some(f.task.clone())
            );
            assert_eq!(
                f.registry.reservation_lease(f.successor.reservation_id),
                Some(f.successor.clone())
            );
        } else {
            assert!(f.registry.acquire_execution_custody(&f.task).is_err());
            assert!(f.registry.acquire_execution_custody(&f.successor).is_err());
        }
    } else if phase == "declared_prestart" && mode != "success" {
        let CandleCpuDeclaredAttemptOutcome::NoStart { receipt, .. } = result.unwrap().unwrap()
        else {
            panic!("no-start receipt required")
        };
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::NoStartAuthorized
        );
        assert!(!a.try_start(id.task_id, id.attempt_id));
        let fresh = f
            .gateway
            .resident_cpu_serial_owner(f.owner.model_ref())
            .unwrap()
            .serial_facts();
        assert_eq!(fresh.generation, owner.generation);
        assert_eq!(fresh.loaded_instance, owner.loaded_instance);
        assert!(f
            .registry
            .release_reservation(f.task.reservation_id)
            .is_err());
        assert!(f
            .registry
            .release_reservation(f.successor.reservation_id)
            .is_err());
        if mode == "stale_owner" {
            f.gateway
                .execute_selected_embedding_with_cancellation(
                    f.request.clone(),
                    f.target.clone(),
                    f.decision.clone(),
                    InferenceExecutionCancellationHandle::running(),
                )
                .await
                .unwrap();
            assert!(receipt.settle_retained().await.is_err());
            assert_eq!(before, state(&f.registry));
            assert_eq!(
                a.disposition(),
                RuntimeHostTaskStartDisposition::NoStartAuthorized
            );
            *f.gateway
                .candle_cpu_calibration
                .as_ref()
                .unwrap()
                .attempt_hook
                .lock()
                .unwrap() = None;
            f.gateway.stop().await.unwrap();
            return;
        }
        let proof = receipt.settle_retained().await.unwrap();
        assert!(proof.matches_authority(&f.gateway, &f.registry));
        assert_eq!(proof.task_lease(), &f.task);
        assert_eq!(proof.retaining_lease(), &f.successor);
        assert_eq!(proof.task_id(), id.task_id);
        assert_eq!(proof.attempt_id(), id.attempt_id);
        assert_eq!(
            proof.input_fingerprint(),
            *blake3::hash(match &f.request.input {
                crate::InferenceExecutionInput::Embedding { texts } => texts[0].as_bytes(),
                _ => unreachable!(),
            })
            .as_bytes()
        );
        assert!(f
            .registry
            .reservation_lease(f.task.reservation_id)
            .is_none());
        assert_eq!(
            f.registry.reservation_lease(f.successor.reservation_id),
            Some(f.successor.clone())
        );
        assert_eq!(
            before.0[0].model_resource_residency,
            state(&f.registry).0[0].model_resource_residency
        );
    } else if mode == "success" {
        let CandleCpuDeclaredAttemptOutcome::Drained { receipt, .. } = result.unwrap().unwrap()
        else {
            panic!("actual drain required")
        };
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::StartedOrUncertain
        );
        assert!(f
            .registry
            .release_reservation(f.successor.reservation_id)
            .is_err());
        receipt.release_retained().await.unwrap();
        assert!(f
            .registry
            .reservation_lease(f.task.reservation_id)
            .is_none());
        assert_eq!(
            f.registry.reservation_lease(f.successor.reservation_id),
            Some(f.successor.clone())
        );
    } else {
        assert!(result.unwrap().is_err());
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::StartedOrUncertain
        );
        assert_eq!(before, state(&f.registry));
        assert!(f
            .registry
            .release_reservation(f.task.reservation_id)
            .is_err());
        assert!(f
            .registry
            .release_reservation(f.successor.reservation_id)
            .is_err());
        // Load cancellation may leave no attested private owner. Never synthesize
        // one for recovery; even the original stamp must remain unable to run.
        let fresh = f.gateway.resident_cpu_serial_owner(f.owner.model_ref());
        let recovery_authority =
            RuntimeHostTaskStartAuthority::new(id.task_id, "recovery-attempt").unwrap();
        let mut input = f.input();
        input.expected_owner = fresh.as_ref().unwrap_or(&f.owner);
        input.identity.attempt_id = "recovery-attempt";
        assert!(f
            .gateway
            .execute_declared_envelope_cpu_warm_attempt(
                f.registry.clone(),
                input,
                &f.successor,
                handle(&recovery_authority, &Arc::new(AtomicBool::new(false)))
            )
            .await
            .is_err());
        assert_eq!(
            recovery_authority.disposition(),
            RuntimeHostTaskStartDisposition::NoStartAuthorized
        );
        assert_eq!(before, state(&f.registry));
    }
    *f.gateway
        .candle_cpu_calibration
        .as_ref()
        .unwrap()
        .attempt_hook
        .lock()
        .unwrap() = None;
    f.gateway.stop().await.unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_prestart_cancellation_mints_sealed_exact_settlement() {
    declared_boundary("declared_prestart", "cancel").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_final_capacity_loss_and_unknown_refuse_before_start() {
    for mode in ["capacity_lost", "capacity_unknown"] {
        declared_boundary("declared_prestart", mode).await;
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_prestart_collector_loss_retains_claims_without_settlement() {
    declared_boundary("declared_prestart", "abort").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_after_start_cancellation_and_collector_loss_fence_complete_declared_claims(
) {
    for phase in ["load", "forward", "drain"] {
        for mode in ["cancel", "abort"] {
            declared_boundary(phase, mode).await;
        }
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_actual_drain_consumes_envelope_cleanup_and_keeps_retaining_lease() {
    declared_boundary("drain", "success").await;
}
#[tokio::test]
async fn declared_native_foreign_or_absent_authority_and_wrong_retaining_lease_refuse() {
    let f = fixture().await;
    configure(&f);
    let before = state(&f.registry);
    let cancel = Arc::new(AtomicBool::new(false));
    let foreign = RuntimeHostTaskStartAuthority::new("foreign", "foreign").unwrap();
    assert!(f
        .gateway
        .execute_declared_envelope_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            &f.successor,
            handle(&foreign, &cancel)
        )
        .await
        .is_err());
    assert_eq!(
        foreign.disposition(),
        RuntimeHostTaskStartDisposition::Prepared
    );
    assert!(f
        .gateway
        .execute_declared_envelope_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            &f.successor,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    let id = identity(&f.task);
    let authority = RuntimeHostTaskStartAuthority::new(id.task_id, id.attempt_id).unwrap();
    assert!(f
        .gateway
        .execute_declared_envelope_cpu_warm_attempt(
            f.registry.clone(),
            f.input(),
            &f.task,
            handle(&authority, &cancel)
        )
        .await
        .is_err());
    assert_eq!(before, state(&f.registry));
    f.gateway.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_drain_capacity_loss_and_unknown_leave_both_claims_fenced() {
    for mode in ["capacity_lost", "capacity_unknown"] {
        declared_boundary("drain", mode).await;
    }
}
#[tokio::test]
async fn declared_native_post_receipt_capacity_loss_and_unknown_refuse_cleanup() {
    for limit in [33279, u64::MAX] {
        let f = fixture().await;
        let ceiling = configure(&f);
        let id = identity(&f.task);
        let a = RuntimeHostTaskStartAuthority::new(id.task_id, id.attempt_id).unwrap();
        let before = state(&f.registry);
        let CandleCpuDeclaredAttemptOutcome::Drained { receipt, .. } = f
            .gateway
            .execute_declared_envelope_cpu_warm_attempt(
                f.registry.clone(),
                f.input(),
                &f.successor,
                handle(&a, &Arc::new(AtomicBool::new(false))),
            )
            .await
            .unwrap()
        else {
            panic!("actual drain required")
        };
        ceiling.0.store(limit, Ordering::Release);
        assert!(receipt.release_retained().await.is_err());
        assert_eq!(before, state(&f.registry));
        assert!(f.registry.acquire_execution_custody(&f.task).is_err());
        assert!(f.registry.acquire_execution_custody(&f.successor).is_err());
        f.gateway.stop().await.unwrap();
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn declared_native_no_start_stale_private_owner_refuses_settlement_without_ack() {
    declared_boundary("declared_prestart", "stale_owner").await;
}
