use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct MediaFormatOption {
    pub format_id: String,
    pub display_name: String,
    pub media_type: String,
    #[serde(default)]
    pub codec_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_min_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_max_percent: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_min_kbps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitrate_max_kbps: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf_min: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crf_max: Option<u8>,
    #[serde(default)]
    pub bit_depths: Vec<String>,
    #[serde(default)]
    pub color_profile_ids: Vec<String>,
    pub provided_by_dependency_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provided_by_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ArtifactFormatCapabilities {
    #[serde(default)]
    pub image_formats: Vec<MediaFormatOption>,
    #[serde(default)]
    pub audio_formats: Vec<MediaFormatOption>,
    #[serde(default)]
    pub video_formats: Vec<MediaFormatOption>,
    #[serde(default)]
    pub three_d_formats: Vec<MediaFormatOption>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableCategory {
    RuntimeSidecar,
    ToolBinary,
    NativeLibraryArtifact,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableReadinessState {
    Missing,
    Downloading,
    Extracting,
    Validating,
    Ready,
    Failed,
    Incompatible,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableStatus {
    pub dependency_id: String,
    pub category: ManagedRedistributableCategory,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_version: Option<String>,
    pub readiness_state: ManagedRedistributableReadinessState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license_id: Option<String>,
    pub source_owner: String,
    pub platform: String,
    #[serde(default)]
    pub expected_files: Vec<String>,
    #[serde(default)]
    pub missing_files: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableStatusQueryResponse {
    #[serde(default)]
    pub dependencies: Vec<ManagedRedistributableStatus>,
}
