use serde::{Deserialize, Serialize};

use crate::util::{validate_required_text, MAX_ID_LEN};
use crate::DiagnosticsLedgerError;

/// A rolling journal, not a complete attempt audit or an unlimited calibration store.
pub const RUNTIME_HOST_OBSERVATION_STORED_LIMIT: u32 = 5_000;

/// Host request elapsed time includes resolution, gateway waiting, model setup
/// and output publication. It is not backend compute time or scheduler queue time.
/// Profiles match only within one host epoch and one exact request fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHostObservationProfile {
    pub host_epoch: String,
    pub request_fingerprint: String,
}

impl RuntimeHostObservationProfile {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("host_epoch", &self.host_epoch, MAX_ID_LEN)?;
        if self.request_fingerprint.len() != 64
            || !self
                .request_fingerprint
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "request_fingerprint",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeHostObservationOutcome {
    Completed,
    Failed,
    Rejected,
    CancellationAcknowledged,
    ShutdownAcknowledged,
    /// The calling future was dropped; this does not prove backend cancellation.
    Abandoned,
    /// Accepted is a nonterminal response, not a successful timing sample.
    Nonterminal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHostRequestObservation {
    pub observation_id: String,
    pub execution_request_id: String,
    pub workflow_id: String,
    pub workflow_run_id: String,
    pub task_id: String,
    pub profile: RuntimeHostObservationProfile,
    pub outcome: RuntimeHostObservationOutcome,
    /// Measured with a monotonic clock; wall-clock changes do not affect elapsed time.
    pub host_elapsed_ms: u64,
    /// UTC wall clock used only for query freshness and retention.
    pub recorded_at_ms: i64,
}

impl RuntimeHostRequestObservation {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        for (field, value) in [
            ("observation_id", &self.observation_id),
            ("execution_request_id", &self.execution_request_id),
            ("workflow_id", &self.workflow_id),
            ("workflow_run_id", &self.workflow_run_id),
            ("task_id", &self.task_id),
        ] {
            validate_required_text(field, value, MAX_ID_LEN)?;
        }
        self.profile.validate()?;
        if self.host_elapsed_ms > i64::MAX as u64 || self.recorded_at_ms < 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "observation_time",
            });
        }
        Ok(())
    }
}

/// Exact profile matching, bounded sample count and explicit UTC freshness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHostObservationQuery {
    pub profile: RuntimeHostObservationProfile,
    pub since_ms: i64,
    pub until_ms: i64,
    pub sample_limit: u32,
}

impl RuntimeHostObservationQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        self.profile.validate()?;
        if self.since_ms < 0 || self.until_ms < self.since_ms {
            return Err(DiagnosticsLedgerError::InvalidTimeRange);
        }
        if self.sample_limit == 0 || self.sample_limit > 500 {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.sample_limit,
                max: 500,
            });
        }
        Ok(())
    }
}

/// Successful host elapsed samples only. Missing phases remain unavailable;
/// consumers must not interpret this aggregate as calibrated hardware service time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHostObservationSummary {
    pub profile: RuntimeHostObservationProfile,
    pub observed_count: u32,
    pub completed_count: u32,
    pub other_outcome_count: u32,
    pub median_completed_host_elapsed_ms: Option<u64>,
}

impl RuntimeHostObservationSummary {
    pub(crate) fn from_observations(
        profile: RuntimeHostObservationProfile,
        observations: Vec<RuntimeHostRequestObservation>,
    ) -> Self {
        let mut completed: Vec<_> = observations
            .iter()
            .filter(|observation| observation.outcome == RuntimeHostObservationOutcome::Completed)
            .map(|observation| observation.host_elapsed_ms)
            .collect();
        completed.sort_unstable();
        let median = match completed.len() {
            0 => None,
            length if length % 2 == 1 => Some(completed[length / 2]),
            length => {
                let low = completed[length / 2 - 1];
                let high = completed[length / 2];
                Some(low + (high - low) / 2)
            }
        };
        Self {
            profile,
            observed_count: observations.len() as u32,
            completed_count: completed.len() as u32,
            other_outcome_count: (observations.len() - completed.len()) as u32,
            median_completed_host_elapsed_ms: median,
        }
    }
}
