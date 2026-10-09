//! Independently authored synthetic protocol oracle, not native evidence.
//! BuiltIn below is an explicit test label used to challenge the strict getter.
use inference::*;
use pantograph_embedded_runtime::estimate_selected_text_empirical_history_duration as estimate;
use pantograph_scheduler::*;

fn profile() -> RuntimeServiceTimingHistoryProfile {
    RuntimeServiceTimingHistoryProfile::new("protocol-owner".into(), "c".repeat(64)).unwrap()
}
fn clock() -> RuntimeServiceTimingClockSnapshot {
    RuntimeServiceTimingClockSnapshot {
        clock_epoch: "protocol-clock".into(),
        now_ns: 11_000_000,
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
fn median() -> SchedulerEmpiricalQuantile {
    SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    }
}
fn row(index: usize, total_ns: u64) -> RuntimeServiceTimingAttempt {
    let fence = format!("genuine-contract-fixture-incarnation-{index}");
    let part = total_ns / 4;
    RuntimeServiceTimingAttempt {
        attempt_id: format!("protocol-attempt-{index}"),
        execution_request_id_digest: None,
        identity: RuntimeServiceTimingIdentity::Unknown {
            reason: RuntimeServiceTimingUnavailableReason::ModelContentIdentityUnavailable,
        },
        history: Some(RuntimeServiceTimingHistoryEvidence {
            profile: profile(),
            runtime_instance_id: format!("raw-loaded-instance-{index}"),
            load_owner_fence: fence.clone(),
            drained_owner_fence: Some(fence),
        }),
        outcome: RuntimeServiceTimingOutcome::Completed,
        phases: [
            RuntimeServiceTimingPhase::GatewayCustodyWait,
            RuntimeServiceTimingPhase::SelectedModelLoad,
            RuntimeServiceTimingPhase::TextExecution,
            RuntimeServiceTimingPhase::WorkerCleanup,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, phase)| RuntimeServiceTimingPhaseEvidence {
            phase,
            value: RuntimeServiceTimingValue::Observed {
                elapsed_ns: if i == 3 { total_ns - 3 * part } else { part },
                outcome: RuntimeServiceTimingOutcome::Completed,
            },
        })
        .collect(),
        capture: Some(RuntimeServiceTimingCapture {
            clock_epoch: clock().clock_epoch,
            observed_at_ns: 10_000_000,
            owner_provenance: RuntimeServiceTimingOwnerProvenance::BuiltIn,
            load_disposition: RuntimeServiceTimingLoadDisposition::Reloaded,
        }),
        lifecycle: Some(RuntimeServiceTimingLifecycle {
            termination: RuntimeServiceTimingTermination::Completed,
            interval: Some(RuntimeServiceTimingInterval {
                started_at_ns: 10_000_000 - total_ns,
                observed_through_ns: 10_000_000,
                worker_drained_at_ns: Some(10_000_000),
            }),
        }),
    }
}
fn require_reason(
    result: SchedulerEmpiricalHistoryServiceResult<'_>,
    expected: SchedulerEmpiricalServiceIncomplete,
) {
    let SchedulerEmpiricalHistoryServiceResult::Incomplete { reason, .. } = result else {
        panic!("invalid protocol window estimated")
    };
    assert_eq!(reason, expected);
}
fn require_refusal(result: SchedulerEmpiricalHistoryServiceResult<'_>) {
    assert!(matches!(
        result,
        SchedulerEmpiricalHistoryServiceResult::Incomplete { .. }
    ));
}

#[test]
fn history_bridge_distinct_raw_instances_and_fences_keep_one_comparable_key() {
    let p = profile();
    let c = clock();
    let rows = vec![row(0, 1001), row(1, 2001), row(2, 3000)];
    let original = rows.clone();
    let report = estimate(
        &p,
        &c,
        &rows,
        median(),
        policy(),
        SchedulerEmpiricalServiceBudget::default(),
    );
    let SchedulerEmpiricalHistoryServiceResult::Estimated(value) = report.estimate else {
        panic!("complete protocol window refused")
    };
    assert_eq!(
        (
            value.duration_us,
            value.rank,
            value.support_count,
            value.total_observations
        ),
        (3, 2, 3, 3)
    );
    assert_eq!(value.source, SchedulerCompletionEvidenceSource::Measured);
    assert_eq!(value.context.owner_epoch, p.owner_epoch());
    assert_eq!(value.context.identity_fingerprint, p.identity_fingerprint());
    assert_eq!(value.context.clock_epoch, c.clock_epoch);
    assert_eq!(value.context.residency_fingerprint, "reloaded");
    assert_eq!(
        value.context.timing_convention,
        SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION
    );
    assert_eq!(report.records_examined, 3);
    assert_eq!(report.outcomes.completed, 3);
    assert_eq!(rows, original);
    // Distinct attempts on one still-current instance are also legitimate.
    let mut same = rows.clone();
    for r in &mut same {
        r.history = rows[0].history.clone();
    }
    assert!(matches!(
        estimate(
            &p,
            &c,
            &same,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default()
        )
        .estimate,
        SchedulerEmpiricalHistoryServiceResult::Estimated(_)
    ));
}

#[test]
fn history_bridge_ns_freshness_precedes_millisecond_projection() {
    let p = profile();
    let c = clock();
    let good = vec![row(0, 1000), row(1, 2000)];
    assert!(matches!(
        estimate(
            &p,
            &c,
            &good,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default()
        )
        .estimate,
        SchedulerEmpiricalHistoryServiceResult::Estimated(_)
    ));
    for case in [
        "ttl-plus-one-ns",
        "late-capture",
        "future-capture",
        "clock",
        "sentinel",
    ] {
        let mut rows = good.clone();
        let mut current = c.clone();
        match case {
            "ttl-plus-one-ns" => current.now_ns += 1,
            "late-capture" => {
                let interval = rows[0]
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap();
                interval.started_at_ns = 0;
                interval.worker_drained_at_ns = Some(1000);
                // Capture remains fresh; only intrinsic completion is stale.
                assert!(rows[0].capture.as_ref().unwrap().observed_at_ns >= 10_000_000);
            }
            "future-capture" => {
                rows[0].capture.as_mut().unwrap().observed_at_ns = current.now_ns + 1
            }
            "clock" => current.clock_epoch = "another-clock".into(),
            "sentinel" => current.now_ns = u64::MAX,
            _ => unreachable!(),
        }
        require_refusal(
            estimate(
                &p,
                &current,
                &rows,
                median(),
                policy(),
                SchedulerEmpiricalServiceBudget::default(),
            )
            .estimate,
        );
    }
}

#[test]
fn history_bridge_never_pools_reloaded_and_reused_strata() {
    let p = profile();
    let c = clock();
    let mut rows = vec![row(0, 1000), row(1, 2000)];
    rows[1].capture.as_mut().unwrap().load_disposition =
        RuntimeServiceTimingLoadDisposition::Reused;
    require_reason(
        estimate(
            &p,
            &c,
            &rows,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )
        .estimate,
        SchedulerEmpiricalServiceIncomplete::IncomparableEvidence,
    );
    rows[0].capture.as_mut().unwrap().load_disposition =
        RuntimeServiceTimingLoadDisposition::Reused;
    let SchedulerEmpiricalHistoryServiceResult::Estimated(v) = estimate(
        &p,
        &c,
        &rows,
        median(),
        policy(),
        SchedulerEmpiricalServiceBudget::default(),
    )
    .estimate
    else {
        panic!("uniform reused refused")
    };
    assert_eq!(v.context.residency_fingerprint, "reused");
    rows[0].capture.as_mut().unwrap().load_disposition =
        RuntimeServiceTimingLoadDisposition::Unknown;
    require_refusal(
        estimate(
            &p,
            &c,
            &rows,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )
        .estimate,
    );
}

#[test]
fn history_bridge_per_row_native_contract_refusal_cannot_be_bypassed_by_synthetic_policy() {
    let p = profile();
    let c = clock();
    let baseline = vec![row(0, 1000), row(1, 2000)];
    for case in 0..16 {
        let mut rows = baseline.clone();
        let r = &mut rows[0];
        match case {
            0 => r.history = None,
            1 => {
                r.history.as_mut().unwrap().profile =
                    RuntimeServiceTimingHistoryProfile::new("foreign-owner".into(), "c".repeat(64))
                        .unwrap()
            }
            2 => r.history.as_mut().unwrap().drained_owner_fence = None,
            3 => {
                r.history.as_mut().unwrap().drained_owner_fence = Some("foreign-generation".into())
            }
            4 => r.history.as_mut().unwrap().runtime_instance_id.clear(),
            5 => r.history.as_mut().unwrap().load_owner_fence = "x".repeat(257),
            6 => {
                r.capture.as_mut().unwrap().owner_provenance =
                    RuntimeServiceTimingOwnerProvenance::Injected
            }
            7 => r.capture = None,
            8 => r.lifecycle = None,
            9 => {
                r.phases.pop();
            }
            10 => r.phases[1].phase = r.phases[0].phase,
            11 => {
                r.phases[0].value =
                    RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 250 }
            }
            12 => {
                r.phases[3].value = RuntimeServiceTimingValue::Observed {
                    elapsed_ns: 250,
                    outcome: RuntimeServiceTimingOutcome::Failed,
                }
            }
            13 => {
                r.lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .worker_drained_at_ns = None
            }
            14 => {
                r.lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .started_at_ns = 10_000_001
            }
            _ => r.capture.as_mut().unwrap().clock_epoch = "x".repeat(257),
        }
        let before = rows.clone();
        let mut permissive = policy();
        permissive.allow_synthetic = true;
        require_refusal(
            estimate(
                &p,
                &c,
                &rows,
                median(),
                permissive,
                SchedulerEmpiricalServiceBudget::default(),
            )
            .estimate,
        );
        assert_eq!(rows, before, "refused case {case} altered raw records");
    }
}

#[test]
fn history_bridge_retains_every_failure_and_censor_in_supplied_window() {
    let p = profile();
    let c = clock();
    let mut rows = vec![row(0, 1000), row(1, 2000)];
    for (index, termination) in [
        RuntimeServiceTimingTermination::Failed,
        RuntimeServiceTimingTermination::CancellationRequested,
        RuntimeServiceTimingTermination::ShutdownRequested,
        RuntimeServiceTimingTermination::Abandoned,
    ]
    .into_iter()
    .enumerate()
    {
        let mut r = row(index + 2, 3000);
        r.lifecycle.as_mut().unwrap().termination = termination;
        r.outcome = if termination == RuntimeServiceTimingTermination::Abandoned {
            RuntimeServiceTimingOutcome::Abandoned
        } else {
            RuntimeServiceTimingOutcome::Failed
        };
        rows.push(r);
    }
    let mut legacy = row(6, 4000);
    legacy.lifecycle = None;
    rows.push(legacy);
    let before = rows.clone();
    let report = estimate(
        &p,
        &c,
        &rows,
        median(),
        policy(),
        SchedulerEmpiricalServiceBudget::default(),
    );
    assert_eq!(report.records_examined, 7);
    assert_eq!(
        (
            report.outcomes.completed,
            report.outcomes.failed,
            report.outcomes.cancellation_requested,
            report.outcomes.shutdown_requested,
            report.outcomes.abandoned,
            report.outcomes.legacy_unknown
        ),
        (2, 1, 1, 1, 1, 1)
    );
    require_reason(
        report.estimate,
        SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt,
    );
    assert_eq!(rows, before);
}

#[test]
fn history_bridge_duplicate_attempt_and_population_budget_refuse_without_filtering() {
    let p = profile();
    let c = clock();
    let mut rows = vec![row(0, 1000), row(1, 2000)];
    rows[1].attempt_id = rows[0].attempt_id.clone();
    require_reason(
        estimate(
            &p,
            &c,
            &rows,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )
        .estimate,
        SchedulerEmpiricalServiceIncomplete::DuplicateAttempt,
    );
    require_reason(
        estimate(
            &p,
            &c,
            &[],
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )
        .estimate,
        SchedulerEmpiricalServiceIncomplete::InsufficientSamples,
    );
    let many: Vec<_> = (0..129).map(|i| row(i, 1000)).collect();
    for (window, budget) in [
        (&many[..], SchedulerEmpiricalServiceBudget::default()),
        (
            &rows[..],
            SchedulerEmpiricalServiceBudget {
                max_work_units: SCHEDULER_EMPIRICAL_MAX_WORK + 1,
            },
        ),
    ] {
        let report = estimate(&p, &c, window, median(), policy(), budget);
        assert_eq!(report.records_examined, 0);
        assert_eq!(report.outcomes.completed, 0);
        require_reason(
            report.estimate,
            SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded,
        );
    }
    let report = estimate(
        &p,
        &c,
        &[row(0, 1000), row(1, 2000)],
        median(),
        policy(),
        SchedulerEmpiricalServiceBudget { max_work_units: 0 },
    );
    require_reason(
        report.estimate,
        SchedulerEmpiricalServiceIncomplete::BudgetExhausted,
    );
}

#[test]
fn history_bridge_rounds_joint_total_once_and_preserves_observed_zero() {
    let p = profile();
    let c = clock();
    let mut one = row(0, 1001);
    // Four phase subtotals do not include the honest 997ns inter-phase gap.
    for phase in &mut one.phases {
        phase.value = RuntimeServiceTimingValue::Observed {
            elapsed_ns: 1,
            outcome: RuntimeServiceTimingOutcome::Completed,
        };
    }
    let mut one_policy = policy();
    one_policy.minimum_samples = 1;
    let SchedulerEmpiricalHistoryServiceResult::Estimated(value) = estimate(
        &p,
        &c,
        &[one],
        median(),
        one_policy,
        SchedulerEmpiricalServiceBudget::default(),
    )
    .estimate
    else {
        panic!("gap observation refused")
    };
    assert_eq!(value.duration_us, 2);
    let SchedulerEmpiricalHistoryServiceResult::Estimated(zero) = estimate(
        &p,
        &c,
        &[row(1, 0)],
        median(),
        one_policy,
        SchedulerEmpiricalServiceBudget::default(),
    )
    .estimate
    else {
        panic!("honest zero refused")
    };
    assert_eq!(zero.duration_us, 0);
}

#[test]
fn history_bridge_scheduler_label_budget_is_not_relaxed_by_raw_observer_budget() {
    let p = RuntimeServiceTimingHistoryProfile::new("o".repeat(129), "c".repeat(64)).unwrap();
    let c = clock();
    let mut rows = vec![row(0, 1000), row(1, 2000)];
    for r in &mut rows {
        r.history.as_mut().unwrap().profile = p.clone();
    }
    assert_eq!(
        rows[0].fresh_production_history_interval_ns(&p, &c, 1_000_000),
        Some(1000)
    );
    require_reason(
        estimate(
            &p,
            &c,
            &rows,
            median(),
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )
        .estimate,
        SchedulerEmpiricalServiceIncomplete::InvalidContext,
    );
}
