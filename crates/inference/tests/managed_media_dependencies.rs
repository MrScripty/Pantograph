use std::fs;
use std::path::Path;

use inference::{
    acquire_media_conversion_dependency_plan, activate_managed_redistributable_version,
    managed_redistributable_catalog_entry, open_color_io_activation_validation_state,
    release_media_conversion_dependency_plan, remove_managed_redistributable_version,
    validate_open_color_io_activation, ManagedRedistributableId, MediaConversionDependencyId,
    MediaConversionDependencyPlanRequest, MediaConversionJobKind,
    OpenColorIoActivationValidationState,
};

#[test]
fn color_managed_image_plan_acquires_expected_managed_dependency_leases() {
    let temp = tempfile::tempdir().unwrap();
    let oiiotool_version = install_and_activate(temp.path(), ManagedRedistributableId::Oiiotool);
    let ocioconvert_version =
        install_and_activate(temp.path(), ManagedRedistributableId::Ocioconvert);
    let ocio_version = install_and_activate(temp.path(), ManagedRedistributableId::OpenColorIo);

    let plan = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Image,
            color_managed: true,
            holder: "image-test".to_string(),
        },
    )
    .unwrap();

    let ids = plan
        .leases
        .iter()
        .map(|lease| lease.dependency.id)
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            MediaConversionDependencyId::Oiiotool,
            MediaConversionDependencyId::Ocioconvert,
            MediaConversionDependencyId::OpenColorIo,
        ]
    );
    assert_eq!(
        plan.open_color_io_activation
            .as_ref()
            .unwrap()
            .abi_validation
            .state,
        OpenColorIoActivationValidationState::NotValidated
    );

    assert_remove_blocked(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &oiiotool_version,
    );
    assert_remove_blocked(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &ocioconvert_version,
    );
    assert_remove_blocked(
        temp.path(),
        ManagedRedistributableId::OpenColorIo,
        &ocio_version,
    );

    release_media_conversion_dependency_plan(temp.path(), &plan).unwrap();

    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &oiiotool_version,
    )
    .unwrap();
    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &ocioconvert_version,
    )
    .unwrap();
    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::OpenColorIo,
        &ocio_version,
    )
    .unwrap();
}

#[test]
fn audio_and_video_plans_use_ffmpeg_only() {
    let temp = tempfile::tempdir().unwrap();
    install_and_activate(temp.path(), ManagedRedistributableId::Ffmpeg);

    for job_kind in [MediaConversionJobKind::Audio, MediaConversionJobKind::Video] {
        let plan = acquire_media_conversion_dependency_plan(
            temp.path(),
            MediaConversionDependencyPlanRequest {
                job_kind,
                color_managed: false,
                holder: format!("{job_kind:?}-test"),
            },
        )
        .unwrap();

        assert_eq!(plan.leases.len(), 1);
        assert_eq!(
            plan.leases[0].dependency.id,
            MediaConversionDependencyId::Ffmpeg
        );
        assert_eq!(plan.open_color_io_activation, None);

        release_media_conversion_dependency_plan(temp.path(), &plan).unwrap();
    }
}

#[test]
fn three_d_plan_uses_oiiotool_without_color_management() {
    let temp = tempfile::tempdir().unwrap();
    install_and_activate(temp.path(), ManagedRedistributableId::Oiiotool);

    let plan = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::ThreeD,
            color_managed: false,
            holder: "three-d-test".to_string(),
        },
    )
    .unwrap();

    assert_eq!(plan.leases.len(), 1);
    assert_eq!(
        plan.leases[0].dependency.id,
        MediaConversionDependencyId::Oiiotool
    );
    assert_eq!(plan.open_color_io_activation, None);

    release_media_conversion_dependency_plan(temp.path(), &plan).unwrap();
}

#[test]
fn missing_or_inactive_dependencies_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let error = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Video,
            color_managed: false,
            holder: "inactive-test".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.contains("does not have an active managed dependency version"));

    let ffmpeg_version = install_and_activate(temp.path(), ManagedRedistributableId::Ffmpeg);
    let ffmpeg = managed_redistributable_catalog_entry(ManagedRedistributableId::Ffmpeg);
    fs::remove_file(
        version_dir(
            temp.path(),
            ManagedRedistributableId::Ffmpeg,
            &ffmpeg_version,
        )
        .join(&ffmpeg.expected_files[0]),
    )
    .unwrap();

    let error = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Video,
            color_managed: false,
            holder: "active-but-unready-test".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.contains("is not ready"));
    assert!(error.contains("missing expected file"));
}

#[test]
fn failed_color_managed_plan_releases_partially_acquired_leases() {
    let temp = tempfile::tempdir().unwrap();
    let oiiotool_version = install_and_activate(temp.path(), ManagedRedistributableId::Oiiotool);
    let ocioconvert_version =
        install_and_activate(temp.path(), ManagedRedistributableId::Ocioconvert);

    let error = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Image,
            color_managed: true,
            holder: "rollback-test".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.contains("OpenColorIO"));

    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Oiiotool,
        &oiiotool_version,
    )
    .unwrap();
    remove_managed_redistributable_version(
        temp.path(),
        ManagedRedistributableId::Ocioconvert,
        &ocioconvert_version,
    )
    .unwrap();
}

#[test]
fn ready_but_inactive_dependency_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let ffmpeg = managed_redistributable_catalog_entry(ManagedRedistributableId::Ffmpeg);
    create_expected_files(
        &version_dir(
            temp.path(),
            ManagedRedistributableId::Ffmpeg,
            &ffmpeg.version,
        ),
        &ffmpeg.expected_files,
    );

    let error = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Video,
            color_managed: false,
            holder: "ready-but-inactive-test".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.contains("does not have an active managed dependency version"));
}

#[test]
fn open_color_io_activation_validation_states_are_explicit() {
    let temp = tempfile::tempdir().unwrap();

    let unavailable = open_color_io_activation_validation_state(temp.path());
    assert_eq!(
        unavailable.state,
        OpenColorIoActivationValidationState::Unavailable
    );
    assert!(unavailable
        .reason
        .contains("does not have an active managed dependency version"));
    assert!(validate_open_color_io_activation(temp.path()).is_err());

    install_and_activate(temp.path(), ManagedRedistributableId::OpenColorIo);

    let activation = validate_open_color_io_activation(temp.path()).unwrap();
    assert_eq!(
        activation.abi_validation.state,
        OpenColorIoActivationValidationState::NotValidated
    );
    assert!(activation.abi_validation.reason.contains("ABI validation"));

    let state = open_color_io_activation_validation_state(temp.path());
    assert_eq!(state, activation.abi_validation);
}

fn install_and_activate(app_data_dir: &Path, id: ManagedRedistributableId) -> String {
    let catalog = managed_redistributable_catalog_entry(id);
    create_expected_files(
        &version_dir(app_data_dir, id, &catalog.version),
        &catalog.expected_files,
    );
    activate_managed_redistributable_version(app_data_dir, id, &catalog.version).unwrap();
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

fn assert_remove_blocked(app_data_dir: &Path, id: ManagedRedistributableId, version: &str) {
    let error = remove_managed_redistributable_version(app_data_dir, id, version).unwrap_err();
    assert!(error.contains("lease"));
}
