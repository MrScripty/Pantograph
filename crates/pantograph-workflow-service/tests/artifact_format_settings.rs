use pantograph_workflow_service::{
    ArtifactFormatDependencyVersion, ArtifactFormatDependencyVersions, ArtifactFormatSettings,
    ArtifactFormatSettingsQueryRequest, ArtifactFormatSettingsUpdateRequest, WorkflowService,
};

#[test]
fn artifact_format_settings_default_and_capabilities_match_required_defaults() {
    let service = WorkflowService::new();

    let settings = service
        .artifact_format_settings(ArtifactFormatSettingsQueryRequest {})
        .expect("settings")
        .settings;
    assert_eq!(settings.image.format_id, "jpg");
    assert_eq!(settings.image.quality_percent, 75);
    assert_eq!(settings.image.color_profile_id, "srgb");
    assert_eq!(settings.audio.container_id, "ogg");
    assert_eq!(settings.audio.codec_id, "opus");
    assert_eq!(settings.audio.bitrate_kbps, 96);
    assert_eq!(settings.video.codec_id, "svt_av1");
    assert_eq!(settings.video.crf, 32);
    assert_eq!(settings.video.bit_depth, "8bit");
    assert_eq!(settings.three_d.format_id, "glb");

    let capabilities = service.artifact_format_capabilities();
    assert!(capabilities
        .image_formats
        .iter()
        .any(|option| option.format_id == "jpg" && option.provided_by_dependency_id == "oiiotool"));
    assert!(capabilities.audio_formats.iter().any(|option| {
        option.format_id == "ogg" && option.codec_ids.contains(&"opus".to_string())
    }));
    assert!(capabilities
        .video_formats
        .iter()
        .any(|option| option.codec_ids.contains(&"svt_av1".to_string())));
    assert!(capabilities
        .three_d_formats
        .iter()
        .any(|option| option.format_id == "glb"));
}

#[test]
fn artifact_format_capabilities_include_host_supplied_active_versions() {
    let service = WorkflowService::new().with_artifact_format_dependency_versions(
        ArtifactFormatDependencyVersions {
            dependencies: vec![
                ArtifactFormatDependencyVersion {
                    dependency_id: "oiiotool".to_string(),
                    active_version: Some("2.5.18".to_string()),
                },
                ArtifactFormatDependencyVersion {
                    dependency_id: "ffmpeg".to_string(),
                    active_version: Some("7.1".to_string()),
                },
            ],
        },
    );

    let capabilities = service.artifact_format_capabilities();

    assert!(capabilities.image_formats.iter().any(|option| {
        option.provided_by_dependency_id == "oiiotool"
            && option.provided_by_version.as_deref() == Some("2.5.18")
    }));
    assert!(capabilities.audio_formats.iter().any(|option| {
        option.provided_by_dependency_id == "ffmpeg"
            && option.provided_by_version.as_deref() == Some("7.1")
    }));
}

#[test]
fn artifact_format_settings_persist_and_reload() {
    let temp = tempfile::tempdir().expect("temp dir");
    let path = temp.path().join("artifact-format-settings.json");
    let service = WorkflowService::new()
        .with_artifact_format_settings_path(&path)
        .expect("settings path");

    let mut settings = ArtifactFormatSettings::default();
    settings.image.format_id = "png".to_string();
    settings.audio.codec_id = "vorbis".to_string();
    settings.video.bit_depth = "10bit".to_string();
    settings.three_d.format_id = "obj".to_string();

    service
        .update_artifact_format_settings(ArtifactFormatSettingsUpdateRequest {
            settings: settings.clone(),
            reason: Some("test".to_string()),
        })
        .expect("update settings");

    let reloaded = WorkflowService::new()
        .with_artifact_format_settings_path(&path)
        .expect("reload settings")
        .artifact_format_settings(ArtifactFormatSettingsQueryRequest {})
        .expect("settings")
        .settings;
    assert_eq!(reloaded, settings);
}

#[test]
fn artifact_format_settings_reject_invalid_values() {
    let service = WorkflowService::new();
    let mut settings = ArtifactFormatSettings::default();
    settings.image.quality_percent = 0;

    let error = service
        .update_artifact_format_settings(ArtifactFormatSettingsUpdateRequest {
            settings,
            reason: Some("test".to_string()),
        })
        .expect_err("invalid quality rejected");
    assert!(error.to_string().contains("outside allowed range"));

    let mut settings = ArtifactFormatSettings::default();
    settings.audio.codec_id = "raw".to_string();
    let error = service
        .update_artifact_format_settings(ArtifactFormatSettingsUpdateRequest {
            settings,
            reason: Some("test".to_string()),
        })
        .expect_err("invalid codec rejected");
    assert!(error.to_string().contains("unsupported audio codec_id"));
}
