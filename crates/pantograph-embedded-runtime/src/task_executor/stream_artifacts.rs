use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind,
    ArtifactStreamChunkWriteRequest, ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest,
    WorkflowService,
};

#[derive(Clone)]
pub(super) struct StreamArtifactizer {
    workflow_service: Arc<WorkflowService>,
    streams: Arc<Mutex<HashMap<MediaStreamKey, MediaStreamState>>>,
}

impl StreamArtifactizer {
    pub(super) fn new(workflow_service: Arc<WorkflowService>) -> Self {
        Self {
            workflow_service,
            streams: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(super) fn artifactize_chunk(
        &self,
        task_id: &str,
        execution_id: &str,
        port: &str,
        chunk: serde_json::Value,
    ) -> serde_json::Value {
        if inline_media_body_from_chunk(&chunk).is_none() {
            return chunk;
        }
        self.try_artifactize_chunk(task_id, execution_id, port, &chunk)
            .unwrap_or_else(|| redacted_failed_media_chunk(chunk))
    }

    fn try_artifactize_chunk(
        &self,
        task_id: &str,
        execution_id: &str,
        port: &str,
        chunk: &serde_json::Value,
    ) -> Option<serde_json::Value> {
        let media_body = inline_media_body_from_chunk(chunk)?;
        let body = decode_base64(media_body.encoded_body).ok()?;
        let media_type =
            explicit_media_type(chunk).unwrap_or_else(|| default_media_type(media_body.kind));
        let sequence = chunk
            .get("sequence")
            .and_then(|value| value.as_u64())
            .unwrap_or_else(|| self.next_sequence(execution_id, task_id, port).unwrap_or(0));
        let is_final = chunk
            .get("is_final")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let relationship = artifact_relationship_from_chunk(chunk);

        let key = MediaStreamKey::new(execution_id, task_id, port);
        let (artifact_id, stream_handle, byte_range_start) = self.open_or_get_stream(
            &key,
            task_id,
            execution_id,
            port,
            media_body.kind,
            &media_type,
            &relationship,
        )?;
        let byte_range_end_exclusive = byte_range_start.saturating_add(body.len() as u64);

        let record = self
            .workflow_service
            .append_artifact_stream_chunk(ArtifactStreamChunkWriteRequest {
                artifact_id: artifact_id.clone(),
                sequence,
                body,
            })
            .ok()?;

        {
            let mut streams = self.streams.lock().ok()?;
            let state = streams.get_mut(&key)?;
            state.next_sequence = sequence.saturating_add(1);
            state.available_byte_length = byte_range_end_exclusive;
            state.stream_handle = Some(record.stream_handle.clone());
        }

        let finalized = if is_final {
            let descriptor = self
                .workflow_service
                .finalize_artifact_stream(ArtifactStreamFinalizeRequest {
                    artifact_id: artifact_id.clone(),
                })
                .ok()?;
            self.streams.lock().ok()?.remove(&key);
            Some(descriptor)
        } else {
            None
        };

        let lifecycle_state = finalized
            .as_ref()
            .map(|descriptor| serde_json::to_value(descriptor.lifecycle_state))
            .unwrap_or_else(|| serde_json::to_value(record.lifecycle_state))
            .ok()?;
        let read_handle = finalized
            .as_ref()
            .and_then(|descriptor| descriptor.read_handle.clone());

        let mut artifact_chunk = chunk.as_object()?.clone();
        artifact_chunk.remove(media_body.field_name);
        artifact_chunk.insert("artifact_id".to_string(), serde_json::json!(artifact_id));
        artifact_chunk.insert(
            "stream_handle".to_string(),
            serde_json::json!(stream_handle.unwrap_or(record.stream_handle)),
        );
        if let Some(read_handle) = read_handle {
            artifact_chunk.insert("read_handle".to_string(), serde_json::json!(read_handle));
        }
        artifact_chunk.insert("media_type".to_string(), serde_json::json!(media_type));
        artifact_chunk.insert(
            "payload_kind".to_string(),
            serde_json::to_value(media_body.kind).ok()?,
        );
        artifact_chunk.insert("sequence".to_string(), serde_json::json!(sequence));
        artifact_chunk.insert(
            "byte_length".to_string(),
            serde_json::json!(record.byte_length),
        );
        artifact_chunk.insert(
            "available_byte_length".to_string(),
            serde_json::json!(byte_range_end_exclusive),
        );
        artifact_chunk.insert(
            "byte_range_start".to_string(),
            serde_json::json!(byte_range_start),
        );
        artifact_chunk.insert(
            "byte_range_end_exclusive".to_string(),
            serde_json::json!(byte_range_end_exclusive),
        );
        artifact_chunk.insert("lifecycle_state".to_string(), lifecycle_state);
        artifact_chunk.insert("is_final".to_string(), serde_json::json!(is_final));
        if let Some(artifact_role) = relationship.artifact_role {
            artifact_chunk.insert(
                "artifact_role".to_string(),
                serde_json::json!(artifact_role),
            );
        }
        if let Some(parent_artifact_id) = relationship.parent_artifact_id {
            artifact_chunk.insert(
                "parent_artifact_id".to_string(),
                serde_json::json!(parent_artifact_id),
            );
        }
        if let Some(revision_index) = relationship.revision_index {
            artifact_chunk.insert(
                "revision_index".to_string(),
                serde_json::json!(revision_index),
            );
        }

        Some(serde_json::Value::Object(artifact_chunk))
    }

    fn open_or_get_stream(
        &self,
        key: &MediaStreamKey,
        task_id: &str,
        execution_id: &str,
        port: &str,
        payload_kind: ArtifactPayloadKind,
        media_type: &str,
        relationship: &ArtifactRelationship,
    ) -> Option<(String, Option<String>, u64)> {
        let mut streams = self.streams.lock().ok()?;
        if !streams.contains_key(key) {
            let descriptor = self
                .workflow_service
                .open_artifact_stream(ArtifactStreamOpenRequest {
                    artifact_id: None,
                    payload_kind,
                    media_type: media_type.to_string(),
                    format: Some(format_metadata(payload_kind, media_type)),
                    attribution: ArtifactAttribution {
                        workflow_run_id: execution_id.to_string(),
                        workflow_id: None,
                        workflow_version_id: None,
                        node_id: Some(task_id.to_string()),
                        port_id: Some(port.to_string()),
                        model_id: None,
                        runtime_id: None,
                    },
                    artifact_role: relationship.artifact_role.clone(),
                    parent_artifact_id: relationship.parent_artifact_id.clone(),
                    revision_index: relationship.revision_index,
                })
                .ok()?;
            streams.insert(
                key.clone(),
                MediaStreamState {
                    artifact_id: descriptor.artifact_id,
                    stream_handle: descriptor.stream_handle,
                    next_sequence: 0,
                    available_byte_length: 0,
                },
            );
        }

        let state = streams.get(key)?;
        Some((
            state.artifact_id.clone(),
            state.stream_handle.clone(),
            state.available_byte_length,
        ))
    }

    fn next_sequence(&self, execution_id: &str, task_id: &str, port: &str) -> Option<u64> {
        self.streams
            .lock()
            .ok()?
            .get(&MediaStreamKey::new(execution_id, task_id, port))
            .map(|state| state.next_sequence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MediaStreamKey {
    workflow_run_id: String,
    node_id: String,
    port: String,
}

impl MediaStreamKey {
    fn new(workflow_run_id: &str, node_id: &str, port: &str) -> Self {
        Self {
            workflow_run_id: workflow_run_id.to_string(),
            node_id: node_id.to_string(),
            port: port.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct MediaStreamState {
    artifact_id: String,
    stream_handle: Option<String>,
    next_sequence: u64,
    available_byte_length: u64,
}

#[derive(Debug, Clone, Copy)]
struct InlineMediaBody<'a> {
    field_name: &'static str,
    encoded_body: &'a str,
    kind: ArtifactPayloadKind,
}

#[derive(Debug, Clone, Default)]
struct ArtifactRelationship {
    artifact_role: Option<String>,
    parent_artifact_id: Option<String>,
    revision_index: Option<u64>,
}

fn artifact_relationship_from_chunk(chunk: &serde_json::Value) -> ArtifactRelationship {
    ArtifactRelationship {
        artifact_role: chunk
            .get("artifact_role")
            .or_else(|| chunk.get("preview_role"))
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        parent_artifact_id: chunk
            .get("parent_artifact_id")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        revision_index: chunk.get("revision_index").and_then(|value| value.as_u64()),
    }
}

fn inline_media_body_from_chunk(data: &serde_json::Value) -> Option<InlineMediaBody<'_>> {
    let object = data.as_object()?;
    for (field_name, kind) in [
        ("audio_base64", ArtifactPayloadKind::Audio),
        ("image_base64", ArtifactPayloadKind::Image),
    ] {
        let Some(encoded_body) = object.get(field_name).and_then(|value| value.as_str()) else {
            continue;
        };
        return Some(InlineMediaBody {
            field_name,
            encoded_body,
            kind,
        });
    }
    None
}

fn redacted_failed_media_chunk(chunk: serde_json::Value) -> serde_json::Value {
    let Some(mut object) = chunk.as_object().cloned() else {
        return chunk;
    };
    object.remove("audio_base64");
    object.remove("image_base64");
    object.insert("lifecycle_state".to_string(), serde_json::json!("failed"));
    object.insert(
        "artifact_error".to_string(),
        serde_json::json!("stream artifactization failed"),
    );
    serde_json::Value::Object(object)
}

fn explicit_media_type(data: &serde_json::Value) -> Option<String> {
    data.as_object()?
        .get("media_type")
        .or_else(|| data.as_object()?.get("mime_type"))
        .or_else(|| data.as_object()?.get("content_type"))
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
}

fn default_media_type(payload_kind: ArtifactPayloadKind) -> String {
    match payload_kind {
        ArtifactPayloadKind::Image => "image/png",
        ArtifactPayloadKind::Audio => "audio/wav",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn format_metadata(payload_kind: ArtifactPayloadKind, media_type: &str) -> ArtifactFormatMetadata {
    match payload_kind {
        ArtifactPayloadKind::Audio => ArtifactFormatMetadata {
            format_id: media_type
                .strip_prefix("audio/")
                .filter(|suffix| !suffix.is_empty())
                .map(|suffix| format!("audio_{suffix}"))
                .unwrap_or_else(|| "audio_wav".to_string()),
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
        ArtifactPayloadKind::Image => ArtifactFormatMetadata {
            format_id: match media_type {
                "image/jpeg" => "jpg",
                "image/webp" => "webp",
                _ => "png",
            }
            .to_string(),
            media_type: media_type.to_string(),
            codec_id: None,
            quality_percent: None,
            bitrate_kbps: None,
            crf: None,
            bit_depth: Some("8bit".to_string()),
            color_profile_id: Some("srgb".to_string()),
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
