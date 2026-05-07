use inference::{list_managed_dependency_statuses, managed_dependency_status};
use pantograph_managed_dependencies::{
    acquire_managed_redistributable_lease, activate_managed_redistributable_version,
    install_managed_redistributable_from_staging, list_managed_redistributable_statuses,
    load_managed_redistributable_state, managed_redistributable_catalog,
    managed_redistributable_catalog_entry, managed_redistributable_status,
    managed_redistributables_dir, release_managed_redistributable_lease,
    remove_managed_redistributable_version, select_managed_redistributable_version,
    set_default_managed_redistributable_version, ManagedDependencyCategory,
    ManagedDependencyInstallState, ManagedDependencyKey, ManagedDependencyReadinessState,
    ManagedRedistributableCategory, ManagedRedistributableId, ManagedRedistributablePackageKind,
    ManagedRedistributableReadiness, MediaToolDependencyId, NativeArtifactDependencyId,
};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

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
fn neutral_dependency_statuses_preserve_media_dependency_facts() {
    let temp = tempfile::tempdir().unwrap();
    let statuses = list_managed_dependency_statuses(temp.path());

    assert_eq!(statuses.len(), 4);
    assert!(statuses.iter().any(|status| {
        status.key == ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ffmpeg)
            && status.category == ManagedDependencyCategory::MediaTool
            && status.install_state == ManagedDependencyInstallState::Missing
            && status.readiness_state == ManagedDependencyReadinessState::Missing
            && !status.available
    }));
    assert!(statuses.iter().any(|status| {
        status.key == ManagedDependencyKey::NativeArtifact(NativeArtifactDependencyId::OpenColorIo)
            && status.category == ManagedDependencyCategory::NativeArtifact
            && status.install_state == ManagedDependencyInstallState::Missing
            && status.readiness_state == ManagedDependencyReadinessState::Missing
            && !status.available
    }));
}

#[test]
fn neutral_dependency_status_preserves_active_version_projection() {
    let temp = tempfile::tempdir().unwrap();
    let version = install_ready_dependency(temp.path(), ManagedRedistributableId::Ocioconvert);

    activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &version,
    )
    .unwrap();

    let status = managed_dependency_status(temp.path(), ManagedRedistributableId::Ocioconvert);

    assert_eq!(
        status.key,
        ManagedDependencyKey::MediaTool(MediaToolDependencyId::Ocioconvert)
    );
    assert_eq!(status.category, ManagedDependencyCategory::MediaTool);
    assert_eq!(
        status.install_state,
        ManagedDependencyInstallState::Installed
    );
    assert_eq!(
        status.readiness_state,
        ManagedDependencyReadinessState::Ready
    );
    assert!(status.available);
    assert_eq!(
        status.selection.active_version.as_deref(),
        Some(version.as_str())
    );
    assert_eq!(status.versions.len(), 1);
    assert_eq!(
        status.versions[0].version.as_deref(),
        Some(version.as_str())
    );
    assert!(status.versions[0].active);
    assert_eq!(
        status.versions[0].readiness_state,
        ManagedDependencyReadinessState::Ready
    );
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
    assert_eq!(ready.selection.active_version, None);
    assert!(!ready.versions[0].active);
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

#[test]
fn state_round_trips_after_selection_and_activation_restart() {
    let temp = tempfile::tempdir().unwrap();
    let version = install_ready_dependency(temp.path(), ManagedRedistributableId::Ffmpeg);

    select_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(&version),
    )
    .unwrap();
    set_default_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(&version),
    )
    .unwrap();
    activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        &version,
    )
    .unwrap();

    let loaded = load_managed_redistributable_state(temp.path()).unwrap();
    let dependency = loaded
        .dependencies
        .iter()
        .find(|dependency| dependency.id == ManagedRedistributableId::Ffmpeg)
        .unwrap();
    assert_eq!(
        dependency.selection.selected_version.as_deref(),
        Some(version.as_str())
    );
    assert_eq!(
        dependency.selection.default_version.as_deref(),
        Some(version.as_str())
    );
    assert_eq!(
        dependency.selection.active_version.as_deref(),
        Some(version.as_str())
    );

    let restarted_status =
        managed_redistributable_status(temp.path(), ManagedRedistributableId::Ffmpeg);
    assert_eq!(
        restarted_status.selection.active_version.as_deref(),
        Some(version.as_str())
    );
    assert!(restarted_status.versions[0].selected);
    assert!(restarted_status.versions[0].active);
}

#[test]
fn select_and_activate_fail_until_expected_files_are_ready() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = managed_redistributable_catalog_entry(ManagedRedistributableId::Ffmpeg);

    let select_error = select_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(&catalog.version),
    )
    .unwrap_err();
    assert!(select_error.contains("missing expected file"));

    let activate_error = activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        &catalog.version,
    )
    .unwrap_err();
    assert!(activate_error.contains("missing expected file"));

    create_expected_files(
        &version_dir(
            temp.path(),
            ManagedRedistributableId::Ffmpeg,
            &catalog.version,
        ),
        &catalog.expected_files,
    );

    select_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(&catalog.version),
    )
    .unwrap();
    activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ffmpeg,
        &catalog.version,
    )
    .unwrap();
}

#[test]
fn install_from_staging_validates_expected_files_before_finalizing() {
    let temp = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let catalog = managed_redistributable_catalog_entry(ManagedRedistributableId::Ocioconvert);

    let error = install_managed_redistributable_from_staging(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &catalog.version,
        staging.path(),
    )
    .unwrap_err();
    assert!(error.contains("missing expected file"));
    assert!(!canonical_version_dir(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &catalog.version
    )
    .exists());

    create_expected_files(staging.path(), &catalog.expected_files);
    let installed = install_managed_redistributable_from_staging(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &catalog.version,
        staging.path(),
    )
    .unwrap();
    assert_eq!(
        installed,
        canonical_version_dir(
            temp.path(),
            ManagedRedistributableId::Ocioconvert,
            &catalog.version
        )
    );
    assert!(installed.join(&catalog.expected_files[0]).is_file());
}

#[test]
fn remove_active_version_is_blocked_by_lease_then_allowed_after_release() {
    let temp = tempfile::tempdir().unwrap();
    let version = install_ready_dependency(temp.path(), ManagedRedistributableId::Oiiotool);
    activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &version,
    )
    .unwrap();
    let lease = acquire_managed_redistributable_lease(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        "test",
    )
    .unwrap();

    let remove_error = remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &version,
    )
    .unwrap_err();
    assert!(remove_error.contains("while 1 lease"));
    assert!(version_dir(temp.path(), ManagedRedistributableId::Oiiotool, &version).exists());

    release_managed_redistributable_lease(temp.path(), &lease).unwrap();
    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &version,
    )
    .unwrap();

    assert!(!version_dir(temp.path(), ManagedRedistributableId::Oiiotool, &version).exists());
    let status = managed_redistributable_status(temp.path(), ManagedRedistributableId::Oiiotool);
    assert_eq!(status.selection.active_version, None);
}

#[test]
fn unsupported_or_uncataloged_versions_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let error = activate_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::OpenColorIo,
        "not-cataloged",
    )
    .unwrap_err();
    assert!(error.contains("not in the managed redistributable catalog"));
}

fn install_ready_dependency(app_data_dir: &Path, id: ManagedRedistributableId) -> String {
    let catalog = managed_redistributable_catalog_entry(id);
    create_expected_files(
        &version_dir(app_data_dir, id, &catalog.version),
        &catalog.expected_files,
    );
    catalog.version
}

fn create_expected_files(root: &Path, expected_files: &[String]) {
    for expected_file in expected_files {
        let file_path = root.join(expected_file);
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(file_path, []).unwrap();
    }
}

fn version_dir(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> std::path::PathBuf {
    app_data_dir
        .join("managed-dependencies")
        .join(id.key())
        .join("versions")
        .join(version)
}

fn canonical_version_dir(
    app_data_dir: &Path,
    id: ManagedRedistributableId,
    version: &str,
) -> std::path::PathBuf {
    managed_redistributables_dir(app_data_dir)
        .join(id.key())
        .join("versions")
        .join(version)
}

fn executable_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}
