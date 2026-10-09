use pantograph_scheduler::*;

fn context() -> SchedulerCompletionContext<'static> {
    SchedulerCompletionContext {
        host_id: "fixture-host",
        runtime_instance_id: "fixture-owner-generation",
        artifact_fingerprint: "fixture-artifact",
        workload_fingerprint: "fixture-workload",
        resource_condition_fingerprint: "fixture-serialized-idle",
        residency_fingerprint: "fixture-cold",
        timing_convention: SCHEDULER_EMPIRICAL_SERVICE_CONVENTION,
    }
}
fn costs(setup: u64, execution: u64) -> SchedulerCohortCosts {
    SchedulerCohortCosts {
        setup_us: Some(setup),
        transfer_us: Some(0),
        execution_us: Some(execution),
        cleanup_us: Some(0),
        retention_us: Some(0),
        reload_us: Some(0),
    }
}
fn row(id: &str, costs: SchedulerCohortCosts) -> SchedulerEmpiricalServiceObservation<'_> {
    SchedulerEmpiricalServiceObservation {
        attempt_id: id,
        context: context(),
        source: SchedulerCompletionEvidenceSource::Synthetic,
        observed_at_ms: 900,
        outcome: SchedulerEmpiricalServiceOutcome::Completed,
        costs,
    }
}
fn policy() -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 1000,
        max_sample_age_ms: 100,
        minimum_samples: 3,
        allow_synthetic: true,
    }
}
fn estimate<'a>(
    rows: &[SchedulerEmpiricalServiceObservation<'_>],
    context: SchedulerCompletionContext<'a>,
    condition: SchedulerEmpiricalServiceCondition,
    numerator: u32,
    denominator: u32,
) -> SchedulerEmpiricalServiceEstimate<'a> {
    match estimate_scheduler_empirical_service_duration(
        context,
        rows,
        SchedulerEmpiricalQuantile {
            numerator,
            denominator,
        },
        condition,
        policy(),
        SchedulerEmpiricalServiceBudget::default(),
    ) {
        SchedulerEmpiricalServiceResult::Estimated(estimate) => estimate,
        other => panic!("expected empirical estimate: {other:?}"),
    }
}
fn incomplete(result: SchedulerEmpiricalServiceResult<'_>) -> SchedulerEmpiricalServiceIncomplete {
    match result {
        SchedulerEmpiricalServiceResult::Incomplete { reason, .. } => reason,
        other => panic!("expected no estimate: {other:?}"),
    }
}

#[test]
fn paired_stages_preserve_joint_order_that_marginal_quantiles_reverse() {
    let a = [
        row("a1", costs(10, 0)),
        row("a2", costs(0, 10)),
        row("a3", costs(10, 0)),
        row("a4", costs(0, 10)),
    ];
    let b = [
        row("b1", costs(6, 6)),
        row("b2", costs(6, 6)),
        row("b3", costs(6, 6)),
        row("b4", costs(6, 6)),
    ];
    let condition = SchedulerEmpiricalServiceCondition::NotStarted;
    let a = estimate(&a, context(), condition, 3, 4);
    let b = estimate(&b, context(), condition, 3, 4);
    assert_eq!((a.duration_us, b.duration_us), (10, 12));
    assert!(a.duration_us < b.duration_us);
    assert!(
        10 + 10 > b.duration_us,
        "summed marginal p75 would reverse this numerical order"
    );
    assert_eq!(a.source, SchedulerCompletionEvidenceSource::Synthetic);
    assert_eq!((a.total_observations, a.support_count, a.rank), (4, 4, 3));
}

#[test]
fn running_residual_conditions_before_quantiles_and_uses_strict_survival() {
    let rows = [
        row("a", costs(0, 1)),
        row("b", costs(0, 2)),
        row("c", costs(0, 3)),
        row("d", costs(0, 10)),
        row("e", costs(0, 20)),
        row("f", costs(0, 30)),
    ];
    let fresh = estimate(
        &rows,
        context(),
        SchedulerEmpiricalServiceCondition::NotStarted,
        1,
        2,
    );
    let running = estimate(
        &rows,
        context(),
        SchedulerEmpiricalServiceCondition::Running { elapsed_us: 3 },
        1,
        2,
    );
    assert_eq!(fresh.duration_us, 3);
    assert_eq!(
        (running.duration_us, running.support_count, running.rank),
        (17, 3, 2)
    );
    assert_eq!(running.total_observations, 6);
    let atoms = [
        row("a", costs(0, 0)),
        row("b", costs(0, 5)),
        row("c", costs(0, 5)),
        row("d", costs(0, 5)),
        row("e", costs(0, 10)),
        row("f", costs(0, 10)),
        row("g", costs(0, 10)),
    ];
    let running = estimate(
        &atoms,
        context(),
        SchedulerEmpiricalServiceCondition::Running { elapsed_us: 5 },
        1,
        2,
    );
    assert_eq!((running.duration_us, running.support_count), (5, 3));
}

#[test]
fn complete_service_includes_drain_and_keeps_explicit_zero_distinct_from_missing() {
    let full = SchedulerCohortCosts {
        setup_us: Some(1),
        transfer_us: Some(2),
        execution_us: Some(4),
        cleanup_us: Some(8),
        retention_us: Some(16),
        reload_us: Some(32),
    };
    let rows = [row("a", full), row("b", full), row("c", full)];
    assert_eq!(
        estimate(
            &rows,
            context(),
            SchedulerEmpiricalServiceCondition::NotStarted,
            1,
            1
        )
        .duration_us,
        63
    );
    let zero = [
        row("a", costs(0, 0)),
        row("b", costs(0, 0)),
        row("c", costs(0, 0)),
    ];
    assert_eq!(
        estimate(
            &zero,
            context(),
            SchedulerEmpiricalServiceCondition::NotStarted,
            1,
            1
        )
        .duration_us,
        0
    );
    for elapsed_us in [0, u64::MAX] {
        assert_eq!(
            incomplete(estimate_scheduler_empirical_service_duration(
                context(),
                &zero,
                SchedulerEmpiricalQuantile {
                    numerator: 1,
                    denominator: 1
                },
                SchedulerEmpiricalServiceCondition::Running { elapsed_us },
                policy(),
                SchedulerEmpiricalServiceBudget::default(),
            )),
            SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors
        );
    }
}

#[test]
fn non_surviving_bad_rows_are_not_dropped_and_tail_support_is_not_original_support() {
    let mut rows = [
        row("a", costs(0, 1)),
        row("b", costs(0, 10)),
        row("c", costs(0, 20)),
    ];
    let run = |rows: &[SchedulerEmpiricalServiceObservation<'_>]| {
        incomplete(estimate_scheduler_empirical_service_duration(
            context(),
            rows,
            SchedulerEmpiricalQuantile {
                numerator: 1,
                denominator: 2,
            },
            SchedulerEmpiricalServiceCondition::Running { elapsed_us: 1 },
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        ))
    };
    assert_eq!(
        run(&rows),
        SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors
    );
    rows[0].observed_at_ms = 899;
    assert_eq!(
        run(&rows),
        SchedulerEmpiricalServiceIncomplete::StaleOrFutureSample
    );
    rows[0].observed_at_ms = 900;
    for outcome in [
        SchedulerEmpiricalServiceOutcome::Cancelled,
        SchedulerEmpiricalServiceOutcome::Failed,
        SchedulerEmpiricalServiceOutcome::Incomplete,
    ] {
        rows[0].outcome = outcome;
        assert_eq!(
            run(&rows),
            SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt
        );
    }
    rows[0].outcome = SchedulerEmpiricalServiceOutcome::Completed;
    rows[0].costs.reload_us = None;
    assert_eq!(
        run(&rows),
        SchedulerEmpiricalServiceIncomplete::MissingStage
    );
}

#[test]
fn extreme_rational_rank_and_total_are_checked_without_clipping_the_tail() {
    let mut rows = [
        row("a", costs(0, 1)),
        row("b", costs(0, 2)),
        row("c", costs(0, u64::MAX)),
    ];
    let result = estimate(
        &rows,
        context(),
        SchedulerEmpiricalServiceCondition::NotStarted,
        u32::MAX,
        u32::MAX,
    );
    assert_eq!((result.duration_us, result.rank), (u64::MAX, 3));
    rows[2].costs.setup_us = Some(1);
    assert_eq!(
        incomplete(estimate_scheduler_empirical_service_duration(
            context(),
            &rows,
            SchedulerEmpiricalQuantile {
                numerator: 1,
                denominator: 1
            },
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy(),
            SchedulerEmpiricalServiceBudget::default(),
        )),
        SchedulerEmpiricalServiceIncomplete::DurationOverflow
    );
}

#[test]
fn maximum_reverse_population_is_fully_bounded_and_budget_failure_returns_no_partial_estimate() {
    let field = "x".repeat(128);
    let context = SchedulerCompletionContext {
        host_id: &field,
        runtime_instance_id: &field,
        artifact_fingerprint: &field,
        workload_fingerprint: &field,
        resource_condition_fingerprint: &field,
        residency_fingerprint: &field,
        timing_convention: SCHEDULER_EMPIRICAL_SERVICE_CONVENTION,
    };
    let ids: Vec<_> = (0..128)
        .map(|i| format!("{i:03}{}", "x".repeat(125)))
        .collect();
    let rows: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut r = row(id, costs(0, 128 - i as u64));
            r.context = context;
            r
        })
        .collect();
    let quantile = SchedulerEmpiricalQuantile {
        numerator: 3,
        denominator: 4,
    };
    let run = |max_work_units| {
        estimate_scheduler_empirical_service_duration(
            context,
            &rows,
            quantile,
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy(),
            SchedulerEmpiricalServiceBudget { max_work_units },
        )
    };
    let result = estimate(
        &rows,
        context,
        SchedulerEmpiricalServiceCondition::NotStarted,
        3,
        4,
    );
    assert_eq!(
        (result.duration_us, result.rank, result.support_count),
        (96, 96, 128)
    );
    assert_eq!(
        (
            result.work.identity_comparisons,
            result.work.order_comparisons,
            result.work.order_swaps
        ),
        (8128, 8128, 8128)
    );
    assert!(result.work.work_units < SCHEDULER_EMPIRICAL_MAX_WORK);
    assert_eq!(
        run(result.work.work_units),
        SchedulerEmpiricalServiceResult::Estimated(result.clone())
    );
    for limit in [0, 1, result.work.work_units - 1] {
        match run(limit) {
            SchedulerEmpiricalServiceResult::Incomplete { reason, work } => {
                assert_eq!(reason, SchedulerEmpiricalServiceIncomplete::BudgetExhausted);
                assert!(work.work_units <= limit);
            }
            other => panic!("budget returned partial estimate: {other:?}"),
        }
    }
    assert_eq!(
        incomplete(run(SCHEDULER_EMPIRICAL_MAX_WORK + 1)),
        SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded
    );
    let mut oversized = rows.clone();
    oversized.push(rows[0]);
    match estimate_scheduler_empirical_service_duration(
        context,
        &oversized,
        quantile,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(),
        SchedulerEmpiricalServiceBudget::default(),
    ) {
        SchedulerEmpiricalServiceResult::Incomplete { reason, work } => {
            assert_eq!(
                reason,
                SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded
            );
            assert_eq!(work.work_units, 0);
        }
        other => panic!("oversized population evaluated: {other:?}"),
    }
}
