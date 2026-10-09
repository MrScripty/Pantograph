//! Independent lifecycle acceptance oracle, frozen before implementation review.
//! Exercises the existing selected gateway path with injected controlled owners.
//! Counterfactual BuiltIn metadata below tests only the trusted data-contract
//! accessor; it never promotes an actually injected capture into production evidence.

use super::*;
use pantograph_timing_contracts::{
    RuntimeServiceTimingOwnerProvenance, RuntimeServiceTimingTermination,
};

// Actual captures remain Injected. This cloned protocol fixture is intentionally
// not a measurement authority and cannot qualify the real producer/host.
fn protocol_builtin_clone(attempt: &RuntimeServiceTimingAttempt) -> RuntimeServiceTimingAttempt {
    let mut clone = attempt.clone();
    assert_eq!(
        clone.capture.as_ref().unwrap().owner_provenance,
        RuntimeServiceTimingOwnerProvenance::Injected
    );
    clone.capture.as_mut().unwrap().owner_provenance = RuntimeServiceTimingOwnerProvenance::BuiltIn;
    clone
}

#[tokio::test]
async fn lifecycle_oracle_complete_interval_includes_real_phase_gap_and_ends_at_drain() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("lifecycle-oracle-content".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let mut backend = Backend::new(clock.clone());
    backend.owner_clock_gap_ns = 5;
    let gateway = instrument(backend, recorder.clone(), clock.clone());
    execute(&gateway, request, target, decision).await.unwrap();
    let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
    let records = recorder.rows.lock().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    let lifecycle = record.lifecycle.as_ref().unwrap();
    assert_eq!(
        lifecycle.termination,
        RuntimeServiceTimingTermination::Completed
    );
    let interval = lifecycle.interval.as_ref().unwrap();
    assert_eq!(interval.started_at_ns, 0);
    assert_eq!(interval.worker_drained_at_ns, Some(46));
    assert!(interval.observed_through_ns >= 46);
    assert_eq!(record.outcome, Outcome::Completed);
    let profile = exact_profile(record);
    let phase_sum: u128 = [
        Phase::GatewayCustodyWait,
        Phase::SelectedModelLoad,
        Phase::TextExecution,
        Phase::WorkerCleanup,
    ]
    .into_iter()
    .map(|phase| u128::from(record.observed_completed_ns(&profile, phase).unwrap()))
    .sum();
    assert_eq!(phase_sum, 41);
    assert_eq!(
        interval.worker_drained_at_ns.unwrap() - interval.started_at_ns,
        46
    );
    // Controlled clock/backend must still refuse actual production qualification.
    assert_eq!(
        record.fresh_production_service_interval_ns(&profile, &snapshot, 100),
        None
    );
    let protocol = protocol_builtin_clone(record);
    assert_eq!(
        protocol.fresh_production_service_interval_ns(&profile, &snapshot, 100),
        Some(46)
    );
}

#[tokio::test]
async fn lifecycle_oracle_failure_at_load_compute_or_drain_never_becomes_full_service() {
    for failure in ["load", "compute", "drain"] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("lifecycle-oracle-content".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.fail_load = failure == "load";
        backend.fail_compute = failure == "compute";
        backend.fail_cleanup = failure == "drain";
        let gateway = instrument(backend, recorder.clone(), clock);
        assert!(execute(&gateway, request, target, decision).await.is_err());
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let records = recorder.rows.lock().unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        let lifecycle = record.lifecycle.as_ref().unwrap();
        assert_eq!(
            lifecycle.termination,
            RuntimeServiceTimingTermination::Failed
        );
        assert_eq!(record.outcome, Outcome::Failed);
        let interval = lifecycle.interval.as_ref().unwrap();
        if failure == "compute" {
            // Failed result with successful actual cleanup preserves the drain
            // fact, but the attempt remains outside the successful population.
            assert!(interval.worker_drained_at_ns.is_some());
            assert!(matches!(
                value(record, Phase::WorkerCleanup),
                Value::Observed {
                    outcome: Outcome::Completed,
                    ..
                }
            ));
        } else {
            assert_eq!(interval.worker_drained_at_ns, None);
        }
        if let RuntimeServiceTimingIdentity::Exact { profile } = &record.identity {
            assert_eq!(
                protocol_builtin_clone(record)
                    .fresh_production_service_interval_ns(profile, &snapshot, 100),
                None
            );
        }
    }
}

#[tokio::test]
async fn lifecycle_oracle_pre_custody_cancel_and_shutdown_have_no_invented_interval() {
    for shutdown in [false, true] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("lifecycle-oracle-content".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let gateway = instrument(Backend::new(clock.clone()), recorder.clone(), clock);
        let cancellation = if shutdown {
            InferenceExecutionCancellationHandle::shutdown_requested("controlled shutdown")
        } else {
            InferenceExecutionCancellationHandle::cancellation_requested("controlled cancellation")
        };
        assert!(gateway
            .execute_selected_text_with_cancellation(request, target, decision, cancellation)
            .await
            .is_err());
        let records = recorder.rows.lock().unwrap();
        assert_eq!(records.len(), 1);
        let lifecycle = records[0].lifecycle.as_ref().unwrap();
        assert_eq!(
            lifecycle.termination,
            if shutdown {
                RuntimeServiceTimingTermination::ShutdownRequested
            } else {
                RuntimeServiceTimingTermination::CancellationRequested
            }
        );
        assert!(lifecycle.interval.is_none());
        assert_eq!(records[0].outcome, Outcome::Failed);
        assert!(matches!(
            value(&records[0], Phase::WorkerCleanup),
            Value::Unknown {
                reason: Unknown::PhaseNotReached
            }
        ));
    }
}

struct CleanupSignal {
    flag: Arc<AtomicBool>,
    shutdown: bool,
}
impl crate::InferenceExecutionCancellationSignal for CleanupSignal {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if !self.flag.load(Ordering::SeqCst) {
            crate::InferenceExecutionCancellationSnapshot::running()
        } else if self.shutdown {
            crate::InferenceExecutionCancellationSnapshot::shutdown_requested(None)
        } else {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(None)
        }
    }
}

#[tokio::test]
async fn lifecycle_oracle_cancel_or_shutdown_during_successful_drain_is_not_success() {
    for shutdown in [false, true] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("lifecycle-oracle-content".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let flag = Arc::new(AtomicBool::new(false));
        let mut backend = Backend::new(clock.clone());
        backend.cancel_at_cleanup = Some(flag.clone());
        let gateway = instrument(backend, recorder.clone(), clock);
        assert!(gateway
            .execute_selected_text_with_cancellation(
                request,
                target,
                decision,
                InferenceExecutionCancellationHandle::with_signal(Arc::new(CleanupSignal {
                    flag,
                    shutdown
                }))
            )
            .await
            .is_err());
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let records = recorder.rows.lock().unwrap();
        assert_eq!(records.len(), 1);
        let record = &records[0];
        let lifecycle = record.lifecycle.as_ref().unwrap();
        assert_eq!(
            lifecycle.termination,
            if shutdown {
                RuntimeServiceTimingTermination::ShutdownRequested
            } else {
                RuntimeServiceTimingTermination::CancellationRequested
            }
        );
        assert_eq!(record.outcome, Outcome::Failed);
        assert_eq!(
            lifecycle.interval.as_ref().unwrap().worker_drained_at_ns,
            Some(41)
        );
        assert!(matches!(
            value(record, Phase::WorkerCleanup),
            Value::Observed {
                outcome: Outcome::Completed,
                ..
            }
        ));
        let profile = exact_profile(record);
        assert_eq!(
            protocol_builtin_clone(record)
                .fresh_production_service_interval_ns(&profile, &snapshot, 100),
            None
        );
    }
}

#[tokio::test]
async fn lifecycle_oracle_dropped_load_future_retains_partial_abandoned_work_only() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("lifecycle-oracle-content".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let entered = Arc::new(tokio::sync::Notify::new());
    let mut backend = Backend::new(clock.clone());
    backend.load_started = Some(entered.clone());
    let gateway = instrument(backend, recorder.clone(), clock.clone());
    let mut call = Box::pin(execute(&gateway, request, target, decision));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::select! { _ = entered.notified() => {}, result = &mut call => panic!("load unexpectedly completed: {result:?}") }
    }).await.unwrap();
    clock.advance(19);
    // Dropping this pinned caller future synchronously destroys the observation
    // guard before any other phase can advance. No backend stop/drain is inferred.
    drop(call);
    let records = recorder.rows.lock().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.outcome, Outcome::Abandoned);
    let lifecycle = record.lifecycle.as_ref().unwrap();
    assert_eq!(
        lifecycle.termination,
        RuntimeServiceTimingTermination::Abandoned
    );
    let interval = lifecycle.interval.as_ref().unwrap();
    assert_eq!(interval.started_at_ns, 0);
    assert_eq!(interval.observed_through_ns, 30);
    assert_eq!(interval.worker_drained_at_ns, None);
    assert!(matches!(
        value(record, Phase::SelectedModelLoad),
        Value::Observed {
            elapsed_ns: 30,
            outcome: Outcome::Abandoned
        }
    ));
    assert!(matches!(
        value(record, Phase::WorkerCleanup),
        Value::Unknown {
            reason: Unknown::PhaseNotReached
        }
    ));
}

struct OpenCleanupOnDrop(Arc<tokio::sync::Notify>);
impl Drop for OpenCleanupOnDrop {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

#[tokio::test]
async fn lifecycle_oracle_cleanup_wait_is_not_an_ack_and_caller_drop_is_censored() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("lifecycle-oracle-content".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let entered = Arc::new(tokio::sync::Notify::new());
    let continue_cleanup = Arc::new(tokio::sync::Notify::new());
    let _opener = OpenCleanupOnDrop(continue_cleanup.clone());
    let mut backend = Backend::new(clock.clone());
    backend.cleanup_started = Some(entered.clone());
    backend.cleanup_continue = Some(continue_cleanup.clone());
    let gateway = instrument(backend, recorder.clone(), clock.clone());
    let mut call = Box::pin(execute(&gateway, request, target, decision));
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::select! { _ = entered.notified() => {}, result = &mut call => panic!("cleanup unexpectedly completed: {result:?}") }
    }).await.unwrap();
    assert!(recorder.rows.lock().unwrap().is_empty());
    assert!(gateway.backend.try_write().is_err());
    clock.advance(19);
    drop(call); // actual caller-future destruction BEFORE opening the gate
    continue_cleanup.notify_one();
    let records = recorder.rows.lock().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    let lifecycle = record.lifecycle.as_ref().unwrap();
    assert_eq!(
        lifecycle.termination,
        RuntimeServiceTimingTermination::Abandoned
    );
    assert_eq!(
        lifecycle.interval.as_ref().unwrap().worker_drained_at_ns,
        None
    );
    assert!(matches!(
        value(record, Phase::WorkerCleanup),
        Value::Observed {
            outcome: Outcome::Abandoned,
            ..
        }
    ));
    assert_eq!(record.outcome, Outcome::Abandoned);
}

#[tokio::test]
async fn lifecycle_oracle_unknown_identity_preserves_intervals_without_prediction_authority() {
    for missing in ["content", "facts"] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        if missing != "content" {
            target.content_fingerprint = Some("lifecycle-oracle-content".into());
        }
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        if missing == "facts" {
            backend.facts = None;
        }
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let records = recorder.rows.lock().unwrap();
        let record = &records[0];
        assert!(matches!(
            record.identity,
            RuntimeServiceTimingIdentity::Unknown { .. }
        ));
        let lifecycle = record.lifecycle.as_ref().unwrap();
        assert_eq!(
            lifecycle.termination,
            RuntimeServiceTimingTermination::Completed
        );
        assert_eq!(
            lifecycle.interval.as_ref().unwrap().worker_drained_at_ns,
            Some(41)
        );
        let foreign =
            RuntimeServiceTimingProfile::new("other".into(), "other".into(), "a".repeat(64))
                .unwrap();
        assert_eq!(
            protocol_builtin_clone(record)
                .fresh_production_service_interval_ns(&foreign, &snapshot, 100),
            None
        );
    }
}

#[tokio::test]
async fn lifecycle_oracle_clock_regression_or_sentinel_never_qualifies_complete_duration() {
    for fault in ["load", "gap", "sentinel"] {
        let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
        target.content_fingerprint = Some("lifecycle-oracle-content".into());
        let clock = Arc::new(Clock::default());
        let recorder = Arc::new(Recorder::default());
        let mut backend = Backend::new(clock.clone());
        backend.rewind_load_clock = fault == "load";
        backend.rewind_owner_clock = fault == "gap";
        // A reset to zero must fall below a boundary the observer has read;
        // starting at zero would expose only 0 -> 0, a legitimate zero phase.
        if fault == "load" {
            clock.value.store(1, Ordering::SeqCst);
        }
        // Entering at the sentinel must not be turned into a valid zero by
        // subtraction; other fixture phases can wrap, and still cannot qualify.
        if fault == "sentinel" {
            clock.value.store(u64::MAX, Ordering::SeqCst);
        }
        let gateway = instrument(backend, recorder.clone(), clock);
        execute(&gateway, request, target, decision).await.unwrap();
        let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
        let records = recorder.rows.lock().unwrap();
        let record = &records[0];
        let profile = exact_profile(record);
        assert_eq!(
            record.fresh_production_service_interval_ns(&profile, &snapshot, 100),
            None
        );
        // Missing capture also refuses without requiring a protocol-clone helper.
        let mut clone = record.clone();
        if let Some(capture) = clone.capture.as_mut() {
            capture.owner_provenance = RuntimeServiceTimingOwnerProvenance::BuiltIn;
        }
        assert_eq!(
            clone.fresh_production_service_interval_ns(&profile, &snapshot, 100),
            None
        );
    }
}

#[tokio::test]
async fn lifecycle_oracle_accessor_requires_frozen_drain_not_publication_and_exact_history() {
    let (_directory, request, mut target, decision) = crate::selected_text_execution::fixture();
    target.content_fingerprint = Some("lifecycle-oracle-content".into());
    let clock = Arc::new(Clock::default());
    let recorder = Arc::new(Recorder::default());
    let gateway = instrument(Backend::new(clock.clone()), recorder.clone(), clock);
    execute(&gateway, request, target, decision).await.unwrap();
    let snapshot = gateway.runtime_service_timing_clock_snapshot().unwrap();
    let records = recorder.rows.lock().unwrap();
    let profile = exact_profile(&records[0]);
    let protocol = protocol_builtin_clone(&records[0]);
    assert_eq!(
        protocol.fresh_production_service_interval_ns(&profile, &snapshot, 100),
        Some(41)
    );
    let mut late = protocol.clone();
    late.capture.as_mut().unwrap().observed_at_ns += 50;
    late.lifecycle
        .as_mut()
        .unwrap()
        .interval
        .as_mut()
        .unwrap()
        .observed_through_ns += 50;
    let mut late_clock = snapshot.clone();
    late_clock.now_ns += 50;
    assert_eq!(
        late.fresh_production_service_interval_ns(&profile, &late_clock, 100),
        Some(41)
    );
    late_clock.now_ns += 100;
    assert_eq!(
        late.fresh_production_service_interval_ns(&profile, &late_clock, 100),
        Some(41)
    );
    late_clock.now_ns += 1;
    assert_eq!(
        late.fresh_production_service_interval_ns(&profile, &late_clock, 100),
        None
    );
    let mut legacy = protocol.clone();
    legacy.lifecycle = None;
    assert_eq!(
        legacy.observed_completed_ns(&profile, Phase::TextExecution),
        Some(23)
    );
    assert_eq!(
        legacy.fresh_production_service_interval_ns(&profile, &snapshot, 100),
        None
    );
    for fault in [
        "no-drain",
        "drain-before-start",
        "interval-shorter-than-phases",
        "duplicate-phase",
        "configured",
        "phase-overflow",
        "clock-epoch",
        "wrong-profile",
        "future",
    ] {
        let mut bad = protocol.clone();
        let mut clock = snapshot.clone();
        let mut expected_profile = profile.clone();
        match fault {
            "no-drain" => {
                bad.lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .worker_drained_at_ns = None
            }
            "drain-before-start" => {
                bad.lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .started_at_ns = 42
            }
            "interval-shorter-than-phases" => {
                bad.lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .started_at_ns = 1
            }
            "duplicate-phase" => bad.phases.push(bad.phases[0].clone()),
            "configured" => bad.phases[0].value = Value::ConfiguredEstimate { elapsed_ns: 0 },
            "phase-overflow" => {
                bad.phases[0].value = Value::Observed {
                    elapsed_ns: u64::MAX,
                    outcome: Outcome::Completed,
                }
            }
            "clock-epoch" => clock.clock_epoch = "different-epoch".into(),
            "wrong-profile" => {
                expected_profile = RuntimeServiceTimingProfile::new(
                    "different".into(),
                    "instance".into(),
                    "b".repeat(64),
                )
                .unwrap()
            }
            _ => clock.now_ns = 40,
        }
        assert_eq!(
            bad.fresh_production_service_interval_ns(&expected_profile, &clock, 100),
            None,
            "{fault}"
        );
    }
}
