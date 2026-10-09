//! Independent histogram oracle, written from the preimplementation spec.
//! No production estimator implementation or directed-test helper was read to
//! construct this reference. Synthetic numerical fixtures are not calibration.

use std::collections::BTreeMap;

use pantograph_scheduler::{
    estimate_scheduler_empirical_service_duration, SchedulerCohortCosts,
    SchedulerCompletionContext, SchedulerCompletionEvidenceSource,
    SchedulerCompletionRankingPolicy, SchedulerEmpiricalQuantile, SchedulerEmpiricalServiceBudget,
    SchedulerEmpiricalServiceCondition, SchedulerEmpiricalServiceIncomplete,
    SchedulerEmpiricalServiceObservation, SchedulerEmpiricalServiceOutcome,
    SchedulerEmpiricalServiceResult,
};

const CONVENTION: &str = "serialized-six-stage-successful-drain-samples-us.v1";

fn context() -> SchedulerCompletionContext<'static> {
    SchedulerCompletionContext {
        host_id: "oracle-host",
        runtime_instance_id: "oracle-instance",
        artifact_fingerprint: "oracle-artifact",
        workload_fingerprint: "oracle-workload",
        resource_condition_fingerprint: "oracle-resource",
        residency_fingerprint: "oracle-residency",
        timing_convention: CONVENTION,
    }
}

fn policy(minimum_samples: u32) -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 100,
        max_sample_age_ms: 20,
        minimum_samples,
        allow_synthetic: true,
    }
}

fn stages(values: [u64; 6]) -> SchedulerCohortCosts {
    SchedulerCohortCosts {
        setup_us: Some(values[0]),
        transfer_us: Some(values[1]),
        reload_us: Some(values[2]),
        execution_us: Some(values[3]),
        cleanup_us: Some(values[4]),
        retention_us: Some(values[5]),
    }
}

fn row<'a>(id: &'a str, values: [u64; 6]) -> SchedulerEmpiricalServiceObservation<'a> {
    SchedulerEmpiricalServiceObservation {
        attempt_id: id,
        context: context(),
        source: SchedulerCompletionEvidenceSource::Synthetic,
        observed_at_ms: 100,
        outcome: SchedulerEmpiricalServiceOutcome::Completed,
        costs: stages(values),
    }
}

// Counts multiplicity at each value, independently of production sorting. The
// reference does arithmetic in u128, checks representability, and does not call
// any production duration/ranking helper. Only admitted fixtures reach it.
fn histogram_reference(
    rows: &[SchedulerEmpiricalServiceObservation<'_>],
    probability: SchedulerEmpiricalQuantile,
    condition: SchedulerEmpiricalServiceCondition,
) -> Option<(u64, usize, usize)> {
    let mut histogram = BTreeMap::<u64, usize>::new();
    for sample in rows {
        let values = [
            sample.costs.setup_us?,
            sample.costs.transfer_us?,
            sample.costs.reload_us?,
            sample.costs.execution_us?,
            sample.costs.cleanup_us?,
            sample.costs.retention_us?,
        ];
        let total: u128 = values.into_iter().map(u128::from).sum();
        let total = u64::try_from(total).ok()?;
        let value = match condition {
            SchedulerEmpiricalServiceCondition::NotStarted => total,
            SchedulerEmpiricalServiceCondition::Running { elapsed_us } => {
                if total <= elapsed_us {
                    continue;
                }
                total - elapsed_us
            }
        };
        *histogram.entry(value).or_default() += 1;
    }
    let support: usize = histogram.values().sum();
    if support == 0 {
        return None;
    }
    let product = (support as u128) * u128::from(probability.numerator);
    let denominator = u128::from(probability.denominator);
    let rank =
        usize::try_from(product / denominator + u128::from(!product.is_multiple_of(denominator)))
            .unwrap();
    let mut cumulative = 0;
    for (value, multiplicity) in histogram {
        cumulative += multiplicity;
        if cumulative >= rank {
            return Some((value, support, rank));
        }
    }
    unreachable!("valid positive probability has an empirical order statistic")
}

fn assert_oracle(
    rows: &[SchedulerEmpiricalServiceObservation<'_>],
    probability: SchedulerEmpiricalQuantile,
    condition: SchedulerEmpiricalServiceCondition,
) {
    let expected = histogram_reference(rows, probability, condition);
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        rows,
        probability,
        condition,
        policy(1),
        SchedulerEmpiricalServiceBudget::default(),
    );
    match (expected, result) {
        (Some((duration, support, rank)), SchedulerEmpiricalServiceResult::Estimated(actual)) => {
            assert_eq!(actual.duration_us, duration);
            assert_eq!(actual.support_count, support);
            assert_eq!(actual.rank, rank);
            assert_eq!(actual.total_observations, rows.len());
            assert_eq!(actual.context, context());
            assert_eq!(actual.source, SchedulerCompletionEvidenceSource::Synthetic);
            assert_eq!(actual.condition, condition);
            assert_eq!(actual.quantile, probability);
            assert!(actual.work.work_units <= 32_768);
            assert_eq!(actual.work.observations_validated, rows.len());
        }
        (None, SchedulerEmpiricalServiceResult::Incomplete { reason, work }) => {
            assert_eq!(
                reason,
                SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors
            );
            assert!(work.work_units <= 32_768);
        }
        (expected, actual) => panic!("histogram={expected:?}, estimator={actual:?}"),
    }
}

fn assert_refusal(
    rows: &[SchedulerEmpiricalServiceObservation<'_>],
    expected: SchedulerEmpiricalServiceIncomplete,
) {
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        rows,
        SchedulerEmpiricalQuantile {
            numerator: 1,
            denominator: 2,
        },
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(1),
        SchedulerEmpiricalServiceBudget::default(),
    );
    match result {
        SchedulerEmpiricalServiceResult::Incomplete { reason, work } => {
            assert_eq!(reason, expected);
            assert!(work.work_units <= 32_768);
        }
        other => panic!("invalid evidence yielded {other:?}"),
    }
}

#[test]
fn histogram_oracle_exhausts_small_populations_probabilities_and_running_ages() {
    let values = [0, 1, 2, 5];
    let ids = ["a", "b", "c", "d"];
    let probabilities = [(1, 1), (1, 2), (1, 3), (2, 3), (3, 4), (u32::MAX, u32::MAX)];
    for count in 1..=4_usize {
        for mut encoding in 0..4_usize.pow(count as u32) {
            let mut rows = Vec::new();
            for (index, id) in ids[..count].iter().enumerate() {
                let total = values[encoding % 4];
                encoding /= 4;
                let mut parts = [0; 6];
                // Joint values arise from different serialized stages, including
                // cleanup and retention, rather than one reused duration helper.
                parts[index % 6] = total / 2;
                parts[(index + 2) % 6] = total - total / 2;
                rows.push(row(id, parts));
            }
            for (numerator, denominator) in probabilities {
                let probability = SchedulerEmpiricalQuantile {
                    numerator,
                    denominator,
                };
                assert_oracle(
                    &rows,
                    probability,
                    SchedulerEmpiricalServiceCondition::NotStarted,
                );
                for elapsed_us in [0, 1, 2, 5, 6, u64::MAX] {
                    assert_oracle(
                        &rows,
                        probability,
                        SchedulerEmpiricalServiceCondition::Running { elapsed_us },
                    );
                }
            }
        }
    }
}

#[test]
fn joint_quantile_reverses_the_incorrect_sum_of_phase_quantiles() {
    let a = [
        row("a0", [10, 0, 0, 0, 0, 0]),
        row("a1", [0, 10, 0, 0, 0, 0]),
        row("a2", [10, 0, 0, 0, 0, 0]),
        row("a3", [0, 10, 0, 0, 0, 0]),
    ];
    let b = [
        row("b0", [6, 6, 0, 0, 0, 0]),
        row("b1", [6, 6, 0, 0, 0, 0]),
        row("b2", [6, 6, 0, 0, 0, 0]),
        row("b3", [6, 6, 0, 0, 0, 0]),
    ];
    let p = SchedulerEmpiricalQuantile {
        numerator: 3,
        denominator: 4,
    };
    let condition = SchedulerEmpiricalServiceCondition::NotStarted;
    assert_eq!(histogram_reference(&a, p, condition), Some((10, 4, 3)));
    assert_eq!(histogram_reference(&b, p, condition), Some((12, 4, 3)));
    assert_oracle(&a, p, condition);
    assert_oracle(&b, p, condition);
    // Each A phase marginal has p75=10: sum20 falsely puts B's12 first.
    let joint_a = histogram_reference(&a, p, condition).unwrap().0;
    let joint_b = histogram_reference(&b, p, condition).unwrap().0;
    let setup_only: Vec<_> = a
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            row(
                a[i].attempt_id,
                [sample.costs.setup_us.unwrap(), 0, 0, 0, 0, 0],
            )
        })
        .collect();
    let transfer_only: Vec<_> = a
        .iter()
        .enumerate()
        .map(|(i, sample)| {
            row(
                a[i].attempt_id,
                [0, sample.costs.transfer_us.unwrap(), 0, 0, 0, 0],
            )
        })
        .collect();
    let incorrect_marginal_sum = histogram_reference(&setup_only, p, condition).unwrap().0
        + histogram_reference(&transfer_only, p, condition).unwrap().0;
    assert!(joint_a < joint_b);
    assert!(incorrect_marginal_sum > joint_b);
}

#[test]
fn conditional_survival_is_not_unconditional_quantile_minus_elapsed() {
    let ids = ["s0", "s1", "s2", "s3", "s4", "s5"];
    let rows: Vec<_> = ids
        .iter()
        .zip([1, 2, 3, 10, 20, 30])
        .map(|(id, total)| row(id, [0, 0, 0, total, 0, 0]))
        .collect();
    let p = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    };
    assert_eq!(
        histogram_reference(&rows, p, SchedulerEmpiricalServiceCondition::NotStarted),
        Some((3, 6, 3))
    );
    let running = SchedulerEmpiricalServiceCondition::Running { elapsed_us: 3 };
    assert_eq!(histogram_reference(&rows, p, running), Some((17, 3, 2)));
    assert_oracle(&rows, p, running);
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        &rows,
        p,
        running,
        policy(4),
        SchedulerEmpiricalServiceBudget::default(),
    );
    assert!(matches!(
        result,
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors,
            ..
        }
    ));
}

#[test]
fn equality_atoms_are_excluded_and_duplicate_durations_keep_multiplicity() {
    let ids = ["z", "eq0", "eq1", "eq2", "tail0", "tail1", "tail2"];
    let rows: Vec<_> = ids
        .iter()
        .zip([0, 5, 5, 5, 10, 10, 10])
        .map(|(id, total)| row(id, [0, 0, 0, 0, 0, total]))
        .collect();
    let p = SchedulerEmpiricalQuantile {
        numerator: 2,
        denominator: 3,
    };
    let running = SchedulerEmpiricalServiceCondition::Running { elapsed_us: 5 };
    assert_eq!(histogram_reference(&rows, p, running), Some((5, 3, 2)));
    assert_oracle(&rows, p, running);
    let zero = [row("z0", [0; 6]), row("z1", [0; 6]), row("z2", [0; 6])];
    assert_oracle(&zero, p, SchedulerEmpiricalServiceCondition::NotStarted);
    assert_oracle(
        &zero,
        p,
        SchedulerEmpiricalServiceCondition::Running { elapsed_us: 0 },
    );
}

#[test]
fn all_six_stages_are_joint_and_checked_without_tail_clipping() {
    let p = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 1,
    };
    for index in 0..6 {
        let mut parts = [0; 6];
        parts[index] = u64::MAX;
        let rows = [row("max", parts)];
        assert_oracle(&rows, p, SchedulerEmpiricalServiceCondition::NotStarted);
        assert_oracle(
            &rows,
            p,
            SchedulerEmpiricalServiceCondition::Running {
                elapsed_us: u64::MAX - 1,
            },
        );
        parts[(index + 1) % 6] = 1;
        assert_refusal(
            &[row("overflow", parts)],
            SchedulerEmpiricalServiceIncomplete::DurationOverflow,
        );
        let mut missing = row("missing", [1; 6]);
        match index {
            0 => missing.costs.setup_us = None,
            1 => missing.costs.transfer_us = None,
            2 => missing.costs.reload_us = None,
            3 => missing.costs.execution_us = None,
            4 => missing.costs.cleanup_us = None,
            _ => missing.costs.retention_us = None,
        }
        assert_refusal(
            &[missing],
            SchedulerEmpiricalServiceIncomplete::MissingStage,
        );
    }
    assert_oracle(
        &[row("six", [1, 2, 3, 4, 5, 6])],
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
    );
}

#[test]
fn every_context_field_is_exact_not_the_legacy_partial_comparator() {
    for field in 0..7 {
        let mut changed = row("context", [1; 6]);
        match field {
            0 => changed.context.host_id = "different-host",
            1 => changed.context.runtime_instance_id = "different-instance",
            2 => changed.context.artifact_fingerprint = "different-artifact",
            3 => changed.context.workload_fingerprint = "different-workload",
            4 => changed.context.resource_condition_fingerprint = "different-resource",
            5 => changed.context.residency_fingerprint = "different-residency",
            _ => changed.context.timing_convention = "old-mean-convention",
        }
        // No estimate is allowed even when the changed row falls outside the tail.
        let result = estimate_scheduler_empirical_service_duration(
            context(),
            &[changed],
            SchedulerEmpiricalQuantile {
                numerator: 1,
                denominator: 2,
            },
            SchedulerEmpiricalServiceCondition::Running { elapsed_us: 100 },
            policy(1),
            SchedulerEmpiricalServiceBudget::default(),
        );
        assert!(matches!(
            result,
            SchedulerEmpiricalServiceResult::Incomplete { .. }
        ));
    }
}

#[test]
fn unsuccessful_stale_duplicate_and_mixed_source_rows_are_never_dropped() {
    for outcome in [
        SchedulerEmpiricalServiceOutcome::Failed,
        SchedulerEmpiricalServiceOutcome::Cancelled,
        SchedulerEmpiricalServiceOutcome::Incomplete,
    ] {
        let mut bad = row("bad", [0; 6]);
        bad.outcome = outcome;
        assert_refusal(
            &[bad],
            SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt,
        );
    }
    for timestamp in [79, 101, u64::MAX] {
        let mut bad = row("time", [1; 6]);
        bad.observed_at_ms = timestamp;
        assert_refusal(
            &[bad],
            SchedulerEmpiricalServiceIncomplete::StaleOrFutureSample,
        );
    }
    let duplicate = [row("same", [1; 6]), row("same", [2; 6])];
    assert_refusal(
        &duplicate,
        SchedulerEmpiricalServiceIncomplete::DuplicateAttempt,
    );
    let mut measured = row("measured", [1; 6]);
    measured.source = SchedulerCompletionEvidenceSource::Measured;
    assert_refusal(
        &[row("synthetic", [1; 6]), measured],
        SchedulerEmpiricalServiceIncomplete::IncomparableEvidence,
    );
    let mut boundary = row("fresh", [1; 6]);
    boundary.observed_at_ms = 80;
    assert_oracle(
        &[boundary],
        SchedulerEmpiricalQuantile {
            numerator: 1,
            denominator: 1,
        },
        SchedulerEmpiricalServiceCondition::NotStarted,
    );
}

#[test]
fn invalid_probability_identity_policy_and_synthetic_permission_refuse() {
    let rows = [row("good", [1; 6])];
    for (numerator, denominator) in [(0, 1), (1, 0), (2, 1), (u32::MAX, u32::MAX - 1)] {
        let result = estimate_scheduler_empirical_service_duration(
            context(),
            &rows,
            SchedulerEmpiricalQuantile {
                numerator,
                denominator,
            },
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy(1),
            SchedulerEmpiricalServiceBudget::default(),
        );
        assert!(matches!(
            result,
            SchedulerEmpiricalServiceResult::Incomplete {
                reason: SchedulerEmpiricalServiceIncomplete::InvalidQuantile,
                ..
            }
        ));
    }
    let overlong = "x".repeat(129);
    for id in ["", "   ", "line\nbreak", overlong.as_str()] {
        assert_refusal(
            &[row(id, [1; 6])],
            SchedulerEmpiricalServiceIncomplete::InvalidAttemptIdentity,
        );
    }
    let mut disabled = policy(1);
    disabled.allow_synthetic = false;
    assert!(matches!(
        estimate_scheduler_empirical_service_duration(
            context(),
            &rows,
            SchedulerEmpiricalQuantile {
                numerator: 1,
                denominator: 2
            },
            SchedulerEmpiricalServiceCondition::NotStarted,
            disabled,
            SchedulerEmpiricalServiceBudget::default()
        ),
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::SyntheticEvidenceDisabled,
            ..
        }
    ));
    for broken in [
        SchedulerCompletionRankingPolicy {
            minimum_samples: 0,
            ..policy(1)
        },
        SchedulerCompletionRankingPolicy {
            max_sample_age_ms: 0,
            ..policy(1)
        },
    ] {
        assert!(matches!(
            estimate_scheduler_empirical_service_duration(
                context(),
                &rows,
                SchedulerEmpiricalQuantile {
                    numerator: 1,
                    denominator: 2
                },
                SchedulerEmpiricalServiceCondition::NotStarted,
                broken,
                SchedulerEmpiricalServiceBudget::default()
            ),
            SchedulerEmpiricalServiceResult::Incomplete {
                reason: SchedulerEmpiricalServiceIncomplete::InvalidPolicy,
                ..
            }
        ));
    }
}

#[test]
fn maximum_reverse_population_and_boundary_fields_fit_the_deterministic_cap() {
    let ids: Vec<_> = (0..128).map(|i| format!("{i:0128}")).collect();
    let field = "x".repeat(128);
    let current = SchedulerCompletionContext {
        host_id: &field,
        runtime_instance_id: &field,
        artifact_fingerprint: &field,
        workload_fingerprint: &field,
        resource_condition_fingerprint: &field,
        residency_fingerprint: &field,
        timing_convention: CONVENTION,
    };
    let mut rows: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut sample = row(id, [0, 0, 0, 127 - i as u64, 0, 0]);
            sample.context = current;
            sample
        })
        .collect();
    for probability in [
        (1, 128),
        (1, 2),
        (127, 128),
        (u32::MAX - 1, u32::MAX),
        (1, 1),
    ] {
        let p = SchedulerEmpiricalQuantile {
            numerator: probability.0,
            denominator: probability.1,
        };
        for _ in 0..2 {
            let expected =
                histogram_reference(&rows, p, SchedulerEmpiricalServiceCondition::NotStarted)
                    .unwrap();
            let actual = estimate_scheduler_empirical_service_duration(
                current,
                &rows,
                p,
                SchedulerEmpiricalServiceCondition::NotStarted,
                policy(3),
                SchedulerEmpiricalServiceBudget::default(),
            );
            match actual {
                SchedulerEmpiricalServiceResult::Estimated(est) => {
                    assert_eq!((est.duration_us, est.support_count, est.rank), expected);
                    assert_eq!(est.total_observations, 128);
                    assert!(est.work.work_units <= 32_768);
                }
                other => panic!("complete maximum population refused: {other:?}"),
            }
            rows.reverse();
        }
    }
    let p = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    };
    let limited = estimate_scheduler_empirical_service_duration(
        current,
        &rows,
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(3),
        SchedulerEmpiricalServiceBudget { max_work_units: 16 },
    );
    match limited {
        SchedulerEmpiricalServiceResult::Incomplete { reason, work } => {
            assert_eq!(reason, SchedulerEmpiricalServiceIncomplete::BudgetExhausted);
            assert!(work.work_units <= 16);
        }
        other => panic!("tiny budget yielded {other:?}"),
    }
    let mut oversized = rows;
    oversized.push(row("extra", [1; 6]));
    let actual = estimate_scheduler_empirical_service_duration(
        current,
        &oversized,
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(3),
        SchedulerEmpiricalServiceBudget::default(),
    );
    match actual {
        SchedulerEmpiricalServiceResult::Incomplete { reason, work } => {
            assert_eq!(
                reason,
                SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded
            );
            assert_eq!(work.work_units, 0);
            assert_eq!(work.observations_validated, 0);
            assert_eq!(work.identity_comparisons, 0);
            assert_eq!(work.order_comparisons, 0);
            assert_eq!(work.order_swaps, 0);
        }
        other => panic!("oversized population yielded {other:?}"),
    }
}

#[test]
fn original_support_measured_provenance_and_raw_context_bounds_are_enforced() {
    let rows = [row("one", [1; 6]), row("two", [2; 6])];
    let p = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    };
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        &rows,
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(3),
        SchedulerEmpiricalServiceBudget::default(),
    );
    assert!(matches!(
        result,
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::InsufficientSamples,
            ..
        }
    ));
    assert_refusal(
        &[],
        SchedulerEmpiricalServiceIncomplete::InsufficientSamples,
    );
    let mut measured = row("measured", [1; 6]);
    measured.source = SchedulerCompletionEvidenceSource::Measured;
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        &[measured],
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
        SchedulerCompletionRankingPolicy {
            allow_synthetic: false,
            ..policy(1)
        },
        SchedulerEmpiricalServiceBudget::default(),
    );
    match result {
        SchedulerEmpiricalServiceResult::Estimated(est) => {
            assert_eq!(est.source, SchedulerCompletionEvidenceSource::Measured);
            assert_eq!(est.duration_us, 6);
        }
        other => panic!("measured evidence falsely needed synthetic permission: {other:?}"),
    }
    let overlong = "x".repeat(129);
    for field in 0..7 {
        let mut bad = row("raw", [1; 6]);
        match field {
            0 => bad.context.host_id = &overlong,
            1 => bad.context.runtime_instance_id = &overlong,
            2 => bad.context.artifact_fingerprint = &overlong,
            3 => bad.context.workload_fingerprint = &overlong,
            4 => bad.context.resource_condition_fingerprint = &overlong,
            5 => bad.context.residency_fingerprint = &overlong,
            _ => bad.context.timing_convention = &overlong,
        }
        assert_refusal(&[bad], SchedulerEmpiricalServiceIncomplete::InvalidContext);
    }
    let mut bad = row("missing-before-elapsed", [0; 6]);
    bad.costs.cleanup_us = None;
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        &[bad],
        p,
        SchedulerEmpiricalServiceCondition::Running {
            elapsed_us: u64::MAX,
        },
        policy(1),
        SchedulerEmpiricalServiceBudget::default(),
    );
    assert!(matches!(
        result,
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::MissingStage,
            ..
        }
    ));
    let result = estimate_scheduler_empirical_service_duration(
        context(),
        &rows,
        p,
        SchedulerEmpiricalServiceCondition::NotStarted,
        policy(1),
        SchedulerEmpiricalServiceBudget {
            max_work_units: 32_769,
        },
    );
    assert!(matches!(
        result,
        SchedulerEmpiricalServiceResult::Incomplete { .. }
    ));
}
