use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_runtime_host_contracts::{
    ReservationLifecycleDiagnostic, ReservationLifecycleDiagnosticCode,
    ReservationLifecycleDiagnosticSeverity, ReservationLifecycleOutcome,
};
use pantograph_runtime_registry::{
    RuntimeObservation, RuntimeRegistry, RuntimeRegistryStatus, RuntimeReservationRequest,
    RuntimeRetentionHint,
};
use pantograph_scheduler::{
    SchedulerNodeId, SchedulerReservationLeaseId, SchedulerTaskId, SchedulerWorkflowId,
    SchedulerWorkflowRunId,
};

use super::*;
use crate::runtime_registry::HostRuntimeProducer;
use crate::HostRuntimeModeSnapshot;

#[tokio::test]
async fn terminal_completion_releases_registry_reservation_and_reconciles_runtime() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.observe_runtimes(vec![runtime_observation("pytorch")]);
    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-image".to_string(),
            reservation_owner_id: Some("run-1:infer".to_string()),
            usage_profile: None,
            model_id: Some("pumas://image/model".to_string()),
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    let controller = Arc::new(FakeLifecycleController::active_runtime("pytorch"));
    let port = EmbeddedReservationLifecyclePort::new(registry.clone(), controller.clone());

    let application = port
        .apply_reservation_lifecycle(event(
            lease.reservation_id,
            ReservationLifecycleOutcome::RuntimeHostCompleted,
            Vec::new(),
        ))
        .await
        .expect("apply lifecycle");

    assert_eq!(
        application.state,
        ReservationLifecycleApplicationState::Applied
    );
    assert!(registry.snapshot().reservations.is_empty());
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        registry.snapshot().runtimes[0].status,
        RuntimeRegistryStatus::Stopped
    );
}

#[tokio::test]
async fn duplicate_terminal_release_is_reported_as_already_applied() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.observe_runtimes(vec![runtime_observation("pytorch")]);
    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-image".to_string(),
            reservation_owner_id: Some("run-1:infer".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    registry
        .release_reservation(lease.reservation_id)
        .expect("pre-release");
    let controller = Arc::new(FakeLifecycleController::inactive());
    let port = EmbeddedReservationLifecyclePort::new(registry, controller);

    let application = port
        .apply_reservation_lifecycle(event(
            lease.reservation_id,
            ReservationLifecycleOutcome::RuntimeHostFailed,
            vec![diagnostic(
                ReservationLifecycleDiagnosticSeverity::Error,
                ReservationLifecycleDiagnosticCode::RuntimeHostFailed,
                "runtime failed",
            )],
        ))
        .await
        .expect("apply lifecycle");

    assert_eq!(
        application.state,
        ReservationLifecycleApplicationState::AlreadyApplied
    );
    assert_eq!(
        application.diagnostics[0].code,
        ReservationLifecycleDiagnosticCode::LeaseNotFound
    );
}

#[tokio::test]
async fn dispatch_started_does_not_release_registry_reservation() {
    let registry = Arc::new(RuntimeRegistry::new());
    registry.observe_runtimes(vec![runtime_observation("pytorch")]);
    let lease = registry
        .acquire_reservation(RuntimeReservationRequest {
            runtime_id: "pytorch".to_string(),
            workflow_id: "wf-image".to_string(),
            reservation_owner_id: Some("run-1:infer".to_string()),
            usage_profile: None,
            model_id: None,
            pin_runtime: false,
            requirements: None,
            retention_hint: RuntimeRetentionHint::Ephemeral,
        })
        .expect("reservation");
    let controller = Arc::new(FakeLifecycleController::active_runtime("pytorch"));
    let port = EmbeddedReservationLifecyclePort::new(registry.clone(), controller.clone());

    let application = port
        .apply_reservation_lifecycle(event(
            lease.reservation_id,
            ReservationLifecycleOutcome::DispatchStarted,
            Vec::new(),
        ))
        .await
        .expect("apply lifecycle");

    assert_eq!(
        application.state,
        ReservationLifecycleApplicationState::Applied
    );
    assert_eq!(registry.snapshot().reservations.len(), 1);
    assert_eq!(controller.stop_count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn non_registry_lease_id_returns_typed_failure_application() {
    let registry = Arc::new(RuntimeRegistry::new());
    let controller = Arc::new(FakeLifecycleController::inactive());
    let port = EmbeddedReservationLifecyclePort::new(registry, controller);
    let mut event = event(
        7,
        ReservationLifecycleOutcome::CandidateUnselected,
        Vec::new(),
    );
    event.reservation_lease_id =
        SchedulerReservationLeaseId::parse("external.7").expect("valid lease id");

    let application = port
        .apply_reservation_lifecycle(event)
        .await
        .expect("apply lifecycle");

    assert_eq!(
        application.state,
        ReservationLifecycleApplicationState::Failed
    );
    assert_eq!(
        application.diagnostics[0].code,
        ReservationLifecycleDiagnosticCode::LeaseOwnerMismatch
    );
}

#[derive(Debug)]
struct FakeLifecycleController {
    runtime_id: String,
    active: AtomicBool,
    stop_count: AtomicUsize,
}

impl FakeLifecycleController {
    fn active_runtime(runtime_id: &str) -> Self {
        Self {
            runtime_id: runtime_id.to_string(),
            active: AtomicBool::new(true),
            stop_count: AtomicUsize::new(0),
        }
    }

    fn inactive() -> Self {
        Self {
            runtime_id: String::new(),
            active: AtomicBool::new(false),
            stop_count: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl HostRuntimeRegistryController for FakeLifecycleController {
    async fn mode_info_snapshot(&self) -> HostRuntimeModeSnapshot {
        if !self.active.load(Ordering::SeqCst) {
            return HostRuntimeModeSnapshot::default();
        }
        HostRuntimeModeSnapshot {
            backend_name: Some(self.runtime_id.clone()),
            backend_key: Some(self.runtime_id.clone()),
            active_model_target: None,
            embedding_model_target: None,
            active_runtime: Some(inference::RuntimeLifecycleSnapshot {
                runtime_id: Some(self.runtime_id.clone()),
                runtime_instance_id: Some(format!("{}-instance", self.runtime_id)),
                warmup_timing_attempt_id: None,
                warmup_started_at_ms: Some(1),
                warmup_completed_at_ms: Some(2),
                warmup_duration_ms: Some(1),
                timing_diagnostics: Vec::new(),
                runtime_reused: Some(true),
                lifecycle_decision_reason: Some("test_runtime_ready".to_string()),
                active: true,
                last_error: None,
            }),
            embedding_runtime: None,
        }
    }

    async fn stop_runtime_producer(
        &self,
        producer: HostRuntimeProducer,
    ) -> Result<(), inference::GatewayError> {
        assert_eq!(producer, HostRuntimeProducer::Active);
        self.active.store(false, Ordering::SeqCst);
        self.stop_count.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

fn runtime_observation(runtime_id: &str) -> RuntimeObservation {
    RuntimeObservation {
        runtime_id: runtime_id.to_string(),
        display_name: runtime_id.to_string(),
        backend_keys: vec![runtime_id.to_string()],
        model_id: None,
        status: RuntimeRegistryStatus::Ready,
        runtime_instance_id: Some(format!("{runtime_id}-instance")),
        last_error: None,
    }
}

fn event(
    reservation_id: u64,
    outcome: ReservationLifecycleOutcome,
    diagnostics: Vec<ReservationLifecycleDiagnostic>,
) -> ReservationLifecycleEvent {
    ReservationLifecycleEvent {
        contract_version: RESERVATION_LIFECYCLE_CONTRACT_VERSION,
        lifecycle_event_id: format!("event.{reservation_id}.{}", outcome_key(&outcome)),
        reservation_lease_id: SchedulerReservationLeaseId::parse(format!(
            "runtime-registry.{reservation_id}"
        ))
        .expect("valid reservation lease id"),
        workflow_id: SchedulerWorkflowId::parse("wf-image").expect("valid workflow id"),
        workflow_run_id: SchedulerWorkflowRunId::parse("run-1").expect("valid run id"),
        node_id: SchedulerNodeId::parse("infer").expect("valid node id"),
        task_id: SchedulerTaskId::parse("infer").expect("valid task id"),
        outcome,
        candidate_id: None,
        diagnostics,
    }
}

fn outcome_key(outcome: &ReservationLifecycleOutcome) -> &'static str {
    match outcome {
        ReservationLifecycleOutcome::CandidateUnselected => "candidate_unselected",
        ReservationLifecycleOutcome::DispatchStarted => "dispatch_started",
        ReservationLifecycleOutcome::RuntimeHostCompleted => "runtime_host_completed",
        ReservationLifecycleOutcome::RuntimeHostFailed => "runtime_host_failed",
        _ => "other",
    }
}
