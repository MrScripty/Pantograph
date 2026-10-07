use pantograph_scheduler::*;
static HUGE_CONTEXT: &str = include_str!("fixtures/dispatch_selection_request_valid.json");

fn offers(count: usize, successor: bool) -> ValidatedSchedulerDispatchSelectionRequest {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    if successor {
        // Preserve the validated proof, replacing only this fixture's task ID.
        value = serde_json::from_str(&value.to_string().replace("task.001", "task.002")).unwrap();
    }
    let mut request: SchedulerDispatchSelectionRequest = serde_json::from_value(value).unwrap();
    request.task_intent.constraints.requested_runtime_id = None;
    request.task_intent.constraints.requested_device_id = None;
    let mut candidate = request.candidates.remove(0);
    candidate.reservations.clear();
    candidate.selected_model_ref.selected_artifact_path = None;
    candidate.batching_group_id = None;
    candidate.candidate_source_diagnostics.clear();
    request.candidates = (0..count)
        .map(|index| {
            let mut c = candidate.clone();
            c.candidate_id = format!("candidate.{index:03}").parse().unwrap();
            c
        })
        .collect();
    request.try_into().unwrap()
}
fn context(successor: bool, branch: usize) -> SchedulerCompletionContext<'static> {
    SchedulerCompletionContext {
        host_id: "synthetic-host",
        runtime_instance_id: "synthetic-instance",
        artifact_fingerprint: "synthetic-artifact",
        workload_fingerprint: if successor {
            "workload.two"
        } else {
            "workload.one"
        },
        resource_condition_fingerprint: if successor {
            [
                "after.a-release-reconcile-retention",
                "after.b-release-reconcile-retention",
                "after.c-release-reconcile-retention",
                "after.d-release-reconcile-retention",
            ][branch]
        } else {
            "initial-idle"
        },
        residency_fingerprint: "explicit-synthetic-residency",
        timing_convention: "synthetic-serialized-mean-us-v1",
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
fn rows<'a>(
    request: &'a ValidatedSchedulerDispatchSelectionRequest,
    costs: &[u64],
    preparation: &[u64],
    successor: bool,
    branch: usize,
) -> Vec<SchedulerCompletionEvidence<'a>> {
    request
        .as_ref()
        .candidates
        .iter()
        .zip(costs.iter().zip(preparation))
        .filter(|(c, _)| {
            c.resource_fit_assessment.as_ref().unwrap().state == SchedulerResourceFitState::Fits
        })
        .map(|(candidate, (cost, prep))| {
            let ctx = context(successor, branch);
            SchedulerCompletionEvidence {
                request,
                candidate,
                current_context: ctx,
                sample: SchedulerCompletionSample {
                    candidate,
                    context: ctx,
                    source: SchedulerCompletionEvidenceSource::Synthetic,
                    sample_count: 3,
                    observed_at_ms: 900,
                    preparation_us: Some(*prep),
                    required_transfer_us: Some(0),
                    execution_us: Some(cost.checked_sub(*prep).unwrap()),
                },
            }
        })
        .collect()
}
struct Matrix {
    first: ValidatedSchedulerDispatchSelectionRequest,
    successor: ValidatedSchedulerDispatchSelectionRequest,
    projections: Vec<ValidatedSchedulerDispatchSelectionRequest>,
    x: Vec<u64>,
    y: Vec<Vec<u64>>,
    first_preparation: Vec<u64>,
    conditional_preparation: Vec<Vec<u64>>,
}
impl Matrix {
    fn new(x: Vec<u64>, y: Vec<Vec<u64>>) -> Self {
        let first = offers(x.len(), false);
        let successor = offers(y[0].len(), true);
        let projections = (0..x.len()).map(|_| successor.clone()).collect();
        let first_preparation = vec![0; x.len()];
        let conditional_preparation = y.iter().map(|row| vec![0; row.len()]).collect();
        Self {
            first,
            successor,
            projections,
            x,
            y,
            first_preparation,
            conditional_preparation,
        }
    }
    fn reversal() -> Self {
        let mut m = Self::new(vec![2, 5], vec![vec![20, 5], vec![20, 1]]);
        m.first_preparation = vec![0, 4];
        m.conditional_preparation = vec![vec![0, 4], vec![0, 0]];
        m
    }
    fn edit_projection(
        &mut self,
        index: usize,
        edit: impl FnOnce(&mut SchedulerDispatchSelectionRequest),
    ) {
        let mut raw = self.projections[index].clone().into_inner();
        edit(&mut raw);
        self.projections[index] = raw.try_into().unwrap();
    }
}
#[derive(Clone, Copy)]
enum Fault {
    None,
    MissingBranch,
    DuplicateBranch,
    ForeignFirst,
    MissingStage,
    Stale,
    Context,
    FutureWorkload,
    ForeignNext,
    MissingRow,
    SyntheticDisabled,
    HugeContext,
    Permuted,
    OversizedRows,
    OversizedBranches,
}
fn evaluate(
    m: &Matrix,
    fault: Fault,
    budget: SchedulerTwoCompletionBudget,
) -> SchedulerTwoCompletionResult {
    let mut first_rows = rows(&m.first, &m.x, &m.first_preparation, false, 0);
    if matches!(fault, Fault::Permuted) {
        first_rows.reverse();
    }
    let mut next_rows: Vec<_> = m
        .projections
        .iter()
        .zip(&m.y)
        .enumerate()
        .map(|(i, (r, costs))| rows(r, costs, &m.conditional_preparation[i], true, i))
        .collect();
    match fault {
        Fault::Permuted => {
            for r in &mut next_rows {
                r.reverse();
            }
        }
        Fault::OversizedRows => {
            while next_rows[0].len() < 5 {
                let duplicate = next_rows[0][0];
                next_rows[0].push(duplicate);
            }
        }
        Fault::HugeContext => {
            for r in &mut next_rows[1] {
                r.current_context.workload_fingerprint = HUGE_CONTEXT;
                r.sample.context = r.current_context;
            }
        }
        Fault::MissingStage => next_rows[1][1].sample.required_transfer_us = None,
        Fault::Stale => next_rows[1][1].sample.observed_at_ms = 899,
        Fault::Context => {
            next_rows[1][1].sample.context.residency_fingerprint = "unqualified-residency"
        }
        Fault::FutureWorkload => {
            for r in &mut next_rows[1] {
                r.current_context.workload_fingerprint = "changed-workload";
                r.sample.context = r.current_context;
            }
        }
        Fault::ForeignNext => next_rows[1][1].request = &m.successor,
        Fault::MissingRow => {
            next_rows[1].pop();
        }
        _ => {}
    }
    let foreign = m.first.clone();
    let mut branches: Vec<_> = m
        .projections
        .iter()
        .zip(&next_rows)
        .enumerate()
        .map(
            |(i, (projected_request, evidence))| SchedulerTwoCompletionContinuation {
                first_candidate: &m.first.as_ref().candidates[i],
                first_context: context(false, 0),
                transition_fingerprint: context(true, i).resource_condition_fingerprint,
                projected_request,
                evidence,
            },
        )
        .collect();
    match fault {
        Fault::Permuted => branches.reverse(),
        Fault::OversizedBranches => {
            while branches.len() < 5 {
                branches.push(SchedulerTwoCompletionContinuation {
                    first_candidate: branches[0].first_candidate,
                    first_context: branches[0].first_context,
                    transition_fingerprint: branches[0].transition_fingerprint,
                    projected_request: branches[0].projected_request,
                    evidence: branches[0].evidence,
                });
            }
        }
        Fault::MissingBranch => {
            branches.pop();
        }
        Fault::DuplicateBranch => branches[1].first_candidate = branches[0].first_candidate,
        Fault::ForeignFirst => branches[1].first_candidate = &foreign.as_ref().candidates[1],
        _ => {}
    }
    let mut p = policy();
    if matches!(fault, Fault::SyntheticDisabled) {
        p.allow_synthetic = false;
    }
    select_scheduler_candidate_with_two_completions(
        &m.first,
        &first_rows,
        SchedulerTwoCompletionPrefix {
            identity: "declared-synthetic-two-task-prefix",
            successor_universe: &m.successor,
        },
        &branches,
        p,
        budget,
    )
}
fn selected(result: &SchedulerTwoCompletionResult) -> &str {
    let SchedulerDispatchReservationSelection::Selected { candidate_id, .. } = &result.selection
    else {
        panic!("{result:?}")
    };
    candidate_id.as_str()
}
fn score(result: &SchedulerTwoCompletionResult) -> &SchedulerTwoCompletionScore {
    let SchedulerTwoCompletionDiagnostic::Ranked(score) = &result.diagnostic else {
        panic!("{result:?}")
    };
    score
}
fn assert_fallback(
    m: &Matrix,
    result: SchedulerTwoCompletionResult,
    reason: SchedulerTwoCompletionFallback,
) {
    assert_eq!(
        result.diagnostic,
        SchedulerTwoCompletionDiagnostic::Fallback(reason)
    );
    assert_eq!(
        result.selection,
        select_scheduler_candidate_with_completion(
            &m.first,
            &rows(&m.first, &m.x, &m.first_preparation, false, 0),
            policy()
        )
        .selection
    );
}
fn fit(
    raw: &mut SchedulerDispatchSelectionRequest,
    index: usize,
    state: SchedulerResourceFitState,
) {
    let f = raw.candidates[index]
        .resource_fit_assessment
        .as_mut()
        .unwrap();
    f.state = state;
    f.diagnostics = vec![SchedulerResourceDiagnostic {
        severity: SchedulerResourceDiagnosticSeverity::Info,
        code: SchedulerResourceDiagnosticCode::ImpossibleFit,
        message: "Explicit conditional synthetic owner capacity".into(),
        hint: None,
    }];
}
#[test]
fn cold_first_choice_can_improve_second_completion_with_explicit_conditional_cost() {
    let m = Matrix::reversal();
    let one = select_scheduler_candidate_with_completion(
        &m.first,
        &rows(&m.first, &m.x, &m.first_preparation, false, 0),
        policy(),
    );
    assert!(
        matches!(one.selection, SchedulerDispatchReservationSelection::Selected { ref candidate_id, .. } if candidate_id.as_str() == "candidate.000")
    );
    let result = evaluate(&m, Fault::None, Default::default());
    assert_eq!(selected(&result), "candidate.001");
    assert_eq!(
        (
            score(&result).first_completion_us,
            score(&result).terminal_completion_us,
            score(&result).completion_sum_us
        ),
        (5, 6, 11)
    );
    assert_eq!(
        (
            score(&result).plans_evaluated,
            score(&result).completion_events_evaluated
        ),
        (4, 8)
    );
}
#[test]
fn declared_terminal_objective_precedes_completion_sum_and_ties_are_stable() {
    let m = Matrix::new(vec![1, 5], vec![vec![7, 7], vec![1, 1]]);
    // a: terminal8, sum9; b: terminal6, sum11. Terminal is primary.
    assert_eq!(
        selected(&evaluate(&m, Fault::None, Default::default())),
        "candidate.001"
    );
    let m = Matrix::new(vec![2, 2], vec![vec![3, 3], vec![3, 3]]);
    let result = evaluate(&m, Fault::None, Default::default());
    assert_eq!(selected(&result), "candidate.000");
    assert_eq!(
        score(&result).successor_candidate_id.as_str(),
        "candidate.000"
    );
}
#[test]
fn incomplete_foreign_and_stale_branches_preserve_exact_one_decision_fallback() {
    use SchedulerTwoCompletionFallback as R;
    let m = Matrix::reversal();
    for (fault, reason) in [
        (Fault::MissingBranch, R::MissingOrForeignBranch),
        (Fault::DuplicateBranch, R::MissingOrForeignBranch),
        (Fault::ForeignFirst, R::MissingOrForeignBranch),
        (Fault::MissingStage, R::ConditionalEvidenceIncomplete),
        (Fault::HugeContext, R::ConditionalEvidenceIncomplete),
        (Fault::Stale, R::ConditionalEvidenceIncomplete),
        (Fault::Context, R::ConditionalEvidenceIncomplete),
        (Fault::FutureWorkload, R::ConditionalEvidenceIncomplete),
        (Fault::ForeignNext, R::ConditionalEvidenceIncomplete),
        (Fault::MissingRow, R::ConditionalEvidenceIncomplete),
    ] {
        assert_fallback(&m, evaluate(&m, fault, Default::default()), reason);
    }
    assert_eq!(
        evaluate(&m, Fault::SyntheticDisabled, Default::default()).diagnostic,
        SchedulerTwoCompletionDiagnostic::Fallback(R::FirstEvidenceIncomplete)
    );
}
#[test]
fn conditional_capacity_can_be_gained_lost_or_unknown_but_never_inferred() {
    let mut m = Matrix::reversal();
    m.edit_projection(1, |raw| {
        fit(raw, 1, SchedulerResourceFitState::WaitingForResources)
    });
    assert_eq!(
        selected(&evaluate(&m, Fault::None, Default::default())),
        "candidate.000"
    );
    m.edit_projection(1, |raw| fit(raw, 1, SchedulerResourceFitState::Unknown));
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::UnknownProjectedCapacity,
    );
    m.edit_projection(1, |raw| fit(raw, 1, SchedulerResourceFitState::Fits));
    let mut base = m.successor.clone().into_inner();
    fit(&mut base, 1, SchedulerResourceFitState::WaitingForResources);
    m.successor = base.try_into().unwrap();
    // Explicit post-release Fits can admit a currently-unavailable successor.
    assert_eq!(
        selected(&evaluate(&m, Fault::None, Default::default())),
        "candidate.001"
    );
}
#[test]
fn changed_universe_and_no_complete_plan_are_explicit() {
    let mut m = Matrix::reversal();
    m.edit_projection(1, |raw| {
        raw.candidates[1].selected_runtime_id = "different-runtime".parse().unwrap();
    });
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::ChangedSuccessorUniverse,
    );
    let mut m = Matrix::reversal();
    for i in 0..2 {
        m.edit_projection(i, |raw| {
            for j in 0..2 {
                fit(raw, j, SchedulerResourceFitState::ImpossibleFit);
            }
        });
    }
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::NoCompletePlan,
    );
}
#[test]
fn completion_sum_overflow_and_exact_deterministic_budget_do_not_choose_partial_winner() {
    let m = Matrix::new(vec![u64::MAX / 2 + 1], vec![vec![0]]);
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::DurationOverflow,
    );
    let m = Matrix::new(vec![u64::MAX], vec![vec![1]]);
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::DurationOverflow,
    );
    let m = Matrix::reversal();
    for budget in [
        SchedulerTwoCompletionBudget {
            max_plans: 3,
            max_events: 8,
        },
        SchedulerTwoCompletionBudget {
            max_plans: 4,
            max_events: 7,
        },
    ] {
        assert_fallback(
            &m,
            evaluate(&m, Fault::None, budget),
            SchedulerTwoCompletionFallback::BudgetExhausted,
        );
    }
    assert_eq!(
        score(&evaluate(
            &m,
            Fault::None,
            SchedulerTwoCompletionBudget {
                max_plans: 4,
                max_events: 8
            }
        ))
        .plans_evaluated,
        4
    );
}
#[test]
fn independent_exhaustive_oracle_matches_many_small_matrices_and_fixed_observations() {
    for count in 1..=4 {
        for seed in 0..50usize {
            let x: Vec<_> = (0..count)
                .map(|i| ((seed * 17 + i * 7) % 23) as u64)
                .collect();
            let y: Vec<Vec<_>> = (0..count)
                .map(|i| {
                    (0..count)
                        .map(|j| ((seed * 11 + i * 3 + j * 19) % 29) as u64)
                        .collect()
                })
                .collect();
            // Independent test oracle enumerates concrete schedule finish vectors.
            let mut schedules = Vec::new();
            for (i, &first) in x.iter().enumerate() {
                for (j, &next_cost) in y[i].iter().enumerate() {
                    let finishes = [first, first + next_cost];
                    schedules.push((finishes[1], finishes.iter().sum::<u64>(), i, j));
                }
            }
            schedules.sort_unstable();
            let expected = schedules[0];
            let m = Matrix::new(x, y);
            let result = evaluate(&m, Fault::None, Default::default());
            let s = score(&result);
            assert_eq!(
                (s.terminal_completion_us, s.completion_sum_us),
                (expected.0, expected.1)
            );
            assert_eq!(selected(&result), format!("candidate.{:03}", expected.2));
            assert_eq!(
                s.successor_candidate_id.as_str(),
                format!("candidate.{:03}", expected.3)
            );
            // Hidden future outcome changes cannot affect this frozen input.
            assert_eq!(result, evaluate(&m, Fault::None, Default::default()));
        }
    }
}
#[test]
fn max_cohort_and_raw_padded_diagnostics_refuse_before_fallback() {
    let mut m = Matrix::reversal();
    let mut raw = m.first.clone().into_inner();
    raw.diagnostics.push(SchedulerDispatchSelectionDiagnostic {
        severity: SchedulerDispatchSelectionDiagnosticSeverity::Info,
        code: SchedulerDispatchSelectionDiagnosticCode::CandidateSelected,
        message: format!("{}fixture", " ".repeat(200_000)),
        candidate_id: None,
        hint: None,
    });
    m.first = raw.try_into().unwrap(); // Existing validator accepts trimmed text.
    assert_eq!(
        evaluate(&m, Fault::None, Default::default()).diagnostic,
        SchedulerTwoCompletionDiagnostic::Refused(
            SchedulerTwoCompletionFallback::WorkLimitExceeded
        )
    );
    let one = select_scheduler_candidate_with_completion(&m.first, &[], policy());
    assert_eq!(
        one.diagnostic,
        SchedulerCompletionRankingDiagnostic::Refused(
            SchedulerCompletionRefusalReason::WorkLimitExceeded
        )
    );
}
#[test]
fn successor_dependency_snapshot_is_frozen_and_offer_order_cannot_break_ties() {
    let mut m = Matrix::reversal();
    m.edit_projection(1, |raw| {
        raw.readiness_proof.execution_context.correlation_id =
            "changed-correlation".parse().unwrap();
    });
    assert_fallback(
        &m,
        evaluate(&m, Fault::None, Default::default()),
        SchedulerTwoCompletionFallback::ChangedSuccessorUniverse,
    );
    let mut m = Matrix::new(vec![2, 2], vec![vec![3, 3], vec![3, 3]]);
    let original = evaluate(&m, Fault::None, Default::default());
    assert_eq!(original, evaluate(&m, Fault::Permuted, Default::default()));
    let mut first = m.first.clone().into_inner();
    first.candidates.reverse();
    m.first = first.try_into().unwrap();
    for i in 0..2 {
        m.edit_projection(i, |raw| raw.candidates.reverse());
    }
    assert_eq!(original, evaluate(&m, Fault::None, Default::default()));
    let oversized = offers(5, false);
    assert_eq!(
        select_scheduler_candidate_with_two_completions(
            &oversized,
            &[],
            SchedulerTwoCompletionPrefix {
                identity: "oversized-prefix",
                successor_universe: &m.successor
            },
            &[],
            policy(),
            Default::default()
        )
        .diagnostic,
        SchedulerTwoCompletionDiagnostic::Refused(
            SchedulerTwoCompletionFallback::WorkLimitExceeded
        )
    );
}

#[test]
fn oversized_evidence_branches_and_budget_refuse_the_entire_comparison() {
    let m = Matrix::reversal();
    for (fault, budget) in [
        (
            Fault::OversizedRows,
            SchedulerTwoCompletionBudget::default(),
        ),
        (
            Fault::OversizedBranches,
            SchedulerTwoCompletionBudget::default(),
        ),
        (
            Fault::None,
            SchedulerTwoCompletionBudget {
                max_plans: 17,
                max_events: 32,
            },
        ),
        (
            Fault::None,
            SchedulerTwoCompletionBudget {
                max_plans: 16,
                max_events: 33,
            },
        ),
    ] {
        assert_eq!(
            evaluate(&m, fault, budget).diagnostic,
            SchedulerTwoCompletionDiagnostic::Refused(
                SchedulerTwoCompletionFallback::WorkLimitExceeded
            )
        );
    }
}

#[test]
#[ignore = "controlled cost probe; run separately without concurrent compilation"]
fn two_completion_cost_probe() {
    for count in [1, 2, 4] {
        let m = Matrix::new(vec![3; count], vec![vec![5; count]; count]);
        let first_rows = rows(&m.first, &m.x, &m.first_preparation, false, 0);
        let next_rows: Vec<_> = m
            .projections
            .iter()
            .zip(&m.y)
            .enumerate()
            .map(|(i, (r, costs))| rows(r, costs, &m.conditional_preparation[i], true, i))
            .collect();
        let branches: Vec<_> = m
            .projections
            .iter()
            .zip(&next_rows)
            .enumerate()
            .map(
                |(i, (projected_request, evidence))| SchedulerTwoCompletionContinuation {
                    first_candidate: &m.first.as_ref().candidates[i],
                    first_context: context(false, 0),
                    transition_fingerprint: context(true, i).resource_condition_fingerprint,
                    projected_request,
                    evidence,
                },
            )
            .collect();
        let mut elapsed = Vec::new();
        for _ in 0..2000 {
            let start = std::time::Instant::now();
            std::hint::black_box(select_scheduler_candidate_with_two_completions(
                &m.first,
                &first_rows,
                SchedulerTwoCompletionPrefix {
                    identity: "cost-synthetic-two-task-prefix",
                    successor_universe: &m.successor,
                },
                &branches,
                policy(),
                Default::default(),
            ));
            elapsed.push(start.elapsed().as_nanos());
        }
        elapsed.sort_unstable();
        println!(
            "offers={count} samples=2000 median_ns={} p95_ns={} max_ns={}",
            elapsed[1000], elapsed[1900], elapsed[1999]
        );
    }
}
