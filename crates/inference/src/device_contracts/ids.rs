use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::DeviceContractError;

const DEVICE_ID_MAX_LEN: usize = 96;
const RUNTIME_VARIANT_ID_MAX_LEN: usize = 96;
const BACKEND_ID_MAX_LEN: usize = 64;

/// A validated concrete device id such as `cpu`, `cuda:0`, `metal:0`, or `mps`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct InferenceDeviceId(String);

impl InferenceDeviceId {
    /// Parse and validate a concrete device id.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DeviceContractError> {
        validate_identifier(
            "device_id",
            value.as_ref(),
            DEVICE_ID_MAX_LEN,
            IdentifierSeparators {
                colon: true,
                dot: false,
            },
        )
        .map(Self)
    }

    /// Borrow the validated id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for InferenceDeviceId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for InferenceDeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for InferenceDeviceId {
    type Err = DeviceContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for InferenceDeviceId {
    type Error = DeviceContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for InferenceDeviceId {
    type Error = DeviceContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for InferenceDeviceId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InferenceDeviceId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// A validated runtime variant id such as `llama_cpp.cpu` or `pytorch.cuda`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct RuntimeVariantId(String);

impl RuntimeVariantId {
    /// Parse and validate a runtime variant id.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DeviceContractError> {
        validate_identifier(
            "runtime_variant_id",
            value.as_ref(),
            RUNTIME_VARIANT_ID_MAX_LEN,
            IdentifierSeparators {
                colon: false,
                dot: true,
            },
        )
        .map(Self)
    }

    /// Borrow the validated id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for RuntimeVariantId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for RuntimeVariantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeVariantId {
    type Err = DeviceContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for RuntimeVariantId {
    type Error = DeviceContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for RuntimeVariantId {
    type Error = DeviceContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for RuntimeVariantId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RuntimeVariantId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// A validated backend id such as `llama_cpp`, `pytorch`, or `vllm`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct BackendId(String);

impl BackendId {
    /// Parse and validate a backend id.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DeviceContractError> {
        validate_identifier(
            "backend_id",
            value.as_ref(),
            BACKEND_ID_MAX_LEN,
            IdentifierSeparators {
                colon: false,
                dot: false,
            },
        )
        .map(Self)
    }

    /// Borrow the validated id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BackendId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BackendId {
    type Err = DeviceContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for BackendId {
    type Error = DeviceContractError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for BackendId {
    type Error = DeviceContractError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for BackendId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for BackendId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
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
) -> Result<String, DeviceContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DeviceContractError::EmptyIdentifier { field });
    }
    if trimmed.len() > max_len {
        return Err(DeviceContractError::IdentifierTooLong {
            field,
            max_len,
            actual_len: trimmed.len(),
        });
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return Err(DeviceContractError::EmptyIdentifier { field });
    };
    if !first.is_ascii_lowercase() {
        return Err(DeviceContractError::InvalidIdentifier {
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
        return Err(DeviceContractError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    if previous_was_separator {
        return Err(DeviceContractError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    Ok(trimmed.to_string())
}
