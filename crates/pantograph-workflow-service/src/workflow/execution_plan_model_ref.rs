use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

const PUMAS_MODEL_REF_PREFIX: &str = "pumas://models/";
const MAX_MODEL_ID_LEN: usize = 480;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct WorkflowExecutionPlanModelRef(String);

impl WorkflowExecutionPlanModelRef {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, WorkflowExecutionPlanModelRefError> {
        let raw = value.as_ref().trim();
        if raw.is_empty() {
            return Err(WorkflowExecutionPlanModelRefError::Missing);
        }
        if raw.chars().any(char::is_control) {
            return Err(WorkflowExecutionPlanModelRefError::InvalidCharacters);
        }

        let model_id = raw
            .strip_prefix(PUMAS_MODEL_REF_PREFIX)
            .unwrap_or(raw)
            .trim();
        if model_id.is_empty() {
            return Err(WorkflowExecutionPlanModelRefError::MissingModelId);
        }
        if model_id.len() > MAX_MODEL_ID_LEN {
            return Err(WorkflowExecutionPlanModelRefError::TooLong {
                max_len: MAX_MODEL_ID_LEN,
            });
        }
        if raw.contains("://") && !raw.starts_with(PUMAS_MODEL_REF_PREFIX) {
            return Err(WorkflowExecutionPlanModelRefError::UnsupportedUri);
        }
        validate_model_id_segments(model_id)?;

        Ok(Self(format!("{PUMAS_MODEL_REF_PREFIX}{model_id}")))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for WorkflowExecutionPlanModelRef {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for WorkflowExecutionPlanModelRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkflowExecutionPlanModelRef {
    type Err = WorkflowExecutionPlanModelRefError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for WorkflowExecutionPlanModelRef {
    type Error = WorkflowExecutionPlanModelRefError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for WorkflowExecutionPlanModelRef {
    type Error = WorkflowExecutionPlanModelRefError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for WorkflowExecutionPlanModelRef {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for WorkflowExecutionPlanModelRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowExecutionPlanModelRefError {
    #[error("selected model ref is required")]
    Missing,
    #[error("selected model ref is missing a model id")]
    MissingModelId,
    #[error("selected model ref exceeds maximum model id length {max_len}")]
    TooLong { max_len: usize },
    #[error("selected model ref contains invalid characters")]
    InvalidCharacters,
    #[error("selected model ref uses an unsupported URI scheme")]
    UnsupportedUri,
    #[error("selected model ref must not be a local or relative filesystem path")]
    LocalPath,
    #[error("selected model ref contains an invalid path segment")]
    InvalidSegment,
}

fn validate_model_id_segments(model_id: &str) -> Result<(), WorkflowExecutionPlanModelRefError> {
    if model_id.starts_with(['/', '\\', '~']) || model_id.contains('\\') {
        return Err(WorkflowExecutionPlanModelRefError::LocalPath);
    }

    let mut saw_segment = false;
    for segment in model_id.split('/') {
        if segment.is_empty() {
            return Err(WorkflowExecutionPlanModelRefError::InvalidSegment);
        }
        if matches!(segment, "." | "..") {
            return Err(WorkflowExecutionPlanModelRefError::LocalPath);
        }
        saw_segment = true;
    }

    if saw_segment {
        Ok(())
    } else {
        Err(WorkflowExecutionPlanModelRefError::MissingModelId)
    }
}
