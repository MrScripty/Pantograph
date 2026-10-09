//! Opt-in service observations. They are evidence, not a scheduling prediction.

use serde::{Deserialize, Serialize};

/// Exact owner epoch, loaded runtime generation and hashed execution identity.
/// The digest binds owner content, implementation, physical device, effective
/// configuration and workload facts. Missing facts must not construct this key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "ProfileWire")]
pub struct RuntimeServiceTimingProfile {
    owner_epoch: String,
    runtime_instance_id: String,
    identity_fingerprint: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileWire {
    owner_epoch: String,
    runtime_instance_id: String,
    identity_fingerprint: String,
}

impl RuntimeServiceTimingProfile {
    pub fn new(
        owner_epoch: String,
        runtime_instance_id: String,
        identity_fingerprint: String,
    ) -> Result<Self, String> {
        for value in [&owner_epoch, &runtime_instance_id] {
            if value.trim().is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                return Err("bounded owner epoch and runtime instance are required".into());
            }
        }
        if identity_fingerprint.len() != 64
            || !identity_fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("a canonical 64-character identity digest is required".into());
        }
        Ok(Self {
            owner_epoch,
            runtime_instance_id,
            identity_fingerprint,
        })
    }
    pub fn owner_epoch(&self) -> &str {
        &self.owner_epoch
    }
    pub fn runtime_instance_id(&self) -> &str {
        &self.runtime_instance_id
    }
    pub fn identity_fingerprint(&self) -> &str {
        &self.identity_fingerprint
    }
}

impl TryFrom<ProfileWire> for RuntimeServiceTimingProfile {
    type Error = String;
    fn try_from(value: ProfileWire) -> Result<Self, Self::Error> {
        Self::new(
            value.owner_epoch,
            value.runtime_instance_id,
            value.identity_fingerprint,
        )
    }
}

/// Comparable history within one gateway owner/clock domain. This deliberately
/// has no runtime-instance field and grants no live execution/residency authority.
/// The fingerprint binds installed content, implementation, effective native
/// configuration, process CPU domain and exact workload; unknown facts refuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HistoryProfileWire")]
pub struct RuntimeServiceTimingHistoryProfile {
    owner_epoch: String,
    identity_fingerprint: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HistoryProfileWire {
    owner_epoch: String,
    identity_fingerprint: String,
}
impl RuntimeServiceTimingHistoryProfile {
    pub fn new(owner_epoch: String, identity_fingerprint: String) -> Result<Self, String> {
        // Reject oversized metadata before scans/copies. Never manufacture an
        // instance label to use this comparable key as live custody authority.
        if owner_epoch.len() > 256
            || owner_epoch.trim().is_empty()
            || owner_epoch.chars().any(char::is_control)
        {
            return Err("a bounded owner epoch is required".into());
        }
        if identity_fingerprint.len() != 64
            || !identity_fingerprint
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("a canonical 64-character history digest is required".into());
        }
        Ok(Self {
            owner_epoch,
            identity_fingerprint,
        })
    }
    pub fn owner_epoch(&self) -> &str {
        &self.owner_epoch
    }
    pub fn identity_fingerprint(&self) -> &str {
        &self.identity_fingerprint
    }
}
impl TryFrom<HistoryProfileWire> for RuntimeServiceTimingHistoryProfile {
    type Error = String;
    fn try_from(value: HistoryProfileWire) -> Result<Self, Self::Error> {
        Self::new(value.owner_epoch, value.identity_fingerprint)
    }
}

/// Preserve the actual loaded instance and native ACK generation separately from
/// stable history. Revalidation must compare actual content/config/device facts
/// too; matching generation labels alone are insufficient. No pooling/prediction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingHistoryEvidence {
    pub profile: RuntimeServiceTimingHistoryProfile,
    pub runtime_instance_id: String,
    pub load_owner_fence: String,
    pub drained_owner_fence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingUnavailableReason {
    ModelContentIdentityUnavailable,
    IdentityBudgetExceeded,
    RuntimeOwnerFactsUnavailable,
    RuntimeGenerationUnavailable,
    PhaseNotReached,
    ClockDiscontinuity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeServiceTimingIdentity {
    Exact {
        profile: RuntimeServiceTimingProfile,
    },
    Unknown {
        reason: RuntimeServiceTimingUnavailableReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingPhase {
    GatewayCustodyWait,
    SelectedModelLoad,
    TextExecution,
    WorkerCleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingOutcome {
    Completed,
    Failed,
    Abandoned,
}

/// Observed caller termination, not proof of backend stop or capacity release.
/// Cancellation/shutdown requests remain distinct from failure and abandonment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingTermination {
    Completed,
    Failed,
    CancellationRequested,
    ShutdownRequested,
    Abandoned,
}

/// Same monotonic clock domain as capture. Intrinsic service ends at the actual
/// successful worker-drain ACK, not classification, publication or guard Drop.
/// A partial interval is retained without asserting acknowledged drain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingInterval {
    pub started_at_ns: u64,
    pub observed_through_ns: u64,
    pub worker_drained_at_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingLifecycle {
    pub termination: RuntimeServiceTimingTermination,
    pub interval: Option<RuntimeServiceTimingInterval>,
}

/// Configured estimates are explicitly authored values, never observed samples.
/// Unknown is distinct from a measured or configured zero. Failed and abandoned
/// elapsed observations describe partial work, not successful service latency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeServiceTimingValue {
    Observed {
        elapsed_ns: u64,
        outcome: RuntimeServiceTimingOutcome,
    },
    ConfiguredEstimate {
        elapsed_ns: u64,
    },
    Unknown {
        reason: RuntimeServiceTimingUnavailableReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingPhaseEvidence {
    pub phase: RuntimeServiceTimingPhase,
    pub value: RuntimeServiceTimingValue,
}

/// Constructor provenance, never inferred from a backend name or supplied facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingOwnerProvenance {
    BuiltIn,
    Injected,
}

/// The successful backend load outcome. Reloaded does not assert cold caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingLoadDisposition {
    Reloaded,
    Reused,
    Unknown,
}

/// Snapshot from the same live instrumentation clock as the observation.
/// This is trusted in-process association, not authentication of external input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingClockSnapshot {
    pub clock_epoch: String,
    pub now_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingCapture {
    pub clock_epoch: String,
    pub observed_at_ns: u64,
    pub owner_provenance: RuntimeServiceTimingOwnerProvenance,
    pub load_disposition: RuntimeServiceTimingLoadDisposition,
}

/// One successful sample's separate phase costs. No prediction or aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeServiceTimingQualifiedObservation {
    pub custody_wait_ns: u64,
    pub selected_model_load_ns: u64,
    pub text_execution_ns: u64,
    pub worker_cleanup_ns: u64,
    pub load_disposition: RuntimeServiceTimingLoadDisposition,
    pub observed_at_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingAttempt {
    pub attempt_id: String,
    /// Optional lowercase BLAKE3 hex digest of the original request-id bytes.
    /// Capture emits exactly 64 characters for IDs within its raw byte budget,
    /// including rejected calls; oversized IDs omit correlation. This is never
    /// the actual execution identity.
    pub execution_request_id_digest: Option<String>,
    pub identity: RuntimeServiceTimingIdentity,
    pub outcome: RuntimeServiceTimingOutcome,
    pub phases: Vec<RuntimeServiceTimingPhaseEvidence>,
    /// Missing legacy metadata cannot qualify as fresh production evidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<RuntimeServiceTimingCapture>,
    /// Missing legacy metadata cannot supply a whole through-drain interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<RuntimeServiceTimingLifecycle>,
    /// Additive comparable history with independent through-drain live fences.
    /// Absent legacy/unsupported/censored records never gain this qualification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history: Option<RuntimeServiceTimingHistoryEvidence>,
}

impl RuntimeServiceTimingAttempt {
    /// Strict native history interval, separate from exact-instance APIs. This
    /// keeps every old key/raw row unchanged and grants no prediction authority.
    /// Includes inter-phase gaps; both capture and actual drain must be fresh.
    pub fn fresh_production_history_interval_ns(
        &self,
        profile: &RuntimeServiceTimingHistoryProfile,
        clock: &RuntimeServiceTimingClockSnapshot,
        max_age_ns: u64,
    ) -> Option<u64> {
        let history = self.history.as_ref()?;
        let capture = self.capture.as_ref()?;
        let valid_label = |label: &str| {
            label.len() <= 256 && !label.trim().is_empty() && !label.chars().any(char::is_control)
        };
        if &history.profile != profile
            || !valid_label(&history.runtime_instance_id)
            || !valid_label(&history.load_owner_fence)
            || history.drained_owner_fence.as_deref() != Some(history.load_owner_fence.as_str())
            || capture.owner_provenance != RuntimeServiceTimingOwnerProvenance::BuiltIn
            || capture.load_disposition == RuntimeServiceTimingLoadDisposition::Unknown
            || !valid_label(&capture.clock_epoch)
            || capture.clock_epoch != clock.clock_epoch
            || max_age_ns == 0
            || clock.now_ns == u64::MAX
            || clock.now_ns.checked_sub(capture.observed_at_ns)? > max_age_ns
            || self.outcome != RuntimeServiceTimingOutcome::Completed
            || self.phases.len() != 4
        {
            return None;
        }
        let mut sum = 0_u64;
        for phase in [
            RuntimeServiceTimingPhase::GatewayCustodyWait,
            RuntimeServiceTimingPhase::SelectedModelLoad,
            RuntimeServiceTimingPhase::TextExecution,
            RuntimeServiceTimingPhase::WorkerCleanup,
        ] {
            let mut matches = self.phases.iter().filter(|item| item.phase == phase);
            let value = &matches.next()?.value;
            if matches.next().is_some() {
                return None;
            }
            let RuntimeServiceTimingValue::Observed {
                elapsed_ns,
                outcome: RuntimeServiceTimingOutcome::Completed,
            } = value
            else {
                return None;
            };
            sum = sum.checked_add(*elapsed_ns)?;
        }
        let lifecycle = self.lifecycle.as_ref()?;
        if lifecycle.termination != RuntimeServiceTimingTermination::Completed {
            return None;
        }
        let interval = lifecycle.interval.as_ref()?;
        let drain = interval.worker_drained_at_ns?;
        if drain > interval.observed_through_ns
            || interval.observed_through_ns > capture.observed_at_ns
            || clock.now_ns.checked_sub(drain)? > max_age_ns
        {
            return None;
        }
        let duration = drain.checked_sub(interval.started_at_ns)?;
        (sum <= duration).then_some(duration)
    }

    /// Whole custody-through-worker-drain interval, including inter-phase gaps.
    /// Freshness here uses capture age, preserving the phase accessor contract.
    /// Predictors must separately qualify the actual drain age.
    /// Successful exact built-in evidence only; preserves the raw attempt and
    /// never turns partial/canceled service or four phase subtotals into a total.
    pub fn fresh_production_service_interval_ns(
        &self,
        profile: &RuntimeServiceTimingProfile,
        clock: &RuntimeServiceTimingClockSnapshot,
        max_age_ns: u64,
    ) -> Option<u64> {
        let phases = self.fresh_production_observation(profile, clock, max_age_ns)?;
        let lifecycle = self.lifecycle.as_ref()?;
        if lifecycle.termination != RuntimeServiceTimingTermination::Completed {
            return None;
        }
        let interval = lifecycle.interval.as_ref()?;
        let drain = interval.worker_drained_at_ns?;
        let capture = self.capture.as_ref()?;
        if drain > interval.observed_through_ns
            || interval.observed_through_ns > capture.observed_at_ns
        {
            return None;
        }
        let duration = drain.checked_sub(interval.started_at_ns)?;
        let phase_sum = phases
            .custody_wait_ns
            .checked_add(phases.selected_model_load_ns)?
            .checked_add(phases.text_execution_ns)?
            .checked_add(phases.worker_cleanup_ns)?;
        (phase_sum <= duration).then_some(duration)
    }
    /// Exact, fresh, fully successful built-in evidence only. Refusal preserves
    /// the caller's fallback. This does not qualify scheduler transfer, host
    /// preparation, reservation release or reuse across runtime generations.
    pub fn fresh_production_observation(
        &self,
        profile: &RuntimeServiceTimingProfile,
        clock: &RuntimeServiceTimingClockSnapshot,
        max_age_ns: u64,
    ) -> Option<RuntimeServiceTimingQualifiedObservation> {
        let capture = self.capture.as_ref()?;
        if capture.owner_provenance != RuntimeServiceTimingOwnerProvenance::BuiltIn
            || capture.load_disposition == RuntimeServiceTimingLoadDisposition::Unknown
            || capture.clock_epoch.len() > 256
            || capture.clock_epoch.trim().is_empty()
            || capture.clock_epoch.chars().any(char::is_control)
            || capture.clock_epoch != clock.clock_epoch
            || max_age_ns == 0
            || clock.now_ns == u64::MAX
            || clock.now_ns.checked_sub(capture.observed_at_ns)? > max_age_ns
            || self.phases.len() != 4
        {
            return None;
        }
        Some(RuntimeServiceTimingQualifiedObservation {
            custody_wait_ns: self
                .observed_completed_ns(profile, RuntimeServiceTimingPhase::GatewayCustodyWait)?,
            selected_model_load_ns: self
                .observed_completed_ns(profile, RuntimeServiceTimingPhase::SelectedModelLoad)?,
            text_execution_ns: self
                .observed_completed_ns(profile, RuntimeServiceTimingPhase::TextExecution)?,
            worker_cleanup_ns: self
                .observed_completed_ns(profile, RuntimeServiceTimingPhase::WorkerCleanup)?,
            load_disposition: capture.load_disposition,
            observed_at_ns: capture.observed_at_ns,
        })
    }
    /// Exact matching and successful observations only. This is a single sample,
    /// not a throughput estimate, calibration, bound or ranking decision.
    pub fn observed_completed_ns(
        &self,
        profile: &RuntimeServiceTimingProfile,
        phase: RuntimeServiceTimingPhase,
    ) -> Option<u64> {
        if self.outcome != RuntimeServiceTimingOutcome::Completed
            || self.identity
                != (RuntimeServiceTimingIdentity::Exact {
                    profile: profile.clone(),
                })
        {
            return None;
        }
        let mut matches = self.phases.iter().filter(|item| item.phase == phase);
        let value = &matches.next()?.value;
        if matches.next().is_some() {
            return None;
        }
        match value {
            RuntimeServiceTimingValue::Observed {
                elapsed_ns,
                outcome: RuntimeServiceTimingOutcome::Completed,
            } => Some(*elapsed_ns),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile() -> RuntimeServiceTimingProfile {
        RuntimeServiceTimingProfile::new("owner".into(), "instance".into(), "a".repeat(64)).unwrap()
    }
    #[test]
    fn service_observed_estimated_and_unknown_zero_have_distinct_wire_shapes() {
        let values = [
            RuntimeServiceTimingValue::Observed {
                elapsed_ns: 0,
                outcome: RuntimeServiceTimingOutcome::Completed,
            },
            RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 0 },
            RuntimeServiceTimingValue::Unknown {
                reason: RuntimeServiceTimingUnavailableReason::PhaseNotReached,
            },
        ];
        for value in values {
            assert_eq!(
                serde_json::from_str::<RuntimeServiceTimingValue>(
                    &serde_json::to_string(&value).unwrap()
                )
                .unwrap(),
                value
            );
        }
        assert!(serde_json::from_str::<RuntimeServiceTimingValue>(
            r#"{"source":"unknown","reason":"phase_not_reached","elapsed_ns":0}"#
        )
        .is_err());
    }
    #[test]
    fn service_matching_excludes_other_identity_estimates_partial_and_duplicate_phases() {
        let mut attempt = RuntimeServiceTimingAttempt {
            history: None,
            capture: None,
            lifecycle: None,
            attempt_id: "attempt".into(),
            execution_request_id_digest: Some("c".repeat(64)),
            identity: RuntimeServiceTimingIdentity::Exact { profile: profile() },
            outcome: RuntimeServiceTimingOutcome::Completed,
            phases: vec![RuntimeServiceTimingPhaseEvidence {
                phase: RuntimeServiceTimingPhase::SelectedModelLoad,
                value: RuntimeServiceTimingValue::Observed {
                    elapsed_ns: 0,
                    outcome: RuntimeServiceTimingOutcome::Completed,
                },
            }],
        };
        assert_eq!(
            attempt.observed_completed_ns(&profile(), RuntimeServiceTimingPhase::SelectedModelLoad),
            Some(0)
        );
        for other in [
            RuntimeServiceTimingProfile::new("other".into(), "instance".into(), "a".repeat(64))
                .unwrap(),
            RuntimeServiceTimingProfile::new("owner".into(), "new-instance".into(), "a".repeat(64))
                .unwrap(),
            RuntimeServiceTimingProfile::new("owner".into(), "instance".into(), "b".repeat(64))
                .unwrap(),
        ] {
            assert_eq!(
                attempt.observed_completed_ns(&other, RuntimeServiceTimingPhase::SelectedModelLoad),
                None
            );
        }
        for value in [
            RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 99 },
            RuntimeServiceTimingValue::Unknown {
                reason: RuntimeServiceTimingUnavailableReason::PhaseNotReached,
            },
            RuntimeServiceTimingValue::Observed {
                elapsed_ns: 99,
                outcome: RuntimeServiceTimingOutcome::Failed,
            },
        ] {
            attempt.phases[0].value = value;
            assert_eq!(
                attempt.observed_completed_ns(
                    &profile(),
                    RuntimeServiceTimingPhase::SelectedModelLoad
                ),
                None
            );
        }
        attempt.phases[0].value = RuntimeServiceTimingValue::Observed {
            elapsed_ns: 12,
            outcome: RuntimeServiceTimingOutcome::Completed,
        };
        attempt.outcome = RuntimeServiceTimingOutcome::Abandoned;
        assert_eq!(
            attempt.observed_completed_ns(&profile(), RuntimeServiceTimingPhase::SelectedModelLoad),
            None
        );
        attempt.outcome = RuntimeServiceTimingOutcome::Completed;
        attempt.phases.push(attempt.phases[0].clone());
        assert_eq!(
            attempt.observed_completed_ns(&profile(), RuntimeServiceTimingPhase::SelectedModelLoad),
            None
        );
    }
    #[test]
    fn service_profiles_validate_during_deserialization() {
        assert!(serde_json::from_value::<RuntimeServiceTimingProfile>(serde_json::json!({"owner_epoch":"", "runtime_instance_id":"instance", "identity_fingerprint":"a".repeat(64)})).is_err());
        assert!(RuntimeServiceTimingProfile::new(
            "owner".into(),
            "instance".into(),
            "A".repeat(64)
        )
        .is_err());
        let profile = profile();
        assert_eq!(
            serde_json::from_value::<RuntimeServiceTimingProfile>(
                serde_json::to_value(&profile).unwrap()
            )
            .unwrap(),
            profile
        );
    }

    fn complete_observation() -> RuntimeServiceTimingAttempt {
        RuntimeServiceTimingAttempt {
            history: None,
            lifecycle: None,
            attempt_id: "synthetic-test-attempt".into(),
            execution_request_id_digest: None,
            identity: RuntimeServiceTimingIdentity::Exact { profile: profile() },
            outcome: RuntimeServiceTimingOutcome::Completed,
            phases: [
                (RuntimeServiceTimingPhase::GatewayCustodyWait, 2),
                (RuntimeServiceTimingPhase::SelectedModelLoad, 11),
                (RuntimeServiceTimingPhase::TextExecution, 23),
                (RuntimeServiceTimingPhase::WorkerCleanup, 7),
            ]
            .into_iter()
            .map(|(phase, elapsed_ns)| RuntimeServiceTimingPhaseEvidence {
                phase,
                value: RuntimeServiceTimingValue::Observed {
                    elapsed_ns,
                    outcome: RuntimeServiceTimingOutcome::Completed,
                },
            })
            .collect(),
            // Hand-authored contract fixture only, never production calibration.
            capture: Some(RuntimeServiceTimingCapture {
                clock_epoch: "test-clock".into(),
                observed_at_ns: 100,
                owner_provenance: RuntimeServiceTimingOwnerProvenance::BuiltIn,
                load_disposition: RuntimeServiceTimingLoadDisposition::Reloaded,
            }),
        }
    }
    fn clock(now_ns: u64) -> RuntimeServiceTimingClockSnapshot {
        RuntimeServiceTimingClockSnapshot {
            clock_epoch: "test-clock".into(),
            now_ns,
        }
    }
    #[test]
    fn fresh_service_observations_keep_load_execution_and_cleanup_separate() {
        let attempt = complete_observation();
        let qualified = attempt
            .fresh_production_observation(&profile(), &clock(110), 10)
            .unwrap();
        assert_eq!(
            (
                qualified.custody_wait_ns,
                qualified.selected_model_load_ns,
                qualified.text_execution_ns,
                qualified.worker_cleanup_ns
            ),
            (2, 11, 23, 7)
        );
        assert_eq!(
            qualified.load_disposition,
            RuntimeServiceTimingLoadDisposition::Reloaded
        );
        assert_eq!(qualified.observed_at_ns, 100);
        assert!(attempt
            .fresh_production_observation(&profile(), &clock(100), 10)
            .is_some());
        assert!(attempt
            .fresh_production_observation(&profile(), &clock(111), 10)
            .is_none());
        assert!(attempt
            .fresh_production_observation(&profile(), &clock(99), 10)
            .is_none());
        assert!(attempt
            .fresh_production_observation(&profile(), &clock(u64::MAX), 10)
            .is_none());
        assert!(attempt
            .fresh_production_observation(&profile(), &clock(100), 0)
            .is_none());
        let mut different_clock = clock(110);
        different_clock.clock_epoch = "another-owner-clock".into();
        assert!(attempt
            .fresh_production_observation(&profile(), &different_clock, 10)
            .is_none());
    }
    #[test]
    fn fresh_service_observations_refuse_unqualified_provenance_phases_and_identity() {
        for bad in [
            "legacy",
            "injected",
            "unknown-load",
            "failed",
            "abandoned",
            "duplicate",
            "missing",
            "estimate",
            "partial",
            "unknown-phase",
            "identity",
            "bad-clock",
        ] {
            let mut attempt = complete_observation();
            match bad {
                "legacy" => attempt.capture = None,
                "injected" => {
                    attempt.capture.as_mut().unwrap().owner_provenance =
                        RuntimeServiceTimingOwnerProvenance::Injected
                }
                "unknown-load" => {
                    attempt.capture.as_mut().unwrap().load_disposition =
                        RuntimeServiceTimingLoadDisposition::Unknown
                }
                "failed" => attempt.outcome = RuntimeServiceTimingOutcome::Failed,
                "abandoned" => attempt.outcome = RuntimeServiceTimingOutcome::Abandoned,
                "duplicate" => attempt.phases[0] = attempt.phases[1].clone(),
                "missing" => {
                    attempt.phases.pop();
                }
                "estimate" => {
                    attempt.phases[1].value =
                        RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 11 }
                }
                "partial" => {
                    attempt.phases[3].value = RuntimeServiceTimingValue::Observed {
                        elapsed_ns: 7,
                        outcome: RuntimeServiceTimingOutcome::Failed,
                    }
                }
                "unknown-phase" => {
                    attempt.phases[2].value = RuntimeServiceTimingValue::Unknown {
                        reason: RuntimeServiceTimingUnavailableReason::PhaseNotReached,
                    }
                }
                "identity" => {
                    attempt.identity = RuntimeServiceTimingIdentity::Unknown {
                        reason: RuntimeServiceTimingUnavailableReason::RuntimeOwnerFactsUnavailable,
                    }
                }
                "bad-clock" => attempt.capture.as_mut().unwrap().clock_epoch.clear(),
                _ => unreachable!(),
            }
            assert!(
                attempt
                    .fresh_production_observation(&profile(), &clock(100), 10)
                    .is_none(),
                "{bad}"
            );
        }
        let attempt = complete_observation();
        for other in [
            RuntimeServiceTimingProfile::new(
                "other-owner".into(),
                "instance".into(),
                "a".repeat(64),
            )
            .unwrap(),
            RuntimeServiceTimingProfile::new("owner".into(), "new-instance".into(), "a".repeat(64))
                .unwrap(),
            RuntimeServiceTimingProfile::new("owner".into(), "instance".into(), "b".repeat(64))
                .unwrap(),
        ] {
            assert!(attempt
                .fresh_production_observation(&other, &clock(100), 10)
                .is_none());
        }
        let mut legacy = serde_json::to_value(&attempt).unwrap();
        legacy.as_object_mut().unwrap().remove("capture");
        assert!(
            serde_json::from_value::<RuntimeServiceTimingAttempt>(legacy)
                .unwrap()
                .capture
                .is_none()
        );
    }
    #[test]
    fn whole_service_interval_requires_ack_and_includes_unmeasured_gaps() {
        let mut attempt = complete_observation();
        assert_eq!(
            attempt.fresh_production_service_interval_ns(&profile(), &clock(110), 10),
            None
        );
        attempt.lifecycle = Some(RuntimeServiceTimingLifecycle {
            termination: RuntimeServiceTimingTermination::Completed,
            interval: Some(RuntimeServiceTimingInterval {
                started_at_ns: 10,
                worker_drained_at_ns: Some(60),
                observed_through_ns: 90,
            }),
        });
        assert_eq!(
            attempt.fresh_production_service_interval_ns(&profile(), &clock(110), 10),
            Some(50)
        );
        // Publication/Drop may be later; neither substitutes for intrinsic drain.
        attempt
            .lifecycle
            .as_mut()
            .unwrap()
            .interval
            .as_mut()
            .unwrap()
            .observed_through_ns = 100;
        assert_eq!(
            attempt.fresh_production_service_interval_ns(&profile(), &clock(110), 10),
            Some(50)
        );
        for interval in [
            RuntimeServiceTimingInterval {
                started_at_ns: 10,
                worker_drained_at_ns: None,
                observed_through_ns: 90,
            },
            RuntimeServiceTimingInterval {
                started_at_ns: 61,
                worker_drained_at_ns: Some(60),
                observed_through_ns: 90,
            },
            RuntimeServiceTimingInterval {
                started_at_ns: 10,
                worker_drained_at_ns: Some(60),
                observed_through_ns: 59,
            },
            RuntimeServiceTimingInterval {
                started_at_ns: 10,
                worker_drained_at_ns: Some(60),
                observed_through_ns: 101,
            },
            RuntimeServiceTimingInterval {
                started_at_ns: 10,
                worker_drained_at_ns: Some(11),
                observed_through_ns: 90,
            },
        ] {
            attempt.lifecycle.as_mut().unwrap().interval = Some(interval);
            assert_eq!(
                attempt.fresh_production_service_interval_ns(&profile(), &clock(110), 10),
                None
            );
        }
    }
    #[test]
    fn termination_and_legacy_phase_evidence_remain_distinct() {
        let mut attempt = complete_observation();
        for termination in [
            RuntimeServiceTimingTermination::Failed,
            RuntimeServiceTimingTermination::CancellationRequested,
            RuntimeServiceTimingTermination::ShutdownRequested,
            RuntimeServiceTimingTermination::Abandoned,
        ] {
            attempt.lifecycle = Some(RuntimeServiceTimingLifecycle {
                termination,
                interval: Some(RuntimeServiceTimingInterval {
                    started_at_ns: 0,
                    worker_drained_at_ns: Some(100),
                    observed_through_ns: 100,
                }),
            });
            assert!(attempt
                .fresh_production_service_interval_ns(&profile(), &clock(100), 10)
                .is_none());
        }
        // A missing additive field retains legacy phase access, never whole totals.
        attempt.lifecycle = None;
        let wire = serde_json::to_value(&attempt).unwrap();
        assert!(wire.get("lifecycle").is_none());
        let legacy: RuntimeServiceTimingAttempt = serde_json::from_value(wire).unwrap();
        assert!(legacy
            .fresh_production_observation(&profile(), &clock(100), 10)
            .is_some());
        assert!(legacy
            .fresh_production_service_interval_ns(&profile(), &clock(100), 10)
            .is_none());
    }
    fn history_observation() -> RuntimeServiceTimingAttempt {
        let mut row = complete_observation();
        row.history = Some(RuntimeServiceTimingHistoryEvidence {
            profile: RuntimeServiceTimingHistoryProfile::new("owner".into(), "f".repeat(64))
                .unwrap(),
            runtime_instance_id: "instance".into(),
            load_owner_fence: "native-epoch:2".into(),
            drained_owner_fence: Some("native-epoch:2".into()),
        });
        row.lifecycle = Some(RuntimeServiceTimingLifecycle {
            termination: RuntimeServiceTimingTermination::Completed,
            interval: Some(RuntimeServiceTimingInterval {
                started_at_ns: 0,
                worker_drained_at_ns: Some(100),
                observed_through_ns: 100,
            }),
        });
        row
    }
    #[test]
    fn history_comparison_never_normalizes_legacy_instances_or_custody_fences() {
        let first = history_observation();
        let mut second = first.clone();
        second.identity = RuntimeServiceTimingIdentity::Exact {
            profile: RuntimeServiceTimingProfile::new(
                "owner".into(),
                "second-instance".into(),
                "a".repeat(64),
            )
            .unwrap(),
        };
        let evidence = second.history.as_mut().unwrap();
        evidence.runtime_instance_id = "second-instance".into();
        evidence.load_owner_fence = "native-epoch:4".into();
        evidence.drained_owner_fence = Some("native-epoch:4".into());
        let second_instance = evidence.runtime_instance_id.clone();
        let key = &first.history.as_ref().unwrap().profile;
        assert_eq!(
            first.fresh_production_history_interval_ns(key, &clock(110), 10),
            Some(100)
        );
        assert_eq!(
            second.fresh_production_history_interval_ns(key, &clock(110), 10),
            Some(100)
        );
        assert!(second
            .fresh_production_service_interval_ns(&profile(), &clock(110), 10)
            .is_none());
        assert_ne!(first.identity, second.identity);
        assert_ne!(
            first.history.as_ref().unwrap().runtime_instance_id,
            second_instance
        );
    }
    #[test]
    fn history_refuses_incomplete_stale_injected_censored_changed_and_duplicate_evidence() {
        let good = history_observation();
        let key = good.history.as_ref().unwrap().profile.clone();
        for case in 0..18 {
            let mut row = good.clone();
            match case {
                0 => row.history = None,
                1 => row.history.as_mut().unwrap().drained_owner_fence = None,
                2 => row.history.as_mut().unwrap().drained_owner_fence = Some("other:2".into()),
                3 => row.history.as_mut().unwrap().runtime_instance_id.clear(),
                4 => {
                    row.capture.as_mut().unwrap().owner_provenance =
                        RuntimeServiceTimingOwnerProvenance::Injected
                }
                5 => {
                    row.capture.as_mut().unwrap().load_disposition =
                        RuntimeServiceTimingLoadDisposition::Unknown
                }
                6 => row.capture.as_mut().unwrap().clock_epoch = "other-clock".into(),
                7 => row.outcome = RuntimeServiceTimingOutcome::Failed,
                8 => row.outcome = RuntimeServiceTimingOutcome::Abandoned,
                9 => {
                    row.lifecycle.as_mut().unwrap().termination =
                        RuntimeServiceTimingTermination::CancellationRequested
                }
                10 => {
                    row.lifecycle.as_mut().unwrap().termination =
                        RuntimeServiceTimingTermination::ShutdownRequested
                }
                11 => {
                    row.phases[0].value =
                        RuntimeServiceTimingValue::ConfiguredEstimate { elapsed_ns: 0 }
                }
                12 => row.phases[1].phase = row.phases[0].phase,
                13 => {
                    row.lifecycle
                        .as_mut()
                        .unwrap()
                        .interval
                        .as_mut()
                        .unwrap()
                        .worker_drained_at_ns = None
                }
                14 => {
                    row.lifecycle
                        .as_mut()
                        .unwrap()
                        .interval
                        .as_mut()
                        .unwrap()
                        .worker_drained_at_ns = Some(90)
                }
                15 => {
                    row.lifecycle
                        .as_mut()
                        .unwrap()
                        .interval
                        .as_mut()
                        .unwrap()
                        .started_at_ns = 99
                }
                16 => row.capture.as_mut().unwrap().observed_at_ns = 111,
                _ => {
                    row.history.as_mut().unwrap().profile =
                        RuntimeServiceTimingHistoryProfile::new("owner".into(), "e".repeat(64))
                            .unwrap()
                }
            }
            assert_eq!(
                row.fresh_production_history_interval_ns(&key, &clock(110), 10),
                None,
                "case {case}"
            );
        }
        let wire = serde_json::to_value(&good).unwrap();
        assert_eq!(
            serde_json::from_value::<RuntimeServiceTimingAttempt>(wire).unwrap(),
            good
        );
        assert!(RuntimeServiceTimingHistoryProfile::new("owner".into(), "F".repeat(64)).is_err());
        assert!(serde_json::from_str::<RuntimeServiceTimingHistoryProfile>(
            r#"{"owner_epoch":"owner","identity_fingerprint":"short","runtime_instance_id":"fake"}"#
        )
        .is_err());
    }
}
