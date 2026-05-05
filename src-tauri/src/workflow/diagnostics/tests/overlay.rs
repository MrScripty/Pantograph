use super::*;

#[test]
fn node_progress_detail_is_exposed_in_diagnostics_snapshot() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Started {
            workflow_id: "wf-1".to_string(),
            node_count: 1,
            workflow_run_id: "exec-1".to_string(),
        },
        1_000,
    );
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeProgress {
            node_id: "llm-1".to_string(),
            progress: 0.0,
            message: Some("kv cache restored".to_string()),
            detail: Some(node_engine::TaskProgressDetail::KvCache(
                node_engine::KvCacheExecutionDiagnostics {
                    action: node_engine::KvCacheEventAction::RestoreInput,
                    outcome: node_engine::KvCacheEventOutcome::Hit,
                    cache_id: Some("cache-1".to_string()),
                    backend_key: Some("llamacpp".to_string()),
                    reuse_source: Some("llamacpp_slot".to_string()),
                    token_count: Some(48),
                    reason: Some("restored_input_handle".to_string()),
                    option_diagnostics: Vec::new(),
                },
            )),
            workflow_run_id: "exec-1".to_string(),
        },
        1_020,
    );

    let snapshot = store.snapshot();
    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    let node = run.nodes.get("llm-1").expect("node trace");
    match node.last_progress_detail.as_ref() {
        Some(node_engine::TaskProgressDetail::KvCache(detail)) => {
            assert_eq!(detail.outcome, node_engine::KvCacheEventOutcome::Hit);
            assert_eq!(detail.cache_id.as_deref(), Some("cache-1"));
        }
        other => panic!("unexpected progress detail: {other:?}"),
    }
    assert_eq!(node.last_progress, None);
}

#[test]
fn diagnostics_overlay_redacts_stream_chunk_inline_media_body() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph("exec-1", &sample_graph());

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeStream {
            node_id: "llm-1".to_string(),
            port: "stream".to_string(),
            chunk: serde_json::json!({
                "type": "audio_chunk",
                "sequence": 7,
                "mime_type": "audio/wav",
                "audio_base64": "data:audio/wav;base64,aGVsbG8="
            }),
            workflow_run_id: "exec-1".to_string(),
        },
        1_030,
    );

    let snapshot = store.snapshot();
    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    let payload = &run.events[0].payload;

    assert_eq!(payload["chunk"]["sequence"], 7);
    assert_eq!(payload["chunk"]["mime_type"], "audio/wav");
    assert_eq!(
        payload["chunk"]["audio_base64"]["diagnostics_redacted"],
        true
    );
    assert_eq!(
        payload["chunk"]["audio_base64"]["reason"],
        "inline_content_body"
    );
}

#[test]
fn diagnostics_overlay_redacts_completed_output_bodies_and_preserves_descriptors() {
    let store = WorkflowDiagnosticsStore::default();
    store.set_execution_metadata("exec-1", Some("wf-1".to_string()));
    store.set_execution_graph("exec-1", &sample_graph());

    let artifact_descriptor = serde_json::json!({
        "artifact_id": "artifact-1",
        "payload_kind": "image",
        "lifecycle_state": "retained",
        "retention_state": "retained",
        "byte_length": 1024,
        "content_hash": "blake3:abc",
        "format": {
            "format_id": "png",
            "media_type": "image/png"
        },
        "attribution": {
            "workflow_run_id": "exec-1",
            "workflow_id": "wf-1",
            "node_id": "llm-1",
            "port_id": "image"
        },
        "access_modes": ["read"],
        "read_handle": "artifact://artifact-1"
    });
    let inline_payload = serde_json::json!({
        "content": "data:image/png;base64,aGVsbG8=",
        "mime_type": "image/png",
        "width": 64,
        "height": 32
    });
    let mut node_outputs = std::collections::HashMap::new();
    node_outputs.insert("image".to_string(), artifact_descriptor.clone());
    node_outputs.insert("preview".to_string(), inline_payload.clone());
    node_outputs.insert("label".to_string(), serde_json::json!("kept text"));

    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::NodeCompleted {
            node_id: "llm-1".to_string(),
            outputs: node_outputs.clone(),
            workflow_run_id: "exec-1".to_string(),
        },
        1_040,
    );

    let mut workflow_outputs = std::collections::HashMap::new();
    workflow_outputs.insert("llm-1".to_string(), node_outputs);
    store.record_workflow_event(
        &crate::workflow::events::WorkflowEvent::Completed {
            workflow_id: "wf-1".to_string(),
            outputs: workflow_outputs,
            workflow_run_id: "exec-1".to_string(),
        },
        1_050,
    );

    let snapshot = store.snapshot();
    let run = snapshot.runs_by_id.get("exec-1").expect("run trace");
    let node_completed_payload = &run.events[0].payload;
    let completed_payload = &run.events[1].payload;

    assert_eq!(
        node_completed_payload["outputs"]["image"],
        artifact_descriptor
    );
    assert_eq!(node_completed_payload["outputs"]["label"], "kept text");
    assert_eq!(
        node_completed_payload["outputs"]["preview"]["mime_type"],
        "image/png"
    );
    assert_eq!(node_completed_payload["outputs"]["preview"]["width"], 64);
    assert_eq!(
        node_completed_payload["outputs"]["preview"]["content"]["diagnostics_redacted"],
        true
    );
    assert_eq!(
        completed_payload["outputs"]["llm-1"]["preview"]["content"]["reason"],
        "inline_content_body"
    );
    assert_eq!(
        completed_payload["outputs"]["llm-1"]["image"]["artifact_id"],
        "artifact-1"
    );
}
