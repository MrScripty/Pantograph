use super::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind, ArtifactWriteRequest,
    WorkflowPortBinding, WorkflowService, WorkflowServiceError,
};

const IMAGE_PORT_ID: &str = "image";
const AUDIO_PORT_ID: &str = "audio";
const DEFAULT_IMAGE_MEDIA_TYPE: &str = "image/jpeg";
const DEFAULT_AUDIO_MEDIA_TYPE: &str = "audio/ogg";

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
            let Some(media_output) = MediaOutput::from_binding(&binding) else {
                return Ok(binding);
            };
            convert_media_output(
                service,
                workflow_id,
                workflow_version_id,
                workflow_run_id,
                binding,
                media_output,
            )
        })
        .collect()
}

fn convert_media_output(
    service: &WorkflowService,
    workflow_id: &str,
    workflow_version_id: &str,
    workflow_run_id: &str,
    binding: WorkflowPortBinding,
    media_output: MediaOutput,
) -> Result<WorkflowPortBinding, WorkflowServiceError> {
    let body = decode_base64(&media_output.encoded_body).map_err(|reason| {
        WorkflowServiceError::InvalidRequest(format!(
            "binding '{}.{}' contains invalid base64 media output: {}",
            binding.node_id, binding.port_id, reason
        ))
    })?;
    let media_type = media_output
        .explicit_media_type
        .unwrap_or_else(|| infer_media_type(media_output.payload_kind, &body));
    let artifact_id = format!(
        "run_{}_{}_{}",
        sanitize_artifact_id(workflow_run_id),
        sanitize_artifact_id(&binding.node_id),
        sanitize_artifact_id(&binding.port_id)
    );
    let descriptor = service.write_artifact(ArtifactWriteRequest {
        artifact_id: Some(artifact_id),
        payload_kind: media_output.payload_kind,
        media_type: media_type.clone(),
        format: Some(format_metadata(media_output.payload_kind, &media_type)),
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

struct MediaOutput {
    payload_kind: ArtifactPayloadKind,
    encoded_body: String,
    explicit_media_type: Option<String>,
}

impl MediaOutput {
    fn from_binding(binding: &WorkflowPortBinding) -> Option<Self> {
        let payload_kind = match binding.port_id.as_str() {
            IMAGE_PORT_ID => ArtifactPayloadKind::Image,
            AUDIO_PORT_ID => ArtifactPayloadKind::Audio,
            _ => return None,
        };
        if let Some(encoded_body) = binding.value.as_str() {
            return Some(Self {
                payload_kind,
                encoded_body: encoded_body.to_string(),
                explicit_media_type: None,
            });
        }
        let object = binding.value.as_object()?;
        let encoded_body = object
            .get("content")
            .or_else(|| object.get("image_base64"))
            .or_else(|| object.get("audio_base64"))
            .or_else(|| object.get("audio_data"))
            .and_then(|value| value.as_str())?;
        let explicit_media_type = object
            .get("media_type")
            .or_else(|| object.get("mime_type"))
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned);
        Some(Self {
            payload_kind,
            encoded_body: encoded_body.to_string(),
            explicit_media_type,
        })
    }
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
        _ => ArtifactFormatMetadata {
            format_id: "binary".to_string(),
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
        _ => "application/octet-stream".to_string(),
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
        ArtifactDescriptor, ArtifactPolicy, ArtifactStore, WorkflowPortBinding, WorkflowService,
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
