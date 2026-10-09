//! Deterministic interface fixture, not an inference backend or a workflow
//! adapter. Existing serialized request data exercises the real dispatcher and
//! lifecycle port; loaded CPU owner identity and worker drain are synthetic.
use async_trait::async_trait;
use pantograph_runtime_host_contracts::*;
use pantograph_scheduler::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify, OwnedMutexGuard};

#[derive(Default)]
struct Host {
    started: Notify,
    allow_drain: Notify,
    drained: AtomicBool,
    calls: AtomicUsize,
}

#[async_trait]
impl RuntimeHostExecutionPort for Host {
    async fn execute_runtime_host_request(
        &self,
        request: RuntimeHostExecutionRequest,
        cancellation: RuntimeHostExecutionCancellationHandle,
    ) -> Result<RuntimeHostExecutionResponse, RuntimeHostExecutionPortError> {
        assert_eq!(
            cancellation.snapshot().state,
            RuntimeHostExecutionCancellationState::Running
        );
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.drained.store(false, Ordering::Release);
        self.started.notify_one();
        self.allow_drain.notified().await;
        self.drained.store(true, Ordering::Release);
        Ok(RuntimeHostExecutionResponse {
            contract_version: RUNTIME_HOST_EXECUTION_CONTRACT_VERSION,
            execution_request_id: request.execution_request_id,
            workflow_id: request.handoff.workflow_id,
            workflow_run_id: request.handoff.workflow_run_id,
            node_id: request.handoff.node_id,
            task_id: request.handoff.task_id,
            state: RuntimeHostExecutionState::Completed,
            outputs: Vec::new(),
            diagnostics: Vec::new(),
            terminal_metadata: None,
        })
    }
}

struct Cleanup {
    owner: SchedulerSerialAdmission,
    started: Notify,
    allow_ack: Notify,
    fail: bool,
    foreign_lease: bool,
}

#[async_trait]
impl ReservationLifecyclePort for Cleanup {
    async fn apply_reservation_lifecycle(
        &self,
        event: ReservationLifecycleEvent,
    ) -> Result<ReservationLifecycleApplication, ReservationLifecyclePortError> {
        event.validate().unwrap();
        assert_eq!(
            event.outcome,
            ReservationLifecycleOutcome::RuntimeHostCompleted
        );
        assert!(matches!(
            self.owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Busy)
        ));
        self.started.notify_one();
        self.allow_ack.notified().await;
        if self.fail {
            return Err(ReservationLifecyclePortError::Failed {
                message: "synthetic release failure".into(),
            });
        }
        Ok(ReservationLifecycleApplication {
            contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
            lifecycle_event_id: event.lifecycle_event_id,
            reservation_lease_id: if self.foreign_lease {
                SchedulerReservationLeaseId::parse("foreign.lease").unwrap()
            } else {
                event.reservation_lease_id
            },
            state: ReservationLifecycleApplicationState::Applied,
            diagnostics: Vec::new(),
        })
    }
}

struct HeldOwner(OwnedMutexGuard<SchedulerSerialOwnerSnapshot>);
impl SchedulerSerialRuntimeOwnerLease for HeldOwner {
    fn snapshot(&self) -> SchedulerSerialOwnerSnapshot {
        *self.0
    }
}

fn snapshot() -> SchedulerSerialOwnerSnapshot {
    SchedulerSerialOwnerSnapshot {
        loaded_instance: [1; 16],
        loaded_profile: [2; 32],
        effective_settings: [3; 32],
        generation: 2,
        cpu_threads: 1,
    }
}

fn request() -> RuntimeHostExecutionRequest {
    serde_json::from_str(include_str!(
        "fixtures/runtime_host_execution_request_dispatch_selected.json"
    ))
    .unwrap()
}

fn identity(request: &RuntimeHostExecutionRequest) -> SchedulerSerialAttemptIdentity<'_> {
    let decision = request.handoff.dispatch_decision.as_ref().unwrap();
    SchedulerSerialAttemptIdentity {
        workflow_id: request.handoff.workflow_id.as_str(),
        workflow_run_id: request.handoff.workflow_run_id.as_str(),
        node_id: request.handoff.node_id.as_str(),
        task_id: request.handoff.task_id.as_str(),
        attempt_id: "fixture.attempt.1",
        execution_request_id: &request.execution_request_id,
        // Candidate/attempt come from owned workflow custody; the host handoff
        // DTO includes the selected lease but has no candidate or attempt field.
        candidate_id: "fixture.candidate.1",
        reservation_lease_id: decision.reservation_lease_id.as_str(),
    }
}

fn cleanup(owner: &SchedulerSerialAdmission, fail: bool, foreign_lease: bool) -> Cleanup {
    Cleanup {
        owner: owner.clone(),
        started: Notify::new(),
        allow_ack: Notify::new(),
        fail,
        foreign_lease,
    }
}

async fn dispatch_fixture(
    owner: &SchedulerSerialAdmission,
    loaded: &Arc<Mutex<SchedulerSerialOwnerSnapshot>>,
    host: Arc<Host>,
    lifecycle: &Cleanup,
) -> Result<(), String> {
    let request = request();
    let id = identity(&request);
    let bound = owner
        .try_prepare()
        .unwrap()
        .begin_dispatch()
        .bind_attempt(id, snapshot())
        .map_err(|e| format!("bind: {e:?}"))?;
    let lease = HeldOwner(loaded.clone().lock_owned().await);
    let executing = bound
        .acquire_owner(&lease)
        .map_err(|e| format!("owner: {e:?}"))?;
    let dispatcher = SchedulerRuntimeHostDispatcher::new(host.clone());
    let response = dispatcher
        .dispatch(
            request.execution_request_id.clone(),
            request.handoff.clone(),
            request.materialized_inputs.clone(),
        )
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    assert!(host.drained.load(Ordering::Acquire));
    let response_id = SchedulerSerialAttemptIdentity {
        workflow_id: response.workflow_id.as_str(),
        workflow_run_id: response.workflow_run_id.as_str(),
        node_id: response.node_id.as_str(),
        task_id: response.task_id.as_str(),
        execution_request_id: &response.execution_request_id,
        ..id
    };
    let state = match response.state {
        RuntimeHostExecutionState::Completed => SchedulerSerialDrainState::Completed,
        RuntimeHostExecutionState::Failed => SchedulerSerialDrainState::Failed,
        RuntimeHostExecutionState::Rejected => SchedulerSerialDrainState::Rejected,
        _ => SchedulerSerialDrainState::Accepted,
    };
    let drained = executing
        .record_drained_response(response_id, state)
        .map_err(|e| format!("drain: {e:?}"))?;
    drop(lease); // Backend owner borrow ends only after actual drain validation.
    let event = ReservationLifecycleEvent {
        contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
        lifecycle_event_id: "fixture.cleanup.1".into(),
        reservation_lease_id: SchedulerReservationLeaseId::parse(id.reservation_lease_id).unwrap(),
        workflow_id: request.handoff.workflow_id.clone(),
        workflow_run_id: request.handoff.workflow_run_id.clone(),
        node_id: request.handoff.node_id.clone(),
        task_id: request.handoff.task_id.clone(),
        candidate_id: Some(SchedulerDispatchCandidateId::parse(id.candidate_id).unwrap()),
        outcome: ReservationLifecycleOutcome::RuntimeHostCompleted,
        diagnostics: Vec::new(),
    };
    let pending = drained
        .expect_cleanup(SchedulerSerialCleanupEvent {
            identity: SchedulerSerialAttemptIdentity {
                workflow_id: event.workflow_id.as_str(),
                workflow_run_id: event.workflow_run_id.as_str(),
                node_id: event.node_id.as_str(),
                task_id: event.task_id.as_str(),
                candidate_id: event.candidate_id.as_ref().unwrap().as_str(),
                reservation_lease_id: event.reservation_lease_id.as_str(),
                ..id
            },
            lifecycle_event_id: &event.lifecycle_event_id,
            releases_reservation: event.outcome
                == ReservationLifecycleOutcome::RuntimeHostCompleted,
        })
        .map_err(|e| format!("event: {e:?}"))?;
    let application = lifecycle
        .apply_reservation_lifecycle(event)
        .await
        .map_err(|e| e.to_string())?;
    application.validate().map_err(|e| e.to_string())?;
    let state = match application.state {
        ReservationLifecycleApplicationState::Applied => SchedulerSerialCleanupState::Applied,
        ReservationLifecycleApplicationState::AlreadyApplied => {
            SchedulerSerialCleanupState::AlreadyApplied
        }
        _ => SchedulerSerialCleanupState::Failed,
    };
    pending
        .acknowledge_cleanup(
            &application.lifecycle_event_id,
            application.reservation_lease_id.as_str(),
            state,
        )
        .map_err(|e| format!("cleanup: {e:?}"))
}

#[tokio::test]
async fn exclusion_survives_terminal_response_until_actual_cleanup_acknowledgement() {
    let owner = SchedulerSerialAdmission::new();
    let loaded = Arc::new(Mutex::new(snapshot()));
    let host = Arc::new(Host::default());
    let lifecycle = cleanup(&owner, false, false);
    let dispatch = dispatch_fixture(&owner, &loaded, host.clone(), &lifecycle);
    fn require_send<T: Send>(_: &T) {}
    require_send(&dispatch);
    let monitor = async {
        host.started.notified().await;
        assert!(!host.drained.load(Ordering::Acquire));
        assert!(loaded.try_lock().is_err());
        assert!(matches!(
            owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Busy)
        ));
        host.allow_drain.notify_one();
        lifecycle.started.notified().await;
        assert!(host.drained.load(Ordering::Acquire));
        assert!(loaded.try_lock().is_ok());
        assert!(matches!(
            owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Busy)
        ));
        lifecycle.allow_ack.notify_one();
    };
    let (result, ()) = tokio::join!(dispatch, monitor);
    result.unwrap();
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
    assert!(owner.try_prepare().is_ok());
    assert!(!owner.is_poisoned());
}

#[tokio::test]
async fn actual_owner_mismatch_refuses_before_host_execution() {
    let owner = SchedulerSerialAdmission::new();
    let loaded = Arc::new(Mutex::new(SchedulerSerialOwnerSnapshot {
        loaded_instance: [9; 16],
        ..snapshot()
    }));
    let host = Arc::new(Host::default());
    let lifecycle = cleanup(&owner, false, false);
    assert!(dispatch_fixture(&owner, &loaded, host.clone(), &lifecycle)
        .await
        .is_err());
    assert_eq!(host.calls.load(Ordering::Relaxed), 0);
    assert!(owner.is_poisoned());
}

#[tokio::test]
async fn failed_or_foreign_lifecycle_application_cannot_reopen_serial_dispatch() {
    for (fail, foreign_lease) in [(true, false), (false, true)] {
        let owner = SchedulerSerialAdmission::new();
        let loaded = Arc::new(Mutex::new(snapshot()));
        let host = Arc::new(Host::default());
        let lifecycle = cleanup(&owner, fail, foreign_lease);
        host.allow_drain.notify_one();
        lifecycle.allow_ack.notify_one();
        assert!(dispatch_fixture(&owner, &loaded, host, &lifecycle)
            .await
            .is_err());
        assert!(matches!(
            owner.try_prepare(),
            Err(SchedulerSerialAdmissionRefusal::Poisoned)
        ));
    }
}

#[tokio::test]
async fn caller_drop_during_host_work_poisoning_does_not_claim_worker_drain() {
    let owner = SchedulerSerialAdmission::new();
    let loaded = Arc::new(Mutex::new(snapshot()));
    let host = Arc::new(Host::default());
    let lifecycle = cleanup(&owner, false, false);
    let mut dispatch = Box::pin(dispatch_fixture(&owner, &loaded, host.clone(), &lifecycle));
    tokio::select! {
        result = &mut dispatch => panic!("dispatch finished without drain permission: {result:?}"),
        () = host.started.notified() => {}
    }
    drop(dispatch);
    assert!(!host.drained.load(Ordering::Acquire));
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
    assert!(matches!(
        owner.try_prepare(),
        Err(SchedulerSerialAdmissionRefusal::Poisoned)
    ));
}

#[tokio::test]
async fn continuation_reacquires_owner_and_refuses_changed_instance_before_second_call() {
    let owner = SchedulerSerialAdmission::new();
    let loaded = Arc::new(Mutex::new(snapshot()));
    let host = Arc::new(Host::default());
    let lifecycle = cleanup(&owner, false, false);
    host.allow_drain.notify_one();
    lifecycle.allow_ack.notify_one();
    dispatch_fixture(&owner, &loaded, host.clone(), &lifecycle)
        .await
        .unwrap();
    assert!(!owner.is_poisoned());
    loaded.lock().await.loaded_instance = [9; 16];
    assert!(dispatch_fixture(&owner, &loaded, host.clone(), &lifecycle)
        .await
        .is_err());
    assert_eq!(host.calls.load(Ordering::Relaxed), 1);
    assert!(owner.is_poisoned());
}
