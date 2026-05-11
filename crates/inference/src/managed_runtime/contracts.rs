use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

use crate::{
    BackendId, DeviceResolutionDiagnostic, DeviceResolutionDiagnosticCode,
    DeviceResolutionDiagnosticSeverity, InferenceDeviceClass, RuntimeVariantId,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagedBinaryId {
    LlamaCpp,
}

impl ManagedBinaryId {
    pub fn all() -> &'static [Self] {
        &[Self::LlamaCpp]
    }

    pub fn is_first_party_supported(self) -> bool {
        matches!(self, Self::LlamaCpp)
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama_cpp",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama.cpp",
        }
    }

    pub(crate) fn install_dir_name(self) -> &'static str {
        match self {
            Self::LlamaCpp => "llama-cpp",
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
    #[serde(default)]
    pub selected_runtime_variant_id: Option<RuntimeVariantId>,
    pub active_version: Option<String>,
    #[serde(default)]
    pub active_runtime_variant_id: Option<RuntimeVariantId>,
    pub default_version: Option<String>,
    #[serde(default)]
    pub default_runtime_variant_id: Option<RuntimeVariantId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeCatalogVersion {
    pub version: String,
    pub display_label: String,
    pub runtime_key: String,
    pub runtime_variant_id: RuntimeVariantId,
    pub platform_key: String,
    pub archive_name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeVersionStatus {
    pub version: Option<String>,
    pub display_label: String,
    pub runtime_key: String,
    pub runtime_variant_id: RuntimeVariantId,
    pub platform_key: String,
    pub install_root: Option<String>,
    pub executable_name: String,
    pub executable_ready: bool,
    pub install_state: ManagedBinaryInstallState,
    pub readiness_state: ManagedRuntimeReadinessState,
    pub catalog_available: bool,
    pub installable: bool,
    pub selected: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ManagedRuntimeJobStatus {
    pub runtime_variant_id: RuntimeVariantId,
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
pub struct ManagedRuntimeJobArtifactStatus {
    pub runtime_variant_id: RuntimeVariantId,
    pub version: String,
    pub archive_name: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub retained: bool,
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
    pub job_artifact: Option<ManagedRuntimeJobArtifactStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub runtime_variant_id: RuntimeVariantId,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ManagedRuntimePathKind {
    InstallRoot,
    ExecutablePath,
    WorkingDirectory,
    DynamicLibraryPath,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ManagedRuntimeCommandResolutionError {
    UnsupportedRuntime {
        runtime_id: ManagedBinaryId,
    },
    MissingRuntimeFiles {
        runtime_id: ManagedBinaryId,
        display_name: String,
        missing_file: String,
    },
    MissingRuntimeVariant {
        diagnostic: DeviceResolutionDiagnostic,
        requested_device: Option<String>,
        missing_path: PathBuf,
    },
    State {
        message: String,
    },
    PathValidation {
        runtime_id: ManagedBinaryId,
        path_kind: ManagedRuntimePathKind,
        message: String,
    },
    Platform {
        message: String,
    },
}

impl ManagedRuntimeCommandResolutionError {
    pub(crate) fn platform(message: impl Into<String>) -> Self {
        Self::Platform {
            message: message.into(),
        }
    }

    pub(crate) fn state(message: impl Into<String>) -> Self {
        Self::State {
            message: message.into(),
        }
    }

    pub(crate) fn path_validation(
        runtime_id: ManagedBinaryId,
        path_kind: ManagedRuntimePathKind,
        message: impl Into<String>,
    ) -> Self {
        Self::PathValidation {
            runtime_id,
            path_kind,
            message: message.into(),
        }
    }

    pub(crate) fn missing_llamacpp_selected_variant(
        runtime_variant_id: &RuntimeVariantId,
        missing_path: PathBuf,
    ) -> Self {
        let backend_id = BackendId::parse("llama_cpp").expect("static backend id is valid");
        let device_class = match runtime_variant_id.as_str() {
            "llama_cpp.cuda" => Some(InferenceDeviceClass::Cuda),
            "llama_cpp.metal" => Some(InferenceDeviceClass::Metal),
            _ => None,
        };
        Self::MissingRuntimeVariant {
            diagnostic: DeviceResolutionDiagnostic {
                code: DeviceResolutionDiagnosticCode::MissingRuntimeVariant,
                severity: DeviceResolutionDiagnosticSeverity::Error,
                message: format!(
                    "llama.cpp runtime variant '{}' is selected but server binary is missing at {}",
                    runtime_variant_id,
                    missing_path.display()
                ),
                device_class,
                device_id: None,
                runtime_variant_id: Some(runtime_variant_id.clone()),
                backend_id: Some(backend_id),
            },
            requested_device: None,
            missing_path,
        }
    }
}

impl fmt::Display for ManagedRuntimeCommandResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedRuntime { runtime_id } => {
                write!(
                    formatter,
                    "{} is not a first-party managed runtime",
                    runtime_id.display_name()
                )
            }
            Self::MissingRuntimeFiles {
                display_name,
                missing_file,
                ..
            } => write!(
                formatter,
                "{} binaries are not installed for the current platform (missing {})",
                display_name, missing_file
            ),
            Self::MissingRuntimeVariant { diagnostic, .. } => {
                formatter.write_str(&diagnostic.message)
            }
            Self::State { message } => formatter.write_str(message),
            Self::PathValidation {
                runtime_id,
                path_kind,
                message,
            } => write!(
                formatter,
                "{} {:?} failed allowed-root validation: {}",
                runtime_id.display_name(),
                path_kind,
                message
            ),
            Self::Platform { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ManagedRuntimeCommandResolutionError {}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ManagedRuntimeCommandResolutionError, ManagedRuntimeCommandResolutionError::*};
    use crate::{DeviceResolutionDiagnosticCode, RuntimeVariantId};

    #[test]
    fn command_resolution_error_serializes_missing_runtime_variant_diagnostic() {
        let runtime_variant_id =
            RuntimeVariantId::parse("llama_cpp.cuda").expect("valid runtime variant");
        let error = ManagedRuntimeCommandResolutionError::missing_llamacpp_selected_variant(
            &runtime_variant_id,
            PathBuf::from("/tmp/runtime/cuda/llama-server"),
        );

        let encoded = serde_json::to_value(&error).expect("serialize command error");

        assert_eq!(
            encoded["kind"],
            serde_json::json!("missing_runtime_variant")
        );
        assert_eq!(encoded["requested_device"], serde_json::Value::Null);
        assert_eq!(
            encoded["diagnostic"]["code"],
            serde_json::json!("missing_runtime_variant")
        );
        assert_eq!(
            encoded["diagnostic"]["runtime_variant_id"],
            serde_json::json!("llama_cpp.cuda")
        );

        let decoded: ManagedRuntimeCommandResolutionError =
            serde_json::from_value(encoded).expect("decode command error");
        let MissingRuntimeVariant { diagnostic, .. } = decoded else {
            panic!("unexpected decoded error");
        };

        assert_eq!(
            diagnostic.code,
            DeviceResolutionDiagnosticCode::MissingRuntimeVariant
        );
    }
}
