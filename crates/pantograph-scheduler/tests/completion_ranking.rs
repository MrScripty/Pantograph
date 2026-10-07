use pantograph_scheduler::*;

fn offers(count: usize) -> SchedulerDispatchSelectionRequest {
    let mut request: SchedulerDispatchSelectionRequest = serde_json::from_str(include_str!(
        "fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    request.task_intent.constraints.requested_runtime_id = None;
    request.task_intent.constraints.requested_device_id = None;
    let mut candidate = request.candidates.remove(0);
    candidate.reservations.clear();
    candidate.batching_group_id = None;
    request.candidates = (0..count)
        .map(|index| {
            let mut candidate = candidate.clone();
            candidate.candidate_id = format!("candidate.{index:03}").parse().unwrap();
            candidate
        })
        .collect();
    request
}

fn context() -> SchedulerCompletionContext<'static> {
    SchedulerCompletionContext {
        host_id: "synthetic-host",
        runtime_instance_id: "synthetic-instance",
        artifact_fingerprint: "synthetic-artifact",
        workload_fingerprint: "synthetic-workload",
        resource_condition_fingerprint: "synthetic-idle",
        residency_fingerprint: "synthetic-cold",
        timing_convention: "synthetic-serialized-single-task-mean-us-v1",
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

fn evidence(
    request: &ValidatedSchedulerDispatchSelectionRequest,
) -> Vec<SchedulerCompletionEvidence<'_>> {
    request
        .as_ref()
        .candidates
        .iter()
        .map(|candidate| SchedulerCompletionEvidence {
            request,
            candidate,
            current_context: context(),
            sample: SchedulerCompletionSample {
                candidate,
                context: context(),
                source: SchedulerCompletionEvidenceSource::Synthetic,
                sample_count: 3,
                observed_at_ms: 900,
                preparation_us: Some(0),
                required_transfer_us: Some(0),
                execution_us: Some(100),
            },
        })
        .collect()
}

fn chosen(result: &SchedulerCompletionRankingResult) -> &str {
    let SchedulerDispatchReservationSelection::Selected { candidate_id, .. } = &result.selection
    else {
        panic!("expected a candidate: {result:?}");
    };
    candidate_id.as_str()
}

fn assert_fallback(
    request: &ValidatedSchedulerDispatchSelectionRequest,
    result: SchedulerCompletionRankingResult,
    reason: SchedulerCompletionRefusalReason,
) {
    assert_eq!(
        result.diagnostic,
        SchedulerCompletionRankingDiagnostic::Fallback(reason)
    );
    assert_eq!(
        result.selection,
        select_scheduler_candidate_for_reservation(request)
    );
}

#[test]
fn thesis_cold_warm_and_transfer_reversal_are_explicitly_synthetic() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/completion_reversal_synthetic.json")).unwrap();
    assert_eq!(fixture["source"], "synthetic");
    let mut request = offers(2);
    request.candidates[0].candidate_id = "candidate.warm".parse().unwrap();
    request.candidates[0].selected_runtime_id = "cpu-runtime".parse().unwrap();
    request.candidates[0].selected_device_ids = vec!["cpu".parse().unwrap()];
    request.candidates[1].candidate_id = "candidate.cold".parse().unwrap();
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(request).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let mut rows = evidence(&request);
        rows[0].current_context.residency_fingerprint = "synthetic-warm";
        rows[0].sample.context = rows[0].current_context;
        for (index, name) in ["warm_us", "cold_us"].into_iter().enumerate() {
            let stages = case[name].as_array().unwrap();
            rows[index].sample.preparation_us = stages[0].as_u64();
            rows[index].sample.required_transfer_us = stages[1].as_u64();
            rows[index].sample.execution_us = stages[2].as_u64();
        }
        let result = select_scheduler_candidate_with_completion(&request, &rows, policy());
        assert_eq!(
            chosen(&result),
            case["expected_candidate"].as_str().unwrap()
        );
        assert_eq!(
            result.diagnostic,
            SchedulerCompletionRankingDiagnostic::Ranked {
                predicted_completion_us: case["expected_completion_us"].as_u64().unwrap(),
                source: SchedulerCompletionEvidenceSource::Synthetic,
                eligible_candidates: 2,
            }
        );
    }
}

#[test]
fn synthetic_load_cost_alone_reverses_cold_and_warm_choice() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    let mut rows = evidence(&request);
    rows[0].current_context.residency_fingerprint = "synthetic-warm";
    rows[0].sample.context = rows[0].current_context;
    rows[0].sample.execution_us = Some(12_000_000);
    rows[1].sample.execution_us = Some(3_000_000);

    // Only the cold candidate's serialized preparation/load estimate changes.
    // Transfers remain explicitly zero and execution costs remain fixed.
    for (load_us, expected_candidate, expected_completion_us) in [
        (2_000_000, "candidate.001", 5_000_000),
        (10_000_000, "candidate.000", 12_000_000),
    ] {
        rows[1].sample.preparation_us = Some(load_us);
        let result = select_scheduler_candidate_with_completion(&request, &rows, policy());
        assert_eq!(chosen(&result), expected_candidate);
        assert_eq!(
            result.diagnostic,
            SchedulerCompletionRankingDiagnostic::Ranked {
                predicted_completion_us: expected_completion_us,
                source: SchedulerCompletionEvidenceSource::Synthetic,
                eligible_candidates: 2,
            }
        );
    }
}

#[test]
fn invalid_samples_retain_the_entire_conservative_population() {
    use SchedulerCompletionRefusalReason as Reason;
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    for (case, reason) in [
        Reason::MissingStage,
        Reason::MissingStage,
        Reason::MissingStage,
        Reason::DurationOverflow,
        Reason::DurationOverflow,
        Reason::InsufficientSamples,
        Reason::StaleOrFutureSample,
        Reason::StaleOrFutureSample,
        Reason::IdentityMismatch,
        Reason::InvalidContext,
        Reason::InvalidContext,
        Reason::IncomparableEvidence,
        Reason::IncomparableEvidence,
        Reason::IncomparableEvidence,
        Reason::IncomparableEvidence,
        Reason::IncomparableEvidence,
    ]
    .into_iter()
    .enumerate()
    {
        let mut rows = evidence(&request);
        // The first row appears much faster. Invalid second-row evidence must
        // never silently discard that alternative and choose the known row.
        rows[0].sample.execution_us = Some(1);
        match case {
            0 => rows[1].sample.preparation_us = None,
            1 => rows[1].sample.required_transfer_us = None,
            2 => rows[1].sample.execution_us = None,
            3 => {
                rows[1].sample.preparation_us = Some(u64::MAX);
                rows[1].sample.required_transfer_us = Some(1);
            }
            4 => {
                rows[1].sample.preparation_us = Some(1);
                rows[1].sample.execution_us = Some(u64::MAX);
            }
            5 => rows[1].sample.sample_count = 2,
            6 => rows[1].sample.observed_at_ms = 899,
            7 => rows[1].sample.observed_at_ms = 1001,
            8 => rows[1].sample.context.runtime_instance_id = "old-instance",
            9 => rows[1].sample.context.artifact_fingerprint = "",
            10 => rows[1].sample.context.host_id = "bad\nidentity",
            11 => rows[1].current_context.host_id = "different-host",
            12 => rows[1].current_context.workload_fingerprint = "different-workload",
            13 => rows[1].current_context.resource_condition_fingerprint = "different-condition",
            14 => rows[1].current_context.timing_convention = "different-convention",
            15 => rows[1].sample.source = SchedulerCompletionEvidenceSource::Measured,
            _ => unreachable!(),
        }
        if (11..=14).contains(&case) {
            rows[1].sample.context = rows[1].current_context;
        }
        assert_fallback(
            &request,
            select_scheduler_candidate_with_completion(&request, &rows, policy()),
            reason,
        );
    }
}

#[test]
fn every_current_identity_field_must_match_the_sample() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    for index in 0..7 {
        let mut rows = evidence(&request);
        let ctx = &mut rows[1].sample.context;
        match index {
            0 => ctx.host_id = "old-host",
            1 => ctx.runtime_instance_id = "old-instance",
            2 => ctx.artifact_fingerprint = "old-artifact",
            3 => ctx.workload_fingerprint = "old-workload",
            4 => ctx.resource_condition_fingerprint = "old-resource-condition",
            5 => ctx.residency_fingerprint = "old-residency",
            6 => ctx.timing_convention = "old-convention",
            _ => unreachable!(),
        }
        assert_fallback(
            &request,
            select_scheduler_candidate_with_completion(&request, &rows, policy()),
            SchedulerCompletionRefusalReason::IdentityMismatch,
        );
    }
}

#[test]
fn missing_duplicate_and_foreign_snapshot_evidence_is_not_a_ranking() {
    use SchedulerCompletionRefusalReason as Reason;
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    let clone = request.clone();
    let candidate_clone = request.as_ref().candidates[1].clone();
    for (case, reason) in [
        Reason::MissingEvidence,
        Reason::DuplicateEvidence,
        Reason::ForeignSnapshot,
        Reason::ForeignSnapshot,
        Reason::ForeignSnapshot,
    ]
    .into_iter()
    .enumerate()
    {
        let mut rows = evidence(&request);
        match case {
            0 => {
                rows.pop();
            }
            1 => rows.push(rows[0]),
            2 => rows[1].request = &clone,
            3 => rows[1].candidate = &candidate_clone,
            4 => rows[1].sample.candidate = &candidate_clone,
            _ => unreachable!(),
        }
        assert_fallback(
            &request,
            select_scheduler_candidate_with_completion(&request, &rows, policy()),
            reason,
        );
    }
}

#[test]
fn ties_are_deterministic_under_offer_and_evidence_reordering() {
    for reverse in [false, true] {
        let mut input = offers(2);
        if reverse {
            input.candidates.reverse();
        }
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(input).unwrap();
        let mut rows = evidence(&request);
        rows.reverse();
        assert_eq!(
            chosen(&select_scheduler_candidate_with_completion(
                &request,
                &rows,
                policy()
            )),
            "candidate.000"
        );
    }
}

#[test]
fn synthetic_source_requires_opt_in_and_measured_source_remains_labelled() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    let mut rows = evidence(&request);
    let mut policy = policy();
    policy.allow_synthetic = false;
    assert_fallback(
        &request,
        select_scheduler_candidate_with_completion(&request, &rows, policy),
        SchedulerCompletionRefusalReason::SyntheticEvidenceDisabled,
    );
    // This is a structural provenance test, not an actual hardware measurement.
    for row in &mut rows {
        row.sample.source = SchedulerCompletionEvidenceSource::Measured;
    }
    assert!(matches!(
        select_scheduler_candidate_with_completion(&request, &rows, policy).diagnostic,
        SchedulerCompletionRankingDiagnostic::Ranked {
            source: SchedulerCompletionEvidenceSource::Measured,
            ..
        }
    ));
}

#[test]
fn invalid_policy_and_missing_evidence_preserve_sole_candidate_selection() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(1)).unwrap();
    for invalid in [false, true] {
        let mut policy = policy();
        if invalid {
            policy.minimum_samples = 0;
        } else {
            policy.max_sample_age_ms = 0;
        }
        assert_fallback(
            &request,
            select_scheduler_candidate_with_completion(&request, &[], policy),
            SchedulerCompletionRefusalReason::InvalidPolicy,
        );
    }
    let result = select_scheduler_candidate_with_completion(&request, &[], policy());
    assert_eq!(chosen(&result), "candidate.000");
    assert_fallback(
        &request,
        result,
        SchedulerCompletionRefusalReason::MissingEvidence,
    );
}

#[test]
fn hard_constraints_and_fits_remain_authoritative() {
    for device_constraint in [false, true] {
        let mut input = offers(2);
        input.candidates[1].selected_runtime_id = "other-runtime".parse().unwrap();
        input.candidates[1].selected_device_ids = vec!["cpu".parse().unwrap()];
        if device_constraint {
            input.task_intent.constraints.requested_device_id = Some("cuda:0".parse().unwrap());
        } else {
            input.task_intent.constraints.requested_runtime_id =
                Some("diffusers-pytorch".parse().unwrap());
        }
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(input).unwrap();
        let mut rows = evidence(&request);
        rows[1].sample.execution_us = Some(1);
        assert_eq!(
            chosen(&select_scheduler_candidate_with_completion(
                &request,
                &rows,
                policy()
            )),
            "candidate.000"
        );
    }
    let mut input = offers(2);
    input.candidates[1].resource_fit_assessment = None;
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(input).unwrap();
    let rows = evidence(&request);
    assert_eq!(
        chosen(&select_scheduler_candidate_with_completion(
            &request,
            &rows,
            policy()
        )),
        "candidate.000"
    );
}

#[test]
fn duplicate_offers_batching_and_multi_device_modes_keep_legacy_behavior() {
    for case in 0..3 {
        let mut input = offers(2);
        match case {
            0 => input.candidates[1].candidate_id = input.candidates[0].candidate_id.clone(),
            1 => input.candidates[0].batching_group_id = Some("batch.001".parse().unwrap()),
            2 => input.candidates[0]
                .selected_device_ids
                .push("cpu".parse().unwrap()),
            _ => unreachable!(),
        }
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(input).unwrap();
        let rows = evidence(&request);
        assert_fallback(
            &request,
            select_scheduler_candidate_with_completion(&request, &rows, policy()),
            if case == 0 {
                SchedulerCompletionRefusalReason::IneligibleOrDuplicateCandidates
            } else {
                SchedulerCompletionRefusalReason::UnsupportedExecutionMode
            },
        );
    }
}

#[test]
fn fixed_work_limits_refuse_without_truncating_or_falling_back() {
    for case in 0..4 {
        let mut input = offers(if case == 0 { 65 } else { 2 });
        if case == 1 {
            input.candidates[0].selected_device_ids = (0..9)
                .map(|index| format!("device.{index}").parse().unwrap())
                .collect();
        }
        if case == 2 {
            input.diagnostics =
                vec![input.candidates[0].candidate_source_diagnostics[0].clone(); 33];
        }
        let request = ValidatedSchedulerDispatchSelectionRequest::try_from(input).unwrap();
        let mut rows = evidence(&request);
        if case == 3 {
            rows = vec![rows[0]; 65];
        }
        let result = select_scheduler_candidate_with_completion(&request, &rows, policy());
        assert_eq!(
            result.diagnostic,
            SchedulerCompletionRankingDiagnostic::Refused(
                SchedulerCompletionRefusalReason::WorkLimitExceeded
            )
        );
        assert!(matches!(
            result.selection,
            SchedulerDispatchReservationSelection::NoSelection { .. }
        ));
    }
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(64)).unwrap();
    assert!(matches!(
        select_scheduler_candidate_with_completion(&request, &evidence(&request), policy())
            .diagnostic,
        SchedulerCompletionRankingDiagnostic::Ranked {
            eligible_candidates: 64,
            ..
        }
    ));
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    let oversized = "x".repeat(129);
    let mut rows = evidence(&request);
    rows[0].current_context.host_id = &oversized;
    assert_fallback(
        &request,
        select_scheduler_candidate_with_completion(&request, &rows, policy()),
        SchedulerCompletionRefusalReason::InvalidContext,
    );
}

#[test]
fn ranking_is_non_mutating_and_does_not_produce_dispatch_authority() {
    let request = ValidatedSchedulerDispatchSelectionRequest::try_from(offers(2)).unwrap();
    let before = request.clone();
    let mut rows = evidence(&request);
    rows[1].sample.execution_us = Some(1);
    let result = select_scheduler_candidate_with_completion(&request, &rows, policy());
    assert_eq!(chosen(&result), "candidate.001");
    assert_eq!(request, before);
    assert!(request
        .as_ref()
        .candidates
        .iter()
        .all(|c| c.reservations.is_empty()));
    let dispatch = select_scheduler_dispatch(request.clone())
        .unwrap()
        .into_inner();
    assert_eq!(dispatch.state, SchedulerDispatchSelectionState::NoSelection);
    assert!(dispatch.dispatch_decision.is_none());
    assert!(dispatch
        .diagnostics
        .iter()
        .any(|d| d.code == SchedulerDispatchSelectionDiagnosticCode::MissingReservation));
}
