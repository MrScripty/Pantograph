//! Backend-owned health assessment helpers.
//!
//! Hosts may own polling and transport errors, but degraded/unhealthy
//! interpretation must stay in backend Rust so adapters do not drift on
//! failure-threshold behavior.

use pantograph_runtime_identity::canonical_runtime_id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHealthProbe {
    Healthy {
        response_time_ms: u64,
    },
    Failed {
        reason: String,
        response_time_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHealthState {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHealthAssessment {
    pub healthy: bool,
    pub state: RuntimeHealthState,
    pub response_time_ms: Option<u64>,
    pub error: Option<String>,
    pub consecutive_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHealthAssessmentRecord {
    pub runtime_id: String,
    pub runtime_instance_id: Option<String>,
    pub assessment: RuntimeHealthAssessment,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeHealthAssessmentSnapshot {
    pub active: Option<RuntimeHealthAssessmentRecord>,
    pub embedding: Option<RuntimeHealthAssessmentRecord>,
}

pub fn runtime_health_assessment_record(
    runtime_id: Option<&str>,
    runtime_instance_id: Option<&str>,
    assessment: Option<RuntimeHealthAssessment>,
) -> Option<RuntimeHealthAssessmentRecord> {
    let runtime_id = runtime_id
        .map(canonical_runtime_id)
        .filter(|runtime_id| !runtime_id.is_empty())?;
    let assessment = assessment?;

    Some(RuntimeHealthAssessmentRecord {
        runtime_id,
        runtime_instance_id: runtime_instance_id.map(ToOwned::to_owned),
        assessment,
    })
}

pub fn assess_runtime_health_probe(
    probe: RuntimeHealthProbe,
    previous_failures: u32,
    failure_threshold: u32,
) -> RuntimeHealthAssessment {
    match probe {
        RuntimeHealthProbe::Healthy { response_time_ms } => RuntimeHealthAssessment {
            healthy: true,
            state: RuntimeHealthState::Healthy,
            response_time_ms: Some(response_time_ms),
            error: None,
            consecutive_failures: 0,
        },
        RuntimeHealthProbe::Failed {
            reason,
            response_time_ms,
        } => {
            let consecutive_failures = previous_failures.saturating_add(1);
            let failure_threshold = failure_threshold.max(1);
            let healthy = consecutive_failures < failure_threshold;
            let state = if healthy {
                RuntimeHealthState::Degraded {
                    reason: reason.clone(),
                }
            } else {
                RuntimeHealthState::Unhealthy {
                    reason: reason.clone(),
                }
            };

            RuntimeHealthAssessment {
                healthy,
                state,
                response_time_ms,
                error: Some(reason),
                consecutive_failures,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        assess_runtime_health_probe, runtime_health_assessment_record, RuntimeHealthProbe,
        RuntimeHealthState,
    };

    #[test]
    fn successful_probe_resets_failures_and_reports_healthy() {
        let assessment = assess_runtime_health_probe(
            RuntimeHealthProbe::Healthy {
                response_time_ms: 42,
            },
            3,
            3,
        );

        assert!(assessment.healthy);
        assert_eq!(assessment.state, RuntimeHealthState::Healthy);
        assert_eq!(assessment.response_time_ms, Some(42));
        assert_eq!(assessment.error, None);
        assert_eq!(assessment.consecutive_failures, 0);
    }

    #[test]
    fn failure_below_threshold_reports_degraded() {
        let assessment = assess_runtime_health_probe(
            RuntimeHealthProbe::Failed {
                reason: "HTTP 503".to_string(),
                response_time_ms: Some(87),
            },
            0,
            3,
        );

        assert!(assessment.healthy);
        assert_eq!(
            assessment.state,
            RuntimeHealthState::Degraded {
                reason: "HTTP 503".to_string(),
            }
        );
        assert_eq!(assessment.response_time_ms, Some(87));
        assert_eq!(assessment.error.as_deref(), Some("HTTP 503"));
        assert_eq!(assessment.consecutive_failures, 1);
    }

    #[test]
    fn failure_at_threshold_reports_unhealthy() {
        let assessment = assess_runtime_health_probe(
            RuntimeHealthProbe::Failed {
                reason: "Request timeout".to_string(),
                response_time_ms: None,
            },
            2,
            3,
        );

        assert!(!assessment.healthy);
        assert_eq!(
            assessment.state,
            RuntimeHealthState::Unhealthy {
                reason: "Request timeout".to_string(),
            }
        );
        assert_eq!(assessment.response_time_ms, None);
        assert_eq!(assessment.error.as_deref(), Some("Request timeout"));
        assert_eq!(assessment.consecutive_failures, 3);
    }

    #[test]
    fn zero_threshold_is_treated_as_immediate_failure() {
        let assessment = assess_runtime_health_probe(
            RuntimeHealthProbe::Failed {
                reason: "Connection refused".to_string(),
                response_time_ms: None,
            },
            0,
            0,
        );

        assert!(!assessment.healthy);
        assert_eq!(
            assessment.state,
            RuntimeHealthState::Unhealthy {
                reason: "Connection refused".to_string(),
            }
        );
        assert_eq!(assessment.consecutive_failures, 1);
    }

    #[test]
    fn runtime_health_assessment_record_canonicalizes_runtime_id() {
        let record = runtime_health_assessment_record(
            Some("PyTorch"),
            Some("pytorch-1"),
            Some(assess_runtime_health_probe(
                RuntimeHealthProbe::Healthy {
                    response_time_ms: 42,
                },
                0,
                3,
            )),
        )
        .expect("record should build");

        assert_eq!(record.runtime_id, "pytorch");
        assert_eq!(record.runtime_instance_id.as_deref(), Some("pytorch-1"));
        assert!(record.assessment.healthy);
    }
}
