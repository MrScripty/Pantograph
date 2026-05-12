use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

const TIMING_ATTEMPT_ID_PREFIX: &str = "timing_attempt_";
const MAX_TIMING_ATTEMPT_ID_LEN: usize = 128;

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum WorkflowTimingContractError {
    #[error("timing attempt id is required")]
    MissingAttemptId,

    #[error("timing attempt id must start with timing_attempt_")]
    InvalidAttemptIdPrefix,

    #[error("timing attempt id exceeds {max_len} bytes")]
    AttemptIdTooLong { max_len: usize },

    #[error("timing attempt id contains control characters")]
    InvalidAttemptIdCharacters,

    #[error(
        "timing duration underflow for {attempt_id}: completed_at_ms {completed_at_ms} is before started_at_ms {started_at_ms}"
    )]
    DurationUnderflow {
        attempt_id: WorkflowTimingAttemptId,
        started_at_ms: u64,
        completed_at_ms: u64,
    },
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkflowTimingAttemptId(String);

impl WorkflowTimingAttemptId {
    #[must_use]
    pub fn generate() -> Self {
        Self(format!("{TIMING_ATTEMPT_ID_PREFIX}{}", Uuid::new_v4()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkflowTimingAttemptId {
    type Error = WorkflowTimingContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_timing_attempt_id(value).map(Self)
    }
}

impl FromStr for WorkflowTimingAttemptId {
    type Err = WorkflowTimingContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl From<WorkflowTimingAttemptId> for String {
    fn from(value: WorkflowTimingAttemptId) -> Self {
        value.0
    }
}

impl Serialize for WorkflowTimingAttemptId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WorkflowTimingAttemptId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Debug for WorkflowTimingAttemptId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WorkflowTimingAttemptId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for WorkflowTimingAttemptId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingAttemptKind {
    RuntimeModelLoad,
    RuntimeModelUnload,
    RuntimeWarmup,
    SchedulerTraceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowTimingAttribution {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_execution_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_variant_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTimingDiagnosticCode {
    TimestampUnderflow,
    TimestampOverflow,
    TimingBaselineUnavailable,
    TimingBaselineExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowTimingDiagnostic {
    pub code: WorkflowTimingDiagnosticCode,
    pub severity: WorkflowTimingDiagnosticSeverity,
    pub message: String,
    pub attempt_id: WorkflowTimingAttemptId,
    pub attempt_kind: WorkflowTimingAttemptKind,
}

impl WorkflowTimingDiagnostic {
    #[must_use]
    pub fn from_contract_error(
        error: &WorkflowTimingContractError,
        attempt_kind: WorkflowTimingAttemptKind,
    ) -> Option<Self> {
        match error {
            WorkflowTimingContractError::DurationUnderflow { attempt_id, .. } => Some(Self {
                code: WorkflowTimingDiagnosticCode::TimestampUnderflow,
                severity: WorkflowTimingDiagnosticSeverity::Error,
                message: error.to_string(),
                attempt_id: attempt_id.clone(),
                attempt_kind,
            }),
            WorkflowTimingContractError::MissingAttemptId
            | WorkflowTimingContractError::InvalidAttemptIdPrefix
            | WorkflowTimingContractError::AttemptIdTooLong { .. }
            | WorkflowTimingContractError::InvalidAttemptIdCharacters => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub struct WorkflowTimingAttemptRecord {
    pub attempt_id: WorkflowTimingAttemptId,
    pub attempt_kind: WorkflowTimingAttemptKind,
    pub attribution: WorkflowTimingAttribution,
    pub started_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<WorkflowTimingDiagnostic>,
}

impl WorkflowTimingAttemptRecord {
    #[must_use]
    pub fn started(
        attempt_id: WorkflowTimingAttemptId,
        attempt_kind: WorkflowTimingAttemptKind,
        attribution: WorkflowTimingAttribution,
        started_at_ms: u64,
    ) -> Self {
        Self {
            attempt_id,
            attempt_kind,
            attribution,
            started_at_ms,
            completed_at_ms: None,
            duration_ms: None,
            diagnostics: Vec::new(),
        }
    }

    pub fn completed(
        attempt_id: WorkflowTimingAttemptId,
        attempt_kind: WorkflowTimingAttemptKind,
        attribution: WorkflowTimingAttribution,
        started_at_ms: u64,
        completed_at_ms: u64,
    ) -> Result<Self, WorkflowTimingContractError> {
        let duration_ms = checked_timing_duration_ms(&attempt_id, started_at_ms, completed_at_ms)?;
        Ok(Self {
            attempt_id,
            attempt_kind,
            attribution,
            started_at_ms,
            completed_at_ms: Some(completed_at_ms),
            duration_ms: Some(duration_ms),
            diagnostics: Vec::new(),
        })
    }
}

pub fn checked_timing_duration_ms(
    attempt_id: &WorkflowTimingAttemptId,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> Result<u64, WorkflowTimingContractError> {
    completed_at_ms.checked_sub(started_at_ms).ok_or_else(|| {
        WorkflowTimingContractError::DurationUnderflow {
            attempt_id: attempt_id.clone(),
            started_at_ms,
            completed_at_ms,
        }
    })
}

fn validate_timing_attempt_id(value: String) -> Result<String, WorkflowTimingContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WorkflowTimingContractError::MissingAttemptId);
    }
    if trimmed.len() > MAX_TIMING_ATTEMPT_ID_LEN {
        return Err(WorkflowTimingContractError::AttemptIdTooLong {
            max_len: MAX_TIMING_ATTEMPT_ID_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(WorkflowTimingContractError::InvalidAttemptIdCharacters);
    }
    if !trimmed.starts_with(TIMING_ATTEMPT_ID_PREFIX) {
        return Err(WorkflowTimingContractError::InvalidAttemptIdPrefix);
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_timing_attempt_record_serializes_with_attribution() {
        let attempt_id = WorkflowTimingAttemptId::try_from(
            "timing_attempt_018f753a-7c2d-72e9-a3d1-a422da29c7cf".to_string(),
        )
        .expect("valid attempt id");
        let record = WorkflowTimingAttemptRecord::completed(
            attempt_id.clone(),
            WorkflowTimingAttemptKind::RuntimeModelLoad,
            WorkflowTimingAttribution {
                workflow_id: Some("workflow-1".to_string()),
                workflow_run_id: Some("run-1".to_string()),
                workflow_execution_session_id: Some("session-1".to_string()),
                runtime_id: Some("pytorch".to_string()),
                runtime_variant_id: Some("pytorch_cuda".to_string()),
                model_id: Some("model-1".to_string()),
                backend_key: Some("diffusers".to_string()),
                device_class: Some("cuda".to_string()),
                device_id: Some("cuda:0".to_string()),
            },
            100,
            175,
        )
        .expect("completed timing record");

        let value = serde_json::to_value(&record).expect("serialize timing record");
        assert_eq!(
            value,
            serde_json::json!({
                "attempt_id": attempt_id.as_str(),
                "attempt_kind": "runtime_model_load",
                "attribution": {
                    "workflow_id": "workflow-1",
                    "workflow_run_id": "run-1",
                    "workflow_execution_session_id": "session-1",
                    "runtime_id": "pytorch",
                    "runtime_variant_id": "pytorch_cuda",
                    "model_id": "model-1",
                    "backend_key": "diffusers",
                    "device_class": "cuda",
                    "device_id": "cuda:0"
                },
                "started_at_ms": 100,
                "completed_at_ms": 175,
                "duration_ms": 75
            })
        );

        let parsed: WorkflowTimingAttemptRecord =
            serde_json::from_value(value).expect("deserialize timing record");
        assert_eq!(parsed, record);
    }

    #[test]
    fn workflow_timing_attempt_id_rejects_invalid_prefix() {
        let error =
            WorkflowTimingAttemptId::try_from("attempt-1".to_string()).expect_err("invalid prefix");
        assert_eq!(error, WorkflowTimingContractError::InvalidAttemptIdPrefix);
    }

    #[test]
    fn checked_timing_duration_reports_underflow_diagnostic() {
        let attempt_id = WorkflowTimingAttemptId::try_from(
            "timing_attempt_018f753a-7c2d-72e9-a3d1-a422da29c7cf".to_string(),
        )
        .expect("valid attempt id");
        let error = checked_timing_duration_ms(&attempt_id, 200, 199).expect_err("underflow error");
        assert_eq!(
            error,
            WorkflowTimingContractError::DurationUnderflow {
                attempt_id: attempt_id.clone(),
                started_at_ms: 200,
                completed_at_ms: 199
            }
        );

        let diagnostic = WorkflowTimingDiagnostic::from_contract_error(
            &error,
            WorkflowTimingAttemptKind::RuntimeWarmup,
        )
        .expect("diagnostic");
        assert_eq!(
            diagnostic.code,
            WorkflowTimingDiagnosticCode::TimestampUnderflow
        );
        assert_eq!(diagnostic.severity, WorkflowTimingDiagnosticSeverity::Error);
        assert_eq!(diagnostic.attempt_id, attempt_id);
        assert!(diagnostic.message.contains("completed_at_ms 199"));
    }
}
