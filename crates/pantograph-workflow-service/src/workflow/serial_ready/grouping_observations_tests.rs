use super::*;
use crate::workflow::serial_ready::WorkflowSerialReadyPair;
use pantograph_runtime_host_contracts::RuntimeHostExecutionInputValue;
use std::sync::atomic::Ordering;

#[test]
fn phase_aggregates_accept_zero_and_add_exact_checked_nanoseconds() {
    let mut aggregate = GroupingPhaseAggregate::default();
    aggregate.record(Some(Duration::ZERO));
    aggregate.record(Some(Duration::from_nanos(17)));
    aggregate.record(Some(Duration::from_nanos(23)));
    assert_eq!(
        aggregate,
        GroupingPhaseAggregate::Available {
            calls: 3,
            elapsed_ns: 40,
        }
    );
}

#[test]
fn clock_duration_and_counter_failures_are_absorbing_without_partial_totals() {
    for (mut aggregate, elapsed, reason) in [
        (
            GroupingPhaseAggregate::default(),
            None,
            GroupingObservationUnavailable::InvalidClock,
        ),
        (
            GroupingPhaseAggregate::default(),
            Some(Duration::from_secs(u64::MAX)),
            GroupingObservationUnavailable::DurationOverflow,
        ),
        (
            GroupingPhaseAggregate::Available {
                calls: u64::MAX,
                elapsed_ns: 7,
            },
            Some(Duration::ZERO),
            GroupingObservationUnavailable::CounterOverflow,
        ),
        (
            GroupingPhaseAggregate::Available {
                calls: 3,
                elapsed_ns: u64::MAX,
            },
            Some(Duration::from_nanos(1)),
            GroupingObservationUnavailable::CounterOverflow,
        ),
    ] {
        aggregate.record(elapsed);
        assert_eq!(aggregate, GroupingPhaseAggregate::Unavailable { reason });
        aggregate.record(Some(Duration::from_nanos(1)));
        assert_eq!(aggregate, GroupingPhaseAggregate::Unavailable { reason });
        let diagnostic = serde_json::to_value(aggregate).unwrap();
        assert!(diagnostic.get("calls").is_none());
        assert!(diagnostic.get("elapsed_ns").is_none());
    }
}

#[test]
fn diagnostic_scopes_exist_only_for_enabled_service_lifetimes() {
    assert!(WorkloadKeyGroup::new(false).observation_scope.is_none());
    let first = WorkloadKeyGroup::new(true);
    let second = WorkloadKeyGroup::new(true);
    assert!(first.observation_scope.is_some());
    assert_ne!(first.observation_scope, second.observation_scope);
    let diagnostic = GroupingObservationSnapshot {
        scope: first.observation_scope,
        phases: first.observations,
    }
    .diagnostic();
    assert_eq!(
        diagnostic["scope"],
        first.observation_scope.unwrap().to_string()
    );
    assert_eq!(diagnostic["source"], "monotonic_elapsed");
    assert_eq!(diagnostic["units"], "ns");
    assert_eq!(diagnostic["aggregation"], "service_lifetime");
    assert_eq!(diagnostic["inclusive"], true);
    assert_eq!(diagnostic["additive"], false);
    assert_eq!(diagnostic["observer_overhead"], "uncalibrated");
    let phases = diagnostic["phases"].as_array().unwrap();
    assert_eq!(phases.len(), 5);
    assert_eq!(
        phases
            .iter()
            .map(|phase| phase["phase"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["probe", "capture", "fill", "lookup", "key_calculation"]
    );
    assert!(diagnostic.to_string().len() < 2048);
}

fn available(aggregate: GroupingPhaseAggregate) -> (u64, u64) {
    match aggregate {
        GroupingPhaseAggregate::Available { calls, elapsed_ns } => (calls, elapsed_ns),
        other => panic!("expected available phase, got {other:?}"),
    }
}

/// Inspect only observations produced by the ordinary controlled worker path.
pub(in crate::workflow) fn assert_worker_phase_coverage(mode: &SerialReadyMode) {
    assert!(mode.config.bounded_grouping);
    let scope = mode
        .grouping_observation_scope()
        .expect("enabled worker diagnostic scope");
    let snapshot = mode.grouping_observation_snapshot();
    assert_eq!(snapshot.scope, Some(scope));
    for (phase, aggregate) in PHASES.iter().zip(snapshot.phases) {
        match aggregate {
            GroupingPhaseAggregate::Available { calls, .. } => {
                // Available elapsed_ns is checked at recording; zero is valid.
                assert!(calls > 0, "worker did not observe {phase:?}");
            }
            GroupingPhaseAggregate::Unavailable { reason } => {
                panic!("worker phase {phase:?} unavailable: {reason:?}");
            }
        }
    }
}

/// The existing controlled worker fixture supplies a genuine bounded Ready pair;
/// this helper adds no provider, native work, sample or scheduling decision.
pub(in crate::workflow) fn assert_grouping_observation_oracle(
    mode: &SerialReadyMode,
    pair: &WorkflowSerialReadyPair,
) {
    assert!(mode.config.bounded_grouping);
    mode.finish_grouping_member(None);
    let before = mode.grouping_observation_snapshot();
    let scope = mode
        .grouping_observation_scope()
        .expect("enabled diagnostic scope");
    let encodings = mode.key_encodings.load(Ordering::Relaxed);
    let hits = mode.key_cache_hits.load(Ordering::Relaxed);

    assert!(mode.grouping_needs_refresh(&pair.members[0]));
    mode.record_grouping_capture(Some(Duration::from_nanos(17)));
    mode.refresh_grouping(Some(pair.members.to_vec()));
    assert!(!mode.grouping_needs_refresh(&pair.members[0]));
    let expected = {
        let group = mode.grouping.lock().unwrap();
        assert_eq!(group.entries.len(), 2);
        group.entries[0].1
    };
    assert_eq!(mode.key_for(&pair.members[0]), Some(expected));
    let mut changed = pair.members[0].clone();
    changed.inputs[0].value =
        RuntimeHostExecutionInputValue::String("changed observed text".into());
    assert!(mode.key_for(&changed).is_some_and(|key| key != expected));
    assert_eq!(mode.key_encodings.load(Ordering::Relaxed) - encodings, 3);
    assert_eq!(mode.key_cache_hits.load(Ordering::Relaxed) - hits, 1);

    let after = mode.grouping_observation_snapshot();
    assert_eq!(after.scope, Some(scope));
    for (phase, expected_calls) in [
        (GroupingPhase::Probe, 2),
        (GroupingPhase::Capture, 1),
        (GroupingPhase::Fill, 1),
        (GroupingPhase::Lookup, 2),
        (GroupingPhase::KeyCalculation, 3),
    ] {
        let prior = available(before.phases[phase as usize]);
        let current = available(after.phases[phase as usize]);
        assert_eq!(current.0 - prior.0, expected_calls);
        assert!(current.1 >= prior.1);
    }
    assert_eq!(
        available(after.phases[GroupingPhase::Capture as usize]).1
            - available(before.phases[GroupingPhase::Capture as usize]).1,
        17
    );
    mode.finish_grouping_member(Some(&pair.members[0]));
    assert_eq!(mode.grouping.lock().unwrap().entries.len(), 1);
    mode.finish_grouping_member(None);
    assert!(mode.grouping.lock().unwrap().entries.is_empty());
    assert_eq!(mode.grouping_observation_snapshot().phases, after.phases);
    assert_eq!(mode.grouping_observation_scope(), Some(scope));
    mode.log_grouping_observations();
    assert_eq!(mode.key_encodings.load(Ordering::Relaxed) - encodings, 3);
    assert_eq!(mode.key_cache_hits.load(Ordering::Relaxed) - hits, 1);
}

pub(in crate::workflow) fn assert_grouping_disabled_and_poison_oracles(
    disabled: &SerialReadyMode,
    enabled: &SerialReadyMode,
    pair: &WorkflowSerialReadyPair,
) {
    assert!(!disabled.config.bounded_grouping);
    assert_eq!(disabled.grouping_observation_scope(), None);
    disabled.record_grouping_capture(Some(Duration::from_nanos(17)));
    assert!(!disabled.grouping_needs_refresh(&pair.members[0]));
    let encodings = disabled.key_encodings.load(Ordering::Relaxed);
    assert!(disabled.key_for(&pair.members[0]).is_some());
    assert_eq!(
        disabled.key_encodings.load(Ordering::Relaxed),
        encodings + 1
    );
    assert_eq!(
        disabled.grouping_observation_snapshot().phases,
        [GroupingPhaseAggregate::default(); 5]
    );

    assert!(enabled.config.bounded_grouping);
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = enabled.grouping.lock().unwrap();
        panic!("controlled cache poison");
    }));
    assert!(poisoned.is_err());
    assert_eq!(enabled.grouping_observation_scope(), None);
    let before = enabled.key_encodings.load(Ordering::Relaxed);
    let hits = enabled.key_cache_hits.load(Ordering::Relaxed);
    assert!(enabled.grouping_needs_refresh(&pair.members[0]));
    assert!(enabled.key_for(&pair.members[0]).is_some());
    assert_eq!(enabled.key_encodings.load(Ordering::Relaxed), before + 1);
    assert_eq!(enabled.key_cache_hits.load(Ordering::Relaxed), hits);
    let snapshot = enabled.grouping_observation_snapshot();
    assert_eq!(snapshot.scope, None);
    assert_eq!(
        snapshot.phases,
        [GroupingPhaseAggregate::Unavailable {
            reason: GroupingObservationUnavailable::LockUnavailable,
        }; 5]
    );
    let diagnostic = snapshot.diagnostic();
    assert!(diagnostic["scope"].is_null());
    assert!(!diagnostic.to_string().contains("controlled cache poison"));
    enabled.log_grouping_observations();
    assert!(enabled.grouping.is_poisoned());
}
