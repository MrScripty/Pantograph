use inference::{
    list_managed_redistributable_statuses, managed_redistributable_catalog,
    managed_redistributable_status, ManagedRedistributableCategory, ManagedRedistributableId,
    ManagedRedistributablePackageKind, ManagedRedistributableReadiness,
};
use std::collections::HashSet;
use std::fs;

#[test]
fn catalog_contains_all_required_dependency_ids() {
    let ids = managed_redistributable_catalog()
        .into_iter()
        .map(|entry| entry.id)
        .collect::<HashSet<_>>();

    assert_eq!(
        ids,
        HashSet::from([
            ManagedRedistributableId::Ffmpeg,
            ManagedRedistributableId::Ocioconvert,
            ManagedRedistributableId::Oiiotool,
            ManagedRedistributableId::OpenColorIo,
        ])
    );
}

#[test]
fn categories_match_tools_and_native_library() {
    let statuses = list_managed_redistributable_statuses(tempfile::tempdir().unwrap().path());

    for status in statuses {
        let expected = match status.id {
            ManagedRedistributableId::Ffmpeg
            | ManagedRedistributableId::Ocioconvert
            | ManagedRedistributableId::Oiiotool => ManagedRedistributableCategory::ToolBinary,
            ManagedRedistributableId::OpenColorIo => {
                ManagedRedistributableCategory::NativeLibraryArtifact
            }
        };
        assert_eq!(status.category, expected);
    }
}

#[test]
fn readiness_does_not_probe_unmanaged_path() {
    let temp = tempfile::tempdir().unwrap();
    let path_bin = temp.path().join("path-bin");
    fs::create_dir_all(&path_bin).unwrap();
    fs::write(path_bin.join(executable_name("ffmpeg")), []).unwrap();

    let original_path = std::env::var_os("PATH");
    std::env::set_var("PATH", path_bin.as_os_str());
    let status = managed_redistributable_status(temp.path(), ManagedRedistributableId::Ffmpeg);
    if let Some(original_path) = original_path {
        std::env::set_var("PATH", original_path);
    } else {
        std::env::remove_var("PATH");
    }

    assert_eq!(status.readiness, ManagedRedistributableReadiness::Missing);
    assert!(!status.available);
    assert_eq!(status.selection.active_version, None);
}

#[test]
fn expected_files_drive_readiness() {
    let temp = tempfile::tempdir().unwrap();
    let missing = managed_redistributable_status(temp.path(), ManagedRedistributableId::Oiiotool);

    assert_eq!(missing.readiness, ManagedRedistributableReadiness::Missing);
    assert_eq!(missing.versions.len(), 1);
    assert_eq!(
        missing.versions[0].missing_files,
        missing.catalog.expected_files
    );

    let install_root = temp
        .path()
        .join("managed-dependencies")
        .join(ManagedRedistributableId::Oiiotool.key())
        .join("versions")
        .join(&missing.catalog.version);
    for expected_file in &missing.catalog.expected_files {
        let file_path = install_root.join(expected_file);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(file_path, []).unwrap();
    }

    let ready = managed_redistributable_status(temp.path(), ManagedRedistributableId::Oiiotool);
    assert_eq!(ready.readiness, ManagedRedistributableReadiness::Ready);
    assert!(ready.available);
    assert_eq!(ready.versions[0].missing_files, Vec::<String>::new());
    assert_eq!(
        ready.selection.active_version.as_deref(),
        Some(ready.catalog.version.as_str())
    );
}

#[test]
fn catalog_metadata_includes_source_license_platform_and_integrity_placeholders() {
    for entry in managed_redistributable_catalog() {
        assert!(!entry.display_name.trim().is_empty());
        assert!(!entry.source.owner.trim().is_empty());
        assert!(!entry.source.project.trim().is_empty());
        assert!(!entry.license_redistribution.trim().is_empty());
        assert!(!entry.platform_key.trim().is_empty());
        assert!(!entry.version.trim().is_empty());
        assert!(!entry.expected_files.is_empty());
        assert!(matches!(
            entry.package_kind,
            ManagedRedistributablePackageKind::Archive
                | ManagedRedistributablePackageKind::NativePackage
        ));

        assert_eq!(entry.checksum_sha256, None);
        assert_eq!(entry.signature, None);
    }
}

fn executable_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
