//! Independent public-contract checks for additive stable history evidence.
//! Rows below are controlled protocol data, not native/BuiltIn provenance proof.
use pantograph_timing_contracts::*;

fn key() -> RuntimeServiceTimingHistoryProfile {
    RuntimeServiceTimingHistoryProfile::new("gateway-owner".into(), "a".repeat(64)).unwrap()
}
fn clock(now_ns: u64) -> RuntimeServiceTimingClockSnapshot {
    RuntimeServiceTimingClockSnapshot {
        clock_epoch: "monotonic-domain".into(),
        now_ns,
    }
}
fn row(instance: &str, fence: &str) -> RuntimeServiceTimingAttempt {
    RuntimeServiceTimingAttempt {
        attempt_id: "independent-controlled-attempt".into(),
        execution_request_id_digest: None,
        // History does not silently turn missing legacy content authority into
        // the old exact-instance key. This remains a raw Unknown legacy record.
        identity: RuntimeServiceTimingIdentity::Unknown {
            reason: RuntimeServiceTimingUnavailableReason::ModelContentIdentityUnavailable,
        },
        outcome: RuntimeServiceTimingOutcome::Completed,
        phases: [
            RuntimeServiceTimingPhase::GatewayCustodyWait,
            RuntimeServiceTimingPhase::SelectedModelLoad,
            RuntimeServiceTimingPhase::TextExecution,
            RuntimeServiceTimingPhase::WorkerCleanup,
        ]
        .into_iter()
        .map(|phase| RuntimeServiceTimingPhaseEvidence {
            phase,
            value: RuntimeServiceTimingValue::Observed {
                elapsed_ns: 5,
                outcome: RuntimeServiceTimingOutcome::Completed,
            },
        })
        .collect(),
        capture: Some(RuntimeServiceTimingCapture {
            clock_epoch: "monotonic-domain".into(),
            observed_at_ns: 110,
            owner_provenance: RuntimeServiceTimingOwnerProvenance::BuiltIn,
            load_disposition: RuntimeServiceTimingLoadDisposition::Reloaded,
        }),
        lifecycle: Some(RuntimeServiceTimingLifecycle {
            termination: RuntimeServiceTimingTermination::Completed,
            interval: Some(RuntimeServiceTimingInterval {
                started_at_ns: 50,
                worker_drained_at_ns: Some(100),
                observed_through_ns: 110,
            }),
        }),
        history: Some(RuntimeServiceTimingHistoryEvidence {
            profile: key(),
            runtime_instance_id: instance.into(),
            load_owner_fence: fence.into(),
            drained_owner_fence: Some(fence.into()),
        }),
    }
}

#[test]
fn independent_history_equivalent_key_keeps_distinct_live_rows_and_legacy_unknown() {
    let first = row("installed-instance-a", "native-epoch:1");
    let mut second = row("installed-instance-b", "native-epoch:3");
    second.capture.as_mut().unwrap().load_disposition = RuntimeServiceTimingLoadDisposition::Reused;
    for current in [&first, &second] {
        assert_eq!(
            current.fresh_production_history_interval_ns(&key(), &clock(120), 20),
            Some(50)
        );
        let legacy = RuntimeServiceTimingProfile::new(
            "gateway-owner".into(),
            current
                .history
                .as_ref()
                .unwrap()
                .runtime_instance_id
                .clone(),
            "a".repeat(64),
        )
        .unwrap();
        assert!(current
            .fresh_production_service_interval_ns(&legacy, &clock(120), 20)
            .is_none());
        assert!(current
            .fresh_production_observation(&legacy, &clock(120), 20)
            .is_none());
    }
    assert_ne!(
        first.history.as_ref().unwrap().runtime_instance_id,
        second.history.as_ref().unwrap().runtime_instance_id
    );
    assert_ne!(
        first.history.as_ref().unwrap().load_owner_fence,
        second.history.as_ref().unwrap().load_owner_fence
    );
    assert_ne!(
        first.capture.as_ref().unwrap().load_disposition,
        second.capture.as_ref().unwrap().load_disposition
    );
    let mut legacy_wire = serde_json::to_value(&first).unwrap();
    legacy_wire.as_object_mut().unwrap().remove("history");
    let legacy_row: RuntimeServiceTimingAttempt = serde_json::from_value(legacy_wire).unwrap();
    assert!(legacy_row
        .fresh_production_history_interval_ns(&key(), &clock(120), 20)
        .is_none());
}

#[test]
fn independent_history_capture_and_actual_drain_use_checked_exact_nanosecond_age() {
    let current = row("instance", "fence");
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &clock(120), 20),
        Some(50)
    );
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &clock(121), 20),
        None
    );
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &clock(109), 20),
        None
    );
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &clock(u64::MAX), u64::MAX),
        None
    );
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &clock(120), 0),
        None
    );
    let other_clock = RuntimeServiceTimingClockSnapshot {
        clock_epoch: "foreign-clock".into(),
        now_ns: 120,
    };
    assert_eq!(
        current.fresh_production_history_interval_ns(&key(), &other_clock, 20),
        None
    );
    let mut delayed = current.clone();
    delayed.capture.as_mut().unwrap().observed_at_ns = 1000;
    delayed
        .lifecycle
        .as_mut()
        .unwrap()
        .interval
        .as_mut()
        .unwrap()
        .observed_through_ns = 1000;
    assert_eq!(
        delayed.fresh_production_history_interval_ns(&key(), &clock(1001), 10),
        None,
        "fresh capture cannot refresh stale actual drain"
    );
    let other_key =
        RuntimeServiceTimingHistoryProfile::new("gateway-owner".into(), "b".repeat(64)).unwrap();
    assert_eq!(
        current.fresh_production_history_interval_ns(&other_key, &clock(120), 20),
        None
    );
    let foreign_owner =
        RuntimeServiceTimingHistoryProfile::new("foreign-owner".into(), "a".repeat(64)).unwrap();
    assert_eq!(
        current.fresh_production_history_interval_ns(&foreign_owner, &clock(120), 20),
        None
    );
}

#[test]
fn independent_history_requires_completed_phases_actual_drain_and_matching_fences() {
    let original = row("instance", "fence");
    for mode in 0..15 {
        let mut current = original.clone();
        match mode {
            0 => current.history = None,
            1 => current.history.as_mut().unwrap().drained_owner_fence = None,
            2 => current.history.as_mut().unwrap().drained_owner_fence = Some("changed".into()),
            3 => current
                .history
                .as_mut()
                .unwrap()
                .runtime_instance_id
                .clear(),
            4 => current.history.as_mut().unwrap().load_owner_fence = " ".repeat(257),
            5 => {
                current.capture.as_mut().unwrap().owner_provenance =
                    RuntimeServiceTimingOwnerProvenance::Injected
            }
            6 => {
                current.capture.as_mut().unwrap().load_disposition =
                    RuntimeServiceTimingLoadDisposition::Unknown
            }
            7 => current.outcome = RuntimeServiceTimingOutcome::Failed,
            8 => {
                current.lifecycle.as_mut().unwrap().termination =
                    RuntimeServiceTimingTermination::CancellationRequested
            }
            9 => {
                current.lifecycle.as_mut().unwrap().termination =
                    RuntimeServiceTimingTermination::ShutdownRequested
            }
            10 => {
                current.lifecycle.as_mut().unwrap().termination =
                    RuntimeServiceTimingTermination::Abandoned
            }
            11 => {
                current.phases.pop();
            }
            12 => current.phases[3].phase = current.phases[0].phase,
            13 => {
                current.phases[0].value =
                    RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 5 }
            }
            14 => {
                current.phases[0].value = RuntimeServiceTimingValue::Observed {
                    elapsed_ns: 5,
                    outcome: RuntimeServiceTimingOutcome::Abandoned,
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(
            current.fresh_production_history_interval_ns(&key(), &clock(120), 20),
            None,
            "mode {mode}"
        );
    }
}

#[test]
fn independent_history_interval_order_phase_overflow_and_real_zero_remain_distinct() {
    let original = row("instance", "fence");
    for mode in 0..7 {
        let mut current = original.clone();
        match mode {
            0 => current.lifecycle.as_mut().unwrap().interval = None,
            1 => {
                current
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .worker_drained_at_ns = None
            }
            2 => {
                current
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .started_at_ns = 101
            }
            3 => {
                current
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .observed_through_ns = 99
            }
            4 => {
                current
                    .lifecycle
                    .as_mut()
                    .unwrap()
                    .interval
                    .as_mut()
                    .unwrap()
                    .observed_through_ns = 111
            }
            5 => {
                current.phases[0].value = RuntimeServiceTimingValue::Observed {
                    elapsed_ns: u64::MAX,
                    outcome: RuntimeServiceTimingOutcome::Completed,
                }
            }
            6 => {
                current.phases[0].value = RuntimeServiceTimingValue::Observed {
                    elapsed_ns: 36,
                    outcome: RuntimeServiceTimingOutcome::Completed,
                }
            }
            _ => unreachable!(),
        }
        assert_eq!(
            current.fresh_production_history_interval_ns(&key(), &clock(120), 20),
            None,
            "mode {mode}"
        );
    }
    let mut zero = original;
    let interval = zero.lifecycle.as_mut().unwrap().interval.as_mut().unwrap();
    interval.started_at_ns = 100;
    for phase in &mut zero.phases {
        phase.value = RuntimeServiceTimingValue::Observed {
            elapsed_ns: 0,
            outcome: RuntimeServiceTimingOutcome::Completed,
        };
    }
    assert_eq!(
        zero.fresh_production_history_interval_ns(&key(), &clock(120), 20),
        Some(0)
    );
}
