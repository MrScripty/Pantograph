use serde::{Deserialize, Serialize};

use crate::DiagnosticsLedgerError;
use crate::util::{MAX_ID_LEN, validate_optional_text, validate_required_text};

pub const MIN_TIMING_EXPECTATION_SAMPLE_COUNT: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingObservationScope {
    Run,
    Node,
}

impl WorkflowTimingObservationScope {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Node => "node",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "run" => Ok(Self::Run),
            "node" => Ok(Self::Node),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "observation_scope",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingObservationStatus {
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowTimingObservationStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "observation_status",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingExpectationComparison {
    InsufficientHistory,
    NoCurrentDuration,
    FasterThanExpected,
    WithinExpectedRange,
    SlowerThanExpected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTimingObservation {
    pub observation_key: String,
    pub scope: WorkflowTimingObservationScope,
    pub workflow_run_id: String,
    pub workflow_id: String,
    pub graph_fingerprint: String,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub runtime_id: Option<String>,
    pub status: WorkflowTimingObservationStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub duration_ms: u64,
    pub recorded_at_ms: i64,
}

impl WorkflowTimingObservation {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("observation_key", &self.observation_key, MAX_ID_LEN)?;
        validate_required_text("workflow_run_id", &self.workflow_run_id, MAX_ID_LEN)?;
        validate_required_text("workflow_id", &self.workflow_id, MAX_ID_LEN)?;
        validate_required_text("graph_fingerprint", &self.graph_fingerprint, MAX_ID_LEN)?;
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("node_type", self.node_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("runtime_id", self.runtime_id.as_deref(), MAX_ID_LEN)?;
        if self.ended_at_ms < self.started_at_ms {
            return Err(DiagnosticsLedgerError::InvalidTimeRange);
        }
        if matches!(self.scope, WorkflowTimingObservationScope::Node)
            && self.node_id.as_deref().unwrap_or_default().is_empty()
        {
            return Err(DiagnosticsLedgerError::MissingField { field: "node_id" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTimingExpectationQuery {
    pub scope: WorkflowTimingObservationScope,
    pub workflow_id: String,
    pub graph_fingerprint: String,
    pub node_id: Option<String>,
    pub node_type: Option<String>,
    pub runtime_id: Option<String>,
    pub current_duration_ms: Option<u64>,
    pub current_duration_is_complete: bool,
}

impl WorkflowTimingExpectationQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("workflow_id", &self.workflow_id, MAX_ID_LEN)?;
        validate_required_text("graph_fingerprint", &self.graph_fingerprint, MAX_ID_LEN)?;
        validate_optional_text("node_id", self.node_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("node_type", self.node_type.as_deref(), MAX_ID_LEN)?;
        validate_optional_text("runtime_id", self.runtime_id.as_deref(), MAX_ID_LEN)?;
        if matches!(self.scope, WorkflowTimingObservationScope::Node)
            && self.node_id.as_deref().unwrap_or_default().is_empty()
        {
            return Err(DiagnosticsLedgerError::MissingField { field: "node_id" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowTimingExpectation {
    pub comparison: WorkflowTimingExpectationComparison,
    pub sample_count: usize,
    pub current_duration_ms: Option<u64>,
    pub median_duration_ms: Option<u64>,
    pub typical_min_duration_ms: Option<u64>,
    pub typical_max_duration_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneTimingObservationsCommand {
    pub prune_recorded_before_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PruneTimingObservationsResult {
    pub pruned_observation_count: u64,
    pub prune_recorded_before_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunSummaryStatus {
    Queued,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowRunSummaryStatus {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, DiagnosticsLedgerError> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "waiting" => Ok(Self::Waiting),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(DiagnosticsLedgerError::InvalidField {
                field: "workflow_run_status",
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunSummaryRecord {
    pub workflow_run_id: String,
    pub workflow_id: String,
    pub session_id: Option<String>,
    pub graph_fingerprint: Option<String>,
    pub status: WorkflowRunSummaryStatus,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub duration_ms: Option<u64>,
    pub node_count_at_start: usize,
    pub event_count: usize,
    pub last_error: Option<String>,
    pub recorded_at_ms: i64,
}

impl WorkflowRunSummaryRecord {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("workflow_run_id", &self.workflow_run_id, MAX_ID_LEN)?;
        validate_required_text("workflow_id", &self.workflow_id, MAX_ID_LEN)?;
        validate_optional_text("session_id", self.session_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "graph_fingerprint",
            self.graph_fingerprint.as_deref(),
            MAX_ID_LEN,
        )?;
        validate_optional_text("last_error", self.last_error.as_deref(), MAX_ID_LEN)?;
        if let Some(ended_at_ms) = self.ended_at_ms {
            if ended_at_ms < self.started_at_ms {
                return Err(DiagnosticsLedgerError::InvalidTimeRange);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunSummaryQuery {
    pub workflow_id: Option<String>,
    pub workflow_run_id: Option<String>,
    pub limit: usize,
}

impl WorkflowRunSummaryQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_optional_text("workflow_id", self.workflow_id.as_deref(), MAX_ID_LEN)?;
        validate_optional_text(
            "workflow_run_id",
            self.workflow_run_id.as_deref(),
            MAX_ID_LEN,
        )?;
        if self.limit == 0 || self.limit > 500 {
            return Err(DiagnosticsLedgerError::InvalidField { field: "limit" });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRunSummaryProjection {
    pub runs: Vec<WorkflowRunSummaryRecord>,
}

impl WorkflowTimingExpectation {
    pub fn from_completed_durations(
        query: &WorkflowTimingExpectationQuery,
        mut durations_ms: Vec<u64>,
    ) -> Self {
        durations_ms.sort_unstable();
        let sample_count = durations_ms.len();
        if sample_count < MIN_TIMING_EXPECTATION_SAMPLE_COUNT {
            return Self {
                comparison: WorkflowTimingExpectationComparison::InsufficientHistory,
                sample_count,
                current_duration_ms: query.current_duration_ms,
                median_duration_ms: None,
                typical_min_duration_ms: None,
                typical_max_duration_ms: None,
            };
        }

        let typical_min_duration_ms = percentile_nearest_rank(&durations_ms, 25);
        let median_duration_ms = percentile_nearest_rank(&durations_ms, 50);
        let typical_max_duration_ms = percentile_nearest_rank(&durations_ms, 75);
        let comparison = match (
            query.current_duration_ms,
            query.current_duration_is_complete,
        ) {
            (Some(current_duration_ms), false) if current_duration_ms > typical_max_duration_ms => {
                WorkflowTimingExpectationComparison::SlowerThanExpected
            }
            (Some(_), false) => WorkflowTimingExpectationComparison::WithinExpectedRange,
            (Some(current_duration_ms), true) if current_duration_ms < typical_min_duration_ms => {
                WorkflowTimingExpectationComparison::FasterThanExpected
            }
            (Some(current_duration_ms), true) if current_duration_ms > typical_max_duration_ms => {
                WorkflowTimingExpectationComparison::SlowerThanExpected
            }
            (Some(_), true) => WorkflowTimingExpectationComparison::WithinExpectedRange,
            (None, _) => WorkflowTimingExpectationComparison::NoCurrentDuration,
        };

        Self {
            comparison,
            sample_count,
            current_duration_ms: query.current_duration_ms,
            median_duration_ms: Some(median_duration_ms),
            typical_min_duration_ms: Some(typical_min_duration_ms),
            typical_max_duration_ms: Some(typical_max_duration_ms),
        }
    }
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: usize) -> u64 {
    debug_assert!(!sorted_values.is_empty());
    let rank = (percentile * (sorted_values.len().saturating_sub(1)) + 50) / 100;
    sorted_values[rank]
}
