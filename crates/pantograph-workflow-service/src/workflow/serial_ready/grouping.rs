//! Pure derived keys only. No proposal, owner, reservation or service turn is
//! retained here; each member follows the ordinary authoritative worker path.

use super::{SerialReadyMode, WorkflowSerialReadyMember};
use pantograph_runtime_host_contracts::RuntimeHostExecutionInput;
use pantograph_scheduler::{
    evaluate_scheduler_bounded_compatibility, SchedulerGroupingCompatibility,
    SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum GroupingPhase {
    Probe,
    Capture,
    Fill,
    Lookup,
    KeyCalculation,
}

const PHASES: [GroupingPhase; 5] = [
    GroupingPhase::Probe,
    GroupingPhase::Capture,
    GroupingPhase::Fill,
    GroupingPhase::Lookup,
    GroupingPhase::KeyCalculation,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum GroupingObservationUnavailable {
    InvalidClock,
    DurationOverflow,
    CounterOverflow,
    LockUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum GroupingPhaseAggregate {
    Available {
        calls: u64,
        elapsed_ns: u64,
    },
    Unavailable {
        reason: GroupingObservationUnavailable,
    },
}

impl Default for GroupingPhaseAggregate {
    fn default() -> Self {
        Self::Available {
            calls: 0,
            elapsed_ns: 0,
        }
    }
}

impl GroupingPhaseAggregate {
    fn record(&mut self, elapsed: Option<Duration>) {
        let Self::Available { calls, elapsed_ns } = *self else {
            return;
        };
        let next = (|| {
            let elapsed = elapsed.ok_or(GroupingObservationUnavailable::InvalidClock)?;
            let elapsed = u64::try_from(elapsed.as_nanos())
                .map_err(|_| GroupingObservationUnavailable::DurationOverflow)?;
            let calls = calls
                .checked_add(1)
                .ok_or(GroupingObservationUnavailable::CounterOverflow)?;
            let elapsed_ns = elapsed_ns
                .checked_add(elapsed)
                .ok_or(GroupingObservationUnavailable::CounterOverflow)?;
            Ok(Self::Available { calls, elapsed_ns })
        })();
        *self = next.unwrap_or_else(|reason| Self::Unavailable { reason });
    }
}

#[derive(Clone, Copy)]
struct GroupingObservationSnapshot {
    scope: Option<Uuid>,
    phases: [GroupingPhaseAggregate; 5],
}

impl GroupingObservationSnapshot {
    fn diagnostic(self) -> serde_json::Value {
        let phases: Vec<_> = PHASES
            .iter()
            .zip(self.phases)
            .map(|(phase, aggregate)| serde_json::json!({"phase": phase, "aggregate": aggregate}))
            .collect();
        serde_json::json!({
            "phase": "grouping_observations",
            "scope": self.scope.map(|scope| scope.to_string()),
            "source": "monotonic_elapsed",
            "units": "ns",
            "aggregation": "service_lifetime",
            "inclusive": true,
            "additive": false,
            "observer_overhead": "uncalibrated",
            "phases": phases,
        })
    }
}

#[derive(Default)]
pub(super) struct WorkloadKeyGroup {
    entries: Vec<(WorkflowSerialReadyMember, [u8; 32])>,
    observation_scope: Option<Uuid>,
    observations: [GroupingPhaseAggregate; 5],
}

impl WorkloadKeyGroup {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            observation_scope: enabled.then(Uuid::new_v4),
            ..Self::default()
        }
    }

    fn key(&self, member: &WorkflowSerialReadyMember) -> Option<[u8; 32]> {
        self.entries
            .iter()
            .find_map(|(snapshot, key)| (snapshot == member).then_some(*key))
    }
}

impl SerialReadyMode {
    fn measure_grouping<T>(&self, phase: GroupingPhase, operation: impl FnOnce() -> T) -> T {
        if !self.config.bounded_grouping {
            return operation();
        }
        let started = Instant::now();
        let result = operation();
        // This phase excludes its own recorder. Inclusive parents also contain
        // nested recorders, so totals cannot be added or treated as calibrated CPU.
        self.record_grouping_span(phase, Instant::now().checked_duration_since(started));
        result
    }

    fn record_grouping_span(&self, phase: GroupingPhase, elapsed: Option<Duration>) {
        if !self.config.bounded_grouping {
            return;
        }
        if let Ok(mut group) = self.grouping.lock() {
            group.observations[phase as usize].record(elapsed);
        }
    }

    pub(in crate::workflow) fn record_grouping_capture(&self, elapsed: Option<Duration>) {
        self.record_grouping_span(GroupingPhase::Capture, elapsed);
    }

    pub(super) fn grouping_observation_scope(&self) -> Option<Uuid> {
        self.grouping.lock().ok()?.observation_scope
    }

    fn grouping_observation_snapshot(&self) -> GroupingObservationSnapshot {
        self.grouping.lock().map_or(
            GroupingObservationSnapshot {
                scope: None,
                phases: [GroupingPhaseAggregate::Unavailable {
                    reason: GroupingObservationUnavailable::LockUnavailable,
                }; 5],
            },
            |group| GroupingObservationSnapshot {
                scope: group.observation_scope,
                phases: group.observations,
            },
        )
    }

    pub(super) fn log_grouping_observations(&self) {
        if self.config.bounded_grouping {
            let snapshot = self.grouping_observation_snapshot();
            // Snapshot copies only fixed scalars; no cache guard reaches logging.
            log::info!(target: "workflow::serial_ready", "{}", snapshot.diagnostic());
        }
    }

    pub(super) fn finish_grouping_member(&self, member: Option<&WorkflowSerialReadyMember>) {
        if !self.config.bounded_grouping {
            return;
        }
        if let Ok(mut group) = self.grouping.lock() {
            if let Some(member) = member {
                group.entries.retain(|(snapshot, _)| {
                    snapshot.task.workflow_run_id != member.task.workflow_run_id
                        || snapshot.task.task_id != member.task.task_id
                });
            } else {
                group.entries.clear();
            }
        }
    }

    pub(crate) fn grouping_needs_refresh(&self, anchor: &WorkflowSerialReadyMember) -> bool {
        self.measure_grouping(GroupingPhase::Probe, || {
            self.config.bounded_grouping
                && (!anchor.grouping_bounded()
                    || self
                        .grouping
                        .lock()
                        .map_or(true, |group| group.key(anchor).is_none()))
        })
    }

    pub(crate) fn refresh_grouping(&self, window: Option<Vec<WorkflowSerialReadyMember>>) {
        // The key-fill closure has no callbacks or awaits and runs after the
        // store guard ends. Subsequent forecast owner queries run outside guards.
        let entries = self.measure_grouping(GroupingPhase::Fill, || {
            window.and_then(|members| {
                if members.is_empty()
                    || members.len() > SCHEDULER_BOUNDED_GROUPING_MAX_MEMBERS
                    || members.iter().any(|member| !member.grouping_bounded())
                {
                    return None;
                }
                let views = members
                    .iter()
                    .map(|member| {
                        let intent = member.task.schedulable_intent.as_ref()?;
                        Some(SchedulerGroupingCompatibility {
                            task_family: &intent.task_type,
                            model_ref: &intent.model_ref,
                            runtime_id: intent.constraints.requested_runtime_id.as_ref()?,
                            device_ids: std::slice::from_ref(
                                intent.constraints.requested_device_id.as_ref()?,
                            ),
                            // A known input kind, not a tokenizer/tensor dimension.
                            input_shape_signature: "single_utf8_text",
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                let count = evaluate_scheduler_bounded_compatibility(&views).ok()?;
                let mut entries = Vec::with_capacity(count);
                for member in members.into_iter().take(count) {
                    let key = self.compute_key(&member, &member.inputs)?;
                    entries.push((member, key));
                }
                Some(entries)
            })
        });
        if let Some(entries) = entries.as_ref().filter(|_| self.config.bounded_grouping) {
            // This is the captured compatible prefix, before cache publication;
            // owner queries and logging must never run under the cache mutex.
            self.log_warm_service_forecasts(entries);
        }
        if let Ok(mut cache) = self.grouping.lock() {
            cache.entries = entries.unwrap_or_default();
        }
        self.log_grouping_observations();
    }

    pub(super) fn key_for(&self, member: &WorkflowSerialReadyMember) -> Option<[u8; 32]> {
        self.measure_grouping(GroupingPhase::Lookup, || {
            // Bound the complete current equality operand before comparing it with
            // stored snapshots. A miss never upgrades or changes ordinary behavior.
            if self.config.bounded_grouping && member.grouping_bounded() {
                let key = self
                    .grouping
                    .lock()
                    .ok()
                    .and_then(|group| group.key(member));
                if let Some(key) = key {
                    #[cfg(any(test, feature = "test-support"))]
                    self.key_cache_hits
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Some(key);
                }
            }
            self.compute_key(member, &member.inputs)
        })
    }

    pub(super) fn compute_key(
        &self,
        member: &WorkflowSerialReadyMember,
        inputs: &[RuntimeHostExecutionInput],
    ) -> Option<[u8; 32]> {
        self.measure_grouping(GroupingPhase::KeyCalculation, || {
            super::observation_key_with_inputs(member, inputs, || {
                // Count every actual key encoding, including eager fill, misses,
                // lookahead and the independent final native-request input fence.
                #[cfg(any(test, feature = "test-support"))]
                self.key_encodings
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            })
        })
    }
}

#[cfg(test)]
#[path = "grouping_observations_tests.rs"]
pub(super) mod tests;

impl super::WorkflowService {
    /// Fallible test-support observation of mechanism work and retained entries.
    /// Absent serial mode or poisoned cache is unavailable, never measured zero.
    /// These fixed counters confer no execution or cleanup authority.
    #[cfg(any(test, feature = "test-support"))]
    pub fn try_serial_grouping_work_for_test(
        &self,
    ) -> Result<(u64, u64, usize), super::WorkflowServiceError> {
        use std::sync::atomic::Ordering;
        let mode = self.serial_ready_mode.as_ref().ok_or_else(|| {
            super::WorkflowServiceError::InvalidRequest(
                "serial grouping observation requires serial mode".into(),
            )
        })?;
        let cache = mode.grouping.lock().map_err(|_| {
            super::WorkflowServiceError::Internal(
                "serial grouping observation cache unavailable".into(),
            )
        })?;
        Ok((
            mode.key_encodings.load(Ordering::Relaxed),
            mode.key_cache_hits.load(Ordering::Relaxed),
            cache.entries.len(),
        ))
    }

    /// Test-only mechanism counts, including independent final-request hashing.
    /// These counters establish no end-to-end or real-model performance result.
    #[cfg(any(test, feature = "test-support"))]
    pub fn serial_grouping_work_for_test(&self) -> (u64, u64, usize) {
        use std::sync::atomic::Ordering;
        self.serial_ready_mode.as_ref().map_or((0, 0, 0), |mode| {
            (
                mode.key_encodings.load(Ordering::Relaxed),
                mode.key_cache_hits.load(Ordering::Relaxed),
                mode.grouping.lock().map_or(0, |group| group.entries.len()),
            )
        })
    }
}
