use pantograph_scheduler::*;

fn intent(app: &str, task: &str) -> ValidatedSchedulableTaskIntent {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    let mut dto = value["task_intent"].take();
    dto["fairness_key"] = app.into();
    dto["task_id"] = task.into();
    let raw: SchedulableTaskIntent = serde_json::from_value(dto).unwrap();
    raw.try_into().unwrap()
}
fn app(s: &str) -> SchedulerFairnessKey {
    SchedulerFairnessKey::parse(s).unwrap()
}
fn opportunity(
    i: &ValidatedSchedulableTaskIntent,
    since: u64,
) -> SchedulerProtectedOpportunity<'_> {
    SchedulerProtectedOpportunity {
        intent: i,
        generation: 1,
        continuously_eligible_since_us: since,
        eligibility: SchedulerProtectedEligibility::Ready,
        feasibility: SchedulerProtectedFeasibility::IndividuallyFeasible,
        episode_contract_identity: "accepted.output.v1",
    }
}
fn snapshot<'a>(
    now: u64,
    apps: &'a [SchedulerFairnessKey],
    ops: &'a [SchedulerProtectedOpportunity<'a>],
    protected: Option<&'a SchedulerProtectedTarget>,
    blockers: &'a [SchedulerProtectedBlockingContinuation<'a>],
) -> SchedulerProtectedSnapshot<'a> {
    SchedulerProtectedSnapshot {
        owner_identity: "owner.1",
        now_us: now,
        admitted_population_complete: true,
        admitted_apps: apps,
        waiting_population_complete: true,
        opportunities: ops,
        blocking_continuations_complete: true,
        protected_target: protected,
        blocking_continuations: blockers,
    }
}
fn observe(
    s: &SchedulerProgressProtection,
    snap: &SchedulerProtectedSnapshot<'_>,
) -> (SchedulerProgressProtection, SchedulerProtectedDecision) {
    s.advance(
        snap,
        SchedulerProtectedEvent::Observe,
        SCHEDULER_PROTECTION_MAX_WORK,
    )
    .unwrap()
}
fn running(
    s: &SchedulerProgressProtection,
    d: &SchedulerProtectedDecision,
    snap: &SchedulerProtectedSnapshot<'_>,
) -> (SchedulerProgressProtection, SchedulerProtectedDecision) {
    let a = d.active().unwrap();
    s.advance(
        snap,
        SchedulerProtectedEvent::Started {
            turn_id: a.turn_id,
            attempt: a.attempt,
        },
        SCHEDULER_PROTECTION_MAX_WORK,
    )
    .unwrap()
}
fn finish(
    s: &SchedulerProgressProtection,
    d: &SchedulerProtectedDecision,
    snap: &SchedulerProtectedSnapshot<'_>,
    outcome: SchedulerProtectedOutcome,
) -> (SchedulerProgressProtection, SchedulerProtectedDecision) {
    let a = d.active().unwrap();
    s.advance(
        snap,
        SchedulerProtectedEvent::Finished {
            turn_id: a.turn_id,
            attempt: a.attempt,
            outcome,
            physical_release_acknowledged: true,
        },
        SCHEDULER_PROTECTION_MAX_WORK,
    )
    .unwrap()
}

#[test]
fn total_round_gap_survives_replanning_arrivals_and_protected_episodes() {
    let a = intent("app.a", "cold.old");
    let b = intent("app.b", "warm.new");
    let c = intent("app.c", "arrival");
    let apps = [app("app.a"), app("app.b"), app("app.c")];
    let old = [opportunity(&a, 0), opportunity(&b, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 10, 2).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps[..2], &old, None, &[]));
    assert!(d.active().is_none());
    let (s, d) = observe(&s, &snapshot(6, &apps[..2], &old, None, &[]));
    assert_eq!(d.discretionary_used_us(), 6);
    let all = [opportunity(&a, 0), opportunity(&b, 0), opportunity(&c, 6)];
    let (s, d) = observe(&s, &snapshot(10, &apps, &all, None, &[]));
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "cold.old");
    let target = d.active().unwrap().target.clone();
    let (s, _) = running(&s, &d, &snapshot(10, &apps, &all, Some(&target), &[]));
    // Cold load/execution can exceed any planner horizon; it does not erase the turn.
    let (s, d) = observe(&s, &snapshot(1000, &apps, &all, Some(&target), &[]));
    assert_eq!(d.discretionary_used_us(), 10);
    assert_eq!(d.active().unwrap().phase, SchedulerProtectedPhase::Running);
    let remaining = [opportunity(&b, 0), opportunity(&c, 6)];
    let (s, d) = finish(
        &s,
        &d,
        &snapshot(1001, &apps, &remaining, Some(&target), &[]),
        SchedulerProtectedOutcome::AcceptedOutput,
    );
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "warm.new");
    assert_eq!(d.round(), 1);
    assert_eq!(d.discretionary_used_us(), 10);
    // Completing A does not provide another G to postpone B.
    let (_s, replanned) = observe(&s, &snapshot(1001, &apps, &remaining, None, &[]));
    assert_eq!(replanned.active(), d.active());
}

#[test]
fn oldest_task_in_same_app_wins_independent_of_workflow_boost_or_warmth() {
    let oldest = intent("app.a", "task.old");
    let new = intent("app.a", "task.new");
    let apps = [app("app.a")];
    for ops in [
        [opportunity(&oldest, 0), opportunity(&new, 5)],
        [opportunity(&new, 5), opportunity(&oldest, 0)],
    ] {
        let s = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
        let (_, d) = observe(&s, &snapshot(10, &apps, &ops, None, &[]));
        assert_eq!(d.active().unwrap().target.task_id.as_str(), "task.old");
    }
}

#[test]
fn full_target_frame_and_exact_blocking_continuations_gate_both_search_and_fallback() {
    let cold = intent("app.a", "cold");
    let pinned = intent("app.b", "pinned.kv");
    let unrelated = intent("app.b", "new.sequence");
    let apps = [app("app.a"), app("app.b")];
    let ops = [opportunity(&cold, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    assert_eq!(
        d.permission(
            "owner.1",
            d.revision(),
            &cold,
            1,
            SchedulerProtectedRelation::Unknown
        ),
        SchedulerProtectedPermission::HoldNewAdmission
    );
    let blocks = [SchedulerProtectedBlockingContinuation {
        intent: &pinned,
        generation: 9,
        envelope_identity: "pinned.kv.bounded.growth",
        full_growth_covered: true,
    }];
    let (_s, d) = observe(&s, &snapshot(0, &apps, &ops, Some(&t), &blocks));
    assert!(d.target_envelope_qualified());
    for _consumer in ["completion-search", "incomplete-ranking-fallback"] {
        let gate = |i, g, r| d.permission("owner.1", d.revision(), i, g, r);
        assert_eq!(
            gate(&cold, 1, SchedulerProtectedRelation::Unknown),
            SchedulerProtectedPermission::ConsiderProtectedTarget
        );
        assert_eq!(
            gate(
                &pinned,
                9,
                SchedulerProtectedRelation::RequiredBlockingContinuation
            ),
            SchedulerProtectedPermission::ConsiderRequiredBlockingContinuation
        );
        assert_eq!(
            gate(
                &pinned,
                10,
                SchedulerProtectedRelation::RequiredBlockingContinuation
            ),
            SchedulerProtectedPermission::HoldNewAdmission
        );
        assert_eq!(
            gate(
                &unrelated,
                9,
                SchedulerProtectedRelation::RequiredBlockingContinuation
            ),
            SchedulerProtectedPermission::HoldNewAdmission
        );
        assert_eq!(
            gate(&unrelated, 1, SchedulerProtectedRelation::Nonconflicting),
            SchedulerProtectedPermission::ConsiderNonconflicting
        );
        assert_eq!(
            gate(&unrelated, 1, SchedulerProtectedRelation::Conflicting),
            SchedulerProtectedPermission::HoldNewAdmission
        );
        assert_eq!(
            gate(&unrelated, 1, SchedulerProtectedRelation::Unknown),
            SchedulerProtectedPermission::HoldNewAdmission
        );
        assert_eq!(
            gate(&cold, 2, SchedulerProtectedRelation::Nonconflicting),
            SchedulerProtectedPermission::HoldNewAdmission
        );
    }
    assert_eq!(
        d.permission(
            "other.owner",
            d.revision(),
            &cold,
            1,
            SchedulerProtectedRelation::Nonconflicting
        ),
        SchedulerProtectedPermission::HoldNewAdmission
    );
    assert_eq!(
        d.permission(
            "owner.1",
            d.revision() - 1,
            &cold,
            1,
            SchedulerProtectedRelation::Nonconflicting
        ),
        SchedulerProtectedPermission::HoldNewAdmission
    );
}

#[test]
fn omission_and_replanning_cannot_clear_active_or_reset_continuity() {
    let i = intent("app.a", "old");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 5, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(5, &apps, &ops, None, &[]));
    let (s, d2) = observe(&s, &snapshot(100, &apps, &[], None, &[]));
    assert_eq!(d.active(), d2.active());
    let (s, _) = observe(&s, &snapshot(100, &apps, &ops, None, &[]));
    let mut changed = opportunity(&i, 100);
    assert_eq!(
        s.advance(
            &snapshot(100, &apps, &[changed], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ContinuityChanged
    );
    changed = opportunity(&i, 0);
    changed.generation = 2;
    assert_eq!(
        s.advance(
            &snapshot(100, &apps, &[changed], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ContinuityChanged
    );
    assert_eq!(
        s.advance(
            &snapshot(100, &[], &[], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ActiveAppRemoved
    );
}

#[test]
fn cold_preparation_failures_consume_retries_and_acknowledged_terminal_failure_advances() {
    let a = intent("app.a", "cold");
    let b = intent("app.b", "background");
    let apps = [app("app.a"), app("app.b")];
    let ops = [opportunity(&a, 0), opportunity(&b, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 2).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let first = d.active().cloned().unwrap();
    // No Started event: load itself failed, but still counts as attempt one.
    let (s, d) = finish(
        &s,
        &d,
        &snapshot(1, &apps, &ops, None, &[]),
        SchedulerProtectedOutcome::Failed,
    );
    assert_eq!(d.active().unwrap().attempt, 2);
    assert_eq!(d.active().unwrap().turn_id, first.turn_id);
    assert!(d.finished().is_none());
    let old = s.clone();
    let stale = SchedulerProtectedEvent::Finished {
        turn_id: first.turn_id,
        attempt: 1,
        outcome: SchedulerProtectedOutcome::Failed,
        physical_release_acknowledged: true,
    };
    assert_eq!(
        s.advance(&snapshot(2, &apps, &ops, None, &[]), stale, 4096)
            .unwrap_err(),
        SchedulerProtectionRefusal::StaleEvent
    );
    assert_eq!(s, old);
    let active = d.active().unwrap();
    let unacked = SchedulerProtectedEvent::Finished {
        turn_id: active.turn_id,
        attempt: active.attempt,
        outcome: SchedulerProtectedOutcome::Failed,
        physical_release_acknowledged: false,
    };
    assert_eq!(
        s.advance(&snapshot(2, &apps, &ops, None, &[]), unacked, 4096)
            .unwrap_err(),
        SchedulerProtectionRefusal::ReleaseUnacknowledged
    );
    let (_s, d) = finish(
        &s,
        &d,
        &snapshot(2, &apps, &ops, None, &[]),
        SchedulerProtectedOutcome::Failed,
    );
    assert_eq!(d.finished(), Some(SchedulerProtectedOutcome::Failed));
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "background");
}

#[test]
fn running_target_continuations_remain_possible_but_duplicate_start_does_not() {
    let i = intent("app.a", "decode");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    let blockers = [SchedulerProtectedBlockingContinuation {
        intent: &i,
        generation: 1,
        envelope_identity: "kv.full.episode",
        full_growth_covered: true,
    }];
    let (s, d) = running(&s, &d, &snapshot(0, &apps, &ops, Some(&t), &blockers));
    assert_eq!(
        d.permission(
            "owner.1",
            d.revision(),
            &i,
            1,
            SchedulerProtectedRelation::RequiredBlockingContinuation
        ),
        SchedulerProtectedPermission::ConsiderRequiredBlockingContinuation
    );
    assert_eq!(
        d.permission(
            "owner.1",
            d.revision(),
            &i,
            1,
            SchedulerProtectedRelation::Nonconflicting
        ),
        SchedulerProtectedPermission::HoldNewAdmission
    );
    let active = d.active().unwrap();
    assert_eq!(
        s.advance(
            &snapshot(1, &apps, &ops, Some(&t), &blockers),
            SchedulerProtectedEvent::Started {
                turn_id: active.turn_id,
                attempt: 1
            },
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::StaleEvent
    );
    assert_eq!(
        s.advance(
            &snapshot(1, &apps, &ops, Some(&t), &blockers),
            SchedulerProtectedEvent::Finished {
                turn_id: active.turn_id,
                attempt: 1,
                outcome: SchedulerProtectedOutcome::AcceptedOutput,
                physical_release_acknowledged: true
            },
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let (_, d) = finish(
        &s,
        &d,
        &snapshot(1, &apps, &[], Some(&t), &[]),
        SchedulerProtectedOutcome::AcceptedOutput,
    );
    assert_eq!(
        d.finished(),
        Some(SchedulerProtectedOutcome::AcceptedOutput)
    );
    assert!(d.active().is_none());
}

#[test]
fn same_app_readmission_cannot_buy_another_turn_or_another_gap_in_this_round() {
    let a = intent("app.a", "old.a");
    let b = intent("app.b", "old.b");
    let new = intent("app.a", "new.a");
    let apps = [app("app.a"), app("app.b")];
    let ops = [opportunity(&a, 0), opportunity(&b, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let (s, d) = running(&s, &d, &snapshot(0, &apps, &ops, None, &[]));
    let b_only = [opportunity(&b, 0)];
    let (s, d) = finish(
        &s,
        &d,
        &snapshot(1, &apps[1..], &b_only, None, &[]),
        SchedulerProtectedOutcome::AcceptedOutput,
    );
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "old.b");
    let rejoined = [opportunity(&new, 1), opportunity(&b, 0)];
    let (s, d) = observe(&s, &snapshot(1, &apps, &rejoined, None, &[]));
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "old.b");
    let (s, d) = running(&s, &d, &snapshot(1, &apps, &rejoined, None, &[]));
    let (_, d) = finish(
        &s,
        &d,
        &snapshot(2, &apps, &[opportunity(&new, 1)], None, &[]),
        SchedulerProtectedOutcome::AcceptedOutput,
    );
    assert_eq!(d.round(), 2);
    assert_eq!(d.active().unwrap().target.task_id.as_str(), "new.a");
}

#[test]
fn incomplete_unknown_or_unqualified_owner_inputs_refuse_transactionally() {
    let i = intent("app.a", "a");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
    for fault in 0..9 {
        let mut ops = [opportunity(&i, 0)];
        let mut snap = snapshot(0, &apps, &[], None, &[]);
        match fault {
            0 => snap.admitted_population_complete = false,
            1 => snap.waiting_population_complete = false,
            2 => snap.blocking_continuations_complete = false,
            3 => snap.owner_identity = "wrong",
            4 => ops[0].eligibility = SchedulerProtectedEligibility::Unknown,
            5 => ops[0].feasibility = SchedulerProtectedFeasibility::Unknown,
            6 => ops[0].generation = 0,
            7 => ops[0].continuously_eligible_since_us = 1,
            _ => ops[0].episode_contract_identity = "",
        }
        snap.opportunities = &ops;
        let before = s.clone();
        assert!(s
            .advance(&snap, SchedulerProtectedEvent::Observe, 4096)
            .is_err());
        assert_eq!(s, before);
    }
    let duplicate_apps = [app("app.a"), app("app.a")];
    assert!(s
        .advance(
            &snapshot(0, &duplicate_apps, &ops, None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .is_err());
    let duplicated = [opportunity(&i, 0), opportunity(&i, 0)];
    assert!(s
        .advance(
            &snapshot(0, &apps, &duplicated, None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .is_err());
    let mut ineligible = opportunity(&i, 0);
    ineligible.feasibility = SchedulerProtectedFeasibility::Infeasible;
    let (_, d) = observe(&s, &snapshot(10, &apps, &[ineligible], None, &[]));
    assert!(d.active().is_none());
}

#[test]
fn hard_population_and_exact_operation_budget_bounds_return_no_transition() {
    let i = intent("app.a", "a");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let snap = snapshot(0, &apps, &ops, None, &[]);
    let (_, expected) = observe(&s, &snap);
    let exact = expected.work_units();
    assert_eq!(
        s.advance(&snap, SchedulerProtectedEvent::Observe, exact)
            .unwrap()
            .1,
        expected
    );
    assert_eq!(
        s.advance(&snap, SchedulerProtectedEvent::Observe, exact - 1)
            .unwrap_err(),
        SchedulerProtectionRefusal::BudgetExhausted
    );
    assert_eq!(
        s.advance(&snap, SchedulerProtectedEvent::Observe, 4097)
            .unwrap_err(),
        SchedulerProtectionRefusal::WorkLimitExceeded
    );
    let too_many: Vec<_> = (0..17).map(|_| opportunity(&i, 0)).collect();
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &too_many, None, &[]),
            SchedulerProtectedEvent::Observe,
            0
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::WorkLimitExceeded
    );
    let many_apps: Vec<_> = (0..5).map(|n| app(&format!("app.{n}"))).collect();
    assert_eq!(
        s.advance(
            &snapshot(0, &many_apps, &[], None, &[]),
            SchedulerProtectedEvent::Observe,
            0
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::WorkLimitExceeded
    );
    let (s, _) = observe(&s, &snapshot(5, &apps, &ops, None, &[]));
    assert_eq!(
        s.advance(
            &snapshot(4, &apps, &ops, None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ClockRegression
    );
}

#[test]
fn continuous_generation_can_advance_only_after_owner_observes_eligibility_loss() {
    let i = intent("app.a", "a");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 100, 1).unwrap();
    let (s, _) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let mut changed = opportunity(&i, 10);
    changed.generation = 2;
    assert_eq!(
        s.advance(
            &snapshot(10, &apps, &[changed], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ContinuityChanged
    );
    let (s, _) = observe(&s, &snapshot(10, &apps, &[], None, &[]));
    let mut changed = opportunity(&i, 10);
    changed.generation = 2;
    let (s, _) = observe(&s, &snapshot(10, &apps, &[changed], None, &[]));
    assert_eq!(
        s.advance(
            &snapshot(10, &apps, &ops, None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ContinuityChanged
    );
}

// Independent closed-form oracle: finite continuously eligible population,
// total G at round entry, then serial bounded episodes; no policy helpers.
#[test]
fn finite_cohort_activation_order_and_bound_match_independent_arithmetic_oracle() {
    for n in 1..=4 {
        for seed in 0..32_u64 {
            let gap = seed % 7;
            let duration: Vec<_> = (0..n)
                .map(|k| 1 + (seed * 11 + k as u64 * 7) % 23)
                .collect();
            let intents: Vec<_> = (0..n)
                .map(|k| intent(&format!("app.{k}"), &format!("task.{k}")))
                .collect();
            let apps: Vec<_> = (0..n).map(|k| app(&format!("app.{k}"))).collect();
            let mut s = SchedulerProgressProtection::new("owner.1", 0, gap, 2).unwrap();
            let mut now = gap;
            for k in 0..n {
                let ops: Vec<_> = intents[k..].iter().map(|i| opportunity(i, 0)).collect();
                let (next, d) = observe(&s, &snapshot(now, &apps, &ops, None, &[]));
                s = next;
                let a = d.active().unwrap();
                assert_eq!(a.target.task_id.as_str(), format!("task.{k}"));
                let expected = gap + duration[..k].iter().sum::<u64>();
                assert_eq!(now, expected);
                assert_eq!(d.discretionary_used_us(), gap);
                assert_eq!(d.round(), 1);
                let (next, d) = running(&s, &d, &snapshot(now, &apps, &ops, None, &[]));
                s = next;
                now += duration[k];
                let remaining: Vec<_> =
                    intents[k + 1..].iter().map(|i| opportunity(i, 0)).collect();
                let (next, d) = finish(
                    &s,
                    &d,
                    &snapshot(now, &apps, &remaining, None, &[]),
                    SchedulerProtectedOutcome::AcceptedOutput,
                );
                s = next;
                assert_eq!(
                    d.finished(),
                    Some(SchedulerProtectedOutcome::AcceptedOutput)
                );
            }
            assert_eq!(now, gap + duration.iter().sum::<u64>());
        }
    }
}

#[test]
fn snapshot_permutations_retain_exact_decision_state_and_bounded_work() {
    let a = intent("app.a", "a");
    let b = intent("app.b", "b");
    let c = intent("app.c", "c");
    let s = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
    let apps = [app("app.a"), app("app.b"), app("app.c")];
    let ops = [opportunity(&a, 0), opportunity(&b, 0), opportunity(&c, 0)];
    let expected = observe(&s, &snapshot(10, &apps, &ops, None, &[]));
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let aa: Vec<_> = order.iter().map(|k| apps[*k].clone()).collect();
        let oo: Vec<_> = order
            .iter()
            .map(|k| opportunity([&a, &b, &c][*k], 0))
            .collect();
        assert_eq!(observe(&s, &snapshot(10, &aa, &oo, None, &[])), expected);
    }
}

#[test]
fn oversized_whitespace_labels_refuse_before_unbounded_text_operations() {
    let oversized = " ".repeat(1_000_000);
    assert_eq!(
        SchedulerProgressProtection::new(&oversized, 0, 0, 1).unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let i = intent("app.a", "a");
    let apps = [app("app.a")];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let mut bad = opportunity(&i, 0);
    bad.episode_contract_identity = &oversized;
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &[bad], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let ops = [opportunity(&i, 0)];
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    let blocks = [SchedulerProtectedBlockingContinuation {
        intent: &i,
        generation: 1,
        envelope_identity: &oversized,
        full_growth_covered: true,
    }];
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &ops, Some(&t), &blocks),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
}

#[test]
fn task_identity_cannot_be_duplicated_or_moved_between_fairness_owners_or_nodes() {
    let i = intent("app.a", "same");
    let moved = intent("app.b", "same");
    let apps = [app("app.a"), app("app.b")];
    let s = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
    let duplicate = [opportunity(&i, 0), opportunity(&moved, 0)];
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &duplicate, None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let (s, _) = observe(&s, &snapshot(0, &apps, &[opportunity(&i, 0)], None, &[]));
    let (s, _) = observe(&s, &snapshot(1, &apps, &[], None, &[]));
    for generation in [1, 2] {
        let mut op = opportunity(&moved, 1);
        op.generation = generation;
        assert_eq!(
            s.advance(
                &snapshot(1, &apps, &[op], None, &[]),
                SchedulerProtectedEvent::Observe,
                4096
            )
            .unwrap_err(),
            SchedulerProtectionRefusal::ContinuityChanged
        );
    }
    let mut raw = i.clone().into_inner();
    raw.node_id = SchedulerNodeId::parse("different.node").unwrap();
    let changed: ValidatedSchedulableTaskIntent = raw.try_into().unwrap();
    assert_eq!(
        s.advance(
            &snapshot(1, &apps, &[opportunity(&changed, 0)], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::ContinuityChanged
    );
    let (s, d) = observe(&s, &snapshot(10, &apps, &[opportunity(&i, 0)], None, &[]));
    let target = d.active().unwrap().target.clone();
    let (_, d) = observe(&s, &snapshot(10, &apps, &[], Some(&target), &[]));
    for different in [&moved, &changed] {
        assert_eq!(
            d.permission(
                "owner.1",
                d.revision(),
                different,
                1,
                SchedulerProtectedRelation::Nonconflicting
            ),
            SchedulerProtectedPermission::HoldNewAdmission
        );
    }
}

#[test]
fn finite_history_caps_survive_inactive_snapshots_and_full_maximum_fixture_fits_budget() {
    let apps: Vec<_> = (0..4).map(|n| app(&format!("app.{n}"))).collect();
    let intents: Vec<_> = (0..16)
        .map(|n| intent(&format!("app.{}", n % 4), &format!("task.{n:02}")))
        .collect();
    let ops: Vec<_> = intents.iter().map(|i| opportunity(i, 0)).collect();
    let s = SchedulerProgressProtection::new("owner.1", 0, 100, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    assert!(d.work_units() <= 4096);
    let (s, _) = observe(&s, &snapshot(1, &apps, &[], None, &[]));
    let new = intent("app.0", "task.seventeenth");
    assert_eq!(
        s.advance(
            &snapshot(1, &apps, &[opportunity(&new, 1)], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::WorkLimitExceeded
    );
    assert_eq!(
        s.advance(
            &snapshot(1, &[app("app.fifth")], &[], None, &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::WorkLimitExceeded
    );
    let (s, d) = observe(&s, &snapshot(100, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    let blockers: Vec<_> = intents
        .iter()
        .map(|i| SchedulerProtectedBlockingContinuation {
            intent: i,
            generation: 1,
            envelope_identity: "bounded.full.growth",
            full_growth_covered: true,
        })
        .collect();
    let snap = snapshot(100, &apps, &ops, Some(&t), &blockers);
    let (_, d) = observe(&s, &snap);
    assert!(d.work_units() <= 4096);
    assert_eq!(
        s.advance(&snap, SchedulerProtectedEvent::Observe, d.work_units())
            .unwrap()
            .1,
        d
    );
    assert_eq!(
        s.advance(&snap, SchedulerProtectedEvent::Observe, d.work_units() - 1)
            .unwrap_err(),
        SchedulerProtectionRefusal::BudgetExhausted
    );
}

#[test]
fn withdrawal_and_old_turn_events_require_real_release_and_do_not_clear_a_new_turn() {
    let i = intent("app.a", "a");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let first = d.active().unwrap().clone();
    let (s, d) = finish(
        &s,
        &d,
        &snapshot(1, &apps, &[], None, &[]),
        SchedulerProtectedOutcome::Withdrawn,
    );
    assert!(d.active().is_none());
    assert_eq!(d.finished(), Some(SchedulerProtectedOutcome::Withdrawn));
    let mut next_op = opportunity(&i, 2);
    next_op.generation = 2;
    let (s, d) = observe(&s, &snapshot(2, &apps, &[next_op], None, &[]));
    assert!(d.active().unwrap().turn_id > first.turn_id);
    let mut next_op = opportunity(&i, 2);
    next_op.generation = 2;
    assert_eq!(
        s.advance(
            &snapshot(3, &apps, &[next_op], None, &[]),
            SchedulerProtectedEvent::Finished {
                turn_id: first.turn_id,
                attempt: 1,
                outcome: SchedulerProtectedOutcome::AcceptedOutput,
                physical_release_acknowledged: true
            },
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::StaleEvent
    );
}

#[test]
fn wrong_target_frames_unqualified_growth_and_duplicate_blockers_refuse() {
    let i = intent("app.a", "a");
    let other = intent("app.b", "b");
    let moved = intent("app.b", "a");
    let apps = [app("app.a"), app("app.b")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    let bad = [SchedulerProtectedBlockingContinuation {
        intent: &other,
        generation: 1,
        envelope_identity: "growth",
        full_growth_covered: false,
    }];
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &ops, Some(&t), &bad),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let blocks = [SchedulerProtectedBlockingContinuation {
        intent: &other,
        generation: 1,
        envelope_identity: "growth",
        full_growth_covered: true,
    }];
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &ops, None, &blocks),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let mut wrong = t.clone();
    wrong.episode_contract_identity = "different.envelope".into();
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &ops, Some(&wrong), &[]),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    let dup = [
        SchedulerProtectedBlockingContinuation {
            intent: &i,
            generation: 1,
            envelope_identity: "growth",
            full_growth_covered: true,
        },
        SchedulerProtectedBlockingContinuation {
            intent: &moved,
            generation: 1,
            envelope_identity: "growth",
            full_growth_covered: true,
        },
    ];
    assert_eq!(
        s.advance(
            &snapshot(0, &apps, &ops, Some(&t), &dup),
            SchedulerProtectedEvent::Observe,
            4096
        )
        .unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    assert_eq!(
        SchedulerProgressProtection::new("owner.1", 0, 0, 0).unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
    assert_eq!(
        SchedulerProgressProtection::new("owner.1", 0, 0, 9).unwrap_err(),
        SchedulerProtectionRefusal::InvalidContract
    );
}

#[test]
#[ignore = "pure bounded controller cost probe; run separately without concurrent compilation"]
fn protected_progress_dispatch_cost_probe() {
    let apps: Vec<_> = (0..4).map(|n| app(&format!("app.{n}"))).collect();
    let intents: Vec<_> = (0..16)
        .map(|n| intent(&format!("app.{}", n % 4), &format!("task.{n:02}")))
        .collect();
    let ops: Vec<_> = intents.iter().map(|i| opportunity(i, 0)).collect();
    let s = SchedulerProgressProtection::new("owner.1", 0, 0, 2).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    let t = d.active().unwrap().target.clone();
    let blockers: Vec<_> = intents
        .iter()
        .map(|i| SchedulerProtectedBlockingContinuation {
            intent: i,
            generation: 1,
            envelope_identity: "bounded.full.growth",
            full_growth_covered: true,
        })
        .collect();
    let snap = snapshot(0, &apps, &ops, Some(&t), &blockers);
    let (s, baseline) = observe(&s, &snap);
    let mut times = vec![];
    for n in 0..216 {
        let start = std::time::Instant::now();
        let result = std::hint::black_box(s.advance(&snap, SchedulerProtectedEvent::Observe, 4096));
        let elapsed = start.elapsed().as_nanos();
        let (_, d) = result.unwrap();
        assert_eq!(d.work_units(), baseline.work_units());
        if n >= 16 {
            times.push(elapsed);
        }
    }
    times.sort_unstable();
    println!(
        "{}",
        serde_json::json!({"scope":"pure controller 4apps/16opportunities/16exactblockingcontinuations; excludes owner capture/physical revalidation/admission/runtime and completion search","samples":times.len(),"work_units":baseline.work_units(),"p50_ns":times[100],"p95_ns":times[190],"max_ns":times[199]})
    );
}

#[test]
fn wakeup_contract_exposes_late_observation_instead_of_claiming_an_exact_deadline() {
    let i = intent("app.a", "cold");
    let apps = [app("app.a")];
    let ops = [opportunity(&i, 0)];
    let s = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
    let (s, d) = observe(&s, &snapshot(0, &apps, &ops, None, &[]));
    assert_eq!(d.next_wake_after_us(), Some(10));
    let (s, d) = observe(&s, &snapshot(6, &apps, &ops, None, &[]));
    assert_eq!(d.next_wake_after_us(), Some(4));
    let (_, d) = observe(&s, &snapshot(25, &apps, &ops, None, &[]));
    assert_eq!(d.discretionary_used_us(), 25);
    assert_eq!(d.next_wake_after_us(), None);
    assert!(d.active().is_some());
    // No elapsed time, physical release or activation at t=10 is manufactured.
    let empty = SchedulerProgressProtection::new("owner.1", 0, 10, 1).unwrap();
    let (_, d) = observe(&empty, &snapshot(0, &apps, &[], None, &[]));
    assert_eq!(d.next_wake_after_us(), None);
}
