//! Event adapter for converting node-engine events to Tauri channel events.
//!
//! The stable `workflow::event_adapter` facade remains in this file while the
//! translation and diagnostics-bridge helpers live in focused submodules.

mod diagnostics_bridge;
mod translation;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Mutex;

use node_engine::{EventError, EventSink};
use pantograph_workflow_service::{
    ArtifactAttribution, ArtifactFormatMetadata, ArtifactPayloadKind,
    ArtifactStreamChunkWriteRequest, ArtifactStreamFinalizeRequest, ArtifactStreamOpenRequest,
    WorkflowGraph,
};
use tauri::ipc::Channel;

use super::commands::SharedWorkflowService;
use super::diagnostics::SharedWorkflowDiagnosticsStore;
use super::events::WorkflowEvent as TauriWorkflowEvent;
use diagnostics_bridge::translate_node_event_with_diagnostics;

/// Adapter that converts node-engine `WorkflowEvent`s to Tauri workflow events
/// and sends them through a Tauri channel to the frontend.
pub struct TauriEventAdapter {
    channel: Channel<TauriWorkflowEvent>,
    workflow_id: String,
    execution_graph: Option<WorkflowGraph>,
    diagnostics_store: SharedWorkflowDiagnosticsStore,
    workflow_service: Option<SharedWorkflowService>,
    media_streams: Mutex<HashMap<MediaStreamKey, MediaStreamState>>,
}

impl TauriEventAdapter {
    /// Create a new adapter with the given Tauri channel and diagnostics store.
    pub fn new(
        channel: Channel<TauriWorkflowEvent>,
        workflow_id: impl Into<String>,
        diagnostics_store: SharedWorkflowDiagnosticsStore,
    ) -> Self {
        Self {
            channel,
            workflow_id: workflow_id.into(),
            execution_graph: None,
            diagnostics_store,
            workflow_service: None,
            media_streams: Mutex::new(HashMap::new()),
        }
    }

    /// Attach the graph that belongs to runtime execution events.
    pub fn with_execution_graph(mut self, graph: WorkflowGraph) -> Self {
        self.execution_graph = Some(graph);
        self
    }

    /// Attach the workflow service used for backend artifact stream storage.
    pub fn with_workflow_service(mut self, workflow_service: SharedWorkflowService) -> Self {
        self.workflow_service = Some(workflow_service);
        self
    }

    fn prepare_event_for_diagnostics(
        &self,
        event: node_engine::WorkflowEvent,
    ) -> Result<node_engine::WorkflowEvent, EventError> {
        let event = workflow_event_with_id(event, &self.workflow_id);
        let execution_id = node_engine_execution_id(&event);
        self.diagnostics_store
            .set_execution_metadata(execution_id, Some(self.workflow_id.clone()));
        if let Some(graph) = &self.execution_graph {
            self.diagnostics_store
                .set_execution_graph(execution_id, graph);
        }
        self.replace_inline_media_stream_body(event)
    }

    fn replace_inline_media_stream_body(
        &self,
        event: node_engine::WorkflowEvent,
    ) -> Result<node_engine::WorkflowEvent, EventError> {
        let node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data,
            occurred_at_ms,
        } = event
        else {
            return Ok(event);
        };

        let Some(media_body) = inline_media_body_from_chunk(&port, &data) else {
            return Ok(node_engine::WorkflowEvent::TaskStream {
                task_id,
                execution_id,
                port,
                data,
                occurred_at_ms,
            });
        };
        let Some(workflow_service) = &self.workflow_service else {
            return Ok(node_engine::WorkflowEvent::TaskStream {
                task_id,
                execution_id,
                port,
                data,
                occurred_at_ms,
            });
        };

        let body = decode_base64(media_body.encoded_body).map_err(event_error)?;
        let media_type =
            media_type_from_chunk(&data, media_body.encoded_body, media_body.kind, &body);
        let is_final = data
            .get("is_final")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let sequence = data
            .get("sequence")
            .and_then(|value| value.as_u64())
            .unwrap_or_else(|| {
                self.media_streams
                    .lock()
                    .ok()
                    .and_then(|streams| {
                        streams
                            .get(&MediaStreamKey::new(&execution_id, &task_id, &port))
                            .map(|state| state.next_sequence)
                    })
                    .unwrap_or(0)
            });
        let relationship = artifact_relationship_from_chunk(&data);

        let key = MediaStreamKey::new(&execution_id, &task_id, &port);
        let (
            artifact_id,
            byte_range_start,
            byte_range_end_exclusive,
            available_byte_length,
            stream_handle,
        ) = {
            let mut streams = self
                .media_streams
                .lock()
                .map_err(|_| event_error("media stream state lock poisoned"))?;
            if !streams.contains_key(&key) {
                let descriptor = workflow_service
                    .open_artifact_stream(ArtifactStreamOpenRequest {
                        artifact_id: None,
                        payload_kind: media_body.kind,
                        media_type: media_type.clone(),
                        format: Some(format_metadata(media_body.kind, &media_type)),
                        attribution: ArtifactAttribution {
                            workflow_run_id: execution_id.clone(),
                            workflow_id: Some(self.workflow_id.clone()),
                            workflow_version_id: None,
                            node_id: Some(task_id.clone()),
                            port_id: Some(port.clone()),
                            model_id: None,
                            runtime_id: None,
                        },
                        artifact_role: relationship.artifact_role.clone(),
                        parent_artifact_id: relationship.parent_artifact_id.clone(),
                        revision_index: relationship.revision_index,
                    })
                    .map_err(|error| event_error(error.to_string()))?;
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

            let state = streams
                .get_mut(&key)
                .ok_or_else(|| event_error("media stream state missing after open"))?;
            let byte_range_start = state.available_byte_length;
            let byte_range_end_exclusive = byte_range_start.saturating_add(body.len() as u64);
            (
                state.artifact_id.clone(),
                byte_range_start,
                byte_range_end_exclusive,
                byte_range_end_exclusive,
                state.stream_handle.clone(),
            )
        };

        let record = workflow_service
            .append_artifact_stream_chunk(ArtifactStreamChunkWriteRequest {
                artifact_id: artifact_id.clone(),
                sequence,
                body,
            })
            .map_err(|error| event_error(error.to_string()))?;

        {
            let mut streams = self
                .media_streams
                .lock()
                .map_err(|_| event_error("media stream state lock poisoned"))?;
            if let Some(state) = streams.get_mut(&key) {
                state.next_sequence = sequence.saturating_add(1);
                state.available_byte_length = available_byte_length;
                state.stream_handle = Some(record.stream_handle.clone());
            }
        }

        let finalized = if is_final {
            let descriptor = workflow_service
                .finalize_artifact_stream(ArtifactStreamFinalizeRequest {
                    artifact_id: artifact_id.clone(),
                })
                .map_err(|error| event_error(error.to_string()))?;
            self.media_streams
                .lock()
                .map_err(|_| event_error("media stream state lock poisoned"))?
                .remove(&key);
            Some(descriptor)
        } else {
            None
        };

        let lifecycle_state = finalized
            .as_ref()
            .map(|descriptor| serde_json::to_value(descriptor.lifecycle_state))
            .unwrap_or_else(|| serde_json::to_value(record.lifecycle_state))
            .map_err(|error| event_error(error.to_string()))?;
        let read_handle = finalized
            .as_ref()
            .and_then(|descriptor| descriptor.read_handle.clone());

        let mut chunk = data.as_object().cloned().unwrap_or_default();
        chunk.remove(media_body.field_name);
        chunk.insert("artifact_id".to_string(), serde_json::json!(artifact_id));
        chunk.insert(
            "stream_handle".to_string(),
            serde_json::json!(stream_handle.unwrap_or(record.stream_handle)),
        );
        if let Some(read_handle) = read_handle {
            chunk.insert("read_handle".to_string(), serde_json::json!(read_handle));
        }
        chunk.insert("media_type".to_string(), serde_json::json!(media_type));
        chunk.insert(
            "payload_kind".to_string(),
            serde_json::to_value(media_body.kind)
                .map_err(|error| event_error(error.to_string()))?,
        );
        chunk.insert("sequence".to_string(), serde_json::json!(sequence));
        chunk.insert(
            "byte_length".to_string(),
            serde_json::json!(record.byte_length),
        );
        chunk.insert(
            "available_byte_length".to_string(),
            serde_json::json!(available_byte_length),
        );
        chunk.insert(
            "byte_range_start".to_string(),
            serde_json::json!(byte_range_start),
        );
        chunk.insert(
            "byte_range_end_exclusive".to_string(),
            serde_json::json!(byte_range_end_exclusive),
        );
        chunk.insert("lifecycle_state".to_string(), lifecycle_state);
        chunk.insert("is_final".to_string(), serde_json::json!(is_final));
        if let Some(artifact_role) = relationship.artifact_role {
            chunk.insert(
                "artifact_role".to_string(),
                serde_json::json!(artifact_role),
            );
        }
        if let Some(parent_artifact_id) = relationship.parent_artifact_id {
            chunk.insert(
                "parent_artifact_id".to_string(),
                serde_json::json!(parent_artifact_id),
            );
        }
        if let Some(revision_index) = relationship.revision_index {
            chunk.insert(
                "revision_index".to_string(),
                serde_json::json!(revision_index),
            );
        }

        Ok(node_engine::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data: serde_json::Value::Object(chunk),
            occurred_at_ms,
        })
    }
}

fn event_error(message: impl Into<String>) -> EventError {
    EventError {
        message: message.into(),
    }
}

impl EventSink for TauriEventAdapter {
    fn send(&self, event: node_engine::WorkflowEvent) -> Result<(), EventError> {
        let event = self.prepare_event_for_diagnostics(event)?;
        let (tauri_event, diagnostics_event) =
            translate_node_event_with_diagnostics(&self.diagnostics_store, event);

        self.channel
            .send(tauri_event)
            .map_err(|_| EventError::channel_closed())
            .and_then(|_| {
                self.channel
                    .send(diagnostics_event)
                    .map_err(|_| EventError::channel_closed())
            })
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

fn artifact_relationship_from_chunk(data: &serde_json::Value) -> ArtifactRelationship {
    ArtifactRelationship {
        artifact_role: data
            .get("artifact_role")
            .or_else(|| data.get("preview_role"))
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        parent_artifact_id: data
            .get("parent_artifact_id")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        revision_index: data.get("revision_index").and_then(|value| value.as_u64()),
    }
}

fn inline_media_body_from_chunk<'a>(
    port: &str,
    data: &'a serde_json::Value,
) -> Option<InlineMediaBody<'a>> {
    let object = data.as_object()?;
    for (field_name, field_kind) in [
        ("image_base64", Some(ArtifactPayloadKind::Image)),
        ("audio_base64", Some(ArtifactPayloadKind::Audio)),
        ("audio_data", Some(ArtifactPayloadKind::Audio)),
        ("video_base64", Some(ArtifactPayloadKind::Video)),
        ("model_base64", Some(ArtifactPayloadKind::ThreeD)),
        ("mesh_base64", Some(ArtifactPayloadKind::ThreeD)),
        ("file_base64", Some(ArtifactPayloadKind::GenericBinary)),
        ("bytes_base64", Some(ArtifactPayloadKind::GenericBinary)),
        ("blob_base64", Some(ArtifactPayloadKind::GenericBinary)),
        ("data_base64", None),
        ("data_url", None),
    ] {
        let Some(encoded_body) = object.get(field_name).and_then(|value| value.as_str()) else {
            continue;
        };
        let kind = field_kind
            .or_else(|| payload_kind_from_object(object))
            .or_else(|| payload_kind_from_label(port))
            .or_else(|| {
                explicit_media_type(object).and_then(|value| payload_kind_from_media_type(&value))
            })
            .or_else(|| data_url_media_type(encoded_body).and_then(payload_kind_from_media_type))?;
        return Some(InlineMediaBody {
            field_name,
            encoded_body,
            kind,
        });
    }
    None
}

fn media_type_from_chunk(
    data: &serde_json::Value,
    encoded_body: &str,
    payload_kind: ArtifactPayloadKind,
    body: &[u8],
) -> String {
    data.as_object()
        .and_then(explicit_media_type)
        .or_else(|| data_url_media_type(encoded_body).map(str::to_string))
        .or_else(|| detect_media_type(payload_kind, body))
        .unwrap_or_else(|| default_media_type(payload_kind).to_string())
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
        if let Some(kind) = object
            .get(field)
            .and_then(|value| value.as_str())
            .and_then(payload_kind_from_label)
        {
            return Some(kind);
        }
    }
    None
}

fn payload_kind_from_label(value: &str) -> Option<ArtifactPayloadKind> {
    let normalized = value
        .strip_suffix("_chunk")
        .or_else(|| value.strip_suffix("-chunk"))
        .unwrap_or(value);
    match normalized {
        "image" | "image_base64" => Some(ArtifactPayloadKind::Image),
        "audio" | "audio_base64" | "audio_data" => Some(ArtifactPayloadKind::Audio),
        "video" | "video_base64" => Some(ArtifactPayloadKind::Video),
        "3d" | "three_d" | "model_3d" | "mesh" | "point_cloud" => Some(ArtifactPayloadKind::ThreeD),
        "generic_binary" | "binary" | "file" | "blob" | "attachment" => {
            Some(ArtifactPayloadKind::GenericBinary)
        }
        _ => None,
    }
}

fn payload_kind_from_media_type(media_type: &str) -> Option<ArtifactPayloadKind> {
    match media_type {
        value if value.starts_with("image/") => Some(ArtifactPayloadKind::Image),
        value if value.starts_with("audio/") => Some(ArtifactPayloadKind::Audio),
        value if value.starts_with("video/") => Some(ArtifactPayloadKind::Video),
        "model/gltf-binary" | "model/gltf+json" | "model/obj" => Some(ArtifactPayloadKind::ThreeD),
        "application/octet-stream" => Some(ArtifactPayloadKind::GenericBinary),
        _ => None,
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
            conversion_id: None,
            conversion_status: None,
            conversion_command_id: None,
            conversion_dependencies: Vec::new(),
        },
        ArtifactPayloadKind::Audio => ArtifactFormatMetadata {
            format_id: media_type
                .strip_prefix("audio/")
                .filter(|suffix| !suffix.is_empty())
                .map(|suffix| format!("audio_{suffix}"))
                .unwrap_or_else(|| "audio_wav".to_string()),
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
            conversion_id: None,
            conversion_status: None,
            conversion_command_id: None,
            conversion_dependencies: Vec::new(),
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
            conversion_id: None,
            conversion_status: None,
            conversion_command_id: None,
            conversion_dependencies: Vec::new(),
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
            conversion_id: None,
            conversion_status: None,
            conversion_command_id: None,
            conversion_dependencies: Vec::new(),
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
            conversion_id: None,
            conversion_status: None,
            conversion_command_id: None,
            conversion_dependencies: Vec::new(),
        },
    }
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
            Some("model/gltf-binary".to_string())
        }
        _ => None,
    }
}

fn default_media_type(payload_kind: ArtifactPayloadKind) -> &'static str {
    match payload_kind {
        ArtifactPayloadKind::Image => "image/jpeg",
        ArtifactPayloadKind::Audio => "audio/wav",
        ArtifactPayloadKind::Video => "video/mp4",
        ArtifactPayloadKind::ThreeD => "model/gltf-binary",
        _ => "application/octet-stream",
    }
}

fn image_format_id(media_type: &str) -> &str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        _ => "jpg",
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
        "application/json" => "json",
        "application/ndjson" => "ndjson",
        _ => "binary",
    }
}

fn data_url_media_type(value: &str) -> Option<&str> {
    let (prefix, _) = value.split_once(',')?;
    prefix
        .strip_prefix("data:")
        .and_then(|value| value.strip_suffix(";base64"))
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

fn node_engine_execution_id(event: &node_engine::WorkflowEvent) -> &str {
    match event {
        node_engine::WorkflowEvent::WorkflowStarted { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowCompleted { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowFailed { execution_id, .. }
        | node_engine::WorkflowEvent::WorkflowCancelled { execution_id, .. }
        | node_engine::WorkflowEvent::WaitingForInput { execution_id, .. }
        | node_engine::WorkflowEvent::TaskStarted { execution_id, .. }
        | node_engine::WorkflowEvent::TaskInputsResolved { execution_id, .. }
        | node_engine::WorkflowEvent::TaskCompleted { execution_id, .. }
        | node_engine::WorkflowEvent::TaskFailed { execution_id, .. }
        | node_engine::WorkflowEvent::TaskProgress { execution_id, .. }
        | node_engine::WorkflowEvent::TaskStream { execution_id, .. }
        | node_engine::WorkflowEvent::GraphModified { execution_id, .. }
        | node_engine::WorkflowEvent::IncrementalExecutionStarted { execution_id, .. } => {
            execution_id
        }
    }
}

fn workflow_event_with_id(
    mut event: node_engine::WorkflowEvent,
    workflow_id: &str,
) -> node_engine::WorkflowEvent {
    match &mut event {
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowCompleted {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowFailed {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WorkflowCancelled {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::WaitingForInput {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::GraphModified {
            workflow_id: event_id,
            ..
        }
        | node_engine::WorkflowEvent::IncrementalExecutionStarted {
            workflow_id: event_id,
            ..
        } => {
            *event_id = workflow_id.to_string();
        }
        node_engine::WorkflowEvent::TaskStarted { .. }
        | node_engine::WorkflowEvent::TaskInputsResolved { .. }
        | node_engine::WorkflowEvent::TaskCompleted { .. }
        | node_engine::WorkflowEvent::TaskFailed { .. }
        | node_engine::WorkflowEvent::TaskProgress { .. }
        | node_engine::WorkflowEvent::TaskStream { .. } => {}
    }

    event
}
