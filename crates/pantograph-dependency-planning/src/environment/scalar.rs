use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DependencyPlanningContractError;
use crate::request::DependencyBindingId;
use crate::result::DependencyPlanningDiagnostic;

const MAX_PROFILE_ID_LEN: usize = 128;
const MAX_REQUIREMENT_NAME_LEN: usize = 128;
const MAX_FIELD_PATH_LEN: usize = 256;
const MAX_DEPENDENCY_TEXT_LEN: usize = 256;

/// Dependency binding profile id.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct DependencyBindingProfileId(String);

impl DependencyBindingProfileId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
        validate_profile_id("dependency_binding_profile_id", value.as_ref()).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for DependencyBindingProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DependencyBindingProfileId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for DependencyBindingProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DependencyBindingProfileId {
    type Err = DependencyPlanningContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for DependencyBindingProfileId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DependencyBindingProfileId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Requirement name used by shared dependency payloads.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct DependencyRequirementName(String);

impl DependencyRequirementName {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
        validate_dependency_name("dependency_requirement.name", value.as_ref()).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for DependencyRequirementName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DependencyRequirementName")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for DependencyRequirementName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DependencyRequirementName {
    type Err = DependencyPlanningContractError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for DependencyRequirementName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DependencyRequirementName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Typed validation field path. This is a contract field path, not a file path.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[must_use]
pub struct DependencyValidationFieldPath(String);

impl DependencyValidationFieldPath {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, DependencyPlanningContractError> {
        validate_validation_field_path("dependency_validation_error.field_path", value.as_ref())
            .map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for DependencyValidationFieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DependencyValidationFieldPath")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for DependencyValidationFieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for DependencyValidationFieldPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DependencyValidationFieldPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Non-zero Unix epoch timestamp in milliseconds for dependency operations.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
#[must_use]
pub struct DependencyOperationTimestampMs(u64);

impl DependencyOperationTimestampMs {
    pub fn parse(value: u64) -> Result<Self, DependencyPlanningContractError> {
        if value == 0 {
            Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_operation.timestamp_ms",
                reason: "operation timestamps must be greater than zero",
            })
        } else {
            Ok(Self(value))
        }
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

impl<'de> Deserialize<'de> for DependencyOperationTimestampMs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

pub(super) fn validate_unique_binding_ids(
    selected_binding_ids: &[DependencyBindingId],
) -> Result<(), DependencyPlanningContractError> {
    for (index, binding_id) in selected_binding_ids.iter().enumerate() {
        if selected_binding_ids[..index].contains(binding_id) {
            return Err(DependencyPlanningContractError::InvalidField {
                field: "dependency_environment_result.selected_binding_ids",
                reason: "selected binding ids must be unique",
            });
        }
    }
    Ok(())
}

pub(super) fn validate_diagnostics(
    diagnostics: &[DependencyPlanningDiagnostic],
) -> Result<(), DependencyPlanningContractError> {
    for diagnostic in diagnostics {
        diagnostic.validate()?;
    }
    Ok(())
}

pub(super) fn validate_dependency_name(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_REQUIREMENT_NAME_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_REQUIREMENT_NAME_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

pub(super) fn validate_dependency_text(
    field: &'static str,
    value: &str,
) -> Result<(), DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_DEPENDENCY_TEXT_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_DEPENDENCY_TEXT_LEN,
        });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DependencyPlanningContractError::InvalidText { field });
    }
    Ok(())
}

pub(super) fn validate_optional_dependency_text(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), DependencyPlanningContractError> {
    if let Some(value) = value {
        validate_dependency_text(field, value)?;
    }
    Ok(())
}

fn validate_profile_id(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_PROFILE_ID_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_PROFILE_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

pub(super) fn validate_validation_field_path(
    field: &'static str,
    value: &str,
) -> Result<String, DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > MAX_FIELD_PATH_LEN {
        return Err(DependencyPlanningContractError::FieldTooLong {
            field,
            max_len: MAX_FIELD_PATH_LEN,
        });
    }
    if trimmed
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    {
        return Err(DependencyPlanningContractError::InvalidField {
            field,
            reason: "validation field paths must be contract fields, not filesystem paths",
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']' | '-' | ':'))
    {
        return Err(DependencyPlanningContractError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}
