use super::*;

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

fn artifact_policy() -> pantograph_workflow_service::ArtifactPolicy {
    pantograph_workflow_service::ArtifactPolicy {
        policy_id: "artifact-test-default".to_string(),
        policy_version: 1,
        ttl_seconds: None,
        max_disk_bytes: Some(1024 * 1024),
        max_memory_bytes: Some(64 * 1024),
        max_single_artifact_bytes: Some(128 * 1024),
        spill_threshold_bytes: Some(1024),
        delete_on_consume: false,
    }
}

fn workflow_service_with_artifact_store(
    temp: &tempfile::TempDir,
) -> Arc<pantograph_workflow_service::WorkflowService> {
    let store = pantograph_workflow_service::ArtifactStore::open(temp.path(), artifact_policy())
        .expect("artifact store opens");
    Arc::new(pantograph_workflow_service::WorkflowService::new().with_artifact_store(store))
}

#[test]
fn adapter_send_emits_primary_and_diagnostics_events() {
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (channel, emitted) = capture_channel();
    let adapter = TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store);

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::WorkflowStarted {
            workflow_id: "wf-1".to_string(),
            execution_id: "exec-1".to_string(),
            occurred_at_ms: Some(55),
        },
    )
    .expect("send should succeed");

    let events = emitted.lock().expect("captured events lock");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["type"], "Started");
    assert_eq!(events[0]["data"]["workflow_id"], "adapter-workflow");
    assert_eq!(events[0]["data"]["workflow_run_id"], "exec-1");
    assert_eq!(events[1]["type"], "DiagnosticsSnapshot");
    assert_eq!(events[1]["data"]["workflow_run_id"], "exec-1");
    assert_eq!(events[1]["data"]["snapshot"]["runOrder"][0], "exec-1");
    assert_eq!(
        events[1]["data"]["snapshot"]["runsById"]["exec-1"]["workflowId"],
        "adapter-workflow"
    );
}

#[test]
fn adapter_replaces_audio_base64_stream_chunks_with_artifact_references() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let workflow_service = workflow_service_with_artifact_store(&temp);
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (channel, emitted) = capture_channel();
    let adapter = TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store)
        .with_workflow_service(workflow_service);

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::TaskStream {
            task_id: "audio-node".to_string(),
            execution_id: "exec-audio".to_string(),
            port: "audio".to_string(),
            data: serde_json::json!({
                "type": "audio_chunk",
                "audio_base64": "aGVsbG8=",
                "sequence": 0,
                "media_type": "audio/wav",
                "is_final": false
            }),
            occurred_at_ms: Some(10),
        },
    )
    .expect("first audio stream chunk sends");
    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::TaskStream {
            task_id: "audio-node".to_string(),
            execution_id: "exec-audio".to_string(),
            port: "audio".to_string(),
            data: serde_json::json!({
                "type": "audio_chunk",
                "audio_base64": "IQ==",
                "sequence": 1,
                "media_type": "audio/wav",
                "is_final": true
            }),
            occurred_at_ms: Some(11),
        },
    )
    .expect("final audio stream chunk sends");

    let events = emitted.lock().expect("captured events lock");
    let stream_events = events
        .iter()
        .filter(|event| event["type"] == "NodeStream")
        .collect::<Vec<_>>();
    assert_eq!(stream_events.len(), 2);

    for event in &stream_events {
        let chunk = &event["data"]["chunk"];
        assert!(chunk.get("audio_base64").is_none());
        assert_eq!(chunk["media_type"], "audio/wav");
        assert!(chunk["artifact_id"].as_str().is_some());
        assert!(chunk["stream_handle"].as_str().is_some());
        assert!(chunk["byte_length"].as_u64().is_some());
        assert!(chunk["available_byte_length"].as_u64().is_some());
        assert!(chunk["byte_range_start"].as_u64().is_some());
        assert!(chunk["byte_range_end_exclusive"].as_u64().is_some());
    }

    assert_eq!(stream_events[0]["data"]["chunk"]["sequence"], 0);
    assert_eq!(stream_events[0]["data"]["chunk"]["byte_length"], 5);
    assert_eq!(
        stream_events[0]["data"]["chunk"]["lifecycle_state"],
        "streaming"
    );
    assert_eq!(stream_events[1]["data"]["chunk"]["sequence"], 1);
    assert_eq!(stream_events[1]["data"]["chunk"]["byte_length"], 1);
    assert_eq!(
        stream_events[1]["data"]["chunk"]["available_byte_length"],
        6
    );
    assert_eq!(
        stream_events[1]["data"]["chunk"]["lifecycle_state"],
        "retained"
    );
    assert!(stream_events[1]["data"]["chunk"]["read_handle"]
        .as_str()
        .is_some());
}

#[test]
fn adapter_replaces_image_base64_stream_chunks_with_artifact_references() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let workflow_service = workflow_service_with_artifact_store(&temp);
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (channel, emitted) = capture_channel();
    let adapter = TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store)
        .with_workflow_service(workflow_service);

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::TaskStream {
            task_id: "image-node".to_string(),
            execution_id: "exec-image".to_string(),
            port: "image".to_string(),
            data: serde_json::json!({
                "type": "image_chunk",
                "image_base64": "iVBORw0KGgo=",
                "sequence": 0,
                "is_final": false
            }),
            occurred_at_ms: Some(12),
        },
    )
    .expect("image stream chunk sends");

    let events = emitted.lock().expect("captured events lock");
    let stream_event = events
        .iter()
        .find(|event| event["type"] == "NodeStream")
        .expect("node stream event");
    let chunk = &stream_event["data"]["chunk"];

    assert!(chunk.get("image_base64").is_none());
    assert_eq!(chunk["payload_kind"], "image");
    assert_eq!(chunk["media_type"], "image/png");
    assert_eq!(chunk["sequence"], 0);
    assert_eq!(chunk["byte_length"], 8);
    assert_eq!(chunk["available_byte_length"], 8);
    assert_eq!(chunk["byte_range_start"], 0);
    assert_eq!(chunk["byte_range_end_exclusive"], 8);
    assert_eq!(chunk["lifecycle_state"], "streaming");
    assert!(chunk["artifact_id"].as_str().is_some());
    assert!(chunk["stream_handle"].as_str().is_some());
}

#[test]
fn adapter_preserves_text_stream_chunks_unchanged() {
    let temp = tempfile::tempdir().expect("temp artifact store");
    let workflow_service = workflow_service_with_artifact_store(&temp);
    let diagnostics_store = Arc::new(WorkflowDiagnosticsStore::default());
    let (channel, emitted) = capture_channel();
    let adapter = TauriEventAdapter::new(channel, "adapter-workflow", diagnostics_store)
        .with_workflow_service(workflow_service);
    let text_chunk = serde_json::json!({
        "type": "text_delta",
        "text": "hello",
        "sequence": 0,
        "is_final": false
    });

    EventSink::send(
        &adapter,
        node_engine::WorkflowEvent::TaskStream {
            task_id: "text-node".to_string(),
            execution_id: "exec-text".to_string(),
            port: "text".to_string(),
            data: text_chunk.clone(),
            occurred_at_ms: Some(12),
        },
    )
    .expect("text stream chunk sends");

    let events = emitted.lock().expect("captured events lock");
    let stream_event = events
        .iter()
        .find(|event| event["type"] == "NodeStream")
        .expect("node stream event");
    assert_eq!(stream_event["data"]["chunk"], text_chunk);
}
