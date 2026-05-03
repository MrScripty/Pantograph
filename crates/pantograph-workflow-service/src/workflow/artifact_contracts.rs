use pantograph_diagnostics_ledger::IoArtifactRetentionState;
use pantograph_managed_dependencies::{ManagedDependencyCategory, ManagedDependencyStatus};
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
pub enum ArtifactConversionStatus {
    Converted,
    PassedThrough,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactConversionDependency {
    pub dependency_id: String,
    pub active_version: String,
    pub lease_id: String,
    pub lease_holder: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_status: Option<ArtifactConversionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversion_command_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conversion_dependencies: Vec<ArtifactConversionDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactDescriptor {
    pub artifact_id: String,
    pub payload_kind: ArtifactPayloadKind,
    pub lifecycle_state: ArtifactLifecycleState,
    pub retention_state: IoArtifactRetentionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_artifact_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_index: Option<u64>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactStreamReadRequest {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range_start: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_range_end_exclusive: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactStreamReadResponse {
    pub artifact_id: String,
    pub stream_handle: String,
    pub media_type: String,
    pub body_transport: ArtifactBodyTransport,
    pub byte_length: u64,
    pub available_byte_length: u64,
    pub lifecycle_state: ArtifactLifecycleState,
    pub complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactStreamBodyRead {
    pub response: ArtifactStreamReadResponse,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettings {
    pub image: ImageArtifactFormatSettings,
    pub audio: AudioArtifactFormatSettings,
    pub video: VideoArtifactFormatSettings,
    pub three_d: ThreeDArtifactFormatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettingsQueryRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettingsQueryResponse {
    pub settings: ArtifactFormatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettingsUpdateRequest {
    pub settings: ArtifactFormatSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatSettingsUpdateResponse {
    pub settings: ArtifactFormatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatDependencyVersions {
    #[serde(default)]
    pub dependencies: Vec<ArtifactFormatDependencyVersion>,
}

impl ArtifactFormatDependencyVersions {
    pub fn from_managed_dependency_statuses(statuses: &[ManagedDependencyStatus]) -> Self {
        Self {
            dependencies: statuses
                .iter()
                .filter(|status| {
                    matches!(
                        status.category,
                        ManagedDependencyCategory::MediaTool
                            | ManagedDependencyCategory::NativeArtifact
                    )
                })
                .map(|status| ArtifactFormatDependencyVersion {
                    dependency_id: status.key.stable_key().to_string(),
                    active_version: status.selection.active_version.clone(),
                })
                .collect(),
        }
    }

    pub fn active_version(&self, dependency_id: &str) -> Option<String> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.dependency_id == dependency_id)
            .and_then(|dependency| dependency.active_version.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatDependencyVersion {
    pub dependency_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_version: Option<String>,
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
