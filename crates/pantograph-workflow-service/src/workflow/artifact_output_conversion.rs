use super::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowPortBinding, WorkflowService, WorkflowServiceError,
};

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
    outputs: Vec<WorkflowPortBinding>,
) -> Result<Vec<WorkflowPortBinding>, WorkflowServiceError> {
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
    binding: WorkflowPortBinding,
    artifact_output: ArtifactOutput,
) -> Result<WorkflowPortBinding, WorkflowServiceError> {
    let body = decode_base64(&artifact_output.encoded_body).map_err(|reason| {
        WorkflowServiceError::InvalidRequest(format!(
            "binding '{}.{}' contains invalid base64 artifact output: {}",
            binding.node_id, binding.port_id, reason
        ))
    })?;
    let media_type = artifact_output
        .explicit_media_type
        .unwrap_or_else(|| infer_media_type(artifact_output.payload_kind, &body));
    let artifact_id = format!(
        "run_{}_{}_{}",
        sanitize_artifact_id(workflow_run_id),
        sanitize_artifact_id(&binding.node_id),
        sanitize_artifact_id(&binding.port_id)
    );
    let descriptor = service.write_artifact(ArtifactWriteRequest {
        artifact_id: Some(artifact_id),
        payload_kind: artifact_output.payload_kind,
        media_type: media_type.clone(),
        format: Some(format_metadata(artifact_output.payload_kind, &media_type)),
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
    match payload_kind {
        ArtifactPayloadKind::Image if body.starts_with(b"\x89PNG\r\n\x1a\n") => {
            "image/png".to_string()
        }
        ArtifactPayloadKind::Image if body.starts_with(&[0xff, 0xd8, 0xff]) => {
            "image/jpeg".to_string()
        }
        ArtifactPayloadKind::Image => DEFAULT_IMAGE_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Audio if body.starts_with(b"RIFF") => "audio/wav".to_string(),
        ArtifactPayloadKind::Audio if body.starts_with(b"OggS") => "audio/ogg".to_string(),
        ArtifactPayloadKind::Audio => DEFAULT_AUDIO_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Video if body.starts_with(&[0, 0, 0]) && body.len() > 7 => {
            "video/mp4".to_string()
        }
        ArtifactPayloadKind::Video => DEFAULT_VIDEO_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::ThreeD if body.starts_with(b"glTF") => {
            DEFAULT_THREE_D_MEDIA_TYPE.to_string()
        }
        ArtifactPayloadKind::ThreeD => DEFAULT_THREE_D_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::LargeTable => DEFAULT_TABLE_MEDIA_TYPE.to_string(),
        ArtifactPayloadKind::Structured => DEFAULT_STRUCTURED_MEDIA_TYPE.to_string(),
        _ => DEFAULT_BINARY_MEDIA_TYPE.to_string(),
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
    use crate::workflow::{
        ArtifactDescriptor, ArtifactPayloadKind, ArtifactPolicy, ArtifactStore,
        WorkflowPortBinding, WorkflowService,
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
    fn convert_media_outputs_requires_configured_artifact_store() {
        let service = WorkflowService::new();

        let error = convert_media_outputs_to_artifacts(
            &service,
            "workflow-a",
            "1.0.0",
            "run-a",
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
}
