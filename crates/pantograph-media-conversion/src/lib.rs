//! Host-owned managed media conversion contracts.
//!
//! This crate defines the neutral boundary for real media conversion without
//! depending on workflow-service, Tauri, or inference implementation modules.

use std::fmt;
use std::str::FromStr;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const MAX_ID_LEN: usize = 128;
const MAX_MEDIA_TYPE_LEN: usize = 128;
const MAX_FORMAT_FIELD_LEN: usize = 128;
const MAX_ERROR_SUMMARY_LEN: usize = 4096;
const MAX_TIMEOUT_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MediaConversionError {
    #[error("{field} is required")]
    MissingField { field: &'static str },
    #[error("{field} is longer than {max_len} characters")]
    FieldTooLong { field: &'static str, max_len: usize },
    #[error("{field} contains unsupported characters")]
    InvalidIdentifier { field: &'static str },
    #[error("{field} contains control characters")]
    InvalidText { field: &'static str },
    #[error("{field} value {value} is outside allowed range")]
    InvalidRange { field: &'static str, value: u64 },
    #[error("conversion from {source_media_type} to {target_media_type} is not supported")]
    UnsupportedConversion {
        source_media_type: String,
        target_media_type: String,
    },
    #[error("{dependency_id} dependency is unavailable: {reason}")]
    DependencyUnavailable {
        dependency_id: ManagedMediaDependencyId,
        reason: String,
    },
    #[error("converter process failed with status {status_code:?}: {stderr_summary}")]
    ProcessFailed {
        status_code: Option<i32>,
        stderr_summary: String,
    },
    #[error("converter process exceeded timeout of {timeout_ms}ms")]
    TimedOut { timeout_ms: u64 },
    #[error("conversion was cancelled")]
    Cancelled,
    #[error("conversion I/O failed: {message}")]
    Io { message: String },
}

macro_rules! conversion_id {
    ($name:ident, $field:literal, $prefix:literal) => {
        #[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn generate() -> Self {
                Self(format!("{}{}", $prefix, Uuid::new_v4()))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = MediaConversionError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                validate_identifier($field, value).map(Self)
            }
        }

        impl FromStr for $name {
            type Err = MediaConversionError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(value.to_string())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0).finish()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

conversion_id!(MediaConversionId, "conversion_id", "conversion_");
conversion_id!(ArtifactId, "artifact_id", "artifact_");
conversion_id!(WorkflowRunId, "workflow_run_id", "run_");
conversion_id!(GraphNodeId, "node_id", "node_");
conversion_id!(PortId, "port_id", "port_");
conversion_id!(
    ManagedMediaDependencyVersion,
    "dependency_version",
    "version_"
);
conversion_id!(
    ManagedMediaDependencyLeaseId,
    "dependency_lease_id",
    "lease_"
);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConversionMediaKind {
    Image,
    Audio,
    Video,
    #[serde(rename = "3d")]
    ThreeD,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MediaConversionStatus {
    Converted,
    PassedThrough,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ManagedMediaDependencyId {
    Ffmpeg,
    Ocioconvert,
    Oiiotool,
    OpenColorIo,
}

impl fmt::Display for ManagedMediaDependencyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ocioconvert => "ocioconvert",
            Self::Oiiotool => "oiiotool",
            Self::OpenColorIo => "opencolorio",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct MediaType(String);

impl MediaType {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MediaType {
    type Error = MediaConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_text_field("media_type", value, MAX_MEDIA_TYPE_LEN).map(Self)
    }
}

impl FromStr for MediaType {
    type Err = MediaConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FormatField(String);

impl FormatField {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for FormatField {
    type Error = MediaConversionError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        validate_text_field("format_field", value, MAX_FORMAT_FIELD_LEN).map(Self)
    }
}

impl FromStr for FormatField {
    type Err = MediaConversionError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionTarget {
    pub format_id: FormatField,
    pub media_type: MediaType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec_id: Option<FormatField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<FormatField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_profile_id: Option<FormatField>,
    pub color_managed: bool,
}

impl MediaConversionTarget {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        format_id: FormatField,
        media_type: MediaType,
        codec_id: Option<FormatField>,
        quality_percent: Option<u8>,
        bitrate_kbps: Option<u32>,
        crf: Option<u8>,
        bit_depth: Option<FormatField>,
        color_profile_id: Option<FormatField>,
        color_managed: bool,
    ) -> Result<Self, MediaConversionError> {
        if let Some(quality_percent) = quality_percent {
            validate_max("quality_percent", quality_percent as u64, 100)?;
        }
        if let Some(crf) = crf {
            validate_max("crf", crf as u64, 63)?;
        }
        if let Some(bitrate_kbps) = bitrate_kbps {
            validate_non_zero("bitrate_kbps", bitrate_kbps as u64)?;
        }
        Ok(Self {
            format_id,
            media_type,
            codec_id,
            quality_percent,
            bitrate_kbps,
            crf,
            bit_depth,
            color_profile_id,
            color_managed,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConversionSource {
    pub artifact_id: ArtifactId,
    pub media_type: MediaType,
    pub body: Vec<u8>,
}

impl MediaConversionSource {
    pub fn try_new(
        artifact_id: ArtifactId,
        media_type: MediaType,
        body: Vec<u8>,
    ) -> Result<Self, MediaConversionError> {
        if body.is_empty() {
            return Err(MediaConversionError::MissingField { field: "body" });
        }
        Ok(Self {
            artifact_id,
            media_type,
            body,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionAttribution {
    pub workflow_run_id: WorkflowRunId,
    pub source_artifact_id: ArtifactId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<GraphNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<PortId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConversionRequest {
    pub conversion_id: MediaConversionId,
    pub kind: ConversionMediaKind,
    pub attribution: MediaConversionAttribution,
    pub source: MediaConversionSource,
    pub target: MediaConversionTarget,
    pub timeout_ms: Option<u64>,
}

impl MediaConversionRequest {
    pub fn try_new(
        conversion_id: MediaConversionId,
        kind: ConversionMediaKind,
        attribution: MediaConversionAttribution,
        source: MediaConversionSource,
        target: MediaConversionTarget,
        timeout_ms: Option<u64>,
    ) -> Result<Self, MediaConversionError> {
        if let Some(timeout_ms) = timeout_ms {
            validate_non_zero("timeout_ms", timeout_ms)?;
            validate_max("timeout_ms", timeout_ms, MAX_TIMEOUT_MS)?;
        }
        Ok(Self {
            conversion_id,
            kind,
            attribution,
            source,
            target,
            timeout_ms,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaConversionDependencyAttribution {
    pub dependency_id: ManagedMediaDependencyId,
    pub version: ManagedMediaDependencyVersion,
    pub lease_id: ManagedMediaDependencyLeaseId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConversionResult {
    pub conversion_id: MediaConversionId,
    pub status: MediaConversionStatus,
    pub media_type: MediaType,
    pub target: MediaConversionTarget,
    pub body: Vec<u8>,
    pub dependencies: Vec<MediaConversionDependencyAttribution>,
    pub stderr_summary: Option<String>,
}

impl MediaConversionResult {
    pub fn try_new(
        conversion_id: MediaConversionId,
        status: MediaConversionStatus,
        media_type: MediaType,
        target: MediaConversionTarget,
        body: Vec<u8>,
        dependencies: Vec<MediaConversionDependencyAttribution>,
        stderr_summary: Option<String>,
    ) -> Result<Self, MediaConversionError> {
        if body.is_empty() {
            return Err(MediaConversionError::MissingField { field: "body" });
        }
        if let Some(summary) = stderr_summary.as_deref() {
            validate_text_field("stderr_summary", summary.to_string(), MAX_ERROR_SUMMARY_LEN)?;
        }
        Ok(Self {
            conversion_id,
            status,
            media_type,
            target,
            body,
            dependencies,
            stderr_summary,
        })
    }
}

#[async_trait]
pub trait MediaConversionExecutor: Send + Sync {
    async fn convert(
        &self,
        request: MediaConversionRequest,
    ) -> Result<MediaConversionResult, MediaConversionError>;
}

fn validate_identifier(field: &'static str, value: String) -> Result<String, MediaConversionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MediaConversionError::MissingField { field });
    }
    if trimmed.len() > MAX_ID_LEN {
        return Err(MediaConversionError::FieldTooLong {
            field,
            max_len: MAX_ID_LEN,
        });
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':'))
    {
        return Err(MediaConversionError::InvalidIdentifier { field });
    }
    Ok(trimmed.to_string())
}

fn validate_text_field(
    field: &'static str,
    value: String,
    max_len: usize,
) -> Result<String, MediaConversionError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MediaConversionError::MissingField { field });
    }
    if trimmed.len() > max_len {
        return Err(MediaConversionError::FieldTooLong { field, max_len });
    }
    if trimmed.chars().any(char::is_control) {
        return Err(MediaConversionError::InvalidText { field });
    }
    Ok(trimmed.to_string())
}

fn validate_non_zero(field: &'static str, value: u64) -> Result<(), MediaConversionError> {
    if value == 0 {
        Err(MediaConversionError::InvalidRange { field, value })
    } else {
        Ok(())
    }
}

fn validate_max(field: &'static str, value: u64, max: u64) -> Result<(), MediaConversionError> {
    if value > max {
        Err(MediaConversionError::InvalidRange { field, value })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id<T: FromStr<Err = MediaConversionError>>(value: &str) -> T {
        value.parse().expect("valid id")
    }

    fn target() -> MediaConversionTarget {
        MediaConversionTarget::try_new(
            "jpg".parse().expect("format id"),
            "image/jpeg".parse().expect("media type"),
            Some("jpeg".parse().expect("codec")),
            Some(75),
            None,
            None,
            Some("8bit".parse().expect("bit depth")),
            Some("srgb".parse().expect("color profile")),
            true,
        )
        .expect("target")
    }

    #[test]
    fn request_rejects_invalid_ids_and_bounds() {
        let invalid_id = "bad/id"
            .parse::<MediaConversionId>()
            .expect_err("invalid id");
        assert!(matches!(
            invalid_id,
            MediaConversionError::InvalidIdentifier {
                field: "conversion_id"
            }
        ));

        let invalid_quality = MediaConversionTarget::try_new(
            "jpg".parse().expect("format id"),
            "image/jpeg".parse().expect("media type"),
            None,
            Some(101),
            None,
            None,
            None,
            None,
            false,
        )
        .expect_err("invalid quality");
        assert!(matches!(
            invalid_quality,
            MediaConversionError::InvalidRange {
                field: "quality_percent",
                value: 101
            }
        ));
    }

    #[test]
    fn request_keeps_media_body_out_of_serialized_attribution() {
        let attribution = MediaConversionAttribution {
            workflow_run_id: id("run-a"),
            source_artifact_id: id("artifact-a"),
            node_id: Some(id("node-a")),
            port_id: Some(id("port-image")),
        };

        let serialized = serde_json::to_value(&attribution).expect("serialize");

        assert_eq!(
            serialized,
            serde_json::json!({
                "workflow_run_id": "run-a",
                "source_artifact_id": "artifact-a",
                "node_id": "node-a",
                "port_id": "port-image"
            })
        );
    }

    #[test]
    fn result_records_per_conversion_dependency_attribution() {
        let dependency = MediaConversionDependencyAttribution {
            dependency_id: ManagedMediaDependencyId::Oiiotool,
            version: id("2.5.18"),
            lease_id: id("lease-1"),
        };

        let result = MediaConversionResult::try_new(
            id("conversion-a"),
            MediaConversionStatus::Converted,
            "image/jpeg".parse().expect("media type"),
            target(),
            vec![1, 2, 3],
            vec![dependency.clone()],
            Some("bounded stderr".to_string()),
        )
        .expect("result");

        assert_eq!(result.dependencies, vec![dependency]);
        assert_eq!(result.status, MediaConversionStatus::Converted);
    }

    #[test]
    fn source_and_result_reject_empty_bodies() {
        let source_error = MediaConversionSource::try_new(
            id("artifact-a"),
            "image/png".parse().expect("media type"),
            Vec::new(),
        )
        .expect_err("empty source body");
        assert!(matches!(
            source_error,
            MediaConversionError::MissingField { field: "body" }
        ));

        let result_error = MediaConversionResult::try_new(
            id("conversion-a"),
            MediaConversionStatus::Converted,
            "image/jpeg".parse().expect("media type"),
            target(),
            Vec::new(),
            Vec::new(),
            None,
        )
        .expect_err("empty result body");
        assert!(matches!(
            result_error,
            MediaConversionError::MissingField { field: "body" }
        ));
    }
}
