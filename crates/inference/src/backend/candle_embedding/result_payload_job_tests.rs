use super::*;
use std::time::Duration;

fn payload(width: usize) -> Vec<EmbeddingResult> {
    vec![EmbeddingResult {
        vector: vec![0.25; width],
        token_count: 2,
    }]
}

fn observed_start(
    jobs: &EmbeddingJobs,
    run: impl FnOnce(Arc<AtomicBool>) -> Result<Vec<EmbeddingResult>, BackendError> + Send + 'static,
) -> Job<Vec<EmbeddingResult>> {
    let ledger = jobs
        .payloads
        .get_or_init(|| Arc::new(ResultPayloadLedger::new()))
        .clone();
    jobs.start_observed(run, Some((ledger, embedding_payload_capacity)))
        .unwrap()
}

fn counts(jobs: &EmbeddingJobs) -> Value {
    jobs.payloads.get().unwrap().diagnostic("test")["observation"].clone()
}

#[tokio::test]
async fn result_payload_unpolled_worker_completion_survives_caller_loss_until_drain() {
    let jobs = EmbeddingJobs::default();
    let output = payload(3);
    let expected = embedding_payload_capacity(&output).unwrap();
    let job = observed_start(&jobs, |_| Ok(output));
    // Never poll Shared/the join; accounting must already happen in the worker.
    tokio::time::timeout(Duration::from_secs(2), async {
        while counts(&jobs)["registered_owners"] != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(job.completion.peek().is_none());
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], expected);
    drop(job);
    assert!(!jobs.is_idle());
    assert_eq!(counts(&jobs)["released_owners"], 0);
    jobs.drain(false).await.unwrap();
    assert!(jobs.is_idle());
    assert_eq!(counts(&jobs)["released_owners"], 1);
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], 0);
}

#[tokio::test]
async fn result_payload_shared_aliases_and_caller_copies_have_distinct_lifetimes() {
    let jobs = EmbeddingJobs::default();
    let job = observed_start(&jobs, |_| Ok(payload(4)));
    let reader = job.completion.clone();
    let completed = job.completion.await;
    let caller_copy = copy_result(completed.as_ref()).unwrap();
    let live = counts(&jobs)["live_payload_capacity_bytes"]
        .as_u64()
        .unwrap();
    assert_eq!(counts(&jobs)["registered_owners"], 1);
    jobs.drain(false).await.unwrap();
    assert!(jobs.is_idle());
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], live);
    drop(completed);
    assert_eq!(counts(&jobs)["released_owners"], 0);
    drop(reader);
    assert_eq!(counts(&jobs)["released_owners"], 1);
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], 0);
    // The independently allocated caller copy is deliberately outside the ledger.
    assert_eq!(caller_copy[0].vector, vec![0.25; 4]);
}

#[tokio::test]
async fn result_payload_completed_slot_replacement_preserves_old_reader_storage() {
    let jobs = EmbeddingJobs::default();
    let first = observed_start(&jobs, |_| Ok(payload(2))).completion.await;
    let first_bytes = counts(&jobs)["live_payload_capacity_bytes"]
        .as_u64()
        .unwrap();
    let next_payload = payload(5);
    let next_bytes = embedding_payload_capacity(&next_payload).unwrap();
    let second = observed_start(&jobs, |_| Ok(next_payload)).completion.await;
    assert_eq!(counts(&jobs)["registered_owners"], 2);
    assert_eq!(
        counts(&jobs)["live_payload_capacity_bytes"],
        first_bytes + next_bytes
    );
    assert_eq!(
        counts(&jobs)["peak_observed_live_payload_capacity_bytes"],
        first_bytes + next_bytes
    );
    drop(second);
    jobs.drain(false).await.unwrap();
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], first_bytes);
    assert_eq!(counts(&jobs)["released_owners"], 1);
    drop(first);
    assert_eq!(counts(&jobs)["released_owners"], 2);
    assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], 0);
}

#[test]
fn result_payload_original_value_drops_before_accounting_lease() {
    struct DropProbe {
        ledger: Arc<ResultPayloadLedger>,
        saw_live: Arc<AtomicBool>,
    }
    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.saw_live.store(
                self.ledger.diagnostic("drop_probe")["observation"]["live_payload_capacity_bytes"]
                    == 7,
                Ordering::Release,
            );
        }
    }
    let ledger = Arc::new(ResultPayloadLedger::new());
    let saw_live = Arc::new(AtomicBool::new(false));
    // Hand-set bookkeeping tests field destruction order, not physical memory.
    let completed = Completed {
        result: Ok::<_, BackendError>(DropProbe {
            ledger: ledger.clone(),
            saw_live: saw_live.clone(),
        }),
        _payload_lease: ledger.retain(Some(7)),
    };
    drop(completed);
    assert!(saw_live.load(Ordering::Acquire));
    assert_eq!(
        ledger.diagnostic("after_drop")["observation"]["live_payload_capacity_bytes"],
        0
    );
}

#[tokio::test]
async fn result_payload_errors_panics_and_unavailable_measurement_preserve_outcomes() {
    let jobs = EmbeddingJobs::default();
    let failed = observed_start(&jobs, |_| Err(inference("controlled failure")))
        .completion
        .await;
    assert!(matches!(
        copy_result(failed.as_ref()),
        Err(BackendError::Inference(_))
    ));
    drop(failed);
    jobs.drain(false).await.unwrap();
    let panicked = observed_start(&jobs, |_| panic!("controlled worker panic"))
        .completion
        .await;
    assert!(matches!(
        copy_result(panicked.as_ref()),
        Err(BackendError::Inference(_))
    ));
    drop(panicked);
    jobs.drain(true).await.unwrap();
    assert_eq!(counts(&jobs)["registered_owners"], 0);
    let ledger = jobs.payloads.get().unwrap().clone();
    let unmeasured = jobs
        .start_observed(|_| Ok(payload(3)), Some((ledger, |_| None)))
        .unwrap()
        .completion
        .await;
    assert_eq!(copy_result(unmeasured.as_ref()).unwrap()[0].vector.len(), 3);
    assert_eq!(counts(&jobs)["state"], "unavailable");
    drop(unmeasured);
    jobs.drain(false).await.unwrap();
    assert!(jobs.is_idle());
    assert_eq!(counts(&jobs)["state"], "unavailable");
}

#[tokio::test]
async fn result_payload_cancellation_waits_for_real_worker_completion() {
    let jobs = Arc::new(EmbeddingJobs::default());
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let job = observed_start(&jobs, move |stop| {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        assert!(stop.load(Ordering::Acquire));
        Ok(payload(3))
    });
    started_rx.await.unwrap();
    drop(job);
    let draining = jobs.clone();
    let drain = tokio::spawn(async move { draining.drain(true).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if jobs
                .slot
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .stop
                .load(Ordering::Acquire)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!drain.is_finished());
    // No completed output exists yet; this says nothing about running memory.
    assert_eq!(counts(&jobs)["registered_owners"], 0);
    release_tx.send(()).unwrap();
    drain.await.unwrap().unwrap();
    assert_eq!(counts(&jobs)["registered_owners"], 1);
    assert_eq!(counts(&jobs)["released_owners"], 1);
    assert!(jobs.is_idle());
}

#[tokio::test]
async fn result_payload_actual_execute_uses_only_existing_calibration_gate() {
    use crate::backend::InferenceBackend;
    let (_directory, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    for enabled in [false, true] {
        let mut backend = if enabled {
            let config =
                crate::CandleCpuCalibrationConfig::new(1, 3, Duration::from_secs(60)).unwrap();
            super::super::candle::CandleBackend::with_calibration(
                crate::candle_cpu_calibration::CalibrationOwner::new(config).unwrap(),
            )
        } else {
            super::super::candle::CandleBackend::new()
        };
        backend
            .load_selected_embedding(&request, &target, &decision)
            .await
            .unwrap();
        let jobs = EmbeddingJobs::default();
        let vectors = jobs
            .execute(
                backend.model.clone().unwrap(),
                vec!["hello".into()],
                InferenceExecutionCancellationHandle::running(),
            )
            .await
            .unwrap();
        assert_eq!(vectors[0].vector.len(), 8);
        assert_eq!(jobs.payloads.get().is_some(), enabled);
        if enabled {
            assert_eq!(counts(&jobs)["registered_owners"], 1);
            assert!(
                counts(&jobs)["live_payload_capacity_bytes"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
        }
        jobs.drain(false).await.unwrap();
        if enabled {
            assert_eq!(counts(&jobs)["released_owners"], 1);
            assert_eq!(counts(&jobs)["live_payload_capacity_bytes"], 0);
        }
        backend.stop().await.unwrap();
    }
}
