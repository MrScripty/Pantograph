use pantograph_runtime_attribution::WorkflowId;
use serde::{Deserialize, Serialize};

use crate::util::{validate_optional_text, validate_required_text, MAX_ID_LEN};
use crate::DiagnosticsLedgerError;

pub const RUNTIME_SELECTION_HISTORY_MIN_SAMPLE_COUNT: u32 = 5;
pub const RUNTIME_SELECTION_HISTORY_MAX_SAMPLE_LIMIT: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSelectionHistoryKey {
    pub workflow_id: WorkflowId,
    pub task_id: String,
    pub model_id: String,
    pub selected_backend_key: String,
    pub selected_runtime_variant_id: String,
    pub selected_device_class: String,
    pub selected_device_id: Option<String>,
}

impl RuntimeSelectionHistoryKey {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        validate_required_text("task_id", &self.task_id, MAX_ID_LEN)?;
        validate_required_text("model_id", &self.model_id, MAX_ID_LEN)?;
        validate_required_text(
            "selected_backend_key",
            &self.selected_backend_key,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "selected_runtime_variant_id",
            &self.selected_runtime_variant_id,
            MAX_ID_LEN,
        )?;
        validate_required_text(
            "selected_device_class",
            &self.selected_device_class,
            MAX_ID_LEN,
        )?;
        validate_optional_text(
            "selected_device_id",
            self.selected_device_id.as_deref(),
            MAX_ID_LEN,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSelectionHistoryQuery {
    pub key: RuntimeSelectionHistoryKey,
    pub min_sample_count: u32,
    pub sample_limit: u32,
}

impl RuntimeSelectionHistoryQuery {
    pub fn validate(&self) -> Result<(), DiagnosticsLedgerError> {
        self.key.validate()?;
        if self.min_sample_count == 0 {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "min_sample_count",
            });
        }
        if self.sample_limit < self.min_sample_count {
            return Err(DiagnosticsLedgerError::InvalidField {
                field: "sample_limit",
            });
        }
        if self.sample_limit > RUNTIME_SELECTION_HISTORY_MAX_SAMPLE_LIMIT {
            return Err(DiagnosticsLedgerError::QueryLimitExceeded {
                requested: self.sample_limit,
                max: RUNTIME_SELECTION_HISTORY_MAX_SAMPLE_LIMIT,
            });
        }
        Ok(())
    }
}

impl Default for RuntimeSelectionHistoryQuery {
    fn default() -> Self {
        Self {
            key: RuntimeSelectionHistoryKey {
                workflow_id: WorkflowId::try_from("workflow".to_string())
                    .expect("default workflow id is valid"),
                task_id: "task".to_string(),
                model_id: "model".to_string(),
                selected_backend_key: "backend".to_string(),
                selected_runtime_variant_id: "runtime.variant".to_string(),
                selected_device_class: "cpu".to_string(),
                selected_device_id: None,
            },
            min_sample_count: RUNTIME_SELECTION_HISTORY_MIN_SAMPLE_COUNT,
            sample_limit: RUNTIME_SELECTION_HISTORY_MAX_SAMPLE_LIMIT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSelectionHistorySummary {
    pub key: RuntimeSelectionHistoryKey,
    pub sample_count: u32,
    pub min_sample_count: u32,
    pub threshold_met: bool,
    pub completed_count: u32,
    pub failed_count: u32,
    pub cancelled_count: u32,
    pub duration_sample_count: u32,
    pub average_duration_ms: Option<u64>,
    pub median_duration_ms: Option<u64>,
    pub typical_min_duration_ms: Option<u64>,
    pub typical_max_duration_ms: Option<u64>,
    pub queue_wait_sample_count: u32,
    pub average_queue_wait_ms: Option<u64>,
    pub median_queue_wait_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSelectionHistoryRunStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeSelectionHistorySample {
    pub status: RuntimeSelectionHistoryRunStatus,
    pub duration_ms: Option<u64>,
    pub accepted_at_ms: Option<i64>,
    pub started_at_ms: Option<i64>,
}

impl RuntimeSelectionHistorySummary {
    pub fn from_samples(
        query: &RuntimeSelectionHistoryQuery,
        samples: Vec<RuntimeSelectionHistorySample>,
    ) -> Result<Self, DiagnosticsLedgerError> {
        let mut completed_count = 0_u32;
        let mut failed_count = 0_u32;
        let mut cancelled_count = 0_u32;
        let mut durations_ms = Vec::new();
        let mut queue_wait_ms = Vec::new();

        for sample in &samples {
            match sample.status {
                RuntimeSelectionHistoryRunStatus::Completed => {
                    completed_count = increment_count(completed_count, "completed_count")?;
                    if let Some(duration_ms) = sample.duration_ms {
                        durations_ms.push(duration_ms);
                    }
                }
                RuntimeSelectionHistoryRunStatus::Failed => {
                    failed_count = increment_count(failed_count, "failed_count")?;
                }
                RuntimeSelectionHistoryRunStatus::Cancelled => {
                    cancelled_count = increment_count(cancelled_count, "cancelled_count")?;
                }
            }
            if let (Some(accepted_at_ms), Some(started_at_ms)) =
                (sample.accepted_at_ms, sample.started_at_ms)
            {
                if started_at_ms < accepted_at_ms {
                    return Err(DiagnosticsLedgerError::InvalidTimeRange);
                }
                queue_wait_ms.push((started_at_ms - accepted_at_ms) as u64);
            }
        }

        durations_ms.sort_unstable();
        queue_wait_ms.sort_unstable();
        let sample_count =
            u32::try_from(samples.len()).map_err(|_| DiagnosticsLedgerError::InvalidField {
                field: "sample_count",
            })?;
        Ok(Self {
            key: query.key.clone(),
            sample_count,
            min_sample_count: query.min_sample_count,
            threshold_met: sample_count >= query.min_sample_count,
            completed_count,
            failed_count,
            cancelled_count,
            duration_sample_count: u32::try_from(durations_ms.len()).map_err(|_| {
                DiagnosticsLedgerError::InvalidField {
                    field: "duration_sample_count",
                }
            })?,
            average_duration_ms: average_u64(&durations_ms),
            median_duration_ms: percentile_nearest_rank(&durations_ms, 50),
            typical_min_duration_ms: percentile_nearest_rank(&durations_ms, 25),
            typical_max_duration_ms: percentile_nearest_rank(&durations_ms, 75),
            queue_wait_sample_count: u32::try_from(queue_wait_ms.len()).map_err(|_| {
                DiagnosticsLedgerError::InvalidField {
                    field: "queue_wait_sample_count",
                }
            })?,
            average_queue_wait_ms: average_u64(&queue_wait_ms),
            median_queue_wait_ms: percentile_nearest_rank(&queue_wait_ms, 50),
        })
    }
}

fn increment_count(count: u32, field: &'static str) -> Result<u32, DiagnosticsLedgerError> {
    count
        .checked_add(1)
        .ok_or(DiagnosticsLedgerError::InvalidField { field })
}

fn average_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let sum = values
        .iter()
        .fold(0_u128, |sum, value| sum + u128::from(*value));
    Some((sum / values.len() as u128) as u64)
}

fn percentile_nearest_rank(sorted_values: &[u64], percentile: usize) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank = (percentile * (sorted_values.len().saturating_sub(1)) + 50) / 100;
    Some(sorted_values[rank])
}
