use super::*;

struct ObjectiveCase<'a> {
    groups: &'a [usize],
    weights: &'a [u32],
    ages: &'a [u64],
    profile: SchedulerCohortWorkflowProfile,
}
fn intent<'a>(t: &'a SchedulerCohortTask<'_>) -> &'a SchedulableTaskIntent {
    match t.request {
        SchedulerCohortTaskRequest::Admitted(r) => &r.as_ref().task_intent,
        SchedulerCohortTaskRequest::Forecast(r) => &r.as_ref().task_intent,
    }
}
fn run(
    f: &Fixture,
    case: &ObjectiveCase<'_>,
    fault: &str,
    evidence_fault: &str,
    budget: SchedulerCohortBudget,
    permute: bool,
) -> SchedulerCohortWorkflowResult {
    let convention = if fault == "old-convention" {
        "serialized-six-stage-mean-us.v1"
    } else {
        SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION
    };
    with_fixture(
        f,
        evidence_fault,
        permute,
        Some(case.groups),
        convention,
        |c, rows| {
            let mut members: Vec<Vec<_>> = (0..=case.groups.iter().copied().max().unwrap())
                .map(|group| {
                    c.tasks
                        .iter()
                        .filter(|t| {
                            let index = task_id(t)
                                .as_str()
                                .rsplit('.')
                                .next()
                                .unwrap()
                                .parse::<usize>()
                                .unwrap()
                                - 1;
                            case.groups[index] == group
                        })
                        .collect()
                })
                .collect();
            let foreign = SchedulerCohortTask {
                request: c.tasks[0].request,
                placements: c.tasks[0].placements,
                dependencies: c.tasks[0].dependencies,
                workload_fingerprint: c.tasks[0].workload_fingerprint,
            };
            match fault {
                "omit-task" => {
                    members[0].pop();
                }
                "duplicate-member" => {
                    members[0][1] = members[0][0];
                }
                "foreign-member" => {
                    members[0][0] = &foreign;
                }
                "split-workflow" => {
                    let all = members.remove(0);
                    members = vec![vec![all[0]], all[1..].to_vec()];
                }
                "oversized-members" => {
                    members[0].resize(5, &c.tasks[0]);
                }
                _ => {}
            }
            if permute {
                for tasks in &mut members {
                    tasks.reverse();
                }
            }
            let mut workflows: Vec<_> = members
                .iter()
                .enumerate()
                .map(|(i, tasks)| {
                    let representative = tasks.first().copied().unwrap_or(&c.tasks[0]);
                    SchedulerCohortWorkflowObligation {
                        workflow_id: &intent(representative).workflow_id,
                        workflow_run_id: &intent(representative).workflow_run_id,
                        completion_contract_identity: [
                            "contract.0",
                            "contract.1",
                            "contract.2",
                            "contract.3",
                        ][i],
                        required_tasks: tasks,
                        required_population_complete: true,
                        accepted_output_before_cleanup: true,
                        release: SchedulerCohortWorkflowRelease::Released {
                            age_us: case.ages.get(i).copied().unwrap_or(0),
                        },
                        weight: case.weights.get(i).copied().unwrap_or(1),
                    }
                })
                .collect();
            let foreign_run: SchedulerWorkflowRunId = "run.foreign".parse().unwrap();
            match fault {
                "wrong-run" => workflows[0].workflow_run_id = &foreign_run,
                "zero-weight" => workflows[0].weight = 0,
                "unknown-release" => workflows[0].release = SchedulerCohortWorkflowRelease::Unknown,
                "future-release" => workflows[0].release = SchedulerCohortWorkflowRelease::Future,
                "incomplete-population" => workflows[0].required_population_complete = false,
                "late-acceptance" => workflows[0].accepted_output_before_cleanup = false,
                _ => {}
            }
            if permute {
                workflows.reverse();
            }
            let objective = SchedulerCohortWorkflowObjective {
                cohort_identity: if fault == "foreign-cohort" {
                    "cohort.foreign"
                } else {
                    c.identity
                },
                objective_identity: if fault == "foreign-objective" {
                    "objective.foreign"
                } else {
                    c.objective_identity
                },
                profile: case.profile,
                workflows: &workflows,
            };
            evaluate_scheduler_cohort_workflow_objective(c, rows, &objective, policy(), budget)
        },
    )
}
fn good(f: &Fixture, case: &ObjectiveCase<'_>) -> SchedulerCohortWorkflowScore {
    match run(f, case, "", "", SchedulerCohortBudget::default(), false) {
        SchedulerCohortWorkflowResult::Ranked(s) => *s,
        other => panic!("controlled complete workflow fixture refused: {other:?}"),
    }
}
fn incomplete(
    r: SchedulerCohortWorkflowResult,
) -> (SchedulerCohortWorkflowIncomplete, SchedulerCohortWork) {
    match r {
        SchedulerCohortWorkflowResult::Incomplete { reason, work } => (reason, work),
        other => panic!("expected incomplete comparison: {other:?}"),
    }
}

#[test]
fn workflow_speed_objective_changes_first_placement_without_rewarding_node_count() {
    let f = Fixture::new(vec![2, 1, 1], vec![vec![]; 3], |h, (t, p)| {
        if t == 0 {
            [1, 5][p]
        } else if t == 1 {
            [10, 1][h[0].1]
        } else {
            [1, 10][h[0].1]
        }
    });
    let case = ObjectiveCase {
        groups: &[0, 1, 2],
        weights: &[1, 20, 1],
        ages: &[0; 3],
        profile: SchedulerCohortWorkflowProfile::Makespan,
    };
    let batch = good(&f, &case);
    let speed = good(
        &f,
        &ObjectiveCase {
            profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
            ..case
        },
    );
    assert_eq!(
        (batch.first_candidate_id.as_str(), batch.objective_loss_us),
        ("candidate.A", 12)
    );
    assert_eq!(
        (speed.first_candidate_id.as_str(), speed.objective_loss_us),
        ("candidate.B", 141)
    );
    assert_eq!(speed.terminal_completion_us, 16);
    assert_eq!(speed.workflow_completions.len(), 3);
    assert_eq!(speed.source, SchedulerCompletionEvidenceSource::Synthetic);
}

#[test]
fn grouping_required_outputs_avoids_task_completion_sum_as_workflow_loss() {
    let f = Fixture::new(vec![1; 4], vec![vec![]; 4], |_, (t, _)| [1, 1, 10, 4][t]);
    let legacy = f.run();
    assert_eq!(legacy.completions[1].task_id.as_str(), "task.002");
    for profile in [
        SchedulerCohortWorkflowProfile::Makespan,
        SchedulerCohortWorkflowProfile::WeightedFlowTime,
    ] {
        let score = good(
            &f,
            &ObjectiveCase {
                groups: &[0, 0, 0, 1],
                weights: &[1, 1],
                ages: &[0, 0],
                profile,
            },
        );
        assert_eq!(score.completions[1].task_id.as_str(), "task.004");
        assert_eq!(score.workflow_completion_sum_us, 21);
        assert_eq!(
            score
                .workflow_completions
                .iter()
                .map(|w| w.output_completion_us)
                .collect::<Vec<_>>(),
            [16, 5]
        );
    }
}

#[test]
fn final_cleanup_is_not_output_loss_but_still_blocks_subsequent_starts() {
    let mut f = Fixture::new(vec![2], vec![vec![]], |_, (_, p)| [2, 10][p]);
    f.costs.get_mut(&(vec![], (0, 0))).unwrap().cleanup_us = Some(100);
    assert_eq!(selected(&f.run()), "candidate.B");
    let score = good(
        &f,
        &ObjectiveCase {
            groups: &[0],
            weights: &[1],
            ages: &[0],
            profile: SchedulerCohortWorkflowProfile::Makespan,
        },
    );
    assert_eq!(
        (
            score.first_candidate_id.as_str(),
            score.objective_loss_us,
            score.terminal_completion_us
        ),
        ("candidate.A", 2, 102)
    );
    let mut serial = Fixture::chain(2, |_, _| 2);
    for ((h, _), c) in &mut serial.costs {
        if h.is_empty() {
            c.cleanup_us = Some(7);
            c.retention_us = Some(3);
        }
    }
    let score = good(
        &serial,
        &ObjectiveCase {
            groups: &[0, 0],
            weights: &[1],
            ages: &[0],
            profile: SchedulerCohortWorkflowProfile::Makespan,
        },
    );
    assert_eq!(score.completions[0].completion_us, 12);
    assert_eq!(score.workflow_makespan_us, 14);
    assert_eq!(score.terminal_completion_us, 14);
}

#[test]
fn common_continuation_keeps_resource_end_ordering_after_event_two() {
    // Remove tail dependencies so both last tasks are legal after event two.
    let mut f = Fixture::new(
        vec![1; 4],
        vec![vec![], vec![0], vec![1], vec![1]],
        |_, (t, _)| if t == 2 { 1 } else { 2 },
    );
    for ((_, (t, _)), c) in &mut f.costs {
        if *t == 2 {
            c.cleanup_us = Some(100);
        }
    }
    let s = good(
        &f,
        &ObjectiveCase {
            groups: &[0; 4],
            weights: &[1],
            ages: &[0],
            profile: SchedulerCohortWorkflowProfile::Makespan,
        },
    );
    assert_eq!(s.completions[2].task_id.as_str(), "task.004");
    assert!(!s.completions[2].optimized);
    assert_eq!(s.terminal_completion_us, 107);
}

#[test]
fn workflow_contract_refuses_missing_foreign_or_split_population_and_unreleased_work() {
    let f = Fixture::new(vec![1; 3], vec![vec![]; 3], |_, _| 1);
    let case = ObjectiveCase {
        groups: &[0; 3],
        weights: &[1],
        ages: &[0],
        profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
    };
    for fault in [
        "omit-task",
        "duplicate-member",
        "foreign-member",
        "split-workflow",
        "wrong-run",
        "zero-weight",
        "unknown-release",
        "future-release",
        "incomplete-population",
        "late-acceptance",
        "foreign-cohort",
        "foreign-objective",
        "old-convention",
    ] {
        assert_eq!(
            incomplete(run(
                &f,
                &case,
                fault,
                "",
                SchedulerCohortBudget::default(),
                false
            ))
            .0,
            SchedulerCohortWorkflowIncomplete::InvalidObjective,
            "{fault}"
        );
    }
    let (reason, work) = incomplete(run(
        &f,
        &case,
        "oversized-members",
        "",
        SchedulerCohortBudget::default(),
        false,
    ));
    assert_eq!(
        reason,
        SchedulerCohortWorkflowIncomplete::Cohort(SchedulerCohortIncomplete::WorkLimitExceeded)
    );
    assert_eq!(work.work_units, 0);
}

#[test]
fn workflow_opt_in_is_required_for_cross_run_population_and_edges_cannot_cross_runs() {
    let f = Fixture::new(vec![1, 1], vec![vec![], vec![]], |_, _| 1);
    let legacy = with_fixture(
        &f,
        "",
        false,
        Some(&[0, 1]),
        SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION,
        |c, rows| {
            evaluate_scheduler_cohort_completion(
                c,
                rows,
                policy(),
                SchedulerCohortBudget::default(),
            )
        },
    );
    assert_eq!(reason(legacy), SchedulerCohortIncomplete::InvalidCohort);
    let case = ObjectiveCase {
        groups: &[0, 1],
        weights: &[1, 1],
        ages: &[0, 0],
        profile: SchedulerCohortWorkflowProfile::Makespan,
    };
    assert_eq!(good(&f, &case).workflow_completions.len(), 2);
    let chain = Fixture::chain(2, |_, _| 1);
    assert_eq!(
        incomplete(run(
            &chain,
            &case,
            "",
            "",
            SchedulerCohortBudget::default(),
            false
        ))
        .0,
        SchedulerCohortWorkflowIncomplete::InvalidObjective
    );
}

#[test]
fn workflow_mode_preserves_evidence_refusal_and_never_returns_partial_budget_winner() {
    let f = Fixture::new(vec![2; 4], vec![vec![]; 4], |h, (t, p)| {
        1 + (h.len() + t + p) as u64
    });
    let case = ObjectiveCase {
        groups: &[0, 1, 0, 1],
        weights: &[2, 3],
        ages: &[5, 7],
        profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
    };
    for (fault, expected) in [
        ("missing-row", SchedulerCohortIncomplete::MissingEvidence),
        ("missing-tail", SchedulerCohortIncomplete::MissingEvidence),
        ("unknown", SchedulerCohortIncomplete::UnknownCapacity),
        ("missing-stage", SchedulerCohortIncomplete::MissingStage),
        ("stale", SchedulerCohortIncomplete::StaleOrFutureSample),
        ("source", SchedulerCohortIncomplete::IncomparableEvidence),
        ("dead-tail", SchedulerCohortIncomplete::NoCompletePlan),
    ] {
        assert_eq!(
            incomplete(run(
                &f,
                &case,
                "",
                fault,
                SchedulerCohortBudget::default(),
                false
            ))
            .0,
            SchedulerCohortWorkflowIncomplete::Cohort(expected),
            "{fault}"
        );
    }
    let work = good(&f, &case).work;
    for budget in [
        SchedulerCohortBudget {
            max_work_units: work.work_units - 1,
            ..SchedulerCohortBudget::default()
        },
        SchedulerCohortBudget {
            max_completion_events: work.completion_events - 1,
            ..SchedulerCohortBudget::default()
        },
        SchedulerCohortBudget {
            max_branch_expansions: work.branch_expansions - 1,
            ..SchedulerCohortBudget::default()
        },
    ] {
        assert_eq!(
            incomplete(run(&f, &case, "", "", budget, false)).0,
            SchedulerCohortWorkflowIncomplete::Cohort(SchedulerCohortIncomplete::BudgetExhausted)
        );
    }
    let exact = SchedulerCohortBudget {
        max_work_units: work.work_units,
        max_completion_events: work.completion_events,
        max_branch_expansions: work.branch_expansions,
    };
    assert!(matches!(
        run(&f, &case, "", "", exact, false),
        SchedulerCohortWorkflowResult::Ranked(_)
    ));
}

#[test]
fn workflow_arithmetic_is_wide_and_does_not_require_legacy_node_sum_to_fit() {
    let f = Fixture::chain(2, |_, (t, _)| if t == 0 { u64::MAX / 2 + 1 } else { 0 });
    assert_eq!(
        reason(evaluate(&f, "", SchedulerCohortBudget::default(), false)),
        SchedulerCohortIncomplete::DurationOverflow
    );
    let score = good(
        &f,
        &ObjectiveCase {
            groups: &[0, 0],
            weights: &[u32::MAX],
            ages: &[u64::MAX],
            profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
        },
    );
    let expected = (u128::from(u64::MAX) + u128::from(u64::MAX / 2 + 1)) * u128::from(u32::MAX);
    assert_eq!(score.objective_loss_us, expected);
    assert!(score.objective_loss_us > u128::from(u64::MAX));
    let overflow = Fixture::chain(2, |_, (t, _)| if t == 0 { u64::MAX } else { 1 });
    assert_eq!(
        incomplete(run(
            &overflow,
            &ObjectiveCase {
                groups: &[0, 0],
                weights: &[1],
                ages: &[0],
                profile: SchedulerCohortWorkflowProfile::Makespan
            },
            "",
            "",
            SchedulerCohortBudget::default(),
            false
        ))
        .0,
        SchedulerCohortWorkflowIncomplete::Cohort(SchedulerCohortIncomplete::DurationOverflow)
    );
}

fn reference_workflow(
    f: &Fixture,
    case: &ObjectiveCase<'_>,
) -> ((u128, u128, u128), Vec<Action>, Vec<u64>, u64) {
    fn enumerate(f: &Fixture, path: Vec<Action>, out: &mut Vec<Vec<Action>>) {
        if path.len() == f.widths.len() {
            out.push(path);
            return;
        }
        for (history, action) in f.costs.keys() {
            if history == &path {
                let mut next = path.clone();
                next.push(*action);
                enumerate(f, next, out);
            }
        }
    }
    let mut schedules = Vec::new();
    enumerate(f, Vec::new(), &mut schedules);
    schedules
        .into_iter()
        .filter(|path| {
            (2..path.len()).all(|depth| {
                let best = f
                    .costs
                    .iter()
                    .filter(|((h, _), _)| h == &path[..depth])
                    .map(|((_, action), c)| (numeric(*c), *action))
                    .min()
                    .unwrap()
                    .1;
                path[depth] == best
            })
        })
        .map(|path| {
            let mut outputs = vec![0; case.groups.iter().max().unwrap() + 1];
            let mut end = 0;
            for (depth, action) in path.iter().enumerate() {
                let c = f.costs[&(path[..depth].to_vec(), *action)];
                end += numeric(c);
                outputs[case.groups[action.0]] = outputs[case.groups[action.0]]
                    .max(end - c.cleanup_us.unwrap() - c.retention_us.unwrap());
            }
            let max = u128::from(*outputs.iter().max().unwrap());
            let sum: u128 = outputs.iter().map(|t| u128::from(*t)).sum();
            let flow: u128 = outputs
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    (u128::from(*t) + u128::from(case.ages.get(i).copied().unwrap_or(0)))
                        * u128::from(case.weights.get(i).copied().unwrap_or(1))
                })
                .sum();
            let key = match case.profile {
                SchedulerCohortWorkflowProfile::Makespan => (max, sum, 0),
                SchedulerCohortWorkflowProfile::WeightedFlowTime => (flow, max, sum),
            };
            (key, path, outputs, end)
        })
        .min_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)))
        .unwrap()
}

#[test]
fn workflow_profiles_match_independent_complete_schedule_oracle_and_permutations() {
    for n in 1..=4 {
        for seed in 0..24_u64 {
            let groups: Vec<_> = (0..n)
                .map(|t| if seed.is_multiple_of(3) { 0 } else { t % 2 })
                .collect();
            let deps = if seed.is_multiple_of(3) {
                (0..n)
                    .map(|t| if t == 0 { vec![] } else { vec![t - 1] })
                    .collect()
            } else {
                vec![vec![]; n]
            };
            let mut f = Fixture::new(vec![2; n], deps, |h, (t, p)| {
                1 + (seed * 13
                    + h.iter().map(|(t, p)| t * 7 + p * 11).sum::<usize>() as u64
                    + (t * 17 + p * 19) as u64)
                    % 23
            });
            for ((h, (t, p)), c) in &mut f.costs {
                c.setup_us = Some((*t % 3) as u64);
                c.transfer_us = Some((*p % 2) as u64);
                c.reload_us = Some((h.len() % 4) as u64);
                c.cleanup_us = Some(((*t + *p + h.len()) % 5) as u64);
                c.retention_us = Some(((*p + h.len()) % 3) as u64);
            }
            for profile in [
                SchedulerCohortWorkflowProfile::Makespan,
                SchedulerCohortWorkflowProfile::WeightedFlowTime,
            ] {
                let case = ObjectiveCase {
                    groups: &groups,
                    weights: &[1 + (seed % 3) as u32, 2 + (seed % 5) as u32],
                    ages: &[seed * 7, seed * 11],
                    profile,
                };
                let actual = good(&f, &case);
                let expected = reference_workflow(&f, &case);
                let actual_key = if profile == SchedulerCohortWorkflowProfile::Makespan {
                    (
                        actual.objective_loss_us,
                        actual.workflow_completion_sum_us,
                        0,
                    )
                } else {
                    (
                        actual.objective_loss_us,
                        u128::from(actual.workflow_makespan_us),
                        actual.workflow_completion_sum_us,
                    )
                };
                let trace: Vec<_> = actual
                    .completions
                    .iter()
                    .map(|c| {
                        (
                            c.task_id
                                .as_str()
                                .rsplit('.')
                                .next()
                                .unwrap()
                                .parse::<usize>()
                                .unwrap()
                                - 1,
                            usize::from(c.candidate_id.as_str() == "candidate.B"),
                        )
                    })
                    .collect();
                assert_eq!(
                    (
                        actual_key,
                        trace,
                        actual
                            .workflow_completions
                            .iter()
                            .map(|w| w.output_completion_us)
                            .collect::<Vec<_>>(),
                        actual.terminal_completion_us
                    ),
                    expected,
                    "n={n},seed={seed},profile={profile:?}"
                );
                assert_eq!(
                    run(&f, &case, "", "", SchedulerCohortBudget::default(), true),
                    SchedulerCohortWorkflowResult::Ranked(Box::new(actual))
                );
            }
        }
    }
}

#[test]
fn frozen_release_shifts_and_common_weight_scaling_preserve_choice() {
    let f = Fixture::new(vec![2; 4], vec![vec![]; 4], |h, (t, p)| {
        1 + (h.len() * 11 + t * 7 + p * 3) as u64
    });
    let case = ObjectiveCase {
        groups: &[0, 1, 0, 1],
        weights: &[2, 3],
        ages: &[7, 11],
        profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
    };
    let first = good(&f, &case);
    let shifted = good(
        &f,
        &ObjectiveCase {
            ages: &[1007, 2011],
            ..case
        },
    );
    let scaled = good(
        &f,
        &ObjectiveCase {
            weights: &[4, 6],
            ..case
        },
    );
    assert_eq!(first.completions, shifted.completions);
    assert_eq!(first.completions, scaled.completions);
    assert_eq!(
        shifted.objective_loss_us,
        first.objective_loss_us + 2 * 1000 + 3 * 2000
    );
    assert_eq!(scaled.objective_loss_us, first.objective_loss_us * 2);
}

#[test]
fn equivalent_required_output_grouping_has_equal_loss_when_a_node_is_split() {
    let original = Fixture::chain(2, |_, (t, _)| [2, 4][t]);
    let split = Fixture::chain(3, |_, (t, _)| [2, 1, 3][t]);
    for profile in [
        SchedulerCohortWorkflowProfile::Makespan,
        SchedulerCohortWorkflowProfile::WeightedFlowTime,
    ] {
        let a = good(
            &original,
            &ObjectiveCase {
                groups: &[0; 2],
                weights: &[3],
                ages: &[5],
                profile,
            },
        );
        let b = good(
            &split,
            &ObjectiveCase {
                groups: &[0; 3],
                weights: &[3],
                ages: &[5],
                profile,
            },
        );
        assert_eq!(a.objective_loss_us, b.objective_loss_us);
        assert_eq!(a.workflow_completion_sum_us, b.workflow_completion_sum_us);
        assert_eq!(a.workflow_completions.len(), 1);
        assert_eq!(b.workflow_completions.len(), 1);
        // No claim that arbitrary graph splitting preserves the two-event search.
    }
}

#[test]
fn legacy_public_score_and_work_counters_keep_the_previous_contract() {
    fn exhaustive_legacy_reason(reason: SchedulerCohortIncomplete) -> u8 {
        use SchedulerCohortIncomplete::*;
        match reason {
            WorkLimitExceeded | InvalidPolicy | InvalidCohort | UnsupportedPlacement
            | InvalidDependencies | ForeignSnapshot | DuplicateEvidence | MissingEvidence
            | InvalidHistory | InvalidContext | StaleOrFutureSample | InsufficientSamples
            | IncomparableEvidence | UnknownCapacity | MissingStage | DurationOverflow
            | NoCompletePlan | BudgetExhausted => 1,
        }
    }
    assert_eq!(
        exhaustive_legacy_reason(SchedulerCohortIncomplete::MissingEvidence),
        1
    );
    let one = Fixture::new(vec![1], vec![vec![]], |_, _| 5).run();
    assert_eq!((one.terminal_completion_us, one.completion_sum_us), (5, 5));
    assert_eq!(
        one.work,
        SchedulerCohortWork {
            work_units: 14,
            evidence_rows_validated: 1,
            candidate_evaluations: 1,
            branch_expansions: 1,
            completion_events: 1,
            continuation_events: 0,
            complete_plans: 1,
            dead_ends: 0,
        }
    );
    let two = Fixture::new(vec![1, 1], vec![vec![], vec![0]], |_, _| 2).run();
    assert_eq!((two.terminal_completion_us, two.completion_sum_us), (4, 6));
    assert_eq!(
        two.work,
        SchedulerCohortWork {
            work_units: 38,
            evidence_rows_validated: 2,
            candidate_evaluations: 2,
            branch_expansions: 2,
            completion_events: 2,
            continuation_events: 0,
            complete_plans: 1,
            dead_ends: 0,
        }
    );
}

#[test]
fn empty_and_oversized_workflow_universes_refuse_before_search() {
    let f = Fixture::new(vec![1], vec![vec![]], |_, _| 1);
    with_fixture(
        &f,
        "",
        false,
        None,
        SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION,
        |c, rows| {
            let empty = SchedulerCohortWorkflowObjective {
                cohort_identity: c.identity,
                objective_identity: c.objective_identity,
                profile: SchedulerCohortWorkflowProfile::Makespan,
                workflows: &[],
            };
            assert_eq!(
                incomplete(evaluate_scheduler_cohort_workflow_objective(
                    c,
                    rows,
                    &empty,
                    policy(),
                    SchedulerCohortBudget::default()
                ))
                .0,
                SchedulerCohortWorkflowIncomplete::InvalidObjective
            );
            let members = [&c.tasks[0]];
            let make = || SchedulerCohortWorkflowObligation {
                workflow_id: &intent(&c.tasks[0]).workflow_id,
                workflow_run_id: &intent(&c.tasks[0]).workflow_run_id,
                completion_contract_identity: "contract",
                required_tasks: &members,
                required_population_complete: true,
                accepted_output_before_cleanup: true,
                release: SchedulerCohortWorkflowRelease::Released { age_us: 0 },
                weight: 1,
            };
            let oversized = [make(), make(), make(), make(), make()];
            let objective = SchedulerCohortWorkflowObjective {
                workflows: &oversized,
                ..empty
            };
            let (reason, work) = incomplete(evaluate_scheduler_cohort_workflow_objective(
                c,
                rows,
                &objective,
                policy(),
                SchedulerCohortBudget::default(),
            ));
            assert_eq!(
                reason,
                SchedulerCohortWorkflowIncomplete::Cohort(
                    SchedulerCohortIncomplete::WorkLimitExceeded
                )
            );
            assert_eq!(work.work_units, 0);
        },
    );
}

#[test]
#[ignore = "controlled pure-kernel dispatch cost; run without concurrent compilation"]
fn workflow_objective_dispatch_cost_probe() {
    let f = Fixture::new(vec![2; 4], vec![vec![]; 4], |h, (t, p)| {
        1 + (h.len() * 11 + t * 7 + p * 3) as u64
    });
    with_fixture(
        &f,
        "",
        false,
        None,
        SCHEDULER_COHORT_WORKFLOW_OUTPUT_CONVENTION,
        |c, rows| {
            let members: Vec<_> = c.tasks.iter().collect();
            let workflows = [SchedulerCohortWorkflowObligation {
                workflow_id: &intent(&c.tasks[0]).workflow_id,
                workflow_run_id: &intent(&c.tasks[0]).workflow_run_id,
                completion_contract_identity: "controlled.complete",
                required_tasks: &members,
                required_population_complete: true,
                accepted_output_before_cleanup: true,
                release: SchedulerCohortWorkflowRelease::Released { age_us: 10 },
                weight: 3,
            }];
            let objective = SchedulerCohortWorkflowObjective {
                cohort_identity: c.identity,
                objective_identity: c.objective_identity,
                profile: SchedulerCohortWorkflowProfile::WeightedFlowTime,
                workflows: &workflows,
            };
            let mut legacy_times = Vec::new();
            let mut workflow_times = Vec::new();
            for i in 0..216 {
                let started = std::time::Instant::now();
                let old = std::hint::black_box(evaluate_scheduler_cohort_completion(
                    c,
                    rows,
                    policy(),
                    SchedulerCohortBudget::default(),
                ));
                let old_time = started.elapsed().as_nanos();
                assert!(matches!(old, SchedulerCohortResult::Ranked(_)));
                let started = std::time::Instant::now();
                let new = std::hint::black_box(evaluate_scheduler_cohort_workflow_objective(
                    c,
                    rows,
                    &objective,
                    policy(),
                    SchedulerCohortBudget::default(),
                ));
                let new_time = started.elapsed().as_nanos();
                assert!(matches!(new, SchedulerCohortWorkflowResult::Ranked(_)));
                if i >= 16 {
                    legacy_times.push(old_time);
                    workflow_times.push(new_time);
                }
            }
            fn summary(mut values: Vec<u128>) -> serde_json::Value {
                values.sort_unstable();
                serde_json::json!({"samples":values.len(),"p50_ns":values[values.len()/2],
                "p95_ns":values[values.len()*95/100],"max_ns":values[values.len()-1]})
            }
            println!(
                "{}",
                serde_json::json!({"scope":"controlled pure4task/2placement/all158rows kernel; excludes capture/provider/admission/runtime work", "legacy":summary(legacy_times),"workflow":summary(workflow_times)})
            );
        },
    );
}
