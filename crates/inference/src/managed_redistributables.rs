use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FFMPEG_VERSION: &str = "n7.1.1";
const OPEN_COLOR_IO_VERSION: &str = "v2.4.2";
const OPEN_IMAGE_IO_VERSION: &str = "v3.0.3.1";

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

pub fn managed_redistributables_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("managed-dependencies")
}

pub fn managed_redistributable_catalog() -> Vec<ManagedRedistributableCatalogEntry> {
    ManagedRedistributableId::all()
        .iter()
        .copied()
        .map(catalog_entry)
        .collect()
}

pub fn managed_redistributable_catalog_entry(
    id: ManagedRedistributableId,
) -> ManagedRedistributableCatalogEntry {
    catalog_entry(id)
}

pub fn managed_redistributable_status(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
) -> ManagedRedistributableStatus {
    let catalog = catalog_entry(id);
    status_from_catalog(app_data_dir, catalog)
}

pub fn list_managed_redistributable_statuses(
    app_data_dir: &Path,
) -> Vec<ManagedRedistributableStatus> {
    managed_redistributable_catalog()
        .into_iter()
        .map(|catalog| status_from_catalog(app_data_dir, catalog))
        .collect()
}

fn status_from_catalog(
    app_data_dir: &Path,
    catalog: ManagedRedistributableCatalogEntry,
) -> ManagedRedistributableStatus {
    let install_root =
        managed_redistributable_version_dir(app_data_dir, catalog.id, &catalog.version);
    let missing_files = missing_expected_files(&install_root, &catalog.expected_files);
    let ready = missing_files.is_empty();
    let install_state = if ready {
        ManagedRedistributableInstallState::Installed
    } else {
        ManagedRedistributableInstallState::Missing
    };
    let readiness = if ready {
        ManagedRedistributableReadiness::Ready
    } else {
        ManagedRedistributableReadiness::Missing
    };
    let active_version = ready.then(|| catalog.version.clone());
    let selection = ManagedRedistributableSelection {
        selected_version: Some(catalog.version.clone()),
        active_version,
        default_version: Some(catalog.version.clone()),
    };

    ManagedRedistributableStatus {
        id: catalog.id,
        display_name: catalog.display_name.clone(),
        category: catalog.category,
        install_state,
        readiness,
        available: ready,
        missing_files: missing_files.clone(),
        versions: vec![ManagedRedistributableVersionStatus {
            version: catalog.version.clone(),
            platform_key: catalog.platform_key.clone(),
            install_root: install_root.display().to_string(),
            expected_files: catalog.expected_files.clone(),
            missing_files,
            install_state,
            readiness,
            selected: true,
            active: ready,
        }],
        catalog,
        selection,
    }
}

fn catalog_entry(id: ManagedRedistributableId) -> ManagedRedistributableCatalogEntry {
    let platform_key = current_platform_key().to_string();
    match id {
        ManagedRedistributableId::Ffmpeg => ManagedRedistributableCatalogEntry {
            id,
            display_name: id.display_name().to_string(),
            category: ManagedRedistributableCategory::ToolBinary,
            source: source("FFmpeg", "FFmpeg"),
            license_redistribution:
                "LGPL-2.1-or-later/GPL-2.0-or-later depending on enabled codecs".to_string(),
            platform_key,
            version: FFMPEG_VERSION.to_string(),
            package_kind: ManagedRedistributablePackageKind::Archive,
            archive_kind: archive_kind_for_current_platform(),
            archive_name: None,
            download_url: None,
            expected_files: vec![tool_path("ffmpeg")],
            checksum_sha256: None,
            signature: None,
        },
        ManagedRedistributableId::Ocioconvert => ManagedRedistributableCatalogEntry {
            id,
            display_name: id.display_name().to_string(),
            category: ManagedRedistributableCategory::ToolBinary,
            source: source("AcademySoftwareFoundation", "OpenColorIO"),
            license_redistribution: "BSD-3-Clause".to_string(),
            platform_key,
            version: OPEN_COLOR_IO_VERSION.to_string(),
            package_kind: ManagedRedistributablePackageKind::Archive,
            archive_kind: archive_kind_for_current_platform(),
            archive_name: None,
            download_url: None,
            expected_files: vec![tool_path("ocioconvert")],
            checksum_sha256: None,
            signature: None,
        },
        ManagedRedistributableId::Oiiotool => ManagedRedistributableCatalogEntry {
            id,
            display_name: id.display_name().to_string(),
            category: ManagedRedistributableCategory::ToolBinary,
            source: source("AcademySoftwareFoundation", "OpenImageIO"),
            license_redistribution: "BSD-3-Clause".to_string(),
            platform_key,
            version: OPEN_IMAGE_IO_VERSION.to_string(),
            package_kind: ManagedRedistributablePackageKind::Archive,
            archive_kind: archive_kind_for_current_platform(),
            archive_name: None,
            download_url: None,
            expected_files: vec![tool_path("oiiotool")],
            checksum_sha256: None,
            signature: None,
        },
        ManagedRedistributableId::OpenColorIo => ManagedRedistributableCatalogEntry {
            id,
            display_name: id.display_name().to_string(),
            category: ManagedRedistributableCategory::NativeLibraryArtifact,
            source: source("AcademySoftwareFoundation", "OpenColorIO"),
            license_redistribution: "BSD-3-Clause".to_string(),
            platform_key,
            version: OPEN_COLOR_IO_VERSION.to_string(),
            package_kind: ManagedRedistributablePackageKind::NativePackage,
            archive_kind: archive_kind_for_current_platform(),
            archive_name: None,
            download_url: None,
            expected_files: vec![library_path("OpenColorIO")],
            checksum_sha256: None,
            signature: None,
        },
    }
}

fn managed_redistributable_version_dir(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> PathBuf {
    managed_redistributables_dir(app_data_dir)
        .join(id.key())
        .join("versions")
        .join(version)
}

fn missing_expected_files(install_root: &Path, expected_files: &[String]) -> Vec<String> {
    expected_files
        .iter()
        .filter(|relative_path| !install_root.join(relative_path).is_file())
        .cloned()
        .collect()
}

fn source(owner: &str, project: &str) -> ManagedRedistributableSource {
    ManagedRedistributableSource {
        owner: owner.to_string(),
        project: project.to_string(),
    }
}

fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "macos-x64"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "macos-arm64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        "unsupported"
    }
}

fn archive_kind_for_current_platform() -> Option<ManagedRedistributableArchiveKind> {
    #[cfg(target_os = "windows")]
    {
        Some(ManagedRedistributableArchiveKind::Zip)
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Some(ManagedRedistributableArchiveKind::TarGz)
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

fn tool_path(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("bin/{name}.exe")
    } else {
        format!("bin/{name}")
    }
}

fn library_path(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("bin/{name}.dll")
    } else if cfg!(target_os = "macos") {
        format!("lib/lib{name}.dylib")
    } else {
        format!("lib/lib{name}.so")
    }
}
