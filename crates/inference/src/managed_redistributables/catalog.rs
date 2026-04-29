use std::path::Path;

use super::contracts::{
    ManagedRedistributableArchiveKind, ManagedRedistributableCatalogEntry,
    ManagedRedistributableCategory, ManagedRedistributableId, ManagedRedistributablePackageKind,
    ManagedRedistributableSource,
};
use super::paths::{
    archive_kind_for_current_platform, current_platform_key, library_path, tool_path,
};

const FFMPEG_VERSION: &str = "n7.1.1";
const OPEN_COLOR_IO_VERSION: &str = "v2.4.2";
const OPEN_IMAGE_IO_VERSION: &str = "v3.0.3.1";

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

pub(crate) fn validate_catalog_version(
    id: ManagedRedistributableId,
    version: &str,
) -> Result<ManagedRedistributableCatalogEntry, String> {
    let catalog = catalog_entry(id);
    if !catalog_supported(&catalog) {
        return Err(format!(
            "{} is unsupported on {}",
            catalog.display_name, catalog.platform_key
        ));
    }
    if catalog.version != version {
        return Err(format!(
            "{} version {} is not in the managed redistributable catalog",
            catalog.display_name, version
        ));
    }
    Ok(catalog)
}

pub(crate) fn validate_expected_files(
    install_root: &Path,
    catalog: &ManagedRedistributableCatalogEntry,
) -> Result<(), String> {
    let missing_files = missing_expected_files(install_root, &catalog.expected_files);
    if missing_files.is_empty() {
        return Ok(());
    }

    Err(format!(
        "{} {} is missing expected file(s): {}",
        catalog.display_name,
        catalog.version,
        missing_files.join(", ")
    ))
}

pub(crate) fn catalog_supported(catalog: &ManagedRedistributableCatalogEntry) -> bool {
    if catalog.platform_key == "unsupported" {
        return false;
    }
    match catalog.package_kind {
        ManagedRedistributablePackageKind::Archive => catalog.archive_kind.is_some(),
        ManagedRedistributablePackageKind::NativePackage => true,
    }
}

pub(crate) fn missing_expected_files(
    install_root: &Path,
    expected_files: &[String],
) -> Vec<String> {
    expected_files
        .iter()
        .filter(|relative_path| !install_root.join(relative_path).is_file())
        .cloned()
        .collect()
}

pub(crate) fn catalog_entry(id: ManagedRedistributableId) -> ManagedRedistributableCatalogEntry {
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

fn source(owner: &str, project: &str) -> ManagedRedistributableSource {
    ManagedRedistributableSource {
        owner: owner.to_string(),
        project: project.to_string(),
    }
}

#[allow(dead_code)]
fn _assert_archive_kind_is_catalog_contract(_: ManagedRedistributableArchiveKind) {}
