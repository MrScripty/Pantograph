use std::fmt;
use std::str::FromStr;

use pantograph_dependency_planning::PumasModelRef;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

pub const INFERENCE_INTERFACE_CONTRACT_VERSION: u32 = 1;

pub(crate) const MAX_ID_LEN: usize = 128;
pub(crate) const MAX_LABEL_LEN: usize = 256;
pub(crate) const MAX_MESSAGE_LEN: usize = 1024;
pub(crate) const MAX_PORTS: usize = 128;
pub(crate) const MAX_OPTIONS: usize = 512;
pub(crate) const MAX_DIAGNOSTICS: usize = 128;
pub(crate) const MAX_REASONS: usize = 32;
pub(crate) const MAX_CHANGES: usize = 256;
pub(crate) const MAX_BINDINGS: usize = 256;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InferenceInterfaceContractError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} exceeds maximum length {max_len}")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("{field} contains {actual_len} items; maximum is {max_len}")]
    TooManyItems {
        field: &'static str,
        actual_len: usize,
        max_len: usize,
    },
    #[error("{field} is invalid: {reason}")]
    InvalidField {
        field: &'static str,
        reason: &'static str,
    },
    #[error("unsupported inference interface contract version {actual}; expected {expected}")]
    UnsupportedContractVersion { actual: u32, expected: u32 },
}

macro_rules! validated_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[must_use]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl AsRef<str>) -> Result<Self, InferenceInterfaceContractError> {
                validate_identifier($field, value.as_ref()).map(Self)
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl FromStr for $name {
            type Err = InferenceInterfaceContractError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InferenceInterfaceContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
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

validated_id!(InferenceTaskKind, "task_kind");
validated_id!(InferenceInterfaceFingerprint, "descriptor_fingerprint");
validated_id!(InferencePortId, "port_id");
validated_id!(InferenceOptionId, "option_id");
validated_id!(WorkflowGraphSessionId, "graph_session_id");
validated_id!(WorkflowGraphRevision, "graph_revision");
validated_id!(WorkflowNodeId, "node_id");
validated_id!(DraftGraphValidationSessionId, "validation_session_id");

pub(crate) fn default_contract_version() -> u32 {
    INFERENCE_INTERFACE_CONTRACT_VERSION
}

pub(crate) fn validate_contract_version(
    version: u32,
) -> Result<(), InferenceInterfaceContractError> {
    if version == INFERENCE_INTERFACE_CONTRACT_VERSION {
        return Ok(());
    }
    Err(
        InferenceInterfaceContractError::UnsupportedContractVersion {
            actual: version,
            expected: INFERENCE_INTERFACE_CONTRACT_VERSION,
        },
    )
}

pub(crate) fn validate_model_ref(
    field: &'static str,
    model_ref: &PumasModelRef,
) -> Result<(), InferenceInterfaceContractError> {
    model_ref
        .validate()
        .map_err(|_| InferenceInterfaceContractError::InvalidField {
            field,
            reason: "model reference failed dependency-planning validation",
        })
}

pub(crate) fn validate_collection_len(
    field: &'static str,
    actual_len: usize,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    if actual_len > max_len {
        return Err(InferenceInterfaceContractError::TooManyItems {
            field,
            actual_len,
            max_len,
        });
    }
    Ok(())
}

pub(crate) fn validate_identifier(
    field: &'static str,
    value: &str,
) -> Result<String, InferenceInterfaceContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(InferenceInterfaceContractError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(InferenceInterfaceContractError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(InferenceInterfaceContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_text(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(InferenceInterfaceContractError::MissingField { field });
    }
    if trimmed.len() > max_len {
        return Err(InferenceInterfaceContractError::FieldTooLong { field, max_len });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(InferenceInterfaceContractError::InvalidText { field });
    }
    Ok(())
}

pub(crate) fn validate_optional_text(
    field: &'static str,
    value: Option<&str>,
    max_len: usize,
) -> Result<(), InferenceInterfaceContractError> {
    match value {
        Some(value) => validate_text(field, value, max_len),
        None => Ok(()),
    }
}
