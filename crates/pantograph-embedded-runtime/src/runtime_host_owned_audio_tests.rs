use super::*;
use crate::runtime_host_audio_execution::tests::{fixture, gateway, Capture};
use crate::runtime_host_embedding_execution::tests::{Package, Target, UnusedMediaSink};
use crate::runtime_host_execution_port::EmbeddedRuntimeHostExecutionPort;
use pantograph_runtime_host_contracts::{
    RuntimeHostExecutionInputValue, RuntimeHostExecutionMediaArtifactRef, RuntimeHostExecutionPort,
    RuntimeHostExecutionState,
};
use pantograph_workflow_service::{ArtifactPolicy, ArtifactStore};
use std::sync::{atomic::Ordering, Arc};

pub(crate) fn wav(frames: usize, rate: u32, channels: u16) -> Vec<u8> {
    let data = frames * channels as usize * 2;
    let mut out = Vec::with_capacity(data + 44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((data + 36) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    out.extend_from_slice(&(channels * 2).to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data as u32).to_le_bytes());
    out.resize(data + 44, 0);
    out
}
fn writer(root: &tempfile::TempDir) -> WorkflowArtifactWriter {
    let policy: ArtifactPolicy = serde_json::from_value(serde_json::json!({"policy_id":"owned-audio-test","policy_version":1,"delete_on_consume":false})).unwrap();
    WorkflowArtifactWriter::new(ArtifactStore::open(root.path(), policy).unwrap())
}

#[tokio::test]
async fn owned_audio_import_snapshot_caps_scope_and_tampering() {
    let root = tempfile::tempdir().unwrap();
    let writer = writer(&root);
    let store = OwnedAudioInputStore::new(writer.clone());
    let body = wav(16000 * 300, 16000, 1);
    let reference = store.import_wav("wf", "source-run", body.clone()).unwrap();
    let snapshot = store
        .resolve(reference.artifact_id.clone(), "wf".into())
        .await
        .unwrap();
    assert!(snapshot.bytes() == body);
    assert!(snapshot.source_run_id() == "source-run");
    assert!(store
        .resolve(reference.artifact_id.clone(), "foreign".into())
        .await
        .is_err());
    assert!(store
        .import_wav("wf", "run", wav(16000 * 300 + 1, 16000, 1))
        .is_err());
    let exact = wav((inference::OWNED_AUDIO_MAX_BYTES - 44) / 4, 48000, 2);
    assert!(store.import_wav("wf", "run", exact.clone()).is_ok());
    let mut over = exact;
    over.extend_from_slice(&[0, 0, 0, 0]);
    assert!(store.import_wav("wf", "run", over).is_err());
    for mut malformed in [wav(0, 16000, 1), wav(16000, 16000, 1), wav(16000, 16000, 1)] {
        if malformed.len() > 44 {
            malformed[20] = 3;
        }
        assert!(store.import_wav("wf", "run", malformed).is_err());
    }
    let mut trailing = wav(16000, 16000, 1);
    trailing.extend_from_slice(&[0, 0]);
    assert!(store.import_wav("wf", "run", trailing).is_err());
    // A retained immutable snapshot survives file replacement; new resolution fails.
    std::fs::write(
        root.path()
            .join("bodies")
            .join(format!("{}.bin", reference.artifact_id)),
        wav(16000, 16000, 1),
    )
    .unwrap();
    assert!(store
        .resolve(reference.artifact_id.clone(), "wf".into())
        .await
        .is_err());
    assert!(snapshot.bytes() == body);
    let encoded = serde_json::to_value(&snapshot).unwrap();
    assert!(encoded.get("bytes").is_none());
    assert!(!format!("{snapshot:?}").contains("source-run"));
}

#[tokio::test]
async fn owned_audio_selected_route_exact_source_long_transcript_and_fail_closed() {
    let (_model, mut request, package, target) = fixture();
    let root = tempfile::tempdir().unwrap();
    let store = OwnedAudioInputStore::new(writer(&root));
    let workflow = request.handoff.workflow_id.as_str().to_owned();
    let reference = store
        .import_wav(&workflow, "original-source", wav(32000, 16000, 1))
        .unwrap();
    request.materialized_inputs[0].value =
        RuntimeHostExecutionInputValue::MediaArtifactRef(RuntimeHostExecutionMediaArtifactRef {
            artifact_id: reference.artifact_id.clone(),
            media_type: reference.media_type.clone(),
        });
    let capture = Arc::new(Capture::default());
    let gateway = gateway(capture.clone());
    let port = EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
        Arc::new(Target(
            serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
        )),
        Arc::new(Package(package)),
        Arc::new(UnusedMediaSink),
        gateway,
    )
    .with_owned_audio_store(store);
    for length in [3000, 65536, 65537] {
        capture.owned_text_bytes.store(length, Ordering::SeqCst);
        let cancellation =
            pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
                request.cancellation_context.clone(),
            );
        let response = port
            .execute_runtime_host_request(request.clone(), cancellation)
            .await
            .unwrap();
        if length <= 65536 {
            assert!(response.state == RuntimeHostExecutionState::Completed);
            assert!(response.outputs.len() == 8);
            assert!(response.validate().is_ok());
        } else {
            assert!(response.state == RuntimeHostExecutionState::Failed);
            assert!(response.outputs.is_empty());
        }
    }
    {
        let sources = capture.owned_sources.lock().unwrap();
        assert!(sources
            .iter()
            .all(|s| s.0 == reference.artifact_id && s.2 == workflow && s.3 == "original-source"));
    }
    let loaded = capture.loads.lock().unwrap().len();
    for bad in ["missing", "audio-unverified"] {
        if let RuntimeHostExecutionInputValue::MediaArtifactRef(r) =
            &mut request.materialized_inputs[0].value
        {
            r.artifact_id = bad.into();
        }
        let c = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
            request.cancellation_context.clone(),
        );
        let response = port
            .execute_runtime_host_request(request.clone(), c)
            .await
            .unwrap();
        assert!(response.state == RuntimeHostExecutionState::Rejected);
        assert!(response.outputs.is_empty());
    }
    assert!(capture.loads.lock().unwrap().len() == loaded);
}

#[tokio::test]
async fn owned_audio_caller_abort_keeps_snapshot_and_owner_until_completion() {
    for loading in [true, false] {
        let (_model, mut request, package, target) = fixture();
        let root = tempfile::tempdir().unwrap();
        let store = OwnedAudioInputStore::new(writer(&root));
        let reference = store
            .import_wav(
                request.handoff.workflow_id.as_str(),
                "source-run",
                wav(48000, 16000, 1),
            )
            .unwrap();
        request.materialized_inputs[0].value = RuntimeHostExecutionInputValue::MediaArtifactRef(
            RuntimeHostExecutionMediaArtifactRef {
                artifact_id: reference.artifact_id.clone(),
                media_type: reference.media_type,
            },
        );
        let capture = Arc::new(Capture::default());
        capture.block_load.store(loading, Ordering::SeqCst);
        capture.block.store(!loading, Ordering::SeqCst);
        let gateway = gateway(capture.clone());
        let port = Arc::new(
            EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(Target(
                    serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
                )),
                Arc::new(Package(package)),
                Arc::new(UnusedMediaSink),
                gateway.clone(),
            )
            .with_owned_audio_store(store),
        );
        let c = pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationHandle::running(
            request.cancellation_context.clone(),
        );
        let task = tokio::spawn(async move { port.execute_runtime_host_request(request, c).await });
        tokio::time::timeout(
            std::time::Duration::from_secs(3),
            capture.entered.notified(),
        )
        .await
        .unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        std::fs::remove_file(
            root.path()
                .join("bodies")
                .join(format!("{}.bin", reference.artifact_id)),
        )
        .unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(30), gateway.is_ready())
                .await
                .is_err()
        );
        capture.release.notify_one();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(3), gateway.is_ready())
                .await
                .unwrap()
        );
        assert!(capture.completed.load(Ordering::SeqCst));
        if !loading {
            assert!(capture.owned_sources.lock().unwrap().len() == 1);
        }
    }
}
