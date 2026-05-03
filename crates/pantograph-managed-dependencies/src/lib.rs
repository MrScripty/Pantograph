//! Neutral managed dependency contracts for Pantograph.
//!
//! This crate defines transport-safe DTOs for runtime sidecars, media tools,
//! native artifacts, install/readiness status, leases, activation, and resolved
//! commands. It intentionally contains no installer, downloader, process
//! launcher, scheduler, or workflow policy.

use serde::{Deserialize, Serialize};

pub mod redistributables;
pub use redistributables::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyKey {
    RuntimeSidecar(RuntimeSidecarDependencyId),
    MediaTool(MediaToolDependencyId),
    NativeArtifact(NativeArtifactDependencyId),
}

impl ManagedDependencyKey {
    pub fn category(self) -> ManagedDependencyCategory {
        match self {
            Self::RuntimeSidecar(_) => ManagedDependencyCategory::RuntimeSidecar,
            Self::MediaTool(_) => ManagedDependencyCategory::MediaTool,
            Self::NativeArtifact(_) => ManagedDependencyCategory::NativeArtifact,
        }
    }

    pub fn stable_key(self) -> &'static str {
        match self {
            Self::RuntimeSidecar(id) => id.stable_key(),
            Self::MediaTool(id) => id.stable_key(),
            Self::NativeArtifact(id) => id.stable_key(),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::RuntimeSidecar(id) => id.display_name(),
            Self::MediaTool(id) => id.display_name(),
            Self::NativeArtifact(id) => id.display_name(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSidecarDependencyId {
    LlamaCpp,
}

impl RuntimeSidecarDependencyId {
    pub fn stable_key(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama_cpp",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaToolDependencyId {
    Ffmpeg,
    Ocioconvert,
    Oiiotool,
}

impl MediaToolDependencyId {
    pub fn stable_key(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ocioconvert => "ocioconvert",
            Self::Oiiotool => "oiiotool",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ffmpeg => "FFmpeg",
            Self::Ocioconvert => "ocioconvert",
            Self::Oiiotool => "oiiotool",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeArtifactDependencyId {
    OpenColorIo,
}

impl NativeArtifactDependencyId {
    pub fn stable_key(self) -> &'static str {
        match self {
            Self::OpenColorIo => "opencolorio",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenColorIo => "OpenColorIO",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyCategory {
    RuntimeSidecar,
    MediaTool,
    NativeArtifact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyOperation {
    RuntimeCommandResolution,
    MediaExecutableResolution,
    NativeArtifactActivation,
    LeaseAcquisition,
    LeaseRelease,
    Install,
    Remove,
    CatalogRefresh,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyOperationScope {
    Common,
    RuntimeSidecar,
    MediaTool,
    NativeArtifact,
}

impl ManagedDependencyOperation {
    pub fn scope(self) -> ManagedDependencyOperationScope {
        match self {
            Self::LeaseAcquisition | Self::LeaseRelease | Self::Install | Self::Remove => {
                ManagedDependencyOperationScope::Common
            }
            Self::RuntimeCommandResolution | Self::CatalogRefresh => {
                ManagedDependencyOperationScope::RuntimeSidecar
            }
            Self::MediaExecutableResolution => ManagedDependencyOperationScope::MediaTool,
            Self::NativeArtifactActivation => ManagedDependencyOperationScope::NativeArtifact,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyInstallState {
    Installed,
    SystemProvided,
    Missing,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyReadinessState {
    Unknown,
    Missing,
    Downloading,
    Extracting,
    Validating,
    Ready,
    Failed,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencySelectionState {
    pub selected_version: Option<String>,
    pub active_version: Option<String>,
    pub default_version: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencySource {
    pub owner: String,
    pub project: String,
    pub license_redistribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencyDescriptor {
    pub key: ManagedDependencyKey,
    pub display_name: String,
    pub category: ManagedDependencyCategory,
    pub source: Option<ManagedDependencySource>,
    pub platform_key: String,
    pub version: Option<String>,
    pub expected_files: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencyVersionStatus {
    pub version: Option<String>,
    pub platform_key: String,
    pub install_root: Option<String>,
    pub expected_files: Vec<String>,
    pub missing_files: Vec<String>,
    pub install_state: ManagedDependencyInstallState,
    pub readiness_state: ManagedDependencyReadinessState,
    pub selected: bool,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencyStatus {
    pub key: ManagedDependencyKey,
    pub display_name: String,
    pub category: ManagedDependencyCategory,
    pub install_state: ManagedDependencyInstallState,
    pub readiness_state: ManagedDependencyReadinessState,
    pub available: bool,
    pub missing_files: Vec<String>,
    pub selection: ManagedDependencySelectionState,
    pub versions: Vec<ManagedDependencyVersionStatus>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencyLease {
    pub key: ManagedDependencyKey,
    pub version: String,
    pub lease_id: String,
    pub holder: String,
    pub acquired_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedDependencyActivation {
    pub key: ManagedDependencyKey,
    pub version: String,
    pub install_root: String,
    pub expected_files: Vec<String>,
    pub validation_state: ManagedDependencyActivationValidationState,
    pub validation_reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedDependencyActivationValidationState {
    NotValidated,
    Valid,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedManagedDependencyCommand {
    pub key: ManagedDependencyKey,
    pub executable_path: String,
    pub working_directory: String,
    pub args: Vec<String>,
    pub env_overrides: Vec<(String, String)>,
    pub pid_file: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_keys_expose_stable_category_and_display_facts() {
        let llama = ManagedDependencyKey::RuntimeSidecar(RuntimeSidecarDependencyId::LlamaCpp);
        let ffmpeg = ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg);
        let ocio = ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo);

        assert_eq!(llama.category(), ManagedDependencyCategory::RuntimeSidecar);
        assert_eq!(llama.stable_key(), "llama_cpp");
        assert_eq!(llama.display_name(), "llama.cpp");

        assert_eq!(ffmpeg.category(), ManagedDependencyCategory::MediaTool);
        assert_eq!(ffmpeg.stable_key(), "ffmpeg");
        assert_eq!(ffmpeg.display_name(), "FFmpeg");

        assert_eq!(ocio.category(), ManagedDependencyCategory::NativeArtifact);
        assert_eq!(ocio.stable_key(), "opencolorio");
        assert_eq!(ocio.display_name(), "OpenColorIO");
    }

    #[test]
    fn operation_scope_separates_common_and_category_specific_actions() {
        assert_eq!(
            ManagedDependencyOperation::Install.scope(),
            ManagedDependencyOperationScope::Common
        );
        assert_eq!(
            ManagedDependencyOperation::RuntimeCommandResolution.scope(),
            ManagedDependencyOperationScope::RuntimeSidecar
        );
        assert_eq!(
            ManagedDependencyOperation::MediaExecutableResolution.scope(),
            ManagedDependencyOperationScope::MediaTool
        );
        assert_eq!(
            ManagedDependencyOperation::NativeArtifactActivation.scope(),
            ManagedDependencyOperationScope::NativeArtifact
        );
    }

    #[test]
    fn status_json_shape_is_stable() {
        let status = ManagedDependencyStatus {
            key: ManagedDependencyKey::MediaTool(MediaToolDependencyId::Oiiotool),
            display_name: "oiiotool".to_string(),
            category: ManagedDependencyCategory::MediaTool,
            install_state: ManagedDependencyInstallState::Installed,
            readiness_state: ManagedDependencyReadinessState::Ready,
            available: true,
            missing_files: Vec::new(),
            selection: ManagedDependencySelectionState {
                selected_version: Some("v1".to_string()),
                active_version: Some("v1".to_string()),
                default_version: None,
            },
            versions: vec![ManagedDependencyVersionStatus {
                version: Some("v1".to_string()),
                platform_key: "linux-x86_64".to_string(),
                install_root: Some("/tmp/pantograph/oiiotool".to_string()),
                expected_files: vec!["bin/oiiotool".to_string()],
                missing_files: Vec::new(),
                install_state: ManagedDependencyInstallState::Installed,
                readiness_state: ManagedDependencyReadinessState::Ready,
                selected: true,
                active: true,
            }],
            unavailable_reason: None,
        };

        let value = serde_json::to_value(&status).expect("serialize status");
        assert_eq!(value["key"]["media_tool"], "oiiotool");
        assert_eq!(value["category"], "media_tool");
        assert_eq!(value["install_state"], "installed");
        assert_eq!(value["readiness_state"], "ready");

        let roundtrip: ManagedDependencyStatus =
            serde_json::from_value(value).expect("deserialize status");
        assert_eq!(roundtrip, status);
    }
}
