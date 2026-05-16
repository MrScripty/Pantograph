//! Typed availability facts for runtime, package, and inference-trait support.
//!
//! This module is a pure contract boundary. Producers can report whether a
//! runtime, package, or trait is selectable, but scheduler policy and UI
//! presentation stay in their owning crates.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

const CAPABILITY_AVAILABILITY_ID_MAX_LEN: usize = 96;
const CAPABILITY_AVAILABILITY_REASON_MAX_LEN: usize = 240;

/// Stable subject family for one availability fact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityAvailabilitySubjectKind {
    /// An executable backend/runtime such as `pytorch` or `llama_cpp`.
    Runtime,
    /// A concrete runtime variant such as `pytorch.cuda`.
    RuntimeVariant,
    /// An optional or required runtime trait such as a denoising scheduler.
    RuntimeTrait,
    /// A package or library dependency such as `diffusers` or `torch`.
    Package,
    /// A managed binary or package dependency entry.
    Dependency,
    /// A model artifact capability or package-fact-derived capability.
    ModelCapability,
}

/// Scheduler-facing availability state for a runtime/package/trait fact.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityAvailabilityState {
    /// The subject can be selected by policy when the rest of the request fits.
    Available,
    /// The subject is supported by Pantograph but not installed locally.
    NotInstalled,
    /// The contract is reserved or planned, but execution is not implemented.
    NotImplemented,
    /// The subject cannot run on the current platform.
    UnsupportedPlatform,
    /// A required runtime/package dependency is absent or not ready.
    MissingDependency,
    /// A host or product policy disables this subject.
    DisabledByPolicy,
    /// Required model/package facts were not available.
    MissingModelFacts,
    /// The subject requires a runtime capability that is not present.
    RequiresRuntimeCapability,
    /// The subject requires a model capability that is not present.
    RequiresModelCapability,
}

impl CapabilityAvailabilityState {
    /// Return true when this state can participate in runtime selection.
    #[must_use]
    pub fn is_selectable(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// A validated primitive id used by capability availability facts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct CapabilityAvailabilityId(String);

impl CapabilityAvailabilityId {
    /// Parse and validate a stable lowercase primitive id.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, CapabilityAvailabilityError> {
        validate_identifier(
            "capability_availability_id",
            value.as_ref(),
            CAPABILITY_AVAILABILITY_ID_MAX_LEN,
        )
        .map(Self)
    }

    /// Borrow the validated id.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for CapabilityAvailabilityId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CapabilityAvailabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CapabilityAvailabilityId {
    type Err = CapabilityAvailabilityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CapabilityAvailabilityId {
    type Error = CapabilityAvailabilityError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for CapabilityAvailabilityId {
    type Error = CapabilityAvailabilityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for CapabilityAvailabilityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CapabilityAvailabilityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Bounded user-facing reason text attached to one availability fact.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[must_use]
pub struct CapabilityAvailabilityReason(String);

impl CapabilityAvailabilityReason {
    /// Parse and validate bounded single-line reason text.
    pub fn parse(value: impl AsRef<str>) -> Result<Self, CapabilityAvailabilityError> {
        let trimmed = value.as_ref().trim();
        if trimmed.is_empty() {
            return Err(CapabilityAvailabilityError::EmptyReason);
        }
        if trimmed.len() > CAPABILITY_AVAILABILITY_REASON_MAX_LEN {
            return Err(CapabilityAvailabilityError::ReasonTooLong {
                max_len: CAPABILITY_AVAILABILITY_REASON_MAX_LEN,
                actual_len: trimmed.len(),
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(CapabilityAvailabilityError::InvalidReason {
                value: trimmed.to_string(),
            });
        }

        Ok(Self(trimmed.to_string()))
    }

    /// Borrow the validated reason.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl AsRef<str> for CapabilityAvailabilityReason {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CapabilityAvailabilityReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CapabilityAvailabilityReason {
    type Err = CapabilityAvailabilityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for CapabilityAvailabilityReason {
    type Error = CapabilityAvailabilityError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for CapabilityAvailabilityReason {
    type Error = CapabilityAvailabilityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for CapabilityAvailabilityReason {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for CapabilityAvailabilityReason {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// One typed availability fact emitted by an owner before scheduler selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct CapabilityAvailabilityFact {
    /// Subject family for this fact.
    pub subject_kind: CapabilityAvailabilitySubjectKind,
    /// Validated primitive id for the subject.
    pub subject_id: CapabilityAvailabilityId,
    /// Availability state for scheduler and provider projections.
    pub state: CapabilityAvailabilityState,
    /// Runtime this fact belongs to, when runtime-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<CapabilityAvailabilityId>,
    /// Stable reason code for diagnostics and UI rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<CapabilityAvailabilityId>,
    /// Bounded single-line reason text for diagnostics and UI rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<CapabilityAvailabilityReason>,
}

impl CapabilityAvailabilityFact {
    /// Build an availability fact from validated parts.
    #[must_use]
    pub fn new(
        subject_kind: CapabilityAvailabilitySubjectKind,
        subject_id: CapabilityAvailabilityId,
        state: CapabilityAvailabilityState,
    ) -> Self {
        Self {
            subject_kind,
            subject_id,
            state,
            runtime_id: None,
            reason_code: None,
            reason: None,
        }
    }

    /// Attach the runtime that scoped this fact.
    #[must_use]
    pub fn with_runtime_id(mut self, runtime_id: CapabilityAvailabilityId) -> Self {
        self.runtime_id = Some(runtime_id);
        self
    }

    /// Attach a stable reason code.
    #[must_use]
    pub fn with_reason_code(mut self, reason_code: CapabilityAvailabilityId) -> Self {
        self.reason_code = Some(reason_code);
        self
    }

    /// Attach bounded reason text.
    #[must_use]
    pub fn with_reason(mut self, reason: CapabilityAvailabilityReason) -> Self {
        self.reason = Some(reason);
        self
    }

    /// Return true when this fact is selectable.
    #[must_use]
    pub fn is_selectable(&self) -> bool {
        self.state.is_selectable()
    }
}

/// Validation failure for capability availability contract values.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapabilityAvailabilityError {
    /// A required identifier was empty after trimming.
    #[error("{field} must not be empty")]
    EmptyIdentifier {
        /// Contract field that failed validation.
        field: &'static str,
    },
    /// An identifier exceeded its bounded wire-contract length.
    #[error("{field} must be at most {max_len} bytes, got {actual_len}")]
    IdentifierTooLong {
        /// Contract field that failed validation.
        field: &'static str,
        /// Maximum accepted byte length.
        max_len: usize,
        /// Actual byte length.
        actual_len: usize,
    },
    /// An identifier did not match the canonical lowercase identifier shape.
    #[error("{field} has invalid identifier shape: {value}")]
    InvalidIdentifier {
        /// Contract field that failed validation.
        field: &'static str,
        /// Invalid value.
        value: String,
    },
    /// Reason text was empty after trimming.
    #[error("capability availability reason must not be empty")]
    EmptyReason,
    /// Reason text exceeded the bounded wire-contract length.
    #[error("capability availability reason must be at most {max_len} bytes, got {actual_len}")]
    ReasonTooLong {
        /// Maximum accepted byte length.
        max_len: usize,
        /// Actual byte length.
        actual_len: usize,
    },
    /// Reason text contained control characters.
    #[error("capability availability reason contains control characters: {value}")]
    InvalidReason {
        /// Invalid value.
        value: String,
    },
}

fn validate_identifier(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<String, CapabilityAvailabilityError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CapabilityAvailabilityError::EmptyIdentifier { field });
    }
    if trimmed.len() > max_len {
        return Err(CapabilityAvailabilityError::IdentifierTooLong {
            field,
            max_len,
            actual_len: trimmed.len(),
        });
    }

    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return Err(CapabilityAvailabilityError::EmptyIdentifier { field });
    };
    if !first.is_ascii_lowercase() {
        return Err(CapabilityAvailabilityError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    let mut previous_was_separator = false;
    for ch in chars {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            previous_was_separator = false;
            continue;
        }
        if matches!(ch, '_' | '-' | '.' | ':') && !previous_was_separator {
            previous_was_separator = true;
            continue;
        }
        return Err(CapabilityAvailabilityError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    if previous_was_separator {
        return Err(CapabilityAvailabilityError::InvalidIdentifier {
            field,
            value: trimmed.to_string(),
        });
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn availability_id(value: &str) -> CapabilityAvailabilityId {
        CapabilityAvailabilityId::parse(value).expect("valid availability id")
    }

    fn reason(value: &str) -> CapabilityAvailabilityReason {
        CapabilityAvailabilityReason::parse(value).expect("valid availability reason")
    }

    #[test]
    fn availability_state_selectable_only_for_available() {
        assert!(CapabilityAvailabilityState::Available.is_selectable());

        for state in [
            CapabilityAvailabilityState::NotInstalled,
            CapabilityAvailabilityState::NotImplemented,
            CapabilityAvailabilityState::UnsupportedPlatform,
            CapabilityAvailabilityState::MissingDependency,
            CapabilityAvailabilityState::DisabledByPolicy,
            CapabilityAvailabilityState::MissingModelFacts,
            CapabilityAvailabilityState::RequiresRuntimeCapability,
            CapabilityAvailabilityState::RequiresModelCapability,
        ] {
            assert!(!state.is_selectable(), "{state:?} must not be selectable");
        }
    }

    #[test]
    fn availability_id_rejects_ambiguous_or_display_values() {
        assert_eq!(availability_id("pytorch.cuda").as_str(), "pytorch.cuda");
        assert_eq!(availability_id("torch:2").as_str(), "torch:2");

        for value in ["", "  ", "PyTorch", "pytorch cuda", "pytorch/", "pytorch."] {
            assert!(
                CapabilityAvailabilityId::parse(value).is_err(),
                "{value:?} should fail validation"
            );
        }
    }

    #[test]
    fn availability_reason_rejects_empty_long_or_multiline_text() {
        assert_eq!(
            reason("Runtime package is not installed").as_str(),
            "Runtime package is not installed"
        );

        assert_eq!(
            CapabilityAvailabilityReason::parse(" ").expect_err("blank reason"),
            CapabilityAvailabilityError::EmptyReason
        );
        assert!(matches!(
            CapabilityAvailabilityReason::parse(
                "x".repeat(CAPABILITY_AVAILABILITY_REASON_MAX_LEN + 1)
            )
            .expect_err("long reason"),
            CapabilityAvailabilityError::ReasonTooLong { .. }
        ));
        assert!(matches!(
            CapabilityAvailabilityReason::parse("first\nsecond").expect_err("multiline reason"),
            CapabilityAvailabilityError::InvalidReason { .. }
        ));
    }

    #[test]
    fn capability_availability_fact_serde_uses_typed_primitive_fields() {
        let fact = CapabilityAvailabilityFact::new(
            CapabilityAvailabilitySubjectKind::RuntimeTrait,
            availability_id("denoising_scheduler"),
            CapabilityAvailabilityState::RequiresRuntimeCapability,
        )
        .with_runtime_id(availability_id("pytorch"))
        .with_reason_code(availability_id("scheduler_not_supported"))
        .with_reason(reason(
            "The selected runtime does not expose this scheduler.",
        ));

        let encoded = serde_json::to_value(&fact).expect("encode fact");
        assert_eq!(
            encoded,
            json!({
                "subject_kind": "runtime_trait",
                "subject_id": "denoising_scheduler",
                "state": "requires_runtime_capability",
                "runtime_id": "pytorch",
                "reason_code": "scheduler_not_supported",
                "reason": "The selected runtime does not expose this scheduler."
            })
        );

        let decoded: CapabilityAvailabilityFact =
            serde_json::from_value(encoded).expect("decode fact");
        assert_eq!(decoded, fact);
        assert!(!decoded.is_selectable());
    }

    #[test]
    fn capability_availability_fact_deserialization_validates_ids() {
        let error = serde_json::from_value::<CapabilityAvailabilityFact>(json!({
            "subject_kind": "package",
            "subject_id": "Diffusers",
            "state": "not_installed"
        }))
        .expect_err("invalid subject id must not deserialize");

        assert!(error.to_string().contains("invalid identifier shape"));
    }
}
