use pantograph_diagnostics_ledger::IoArtifactRetentionState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPayloadKind {
    Text,
    Image,
    Audio,
    Video,
    #[serde(rename = "3d")]
    ThreeD,
    LargeTable,
    GenericBinary,
    Structured,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactLifecycleState {
    Declared,
    Writing,
    Streaming,
    Finalizing,
    Retained,
    Failed,
    Expired,
    Deleted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactAccessMode {
    Read,
    Download,
    Stream,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBodyTransport {
    BinaryBody,
    RedirectUrl,
    StreamHandle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactAttribution {
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatMetadata {
    pub format_id: String,
    pub media_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codec_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_depth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converter_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub library_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub payload_kind: ArtifactPayloadKind,
    pub lifecycle_state: ArtifactLifecycleState,
    pub retention_state: IoArtifactRetentionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_length: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<ArtifactFormatMetadata>,
    pub attribution: ArtifactAttribution,
    #[serde(default)]
    pub access_modes: Vec<ArtifactAccessMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactPolicy {
    pub policy_id: String,
    pub policy_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_disk_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_memory_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_single_artifact_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spill_threshold_bytes: Option<u64>,
    pub delete_on_consume: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactDescriptorQueryRequest {
    pub artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactDescriptorQueryResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<ArtifactDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactReadRequest {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range_start: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range_end_exclusive: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactReadResponse {
    pub artifact_id: String,
    pub media_type: String,
    pub body_transport: ArtifactBodyTransport,
    pub read_handle: String,
    pub byte_length: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactConsumeAcknowledgementRequest {
    pub artifact_id: String,
    pub consumer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactConsumeAcknowledgementResponse {
    pub artifact_id: String,
    pub retained_after_consume: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactStreamChunkRecord {
    pub artifact_id: String,
    pub stream_handle: String,
    pub sequence: u64,
    pub byte_length: u64,
    pub lifecycle_state: ArtifactLifecycleState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettings {
    pub image: ImageArtifactFormatSettings,
    pub audio: AudioArtifactFormatSettings,
    pub video: VideoArtifactFormatSettings,
    pub three_d: ThreeDArtifactFormatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ImageArtifactFormatSettings {
    pub format_id: String,
    pub quality_percent: u8,
    pub color_profile_id: String,
}

impl Default for ImageArtifactFormatSettings {
    fn default() -> Self {
        Self {
            format_id: "jpg".to_string(),
            quality_percent: 75,
            color_profile_id: "srgb".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AudioArtifactFormatSettings {
    pub container_id: String,
    pub codec_id: String,
    pub bitrate_kbps: u32,
}

impl Default for AudioArtifactFormatSettings {
    fn default() -> Self {
        Self {
            container_id: "ogg".to_string(),
            codec_id: "opus".to_string(),
            bitrate_kbps: 96,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct VideoArtifactFormatSettings {
    pub container_id: String,
    pub codec_id: String,
    pub crf: u8,
    pub bit_depth: String,
}

impl Default for VideoArtifactFormatSettings {
    fn default() -> Self {
        Self {
            container_id: "ivf".to_string(),
            codec_id: "svt_av1".to_string(),
            crf: 32,
            bit_depth: "8bit".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ThreeDArtifactFormatSettings {
    pub format_id: String,
}

impl Default for ThreeDArtifactFormatSettings {
    fn default() -> Self {
        Self {
            format_id: "glb".to_string(),
        }
    }
}
