use std::sync::{Arc, Mutex};

use node_engine::EventSink;
use pantograph_workflow_service::{
    ArtifactBodyTransport, ArtifactDescriptorQueryRequest, ArtifactLifecycleState,
    ArtifactPayloadKind, ArtifactPolicy, ArtifactReadRequest, ArtifactStore, WorkflowService,
};
use serde_json::Value;
use tauri::ipc::{Channel, InvokeResponseBody};

use crate::workflow::event_adapter::TauriEventAdapter;
use crate::workflow::events::WorkflowEvent as TauriWorkflowEvent;
use crate::workflow::WorkflowDiagnosticsStore;

const PNG_BYTES: &[u8] = b"\x89PNG\r\n\x1a\n";
const PNG_BYTES_BASE64: &str = "iVBORw0KGgo=";

fn capture_channel() -> (Channel<TauriWorkflowEvent>, Arc<Mutex<Vec<Value>>>) {
    let emitted = Arc::new(Mutex::new(Vec::<Value>::new()));
    let captured = emitted.clone();
    let channel: Channel<TauriWorkflowEvent> = Channel::new(move |body| {
        let value = match body {
            InvokeResponseBody::Json(json) => {
                serde_json::from_str::<Value>(&json).expect("channel event json")
            }
            InvokeResponseBody::Raw(bytes) => {
                serde_json::from_slice::<Value>(&bytes).expect("channel event raw json")
            }
        };
        captured.lock().expect("captured events lock").push(value);
        Ok(())
    });
    (channel, emitted)
}

fn artifact_policy() -> ArtifactPolicy {
    ArtifactPolicy {
        policy_id: "command-bridge-test".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(64 * 1024),
        max_single_artifact_bytes: Some(128 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

fn workflow_service_with_artifact_store(temp: &tempfile::TempDir) -> Arc<WorkflowService> {
    let store = ArtifactStore::open(temp.path(), artifact_policy()).expect("artifact store opens");
    Arc::new(WorkflowService::new().with_artifact_store(store))
}

#[test]
fn command_bridge_preserves_image_artifact_event_and_body_read() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let workflow_service = workflow_service_with_artifact_store(&temp);
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (channel, emitted) = capture_channel();
    let workflow_id = "workflow-editor-image";
    let workflow_run_id = "workflow-run-image";
    let adapter = TauriEventAdapter::new(channel, workflow_id, diagnostics_store)
        .with_workflow_service(workflow_service.clone());

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::TaskStream {
            task_id: "image-node".to_string(),
            execution_id: workflow_run_id.to_string(),
            port: "image".to_string(),
            data: serde_json::json!({
                "type": "image_chunk",
                "image_base64": PNG_BYTES_BASE64,
                "sequence": 0,
                "media_type": "image/png",
                "is_final": true,
                "artifact_role": "generated_image"
            }),
            occurred_at_ms: Some(42),
        },
    )
    .expect("image stream event crosses Tauri channel");

    let artifact_id = {
        let events = emitted.lock().expect("captured events lock");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["type"], "NodeStream");
        assert_eq!(events[1]["type"], "DiagnosticsSnapshot");
        assert_eq!(events[1]["data"]["workflow_run_id"], workflow_run_id);

        let chunk = &events[0]["data"]["chunk"];
        assert!(chunk.get("image_base64").is_none());
        assert_eq!(chunk["payload_kind"], "image");
        assert_eq!(chunk["media_type"], "image/png");
        assert_eq!(chunk["sequence"], 0);
        assert_eq!(chunk["byte_length"], PNG_BYTES.len() as u64);
        assert_eq!(chunk["available_byte_length"], PNG_BYTES.len() as u64);
        assert_eq!(chunk["byte_range_start"], 0);
        assert_eq!(chunk["byte_range_end_exclusive"], PNG_BYTES.len() as u64);
        assert_eq!(chunk["lifecycle_state"], "retained");
        assert_eq!(chunk["artifact_role"], "generated_image");
        assert!(chunk["stream_handle"].as_str().is_some());
        assert!(chunk["read_handle"].as_str().is_some());
        chunk["artifact_id"]
            .as_str()
            .expect("artifact id")
            .to_string()
    };

    let descriptor = super::super::workflow_artifact_descriptor_response(
        workflow_service.as_ref(),
        ArtifactDescriptorQueryRequest {
            artifact_id: artifact_id.clone(),
        },
    )
    .expect("artifact descriptor command bridge forwards backend response")
    .artifact
    .expect("artifact descriptor exists");
    assert_eq!(descriptor.artifact_id, artifact_id);
    assert_eq!(descriptor.payload_kind, ArtifactPayloadKind::Image);
    assert_eq!(descriptor.lifecycle_state, ArtifactLifecycleState::Retained);
    assert_eq!(descriptor.artifact_role.as_deref(), Some("generated_image"));
    assert_eq!(descriptor.attribution.workflow_run_id, workflow_run_id);
    assert_eq!(
        descriptor.attribution.workflow_id.as_deref(),
        Some(workflow_id)
    );
    assert_eq!(
        descriptor.attribution.node_id.as_deref(),
        Some("image-node")
    );
    assert_eq!(descriptor.attribution.port_id.as_deref(), Some("image"));
    assert!(descriptor.read_handle.as_deref().is_some());

    let body = super::super::workflow_read_artifact_body_response(
        workflow_service.as_ref(),
        ArtifactReadRequest {
            artifact_id: artifact_id.clone(),
            byte_range_start: None,
            byte_range_end_exclusive: None,
        },
    )
    .expect("artifact body command bridge forwards backend response");
    assert_eq!(body.body.as_slice(), PNG_BYTES);
    assert_eq!(body.response.artifact_id, artifact_id);
    assert_eq!(body.response.media_type, "image/png");
    assert_eq!(
        body.response.body_transport,
        ArtifactBodyTransport::BinaryBody
    );
    assert_eq!(body.response.byte_length, PNG_BYTES.len() as u64);
    assert!(body.response.complete);
}
