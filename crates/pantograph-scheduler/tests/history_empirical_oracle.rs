//! Independent synthetic histogram oracle. No native measurements or selection.
//! Authored from the frozen prospective contract before new implementation read.
use pantograph_scheduler::*;
use std::collections::BTreeMap;

const KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OTHER_KEY: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn context<'a>() -> SchedulerEmpiricalHistoryContext<'a> {
    SchedulerEmpiricalHistoryContext {
        owner_epoch: "oracle-owner",
        identity_fingerprint: KEY,
        clock_epoch: "oracle-clock",
        residency_fingerprint: "reloaded",
        timing_convention: SCHEDULER_EMPIRICAL_HISTORY_TOTAL_CONVENTION,
    }
}

fn policy() -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 12,
        max_sample_age_ms: 2,
        minimum_samples: 1,
        allow_synthetic: true,
    }
}

fn row(id: &str, duration: u64) -> SchedulerEmpiricalHistoryTotalObservation<'_> {
    SchedulerEmpiricalHistoryTotalObservation {
        attempt_id: id,
        context: context(),
        source: SchedulerCompletionEvidenceSource::Synthetic,
        observed_at_ms: 10,
        outcome: SchedulerEmpiricalServiceOutcome::Completed,
        duration_us: Some(duration),
    }
}

fn reference(values: &[u64], q: SchedulerEmpiricalQuantile) -> (usize, u64) {
    assert!(!values.is_empty() && q.numerator > 0 && q.numerator <= q.denominator);
    let rank = usize::try_from(
        (u128::try_from(values.len()).unwrap() * u128::from(q.numerator))
            .div_ceil(u128::from(q.denominator)),
    )
    .unwrap();
    let mut histogram = BTreeMap::<u64, usize>::new();
    for value in values {
        *histogram.entry(*value).or_default() += 1;
    }
    let mut prefix = 0;
    for (value, frequency) in histogram {
        prefix += frequency;
        if prefix >= rank {
            return (rank, value);
        }
    }
    unreachable!("valid rank is inside complete histogram")
}

fn require_estimate(
    result: SchedulerEmpiricalHistoryServiceResult<'_>,
) -> SchedulerEmpiricalHistoryServiceEstimate<'_> {
    match result {
        SchedulerEmpiricalHistoryServiceResult::Estimated(estimate) => estimate,
        other => panic!("expected synthetic complete history estimate: {other:?}"),
    }
}

fn require_reason(
    result: SchedulerEmpiricalHistoryServiceResult<'_>,
    expected: SchedulerEmpiricalServiceIncomplete,
) {
    let SchedulerEmpiricalHistoryServiceResult::Incomplete { reason, work } = result else {
        panic!("invalid history unexpectedly estimated")
    };
    assert_eq!(reason, expected);
    assert!(work.work_units <= SCHEDULER_EMPIRICAL_MAX_WORK);
}

#[test]
fn history_histogram_generated_differential_is_exact() {
    let probabilities = [(1, 1), (1, 2), (1, 3), (2, 3), (3, 4)];
    for count in 1..=12 {
        let ids: Vec<_> = (0..count).map(|i| format!("generated-{i}")).collect();
        for seed in 0..24_u64 {
            let values: Vec<_> = (0..count)
                .map(|i| (seed * 37 + u64::try_from(i).unwrap() * 19 + seed / 3) % 31)
                .collect();
            let samples: Vec<_> = ids.iter().zip(&values).map(|(id, d)| row(id, *d)).collect();
            for (numerator, denominator) in probabilities {
                let q = SchedulerEmpiricalQuantile {
                    numerator,
                    denominator,
                };
                let (rank, duration) = reference(&values, q);
                let value = require_estimate(estimate_scheduler_empirical_history_total_duration(
                    context(),
                    &samples,
                    q,
                    policy(),
                    Default::default(),
                ));
                assert_eq!((value.rank, value.duration_us), (rank, duration));
                assert_eq!(value.total_observations, count);
                assert_eq!(value.support_count, count);
                assert_eq!(value.quantile, q);
                assert_eq!(value.source, SchedulerCompletionEvidenceSource::Synthetic);
                assert_eq!(value.context.clock_epoch, "oracle-clock");
                assert!(value.work.work_units <= SCHEDULER_EMPIRICAL_MAX_WORK);
            }
        }
    }
}

#[test]
fn history_zero_ties_extreme_values_and_exact_rational_ranks() {
    for values in [
        vec![0],
        vec![0, 0, 0, u64::MAX],
        vec![u64::MAX, u64::MAX],
        vec![9, 1, 9, 1, 9, 1],
    ] {
        let ids: Vec<_> = (0..values.len()).map(|i| format!("edge-{i}")).collect();
        let samples: Vec<_> = ids.iter().zip(&values).map(|(id, d)| row(id, *d)).collect();
        for (numerator, denominator) in [
            (1, u32::MAX),
            (1, 2),
            (2, 3),
            (u32::MAX - 1, u32::MAX),
            (u32::MAX, u32::MAX),
        ] {
            let q = SchedulerEmpiricalQuantile {
                numerator,
                denominator,
            };
            let expected = reference(&values, q);
            let value = require_estimate(estimate_scheduler_empirical_history_total_duration(
                context(),
                &samples,
                q,
                policy(),
                Default::default(),
            ));
            assert_eq!((value.rank, value.duration_us), expected);
        }
    }
}

#[test]
fn history_permutations_preserve_values_and_increasing_transform() {
    let mut values = vec![0, 9, 9, 2, 25, 3, 1];
    let ids: Vec<_> = (0..values.len()).map(|i| format!("order-{i}")).collect();
    let q = SchedulerEmpiricalQuantile {
        numerator: 3,
        denominator: 5,
    };
    let expected = reference(&values, q);
    for reverse in [false, true] {
        if reverse {
            values.reverse();
        }
        for _ in 0..values.len() {
            values.rotate_left(1);
            let samples: Vec<_> = ids.iter().zip(&values).map(|(id, d)| row(id, *d)).collect();
            let estimate = require_estimate(estimate_scheduler_empirical_history_total_duration(
                context(),
                &samples,
                q,
                policy(),
                Default::default(),
            ));
            assert_eq!((estimate.rank, estimate.duration_us), expected);
            let transformed: Vec<_> = ids
                .iter()
                .zip(&values)
                .map(|(id, d)| row(id, d * 7 + 4))
                .collect();
            let transformed_estimate =
                require_estimate(estimate_scheduler_empirical_history_total_duration(
                    context(),
                    &transformed,
                    q,
                    policy(),
                    Default::default(),
                ));
            assert_eq!(transformed_estimate.duration_us, expected.1 * 7 + 4);
        }
    }
}

#[test]
fn history_every_context_field_and_source_are_exact() {
    let q = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    };
    for field in 0..5 {
        let mut samples = [row("left", 7), row("right", 11)];
        match field {
            0 => samples[1].context.owner_epoch = "other-owner",
            1 => samples[1].context.identity_fingerprint = OTHER_KEY,
            2 => samples[1].context.clock_epoch = "other-clock",
            3 => samples[1].context.residency_fingerprint = "reused",
            4 => samples[1].context.timing_convention = "foreign-convention.v1",
            _ => unreachable!(),
        }
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &samples,
                q,
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::IncomparableEvidence,
        );
    }
    let mut samples = [row("one", 7), row("two", 11)];
    samples[1].source = SchedulerCompletionEvidenceSource::Measured;
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &samples,
            q,
            policy(),
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::IncomparableEvidence,
    );
    let mut measured_only = policy();
    measured_only.allow_synthetic = false;
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &samples[..1],
            q,
            measured_only,
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::SyntheticEvidenceDisabled,
    );
}

#[test]
fn history_failures_missing_duration_and_insufficient_support_never_drop_rows() {
    let q = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 1,
    };
    for outcome in [
        SchedulerEmpiricalServiceOutcome::Failed,
        SchedulerEmpiricalServiceOutcome::Cancelled,
        SchedulerEmpiricalServiceOutcome::Incomplete,
    ] {
        let mut samples = [row("success", 1), row("unsuccessful", 999)];
        samples[1].outcome = outcome;
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &samples,
                q,
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt,
        );
    }
    let mut samples = [row("present", 1), row("missing", 999)];
    samples[1].duration_us = None;
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &samples,
            q,
            policy(),
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::MissingStage,
    );
    let mut need_three = policy();
    need_three.minimum_samples = 3;
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &[row("one", 1), row("two", 2)],
            q,
            need_three,
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::InsufficientSamples,
    );
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &[],
            q,
            policy(),
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::InsufficientSamples,
    );
}

#[test]
fn history_age_boundaries_and_future_refuse_with_checked_arithmetic() {
    let q = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 1,
    };
    require_estimate(estimate_scheduler_empirical_history_total_duration(
        context(),
        &[row("inclusive-ttl", 8)],
        q,
        policy(),
        Default::default(),
    ));
    for timestamp in [9, 13, u64::MAX] {
        let mut sample = row("bad-age", 8);
        sample.observed_at_ms = timestamp;
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &[sample],
                q,
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::StaleOrFutureSample,
        );
    }
    let mut maximum_clock = policy();
    maximum_clock.now_ms = u64::MAX;
    let mut sample = row("max-clock", u64::MAX);
    sample.observed_at_ms = u64::MAX - 2;
    assert_eq!(
        require_estimate(estimate_scheduler_empirical_history_total_duration(
            context(),
            &[sample],
            q,
            maximum_clock,
            Default::default(),
        ))
        .duration_us,
        u64::MAX
    );
}

#[test]
fn history_invalid_identity_policy_quantile_and_convention_are_refusals() {
    let q = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 1,
    };
    let overlong = "x".repeat(129);
    for id in ["", " ", "control\0id", overlong.as_str()] {
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &[row(id, 8)],
                q,
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::InvalidAttemptIdentity,
        );
    }
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            context(),
            &[row("same", 8), row("same", 9)],
            q,
            policy(),
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::DuplicateAttempt,
    );
    for field in 0..5 {
        let mut invalid = context();
        match field {
            0 => invalid.owner_epoch = &overlong,
            1 => invalid.identity_fingerprint = &overlong,
            2 => invalid.clock_epoch = &overlong,
            3 => invalid.residency_fingerprint = &overlong,
            4 => invalid.timing_convention = &overlong,
            _ => unreachable!(),
        }
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                invalid,
                &[row("valid-id", 8)],
                q,
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::InvalidContext,
        );
    }
    let mut unsupported = context();
    unsupported.timing_convention = SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION;
    require_reason(
        estimate_scheduler_empirical_history_total_duration(
            unsupported,
            &[row("valid-id", 8)],
            q,
            policy(),
            Default::default(),
        ),
        SchedulerEmpiricalServiceIncomplete::UnsupportedConvention,
    );
    for (numerator, denominator) in [(0, 1), (1, 0), (2, 1)] {
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &[row("valid-id", 8)],
                SchedulerEmpiricalQuantile {
                    numerator,
                    denominator,
                },
                policy(),
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::InvalidQuantile,
        );
    }
    for invalid_policy in [
        SchedulerCompletionRankingPolicy {
            minimum_samples: 0,
            ..policy()
        },
        SchedulerCompletionRankingPolicy {
            max_sample_age_ms: 0,
            ..policy()
        },
    ] {
        require_reason(
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &[row("valid-id", 8)],
                q,
                invalid_policy,
                Default::default(),
            ),
            SchedulerEmpiricalServiceIncomplete::InvalidPolicy,
        );
    }
}

#[test]
fn history_hard_limits_and_exact_budget_boundary_are_bounded() {
    let ids: Vec<_> = (0..129).map(|i| format!("bounded-{i}")).collect();
    let samples: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| row(id, 129 - u64::try_from(i).unwrap()))
        .collect();
    let q = SchedulerEmpiricalQuantile {
        numerator: 1,
        denominator: 2,
    };
    for (rows, budget) in [
        (&samples[..], SchedulerEmpiricalServiceBudget::default()),
        (
            &samples[..1],
            SchedulerEmpiricalServiceBudget {
                max_work_units: SCHEDULER_EMPIRICAL_MAX_WORK + 1,
            },
        ),
    ] {
        let SchedulerEmpiricalHistoryServiceResult::Incomplete { reason, work } =
            estimate_scheduler_empirical_history_total_duration(
                context(),
                rows,
                q,
                policy(),
                budget,
            )
        else {
            panic!("hard bound must refuse")
        };
        assert_eq!(
            reason,
            SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded
        );
        assert_eq!(work, SchedulerEmpiricalServiceWork::default());
    }
    let full = require_estimate(estimate_scheduler_empirical_history_total_duration(
        context(),
        &samples[..128],
        q,
        policy(),
        Default::default(),
    ));
    assert_eq!(full.support_count, 128);
    assert_eq!(
        (full.rank, full.duration_us),
        reference(&(2..=129).collect::<Vec<_>>(), q)
    );
    assert!(full.work.work_units <= SCHEDULER_EMPIRICAL_MAX_WORK);
    let exact = require_estimate(estimate_scheduler_empirical_history_total_duration(
        context(),
        &samples[..128],
        q,
        policy(),
        SchedulerEmpiricalServiceBudget {
            max_work_units: full.work.work_units,
        },
    ));
    assert_eq!(exact.work, full.work);
    for limit in [0, full.work.work_units - 1] {
        let SchedulerEmpiricalHistoryServiceResult::Incomplete { reason, work } =
            estimate_scheduler_empirical_history_total_duration(
                context(),
                &samples[..128],
                q,
                policy(),
                SchedulerEmpiricalServiceBudget {
                    max_work_units: limit,
                },
            )
        else {
            panic!("short budget must refuse")
        };
        assert_eq!(reason, SchedulerEmpiricalServiceIncomplete::BudgetExhausted);
        assert!(work.work_units <= limit);
    }
}

#[test]
fn history_and_legacy_totals_share_meter_without_identity_substitution() {
    let old_context = SchedulerCompletionContext {
        host_id: "separate-legacy-fixture-owner",
        runtime_instance_id: "one-exact-legacy-fixture-runtime",
        artifact_fingerprint: KEY,
        workload_fingerprint: KEY,
        resource_condition_fingerprint: KEY,
        residency_fingerprint: "reloaded",
        timing_convention: SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION,
    };
    let new_rows = [row("a", 25), row("b", 7), row("c", 31)];
    let legacy_rows: Vec<_> = new_rows
        .iter()
        .map(|sample| SchedulerEmpiricalServiceTotalObservation {
            attempt_id: sample.attempt_id,
            context: old_context,
            source: sample.source,
            observed_at_ms: sample.observed_at_ms,
            outcome: sample.outcome,
            duration_us: sample.duration_us,
        })
        .collect();
    let q = SchedulerEmpiricalQuantile {
        numerator: 2,
        denominator: 3,
    };
    let history = require_estimate(estimate_scheduler_empirical_history_total_duration(
        context(),
        &new_rows,
        q,
        policy(),
        Default::default(),
    ));
    let SchedulerEmpiricalServiceResult::Estimated(legacy) =
        estimate_scheduler_empirical_service_total_duration(
            old_context,
            &legacy_rows,
            q,
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy(),
            Default::default(),
        )
    else {
        panic!("legacy fixture should estimate")
    };
    assert_eq!(
        (history.duration_us, history.rank, history.support_count),
        (legacy.duration_us, legacy.rank, legacy.support_count)
    );
    assert_eq!(history.work, legacy.work);
    assert_eq!(
        legacy.context.runtime_instance_id,
        "one-exact-legacy-fixture-runtime"
    );
    // Exhaustive history estimate has no hidden runtime_instance_id or Running field.
    let SchedulerEmpiricalHistoryServiceEstimate {
        context,
        source,
        quantile,
        duration_us,
        total_observations,
        support_count,
        rank,
        work,
    } = history;
    assert_eq!(context.clock_epoch, "oracle-clock");
    assert_eq!(
        (
            source,
            quantile,
            duration_us,
            total_observations,
            support_count,
            rank,
            work
        ),
        (
            legacy.source,
            legacy.quantile,
            legacy.duration_us,
            legacy.total_observations,
            legacy.support_count,
            legacy.rank,
            legacy.work
        )
    );
}

#[test]
fn history_preserves_the_existing_exhaustive_incomplete_reason_contract() {
    fn exhaustive(reason: SchedulerEmpiricalServiceIncomplete) -> bool {
        match reason {
            SchedulerEmpiricalServiceIncomplete::WorkLimitExceeded
            | SchedulerEmpiricalServiceIncomplete::BudgetExhausted
            | SchedulerEmpiricalServiceIncomplete::InvalidPolicy
            | SchedulerEmpiricalServiceIncomplete::InvalidQuantile
            | SchedulerEmpiricalServiceIncomplete::InvalidContext
            | SchedulerEmpiricalServiceIncomplete::UnsupportedConvention
            | SchedulerEmpiricalServiceIncomplete::InvalidAttemptIdentity
            | SchedulerEmpiricalServiceIncomplete::DuplicateAttempt
            | SchedulerEmpiricalServiceIncomplete::IncomparableEvidence
            | SchedulerEmpiricalServiceIncomplete::SyntheticEvidenceDisabled
            | SchedulerEmpiricalServiceIncomplete::UnsuccessfulAttempt
            | SchedulerEmpiricalServiceIncomplete::StaleOrFutureSample
            | SchedulerEmpiricalServiceIncomplete::MissingStage
            | SchedulerEmpiricalServiceIncomplete::DurationOverflow
            | SchedulerEmpiricalServiceIncomplete::InsufficientSamples
            | SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors => true,
        }
    }
    assert!(exhaustive(
        SchedulerEmpiricalServiceIncomplete::InsufficientSurvivors
    ));
}
