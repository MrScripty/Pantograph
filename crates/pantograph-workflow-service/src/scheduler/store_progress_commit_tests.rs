use super::*;
use crate::workflow::WorkflowExecutionSessionRunRequest;
use pantograph_scheduler::{
    SchedulableTaskIntent, SchedulerProtectedOutcome, SchedulerProtectedPhase,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
struct Clock(AtomicU64);
impl WorkflowQueueProgressClock for Clock {
    fn now_us(&self) -> Result<u64, WorkflowServiceError> {
        Ok(self.0.load(Ordering::SeqCst))
    }
}
#[derive(Debug)]
struct Provider {
    mode: AtomicU64,
    finished: AtomicU64,
    clock: Arc<Clock>,
}
impl WorkflowQueueProgressProvider for Provider {
    fn capture(
        &self,
        v: &WorkflowQueueProgressView,
    ) -> Result<WorkflowQueueProgressFacts, WorkflowServiceError> {
        let mode = self.mode.load(Ordering::SeqCst);
        let mut ops = Vec::new();
        let mut bindings = Vec::new();
        for run in &v.runs {
            let intent = intent(&run.workflow_id, &run.workflow_run_id);
            if !run.active || self.finished.load(Ordering::SeqCst) == 0 {
                ops.push(WorkflowQueueProgressOpportunity {
                    intent: intent.clone(),
                    generation: 1,
                    since_us: 0,
                    eligibility: if mode == 1 {
                        SchedulerProtectedEligibility::Unknown
                    } else {
                        SchedulerProtectedEligibility::Ready
                    },
                    feasibility: if mode == 7 {
                        SchedulerProtectedFeasibility::Infeasible
                    } else {
                        SchedulerProtectedFeasibility::IndividuallyFeasible
                    },
                    episode_contract: "controlled.graph-and-full-envelope.v1".into(),
                });
            }
            if !run.active {
                let mut b = WorkflowQueueProgressBinding {
                    session_id: run.session_id.clone(),
                    intent,
                    generation: 1,
                    whole_run_admission_covered: true,
                    request_fingerprint: run.request_fingerprint.clone().unwrap(),
                    relation: if mode == 4 {
                        SchedulerProtectedRelation::RequiredBlockingContinuation
                    } else if mode == 8 && run.workflow_run_id == "hot" {
                        SchedulerProtectedRelation::Nonconflicting
                    } else {
                        SchedulerProtectedRelation::Conflicting
                    },
                };
                if mode == 3 {
                    b.request_fingerprint = "stale.request".into();
                }
                if mode == 9 {
                    let mut raw = b.intent.into_inner();
                    raw.model_ref.selected_artifact_id = Some("wrong.artifact".into());
                    b.intent = raw.try_into().unwrap();
                }
                bindings.push(b);
            }
        }
        if mode == 10 {
            bindings.pop();
        }
        if mode == 11 {
            ops.push(WorkflowQueueProgressOpportunity {
                intent: intent("foreign", "phantom"),
                generation: 1,
                since_us: 0,
                eligibility: SchedulerProtectedEligibility::Ready,
                feasibility: SchedulerProtectedFeasibility::IndividuallyFeasible,
                episode_contract: "fake".into(),
            });
        }
        if mode == 6 {
            self.clock.0.store(v.now_us + 20, Ordering::SeqCst);
        }
        Ok(WorkflowQueueProgressFacts {
            epoch: if mode == 2 {
                v.epoch.saturating_sub(1)
            } else {
                v.epoch
            },
            valid_until_us: if mode == 5 {
                v.now_us.saturating_sub(1)
            } else {
                v.now_us + 100_000
            },
            owner: "controlled.owner".into(),
            admitted_apps: vec!["app.a".parse().unwrap(), "app.z".parse().unwrap()],
            population_complete: true,
            opportunities: ops,
            bindings,
            target: v.protected_target.clone(),
            blockers_complete: true,
            blockers: vec![],
        })
    }
}
fn intent(workflow: &str, run: &str) -> ValidatedSchedulableTaskIntent {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../pantograph-scheduler/tests/fixtures/dispatch_selection_request_valid.json"
    ))
    .unwrap();
    let mut dto = v["task_intent"].clone();
    dto["workflow_id"] = workflow.into();
    dto["workflow_run_id"] = run.into();
    dto["fairness_key"] = if run == "cold" { "app.a" } else { "app.z" }.into();
    let raw: SchedulableTaskIntent = serde_json::from_value(dto).unwrap();
    raw.try_into().unwrap()
}
fn request(priority: i32) -> WorkflowExecutionSessionRunRequest {
    WorkflowExecutionSessionRunRequest {
        session_id: "ignored".into(),
        workflow_semantic_version: "1.2.3".into(),
        inputs: vec![],
        output_targets: None,
        override_selection: None,
        timeout_ms: None,
        priority: Some(priority),
    }
}
fn fixture(
    split: bool,
    gap: u64,
) -> (
    WorkflowExecutionSessionStore,
    String,
    String,
    Arc<Provider>,
    Arc<Clock>,
) {
    let mut store = WorkflowExecutionSessionStore::new(4, 4);
    let cold = store
        .create_session("workflow.cold".into(), None, None, vec![], vec![], false)
        .unwrap();
    let hot = if split {
        store
            .create_session("workflow.hot".into(), None, None, vec![], vec![], false)
            .unwrap()
    } else {
        cold.clone()
    };
    store
        .enqueue_run_with_id(&cold, &request(-100), "cold".into())
        .unwrap();
    store
        .enqueue_run_with_id(&hot, &request(1000), "hot".into())
        .unwrap();
    store.mark_runtime_loaded(&cold, true).unwrap();
    store.mark_runtime_loaded(&hot, true).unwrap();
    let clock = Arc::new(Clock::default());
    let provider = Arc::new(Provider {
        mode: AtomicU64::new(0),
        finished: AtomicU64::new(0),
        clock: clock.clone(),
    });
    store
        .enable_queue_progress(
            "controlled.owner",
            gap,
            2,
            4096,
            provider.clone(),
            clock.clone(),
        )
        .unwrap();
    (store, cold, hot, provider, clock)
}
fn prime(s: &mut WorkflowExecutionSessionStore) {
    s.observe_queue_progress_event(SchedulerProtectedEvent::Observe)
        .unwrap();
}

#[test]
fn actual_queue_commit_overrides_foreground_and_keeps_target_caller() {
    let (mut s, c, h, _, clock) = fixture(false, 10);
    prime(&mut s);
    clock.0.store(10, Ordering::SeqCst);
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_none());
    assert!(s.active[&c].active_run.is_none());
    let out = s.begin_queued_run(&c, "cold").unwrap().unwrap();
    assert_eq!(out.queued.workflow_run_id, "cold");
    assert_eq!(
        out.scheduler_decision_reason,
        crate::scheduler::WorkflowSchedulerDecisionReason::StarvationProtection
    );
    assert_eq!(s.active[&c].queue[0].workflow_run_id, "hot");
    assert_eq!(
        s.progress_commit
            .as_ref()
            .unwrap()
            .decision
            .as_ref()
            .unwrap()
            .active()
            .unwrap()
            .phase,
        SchedulerProtectedPhase::Draining
    );
}
#[test]
fn global_owner_protects_across_sessions_without_boost_reset() {
    let (mut s, c, h, _, clock) = fixture(true, 10);
    prime(&mut s);
    clock.0.store(6, Ordering::SeqCst);
    prime(&mut s);
    s.reprioritize_queue_item(&h, "hot", i32::MAX).unwrap();
    assert_eq!(s.queue_progress_retry_after(), Duration::from_micros(4));
    clock.0.store(30, Ordering::SeqCst);
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_none());
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_none());
    assert_eq!(
        s.progress_commit
            .as_ref()
            .unwrap()
            .decision
            .as_ref()
            .unwrap()
            .discretionary_used_us(),
        30
    );
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_some());
}
#[test]
fn stale_incomplete_unknown_or_mismatched_provider_never_dequeues() {
    for mode in [1, 2, 3, 5, 9, 10, 11] {
        let (mut s, c, _, p, clock) = fixture(false, 10);
        prime(&mut s);
        clock.0.store(20, Ordering::SeqCst);
        let before = s.progress_commit.as_ref().unwrap().controller.clone();
        p.mode.store(mode, Ordering::SeqCst);
        assert!(
            s.begin_queued_run(&c, "cold").unwrap().is_none(),
            "mode {mode}"
        );
        assert_eq!(s.progress_commit.as_ref().unwrap().controller, before);
        assert_eq!(s.active[&c].queue.len(), 2);
        assert!(s.active[&c].active_run.is_none());
    }
}
#[test]
fn infeasible_work_and_continuation_labels_cannot_admit_new_run() {
    for mode in [4, 7] {
        let (mut s, c, _, p, _) = fixture(false, 0);
        p.mode.store(mode, Ordering::SeqCst);
        assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
        assert!(s.active[&c].active_run.is_none());
    }
}
#[test]
fn capture_elapsed_time_cannot_reuse_discretionary_permission() {
    let (mut s, c, _, p, _) = fixture(false, 10);
    prime(&mut s);
    p.mode.store(6, Ordering::SeqCst);
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
    let d = s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap();
    assert_eq!(d.discretionary_used_us(), 20);
    assert!(d.active().is_some());
}
#[test]
fn nonconflicting_work_may_commit_after_exact_target_refresh() {
    let (mut s, _, h, p, _) = fixture(true, 0);
    prime(&mut s);
    p.mode.store(8, Ordering::SeqCst);
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_some());
}
#[test]
fn budget_refusal_and_reinstallation_preserve_the_existing_controller() {
    let (mut s, c, _, p, clock) = fixture(false, 10);
    prime(&mut s);
    let before = s.progress_commit.as_ref().unwrap().controller.clone();
    assert!(s
        .enable_queue_progress("controlled.owner", 0, 2, 4096, p, clock)
        .is_err());
    s.progress_commit.as_mut().unwrap().budget = 0;
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
    assert_eq!(s.progress_commit.as_ref().unwrap().controller, before);
}
#[test]
fn active_and_cancelled_run_membership_errors_are_preserved() {
    let (mut s, c, _, _, _) = fixture(false, 0);
    prime(&mut s);
    s.begin_queued_run(&c, "cold").unwrap().unwrap();
    assert!(matches!(
        s.begin_queued_run(&c, "missing"),
        Err(WorkflowServiceError::QueueItemNotFound(_))
    ));
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
    s.cancel_queue_item(&c, "hot").unwrap();
    assert!(matches!(
        s.begin_queued_run(&c, "hot"),
        Err(WorkflowServiceError::QueueItemNotFound(_))
    ));
}
#[test]
fn logical_queue_finish_does_not_acknowledge_physical_release() {
    let (mut s, c, _, p, _) = fixture(false, 0);
    prime(&mut s);
    s.begin_queued_run(&c, "cold").unwrap().unwrap();
    let a = s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap()
        .active()
        .unwrap()
        .clone();
    s.observe_queue_progress_event(SchedulerProtectedEvent::Started {
        turn_id: a.turn_id,
        attempt: a.attempt,
    })
    .unwrap();
    s.finish_run(&c, "cold").unwrap();
    assert!(s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap()
        .active()
        .is_some());
    p.finished.store(1, Ordering::SeqCst);
    assert!(s
        .observe_queue_progress_event(SchedulerProtectedEvent::Finished {
            turn_id: a.turn_id,
            attempt: a.attempt,
            outcome: SchedulerProtectedOutcome::AcceptedOutput,
            physical_release_acknowledged: false
        })
        .is_err());
}
#[test]
fn actual_owner_release_advances_turn_and_stale_event_cannot_clear_next() {
    let (mut s, c, h, p, _) = fixture(true, 0);
    prime(&mut s);
    s.begin_queued_run(&c, "cold").unwrap().unwrap();
    let a = s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap()
        .active()
        .unwrap()
        .clone();
    s.observe_queue_progress_event(SchedulerProtectedEvent::Started {
        turn_id: a.turn_id,
        attempt: a.attempt,
    })
    .unwrap();
    p.finished.store(1, Ordering::SeqCst);
    s.observe_queue_progress_event(SchedulerProtectedEvent::Finished {
        turn_id: a.turn_id,
        attempt: a.attempt,
        outcome: SchedulerProtectedOutcome::AcceptedOutput,
        physical_release_acknowledged: true,
    })
    .unwrap();
    let next = s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap()
        .active()
        .unwrap()
        .clone();
    assert_eq!(next.target.workflow_run_id.as_str(), "hot");
    assert!(s
        .observe_queue_progress_event(SchedulerProtectedEvent::Finished {
            turn_id: a.turn_id,
            attempt: a.attempt,
            outcome: SchedulerProtectedOutcome::AcceptedOutput,
            physical_release_acknowledged: true
        })
        .is_err());
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_some());
}
#[test]
fn cancelled_protected_caller_requires_explicit_withdrawal() {
    let (mut s, c, h, _, _) = fixture(true, 0);
    prime(&mut s);
    let a = s
        .progress_commit
        .as_ref()
        .unwrap()
        .decision
        .as_ref()
        .unwrap()
        .active()
        .unwrap()
        .clone();
    s.cancel_queue_item(&c, "cold").unwrap();
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_none());
    s.observe_queue_progress_event(SchedulerProtectedEvent::Finished {
        turn_id: a.turn_id,
        attempt: a.attempt,
        outcome: SchedulerProtectedOutcome::Withdrawn,
        physical_release_acknowledged: true,
    })
    .unwrap();
    assert!(s.begin_queued_run(&h, "hot").unwrap().is_some());
}
#[test]
fn request_fingerprint_changes_for_semantic_input_and_override_changes() {
    let (mut s, c, _, _, _) = fixture(false, 10);
    let first = s
        .progress_view(0)
        .unwrap()
        .runs
        .iter()
        .find(|r| r.workflow_run_id == "cold")
        .unwrap()
        .request_fingerprint
        .clone();
    s.active
        .get_mut(&c)
        .unwrap()
        .queue
        .iter_mut()
        .find(|r| r.workflow_run_id == "cold")
        .unwrap()
        .workflow_semantic_version = "2.0.0".into();
    let second = s
        .progress_view(0)
        .unwrap()
        .runs
        .iter()
        .find(|r| r.workflow_run_id == "cold")
        .unwrap()
        .request_fingerprint
        .clone();
    assert_ne!(first, second);
    let hot_before = s
        .progress_view(0)
        .unwrap()
        .runs
        .iter()
        .find(|r| r.workflow_run_id == "hot")
        .unwrap()
        .request_fingerprint
        .clone();
    s.active
        .get_mut(&c)
        .unwrap()
        .queue
        .iter_mut()
        .find(|r| r.workflow_run_id == "hot")
        .unwrap()
        .inputs
        .push(crate::workflow::WorkflowPortBinding {
            node_id: "n".into(),
            port_id: "p".into(),
            value: serde_json::json!({"changed":true}),
        });
    assert_ne!(
        hot_before,
        s.progress_view(0)
            .unwrap()
            .runs
            .iter()
            .find(|r| r.workflow_run_id == "hot")
            .unwrap()
            .request_fingerprint
            .clone()
    );
    s.active
        .get_mut(&c)
        .unwrap()
        .queue
        .iter_mut()
        .find(|r| r.workflow_run_id == "cold")
        .unwrap()
        .override_selection = Some(crate::technical_fit::WorkflowTechnicalFitOverride {
        model_id: Some("different.model".into()),
        ..Default::default()
    });
    assert_ne!(
        second,
        s.progress_view(0)
            .unwrap()
            .runs
            .iter()
            .find(|r| r.workflow_run_id == "cold")
            .unwrap()
            .request_fingerprint
            .clone()
    );
}
#[test]
fn oversized_and_deep_requests_refuse_before_provider_capture() {
    let (mut s, c, _, _, _) = fixture(false, 10);
    let mut value = serde_json::Value::Null;
    for _ in 0..18 {
        value = serde_json::json!([value]);
    }
    s.active.get_mut(&c).unwrap().queue[0]
        .inputs
        .push(crate::workflow::WorkflowPortBinding {
            node_id: "n".into(),
            port_id: "p".into(),
            value,
        });
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
    assert_eq!(s.active[&c].queue.len(), 2);
}
#[tokio::test]
async fn actual_polling_worker_uses_owner_time_and_target_caller() {
    let (mut s, c, _, _, clock) = fixture(false, 10);
    prime(&mut s);
    clock.0.store(10, Ordering::SeqCst);
    let store = Arc::new(std::sync::Mutex::new(s));
    let out = tokio::time::timeout(
        Duration::from_secs(1),
        crate::scheduler::WorkflowSchedulerQueueWorker::admit_queued_run(
            crate::scheduler::WorkflowSchedulerQueueAdmissionCommand::new(
                store.clone(),
                c.clone(),
                "cold",
            ),
        ),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(out.queued.workflow_run_id, "cold");
    assert_eq!(
        store.lock().unwrap().active[&c].queue[0].workflow_run_id,
        "hot"
    );
}

#[derive(Debug)]
struct ScriptClock(std::sync::Mutex<(std::collections::VecDeque<u64>, u64)>);
impl WorkflowQueueProgressClock for ScriptClock {
    fn now_us(&self) -> Result<u64, WorkflowServiceError> {
        let mut s = self.0.lock().unwrap();
        if let Some(v) = s.0.pop_front() {
            s.1 = v;
        }
        Ok(s.1)
    }
}
#[test]
fn final_commit_deadline_crossing_holds_after_ordinary_selection() {
    let (mut s, c, _, p, _) = fixture(false, 10);
    // Separate controller only in this fresh fixture, before any observations.
    s.progress_commit = None;
    let clock = Arc::new(ScriptClock(std::sync::Mutex::new((
        [0, 0, 0, 11].into(),
        11,
    ))));
    s.enable_queue_progress("controlled.owner", 10, 2, 4096, p, clock)
        .unwrap();
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    assert_eq!(s.active[&c].queue.len(), 2);
    assert!(s.active[&c].active_run.is_none());
    assert_eq!(s.queue_progress_retry_after(), Duration::ZERO);
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_some());
}
#[test]
fn final_commit_expired_owner_frame_holds_and_can_recover() {
    let (mut s, c, _, p, _) = fixture(false, 1_000_000);
    s.progress_commit = None;
    let clock = Arc::new(ScriptClock(std::sync::Mutex::new((
        [0, 0, 0, 100_001].into(),
        100_001,
    ))));
    s.enable_queue_progress("controlled.owner", 1_000_000, 2, 4096, p, clock)
        .unwrap();
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    assert!(s.active[&c].active_run.is_none());
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_some());
}
#[test]
fn unknown_owner_evidence_does_not_drop_the_real_polling_caller() {
    // Synchronous queue seam used by the polling caller keeps both items queued.
    let (mut s, c, _, p, _) = fixture(false, 10);
    p.mode.store(1, Ordering::SeqCst);
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    p.mode.store(0, Ordering::SeqCst);
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_some());
}
#[test]
fn queue_population_limits_are_refusal_and_default_is_unaffected() {
    let (mut s, c, _, _, _) = fixture(false, 10);
    for n in 0..15 {
        s.enqueue_run_with_id(&c, &request(0), format!("extra.{n}"))
            .unwrap();
    }
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    assert_eq!(s.active[&c].queue.len(), 17);
    let mut default = WorkflowExecutionSessionStore::new(4, 4);
    let id = default
        .create_session("workflow.default".into(), None, None, vec![], vec![], false)
        .unwrap();
    default
        .enqueue_run_with_id(&id, &request(-100), "cold".into())
        .unwrap();
    default
        .enqueue_run_with_id(&id, &request(1000), "hot".into())
        .unwrap();
    assert!(default.begin_queued_run(&id, "cold").unwrap().is_none());
    assert_eq!(
        default
            .begin_queued_run(&id, "hot")
            .unwrap()
            .unwrap()
            .queued
            .workflow_run_id,
        "hot"
    );
}
#[test]
#[ignore = "release dispatch cost probe; excludes native provider/model/runtime work"]
fn queue_commit_dispatch_cost_probe() {
    let mut samples = Vec::new();
    for iteration in 0..216 {
        let (mut s, c, _, _, _) = fixture(false, 10);
        let start = Instant::now();
        let out = s.begin_queued_run(&c, "hot").unwrap();
        let elapsed = start.elapsed().as_nanos();
        assert!(out.is_some());
        if iteration >= 16 {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    println!(
        "{}",
        serde_json::json!({"samples":samples.len(),"p50_ns":samples[100],"p95_ns":samples[190],"max_ns":samples[199],
        "scope":"actual begin_queued_run: bounded queue projection/request hashing, controlled provider capture, protection advance, original priority ranking, final owner time check, dequeue/active run; excludes native physical admission/model/runtime"})
    );
}

#[test]
fn legacy_public_reason_enum_remains_exhaustively_matchable() {
    use crate::scheduler::WorkflowSchedulerDecisionReason::*;
    fn legacy(r: crate::scheduler::WorkflowSchedulerDecisionReason) -> &'static str {
        match r {
            SchedulerSnapshotFailed => "scheduler_snapshot_failed",
            MatchedPendingItem => "matched_pending_item",
            MatchedRunningItem => "matched_running_item",
            SessionRunningWithBacklog => "session_running_with_backlog",
            SessionRunning => "session_running",
            SessionQueued => "session_queued",
            IdleLoaded => "idle_loaded",
            IdleUnloaded => "idle_unloaded",
            HighestPriorityFirst => "highest_priority_first",
            FifoPriorityTieBreak => "fifo_priority_tie_break",
            WaitingForHigherPriority => "waiting_for_higher_priority",
            WaitingForRuntimeCapacity => "waiting_for_runtime_capacity",
            WaitingForRuntimeAdmission => "waiting_for_runtime_admission",
            StarvationProtection => "starvation_protection",
            WarmSessionReused => "warm_session_reused",
            RuntimeReloadRequired => "runtime_reload_required",
            ColdStartRequired => "cold_start_required",
        }
    }
    assert_eq!(legacy(StarvationProtection), StarvationProtection.as_str());
}
#[test]
fn refused_evidence_uses_bounded_retry_instead_of_overdue_timer_spin() {
    let (mut s, c, _, p, clock) = fixture(false, 10);
    prime(&mut s);
    clock.0.store(20, Ordering::SeqCst);
    p.mode.store(1, Ordering::SeqCst);
    assert!(s.begin_queued_run(&c, "cold").unwrap().is_none());
    assert_eq!(s.queue_progress_retry_after(), Duration::from_millis(10));
}

#[test]
fn real_monotonic_clock_is_used_at_actual_queue_commit() {
    let (mut s, c, _, p, _) = fixture(false, 1_000_000);
    s.progress_commit = None;
    s.enable_queue_progress(
        "controlled.owner",
        1_000_000,
        2,
        4096,
        p,
        Arc::new(WorkflowQueueMonotonicClock::new()),
    )
    .unwrap();
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_some());
}
#[test]
fn oversized_legacy_metadata_is_refused_before_capacity_or_ranking_clones() {
    let (mut s, c, _, _, _) = fixture(false, 10);
    s.active.get_mut(&c).unwrap().required_models = vec!["m".into(); 33];
    assert!(s.begin_queued_run(&c, "hot").unwrap().is_none());
    assert_eq!(s.active[&c].queue.len(), 2);
}
