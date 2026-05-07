use std::fs;

use pantograph_managed_dependencies::{
    ManagedRedistributableId, ManagedRedistributableInstallState, ManagedRedistributableReadiness,
};

use crate::workflow::headless_workflow_commands::{
    artifact_format_dependency_versions_from_statuses,
    workflow_activate_managed_media_dependency_version,
    workflow_install_managed_media_dependency_from_staging,
    workflow_list_managed_media_dependencies, workflow_managed_media_dependency_status,
    workflow_remove_managed_media_dependency_version,
    workflow_select_managed_media_dependency_version,
    workflow_set_default_managed_media_dependency_version,
};

#[test]
fn managed_media_dependency_helpers_project_status_and_actions() {
    let app_data_dir = tempfile::tempdir().expect("temp app data dir");
    let staging_dir = tempfile::tempdir().expect("temp staging dir");

    let initial_statuses =
        workflow_list_managed_media_dependencies(app_data_dir.path()).expect("list statuses");
    assert_eq!(initial_statuses.len(), 4);

    let initial_ffmpeg = workflow_managed_media_dependency_status(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
    )
    .expect("ffmpeg status");
    assert_eq!(
        initial_ffmpeg.install_state,
        ManagedRedistributableInstallState::Missing
    );
    assert_eq!(
        initial_ffmpeg.readiness,
        ManagedRedistributableReadiness::Missing
    );

    for expected_file in &initial_ffmpeg.catalog.expected_files {
        let path = staging_dir.path().join(expected_file);
        fs::create_dir_all(path.parent().expect("expected file parent"))
            .expect("create expected file parent");
        fs::write(path, b"stub executable").expect("write expected file");
    }

    let installed = workflow_install_managed_media_dependency_from_staging(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
        initial_ffmpeg.catalog.version.clone(),
        staging_dir.path(),
    )
    .expect("install staged dependency");
    assert_eq!(
        installed.install_state,
        ManagedRedistributableInstallState::Installed
    );

    let selected = workflow_select_managed_media_dependency_version(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(initial_ffmpeg.catalog.version.clone()),
    )
    .expect("select dependency version");
    assert_eq!(
        selected.selection.selected_version.as_deref(),
        Some(initial_ffmpeg.catalog.version.as_str())
    );

    let defaulted = workflow_set_default_managed_media_dependency_version(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
        Some(initial_ffmpeg.catalog.version.clone()),
    )
    .expect("set default dependency version");
    assert_eq!(
        defaulted.selection.default_version.as_deref(),
        Some(initial_ffmpeg.catalog.version.as_str())
    );

    let activated = workflow_activate_managed_media_dependency_version(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
        initial_ffmpeg.catalog.version.clone(),
    )
    .expect("activate dependency version");
    assert_eq!(
        activated.selection.active_version.as_deref(),
        Some(initial_ffmpeg.catalog.version.as_str())
    );
    let versions =
        artifact_format_dependency_versions_from_statuses(std::slice::from_ref(&activated));
    assert_eq!(versions.dependencies.len(), 1);
    assert_eq!(versions.dependencies[0].dependency_id, "ffmpeg");
    assert_eq!(
        versions.dependencies[0].active_version.as_deref(),
        Some(initial_ffmpeg.catalog.version.as_str())
    );

    let removed = workflow_remove_managed_media_dependency_version(
        app_data_dir.path(),
        ManagedRedistributableId::Ffmpeg,
        initial_ffmpeg.catalog.version,
    )
    .expect("remove dependency version");
    assert_eq!(
        removed.install_state,
        ManagedRedistributableInstallState::Missing
    );
    assert_eq!(removed.selection.active_version, None);
}
