use serde::{Deserialize, Serialize};

const MANAGED_REDISTRIBUTABLES_STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableId {
    Ffmpeg,
    Ocioconvert,
    Oiiotool,
    OpenColorIo,
}

impl ManagedRedistributableId {
    pub fn all() -> &'static [Self] {
        &[
            Self::Ffmpeg,
            Self::Ocioconvert,
            Self::Oiiotool,
            Self::OpenColorIo,
        ]
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ocioconvert => "ocioconvert",
            Self::Oiiotool => "oiiotool",
            Self::OpenColorIo => "opencolorio",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Ffmpeg => "FFmpeg",
            Self::Ocioconvert => "ocioconvert",
            Self::Oiiotool => "oiiotool",
            Self::OpenColorIo => "OpenColorIO",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableCategory {
    ToolBinary,
    NativeLibraryArtifact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableInstallState {
    Installed,
    Missing,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableReadiness {
    Missing,
    Ready,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributablePackageKind {
    Archive,
    NativePackage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRedistributableArchiveKind {
    TarGz,
    TarXz,
    Zip,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableSource {
    pub owner: String,
    pub project: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableCatalogEntry {
    pub id: ManagedRedistributableId,
    pub display_name: String,
    pub category: ManagedRedistributableCategory,
    pub source: ManagedRedistributableSource,
    pub license_redistribution: String,
    pub platform_key: String,
    pub version: String,
    pub package_kind: ManagedRedistributablePackageKind,
    pub archive_kind: Option<ManagedRedistributableArchiveKind>,
    pub archive_name: Option<String>,
    pub download_url: Option<String>,
    pub expected_files: Vec<String>,
    pub checksum_sha256: Option<String>,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableSelection {
    pub selected_version: Option<String>,
    pub active_version: Option<String>,
    pub default_version: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableLease {
    pub id: String,
    pub version: String,
    pub holder: String,
    pub acquired_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributablePersistedDependency {
    pub id: ManagedRedistributableId,
    #[serde(default)]
    pub selection: ManagedRedistributableSelection,
    #[serde(default)]
    pub active_leases: Vec<ManagedRedistributableLease>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributablePersistedState {
    pub schema_version: u32,
    #[serde(default)]
    pub dependencies: Vec<ManagedRedistributablePersistedDependency>,
}

impl Default for ManagedRedistributablePersistedState {
    fn default() -> Self {
        Self {
            schema_version: MANAGED_REDISTRIBUTABLES_STATE_SCHEMA_VERSION,
            dependencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableLeaseToken {
    pub id: ManagedRedistributableId,
    pub version: String,
    pub lease_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableVersionStatus {
    pub version: String,
    pub platform_key: String,
    pub install_root: String,
    pub expected_files: Vec<String>,
    pub missing_files: Vec<String>,
    pub install_state: ManagedRedistributableInstallState,
    pub readiness: ManagedRedistributableReadiness,
    pub selected: bool,
    pub active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRedistributableStatus {
    pub id: ManagedRedistributableId,
    pub display_name: String,
    pub category: ManagedRedistributableCategory,
    pub install_state: ManagedRedistributableInstallState,
    pub readiness: ManagedRedistributableReadiness,
    pub available: bool,
    pub missing_files: Vec<String>,
    pub catalog: ManagedRedistributableCatalogEntry,
    pub selection: ManagedRedistributableSelection,
    pub versions: Vec<ManagedRedistributableVersionStatus>,
}
