use super::*;

fn evaluate(
    x: Vec<u64>,
    y: Vec<Vec<u64>>,
    fault: &str,
) -> (
    SchedulerTwoCompletionResult,
    SchedulerDispatchReservationSelection,
) {
    let first = offers(x.len(), false);
    let base = offers(y[0].len(), true);
    let mut next = SchedulerCompletionSuccessorRequest {
        task_intent: base.as_ref().task_intent.clone(),
        snapshot_identity: "workload.two".into(),
        candidates: base.as_ref().candidates.clone(),
    };
    for c in &mut next.candidates {
        c.resource_fit_assessment = None;
    }
    if fault == "hard-future" {
        next.task_intent.constraints.requested_runtime_id =
            Some(next.candidates[0].selected_runtime_id.clone());
        next.candidates[1].selected_runtime_id = "foreign-runtime".parse().unwrap();
    }
    let next: ValidatedSchedulerCompletionSuccessor = next.try_into().unwrap();
    let first_rows = rows(&first, &x, &vec![0; x.len()], false, 0);
    let baseline =
        select_scheduler_candidate_with_completion(&first, &first_rows, policy()).selection;
    let mut fits: Vec<_> = (0..x.len() * y[0].len())
        .map(|_| SchedulerResourceFitAssessment {
            workflow_run_id: next.as_ref().task_intent.workflow_run_id.clone(),
            task_id: next.as_ref().task_intent.task_id.clone(),
            state: SchedulerResourceFitState::Fits,
            diagnostics: vec![],
        })
        .collect();
    if fault == "unknown" {
        fits[0].state = SchedulerResourceFitState::Unknown;
    }
    if matches!(fault, "infeasible-first" | "all-infeasible") {
        let count = if fault == "all-infeasible" {
            fits.len()
        } else {
            y[0].len()
        };
        for fit in fits.iter_mut().take(count) {
            fit.state = SchedulerResourceFitState::WaitingForResources;
            fit.diagnostics.push(SchedulerResourceDiagnostic {
                severity: SchedulerResourceDiagnosticSeverity::Info,
                code: SchedulerResourceDiagnosticCode::ObservationUnavailable,
                message: "qualified conditional capacity deficit".into(),
                hint: None,
            });
        }
    }
    let mut evidence = Vec::new();
    for i in 0..x.len() {
        for j in 0..y[i].len() {
            if fault == "hard-future" && j == 1 {
                continue;
            }
            let candidate = &next.as_ref().candidates[j];
            let mut ctx = context(true, i);
            if fault == "transition" && i == 0 && j == 1 {
                ctx.resource_condition_fingerprint = "another-post-release-state";
            }
            if fault == "foreign-workload" {
                ctx.workload_fingerprint = "foreign";
            }
            let sample = SchedulerCompletionSample {
                candidate,
                context: ctx,
                source: SchedulerCompletionEvidenceSource::Synthetic,
                sample_count: if fault == "uncalibrated" { 0 } else { 3 },
                observed_at_ms: if fault == "stale" { 0 } else { 900 },
                preparation_us: Some(0),
                required_transfer_us: if fault == "unknown-transfer" {
                    None
                } else {
                    Some(0)
                },
                execution_us: Some(y[i][j]),
            };
            evidence.push(SchedulerDependencyCompletionEvidence {
                successor: &next,
                predecessor_candidate: &first.as_ref().candidates[i],
                predecessor_context: first_rows[i].current_context,
                candidate,
                context: ctx,
                transition_fingerprint: ctx.resource_condition_fingerprint,
                resource_fit: &fits[i * y[0].len() + j],
                sample: (fits[i * y[0].len() + j].state == SchedulerResourceFitState::Fits
                    || fault == "unknown")
                    .then_some(sample),
            });
        }
    }
    if fault == "missing" {
        evidence.pop();
    }
    if fault == "duplicate" {
        evidence[1].candidate = evidence[0].candidate;
    }
    if fault == "permuted" {
        evidence.reverse();
    }
    (
        select_scheduler_candidate_with_dependency_completion(
            &first,
            &first_rows,
            &next,
            &evidence,
            policy(),
        ),
        baseline,
    )
}
#[test]
fn dependency_forecast_reverses_greedy_for_two_completion_terminal_objective() {
    let (r, baseline) = evaluate(vec![2, 5], vec![vec![20, 21], vec![1, 2]], "");
    assert!(
        matches!(baseline, SchedulerDispatchReservationSelection::Selected { ref candidate_id, .. } if candidate_id.as_str() == "candidate.000")
    );
    assert!(
        matches!(r.selection, SchedulerDispatchReservationSelection::Selected { ref candidate_id, .. } if candidate_id.as_str() == "candidate.001")
    );
    let SchedulerTwoCompletionDiagnostic::Ranked(score) = r.diagnostic else {
        panic!("expected bounded completion plan");
    };
    assert_eq!(
        (
            score.first_completion_us,
            score.terminal_completion_us,
            score.completion_sum_us,
            score.plans_evaluated,
            score.completion_events_evaluated
        ),
        (5, 6, 11, 4, 8)
    );
}
#[test]
fn dependency_forecast_unknown_incomplete_foreign_stale_and_uncalibrated_preserve_exact_baseline() {
    for fault in [
        "unknown",
        "missing",
        "duplicate",
        "foreign-workload",
        "uncalibrated",
        "stale",
        "unknown-transfer",
        "transition",
    ] {
        let (r, baseline) = evaluate(vec![2, 5], vec![vec![20, 21], vec![1, 2]], fault);
        assert_eq!(r.selection, baseline, "{fault}");
        assert!(
            matches!(r.diagnostic, SchedulerTwoCompletionDiagnostic::Fallback(_)),
            "{fault}"
        );
    }
}
#[test]
fn dependency_forecast_overflow_preserves_baseline() {
    let (r, baseline) = evaluate(vec![2, 5], vec![vec![u64::MAX, 1], vec![1, 2]], "");
    assert_eq!(r.selection, baseline);
    assert_eq!(
        r.diagnostic,
        SchedulerTwoCompletionDiagnostic::Fallback(
            SchedulerTwoCompletionFallback::DurationOverflow
        )
    );
}
#[test]
fn dependency_successor_has_no_dispatch_proof_or_reservations_and_rejects_oversized_fit_before_validation(
) {
    let base = offers(2, true);
    let mut request = SchedulerCompletionSuccessorRequest {
        task_intent: base.as_ref().task_intent.clone(),
        snapshot_identity: "workload.two".into(),
        candidates: base.as_ref().candidates.clone(),
    };
    request.candidates[0]
        .resource_fit_assessment
        .as_mut()
        .unwrap()
        .diagnostics = vec![SchedulerResourceDiagnostic {
        severity: SchedulerResourceDiagnosticSeverity::Info,
        code: SchedulerResourceDiagnosticCode::ObservationUnavailable,
        message: "x".repeat(1025),
        hint: None,
    }];
    assert!(ValidatedSchedulerCompletionSuccessor::try_from(request).is_err());
}

#[test]
fn dependency_forecast_infeasible_paths_are_qualified_and_never_assume_unknown_capacity() {
    let (r, _) = evaluate(vec![2, 5], vec![vec![1, 1], vec![1, 2]], "infeasible-first");
    let SchedulerTwoCompletionDiagnostic::Ranked(score) = r.diagnostic else {
        panic!("qualified feasible branch");
    };
    assert_eq!(
        (
            score.terminal_completion_us,
            score.plans_evaluated,
            score.completion_events_evaluated
        ),
        (6, 2, 4)
    );
    assert!(
        matches!(r.selection,SchedulerDispatchReservationSelection::Selected { candidate_id,.. } if candidate_id.as_str()=="candidate.001")
    );
    let (r, baseline) = evaluate(vec![2, 5], vec![vec![1, 1], vec![1, 2]], "all-infeasible");
    assert_eq!(r.selection, baseline);
    assert_eq!(
        r.diagnostic,
        SchedulerTwoCompletionDiagnostic::Fallback(SchedulerTwoCompletionFallback::NoCompletePlan)
    );
}
#[test]
fn dependency_forecast_matches_existing_two_event_semantics_for_many_matrices_including_ties_and_maximum(
) {
    for n in 1..=4 {
        for seed in 0..64u64 {
            let x: Vec<_> = (0..n).map(|i| (seed * 7 + i as u64 * 11) % 29).collect();
            let y: Vec<Vec<_>> = (0..n)
                .map(|i| {
                    (0..n)
                        .map(|j| (seed * 13 + i as u64 * 5 + j as u64 * 3) % 31)
                        .collect()
                })
                .collect();
            let old = super::evaluate(
                &Matrix::new(x.clone(), y.clone()),
                Fault::None,
                SchedulerTwoCompletionBudget::default(),
            );
            let (new, _) = evaluate(x.clone(), y.clone(), "");
            let (permuted, _) = evaluate(x, y, "permuted");
            let SchedulerTwoCompletionDiagnostic::Ranked(a) = old.diagnostic else {
                panic!("old qualified score");
            };
            let SchedulerTwoCompletionDiagnostic::Ranked(b) = &new.diagnostic else {
                panic!("new qualified score");
            };
            assert_eq!(
                (
                    &a.successor_candidate_id,
                    a.first_completion_us,
                    a.terminal_completion_us,
                    a.completion_sum_us,
                    a.plans_evaluated,
                    a.completion_events_evaluated
                ),
                (
                    &b.successor_candidate_id,
                    b.first_completion_us,
                    b.terminal_completion_us,
                    b.completion_sum_us,
                    b.plans_evaluated,
                    b.completion_events_evaluated
                )
            );
            let selected = |s: SchedulerDispatchReservationSelection| match s {
                SchedulerDispatchReservationSelection::Selected { candidate_id, .. } => {
                    candidate_id
                }
                _ => panic!("selection"),
            };
            assert_eq!(selected(old.selection), selected(new.selection.clone()));
            assert_eq!(new, permuted);
        }
    }
    let (r, _) = evaluate(
        vec![2, 5],
        vec![vec![1000, 1], vec![2, 2000]],
        "hard-future",
    );
    assert!(
        matches!(r.selection,SchedulerDispatchReservationSelection::Selected { candidate_id,.. } if candidate_id.as_str()=="candidate.001")
    );
}
