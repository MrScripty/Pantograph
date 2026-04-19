use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBinaryId {
    LlamaCpp,
    Ollama,
}

impl ManagedBinaryId {
    pub fn all() -> &'static [Self] {
        &[Self::LlamaCpp, Self::Ollama]
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama_cpp",
            Self::Ollama => "ollama",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
        }
    }

    pub(crate) fn install_dir_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama-cpp",
            Self::Ollama => "ollama",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBinaryInstallState {
    Installed,
    SystemProvided,
    Missing,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedBinaryCapability {
    pub id: ManagedBinaryId,
    pub display_name: String,
    pub install_state: ManagedBinaryInstallState,
    pub available: bool,
    pub can_install: bool,
    pub can_remove: bool,
    pub missing_files: Vec<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryStatus {
    pub available: bool,
    pub missing_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuntimeReadinessState {
    Unknown,
    Missing,
    Downloading,
    Extracting,
    Validating,
    Ready,
    Failed,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuntimeJobState {
    Queued,
    Downloading,
    Paused,
    Extracting,
    Validating,
    Ready,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeSelectionState {
    pub selected_version: Option<String>,
    pub active_version: Option<String>,
    pub default_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeVersionStatus {
    pub version: Option<String>,
    pub display_label: String,
    pub install_state: ManagedBinaryInstallState,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub selected: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeJobStatus {
    pub state: ManagedRuntimeJobState,
    pub status: String,
    pub current: u64,
    pub total: u64,
    pub resumable: bool,
    pub cancellable: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeSnapshot {
    pub id: ManagedBinaryId,
    pub display_name: String,
    pub install_state: ManagedBinaryInstallState,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub available: bool,
    pub can_install: bool,
    pub can_remove: bool,
    pub missing_files: Vec<String>,
    pub unavailable_reason: Option<String>,
    #[serde(default)]
    pub versions: Vec<ManagedRuntimeVersionStatus>,
    #[serde(default)]
    pub selection: ManagedRuntimeSelectionState,
    pub active_job: Option<ManagedRuntimeJobStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub status: String,
    pub current: u64,
    pub total: u64,
    pub done: bool,
    pub error: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArchiveKind {
    TarGz,
    TarZst,
    Zip,
}

#[derive(Clone, Debug)]
pub(crate) struct ReleaseAsset {
    pub(crate) archive_name: String,
    pub(crate) archive_kind: ArchiveKind,
}

#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    pub executable_path: PathBuf,
    pub working_directory: PathBuf,
    pub args: Vec<OsString>,
    pub env_overrides: Vec<(OsString, OsString)>,
    pub pid_file: Option<PathBuf>,
}
