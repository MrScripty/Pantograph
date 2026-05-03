//! Host-owned managed media conversion contracts.
//!
//! This crate defines the neutral boundary for real media conversion without
//! depending on workflow-service, Tauri, or inference implementation modules.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use pantograph_managed_dependencies::{
    ManagedDependencyKey, MediaToolDependencyId, NativeArtifactDependencyId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

const MAX_ID_LEN: usize = 128;
const MAX_MEDIA_TYPE_LEN: usize = 128;
const MAX_FORMAT_FIELD_LEN: usize = 128;
const MAX_ERROR_SUMMARY_LEN: usize = 4096;
const MAX_TIMEOUT_MS: u64 = 24 * 60 * 60 * 1000;
const MAX_LEASE_HOLDER_LEN: usize = 640;

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
    #[error("{field} must be an absolute executable path supplied by host boundary: {reason}")]
    InvalidExecutablePath { field: &'static str, reason: String },
    #[error("conversion from {source_media_type} to {target_media_type} is not supported")]
    UnsupportedConversion {
        source_media_type: String,
        target_media_type: String,
    },
    #[error("command planning for {kind:?} target {target_media_type} is not supported: {reason}")]
    UnsupportedCommandPlan {
        kind: ConversionMediaKind,
        target_media_type: String,
        reason: String,
    },
    #[error("{dependency_id} dependency is unavailable: {reason}")]
    DependencyUnavailable {
        dependency_id: ManagedMediaDependencyId,
        reason: String,
    },
    #[error("managed dependency key {key} is not a media conversion dependency")]
    UnsupportedManagedDependencyKey { key: String },
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

impl From<ManagedMediaDependencyId> for ManagedDependencyKey {
    fn from(value: ManagedMediaDependencyId) -> Self {
        match value {
            ManagedMediaDependencyId::Ffmpeg => Self::MediaTool(MediaToolDependencyId::Ffmpeg),
            ManagedMediaDependencyId::Ocioconvert => {
                Self::MediaTool(MediaToolDependencyId::Ocioconvert)
            }
            ManagedMediaDependencyId::Oiiotool => Self::MediaTool(MediaToolDependencyId::Oiiotool),
            ManagedMediaDependencyId::OpenColorIo => {
                Self::NativeArtifact(NativeArtifactDependencyId::OpenColorIo)
            }
        }
    }
}

impl TryFrom<ManagedDependencyKey> for ManagedMediaDependencyId {
    type Error = MediaConversionError;

    fn try_from(value: ManagedDependencyKey) -> Result<Self, Self::Error> {
        match value {
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg) => Ok(Self::Ffmpeg),
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ocioconvert) => {
                Ok(Self::Ocioconvert)
            }
            ManagedDependencyKey::MediaTool(MediaToolDependencyId::Oiiotool) => Ok(Self::Oiiotool),
            ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo) => {
                Ok(Self::OpenColorIo)
            }
            ManagedDependencyKey::RuntimeSidecar(_) => {
                Err(MediaConversionError::UnsupportedManagedDependencyKey {
                    key: value.stable_key().to_string(),
                })
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum MediaCommandPlanStream {
    Stdin,
    Stdout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaCommandPlanStep {
    pub dependency_id: ManagedMediaDependencyId,
    pub argv: Vec<String>,
    pub input: MediaCommandPlanStream,
    pub output: MediaCommandPlanStream,
}

impl MediaCommandPlanStep {
    pub fn try_new(
        dependency_id: ManagedMediaDependencyId,
        argv: Vec<String>,
        input: MediaCommandPlanStream,
        output: MediaCommandPlanStream,
    ) -> Result<Self, MediaConversionError> {
        if argv.is_empty() {
            return Err(MediaConversionError::MissingField { field: "argv" });
        }
        for arg in &argv {
            validate_process_arg(arg)?;
        }
        Ok(Self {
            dependency_id,
            argv,
            input,
            output,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaCommandPlan {
    pub kind: ConversionMediaKind,
    pub target: MediaConversionTarget,
    pub required_dependency_ids: Vec<ManagedMediaDependencyId>,
    pub steps: Vec<MediaCommandPlanStep>,
}

impl MediaCommandPlan {
    pub fn try_for_target(
        kind: ConversionMediaKind,
        target: MediaConversionTarget,
    ) -> Result<Self, MediaConversionError> {
        match kind {
            ConversionMediaKind::Image => plan_image_command(target),
            ConversionMediaKind::Audio => plan_audio_command(target),
            ConversionMediaKind::Video => plan_video_command(target),
            ConversionMediaKind::ThreeD => Err(unsupported_command_plan(
                kind,
                &target,
                "managed 3D conversion has no concrete converter dependency",
            )),
        }
    }

    fn try_new(
        kind: ConversionMediaKind,
        target: MediaConversionTarget,
        steps: Vec<MediaCommandPlanStep>,
        extra_dependency_ids: Vec<ManagedMediaDependencyId>,
    ) -> Result<Self, MediaConversionError> {
        if steps.is_empty() {
            return Err(MediaConversionError::MissingField { field: "steps" });
        }

        let mut required_dependency_ids = Vec::new();
        for dependency_id in extra_dependency_ids {
            push_unique_dependency(&mut required_dependency_ids, dependency_id);
        }
        for step in &steps {
            push_unique_dependency(&mut required_dependency_ids, step.dependency_id);
        }

        Ok(Self {
            kind,
            target,
            required_dependency_ids,
            steps,
        })
    }
}

pub fn plan_image_command(
    target: MediaConversionTarget,
) -> Result<MediaCommandPlan, MediaConversionError> {
    ensure_target_media_type(ConversionMediaKind::Image, &target, "image/")?;

    let mut steps = Vec::new();
    let mut extra_dependency_ids = Vec::new();
    if target.color_managed {
        let mut argv = vec![
            "--input".to_string(),
            "-".to_string(),
            "--output".to_string(),
            "-".to_string(),
        ];
        if let Some(color_profile_id) = target.color_profile_id.as_ref() {
            argv.push("--output-color-space".to_string());
            argv.push(color_profile_id.as_str().to_string());
        }
        steps.push(MediaCommandPlanStep::try_new(
            ManagedMediaDependencyId::Ocioconvert,
            argv,
            MediaCommandPlanStream::Stdin,
            MediaCommandPlanStream::Stdout,
        )?);
        extra_dependency_ids.push(ManagedMediaDependencyId::OpenColorIo);
    }

    let mut argv = vec![
        "-".to_string(),
        "--format".to_string(),
        target.format_id.as_str().to_string(),
    ];
    if let Some(codec_id) = target.codec_id.as_ref() {
        argv.push("--compression".to_string());
        argv.push(codec_id.as_str().to_string());
    }
    if let Some(quality_percent) = target.quality_percent {
        argv.push("--quality".to_string());
        argv.push(quality_percent.to_string());
    }
    if let Some(bit_depth) = target.bit_depth.as_ref() {
        argv.push("--bitdepth".to_string());
        argv.push(bit_depth.as_str().to_string());
    }
    if !target.color_managed {
        if let Some(color_profile_id) = target.color_profile_id.as_ref() {
            argv.push("--color-profile".to_string());
            argv.push(color_profile_id.as_str().to_string());
        }
    }
    argv.push("-o".to_string());
    argv.push("-".to_string());
    steps.push(MediaCommandPlanStep::try_new(
        ManagedMediaDependencyId::Oiiotool,
        argv,
        MediaCommandPlanStream::Stdin,
        MediaCommandPlanStream::Stdout,
    )?);

    MediaCommandPlan::try_new(
        ConversionMediaKind::Image,
        target,
        steps,
        extra_dependency_ids,
    )
}

pub fn plan_audio_command(
    target: MediaConversionTarget,
) -> Result<MediaCommandPlan, MediaConversionError> {
    ensure_target_media_type(ConversionMediaKind::Audio, &target, "audio/")?;

    let mut argv = ffmpeg_common_argv(&target);
    if let Some(codec_id) = target.codec_id.as_ref() {
        argv.push("-codec:a".to_string());
        argv.push(codec_id.as_str().to_string());
    }
    if let Some(bitrate_kbps) = target.bitrate_kbps {
        argv.push("-b:a".to_string());
        argv.push(format!("{bitrate_kbps}k"));
    }
    if let Some(quality_percent) = target.quality_percent {
        argv.push("-q:a".to_string());
        argv.push(quality_percent.to_string());
    }
    argv.push("pipe:1".to_string());

    MediaCommandPlan::try_new(
        ConversionMediaKind::Audio,
        target,
        vec![MediaCommandPlanStep::try_new(
            ManagedMediaDependencyId::Ffmpeg,
            argv,
            MediaCommandPlanStream::Stdin,
            MediaCommandPlanStream::Stdout,
        )?],
        Vec::new(),
    )
}

pub fn plan_video_command(
    target: MediaConversionTarget,
) -> Result<MediaCommandPlan, MediaConversionError> {
    ensure_target_media_type(ConversionMediaKind::Video, &target, "video/")?;

    let mut argv = ffmpeg_common_argv(&target);
    if let Some(codec_id) = target.codec_id.as_ref() {
        argv.push("-codec:v".to_string());
        argv.push(codec_id.as_str().to_string());
    }
    if let Some(bitrate_kbps) = target.bitrate_kbps {
        argv.push("-b:v".to_string());
        argv.push(format!("{bitrate_kbps}k"));
    }
    if let Some(crf) = target.crf {
        argv.push("-crf".to_string());
        argv.push(crf.to_string());
    }
    argv.push("pipe:1".to_string());

    MediaCommandPlan::try_new(
        ConversionMediaKind::Video,
        target,
        vec![MediaCommandPlanStep::try_new(
            ManagedMediaDependencyId::Ffmpeg,
            argv,
            MediaCommandPlanStream::Stdin,
            MediaCommandPlanStream::Stdout,
        )?],
        Vec::new(),
    )
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
    pub lease_holder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManagedExecutablePath(PathBuf);

impl ManagedExecutablePath {
    pub fn try_new(path: PathBuf) -> Result<Self, MediaConversionError> {
        validate_executable_path("executable_path", &path)?;
        Ok(Self(path))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

impl TryFrom<PathBuf> for ManagedExecutablePath {
    type Error = MediaConversionError;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMediaConverter {
    pub dependency: MediaConversionDependencyAttribution,
    pub executable_path: ManagedExecutablePath,
    pub source_media_type: MediaType,
    pub target_media_type: MediaType,
    pub args: Vec<String>,
}

impl ManagedMediaConverter {
    pub fn try_new(
        dependency: MediaConversionDependencyAttribution,
        executable_path: ManagedExecutablePath,
        source_media_type: MediaType,
        target_media_type: MediaType,
        args: Vec<String>,
    ) -> Result<Self, MediaConversionError> {
        for arg in &args {
            validate_process_arg(arg)?;
        }
        Ok(Self {
            dependency,
            executable_path,
            source_media_type,
            target_media_type,
            args,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRunRequest {
    pub executable_path: ManagedExecutablePath,
    pub args: Vec<String>,
    pub stdin: Vec<u8>,
    pub timeout_ms: Option<u64>,
}

impl ProcessRunRequest {
    pub fn try_new(
        executable_path: ManagedExecutablePath,
        args: Vec<String>,
        stdin: Vec<u8>,
        timeout_ms: Option<u64>,
    ) -> Result<Self, MediaConversionError> {
        if stdin.is_empty() {
            return Err(MediaConversionError::MissingField { field: "stdin" });
        }
        if let Some(timeout_ms) = timeout_ms {
            validate_non_zero("timeout_ms", timeout_ms)?;
            validate_max("timeout_ms", timeout_ms, MAX_TIMEOUT_MS)?;
        }
        for arg in &args {
            validate_process_arg(arg)?;
        }
        Ok(Self {
            executable_path,
            args,
            stdin,
            timeout_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessRunOutput {
    pub status_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr_summary: Option<String>,
}

impl ProcessRunOutput {
    pub fn new(status_code: Option<i32>, stdout: Vec<u8>, stderr: &[u8]) -> Self {
        Self {
            status_code,
            stdout,
            stderr_summary: bounded_stderr_summary(stderr),
        }
    }

    pub fn successful(&self) -> bool {
        self.status_code == Some(0)
    }
}

#[async_trait]
pub trait ProcessRunner: Send + Sync {
    async fn run(
        &self,
        request: ProcessRunRequest,
    ) -> Result<ProcessRunOutput, MediaConversionError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StdProcessRunner;

#[async_trait]
impl ProcessRunner for StdProcessRunner {
    async fn run(
        &self,
        request: ProcessRunRequest,
    ) -> Result<ProcessRunOutput, MediaConversionError> {
        let mut command = Command::new(request.executable_path.as_path());
        command
            .args(&request.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let mut child = command.spawn().map_err(|error| MediaConversionError::Io {
            message: error.to_string(),
        })?;

        let mut stdin = child.stdin.take().ok_or_else(|| MediaConversionError::Io {
            message: "converter stdin was unavailable".to_string(),
        })?;
        stdin
            .write_all(&request.stdin)
            .await
            .map_err(|error| MediaConversionError::Io {
                message: error.to_string(),
            })?;
        drop(stdin);

        let output = if let Some(timeout_ms) = request.timeout_ms {
            match timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await {
                Ok(output) => output,
                Err(_) => return Err(MediaConversionError::TimedOut { timeout_ms }),
            }
        } else {
            child.wait_with_output().await
        }
        .map_err(|error| MediaConversionError::Io {
            message: error.to_string(),
        })?;

        Ok(ProcessRunOutput::new(
            output.status.code(),
            output.stdout,
            &output.stderr,
        ))
    }
}

#[derive(Clone)]
pub struct ManagedProcessConversionExecutor<R> {
    runner: Arc<R>,
    converter: ManagedMediaConverter,
}

impl<R> ManagedProcessConversionExecutor<R>
where
    R: ProcessRunner,
{
    pub fn new(runner: Arc<R>, converter: ManagedMediaConverter) -> Self {
        Self { runner, converter }
    }
}

#[async_trait]
impl<R> MediaConversionExecutor for ManagedProcessConversionExecutor<R>
where
    R: ProcessRunner + 'static,
{
    async fn convert(
        &self,
        request: MediaConversionRequest,
    ) -> Result<MediaConversionResult, MediaConversionError> {
        if request.source.media_type != self.converter.source_media_type
            || request.target.media_type != self.converter.target_media_type
        {
            return Err(MediaConversionError::UnsupportedConversion {
                source_media_type: request.source.media_type.to_string(),
                target_media_type: request.target.media_type.to_string(),
            });
        }

        let process_request = ProcessRunRequest::try_new(
            self.converter.executable_path.clone(),
            self.converter.args.clone(),
            request.source.body.clone(),
            request.timeout_ms,
        )?;
        let output = self.runner.run(process_request).await?;
        if !output.successful() {
            return Err(MediaConversionError::ProcessFailed {
                status_code: output.status_code,
                stderr_summary: output
                    .stderr_summary
                    .unwrap_or_else(|| "no stderr captured".to_string()),
            });
        }

        let command_id = request.target.format_id.as_str().to_string();
        MediaConversionResult::try_new(
            request.conversion_id,
            MediaConversionStatus::Converted,
            self.converter.target_media_type.clone(),
            request.target,
            command_id,
            output.stdout,
            vec![self.converter.dependency.clone()],
            output.stderr_summary,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaConversionResult {
    pub conversion_id: MediaConversionId,
    pub status: MediaConversionStatus,
    pub media_type: MediaType,
    pub target: MediaConversionTarget,
    pub command_id: String,
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
        command_id: String,
        body: Vec<u8>,
        dependencies: Vec<MediaConversionDependencyAttribution>,
        stderr_summary: Option<String>,
    ) -> Result<Self, MediaConversionError> {
        if body.is_empty() {
            return Err(MediaConversionError::MissingField { field: "body" });
        }
        let command_id = validate_identifier("command_id", command_id)?;
        if let Some(summary) = stderr_summary.as_deref() {
            validate_text_field("stderr_summary", summary.to_string(), MAX_ERROR_SUMMARY_LEN)?;
        }
        for dependency in &dependencies {
            validate_text_field(
                "dependency_lease_holder",
                dependency.lease_holder.clone(),
                MAX_LEASE_HOLDER_LEN,
            )?;
        }
        Ok(Self {
            conversion_id,
            status,
            media_type,
            target,
            command_id,
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

fn validate_executable_path(field: &'static str, path: &Path) -> Result<(), MediaConversionError> {
    let value = path.as_os_str().to_string_lossy();
    if value.trim().is_empty() {
        return Err(MediaConversionError::MissingField { field });
    }
    if !path.is_absolute() {
        return Err(MediaConversionError::InvalidExecutablePath {
            field,
            reason: "path is not absolute".to_string(),
        });
    }
    if value.chars().any(char::is_control) {
        return Err(MediaConversionError::InvalidExecutablePath {
            field,
            reason: "path contains control characters".to_string(),
        });
    }
    if value
        .chars()
        .any(|ch| matches!(ch, ';' | '|' | '&' | '<' | '>' | '`' | '$'))
    {
        return Err(MediaConversionError::InvalidExecutablePath {
            field,
            reason: "path contains shell metacharacters".to_string(),
        });
    }
    Ok(())
}

fn validate_process_arg(arg: &str) -> Result<(), MediaConversionError> {
    if arg.is_empty() {
        return Err(MediaConversionError::MissingField { field: "arg" });
    }
    if arg.chars().any(char::is_control) {
        return Err(MediaConversionError::InvalidText { field: "arg" });
    }
    Ok(())
}

fn ensure_target_media_type(
    kind: ConversionMediaKind,
    target: &MediaConversionTarget,
    expected_prefix: &str,
) -> Result<(), MediaConversionError> {
    if target.media_type.as_str().starts_with(expected_prefix) {
        Ok(())
    } else {
        Err(unsupported_command_plan(
            kind,
            target,
            "target media type does not match requested conversion kind",
        ))
    }
}

fn unsupported_command_plan(
    kind: ConversionMediaKind,
    target: &MediaConversionTarget,
    reason: impl Into<String>,
) -> MediaConversionError {
    MediaConversionError::UnsupportedCommandPlan {
        kind,
        target_media_type: target.media_type.to_string(),
        reason: reason.into(),
    }
}

fn push_unique_dependency(
    dependency_ids: &mut Vec<ManagedMediaDependencyId>,
    dependency_id: ManagedMediaDependencyId,
) {
    if !dependency_ids.contains(&dependency_id) {
        dependency_ids.push(dependency_id);
    }
}

fn ffmpeg_common_argv(target: &MediaConversionTarget) -> Vec<String> {
    vec![
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-i".to_string(),
        "pipe:0".to_string(),
        "-f".to_string(),
        target.format_id.as_str().to_string(),
    ]
}

fn bounded_stderr_summary(stderr: &[u8]) -> Option<String> {
    let normalized = String::from_utf8_lossy(stderr)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if normalized.is_empty() {
        return None;
    }
    Some(normalized.chars().take(MAX_ERROR_SUMMARY_LEN).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pantograph_managed_dependencies::RuntimeSidecarDependencyId;
    use std::future::Future;
    use std::sync::Mutex;
    use std::task::{Context, Poll, Wake, Waker};

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

    #[allow(clippy::too_many_arguments)]
    fn command_target(
        format_id: &str,
        media_type: &str,
        codec_id: Option<&str>,
        quality_percent: Option<u8>,
        bitrate_kbps: Option<u32>,
        crf: Option<u8>,
        bit_depth: Option<&str>,
        color_profile_id: Option<&str>,
        color_managed: bool,
    ) -> MediaConversionTarget {
        MediaConversionTarget::try_new(
            format_id.parse().expect("format id"),
            media_type.parse().expect("media type"),
            codec_id.map(|value| value.parse().expect("codec id")),
            quality_percent,
            bitrate_kbps,
            crf,
            bit_depth.map(|value| value.parse().expect("bit depth")),
            color_profile_id.map(|value| value.parse().expect("color profile")),
            color_managed,
        )
        .expect("target")
    }

    fn request(timeout_ms: Option<u64>) -> MediaConversionRequest {
        let source = MediaConversionSource::try_new(
            id("artifact-a"),
            "image/png".parse().expect("source media type"),
            vec![1, 2, 3],
        )
        .expect("source");
        let attribution = MediaConversionAttribution {
            workflow_run_id: id("run-a"),
            source_artifact_id: id("artifact-a"),
            node_id: None,
            port_id: None,
        };
        MediaConversionRequest::try_new(
            id("conversion-a"),
            ConversionMediaKind::Image,
            attribution,
            source,
            target(),
            timeout_ms,
        )
        .expect("request")
    }

    fn converter() -> ManagedMediaConverter {
        ManagedMediaConverter::try_new(
            MediaConversionDependencyAttribution {
                dependency_id: ManagedMediaDependencyId::Oiiotool,
                version: id("2.5.18"),
                lease_id: id("lease-1"),
                lease_holder:
                    "workflow_run:run-a/node:node-a/port:port-image/conversion:conversion-a"
                        .to_string(),
            },
            ManagedExecutablePath::try_new(PathBuf::from("/managed/bin/oiiotool"))
                .expect("executable path"),
            "image/png".parse().expect("source media type"),
            "image/jpeg".parse().expect("target media type"),
            vec!["--stdout".to_string()],
        )
        .expect("converter")
    }

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    fn block_on<F>(future: F) -> F::Output
    where
        F: Future,
    {
        let waker = Waker::from(Arc::new(NoopWaker));
        let mut context = Context::from_waker(&waker);
        let mut future = Box::pin(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    #[derive(Default)]
    struct FakeProcessRunner {
        requests: Mutex<Vec<ProcessRunRequest>>,
        result: Mutex<Option<Result<ProcessRunOutput, MediaConversionError>>>,
    }

    impl FakeProcessRunner {
        fn with_result(result: Result<ProcessRunOutput, MediaConversionError>) -> Self {
            Self {
                requests: Mutex::new(Vec::new()),
                result: Mutex::new(Some(result)),
            }
        }

        fn requests(&self) -> Vec<ProcessRunRequest> {
            self.requests.lock().expect("requests").clone()
        }
    }

    #[async_trait]
    impl ProcessRunner for FakeProcessRunner {
        async fn run(
            &self,
            request: ProcessRunRequest,
        ) -> Result<ProcessRunOutput, MediaConversionError> {
            self.requests.lock().expect("requests").push(request);
            self.result
                .lock()
                .expect("result")
                .take()
                .expect("fake result")
        }
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
    fn image_command_plan_uses_oiiotool_defaults_without_host_paths() {
        let target = command_target(
            "png",
            "image/png",
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );

        let plan = plan_image_command(target.clone()).expect("image plan");

        assert_eq!(plan.kind, ConversionMediaKind::Image);
        assert_eq!(plan.target, target);
        assert_eq!(
            plan.required_dependency_ids,
            vec![ManagedMediaDependencyId::Oiiotool]
        );
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(
            plan.steps[0],
            MediaCommandPlanStep {
                dependency_id: ManagedMediaDependencyId::Oiiotool,
                argv: vec![
                    "-".to_string(),
                    "--format".to_string(),
                    "png".to_string(),
                    "-o".to_string(),
                    "-".to_string(),
                ],
                input: MediaCommandPlanStream::Stdin,
                output: MediaCommandPlanStream::Stdout,
            }
        );
    }

    #[test]
    fn image_command_plan_includes_explicit_target_fields_and_color_management() {
        let target = command_target(
            "jpg",
            "image/jpeg",
            Some("jpeg"),
            Some(82),
            None,
            None,
            Some("uint8"),
            Some("acescg"),
            true,
        );

        let plan = MediaCommandPlan::try_for_target(ConversionMediaKind::Image, target)
            .expect("image plan");

        assert_eq!(
            plan.required_dependency_ids,
            vec![
                ManagedMediaDependencyId::OpenColorIo,
                ManagedMediaDependencyId::Ocioconvert,
                ManagedMediaDependencyId::Oiiotool
            ]
        );
        assert_eq!(plan.steps.len(), 2);
        assert_eq!(
            plan.steps[0].dependency_id,
            ManagedMediaDependencyId::Ocioconvert
        );
        assert_eq!(
            plan.steps[0].argv,
            vec![
                "--input",
                "-",
                "--output",
                "-",
                "--output-color-space",
                "acescg"
            ]
        );
        assert_eq!(
            plan.steps[1].dependency_id,
            ManagedMediaDependencyId::Oiiotool
        );
        assert_eq!(
            plan.steps[1].argv,
            vec![
                "-",
                "--format",
                "jpg",
                "--compression",
                "jpeg",
                "--quality",
                "82",
                "--bitdepth",
                "uint8",
                "-o",
                "-"
            ]
        );
    }

    #[test]
    fn audio_command_plan_uses_ffmpeg_defaults_and_explicit_fields() {
        let defaults = command_target(
            "wav",
            "audio/wav",
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );
        let explicit = command_target(
            "mp3",
            "audio/mpeg",
            Some("libmp3lame"),
            Some(4),
            Some(192),
            None,
            None,
            None,
            false,
        );

        let defaults_plan = plan_audio_command(defaults).expect("default audio plan");
        let explicit_plan = plan_audio_command(explicit).expect("explicit audio plan");

        assert_eq!(
            defaults_plan.required_dependency_ids,
            vec![ManagedMediaDependencyId::Ffmpeg]
        );
        assert_eq!(
            defaults_plan.steps[0].argv,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                "pipe:0",
                "-f",
                "wav",
                "pipe:1"
            ]
        );
        assert_eq!(
            explicit_plan.steps[0].argv,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                "pipe:0",
                "-f",
                "mp3",
                "-codec:a",
                "libmp3lame",
                "-b:a",
                "192k",
                "-q:a",
                "4",
                "pipe:1"
            ]
        );
    }

    #[test]
    fn video_command_plan_uses_ffmpeg_defaults_and_explicit_fields() {
        let defaults = command_target(
            "mp4",
            "video/mp4",
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );
        let explicit = command_target(
            "mp4",
            "video/mp4",
            Some("libx264"),
            None,
            Some(4_000),
            Some(23),
            None,
            None,
            false,
        );

        let defaults_plan = plan_video_command(defaults).expect("default video plan");
        let explicit_plan = plan_video_command(explicit).expect("explicit video plan");

        assert_eq!(
            defaults_plan.required_dependency_ids,
            vec![ManagedMediaDependencyId::Ffmpeg]
        );
        assert_eq!(
            defaults_plan.steps[0].argv,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                "pipe:0",
                "-f",
                "mp4",
                "pipe:1"
            ]
        );
        assert_eq!(
            explicit_plan.steps[0].argv,
            vec![
                "-hide_banner",
                "-loglevel",
                "error",
                "-i",
                "pipe:0",
                "-f",
                "mp4",
                "-codec:v",
                "libx264",
                "-b:v",
                "4000k",
                "-crf",
                "23",
                "pipe:1"
            ]
        );
    }

    #[test]
    fn command_planning_fails_closed_for_unsupported_3d_and_kind_mismatch() {
        let three_d_target = command_target(
            "glb",
            "model/gltf-binary",
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );
        let image_target = command_target(
            "png",
            "image/png",
            None,
            None,
            None,
            None,
            None,
            None,
            false,
        );

        let unsupported_three_d =
            MediaCommandPlan::try_for_target(ConversionMediaKind::ThreeD, three_d_target)
                .expect_err("unsupported 3D plan");
        assert!(matches!(
            unsupported_three_d,
            MediaConversionError::UnsupportedCommandPlan {
                kind: ConversionMediaKind::ThreeD,
                ..
            }
        ));

        let mismatch = MediaCommandPlan::try_for_target(ConversionMediaKind::Audio, image_target)
            .expect_err("kind mismatch");
        assert!(matches!(
            mismatch,
            MediaConversionError::UnsupportedCommandPlan {
                kind: ConversionMediaKind::Audio,
                ..
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
            lease_holder: "workflow_run:run-a/node:node-a/port:port-image/conversion:conversion-a"
                .to_string(),
        };

        let result = MediaConversionResult::try_new(
            id("conversion-a"),
            MediaConversionStatus::Converted,
            "image/jpeg".parse().expect("media type"),
            target(),
            "oiiotool_jpg".to_string(),
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
            "oiiotool_jpg".to_string(),
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

    #[test]
    fn executable_path_rejects_empty_relative_and_command_strings() {
        let empty =
            ManagedExecutablePath::try_new(PathBuf::from("")).expect_err("empty executable path");
        assert!(matches!(
            empty,
            MediaConversionError::MissingField {
                field: "executable_path"
            }
        ));

        let relative = ManagedExecutablePath::try_new(PathBuf::from("ffmpeg"))
            .expect_err("relative executable path");
        assert!(matches!(
            relative,
            MediaConversionError::InvalidExecutablePath {
                field: "executable_path",
                ..
            }
        ));

        let spaced_path = ManagedExecutablePath::try_new(PathBuf::from("/managed tools/ffmpeg"))
            .expect("spaced managed path");
        assert_eq!(spaced_path.as_path(), Path::new("/managed tools/ffmpeg"));

        let command_string =
            ManagedExecutablePath::try_new(PathBuf::from("/usr/bin/ffmpeg; rm -rf /"))
                .expect_err("command string");
        assert!(matches!(
            command_string,
            MediaConversionError::InvalidExecutablePath {
                field: "executable_path",
                ..
            }
        ));
    }

    #[test]
    fn managed_executor_runs_process_with_separate_args_and_records_dependency() {
        let runner = Arc::new(FakeProcessRunner::with_result(Ok(ProcessRunOutput::new(
            Some(0),
            vec![9, 8, 7],
            b"converted\nok",
        ))));
        let executor = ManagedProcessConversionExecutor::new(runner.clone(), converter());

        let result = block_on(executor.convert(request(Some(500)))).expect("convert");

        assert_eq!(result.body, vec![9, 8, 7]);
        assert_eq!(result.stderr_summary.as_deref(), Some("converted ok"));
        assert_eq!(
            result.dependencies[0].dependency_id,
            ManagedMediaDependencyId::Oiiotool
        );

        let requests = runner.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].executable_path.as_path(),
            Path::new("/managed/bin/oiiotool")
        );
        assert_eq!(requests[0].args, vec!["--stdout"]);
        assert_eq!(requests[0].stdin, vec![1, 2, 3]);
        assert_eq!(requests[0].timeout_ms, Some(500));
    }

    #[test]
    fn managed_executor_maps_non_zero_status_to_bounded_process_failure() {
        let stderr = vec![b'x'; MAX_ERROR_SUMMARY_LEN + 64];
        let runner = Arc::new(FakeProcessRunner::with_result(Ok(ProcessRunOutput::new(
            Some(2),
            Vec::new(),
            &stderr,
        ))));
        let executor = ManagedProcessConversionExecutor::new(runner, converter());

        let error = executor.convert(request(None));
        let error = block_on(error).expect_err("process failure");

        match error {
            MediaConversionError::ProcessFailed {
                status_code,
                stderr_summary,
            } => {
                assert_eq!(status_code, Some(2));
                assert_eq!(stderr_summary.len(), MAX_ERROR_SUMMARY_LEN);
                assert!(stderr_summary.chars().all(|ch| ch == 'x'));
            }
            other => panic!("expected process failure, got {other:?}"),
        }
    }

    #[test]
    fn managed_executor_preserves_timeout_and_cancellation_errors() {
        let timeout_runner = Arc::new(FakeProcessRunner::with_result(Err(
            MediaConversionError::TimedOut { timeout_ms: 10 },
        )));
        let timeout_executor = ManagedProcessConversionExecutor::new(timeout_runner, converter());
        assert!(matches!(
            block_on(timeout_executor.convert(request(Some(10)))),
            Err(MediaConversionError::TimedOut { timeout_ms: 10 })
        ));

        let cancel_runner = Arc::new(FakeProcessRunner::with_result(Err(
            MediaConversionError::Cancelled,
        )));
        let cancel_executor = ManagedProcessConversionExecutor::new(cancel_runner, converter());
        assert!(matches!(
            block_on(cancel_executor.convert(request(None))),
            Err(MediaConversionError::Cancelled)
        ));
    }

    #[test]
    fn managed_media_dependency_ids_round_trip_through_neutral_keys() {
        for dependency_id in [
            ManagedMediaDependencyId::Ffmpeg,
            ManagedMediaDependencyId::Ocioconvert,
            ManagedMediaDependencyId::Oiiotool,
            ManagedMediaDependencyId::OpenColorIo,
        ] {
            let key = ManagedDependencyKey::from(dependency_id);
            assert_eq!(key.stable_key(), dependency_id.to_string());
            assert_eq!(
                ManagedMediaDependencyId::try_from(key).expect("round trip dependency id"),
                dependency_id
            );
        }
    }

    #[test]
    fn runtime_sidecars_are_not_media_conversion_dependencies() {
        let error = ManagedMediaDependencyId::try_from(ManagedDependencyKey::RuntimeSidecar(
            RuntimeSidecarDependencyId::LlamaCpp,
        ))
        .expect_err("runtime sidecars are not media conversion dependencies");

        match error {
            MediaConversionError::UnsupportedManagedDependencyKey { key } => {
                assert_eq!(key, "llama_cpp");
            }
            other => panic!("expected unsupported key error, got {other:?}"),
        }
    }
}
