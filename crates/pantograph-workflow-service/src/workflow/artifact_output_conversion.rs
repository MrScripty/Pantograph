use super::{
    ArtifactAttribution, ArtifactFormatCapabilities, ArtifactFormatMetadata,
    ArtifactFormatSettings, ArtifactPayloadKind, ArtifactWriteRequest, AudioArtifactFormatSettings,
    ImageArtifactFormatSettings, WorkflowPortBinding, WorkflowService, WorkflowServiceError,
};
use crate::graph::WorkflowGraphRunSettings;

const IMAGE_PORT_ID: &str = "image";
const AUDIO_PORT_ID: &str = "audio";
const DEFAULT_IMAGE_MEDIA_TYPE: &str = "image/jpeg";
const DEFAULT_AUDIO_MEDIA_TYPE: &str = "audio/ogg";
const DEFAULT_VIDEO_MEDIA_TYPE: &str = "video/mp4";
const DEFAULT_THREE_D_MEDIA_TYPE: &str = "model/gltf-binary";
const DEFAULT_TABLE_MEDIA_TYPE: &str = "text/csv";
const DEFAULT_BINARY_MEDIA_TYPE: &str = "application/octet-stream";
const DEFAULT_STRUCTURED_MEDIA_TYPE: &str = "application/json";

pub(super) fn convert_media_outputs_to_artifacts(
    service: &WorkflowService,
    workflow_id: &str,
    workflow_version_id: &str,
    workflow_run_id: &str,
    graph_run_settings: Option<&WorkflowGraphRunSettings>,
    outputs: Vec<WorkflowPortBinding>,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
    let format_settings = service.artifact_format_settings_guard()?.clone();
    let format_capabilities = service.artifact_format_capabilities();
    outputs
        .into_iter()
        .map(|binding| {
            let Some(artifact_output) = ArtifactOutput::from_binding(&binding) else {
                return Ok(binding);
            };
            convert_artifact_output(
                service,
                workflow_id,
                workflow_version_id,
                workflow_run_id,
                graph_run_settings,
                &format_settings,
                &format_capabilities,
                binding,
                artifact_output,
            )
        })
        .collect()
}

fn convert_artifact_output(
    service: &WorkflowService,
    workflow_id: &str,
    workflow_version_id: &str,
    workflow_run_id: &str,
    graph_run_settings: Option<&WorkflowGraphRunSettings>,
    format_settings: &ArtifactFormatSettings,
    format_capabilities: &ArtifactFormatCapabilities,
    binding: WorkflowPortBinding,
    artifact_output: ArtifactOutput,
) -> Result<WorkflowPortBinding, WorkflowServiceError> {
    let body = decode_base64(&artifact_output.encoded_body).map_err(|reason| {
        WorkflowServiceError::InvalidRequest(format!(
            "binding '{}.{}' contains invalid base64 artifact output: {}",
            binding.node_id, binding.port_id, reason
        ))
    })?;
    let media_type =
        authoritative_media_type(artifact_output.payload_kind, &artifact_output, &body);
    let output_format = resolve_output_format_metadata(
        &binding,
        artifact_output.payload_kind,
        media_type.as_deref(),
        graph_run_settings,
        format_settings,
        format_capabilities,
    )?;
    let artifact_id = format!(
        "run_{}_{}_{}",
        sanitize_artifact_id(workflow_run_id),
        sanitize_artifact_id(&binding.node_id),
        sanitize_artifact_id(&binding.port_id)
    );
    let descriptor = service.write_artifact(ArtifactWriteRequest {
        artifact_id: Some(artifact_id),
        payload_kind: artifact_output.payload_kind,
        media_type: output_format.media_type.clone(),
        format: Some(output_format.metadata),
        attribution: ArtifactAttribution {
            workflow_run_id: workflow_run_id.to_string(),
            workflow_id: Some(workflow_id.to_string()),
            workflow_version_id: Some(workflow_version_id.to_string()),
            node_id: Some(binding.node_id.clone()),
            port_id: Some(binding.port_id.clone()),
            model_id: None,
            runtime_id: None,
        },
        body,
    })?;
    Ok(WorkflowPortBinding {
        value: serde_json::to_value(descriptor).map_err(|error| {
            WorkflowServiceError::Internal(format!(
                "failed to serialize artifact descriptor: {error}"
            ))
        })?,
        ..binding
    })
}

struct ResolvedOutputFormat {
    media_type: String,
    metadata: ArtifactFormatMetadata,
}

fn resolve_output_format_metadata(
    binding: &WorkflowPortBinding,
    payload_kind: ArtifactPayloadKind,
    authoritative_media_type: Option<&str>,
    graph_run_settings: Option<&WorkflowGraphRunSettings>,
    settings: &ArtifactFormatSettings,
    capabilities: &ArtifactFormatCapabilities,
) -> Result<ResolvedOutputFormat, WorkflowServiceError> {
    match payload_kind {
        ArtifactPayloadKind::Image => resolve_image_output_format(
            binding,
            authoritative_media_type,
            graph_run_settings,
            &settings.image,
            capabilities,
        ),
        ArtifactPayloadKind::Audio => resolve_audio_output_format(
            binding,
            authoritative_media_type,
            graph_run_settings,
            &settings.audio,
            capabilities,
        ),
        _ => {
            let media_type = authoritative_media_type
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| infer_media_type(payload_kind, &[]));
            Ok(ResolvedOutputFormat {
                metadata: format_metadata(payload_kind, &media_type),
                media_type,
            })
        }
    }
}

fn resolve_image_output_format(
    binding: &WorkflowPortBinding,
    authoritative_media_type: Option<&str>,
    graph_run_settings: Option<&WorkflowGraphRunSettings>,
    settings: &ImageArtifactFormatSettings,
    capabilities: &ArtifactFormatCapabilities,
) -> Result<ResolvedOutputFormat, WorkflowServiceError> {
    let override_object = artifact_format_override_object(binding, graph_run_settings)?;
    let selection = ImageOutputFormatSelection {
        format_id: read_string_override(
            binding,
            override_object,
            "format_id",
            &settings.format_id,
        )?,
        quality_percent: read_u8_override(
            binding,
            override_object,
            "quality_percent",
            settings.quality_percent,
        )?,
        color_profile_id: read_string_override(
            binding,
            override_object,
            "color_profile_id",
            &settings.color_profile_id,
        )?,
        is_override: override_object.is_some(),
    };
    let selected_format = capabilities
        .image_formats
        .iter()
        .find(|option| option.format_id == selection.format_id)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "unsupported image artifact_format_override format_id '{}' for binding '{}.{}'",
                selection.format_id, binding.node_id, binding.port_id
            ))
        })?;
    validate_u8_range(
        binding,
        "quality_percent",
        selection.quality_percent,
        selected_format.quality_min_percent,
        selected_format.quality_max_percent,
    )?;
    validate_member(
        binding,
        "color_profile_id",
        &selection.color_profile_id,
        &selected_format.color_profile_ids,
    )?;

    let actual_media_type = authoritative_media_type.unwrap_or(&selected_format.media_type);
    if selection.is_override && actual_media_type != selected_format.media_type {
        return Err(transcode_required_error(
            binding,
            actual_media_type,
            &selected_format.media_type,
        ));
    }
    let format = capabilities
        .image_formats
        .iter()
        .find(|option| option.media_type == actual_media_type)
        .unwrap_or(selected_format);

    Ok(ResolvedOutputFormat {
        media_type: actual_media_type.to_string(),
        metadata: ArtifactFormatMetadata {
            format_id: format.format_id.clone(),
            media_type: actual_media_type.to_string(),
            codec_id: None,
            quality_percent: Some(selection.quality_percent),
            bitrate_kbps: None,
            crf: None,
            bit_depth: Some("8bit".to_string()),
            color_profile_id: Some(selection.color_profile_id),
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
    })
}

fn resolve_audio_output_format(
    binding: &WorkflowPortBinding,
    authoritative_media_type: Option<&str>,
    graph_run_settings: Option<&WorkflowGraphRunSettings>,
    settings: &AudioArtifactFormatSettings,
    capabilities: &ArtifactFormatCapabilities,
) -> Result<ResolvedOutputFormat, WorkflowServiceError> {
    let override_object = artifact_format_override_object(binding, graph_run_settings)?;
    let selection = AudioOutputFormatSelection {
        container_id: read_string_override(
            binding,
            override_object,
            "container_id",
            &settings.container_id,
        )?,
        codec_id: read_string_override(binding, override_object, "codec_id", &settings.codec_id)?,
        bitrate_kbps: read_u32_override(
            binding,
            override_object,
            "bitrate_kbps",
            settings.bitrate_kbps,
        )?,
        is_override: override_object.is_some(),
    };
    let selected_format = capabilities
        .audio_formats
        .iter()
        .find(|option| option.format_id == selection.container_id)
        .ok_or_else(|| {
            WorkflowServiceError::InvalidRequest(format!(
                "unsupported audio artifact_format_override container_id '{}' for binding '{}.{}'",
                selection.container_id, binding.node_id, binding.port_id
            ))
        })?;
    validate_member(
        binding,
        "codec_id",
        &selection.codec_id,
        &selected_format.codec_ids,
    )?;
    validate_u32_range(
        binding,
        "bitrate_kbps",
        selection.bitrate_kbps,
        selected_format.bitrate_min_kbps,
        selected_format.bitrate_max_kbps,
    )?;

    let actual_media_type = authoritative_media_type.unwrap_or(&selected_format.media_type);
    if selection.is_override && actual_media_type != selected_format.media_type {
        return Err(transcode_required_error(
            binding,
            actual_media_type,
            &selected_format.media_type,
        ));
    }
    let format = capabilities
        .audio_formats
        .iter()
        .find(|option| option.media_type == actual_media_type)
        .unwrap_or(selected_format);
    let codec_id = if format
        .codec_ids
        .iter()
        .any(|codec| codec == &selection.codec_id)
    {
        selection.codec_id
    } else {
        format
            .codec_ids
            .first()
            .cloned()
            .unwrap_or_else(|| selection.codec_id)
    };

    Ok(ResolvedOutputFormat {
        media_type: actual_media_type.to_string(),
        metadata: ArtifactFormatMetadata {
            format_id: format.format_id.clone(),
            media_type: actual_media_type.to_string(),
            codec_id: Some(codec_id),
            quality_percent: None,
            bitrate_kbps: Some(selection.bitrate_kbps),
            crf: None,
            bit_depth: None,
            color_profile_id: None,
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
    })
}

struct ImageOutputFormatSelection {
    format_id: String,
    quality_percent: u8,
    color_profile_id: String,
    is_override: bool,
}

struct AudioOutputFormatSelection {
    container_id: String,
    codec_id: String,
    bitrate_kbps: u32,
    is_override: bool,
}

fn artifact_format_override_object<'a>(
    binding: &WorkflowPortBinding,
    graph_run_settings: Option<&'a WorkflowGraphRunSettings>,
) -> Result<Option<&'a serde_json::Map<String, serde_json::Value>>, WorkflowServiceError> {
    let Some(settings) = graph_run_settings else {
        return Ok(None);
    };
    let Some(node) = settings
        .nodes
        .iter()
        .find(|node| node.node_id == binding.node_id)
    else {
        return Ok(None);
    };
    let Some(value) = node.data.get("artifact_format_override") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_object().map(Some).ok_or_else(|| {
        WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override for binding '{}.{}' must be an object or null",
            binding.node_id, binding.port_id
        ))
    })
}

fn read_string_override(
    binding: &WorkflowPortBinding,
    object: Option<&serde_json::Map<String, serde_json::Value>>,
    field: &str,
    fallback: &str,
) -> Result<String, WorkflowServiceError> {
    let Some(value) = object.and_then(|object| object.get(field)) else {
        return Ok(fallback.to_string());
    };
    let Some(value) = value.as_str() else {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' must be a string",
            binding.node_id, binding.port_id
        )));
    };
    if value.trim().is_empty() {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' must be non-empty",
            binding.node_id, binding.port_id
        )));
    }
    Ok(value.to_string())
}

fn read_u8_override(
    binding: &WorkflowPortBinding,
    object: Option<&serde_json::Map<String, serde_json::Value>>,
    field: &str,
    fallback: u8,
) -> Result<u8, WorkflowServiceError> {
    let Some(value) = object.and_then(|object| object.get(field)) else {
        return Ok(fallback);
    };
    let Some(value) = value.as_u64() else {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' must be an unsigned integer",
            binding.node_id, binding.port_id
        )));
    };
    u8::try_from(value).map_err(|_| {
        WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' exceeds u8 range",
            binding.node_id, binding.port_id
        ))
    })
}

fn read_u32_override(
    binding: &WorkflowPortBinding,
    object: Option<&serde_json::Map<String, serde_json::Value>>,
    field: &str,
    fallback: u32,
) -> Result<u32, WorkflowServiceError> {
    let Some(value) = object.and_then(|object| object.get(field)) else {
        return Ok(fallback);
    };
    let Some(value) = value.as_u64() else {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' must be an unsigned integer",
            binding.node_id, binding.port_id
        )));
    };
    u32::try_from(value).map_err(|_| {
        WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override field '{field}' for binding '{}.{}' exceeds u32 range",
            binding.node_id, binding.port_id
        ))
    })
}

fn validate_member(
    binding: &WorkflowPortBinding,
    field: &str,
    value: &str,
    allowed: &[String],
) -> Result<(), WorkflowServiceError> {
    if allowed.iter().any(|allowed_value| allowed_value == value) {
        Ok(())
    } else {
        Err(WorkflowServiceError::InvalidRequest(format!(
            "unsupported artifact_format_override {field} '{value}' for binding '{}.{}'",
            binding.node_id, binding.port_id
        )))
    }
}

fn validate_u8_range(
    binding: &WorkflowPortBinding,
    field: &str,
    value: u8,
    min: Option<u8>,
    max: Option<u8>,
) -> Result<(), WorkflowServiceError> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override {field} {value} for binding '{}.{}' is outside allowed range",
            binding.node_id, binding.port_id
        )));
    }
    Ok(())
}

fn validate_u32_range(
    binding: &WorkflowPortBinding,
    field: &str,
    value: u32,
    min: Option<u32>,
    max: Option<u32>,
) -> Result<(), WorkflowServiceError> {
    if min.is_some_and(|min| value < min) || max.is_some_and(|max| value > max) {
        return Err(WorkflowServiceError::InvalidRequest(format!(
            "artifact_format_override {field} {value} for binding '{}.{}' is outside allowed range",
            binding.node_id, binding.port_id
        )));
    }
    Ok(())
}

fn transcode_required_error(
    binding: &WorkflowPortBinding,
    actual_media_type: &str,
    requested_media_type: &str,
) -> WorkflowServiceError {
    WorkflowServiceError::CapabilityViolation(format!(
        "artifact_format_override for binding '{}.{}' requests media_type '{}' but payload is '{}'; transcoding is not implemented",
        binding.node_id, binding.port_id, requested_media_type, actual_media_type
    ))
}

fn authoritative_media_type(
    payload_kind: ArtifactPayloadKind,
    artifact_output: &ArtifactOutput,
    body: &[u8],
) -> Option<String> {
    artifact_output
        .explicit_media_type
        .clone()
        .or_else(|| detect_media_type(payload_kind, body))
}

struct ArtifactOutput {
    payload_kind: ArtifactPayloadKind,
    encoded_body: String,
    explicit_media_type: Option<String>,
}

impl ArtifactOutput {
    fn from_binding(binding: &WorkflowPortBinding) -> Option<Self> {
        let payload_kind = payload_kind_from_port_id(&binding.port_id);
        if let Some(encoded_body) = binding.value.as_str() {
            let payload_kind = payload_kind?;
            return Some(Self {
                payload_kind,
                encoded_body: encoded_body.to_string(),
                explicit_media_type: media_type_from_data_url(encoded_body),
            });
        }
        let object = binding.value.as_object()?;
        let encoded_body = encoded_body_from_object(object, payload_kind)?;
        let explicit_media_type =
            explicit_media_type(object).or_else(|| media_type_from_data_url(encoded_body));
        let payload_kind = payload_kind
            .or_else(|| payload_kind_from_object(object))
            .or_else(|| {
                explicit_media_type
                    .as_deref()
                    .and_then(payload_kind_from_media_type)
            })
            .or_else(|| payload_kind_from_body_field(object))?;
        Some(Self {
            payload_kind,
            encoded_body: encoded_body.to_string(),
            explicit_media_type,
        })
    }
}

fn payload_kind_from_port_id(port_id: &str) -> Option<ArtifactPayloadKind> {
    payload_kind_from_label(port_id)
}

fn encoded_body_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
    port_payload_kind: Option<ArtifactPayloadKind>,
) -> Option<&str> {
    if matches!(
        port_payload_kind,
        Some(
            ArtifactPayloadKind::Image
                | ArtifactPayloadKind::Audio
                | ArtifactPayloadKind::Video
                | ArtifactPayloadKind::ThreeD
        )
    ) {
        if let Some(content) = object.get("content").and_then(|value| value.as_str()) {
            return Some(content);
        }
    }
    for field in [
        "image_base64",
        "audio_base64",
        "audio_data",
        "video_base64",
        "model_base64",
        "mesh_base64",
        "table_base64",
        "csv_base64",
        "parquet_base64",
        "content_base64",
        "body_base64",
        "payload_base64",
        "data_base64",
        "file_base64",
        "bytes_base64",
        "blob_base64",
        "data_url",
        "content_data_url",
    ] {
        if let Some(encoded_body) = object.get(field).and_then(|value| value.as_str()) {
            return Some(encoded_body);
        }
    }
    None
}

fn explicit_media_type(object: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    object
        .get("media_type")
        .or_else(|| object.get("mime_type"))
        .or_else(|| object.get("content_type"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn payload_kind_from_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<ArtifactPayloadKind> {
    for field in ["payload_kind", "artifact_kind", "kind", "type"] {
        if let Some(payload_kind) = object
            .get(field)
            .and_then(|value| value.as_str())
            .and_then(payload_kind_from_label)
        {
            return Some(payload_kind);
        }
    }
    None
}

fn payload_kind_from_label(value: &str) -> Option<ArtifactPayloadKind> {
    match value {
        IMAGE_PORT_ID | "image_base64" => Some(ArtifactPayloadKind::Image),
        AUDIO_PORT_ID | "audio_base64" | "audio_data" => Some(ArtifactPayloadKind::Audio),
        "video" | "video_base64" => Some(ArtifactPayloadKind::Video),
        "3d" | "three_d" | "model_3d" | "mesh" | "point_cloud" => Some(ArtifactPayloadKind::ThreeD),
        "large_table" | "table" | "dataframe" | "csv" | "parquet" => {
            Some(ArtifactPayloadKind::LargeTable)
        }
        "generic_binary" | "binary" | "file" | "blob" | "attachment" => {
            Some(ArtifactPayloadKind::GenericBinary)
        }
        "structured" | "json" | "structured_payload" => Some(ArtifactPayloadKind::Structured),
        _ => None,
    }
}

fn payload_kind_from_media_type(media_type: &str) -> Option<ArtifactPayloadKind> {
    match media_type {
        value if value.starts_with("image/") => Some(ArtifactPayloadKind::Image),
        value if value.starts_with("audio/") => Some(ArtifactPayloadKind::Audio),
        value if value.starts_with("video/") => Some(ArtifactPayloadKind::Video),
        "model/gltf-binary" | "model/gltf+json" | "model/obj" => Some(ArtifactPayloadKind::ThreeD),
        "text/csv" | "application/vnd.apache.parquet" => Some(ArtifactPayloadKind::LargeTable),
        "application/json" | "application/ndjson" => Some(ArtifactPayloadKind::Structured),
        "application/octet-stream" => Some(ArtifactPayloadKind::GenericBinary),
        _ => None,
    }
}

fn payload_kind_from_body_field(
    object: &serde_json::Map<String, serde_json::Value>,
) -> Option<ArtifactPayloadKind> {
    for field in object.keys().map(String::as_str) {
        match field {
            "image_base64" => return Some(ArtifactPayloadKind::Image),
            "audio_base64" | "audio_data" => return Some(ArtifactPayloadKind::Audio),
            "video_base64" => return Some(ArtifactPayloadKind::Video),
            "model_base64" | "mesh_base64" => return Some(ArtifactPayloadKind::ThreeD),
            "table_base64" | "csv_base64" | "parquet_base64" => {
                return Some(ArtifactPayloadKind::LargeTable);
            }
            "file_base64" | "bytes_base64" | "blob_base64" => {
                return Some(ArtifactPayloadKind::GenericBinary);
            }
            _ => {}
        }
    }
    None
}

fn format_metadata(payload_kind: ArtifactPayloadKind, media_type: &str) -> ArtifactFormatMetadata {
    match payload_kind {
        ArtifactPayloadKind::Image => ArtifactFormatMetadata {
            format_id: image_format_id(media_type).to_string(),
            media_type: media_type.to_string(),
            codec_id: None,
            quality_percent: Some(75),
            bitrate_kbps: None,
            crf: None,
            bit_depth: Some("8bit".to_string()),
            color_profile_id: Some("srgb".to_string()),
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
        ArtifactPayloadKind::Audio => ArtifactFormatMetadata {
            format_id: audio_format_id(media_type).to_string(),
            media_type: media_type.to_string(),
            codec_id: Some("opus".to_string()),
            quality_percent: None,
            bitrate_kbps: Some(96),
            crf: None,
            bit_depth: None,
            color_profile_id: None,
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
        ArtifactPayloadKind::Video => ArtifactFormatMetadata {
            format_id: video_format_id(media_type).to_string(),
            media_type: media_type.to_string(),
            codec_id: Some("av1".to_string()),
            quality_percent: None,
            bitrate_kbps: None,
            crf: Some(32),
            bit_depth: Some("8bit".to_string()),
            color_profile_id: None,
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
        ArtifactPayloadKind::ThreeD => ArtifactFormatMetadata {
            format_id: three_d_format_id(media_type).to_string(),
            media_type: media_type.to_string(),
            codec_id: None,
            quality_percent: None,
            bitrate_kbps: None,
            crf: None,
            bit_depth: None,
            color_profile_id: None,
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
        _ => ArtifactFormatMetadata {
            format_id: generic_format_id(media_type).to_string(),
            media_type: media_type.to_string(),
            codec_id: None,
            quality_percent: None,
            bitrate_kbps: None,
            crf: None,
            bit_depth: None,
            color_profile_id: None,
            converter_id: None,
            converter_version: None,
            library_version: None,
        },
    }
}

fn infer_media_type(payload_kind: ArtifactPayloadKind, body: &[u8]) -> String {
    detect_media_type(payload_kind, body).unwrap_or_else(|| match payload_kind {
        ArtifactPayloadKind::Image => DEFAULT_IMAGE_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Audio => DEFAULT_AUDIO_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Video => DEFAULT_VIDEO_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::ThreeD => DEFAULT_THREE_D_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::LargeTable => DEFAULT_TABLE_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Structured => DEFAULT_STRUCTURED_MEDIA_TYPE.to_string(),
        _ => DEFAULT_BINARY_MEDIA_TYPE.to_string(),
    })
}

fn detect_media_type(payload_kind: ArtifactPayloadKind, body: &[u8]) -> Option<String> {
    match payload_kind {
        ArtifactPayloadKind::Image if body.starts_with(b"\x89PNG\r\n\x1a\n") => {
            Some("image/png".to_string())
        }
        ArtifactPayloadKind::Image if body.starts_with(&[0xff, 0xd8, 0xff]) => {
            Some("image/jpeg".to_string())
        }
        ArtifactPayloadKind::Audio if body.starts_with(b"RIFF") => Some("audio/wav".to_string()),
        ArtifactPayloadKind::Audio if body.starts_with(b"OggS") => Some("audio/ogg".to_string()),
        ArtifactPayloadKind::Video if body.starts_with(&[0, 0, 0]) && body.len() > 7 => {
            Some("video/mp4".to_string())
        }
        ArtifactPayloadKind::ThreeD if body.starts_with(b"glTF") => {
            Some(DEFAULT_THREE_D_MEDIA_TYPE.to_string())
        }
        _ => None,
    }
}

fn image_format_id(media_type: &str) -> &str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "jpg",
    }
}

fn audio_format_id(media_type: &str) -> &str {
    match media_type {
        "audio/wav" => "wav",
        "audio/mpeg" => "mp3",
        _ => "ogg",
    }
}

fn video_format_id(media_type: &str) -> &str {
    match media_type {
        "video/webm" => "webm",
        "video/x-matroska" => "mkv",
        _ => "mp4",
    }
}

fn three_d_format_id(media_type: &str) -> &str {
    match media_type {
        "model/gltf+json" => "gltf",
        "model/obj" => "obj",
        _ => "glb",
    }
}

fn generic_format_id(media_type: &str) -> &str {
    match media_type {
        "text/csv" => "csv",
        "application/vnd.apache.parquet" => "parquet",
        "application/json" => "json",
        "application/ndjson" => "ndjson",
        _ => "binary",
    }
}

fn media_type_from_data_url(input: &str) -> Option<String> {
    let (prefix, _) = input.split_once(',')?;
    let prefix = prefix.strip_prefix("data:")?;
    let media_type = prefix.strip_suffix(";base64")?;
    if media_type.is_empty() {
        None
    } else {
        Some(media_type.to_string())
    }
}

fn sanitize_artifact_id(value: &str) -> String {
    let sanitized = value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
                byte as char
            } else {
                '_'
            }
        })
        .collect::<String>();
    sanitized.trim_matches('_').chars().take(40).collect()
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let encoded = input
        .split_once(',')
        .filter(|(prefix, _)| prefix.starts_with("data:") && prefix.ends_with(";base64"))
        .map(|(_, body)| body)
        .unwrap_or(input);
    let clean = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    if clean.len() % 4 == 1 {
        return Err("invalid base64 length".to_string());
    }

    let mut output = Vec::with_capacity(clean.len().saturating_mul(3) / 4);
    for chunk in clean.chunks(4) {
        let mut values = [0_u8; 4];
        let mut padding = 0;
        for (index, byte) in chunk.iter().copied().enumerate() {
            if byte == b'=' {
                padding += 1;
                values[index] = 0;
            } else {
                values[index] = decode_base64_byte(byte)?;
            }
        }
        if chunk.len() < 4 {
            padding += 4 - chunk.len();
        }
        let triple = ((values[0] as u32) << 18)
            | ((values[1] as u32) << 12)
            | ((values[2] as u32) << 6)
            | values[3] as u32;
        output.push(((triple >> 16) & 0xff) as u8);
        if padding < 2 {
            output.push(((triple >> 8) & 0xff) as u8);
        }
        if padding == 0 {
            output.push((triple & 0xff) as u8);
        }
    }
    Ok(output)
}

fn decode_base64_byte(byte: u8) -> Result<u8, String> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(format!("invalid base64 byte 0x{byte:02x}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{convert_media_outputs_to_artifacts, decode_base64};
    use crate::graph::{WorkflowGraphRunSettings, WorkflowGraphRunSettingsNode};
    use crate::workflow::{
        ArtifactDescriptor, ArtifactFormatSettings, ArtifactFormatSettingsUpdateRequest,
        ArtifactPayloadKind, ArtifactPolicy, ArtifactStore, WorkflowPortBinding, WorkflowService,
        WorkflowServiceError,
    };

    fn policy() -> ArtifactPolicy {
        ArtifactPolicy {
            policy_id: "artifact-global-default".to_string(),
            policy_version: 1,
            ttl_seconds: None,
            max_disk_bytes: Some(1024 * 1024),
            max_memory_bytes: Some(64 * 1024),
            max_single_artifact_bytes: Some(128 * 1024),
            spill_threshold_bytes: Some(1024),
            delete_on_consume: false,
        }
    }

    #[test]
    fn decode_base64_accepts_data_urls() {
        let decoded = decode_base64("data:image/png;base64,aGVsbG8=").expect("decode");

        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn convert_media_outputs_replaces_base64_with_artifact_descriptor() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
                value: serde_json::json!("aGVsbG8="),
            }],
        )
        .expect("convert output");

        assert_eq!(converted.len(), 1);
        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        assert_eq!(descriptor.artifact_id, "run_run-a_image-output_image");
        assert_eq!(descriptor.byte_length, Some(5));
        assert_eq!(
            descriptor.read_handle.as_deref(),
            Some("artifact-read://run_run-a_image-output_image")
        );
        assert!(!converted[0].value.to_string().contains("aGVsbG8="));
    }

    #[test]
    fn convert_media_outputs_replaces_video_base64_with_artifact_descriptor() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "video-output".to_string(),
                port_id: "video".to_string(),
                value: serde_json::json!({
                    "content": "data:video/webm;base64,aGVsbG8=",
                }),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        assert_eq!(descriptor.payload_kind, ArtifactPayloadKind::Video);
        assert_eq!(descriptor.byte_length, Some(5));
        assert_eq!(descriptor.format.expect("format").media_type, "video/webm");
        assert!(!converted[0].value.to_string().contains("aGVsbG8="));
    }

    #[test]
    fn convert_media_outputs_replaces_file_base64_with_generic_binary_descriptor() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "file-output".to_string(),
                port_id: "file".to_string(),
                value: serde_json::json!({
                    "file_name": "payload.bin",
                    "file_base64": "aGVsbG8=",
                    "media_type": "application/octet-stream",
                }),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        assert_eq!(descriptor.payload_kind, ArtifactPayloadKind::GenericBinary);
        assert_eq!(descriptor.byte_length, Some(5));
        assert_eq!(
            descriptor.read_handle.as_deref(),
            Some("artifact-read://run_run-a_file-output_file")
        );
        assert!(!converted[0].value.to_string().contains("aGVsbG8="));
    }

    #[test]
    fn convert_media_outputs_replaces_table_base64_with_large_table_descriptor() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "table-output".to_string(),
                port_id: "table".to_string(),
                value: serde_json::json!({
                    "kind": "table",
                    "data_base64": "YSxiCg==",
                    "media_type": "text/csv",
                }),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        assert_eq!(descriptor.payload_kind, ArtifactPayloadKind::LargeTable);
        assert_eq!(descriptor.byte_length, Some(4));
        assert_eq!(descriptor.format.expect("format").format_id, "csv");
        assert!(!converted[0].value.to_string().contains("YSxiCg=="));
    }

    #[test]
    fn convert_media_outputs_uses_backend_format_defaults_without_graph_settings() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let mut settings = ArtifactFormatSettings::default();
        settings.image.format_id = "png".to_string();
        service
            .update_artifact_format_settings(ArtifactFormatSettingsUpdateRequest {
                settings,
                reason: Some("test default".to_string()),
            })
            .expect("update artifact format settings");

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
                value: serde_json::json!("aGVsbG8="),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        let format = descriptor.format.expect("format");
        assert_eq!(format.format_id, "png");
        assert_eq!(format.media_type, "image/png");
    }

    #[test]
    fn convert_media_outputs_uses_graph_image_format_override() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let graph_settings = graph_settings_for_override(
            "image-output",
            serde_json::json!({
                "format_id": "png",
                "quality_percent": 90,
                "color_profile_id": "srgb"
            }),
        );

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            Some(&graph_settings),
            vec![WorkflowPortBinding {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
                value: serde_json::json!("data:image/png;base64,aGVsbG8="),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        let format = descriptor.format.expect("format");
        assert_eq!(format.format_id, "png");
        assert_eq!(format.media_type, "image/png");
        assert_eq!(format.quality_percent, Some(90));
        assert_eq!(format.color_profile_id.as_deref(), Some("srgb"));
    }

    #[test]
    fn convert_media_outputs_uses_graph_audio_format_override() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let graph_settings = graph_settings_for_override(
            "audio-output",
            serde_json::json!({
                "container_id": "ogg",
                "codec_id": "vorbis",
                "bitrate_kbps": 128
            }),
        );

        let converted = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            Some(&graph_settings),
            vec![WorkflowPortBinding {
                node_id: "audio-output".to_string(),
                port_id: "audio".to_string(),
                value: serde_json::json!("data:audio/ogg;base64,T2dnUw=="),
            }],
        )
        .expect("convert output");

        let descriptor: ArtifactDescriptor =
            serde_json::from_value(converted[0].value.clone()).expect("descriptor");
        let format = descriptor.format.expect("format");
        assert_eq!(format.format_id, "ogg");
        assert_eq!(format.media_type, "audio/ogg");
        assert_eq!(format.codec_id.as_deref(), Some("vorbis"));
        assert_eq!(format.bitrate_kbps, Some(128));
    }

    #[test]
    fn convert_media_outputs_rejects_invalid_graph_format_override() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let graph_settings = graph_settings_for_override(
            "image-output",
            serde_json::json!({
                "format_id": "bad-format",
                "quality_percent": 90,
                "color_profile_id": "srgb"
            }),
        );

        let error = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            Some(&graph_settings),
            vec![WorkflowPortBinding {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
                value: serde_json::json!("aGVsbG8="),
            }],
        )
        .expect_err("invalid format");

        assert!(matches!(error, WorkflowServiceError::InvalidRequest(_)));
        assert!(error.to_string().contains("bad-format"));
    }

    #[test]
    fn convert_media_outputs_rejects_override_that_requires_transcoding() {
        let temp = tempfile::tempdir().expect("temp dir");
        let store = ArtifactStore::open(temp.path(), policy()).expect("artifact store");
        let service = WorkflowService::new().with_artifact_store(store);
        let graph_settings = graph_settings_for_override(
            "image-output",
            serde_json::json!({
                "format_id": "jpg",
                "quality_percent": 90,
                "color_profile_id": "srgb"
            }),
        );

        let error = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            Some(&graph_settings),
            vec![WorkflowPortBinding {
                node_id: "image-output".to_string(),
                port_id: "image".to_string(),
                value: serde_json::json!("data:image/png;base64,aGVsbG8="),
            }],
        )
        .expect_err("transcode not implemented");

        assert!(matches!(
            error,
            WorkflowServiceError::CapabilityViolation(_)
        ));
        assert!(error.to_string().contains("transcoding is not implemented"));
    }

    #[test]
    fn convert_media_outputs_requires_configured_artifact_store() {
        let service = WorkflowService::new();

        let error = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
            None,
            vec![WorkflowPortBinding {
                node_id: "audio-output".to_string(),
                port_id: "audio".to_string(),
                value: serde_json::json!("aGVsbG8="),
            }],
        )
        .expect_err("artifact store required");

        assert!(error
            .to_string()
            .contains("artifact store is not configured"));
    }

    fn graph_settings_for_override(
        node_id: &str,
        override_value: serde_json::Value,
    ) -> WorkflowGraphRunSettings {
        WorkflowGraphRunSettings {
            schema_version: 1,
            nodes: vec![WorkflowGraphRunSettingsNode {
                node_id: node_id.to_string(),
                node_type: "output".to_string(),
                data: serde_json::json!({
                    "artifact_format_override": override_value
                }),
            }],
        }
    }
}
