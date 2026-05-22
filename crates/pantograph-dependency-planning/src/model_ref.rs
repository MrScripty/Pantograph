use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{DependencyPlanningContractError, PumasArtifactEntryPathError};

const MAX_PUMAS_MODEL_ID_LEN: usize = 512;
const MAX_PUMAS_ARTIFACT_ENTRY_PATH_LEN: usize = 1024;

/// Stable model reference resolved from the model library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PumasModelRef {
    /// Canonical model id assigned by Pumas or an equivalent model library.
    pub model_id: String,
    /// Optional source revision or immutable package revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Optional selected artifact id when a model exposes multiple artifacts.
    #[serde(
        default,
        alias = "artifact_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub selected_artifact_id: Option<String>,
    /// Optional selected artifact path returned during legacy-reference migration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_artifact_path: Option<String>,
    /// Bounded diagnostics emitted while migrating legacy references to Pumas refs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_diagnostics: Vec<ModelRefMigrationDiagnostic>,
}

impl PumasModelRef {
    pub fn validate(&self) -> Result<(), DependencyPlanningContractError> {
        validate_text_field(
            "pumas_model_ref.model_id",
            &self.model_id,
            MAX_PUMAS_MODEL_ID_LEN,
        )?;
        if let Some(revision) = &self.revision {
            validate_text_field("pumas_model_ref.revision", revision, MAX_PUMAS_MODEL_ID_LEN)?;
        }
        if let Some(artifact_id) = &self.selected_artifact_id {
            validate_text_field(
                "pumas_model_ref.selected_artifact_id",
                artifact_id,
                MAX_PUMAS_MODEL_ID_LEN,
            )?;
        }
        if let Some(artifact_path) = &self.selected_artifact_path {
            validate_text_field(
                "pumas_model_ref.selected_artifact_path",
                artifact_path,
                MAX_PUMAS_ARTIFACT_ENTRY_PATH_LEN,
            )?;
        }
        Ok(())
    }
}

/// Diagnostic produced while converting a legacy model reference to a Pumas ref.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ModelRefMigrationDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<String>,
}

/// Root-relative artifact entry path resolved by Pumas before worker dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub struct PumasArtifactEntryPath(String);

impl PumasArtifactEntryPath {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, PumasArtifactEntryPathError> {
        let raw = value.as_ref().trim();
        if raw.is_empty() {
            return Err(PumasArtifactEntryPathError::Missing);
        }
        if raw.len() > MAX_PUMAS_ARTIFACT_ENTRY_PATH_LEN {
            return Err(PumasArtifactEntryPathError::TooLong {
                max_len: MAX_PUMAS_ARTIFACT_ENTRY_PATH_LEN,
            });
        }
        if raw.chars().any(char::is_control) {
            return Err(PumasArtifactEntryPathError::InvalidCharacters);
        }
        if raw.contains("://") {
            return Err(PumasArtifactEntryPathError::UnsupportedUri);
        }
        if raw.starts_with(['/', '\\', '~']) || raw.contains('\\') {
            return Err(PumasArtifactEntryPathError::LocalPath);
        }

        let mut saw_segment = false;
        for segment in raw.split('/') {
            if segment.is_empty() {
                return Err(PumasArtifactEntryPathError::InvalidSegment);
            }
            if matches!(segment, "." | "..") {
                return Err(PumasArtifactEntryPathError::LocalPath);
            }
            saw_segment = true;
        }

        if !saw_segment {
            return Err(PumasArtifactEntryPathError::Missing);
        }

        Ok(Self(raw.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for PumasArtifactEntryPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for PumasArtifactEntryPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PumasArtifactEntryPath {
    type Err = PumasArtifactEntryPathError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for PumasArtifactEntryPath {
    type Error = PumasArtifactEntryPathError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for PumasArtifactEntryPath {
    type Error = PumasArtifactEntryPathError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl Serialize for PumasArtifactEntryPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for PumasArtifactEntryPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

/// Model artifact kind exposed by the model library.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelArtifactKind {
    Gguf,
    HfCompatibleDirectory,
    Safetensors,
    DiffusersBundle,
    Onnx,
    Adapter,
    Shard,
    Unknown,
}

/// Durable storage location class for a resolved model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelStorageKind {
    #[serde(alias = "managed_library")]
    LibraryOwned,
    #[serde(alias = "local_path", alias = "remote_reference")]
    ExternalReference,
    Unknown,
}

/// Validation state for the selected model artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelValidationState {
    Valid,
    #[serde(alias = "warning", alias = "stale")]
    Degraded,
    Invalid,
    Unknown,
}

/// Pumas-approved local load target shape for a selected artifact.
///
/// This mirrors the Pumas resolver response target closely enough for
/// Pantograph dependency and inference planning without making contracts depend
/// on Pumas library internals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct PumasArtifactLoadTarget {
    pub model_ref: PumasModelRef,
    pub artifact_kind: ModelArtifactKind,
    pub local_load_path: String,
    pub load_path_kind: PumasArtifactLoadPathKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_root_id: Option<String>,
    pub storage_kind: ModelStorageKind,
    pub validation_state: ModelValidationState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_facts_contract_version: Option<u32>,
}

impl PumasArtifactLoadTarget {
    pub fn validate_for_handoff(&self) -> Result<(), DependencyPlanningContractError> {
        self.model_ref.validate()?;
        validate_text_field(
            "artifact_load_target.local_load_path",
            &self.local_load_path,
            MAX_PUMAS_ARTIFACT_ENTRY_PATH_LEN * 4,
        )?;
        Ok(())
    }
}

/// Filesystem shape Pumas approved for runtime loading.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PumasArtifactLoadPathKind {
    Directory,
    File,
}

fn validate_text_field(
    field: &'static str,
    value: &str,
    max_len: usize,
) -> Result<(), DependencyPlanningContractError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DependencyPlanningContractError::MissingField { field });
    }
    if trimmed.len() > max_len {
        return Err(DependencyPlanningContractError::FieldTooLong { field, max_len });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(DependencyPlanningContractError::InvalidText { field });
    }
    Ok(())
}
