use std::fs;
use std::path::Path;

use inference::managed_media_dependencies::{
    format_media_conversion_dependency_lease_holder,
    media_conversion_dependency_lease_holder_convention,
    validate_media_conversion_dependency_lease_holder,
};
use inference::{
    acquire_media_conversion_dependency_plan, activate_managed_redistributable_version,
    load_managed_redistributable_state, managed_redistributable_catalog_entry,
    open_color_io_activation_validation_state, release_media_conversion_dependency_plan,
    remove_managed_redistributable_version, validate_open_color_io_activation,
    ManagedRedistributableId, MediaConversionDependencyId, MediaConversionDependencyLease,
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
    let holder = test_holder("image-color-managed");

    let plan = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Image,
            color_managed: true,
            holder: holder.clone(),
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
    assert_lease_attribution(
        temp.path(),
        &plan.leases[0],
        MediaConversionDependencyId::Oiiotool,
        ManagedRedistributableId::Oiiotool,
        &oiiotool_version,
        &holder,
    );
    assert_lease_attribution(
        temp.path(),
        &plan.leases[1],
        MediaConversionDependencyId::Ocioconvert,
        ManagedRedistributableId::Ocioconvert,
        &ocioconvert_version,
        &holder,
    );
    assert_lease_attribution(
        temp.path(),
        &plan.leases[2],
        MediaConversionDependencyId::OpenColorIo,
        ManagedRedistributableId::OpenColorIo,
        &ocio_version,
        &holder,
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
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Oiiotool, 1);
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Ocioconvert, 1);
    assert_active_lease_count(temp.path(), ManagedRedistributableId::OpenColorIo, 1);

    release_media_conversion_dependency_plan(temp.path(), &plan).unwrap();
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Oiiotool, 0);
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Ocioconvert, 0);
    assert_active_lease_count(temp.path(), ManagedRedistributableId::OpenColorIo, 0);

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
                holder: test_holder(&format!("{job_kind:?}-test")),
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
            holder: test_holder("three-d-test"),
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
            holder: test_holder("inactive-test"),
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
            holder: test_holder("active-but-unready-test"),
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
            holder: test_holder("rollback-test"),
        },
    )
    .unwrap_err();
    assert!(error.contains("OpenColorIO"));
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Oiiotool, 0);
    assert_active_lease_count(temp.path(), ManagedRedistributableId::Ocioconvert, 0);

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
            holder: test_holder("ready-but-inactive-test"),
        },
    )
    .unwrap_err();
    assert!(error.contains("does not have an active managed dependency version"));
}

#[test]
fn dependency_plan_requires_attribution_holder_convention() {
    let temp = tempfile::tempdir().unwrap();
    install_and_activate(temp.path(), ManagedRedistributableId::Ffmpeg);

    let error = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Video,
            color_managed: false,
            holder: "video-test".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.contains(media_conversion_dependency_lease_holder_convention()));

    let error = format_media_conversion_dependency_lease_holder(
        "workflow.run",
        "node-1",
        "port/in",
        "conversion-1",
    )
    .unwrap_err();
    assert!(error.contains("port_id"));

    let holder = format_media_conversion_dependency_lease_holder(
        "workflow.run",
        "node-1",
        "port_in",
        "conversion-1",
    )
    .unwrap();
    validate_media_conversion_dependency_lease_holder(&holder).unwrap();
    let plan = acquire_media_conversion_dependency_plan(
        temp.path(),
        MediaConversionDependencyPlanRequest {
            job_kind: MediaConversionJobKind::Video,
            color_managed: false,
            holder: holder.clone(),
        },
    )
    .unwrap();

    assert_eq!(plan.leases[0].token.holder, holder);

    release_media_conversion_dependency_plan(temp.path(), &plan).unwrap();
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

fn test_holder(conversion_id: &str) -> String {
    format_media_conversion_dependency_lease_holder(
        "workflow-run-test",
        "node-test",
        "port-test",
        conversion_id,
    )
    .unwrap()
}

fn assert_lease_attribution(
    app_data_dir: &Path,
    lease: &MediaConversionDependencyLease,
    dependency_id: MediaConversionDependencyId,
    redistributable_id: ManagedRedistributableId,
    version: &str,
    holder: &str,
) {
    let catalog = managed_redistributable_catalog_entry(redistributable_id);

    assert_eq!(lease.dependency.id, dependency_id);
    assert_eq!(lease.dependency.version, version);
    assert_eq!(
        lease.dependency.install_root,
        version_dir(app_data_dir, redistributable_id, version)
            .display()
            .to_string()
    );
    assert_eq!(lease.dependency.expected_files, catalog.expected_files);
    assert_eq!(lease.token.id, dependency_id);
    assert_eq!(lease.token.version, version);
    assert!(!lease.token.lease_id.is_empty());
    assert_eq!(lease.token.holder, holder);
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

fn assert_active_lease_count(app_data_dir: &Path, id: ManagedRedistributableId, expected: usize) {
    let state = load_managed_redistributable_state(app_data_dir).unwrap();
    let active_lease_count = state
        .dependencies
        .iter()
        .find(|dependency| dependency.id == id)
        .map(|dependency| dependency.active_leases.len())
        .unwrap_or_default();
    assert_eq!(active_lease_count, expected);
}
