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

pub(crate) static OWNED_AUDIO_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

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
    let _owned_fixture = OWNED_AUDIO_TEST_LOCK.lock().await;
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
    let _owned_fixture = OWNED_AUDIO_TEST_LOCK.lock().await;
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
    let _owned_fixture = OWNED_AUDIO_TEST_LOCK.lock().await;
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

struct ReadPhaseCancellation {
    context: String,
    cancelled: std::sync::atomic::AtomicBool,
}
impl pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSignal
    for ReadPhaseCancellation
{
    fn snapshot(
        &self,
    ) -> pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
        use pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationState;
        pantograph_runtime_host_contracts::RuntimeHostExecutionCancellationSnapshot {
            cancellation_context_id: self.context.clone(),
            state: if self.cancelled.load(Ordering::SeqCst) {
                RuntimeHostExecutionCancellationState::CancellationRequested
            } else {
                RuntimeHostExecutionCancellationState::Running
            },
            reason: None,
        }
    }
}

#[tokio::test]
async fn owned_audio_read_hash_barrier_retains_physical_snapshot_and_admission_until_worker_completion(
) {
    use pantograph_runtime_host_contracts::{
        RuntimeHostExecutionCancellationContext, RuntimeHostExecutionCancellationHandle,
    };
    use std::time::Duration;
    let _owned_fixture = OWNED_AUDIO_TEST_LOCK.lock().await;
    for mode in ["success", "cancel", "abort", "hash_error", "queued_cancel"] {
        let (_model, mut request, package, target) = fixture();
        let root = tempfile::tempdir().unwrap();
        let mut store = OwnedAudioInputStore::new(writer(&root));
        let body = wav(32000, 16000, 1);
        let body_length = body.len();
        let reference = store
            .import_wav(
                request.handoff.workflow_id.as_str(),
                "original-read-source",
                body,
            )
            .unwrap();
        if mode == "hash_error" {
            let path = root
                .path()
                .join("bodies")
                .join(format!("{}.bin", reference.artifact_id));
            let mut replaced = std::fs::read(&path).unwrap();
            *replaced.last_mut().unwrap() = 1;
            std::fs::write(path, replaced).unwrap();
        }
        request.materialized_inputs[0].value = RuntimeHostExecutionInputValue::MediaArtifactRef(
            RuntimeHostExecutionMediaArtifactRef {
                artifact_id: reference.artifact_id.clone(),
                media_type: reference.media_type,
            },
        );
        let physical = Arc::new(std::sync::Mutex::new(None::<std::sync::Weak<[u8]>>));
        let read_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let admission_entered = Arc::new(tokio::sync::Notify::new());
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let entered_tx = std::sync::Mutex::new(Some(entered_tx));
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        store.before_snapshot_admission = Some(admission_entered.clone());
        store.snapshot_read_barrier = Some({
            let physical = physical.clone();
            let read_count = read_count.clone();
            Arc::new(move |bytes| {
                assert!(
                    bytes.len() == body_length,
                    "real bounded disk read completed"
                );
                read_count.fetch_add(1, Ordering::SeqCst);
                *physical.lock().unwrap() = Some(Arc::downgrade(bytes));
                entered_tx.lock().unwrap().take().unwrap().send(()).unwrap();
                release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
            })
        });
        let capture = Arc::new(Capture::default());
        capture.owned_text_bytes.store(3000, Ordering::SeqCst);
        let port = Arc::new(
            EmbeddedRuntimeHostExecutionPort::with_runtime_dependencies(
                Arc::new(Target(
                    serde_json::from_value(serde_json::to_value(target).unwrap()).unwrap(),
                )),
                Arc::new(Package(package)),
                Arc::new(UnusedMediaSink),
                gateway(capture.clone()),
            )
            .with_owned_audio_store(store),
        );
        let cancellation = Arc::new(ReadPhaseCancellation {
            context: request.cancellation_context.cancellation_context_id.clone(),
            cancelled: Default::default(),
        });
        let mut caller = Some({
            let port = port.clone();
            let request = request.clone();
            let cancellation =
                RuntimeHostExecutionCancellationHandle::with_signal(cancellation.clone());
            tokio::spawn(async move {
                port.execute_runtime_host_request(request, cancellation)
                    .await
            })
        });
        tokio::time::timeout(Duration::from_secs(3), entered_rx)
            .await
            .unwrap()
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), admission_entered.notified())
            .await
            .unwrap();
        let second = tokio::time::timeout(
            Duration::from_secs(3),
            inference::acquire_owned_audio_admission(),
        )
        .await
        .unwrap()
        .unwrap();
        let mut third = if mode == "queued_cancel" {
            None
        } else {
            Some(tokio::spawn(inference::acquire_owned_audio_admission()))
        };
        let mut queued = None;
        if mode == "queued_cancel" {
            let mut queued_request = request.clone();
            queued_request.execution_request_id = "owned-read-queued-request".into();
            queued_request.cancellation_context =
                RuntimeHostExecutionCancellationContext::workflow_service(
                    &queued_request.execution_request_id,
                );
            let queued_signal = Arc::new(ReadPhaseCancellation {
                context: queued_request
                    .cancellation_context
                    .cancellation_context_id
                    .clone(),
                cancelled: Default::default(),
            });
            let queued_handle =
                RuntimeHostExecutionCancellationHandle::with_signal(queued_signal.clone());
            let port = port.clone();
            let mut queued_task = tokio::spawn(async move {
                port.execute_runtime_host_request(queued_request, queued_handle)
                    .await
            });
            tokio::time::timeout(Duration::from_secs(3), admission_entered.notified())
                .await
                .unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(30), &mut queued_task)
                    .await
                    .is_err()
            );
            queued_signal.cancelled.store(true, Ordering::SeqCst);
            queued = Some(queued_task);
        }
        if let Some(waiter) = &mut third {
            assert!(tokio::time::timeout(Duration::from_millis(30), waiter)
                .await
                .is_err());
        }
        if matches!(mode, "abort" | "queued_cancel") {
            let aborted = caller.take().unwrap();
            aborted.abort();
            let stopped = tokio::time::timeout(Duration::from_secs(3), aborted)
                .await
                .unwrap();
            assert!(
                matches!(stopped, Err(error) if error.is_cancelled()),
                "caller loss observed while read/hash remains blocked"
            );
        } else if mode == "cancel" {
            cancellation.cancelled.store(true, Ordering::SeqCst);
        }
        assert!(
            physical
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_some(),
            "actual disk bytes survive caller cancellation/loss"
        );
        assert!(capture.loads.lock().unwrap().is_empty());
        assert!(read_count.load(Ordering::SeqCst) == 1);
        if let Some(waiter) = &mut third {
            assert!(
                tokio::time::timeout(Duration::from_millis(30), waiter)
                    .await
                    .is_err(),
                "read/hash custody retains admission after cancellation/loss"
            );
        }
        if let Some(waiter) = &mut queued {
            assert!(tokio::time::timeout(Duration::from_millis(30), waiter)
                .await
                .is_err());
        }
        release_tx.send(()).unwrap();
        let result = if let Some(caller) = caller {
            Some(
                tokio::time::timeout(Duration::from_secs(3), caller)
                    .await
                    .unwrap(),
            )
        } else {
            None
        };
        match mode {
            "abort" | "queued_cancel" => {
                assert!(result.is_none())
            }
            "cancel" => {
                let response = result.unwrap().unwrap().unwrap();
                assert!(response.state == RuntimeHostExecutionState::Rejected);
                assert!(response.outputs.is_empty());
                assert!(response.diagnostics.iter().any(|d| d.code == pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticCode::CancellationRequested));
            }
            "hash_error" => {
                let response = result.unwrap().unwrap().unwrap();
                assert!(response.state == RuntimeHostExecutionState::Rejected);
                assert!(response.outputs.is_empty());
            }
            _ => {
                let response = result.unwrap().unwrap().unwrap();
                assert!(response.state == RuntimeHostExecutionState::Completed);
                assert!(response.outputs.len() == 8);
            }
        }
        if let Some(waiter) = third {
            let permit = tokio::time::timeout(Duration::from_secs(3), waiter)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(
                physical
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .is_none(),
                "physical bytes released after actual worker completion"
            );
            drop(permit);
        }
        if let Some(waiter) = queued {
            let response = tokio::time::timeout(Duration::from_secs(3), waiter)
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(response.state == RuntimeHostExecutionState::Rejected);
            assert!(response.outputs.is_empty());
            assert!(response.diagnostics.iter().any(|d| d.code == pantograph_runtime_host_contracts::RuntimeHostExecutionDiagnosticCode::CancellationRequested));
            assert!(
                read_count.load(Ordering::SeqCst) == 1,
                "queued cancellation checked before any second disk read"
            );
            assert!(physical
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .upgrade()
                .is_none());
        }
        if mode != "success" {
            assert!(capture.loads.lock().unwrap().is_empty());
        }
        drop(second);
    }
}
