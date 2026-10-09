//! Actual tiny CPU producer plus real service TaskWorker; declared graph/RAM are controlled.
use super::*;
use pantograph_workflow_service::workflow::*;
use std::sync::{Mutex, Weak};
use std::time::Instant;

struct Signal(RuntimeHostTaskStartAuthority);
impl RuntimeHostExecutionCancellationSignal for Signal {
    fn snapshot(&self) -> RuntimeHostExecutionCancellationSnapshot {
        // Intentionally running DTO: only the shared authority decides start.
        RuntimeHostExecutionCancellationSnapshot {
            cancellation_context_id: "native.start.test".into(),
            state: RuntimeHostExecutionCancellationState::Running,
            reason: None,
        }
    }
    fn task_start_authority(&self) -> Option<RuntimeHostTaskStartAuthority> {
        Some(self.0.clone())
    }
}
fn authority(request: &RuntimeHostBatchExecutionRequest) -> RuntimeHostTaskStartAuthority {
    RuntimeHostTaskStartAuthority::new(
        request.members[0].handoff.task_id.as_str(),
        "native.attempt",
    )
    .unwrap()
}
fn handle(a: &RuntimeHostTaskStartAuthority) -> RuntimeHostExecutionCancellationHandle {
    RuntimeHostExecutionCancellationHandle::with_signal(Arc::new(Signal(a.clone())))
}
#[tokio::test]
async fn revoked_and_duplicate_native_start_preserve_exact_custody() {
    let f = fixture().await;
    let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "start.exact");
    let request = batch(&f, &lease);
    let a = authority(&request);
    a.revoke();
    let before = f
        .port
        .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
        .unwrap()
        .snapshot;
    let admission = SchedulerSerialAdmission::new();
    assert!(f
        .port
        .execute_serial_singleton_with_cleanup(
            request.clone(),
            handle(&a),
            bind(&admission, &request, None)
        )
        .await
        .is_err());
    assert_eq!(
        a.disposition(),
        RuntimeHostTaskStartDisposition::NoStartAuthorized
    );
    assert_eq!(
        f.port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot,
        before
    );
    // No task work authorized; charge remains. Prestart custody has dropped, not released.
    drop(f.port.registry.acquire_execution_custody(&lease).unwrap());
    let a = authority(&request);
    let admission = SchedulerSerialAdmission::new();
    let execution = f
        .port
        .execute_serial_singleton_with_cleanup(
            request.clone(),
            handle(&a),
            bind(&admission, &request, None),
        )
        .await
        .unwrap();
    assert_eq!(
        a.disposition(),
        RuntimeHostTaskStartDisposition::StartedOrUncertain
    );
    assert!(f.port.registry.acquire_execution_custody(&lease).is_err());
    let competitor = SchedulerSerialAdmission::new();
    assert!(f
        .port
        .execute_serial_singleton_with_cleanup(
            request.clone(),
            handle(&a),
            bind(&competitor, &request, None)
        )
        .await
        .is_err());
    assert_eq!(
        a.revoke(),
        RuntimeHostTaskStartDisposition::StartedOrUncertain
    );
    assert!(f
        .port
        .registry
        .release_reservation(lease.reservation_id)
        .is_err());
    execution
        .cleanup
        .unwrap()
        .apply(cleanup_event(&request))
        .await
        .unwrap();
    assert!(f
        .port
        .registry
        .reservation_lease(lease.reservation_id)
        .is_none());
    // Rebinding a later lease to the old exact attempt cannot authorize a second native start.
    let next = task_lease(&f, f.request.handoff.workflow_id.as_str(), "start.retry");
    let next_request = batch(&f, &next);
    let retry = SchedulerSerialAdmission::new();
    let before = f
        .port
        .resident_serial_cpu_owner(&next_request.members[0].handoff.task_intent)
        .unwrap()
        .snapshot;
    assert!(f
        .port
        .execute_serial_singleton_with_cleanup(
            next_request.clone(),
            handle(&a),
            bind(&retry, &next_request, None)
        )
        .await
        .is_err());
    assert_eq!(
        f.port
            .resident_serial_cpu_owner(&next_request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot,
        before
    );
    drop(f.port.registry.acquire_execution_custody(&next).unwrap());
}
#[tokio::test]
async fn cancellation_and_collector_loss_in_resolver_revoke_same_start_cell() {
    for abort in [false, true] {
        let f = fixture().await;
        let lease = task_lease(&f, f.request.handoff.workflow_id.as_str(), "start.paused");
        let request = batch(&f, &lease);
        let a = authority(&request);
        let entered = Arc::new(tokio::sync::Notify::new());
        let proceed = Arc::new(tokio::sync::Notify::new());
        let port = EmbeddedRetainedCpuSerialPort {
            package: Arc::new(PausedPackage {
                package: f.package.clone(),
                entered: entered.clone(),
                proceed: proceed.clone(),
            }),
            ..EmbeddedRetainedCpuSerialPort {
                gateway: f.port.gateway.clone(),
                registry: f.port.registry.clone(),
                package: f.port.package.clone(),
                target: f.port.target.clone(),
                release_observer: None,
                session_reservations: None,
            }
        };
        let before = f
            .port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        let h = handle(&a);
        let req = request.clone();
        let task = tokio::spawn(async move {
            let admission = SchedulerSerialAdmission::new();
            port.execute_serial_singleton_with_cleanup(req.clone(), h, bind(&admission, &req, None))
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), entered.notified())
            .await
            .unwrap();
        if abort {
            task.abort();
            assert!(matches!(task.await, Err(e) if e.is_cancelled()));
        } else {
            a.revoke();
            proceed.notify_one();
            assert!(task.await.unwrap().is_err());
        }
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::NoStartAuthorized
        );
        assert_eq!(
            f.port
                .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
                .unwrap()
                .snapshot,
            before
        );
        assert_eq!(
            f.port.registry.reservation_lease(lease.reservation_id),
            Some(lease)
        );
    }
}

#[derive(Default)]
struct EpisodeEvidence {
    request: Option<RuntimeHostBatchExecutionRequest>,
    authority: Option<RuntimeHostTaskStartAuthority>,
    accepted: Option<serde_json::Value>,
    start: Option<Instant>,
    checkpoints: Vec<serde_json::Value>,
}
struct EpisodePort {
    native: Arc<EmbeddedRetainedCpuSerialPort>,
    owner: Arc<Mutex<Weak<WorkflowControlledNativeWorkerEpisode>>>,
    evidence: Arc<Mutex<EpisodeEvidence>>,
}
fn rss_kib() -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmRSS:")?
                .split_whitespace()
                .next()?
                .parse()
                .ok()
        })
}
fn checkpoint(e: &mut EpisodeEvidence, phase: &str) {
    e.checkpoints.push(serde_json::json!({"phase": phase, "elapsed_ns": e.start.map(|s|s.elapsed().as_nanos()), "process_rss_kib": rss_kib()}));
}
#[async_trait]
impl RuntimeHostBatchExecutionPort for EpisodePort {
    async fn execute_runtime_host_batch_request(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        panic!("real Worker must use existing serial admission")
    }
}
#[async_trait]
impl ReservationLifecyclePort for EpisodePort {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native.apply_reservation_lifecycle(event).await
    }
}
#[async_trait]
impl SerialRuntimeHostBatchExecutionPort for EpisodePort {
    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native.apply_serial_cleanup(event, expected).await
    }

    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        self.native.resident_serial_cpu_owner(intent)
    }
    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        bound: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        let a = cancellation
            .task_start_authority()
            .expect("actual TaskLifecycle owner issues shared cell");
        assert_eq!(a.disposition(), RuntimeHostTaskStartDisposition::Prepared);
        {
            let mut e = self.evidence.lock().unwrap();
            e.request = Some(request.clone());
            e.authority = Some(a.clone());
            checkpoint(&mut e, "port.before_resolution");
        }
        let mut execution = self
            .native
            .execute_serial_singleton_with_cleanup(request, cancellation, bound)
            .await?;
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::StartedOrUncertain
        );
        {
            let mut e = self.evidence.lock().unwrap();
            checkpoint(&mut e, "native.drained_and_collected");
        }
        let vector = match &execution.response.members[0]
            .outputs
            .iter()
            .find(|o| o.port_id == "embedding")
            .unwrap()
            .value
        {
            RuntimeHostExecutionOutputValue::Json(v) => v.clone(),
            _ => panic!("native vector"),
        };
        execution.cleanup = Some(Box::new(EpisodeCleanup {
            native: execution.cleanup.take().unwrap(),
            owner: self.owner.clone(),
            evidence: self.evidence.clone(),
            vector,
        }));
        Ok(execution)
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
        panic!("existing linear cleanup route required")
    }
}
struct EpisodeCleanup {
    native: Box<dyn SerialRuntimeHostCleanupReceipt>,
    owner: Arc<Mutex<Weak<WorkflowControlledNativeWorkerEpisode>>>,
    evidence: Arc<Mutex<EpisodeEvidence>>,
    vector: serde_json::Value,
}
#[async_trait]
impl SerialRuntimeHostCleanupReceipt for EpisodeCleanup {
    fn previous_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.native.previous_owner()
    }
    fn current_owner(&self) -> SerialRuntimeHostCpuOwnerEvidence {
        self.native.current_owner()
    }
    async fn apply(
        self: Box<Self>,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        let accepted = self
            .owner
            .lock()
            .unwrap()
            .upgrade()
            .unwrap()
            .accepted_vector()
            .expect("actual Worker inserts Completed vector in SAME task store BEFORE cleanup");
        assert_eq!(accepted, self.vector);
        {
            let mut e = self.evidence.lock().unwrap();
            e.accepted = Some(accepted);
            checkpoint(&mut e, "task_store.accepted_before_release");
        }
        let application = self.native.apply(event).await?;
        {
            let mut e = self.evidence.lock().unwrap();
            checkpoint(&mut e, "task_lease.release_ack");
        }
        Ok(application)
    }
}
fn candidate(request: &RuntimeHostBatchExecutionRequest) -> WorkflowRuntimeDispatchCandidateFact {
    let m = &request.members[0];
    let d = m.handoff.dispatch_decision.as_ref().unwrap();
    WorkflowRuntimeDispatchCandidateFact {
        candidate_id: "native.candidate".parse().unwrap(),
        selected_runtime_id: "candle".parse().unwrap(),
        selected_runtime_variant_id: d.selected_runtime_variant_id.clone(),
        selected_backend_key: "candle".into(),
        runtime_family: "candle".into(),
        resolved_load_target: "controlled.local.cpu".into(),
        runtime_residency_key: "controlled.local.cpu".into(),
        loaded_runtime_memory_estimate_bytes: 32768,
        runtime_load_state: WorkflowRuntimeDispatchLoadState::Loaded,
        runtime_instance_id: Some("controlled.cpu.instance".into()),
        selected_device_ids: vec!["cpu".parse().unwrap()],
        selected_model_ref: m.handoff.task_intent.model_ref.clone(),
        runtime_trait_settings: vec![],
        environment_ref: m
            .handoff
            .readiness_proof
            .preflight_result
            .environment_ref
            .clone()
            .unwrap(),
        reservations: d
            .reservations
            .iter()
            .cloned()
            .map(|mut r| {
                r.resource_kind = SchedulerResourceKind::SystemRam;
                r.reserved_bytes = 256;
                r
            })
            .collect(),
        resource_fit_assessment: SchedulerResourceFitAssessment {
            workflow_run_id: m.handoff.workflow_run_id.clone(),
            task_id: m.handoff.task_id.clone(),
            state: SchedulerResourceFitState::Fits,
            diagnostics: vec![],
        },
        batching_group_id: None,
    }
}
#[tokio::test]
async fn real_task_worker_cpu_episode_covers_start_acceptance_release_and_reuse() {
    let f = fixture().await;
    for i in 0..3 {
        let lease = task_lease(
            &f,
            f.request.handoff.workflow_id.as_str(),
            &format!("worker.episode.{i}"),
        );
        let request = batch(&f, &lease);
        let before = f
            .port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        let owner = Arc::new(Mutex::new(Weak::new()));
        let evidence = Arc::new(Mutex::new(EpisodeEvidence::default()));
        let port = Arc::new(EpisodePort {
            native: f.port.clone(),
            owner: owner.clone(),
            evidence: evidence.clone(),
        });
        let driver = Arc::new(
            WorkflowControlledNativeWorkerEpisode::new(&request, port, candidate(&request))
                .unwrap(),
        );
        *owner.lock().unwrap() = Arc::downgrade(&driver);
        {
            let mut e = evidence.lock().unwrap();
            e.start = Some(driver.timing_origin());
            checkpoint(&mut e, "queue.admitted_ready_before_worker_enqueue");
        }
        let response = driver.run().await.unwrap();
        let after = f
            .port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot;
        assert_eq!(before.loaded_instance, after.loaded_instance);
        assert_eq!(before.generation + 2, after.generation);
        assert!(f
            .port
            .registry
            .reservation_lease(lease.reservation_id)
            .is_none());
        assert_eq!(
            f.port
                .registry
                .reservation_lease(f.successor.reservation_id),
            Some(f.successor.clone())
        );
        let mut e = evidence.lock().unwrap();
        checkpoint(&mut e, "worker.completed_response_and_shutdown");
        assert!(e.accepted.is_some());
        assert!(response.outputs.iter().any(|o| o.port_id == "embedding"));
        let id = e.request.as_ref().unwrap().members[0]
            .handoff
            .task_id
            .to_string();
        let a = e.authority.as_ref().unwrap();
        assert_eq!(
            a.disposition(),
            RuntimeHostTaskStartDisposition::StartedOrUncertain
        );
        assert!(!a.try_start(&id, "native.attempt"));
        eprintln!(
            "NATIVE_WORKER_EPISODE {}",
            serde_json::json!({"sample":i,"checkpoints":e.checkpoints,"task_charge_bytes":256,"retained_session_charge_bytes":256,"declared_resident_bytes":32768,"cpu_threads":after.cpu_threads,"loaded_generation_before":before.generation,"loaded_generation_after":after.generation,"scope":"debug local tiny model, process point RSS not peak, controlled readiness/candidate/RAM; no full-envelope or default timing authority"})
        );
    }
}

struct HoldDrained {
    native: Arc<EmbeddedRetainedCpuSerialPort>,
    entered: Arc<tokio::sync::Notify>,
    proceed: Arc<tokio::sync::Notify>,
}
#[async_trait]
impl RuntimeHostBatchExecutionPort for HoldDrained {
    async fn execute_runtime_host_batch_request(
        &self,
        _: RuntimeHostBatchExecutionRequest,
        _: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostBatchExecutionResponse, RuntimeHostExecutionPortError> {
        panic!("serial only")
    }
}
#[async_trait]
impl ReservationLifecyclePort for HoldDrained {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native.apply_reservation_lifecycle(event).await
    }
}
#[async_trait]
impl SerialRuntimeHostBatchExecutionPort for HoldDrained {
    fn resident_serial_cpu_owner(
        &self,
        intent: &SchedulableTaskIntent,
    ) -> Option<SerialRuntimeHostCpuOwnerEvidence> {
        self.native.resident_serial_cpu_owner(intent)
    }
    async fn apply_serial_cleanup(
        &self,
        event: ReservationLifecycleEvent,
        expected: Option<SchedulerSerialOwnerSnapshot>,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        self.native.apply_serial_cleanup(event, expected).await
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
        panic!("linear receipt only")
    }
    async fn execute_serial_singleton_with_cleanup(
        &self,
        request: RuntimeHostBatchExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
        bound: SchedulerSerialBoundDispatch,
    ) -> Result<SerialRuntimeHostDrainedExecution, RuntimeHostExecutionPortError> {
        let execution = self
            .native
            .execute_serial_singleton_with_cleanup(request, cancellation, bound)
            .await?;
        self.entered.notify_one();
        self.proceed.notified().await;
        Ok(execution)
    }
}
#[tokio::test]
async fn actual_start_then_collector_abort_cannot_claim_no_effect_or_reopen_charge() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "start.abandoned",
    );
    let request = batch(&f, &lease);
    let a = authority(&request);
    let entered = Arc::new(tokio::sync::Notify::new());
    let port = HoldDrained {
        native: f.port.clone(),
        entered: entered.clone(),
        proceed: Arc::new(tokio::sync::Notify::new()),
    };
    let h = handle(&a);
    let req = request.clone();
    let admission = SchedulerSerialAdmission::new();
    let admitted = admission.clone();
    let task = tokio::spawn(async move {
        port.execute_serial_singleton_with_cleanup(req.clone(), h, bind(&admitted, &req, None))
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    assert_eq!(
        a.revoke(),
        RuntimeHostTaskStartDisposition::StartedOrUncertain
    );
    task.abort();
    assert!(matches!(task.await, Err(e) if e.is_cancelled()));
    assert_eq!(
        a.disposition(),
        RuntimeHostTaskStartDisposition::StartedOrUncertain
    );
    assert!(admission.is_poisoned());
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease.clone())
    );
    assert!(f
        .port
        .registry
        .release_reservation(lease.reservation_id)
        .is_err());
    assert!(f.port.registry.acquire_execution_custody(&lease).is_err());
}
#[tokio::test]
async fn real_worker_cancellation_before_native_start_revokes_actual_lifecycle_authority() {
    let f = fixture().await;
    let lease = task_lease(
        &f,
        f.request.handoff.workflow_id.as_str(),
        "worker.cancelled",
    );
    let request = batch(&f, &lease);
    let entered = Arc::new(tokio::sync::Notify::new());
    let proceed = Arc::new(tokio::sync::Notify::new());
    let native = Arc::new(EmbeddedRetainedCpuSerialPort {
        package: Arc::new(PausedPackage {
            package: f.package.clone(),
            entered: entered.clone(),
            proceed: proceed.clone(),
        }),
        gateway: f.port.gateway.clone(),
        registry: f.port.registry.clone(),
        target: f.port.target.clone(),
        release_observer: None,
        session_reservations: None,
    });
    let owner = Arc::new(Mutex::new(Weak::new()));
    let evidence = Arc::new(Mutex::new(EpisodeEvidence::default()));
    let port = Arc::new(EpisodePort {
        native,
        owner: owner.clone(),
        evidence: evidence.clone(),
    });
    let driver = Arc::new(
        WorkflowControlledNativeWorkerEpisode::new(&request, port, candidate(&request)).unwrap(),
    );
    *owner.lock().unwrap() = Arc::downgrade(&driver);
    let before = f
        .port
        .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
        .unwrap()
        .snapshot;
    let running = driver.clone();
    let task = tokio::spawn(async move { running.run().await });
    tokio::time::timeout(Duration::from_secs(5), entered.notified())
        .await
        .unwrap();
    driver.cancel().unwrap();
    proceed.notify_one();
    assert!(tokio::time::timeout(Duration::from_secs(15), task)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    let e = evidence.lock().unwrap();
    assert_eq!(
        e.authority.as_ref().unwrap().disposition(),
        RuntimeHostTaskStartDisposition::NoStartAuthorized
    );
    assert!(e.accepted.is_none());
    assert_eq!(
        f.port
            .resident_serial_cpu_owner(&request.members[0].handoff.task_intent)
            .unwrap()
            .snapshot,
        before
    );
    assert_eq!(
        f.port.registry.reservation_lease(lease.reservation_id),
        Some(lease.clone())
    );
    // No native start: normal exact lease custody is recoverable, but no
    // no-effect/release ACK has been fabricated by the Worker error path.
    drop(f.port.registry.acquire_execution_custody(&lease).unwrap());
}
