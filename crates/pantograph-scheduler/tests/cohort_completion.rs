use std::collections::BTreeMap;

use pantograph_scheduler::*;

type Action = (usize, usize);
type Edge = (Vec<Action>, Action);

struct Fixture {
    widths: Vec<usize>,
    dependencies: Vec<Vec<usize>>,
    costs: BTreeMap<Edge, SchedulerCohortCosts>,
}
fn costs(value: u64) -> SchedulerCohortCosts {
    SchedulerCohortCosts {
        setup_us: Some(0),
        transfer_us: Some(0),
        execution_us: Some(value),
        cleanup_us: Some(0),
        retention_us: Some(0),
        reload_us: Some(0),
    }
}
fn numeric(c: SchedulerCohortCosts) -> u64 {
    [
        c.setup_us,
        c.transfer_us,
        c.execution_us,
        c.cleanup_us,
        c.retention_us,
        c.reload_us,
    ]
    .into_iter()
    .map(Option::unwrap)
    .sum()
}
impl Fixture {
    fn new(
        widths: Vec<usize>,
        dependencies: Vec<Vec<usize>>,
        cost: impl Fn(&[Action], Action) -> u64,
    ) -> Self {
        fn visit(f: &mut Fixture, path: Vec<Action>, cost: &impl Fn(&[Action], Action) -> u64) {
            for task in 0..f.widths.len() {
                if (path.is_empty() && task != 0)
                    || path.iter().any(|(t, _)| *t == task)
                    || f.dependencies[task]
                        .iter()
                        .any(|d| !path.iter().any(|(t, _)| t == d))
                {
                    continue;
                }
                for placement in 0..f.widths[task] {
                    let action = (task, placement);
                    f.costs
                        .insert((path.clone(), action), costs(cost(&path, action)));
                    let mut next = path.clone();
                    next.push(action);
                    visit(f, next, cost);
                }
            }
        }
        let mut result = Self {
            widths,
            dependencies,
            costs: BTreeMap::new(),
        };
        visit(&mut result, Vec::new(), &cost);
        result
    }
    fn chain(n: usize, cost: impl Fn(&[Action], Action) -> u64) -> Self {
        Self::new(
            (0..n).map(|i| if i == 0 { 2 } else { 1 }).collect(),
            (0..n)
                .map(|i| if i == 0 { vec![] } else { vec![i - 1] })
                .collect(),
            cost,
        )
    }
    fn run(&self) -> SchedulerCohortScore {
        ranked(evaluate(self, "", SchedulerCohortBudget::default(), false))
    }
}
fn request(index: usize, width: usize) -> ValidatedSchedulerDispatchSelectionRequest {
    let fixture = include_str!("fixtures/dispatch_selection_request_valid.json")
        .replace("task.001", &format!("task.{:03}", index + 1));
    let mut r: SchedulerDispatchSelectionRequest = serde_json::from_str(&fixture).unwrap();
    r.task_intent.constraints.requested_runtime_id = None;
    r.task_intent.constraints.requested_device_id = None;
    let mut candidate = r.candidates.remove(0);
    candidate.reservations.clear();
    candidate.selected_model_ref.selected_artifact_path = None;
    candidate.batching_group_id = None;
    candidate.candidate_source_diagnostics.clear();
    r.candidates = (0..width)
        .map(|i| {
            let mut c = candidate.clone();
            c.candidate_id = format!("candidate.{}", ["A", "B", "C"][i]).parse().unwrap();
            c
        })
        .collect();
    r.try_into().unwrap()
}
fn policy() -> SchedulerCompletionRankingPolicy {
    SchedulerCompletionRankingPolicy {
        now_ms: 1000,
        max_sample_age_ms: 100,
        minimum_samples: 3,
        allow_synthetic: true,
    }
}
fn task_id<'a>(t: &'a SchedulerCohortTask<'_>) -> &'a SchedulerTaskId {
    match t.request {
        SchedulerCohortTaskRequest::Admitted(r) => &r.as_ref().task_intent.task_id,
        SchedulerCohortTaskRequest::Forecast(r) => &r.as_ref().task_intent.task_id,
    }
}
fn label(path: &[Action]) -> String {
    format!(
        "state.root{}",
        path.iter()
            .map(|(t, p)| format!(".{t}{p}"))
            .collect::<String>()
    )
}
fn evaluate(
    f: &Fixture,
    fault: &str,
    budget: SchedulerCohortBudget,
    permute: bool,
) -> SchedulerCohortResult {
    let mut requests: Vec<_> = f
        .widths
        .iter()
        .enumerate()
        .map(|(i, n)| request(i, *n))
        .collect();
    if fault == "hard-constraint" {
        let mut r = requests[0].clone().into_inner();
        r.task_intent.constraints.requested_runtime_id =
            Some("unsupported-runtime".parse().unwrap());
        requests[0] = r.try_into().unwrap();
    }
    if permute {
        for r in &mut requests {
            let mut raw = r.clone().into_inner();
            raw.candidates.reverse();
            *r = raw.try_into().unwrap();
        }
    }
    let first = &requests[0];
    let forecasts: Vec<ValidatedSchedulerCompletionSuccessor> = requests
        .iter()
        .skip(1)
        .enumerate()
        .map(|(i, r)| {
            let mut candidates = r.as_ref().candidates.clone();
            for c in &mut candidates {
                c.resource_fit_assessment = None;
            }
            SchedulerCompletionSuccessorRequest {
                task_intent: r.as_ref().task_intent.clone(),
                snapshot_identity: format!("workload.{}", i + 1),
                candidates,
            }
            .try_into()
            .unwrap()
        })
        .collect();
    let mut placements: Vec<Vec<_>> = (0..f.widths.len())
        .map(|i| {
            let cs = if i == 0 {
                &first.as_ref().candidates
            } else {
                &forecasts[i - 1].as_ref().candidates
            };
            cs.iter()
                .map(|c| SchedulerCohortPlacement {
                    candidate: c,
                    runtime_instance_id: "instance.fixed",
                    artifact_fingerprint: "artifact.fixed",
                })
                .collect()
        })
        .collect();
    if fault == "omitted-placement" {
        placements[0].pop();
    }
    if permute {
        for p in &mut placements {
            p.reverse();
        }
    }
    let mut deps: Vec<Vec<SchedulerTaskId>> = f
        .dependencies
        .iter()
        .map(|ds| {
            ds.iter()
                .map(|i| requests[*i].as_ref().task_intent.task_id.clone())
                .collect()
        })
        .collect();
    if fault == "unknown-dependency" {
        deps[1].push("task.unrepresented".parse().unwrap());
    }
    if fault == "cycle" {
        deps[1] = vec![requests[2].as_ref().task_intent.task_id.clone()];
        deps[2] = vec![requests[1].as_ref().task_intent.task_id.clone()];
    }
    let mut tasks: Vec<_> = (0..f.widths.len())
        .map(|i| SchedulerCohortTask {
            request: if i == 0 {
                SchedulerCohortTaskRequest::Admitted(first)
            } else {
                SchedulerCohortTaskRequest::Forecast(&forecasts[i - 1])
            },
            placements: &placements[i],
            dependencies: &deps[i],
            workload_fingerprint: ["workload.0", "workload.1", "workload.2", "workload.3"][i],
        })
        .collect();
    if permute {
        tasks.reverse();
    }
    let action = |(t, p): Action| {
        let task = tasks
            .iter()
            .find(|x| task_id(x) == &requests[t].as_ref().task_intent.task_id)
            .unwrap();
        let placement = task
            .placements
            .iter()
            .find(|x| x.candidate.candidate_id.as_str() == format!("candidate.{}", ["A", "B"][p]))
            .unwrap_or(&task.placements[0]);
        SchedulerCohortAction { task, placement }
    };
    let cohort = SchedulerFrozenCohort {
        identity: "cohort.v1",
        objective_identity: "terminal-then-completion-sum.v1",
        dependency_snapshot_identity: "dependencies.v1",
        first,
        tasks: &tasks,
        host_id: "host.fixed",
        timing_convention: "serialized-six-stage-mean-us.v1",
        initial_condition_fingerprint: "state.root",
        initial_residency_fingerprint: "state.root",
        source: SchedulerCompletionEvidenceSource::Synthetic,
    };
    let histories: Vec<Vec<_>> = f
        .costs
        .keys()
        .map(|(h, _)| h.iter().copied().map(action).collect())
        .collect();
    let labels: Vec<_> = f
        .costs
        .keys()
        .map(|(h, a)| {
            let mut after = h.clone();
            after.push(*a);
            (label(h), label(&after))
        })
        .collect();
    let mut rows: Vec<_> = f
        .costs
        .iter()
        .enumerate()
        .map(|(i, ((_, a), costs))| {
            let a = action(*a);
            let context = SchedulerCompletionContext {
                host_id: cohort.host_id,
                runtime_instance_id: a.placement.runtime_instance_id,
                artifact_fingerprint: a.placement.artifact_fingerprint,
                workload_fingerprint: a.task.workload_fingerprint,
                resource_condition_fingerprint: &labels[i].0,
                residency_fingerprint: &labels[i].0,
                timing_convention: cohort.timing_convention,
            };
            SchedulerCohortEvidence {
                cohort: &cohort,
                history: &histories[i],
                action: a,
                context,
                resource_fit: SchedulerResourceFitState::Fits,
                sample: Some(SchedulerCohortSample {
                    context,
                    source: cohort.source,
                    sample_count: 3,
                    observed_at_ms: 900,
                    costs: *costs,
                }),
                transition: Some(SchedulerCohortTransition {
                    condition_fingerprint: &labels[i].1,
                    residency_fingerprint: &labels[i].1,
                }),
            }
        })
        .collect();
    let future = rows.iter().position(|r| !r.history.is_empty()).unwrap_or(0);
    match fault {
        "missing-row" => {
            rows.remove(0);
        }
        "missing-tail" => {
            let i = rows
                .iter()
                .position(|r| r.history.len() == f.widths.len() - 1)
                .unwrap();
            rows.remove(i);
        }
        "duplicate" => rows.push(rows[0].clone()),
        "oversized-evidence" => {
            while rows.len() <= SCHEDULER_COHORT_MAX_EVIDENCE {
                rows.push(rows[0].clone());
            }
        }
        "unknown" => rows[future].resource_fit = SchedulerResourceFitState::Unknown,
        "missing-stage" => rows[future].sample.as_mut().unwrap().costs.cleanup_us = None,
        "stale" => rows[future].sample.as_mut().unwrap().observed_at_ms = 899,
        "future" => rows[future].sample.as_mut().unwrap().observed_at_ms = 1001,
        "few-samples" => rows[future].sample.as_mut().unwrap().sample_count = 2,
        "sample-context" => {
            rows[future]
                .sample
                .as_mut()
                .unwrap()
                .context
                .residency_fingerprint = "foreign.residency"
        }
        "transition" => {
            rows[0].transition.as_mut().unwrap().condition_fingerprint = "stale.transition"
        }
        "residency" => {
            rows[0].transition.as_mut().unwrap().residency_fingerprint = "stale.residency"
        }
        "workload" => rows[future].context.workload_fingerprint = "foreign.workload",
        "source" => {
            rows[future].sample.as_mut().unwrap().source =
                SchedulerCompletionEvidenceSource::Measured
        }
        "dead-tail" | "dead-A-tail" => {
            for row in &mut rows {
                if row.history.len() == f.widths.len() - 1
                    && (fault == "dead-tail"
                        || row.history[0].placement.candidate.candidate_id.as_str()
                            == "candidate.A")
                {
                    row.resource_fit = SchedulerResourceFitState::WaitingForResources;
                    row.sample = None;
                    row.transition = None;
                }
            }
        }
        _ => {}
    }
    let foreign = request(0, f.widths[0]);
    let foreign_placement = SchedulerCohortPlacement {
        candidate: &foreign.as_ref().candidates[0],
        runtime_instance_id: "instance.fixed",
        artifact_fingerprint: "artifact.fixed",
    };
    if fault == "foreign-placement" {
        rows[0].action.placement = &foreign_placement;
    }
    if permute {
        rows.reverse();
    }
    let before = serde_json::to_value(first.as_ref()).unwrap();
    let result = evaluate_scheduler_cohort_completion(&cohort, &rows, policy(), budget);
    assert_eq!(
        serde_json::to_value(first.as_ref()).unwrap(),
        before,
        "kernel must not mutate admitted request"
    );
    result
}
fn ranked(result: SchedulerCohortResult) -> SchedulerCohortScore {
    match result {
        SchedulerCohortResult::Ranked(s) => s,
        other => panic!("expected complete score: {other:?}"),
    }
}
fn reason(result: SchedulerCohortResult) -> SchedulerCohortIncomplete {
    match result {
        SchedulerCohortResult::Incomplete { reason, .. } => reason,
        other => panic!("expected incomplete: {other:?}"),
    }
}
fn selected(s: &SchedulerCohortScore) -> &str {
    s.first_candidate_id.as_str()
}

#[test]
fn third_obligation_reverses_the_pair_from_a2_to_b5() {
    let f = Fixture::chain(3, |h, (t, p)| match t {
        0 => [1, 2][p],
        1 => [1, 2][h[0].1],
        _ => [100, 1][h[0].1],
    });
    let pair = Fixture::chain(
        2,
        |h, (t, p)| if t == 0 { [1, 2][p] } else { [1, 2][h[0].1] },
    )
    .run();
    assert_eq!(
        (selected(&pair), pair.terminal_completion_us),
        ("candidate.A", 2)
    );
    let score = f.run();
    assert_eq!(
        (
            selected(&score),
            score.terminal_completion_us,
            score.completion_sum_us
        ),
        ("candidate.B", 5, 11)
    );
    assert_eq!(
        score
            .completions
            .iter()
            .map(|c| c.optimized)
            .collect::<Vec<_>>(),
        [true, true, false]
    );
    assert_eq!(score.work.continuation_events, 2);
}
#[test]
fn fourth_obligation_reverses_a_shorter_three_task_path() {
    let f = Fixture::chain(4, |h, (t, p)| {
        if t == 0 {
            [1, 2][p]
        } else if t == 3 {
            [100, 1][h[0].1]
        } else {
            1
        }
    });
    let s = f.run();
    assert_eq!(
        (selected(&s), s.terminal_completion_us, s.completion_sum_us),
        ("candidate.B", 5, 14)
    );
    assert_eq!(s.completions.len(), 4);
    assert_eq!(s.work.continuation_events, 4);
}
#[test]
fn dependencies_unlock_only_after_their_projected_completions() {
    let f = Fixture::new(
        vec![2, 1, 1, 1],
        vec![vec![], vec![0], vec![0], vec![1, 2]],
        |_, (t, _)| [1, 5, 2, 1][t],
    );
    let s = f.run();
    assert_eq!(
        s.completions
            .iter()
            .map(|c| c.task_id.as_str())
            .collect::<Vec<_>>(),
        ["task.001", "task.003", "task.002", "task.004"]
    );
    assert_eq!((s.terminal_completion_us, s.completion_sum_us), (9, 21));
}
#[test]
fn continuation_is_earliest_completion_not_a_hidden_third_event_search() {
    let f = Fixture::new(
        vec![1, 1, 1, 1],
        vec![vec![], vec![0], vec![1], vec![1]],
        |h, (t, _)| {
            if t == 3 && h.last().is_some_and(|(t, _)| *t == 2) {
                100
            } else {
                [1, 1, 1, 2][t]
            }
        },
    );
    let s = f.run();
    assert_eq!(s.terminal_completion_us, 103);
    assert_eq!(s.completions[2].task_id.as_str(), "task.003");
    // Globally reordering the tail would finish at 5. That is outside this policy.
    assert!(s.terminal_completion_us > 5);
}
#[test]
fn cold_setup_can_win_but_reload_cleanup_transfer_and_retention_can_reverse_it() {
    let mut f = Fixture::chain(
        3,
        |h, (t, p)| if t == 0 { [1, 5][p] } else { [10, 1][h[0].1] },
    );
    assert_eq!(selected(&f.run()), "candidate.B");
    let b = f.costs.get_mut(&(vec![], (0, 1))).unwrap();
    b.setup_us = Some(4);
    b.execution_us = Some(1); // Same cold total, explicit stages.
    assert_eq!(selected(&f.run()), "candidate.B");
    for field in 0..4 {
        let mut altered = Fixture::chain(
            3,
            |h, (t, p)| if t == 0 { [1, 5][p] } else { [10, 1][h[0].1] },
        );
        for ((h, _), c) in &mut altered.costs {
            if h.len() == 2 && h[0].1 == 1 {
                match field {
                    0 => c.reload_us = Some(30),
                    1 => c.cleanup_us = Some(30),
                    2 => c.transfer_us = Some(30),
                    _ => c.retention_us = Some(30),
                }
            }
        }
        assert_eq!(
            selected(&altered.run()),
            "candidate.A",
            "stage {field} must affect terminal score"
        );
    }
}
#[test]
fn objective_uses_terminal_then_completion_sum_then_stable_ids() {
    let f = Fixture::chain(
        2,
        |h, (t, p)| if t == 0 { [3, 1][p] } else { [1, 3][h[0].1] },
    );
    let s = f.run();
    assert_eq!(
        (selected(&s), s.terminal_completion_us, s.completion_sum_us),
        ("candidate.B", 4, 5)
    );
    let tied = Fixture::new(vec![2, 2, 2, 2], vec![vec![]; 4], |_, _| 1);
    let s = tied.run();
    assert_eq!(selected(&s), "candidate.A");
    assert_eq!(
        s.completions
            .iter()
            .map(|c| c.task_id.as_str())
            .collect::<Vec<_>>(),
        ["task.001", "task.002", "task.003", "task.004"]
    );
}

// Independent tiny numeric reference. Enumerate every legal full schedule, then
// retain only schedules whose events after event two satisfy the fixed greedy
// continuation at their own prefix. This does not invoke production helpers or
// perform its nested two-event search and rollout implementation.
fn reference(f: &Fixture) -> (u64, u64, Vec<Action>) {
    fn enumerate(
        f: &Fixture,
        path: Vec<Action>,
        end: u64,
        sum: u64,
        out: &mut Vec<(u64, u64, Vec<Action>)>,
    ) {
        if path.len() == f.widths.len() {
            out.push((end, sum, path));
            return;
        }
        for ((history, action), c) in &f.costs {
            if history == &path {
                let mut next = path.clone();
                next.push(*action);
                let completion = end + numeric(*c);
                enumerate(f, next, completion, sum + completion, out);
            }
        }
    }
    let mut schedules = Vec::new();
    enumerate(f, vec![], 0, 0, &mut schedules);
    schedules
        .into_iter()
        .filter(|(_, _, path)| {
            (2..path.len()).all(|depth| {
                let best = f
                    .costs
                    .iter()
                    .filter(|((h, _), _)| h == &path[..depth])
                    .map(|((_, a), c)| (numeric(*c), *a))
                    .min()
                    .unwrap()
                    .1;
                path[depth] == best
            })
        })
        .min()
        .unwrap()
}
#[test]
fn bounded_algorithm_matches_independent_tiny_reference_enumeration() {
    for n in 1..=4 {
        for seed in 0..48_u64 {
            let deps = if seed % 3 == 0 {
                (0..n)
                    .map(|i| if i == 0 { vec![] } else { vec![i - 1] })
                    .collect()
            } else {
                vec![vec![]; n]
            };
            let f = Fixture::new(vec![2; n], deps, |h, (t, p)| {
                let prefix = h.iter().fold(seed + 11, |v, (t, p)| {
                    v.wrapping_mul(17).wrapping_add((t * 3 + p) as u64)
                });
                (prefix
                    .wrapping_mul(13)
                    .wrapping_add((t * 7 + p * 19) as u64)
                    % 31)
                    + 1
            });
            let expected = reference(&f);
            let actual = f.run();
            let path: Vec<_> = actual
                .completions
                .iter()
                .map(|x| {
                    let t = x
                        .task_id
                        .as_str()
                        .strip_prefix("task.")
                        .unwrap()
                        .parse::<usize>()
                        .unwrap()
                        - 1;
                    let p = usize::from(x.candidate_id.as_str() == "candidate.B");
                    (t, p)
                })
                .collect();
            assert_eq!(
                (
                    actual.terminal_completion_us,
                    actual.completion_sum_us,
                    path
                ),
                expected,
                "n={n} seed={seed}"
            );
        }
    }
}
#[test]
fn task_offer_and_evidence_permutations_preserve_score_path_and_work() {
    let f = Fixture::new(vec![2; 4], vec![vec![]; 4], |h, (t, p)| {
        1 + ((h.len() * 3 + t + p) % 7) as u64
    });
    assert_eq!(
        f.run(),
        ranked(evaluate(&f, "", SchedulerCohortBudget::default(), true))
    );
}
#[test]
fn missing_unknown_stale_or_foreign_evidence_never_drops_an_obligation() {
    let f = Fixture::chain(4, |_, _| 1);
    for (fault, expected) in [
        ("missing-row", SchedulerCohortIncomplete::MissingEvidence),
        ("missing-tail", SchedulerCohortIncomplete::MissingEvidence),
        ("duplicate", SchedulerCohortIncomplete::DuplicateEvidence),
        ("unknown", SchedulerCohortIncomplete::UnknownCapacity),
        ("missing-stage", SchedulerCohortIncomplete::MissingStage),
        ("stale", SchedulerCohortIncomplete::StaleOrFutureSample),
        ("future", SchedulerCohortIncomplete::StaleOrFutureSample),
        (
            "few-samples",
            SchedulerCohortIncomplete::InsufficientSamples,
        ),
        (
            "sample-context",
            SchedulerCohortIncomplete::IncomparableEvidence,
        ),
        ("transition", SchedulerCohortIncomplete::InvalidContext),
        ("residency", SchedulerCohortIncomplete::InvalidContext),
        ("workload", SchedulerCohortIncomplete::InvalidContext),
        ("source", SchedulerCohortIncomplete::IncomparableEvidence),
        (
            "foreign-placement",
            SchedulerCohortIncomplete::ForeignSnapshot,
        ),
        (
            "omitted-placement",
            SchedulerCohortIncomplete::InvalidCohort,
        ),
        (
            "unknown-dependency",
            SchedulerCohortIncomplete::InvalidDependencies,
        ),
        ("cycle", SchedulerCohortIncomplete::InvalidDependencies),
        (
            "hard-constraint",
            SchedulerCohortIncomplete::UnsupportedPlacement,
        ),
    ] {
        assert_eq!(
            reason(evaluate(&f, fault, SchedulerCohortBudget::default(), false)),
            expected,
            "{fault}"
        );
    }
}
#[test]
fn known_dead_end_rejects_that_path_without_omitting_the_tail() {
    let f = Fixture::chain(4, |_, _| 1);
    let score = ranked(evaluate(
        &f,
        "dead-A-tail",
        SchedulerCohortBudget::default(),
        false,
    ));
    assert_eq!(selected(&score), "candidate.B");
    assert_eq!((score.completions.len(), score.work.dead_ends), (4, 1));
    assert_eq!(
        reason(evaluate(
            &f,
            "dead-tail",
            SchedulerCohortBudget::default(),
            false
        )),
        SchedulerCohortIncomplete::NoCompletePlan
    );
}
#[test]
fn every_budget_includes_continuation_and_rejects_partial_search() {
    let f = Fixture::new(vec![2; 4], vec![vec![]; 4], |_, _| 1);
    let score = f.run();
    assert_eq!(score.work.evidence_rows_validated, 158);
    assert_eq!(
        (
            score.work.branch_expansions,
            score.work.completion_events,
            score.work.continuation_events,
            score.work.complete_plans
        ),
        (14, 38, 24, 12)
    );
    let exact = SchedulerCohortBudget {
        max_branch_expansions: score.work.branch_expansions,
        max_completion_events: score.work.completion_events,
        max_work_units: score.work.work_units,
    };
    assert_eq!(ranked(evaluate(&f, "", exact, false)), score);
    for budget in [
        SchedulerCohortBudget {
            max_branch_expansions: exact.max_branch_expansions - 1,
            ..exact
        },
        SchedulerCohortBudget {
            max_completion_events: exact.max_completion_events - 1,
            ..exact
        },
        SchedulerCohortBudget {
            max_work_units: exact.max_work_units - 1,
            ..exact
        },
        SchedulerCohortBudget {
            max_work_units: 0,
            ..exact
        },
    ] {
        assert_eq!(
            reason(evaluate(&f, "", budget, false)),
            SchedulerCohortIncomplete::BudgetExhausted
        );
    }
    assert_eq!(
        reason(evaluate(
            &f,
            "",
            SchedulerCohortBudget {
                max_completion_events: SCHEDULER_COHORT_MAX_EVENTS + 1,
                ..exact
            },
            false
        )),
        SchedulerCohortIncomplete::WorkLimitExceeded
    );
}
#[test]
fn stage_terminal_and_completion_sum_overflow_are_distinct_complete_failures() {
    let mut stage = Fixture::chain(2, |_, _| 1);
    let c = stage.costs.get_mut(&(vec![], (0, 0))).unwrap();
    c.execution_us = Some(u64::MAX);
    c.reload_us = Some(1);
    let terminal = Fixture::chain(2, |_, (t, _)| if t == 0 { u64::MAX } else { 1 });
    let sum = Fixture::chain(2, |_, (t, _)| if t == 0 { u64::MAX / 2 + 1 } else { 0 });
    for f in [stage, terminal, sum] {
        assert_eq!(
            reason(evaluate(&f, "", SchedulerCohortBudget::default(), false)),
            SchedulerCohortIncomplete::DurationOverflow
        );
    }
}

#[test]
fn two_task_scope_matches_existing_two_completion_selector() {
    for seed in 0..24 {
        let f = Fixture::new(vec![2, 2], vec![vec![], vec![0]], |h, (t, p)| {
            if t == 0 {
                1 + (seed + p as u64 * 7) % 13
            } else {
                1 + (seed * 3 + h[0].1 as u64 * 11 + p as u64 * 5) % 17
            }
        });
        let first = request(0, 2);
        let successor = request(1, 2);
        let projected = [successor.clone(), successor.clone()];
        let ctx = |next: bool, branch: usize| SchedulerCompletionContext {
            host_id: "host.fixed",
            runtime_instance_id: "instance.fixed",
            artifact_fingerprint: "artifact.fixed",
            workload_fingerprint: if next { "workload.1" } else { "workload.0" },
            resource_condition_fingerprint: if next {
                ["after.A", "after.B"][branch]
            } else {
                "initial"
            },
            residency_fingerprint: "qualified.residency",
            timing_convention: "serialized-mean-us.v1",
        };
        let first_rows: Vec<_> = first
            .as_ref()
            .candidates
            .iter()
            .enumerate()
            .map(|(p, c)| SchedulerCompletionEvidence {
                request: &first,
                candidate: c,
                current_context: ctx(false, 0),
                sample: SchedulerCompletionSample {
                    candidate: c,
                    context: ctx(false, 0),
                    source: SchedulerCompletionEvidenceSource::Synthetic,
                    sample_count: 3,
                    observed_at_ms: 900,
                    preparation_us: Some(0),
                    required_transfer_us: Some(0),
                    execution_us: Some(numeric(f.costs[&(vec![], (0, p))])),
                },
            })
            .collect();
        let next_rows: Vec<Vec<_>> = projected
            .iter()
            .enumerate()
            .map(|(branch, r)| {
                r.as_ref()
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(p, c)| SchedulerCompletionEvidence {
                        request: r,
                        candidate: c,
                        current_context: ctx(true, branch),
                        sample: SchedulerCompletionSample {
                            candidate: c,
                            context: ctx(true, branch),
                            source: SchedulerCompletionEvidenceSource::Synthetic,
                            sample_count: 3,
                            observed_at_ms: 900,
                            preparation_us: Some(0),
                            required_transfer_us: Some(0),
                            execution_us: Some(numeric(f.costs[&(vec![(0, branch)], (1, p))])),
                        },
                    })
                    .collect()
            })
            .collect();
        let branches: Vec<_> = (0..2)
            .map(|branch| SchedulerTwoCompletionContinuation {
                first_candidate: &first.as_ref().candidates[branch],
                first_context: ctx(false, 0),
                transition_fingerprint: ctx(true, branch).resource_condition_fingerprint,
                projected_request: &projected[branch],
                evidence: &next_rows[branch],
            })
            .collect();
        let old = select_scheduler_candidate_with_two_completions(
            &first,
            &first_rows,
            SchedulerTwoCompletionPrefix {
                identity: "pair",
                successor_universe: &successor,
            },
            &branches,
            policy(),
            SchedulerTwoCompletionBudget::default(),
        );
        let SchedulerTwoCompletionDiagnostic::Ranked(old_score) = old.diagnostic else {
            panic!("old pair did not rank");
        };
        let SchedulerDispatchReservationSelection::Selected { candidate_id, .. } = old.selection
        else {
            panic!("old pair did not select");
        };
        let new = f.run();
        assert_eq!(
            (
                new.first_candidate_id,
                new.terminal_completion_us,
                new.completion_sum_us,
                new.completions[1].candidate_id.clone()
            ),
            (
                candidate_id,
                old_score.terminal_completion_us,
                old_score.completion_sum_us,
                old_score.successor_candidate_id
            )
        );
    }
}

#[test]
fn identical_frozen_inputs_produce_deterministic_results() {
    let f = Fixture::chain(4, |h, (t, p)| 1 + (h.len() + t + p) as u64);
    let observation = f.run();
    for _ in 0..3 {
        assert_eq!(f.run(), observation);
    }
}

#[test]
fn oversized_cohorts_placements_and_evidence_refuse_before_search() {
    for (task_count, placement_count) in [(5, 1), (1, 3)] {
        let first = request(0, placement_count);
        let placements: Vec<_> = first
            .as_ref()
            .candidates
            .iter()
            .map(|candidate| SchedulerCohortPlacement {
                candidate,
                runtime_instance_id: "instance",
                artifact_fingerprint: "artifact",
            })
            .collect();
        let tasks: Vec<_> = (0..task_count)
            .map(|_| SchedulerCohortTask {
                request: SchedulerCohortTaskRequest::Admitted(&first),
                placements: &placements,
                dependencies: &[],
                workload_fingerprint: "workload",
            })
            .collect();
        let cohort = SchedulerFrozenCohort {
            identity: "cohort",
            objective_identity: "objective",
            dependency_snapshot_identity: "dependencies",
            first: &first,
            tasks: &tasks,
            host_id: "host",
            timing_convention: "us",
            initial_condition_fingerprint: "condition",
            initial_residency_fingerprint: "residency",
            source: SchedulerCompletionEvidenceSource::Synthetic,
        };
        assert_eq!(
            evaluate_scheduler_cohort_completion(
                &cohort,
                &[],
                policy(),
                SchedulerCohortBudget::default()
            ),
            SchedulerCohortResult::Incomplete {
                reason: SchedulerCohortIncomplete::WorkLimitExceeded,
                work: SchedulerCohortWork::default()
            }
        );
    }
    let f = Fixture::chain(2, |_, _| 1);
    assert_eq!(
        evaluate(
            &f,
            "oversized-evidence",
            SchedulerCohortBudget::default(),
            false
        ),
        SchedulerCohortResult::Incomplete {
            reason: SchedulerCohortIncomplete::WorkLimitExceeded,
            work: SchedulerCohortWork::default()
        }
    );
}
