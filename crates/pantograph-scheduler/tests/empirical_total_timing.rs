use pantograph_scheduler::*;

fn context() -> SchedulerCompletionContext<'static> {
    SchedulerCompletionContext {
        host_id: "fixture-owner",
        runtime_instance_id: "fixture-generation",
        artifact_fingerprint: "artifact",
        workload_fingerprint: "workload",
        resource_condition_fingerprint: "resource",
        residency_fingerprint: "residency",
        timing_convention: SCHEDULER_EMPIRICAL_SERVICE_TOTAL_CONVENTION,
    }
}
fn policy() -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 100,
        max_sample_age_ms: 1,
        minimum_samples: 2,
        allow_synthetic: true,
    }
}
fn rows() -> Vec<SchedulerEmpiricalServiceTotalObservation<'static>> {
    ["a", "b", "c", "d", "e", "f"]
        .into_iter()
        .zip([1, 2, 3, 10, 20, 30])
        .map(
            |(attempt_id, duration_us)| SchedulerEmpiricalServiceTotalObservation {
                attempt_id,
                context: context(),
                source: SchedulerCompletionEvidenceSource::Synthetic,
                observed_at_ms: 100,
                outcome: SchedulerEmpiricalServiceOutcome::Completed,
                duration_us: Some(duration_us),
            },
        )
        .collect()
}
#[test]
fn whole_total_kernel_preserves_strict_survival_and_distinct_convention() {
    let rows = rows();
    let estimate = estimate_scheduler_empirical_service_total_duration(
        context(),
        &rows,
        SchedulerEmpiricalQuantile {
            numerator: 1,
            denominator: 2,
        },
        SchedulerEmpiricalServiceCondition::Running { elapsed_us: 3 },
        policy(),
        Default::default(),
    );
    let SchedulerEmpiricalServiceResult::Estimated(value) = estimate else {
        panic!("expected synthetic estimate")
    };
    assert_eq!(
        (value.duration_us, value.support_count, value.rank),
        (17, 3, 2)
    );
    assert_eq!(value.source, SchedulerCompletionEvidenceSource::Synthetic);
    let mut wrong = context();
    wrong.timing_convention = SCHEDULER_EMPIRICAL_SERVICE_CONVENTION;
    assert!(matches!(
        estimate_scheduler_empirical_service_total_duration(
            wrong,
            &rows,
            SchedulerEmpiricalQuantile {
                numerator: 1,
                denominator: 2
            },
            SchedulerEmpiricalServiceCondition::NotStarted,
            policy(),
            Default::default()
        ),
        SchedulerEmpiricalServiceResult::Incomplete {
            reason: SchedulerEmpiricalServiceIncomplete::UnsupportedConvention,
            ..
        }
    ));
}

#[test]
fn missing_failed_and_budget_exhausted_totals_never_return_partial_estimates() {
    for case in ["missing", "failed", "budget"] {
        let mut rows = rows();
        let mut budget = SchedulerEmpiricalServiceBudget::default();
        match case {
            "missing" => rows[5].duration_us = None,
            "failed" => rows[5].outcome = SchedulerEmpiricalServiceOutcome::Failed,
            "budget" => budget.max_work_units = 1,
            _ => unreachable!(),
        }
        assert!(
            matches!(
                estimate_scheduler_empirical_service_total_duration(
                    context(),
                    &rows,
                    SchedulerEmpiricalQuantile {
                        numerator: 1,
                        denominator: 2
                    },
                    SchedulerEmpiricalServiceCondition::NotStarted,
                    policy(),
                    budget
                ),
                SchedulerEmpiricalServiceResult::Incomplete { .. }
            ),
            "{case}"
        );
    }
}
