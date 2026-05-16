use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

const BACKEND_KEY_MAX_LEN: usize = 64;
const RUNTIME_ID_MAX_LEN: usize = 96;
const RUNTIME_VARIANT_ID_MAX_LEN: usize = 96;
const DEVICE_ID_MAX_LEN: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct WorkflowExecutionPlanBackendKey(String);

impl WorkflowExecutionPlanBackendKey {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, WorkflowExecutionPlanSelectedFactError> {
        validate_identifier(
            "selected_backend_key",
            value.as_ref(),
            BACKEND_KEY_MAX_LEN,
            IdentifierSeparators {
                colon: false,
                dot: false,
            },
        )
        .map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct WorkflowExecutionPlanRuntimeId(String);

impl WorkflowExecutionPlanRuntimeId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, WorkflowExecutionPlanSelectedFactError> {
        validate_identifier(
            "selected_runtime_id",
            value.as_ref(),
            RUNTIME_ID_MAX_LEN,
            IdentifierSeparators {
                colon: false,
                dot: false,
            },
        )
        .map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct WorkflowExecutionPlanRuntimeVariantId(String);

impl WorkflowExecutionPlanRuntimeVariantId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, WorkflowExecutionPlanSelectedFactError> {
        validate_identifier(
            "selected_runtime_variant_id",
            value.as_ref(),
            RUNTIME_VARIANT_ID_MAX_LEN,
            IdentifierSeparators {
                colon: false,
                dot: true,
            },
        )
        .map(Self)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct WorkflowExecutionPlanDeviceId(String);

impl WorkflowExecutionPlanDeviceId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, WorkflowExecutionPlanSelectedFactError> {
        let device_id = validate_identifier(
            "selected_device_id",
            value.as_ref(),
            DEVICE_ID_MAX_LEN,
            IdentifierSeparators {
                colon: true,
                dot: false,
            },
        )?;
        if device_id == "auto" {
            return Err(WorkflowExecutionPlanSelectedFactError::ReservedIdentifier {
                field: "selected_device_id",
                value: device_id,
            });
        }
        Ok(Self(device_id))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

macro_rules! impl_selected_fact {
    ($type:ty) => {
        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $type {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $type {
            type Err = WorkflowExecutionPlanSelectedFactError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<&str> for $type {
            type Error = WorkflowExecutionPlanSelectedFactError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $type {
            type Error = WorkflowExecutionPlanSelectedFactError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

impl_selected_fact!(WorkflowExecutionPlanBackendKey);
impl_selected_fact!(WorkflowExecutionPlanRuntimeId);
impl_selected_fact!(WorkflowExecutionPlanRuntimeVariantId);
impl_selected_fact!(WorkflowExecutionPlanDeviceId);

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum WorkflowExecutionPlanSelectedFactError {
    #[error("{field} is required")]
    Missing { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    TooLong {
        field: &'static str,
        max_len: usize,
        actual_len: usize,
    },
    #[error("{field} contains an invalid identifier '{value}'")]
    InvalidIdentifier { field: &'static str, value: String },
    #[error("{field} uses reserved identifier '{value}'")]
    ReservedIdentifier { field: &'static str, value: String },
}

#[derive(Debug, Clone, Copy)]
struct IdentifierSeparators {
    colon: bool,
    dot: bool,
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    max_len: usize,
    separators: IdentifierSeparators,
) -> Result<String, WorkflowExecutionPlanSelectedFactError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(WorkflowExecutionPlanSelectedFactError::Missing { field });
    }
    if trimmed.len() > max_len {
        return Err(WorkflowExecutionPlanSelectedFactError::TooLong {
            field,
            max_len,
            actual_len: trimmed.len(),
        });
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return Err(WorkflowExecutionPlanSelectedFactError::Missing { field });
    };
    if !first.is_ascii_lowercase() {
        return Err(WorkflowExecutionPlanSelectedFactError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    let mut previous_was_separator = false;
    for ch in chars {
        let allowed_separator = matches!(ch, '_' | '-')
            || (separators.colon && ch == ':')
            || (separators.dot && ch == '.');
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            previous_was_separator = false;
            continue;
        }
        if allowed_separator && !previous_was_separator {
            previous_was_separator = true;
            continue;
        }
        return Err(WorkflowExecutionPlanSelectedFactError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    if previous_was_separator {
        return Err(WorkflowExecutionPlanSelectedFactError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    Ok(trimmed.to_string())
}
