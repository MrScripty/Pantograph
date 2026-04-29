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
        self.replace_audio_stream_body(event)
    }

    fn replace_audio_stream_body(
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

        let Some(audio_base64) = data.get("audio_base64").and_then(|value| value.as_str()) else {
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

        let body = decode_base64(audio_base64).map_err(event_error)?;
        let media_type =
            media_type_from_chunk(&data, audio_base64).unwrap_or_else(|| "audio/wav".to_string());
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
                        payload_kind: ArtifactPayloadKind::Audio,
                        media_type: media_type.clone(),
                        format: Some(audio_format_metadata(&media_type)),
                        attribution: ArtifactAttribution {
                            workflow_run_id: execution_id.clone(),
                            workflow_id: Some(self.workflow_id.clone()),
                            workflow_version_id: None,
                            node_id: Some(task_id.clone()),
                            port_id: Some(port.clone()),
                            model_id: None,
                            runtime_id: None,
                        },
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
        chunk.remove("audio_base64");
        chunk.insert("artifact_id".to_string(), serde_json::json!(artifact_id));
        chunk.insert(
            "stream_handle".to_string(),
            serde_json::json!(stream_handle.unwrap_or(record.stream_handle)),
        );
        if let Some(read_handle) = read_handle {
            chunk.insert("read_handle".to_string(), serde_json::json!(read_handle));
        }
        chunk.insert("media_type".to_string(), serde_json::json!(media_type));
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

fn audio_format_metadata(media_type: &str) -> ArtifactFormatMetadata {
    ArtifactFormatMetadata {
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
    }
}

fn media_type_from_chunk(data: &serde_json::Value, audio_base64: &str) -> Option<String> {
    data.get("media_type")
        .or_else(|| data.get("mime_type"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| data_url_media_type(audio_base64).map(str::to_string))
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
        | node_engine::WorkflowEvent::TaskCompleted { .. }
        | node_engine::WorkflowEvent::TaskFailed { .. }
        | node_engine::WorkflowEvent::TaskProgress { .. }
        | node_engine::WorkflowEvent::TaskStream { .. } => {}
    }

    event
}
