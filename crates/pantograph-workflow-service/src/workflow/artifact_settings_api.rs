use std::path::Path;

use super::{
    ArtifactFormatCapabilities, ArtifactFormatDependencyVersions, ArtifactFormatSettings,
    ArtifactFormatSettingsQueryRequest, ArtifactFormatSettingsQueryResponse,
    ArtifactFormatSettingsUpdateRequest, ArtifactFormatSettingsUpdateResponse, MediaFormatOption,
    WorkflowService, WorkflowServiceError,
};

impl WorkflowService {
    pub fn artifact_format_settings(
        &self,
        _request: ArtifactFormatSettingsQueryRequest,
    ) -> Result<ArtifactFormatSettingsQueryResponse, WorkflowServiceError> {
        let settings = self.artifact_format_settings_guard()?.clone();
        Ok(ArtifactFormatSettingsQueryResponse { settings })
    }

    pub fn update_artifact_format_settings(
        &self,
        request: ArtifactFormatSettingsUpdateRequest,
    ) -> Result<ArtifactFormatSettingsUpdateResponse, WorkflowServiceError> {
        validate_artifact_format_settings(&request.settings)?;
        if let Some(path) = self.artifact_format_settings_path() {
            persist_artifact_format_settings(&path, &request.settings)?;
        }
        *self.artifact_format_settings_guard()? = request.settings.clone();
        Ok(ArtifactFormatSettingsUpdateResponse {
            settings: request.settings,
        })
    }

    pub fn artifact_format_capabilities(&self) -> ArtifactFormatCapabilities {
        artifact_format_capabilities_with_versions(&self.artifact_format_dependency_versions())
    }
}

fn validate_artifact_format_settings(
    settings: &ArtifactFormatSettings,
) -> Result<(), WorkflowServiceError> {
    let capabilities =
        artifact_format_capabilities_with_versions(&ArtifactFormatDependencyVersions::default());
    let image = capabilities
        .image_formats
        .iter()
        .find(|option| option.format_id == settings.image.format_id)
        .ok_or_else(|| invalid_setting("image format", &settings.image.format_id))?;
    validate_u8_range(
        "image quality_percent",
        settings.image.quality_percent,
        image.quality_min_percent,
        image.quality_max_percent,
    )?;
    validate_member(
        "image color_profile_id",
        &settings.image.color_profile_id,
        &image.color_profile_ids,
    )?;

    let audio = capabilities
        .audio_formats
        .iter()
        .find(|option| option.format_id == settings.audio.container_id)
        .ok_or_else(|| invalid_setting("audio container", &settings.audio.container_id))?;
    validate_member("audio codec_id", &settings.audio.codec_id, &audio.codec_ids)?;
    validate_u32_range(
        "audio bitrate_kbps",
        settings.audio.bitrate_kbps,
        audio.bitrate_min_kbps,
        audio.bitrate_max_kbps,
    )?;

    let video = capabilities
        .video_formats
        .iter()
        .find(|option| option.format_id == settings.video.container_id)
        .ok_or_else(|| invalid_setting("video container", &settings.video.container_id))?;
    validate_member("video codec_id", &settings.video.codec_id, &video.codec_ids)?;
    validate_u8_range(
        "video crf",
        settings.video.crf,
        video.crf_min,
        video.crf_max,
    )?;
    validate_member(
        "video bit_depth",
        &settings.video.bit_depth,
        &video.bit_depths,
    )?;

    capabilities
        .three_d_formats
        .iter()
        .find(|option| option.format_id == settings.three_d.format_id)
        .ok_or_else(|| invalid_setting("3d format", &settings.three_d.format_id))?;

    Ok(())
}

fn persist_artifact_format_settings(
    path: &Path,
    settings: &ArtifactFormatSettings,
) -> Result<(), WorkflowServiceError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "failed to create artifact format settings directory {:?}: {error}",
                parent
            ))
        })?;
    }
    let content = serde_json::to_string_pretty(settings).map_err(|error| {
        WorkflowServiceError::Internal(format!(
            "failed to serialize artifact format settings: {error}"
        ))
    })?;
    std::fs::write(path, content).map_err(|error| {
        WorkflowServiceError::Internal(format!(
            "failed to write artifact format settings {:?}: {error}",
            path
        ))
    })
}

fn artifact_format_capabilities() -> ArtifactFormatCapabilities {
    ArtifactFormatCapabilities {
        image_formats: vec![
            image_format("jpg", "JPEG", Some(1), Some(100)),
            image_format("png", "PNG", None, None),
            image_format("tiff", "TIFF", None, None),
            image_format("exr", "OpenEXR", None, None),
        ],
        audio_formats: vec![
            audio_format("ogg", "Ogg", &["opus", "vorbis"], Some(32), Some(512)),
            audio_format("wav", "WAV", &["pcm"], None, None),
            audio_format("mp3", "MP3", &["mp3"], Some(32), Some(320)),
            audio_format("aiff", "AIFF", &["pcm"], None, None),
            audio_format("flac", "FLAC", &["flac"], None, None),
        ],
        video_formats: vec![MediaFormatOption {
            format_id: "ivf".to_string(),
            display_name: "AV1 IVF".to_string(),
            media_type: "video/av1".to_string(),
            codec_ids: vec!["svt_av1".to_string()],
            quality_min_percent: None,
            quality_max_percent: None,
            bitrate_min_kbps: None,
            bitrate_max_kbps: None,
            crf_min: Some(0),
            crf_max: Some(63),
            bit_depths: vec!["8bit".to_string(), "10bit".to_string()],
            color_profile_ids: Vec::new(),
            provided_by_dependency_id: "ffmpeg".to_string(),
            provided_by_version: None,
        }],
        three_d_formats: vec![
            three_d_format("glb", "GLB"),
            three_d_format("gltf", "glTF"),
            three_d_format("obj", "OBJ"),
        ],
    }
}

fn artifact_format_capabilities_with_versions(
    versions: &ArtifactFormatDependencyVersions,
) -> ArtifactFormatCapabilities {
    let mut capabilities = artifact_format_capabilities();
    apply_active_versions(&mut capabilities.image_formats, versions);
    apply_active_versions(&mut capabilities.audio_formats, versions);
    apply_active_versions(&mut capabilities.video_formats, versions);
    apply_active_versions(&mut capabilities.three_d_formats, versions);
    capabilities
}

fn apply_active_versions(
    formats: &mut [MediaFormatOption],
    versions: &ArtifactFormatDependencyVersions,
) {
    for format in formats {
        format.provided_by_version = versions.active_version(&format.provided_by_dependency_id);
    }
}

fn image_format(
    format_id: &str,
    display_name: &str,
    quality_min_percent: Option<u8>,
    quality_max_percent: Option<u8>,
) -> MediaFormatOption {
    MediaFormatOption {
        format_id: format_id.to_string(),
        display_name: display_name.to_string(),
        media_type: format!("image/{format_id}").replace("image/jpg", "image/jpeg"),
        codec_ids: Vec::new(),
        quality_min_percent,
        quality_max_percent,
        bitrate_min_kbps: None,
        bitrate_max_kbps: None,
        crf_min: None,
        crf_max: None,
        bit_depths: vec!["8bit".to_string(), "16bit".to_string(), "float".to_string()],
        color_profile_ids: vec!["srgb".to_string()],
        provided_by_dependency_id: "oiiotool".to_string(),
        provided_by_version: None,
    }
}

fn audio_format(
    format_id: &str,
    display_name: &str,
    codec_ids: &[&str],
    bitrate_min_kbps: Option<u32>,
    bitrate_max_kbps: Option<u32>,
) -> MediaFormatOption {
    MediaFormatOption {
        format_id: format_id.to_string(),
        display_name: display_name.to_string(),
        media_type: match format_id {
            "mp3" => "audio/mpeg".to_string(),
            _ => format!("audio/{format_id}"),
        },
        codec_ids: codec_ids.iter().map(|codec| (*codec).to_string()).collect(),
        quality_min_percent: None,
        quality_max_percent: None,
        bitrate_min_kbps,
        bitrate_max_kbps,
        crf_min: None,
        crf_max: None,
        bit_depths: Vec::new(),
        color_profile_ids: Vec::new(),
        provided_by_dependency_id: "ffmpeg".to_string(),
        provided_by_version: None,
    }
}

fn three_d_format(format_id: &str, display_name: &str) -> MediaFormatOption {
    MediaFormatOption {
        format_id: format_id.to_string(),
        display_name: display_name.to_string(),
        media_type: "model/gltf-binary".to_string(),
        codec_ids: Vec::new(),
        quality_min_percent: None,
        quality_max_percent: None,
        bitrate_min_kbps: None,
        bitrate_max_kbps: None,
        crf_min: None,
        crf_max: None,
        bit_depths: Vec::new(),
        color_profile_ids: Vec::new(),
        provided_by_dependency_id: "pantograph-3d".to_string(),
        provided_by_version: None,
    }
}

fn validate_member(
    label: &str,
    value: &str,
    allowed: &[String],
) -> Result<(), WorkflowServiceError> {
    if allowed.iter().any(|allowed_value| allowed_value == value) {
        Ok(())
    } else {
        Err(invalid_setting(label, value))
    }
}

fn validate_u8_range(
    label: &str,
    value: u8,
    min: Option<u8>,
    max: Option<u8>,
) -> Result<(), WorkflowServiceError> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "{label} {value} is outside allowed range"
        )));
    }
    Ok(())
}

fn validate_u32_range(
    label: &str,
    value: u32,
    min: Option<u32>,
    max: Option<u32>,
) -> Result<(), WorkflowServiceError> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "{label} {value} is outside allowed range"
        )));
    }
    Ok(())
}

fn invalid_setting(label: &str, value: &str) -> WorkflowServiceError {
    WorkflowServiceError::InvalidRequest(format!("unsupported {label}: {value}"))
}
