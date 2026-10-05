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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeServiceTimingUnavailableReason {
    ModelContentIdentityUnavailable,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeServiceTimingAttempt {
    pub attempt_id: String,
    pub execution_request_id: String,
    pub identity: RuntimeServiceTimingIdentity,
    pub outcome: RuntimeServiceTimingOutcome,
    pub phases: Vec<RuntimeServiceTimingPhaseEvidence>,
}

impl RuntimeServiceTimingAttempt {
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
            attempt_id: "attempt".into(),
            execution_request_id: "request".into(),
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
}
