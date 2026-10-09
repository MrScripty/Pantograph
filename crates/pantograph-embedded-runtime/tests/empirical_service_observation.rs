//! Hand-authored boundary fixtures; no native timing/calibration claim.
use inference::*;
use pantograph_embedded_runtime::estimate_selected_text_empirical_service_duration as estimate;
use pantograph_scheduler::*;

fn profile() -> RuntimeServiceTimingProfile {
    RuntimeServiceTimingProfile::new(
        "fixture-owner".into(),
        "fixture-generation".into(),
        "a".repeat(64),
    )
    .unwrap()
}
fn row(id: &str, total_ns: u64) -> RuntimeServiceTimingAttempt {
    let drain_ns = 10_000_000_u64.max(total_ns);
    RuntimeServiceTimingAttempt {
        history: None,
        attempt_id: id.into(),
        execution_request_id_digest: None,
        identity: RuntimeServiceTimingIdentity::Exact { profile: profile() },
        outcome: RuntimeServiceTimingOutcome::Completed,
        phases: [
            RuntimeServiceTimingPhase::GatewayCustodyWait,
            RuntimeServiceTimingPhase::SelectedModelLoad,
            RuntimeServiceTimingPhase::TextExecution,
            RuntimeServiceTimingPhase::WorkerCleanup,
        ]
        .into_iter()
        .map(|phase| RuntimeServiceTimingPhaseEvidence {
            phase,
            value: RuntimeServiceTimingValue::Observed {
                elapsed_ns: 0,
                outcome: RuntimeServiceTimingOutcome::Completed,
            },
        })
        .collect(),
        capture: Some(RuntimeServiceTimingCapture {
            clock_epoch: "fixture-clock".into(),
            observed_at_ns: drain_ns,
            owner_provenance: RuntimeServiceTimingOwnerProvenance::BuiltIn,
            load_disposition: RuntimeServiceTimingLoadDisposition::Reloaded,
        }),
        lifecycle: Some(RuntimeServiceTimingLifecycle {
            termination: RuntimeServiceTimingTermination::Completed,
            interval: Some(RuntimeServiceTimingInterval {
                started_at_ns: drain_ns - total_ns,
                observed_through_ns: drain_ns,
                worker_drained_at_ns: Some(drain_ns),
            }),
        }),
    }
}
fn policy() -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 0,
        max_sample_age_ms: 1,
        minimum_samples: 2,
        allow_synthetic: false,
    }
}
fn clock() -> RuntimeServiceTimingClockSnapshot {
    RuntimeServiceTimingClockSnapshot {
        clock_epoch: "fixture-clock".into(),
        now_ns: 11_000_000,
    }
}
fn q() -> SchedulerEmpiricalQuantile {
    SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    }
}

#[test]
fn whole_totals_round_once_and_use_exact_original_clock_freshness() {
    let mut rows = [row("a", 1_001), row("b", 2_001), row("c", 3_000)];
    for (phase, elapsed_ns) in rows[0].phases.iter_mut().zip([250, 250, 250, 251]) {
        phase.value = RuntimeServiceTimingValue::Observed {
            elapsed_ns,
            outcome: RuntimeServiceTimingOutcome::Completed,
        };
    }
    let p = profile();
    let mut single_policy = policy();
    single_policy.minimum_samples = 1;
    let single = estimate(
        &p,
        &clock(),
        &rows[..1],
        q(),
        single_policy,
        Default::default(),
    );
    assert!(matches!(
        single.estimate,
        SchedulerEmpiricalServiceResult::Estimated(SchedulerEmpiricalServiceEstimate {
            duration_us: 2,
            ..
        })
    ));
    let report = estimate(&p, &clock(), &rows, q(), policy(), Default::default());
    assert_eq!(report.records_examined, 3);
    assert_eq!(report.outcomes.completed, 3);
    let SchedulerEmpiricalServiceResult::Estimated(value) = report.estimate else {
        panic!("expected fixture estimate")
    };
    assert_eq!(value.duration_us, 3);
    assert_eq!(value.source, SchedulerCompletionEvidenceSource::Measured); // Fixture provenance contract, not a native measurement.
    assert_eq!(
        value.context.timing_convention,
        SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION
    );
    let mut expired = clock();
    expired.now_ns += 1;
    assert!(matches!(
        estimate(&p, &expired, &rows, q(), policy(), Default::default()).estimate,
        SchedulerEmpiricalServiceResult::Incomplete { .. }
    ));
}

#[test]
fn mixed_outcomes_are_reported_and_never_filtered_to_a_successful_estimate() {
    let mut rows = vec![row("a", 100), row("b", 200)];
    for (id, termination) in [
        ("failed", RuntimeServiceTimingTermination::Failed),
        (
            "cancel",
            RuntimeServiceTimingTermination::CancellationRequested,
        ),
        (
            "shutdown",
            RuntimeServiceTimingTermination::ShutdownRequested,
        ),
        ("drop", RuntimeServiceTimingTermination::Abandoned),
    ] {
        let mut r = row(id, 300);
        r.lifecycle.as_mut().unwrap().termination = termination;
        r.outcome = if termination == RuntimeServiceTimingTermination::Abandoned {
            RuntimeServiceTimingOutcome::Abandoned
        } else {
            RuntimeServiceTimingOutcome::Failed
        };
        rows.push(r);
    }
    let p = profile();
    let report = estimate(&p, &clock(), &rows, q(), policy(), Default::default());
    assert_eq!(report.outcomes.completed, 2);
    assert_eq!(report.outcomes.failed, 1);
    assert_eq!(report.outcomes.cancellation_requested, 1);
    assert_eq!(report.outcomes.shutdown_requested, 1);
    assert_eq!(report.outcomes.abandoned, 1);
    assert!(matches!(
        report.estimate,
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt,
            ..
        }
    ));
}

#[test]
fn unknown_legacy_injected_changed_generation_and_oversized_windows_refuse() {
    let p = profile();
    for bad in [
        "legacy",
        "owner",
        "injected",
        "generation",
        "clock",
        "no-drain",
        "duplicate",
    ] {
        let mut rows = [row("a", 100), row("b", 200)];
        match bad {
            "legacy" => rows[1].lifecycle = None,
            "owner" => {
                rows[1].identity = RuntimeServiceTimingIdentity::Unknown {
                    reason: RuntimeServiceTimingUnavailableReason::RuntimeOwnerFactsUnavailable,
                }
            }
            "injected" => {
                rows[1].capture.as_mut().unwrap().owner_provenance =
                    RuntimeServiceTimingOwnerProvenance::Injected
            }
            "generation" => {
                rows[1].identity = RuntimeServiceTimingIdentity::Exact {
                    profile: RuntimeServiceTimingProfile::new(
                        "fixture-owner".into(),
                        "other-generation".into(),
                        "a".repeat(64),
                    )
                    .unwrap(),
                }
            }
            "clock" => rows[1].capture.as_mut().unwrap().clock_epoch = "other-clock".into(),
            "no-drain" => {
                rows[1]
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .worker_drained_at_ns = None
            }
            "duplicate" => rows[1].attempt_id = "a".into(),
            _ => unreachable!(),
        }
        assert!(
            matches!(
                estimate(&p, &clock(), &rows, q(), policy(), Default::default()).estimate,
                SchedulerEmpiricalServiceResult::Incomplete { .. }
            ),
            "{bad}"
        );
    }
    let rows = vec![row("never-scanned", 0); 129];
    let report = estimate(&p, &clock(), &rows, q(), policy(), Default::default());
    assert_eq!(report.records_examined, 0);
    assert!(matches!(
        report.estimate,
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded,
            ..
        }
    ));
}

#[test]
fn nanosecond_projection_never_overflows_at_maximum_observed_interval() {
    let p = profile();
    let mut r = row("maximum", u64::MAX - 1);
    r.capture.as_mut().unwrap().observed_at_ns = u64::MAX - 1;
    let clock = RuntimeServiceTimingClockSnapshot {
        clock_epoch: "fixture-clock".into(),
        now_ns: u64::MAX - 1,
    };
    let mut policy = policy();
    policy.minimum_samples = 1;
    let report = estimate(&p, &clock, &[r], q(), policy, Default::default());
    assert!(matches!(
        report.estimate,
        SchedulerEmpiricalServiceResult::Estimated(SchedulerEmpiricalServiceEstimate {
            duration_us: 18_446_744_073_709_552,
            ..
        })
    ));
}

#[test]
fn fresh_publication_does_not_refresh_a_stale_intrinsic_drain_event() {
    let p = profile();
    let mut r = row("late-publication", 1_000);
    let interval = r.lifecycle.as_mut().unwrap().interval.as_mut().unwrap();
    interval.started_at_ns = 0;
    interval.worker_drained_at_ns = Some(1_000);
    let mut policy = policy();
    policy.minimum_samples = 1;
    // Capture age is exactly1ms; actual drain is much older. Existing getter
    // retains capture-age semantics, but the predictor must refuse stale drain.
    assert_eq!(
        r.fresh_production_service_interval_ns(&p, &clock(), 1_000_000),
        Some(1_000)
    );
    let report = estimate(&p, &clock(), &[r], q(), policy, Default::default());
    assert_eq!(report.records_examined, 1);
    assert_eq!(report.outcomes.completed, 1);
    assert!(matches!(
        report.estimate,
        SchedulerEmpiricalServiceResult::Incomplete { .. }
    ));
}

#[test]
fn drain_age_is_inclusive_and_enforced_before_millisecond_projection() {
    let p = profile();
    let mut r = row("delayed-capture", 1_000);
    r.capture.as_mut().unwrap().observed_at_ns = 10_500_000;
    r.lifecycle
        .as_mut()
        .unwrap()
        .interval
        .as_mut()
        .unwrap()
        .observed_through_ns = 10_500_000;
    let mut policy = policy();
    policy.minimum_samples = 1;
    let at_boundary = estimate(&p, &clock(), &[r.clone()], q(), policy, Default::default());
    assert!(matches!(
        at_boundary.estimate,
        SchedulerEmpiricalServiceResult::Estimated(_)
    ));
    let mut expired = clock();
    expired.now_ns += 1;
    // Capture still fresh, and floor(ms) would still pass: only the precise
    // actual completion age rejects this record at TTL+one nanosecond.
    assert_eq!(
        r.fresh_production_service_interval_ns(&p, &expired, 1_000_000),
        Some(1_000)
    );
    let rejected = estimate(&p, &expired, &[r], q(), policy, Default::default());
    assert_eq!(rejected.outcomes.completed, 1);
    assert!(matches!(
        rejected.estimate,
        SchedulerEmpiricalServiceResult::Incomplete { .. }
    ));
}
