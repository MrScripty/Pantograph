//! The prior backend has a distinct registry identity but delegates real forward
//! execution to frozen Candle weights. This does not qualify llama.cpp or a GPU.
use super::*;
use crate::backend::{BackendFactory, BackendStartOutcome, CandleBackend};
use futures_util::FutureExt;
use std::sync::atomic::AtomicBool;

struct Resident {
    candle: CandleBackend,
    stops: Arc<AtomicUsize>,
}

#[async_trait]
impl InferenceBackend for Resident {
    fn name(&self) -> &'static str {
        "fixture-resident"
    }
    fn description(&self) -> &'static str {
        "real Candle forward with distinct test backend identity"
    }
    fn capabilities(&self) -> BackendCapabilities {
        self.candle.capabilities()
    }
    async fn start(
        &mut self,
        config: &BackendConfig,
        spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.candle.start(config, spawner).await
    }
    async fn stop(&mut self) -> Result<(), BackendError> {
        self.stops.fetch_add(1, Ordering::SeqCst);
        self.candle.stop().await
    }
    fn is_ready(&self) -> bool {
        self.candle.is_ready()
    }
    async fn health_check(&self) -> bool {
        self.candle.health_check().await
    }
    fn base_url(&self) -> Option<String> {
        None
    }
    async fn chat_completion_stream(
        &self,
        request: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        self.candle.chat_completion_stream(request).await
    }
    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        self.candle.embeddings(texts, model).await
    }
    async fn rerank(&self, request: RerankRequest) -> Result<RerankResponse, BackendError> {
        self.candle.rerank(request).await
    }
}

struct Factory {
    hook: Arc<dyn Fn() + Send + Sync>,
}
impl BackendFactory for Factory {
    fn create(&self) -> Result<Box<dyn InferenceBackend>, BackendError> {
        let mut candle = CandleBackend::new();
        candle.load_hook = Some(self.hook.clone());
        Ok(Box::new(candle))
    }
    fn info(&self) -> BackendInfo {
        crate::backend::registry::CandleFactory.info()
    }
}

struct Cancellation(AtomicBool);
impl crate::InferenceExecutionCancellationSignal for Cancellation {
    fn snapshot(&self) -> crate::InferenceExecutionCancellationSnapshot {
        if self.0.load(Ordering::Acquire) {
            crate::InferenceExecutionCancellationSnapshot::cancellation_requested(Some(
                "test cancellation".into(),
            ))
        } else {
            crate::InferenceExecutionCancellationSnapshot::running()
        }
    }
}

async fn resident() -> (tempfile::TempDir, InferenceGateway, Arc<AtomicUsize>) {
    let (directory, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    let mut candle = CandleBackend::new();
    candle
        .load_selected_embedding(&request, &target, &decision)
        .await
        .unwrap();
    let stops = Arc::new(AtomicUsize::new(0));
    let gateway = InferenceGateway::with_backend(
        Box::new(Resident {
            candle,
            stops: stops.clone(),
        }),
        "fixture-resident",
    );
    *gateway.current_runtime_config.write().await = Some(BackendConfig {
        model_name: Some(target.model_ref.model_id),
        ..Default::default()
    });
    gateway.runtime_lifecycle.write().await.runtime_instance_id = Some("resident-A".into());
    gateway.runtime_lifecycle.write().await.active = true;
    (directory, gateway, stops)
}

async fn assert_resident(gateway: &InferenceGateway, stops: &AtomicUsize) {
    assert_eq!(stops.load(Ordering::SeqCst), 0);
    assert_eq!(gateway.current_backend_name().await, "fixture-resident");
    assert_eq!(
        gateway
            .runtime_lifecycle
            .read()
            .await
            .runtime_instance_id
            .as_deref(),
        Some("resident-A")
    );
    assert_eq!(
        gateway
            .current_runtime_config
            .read()
            .await
            .as_ref()
            .unwrap()
            .model_name
            .as_deref(),
        Some("embedding/test/synthetic-bert-8")
    );
    assert_eq!(
        gateway.embeddings(vec!["hello".into()], "").await.unwrap()[0]
            .vector
            .len(),
        8
    );
}

#[tokio::test]
async fn backend_and_gateway_refuse_other_model_without_changing_real_residency() {
    let (_a, request, target, decision) = crate::selected_embedding_execution::fixture(8);
    let (_b, request_b, target_b, decision_b) = crate::selected_embedding_execution::fixture(12);
    let mut backend = CandleBackend::new();
    backend
        .load_selected_embedding(&request, &target, &decision)
        .await
        .unwrap();
    let baseline = backend.embeddings(vec!["hello".into()], "").await.unwrap()[0]
        .vector
        .clone();
    for name in [&target_b.model_ref.model_id, "synthetic-bert-8", " "] {
        assert!(matches!(
            backend.embeddings(vec!["hello".into()], name).await,
            Err(BackendError::Config(_))
        ));
    }
    for name in [
        target.model_ref.model_id.clone(),
        format!("pumas://models/{}", target.model_ref.model_id),
        String::new(),
    ] {
        assert_eq!(
            backend
                .embeddings(vec!["hello".into()], &name)
                .await
                .unwrap()[0]
                .vector,
            baseline
        );
    }
    let gateway = InferenceGateway::with_backend(Box::new(backend), "Candle");
    assert!(matches!(
        gateway
            .embeddings(vec!["hello".into()], &target_b.model_ref.model_id)
            .await,
        Err(GatewayError::Backend(BackendError::Config(_)))
    ));
    assert_eq!(
        gateway
            .embeddings(vec!["hello".into()], &target.model_ref.model_id)
            .await
            .unwrap()[0]
            .vector,
        baseline
    );
    gateway
        .execute_selected_embedding_with_cancellation(
            request_b,
            target_b.clone(),
            decision_b,
            InferenceExecutionCancellationHandle::running(),
        )
        .await
        .unwrap();
    let instance = gateway
        .runtime_lifecycle
        .read()
        .await
        .runtime_instance_id
        .clone();
    assert!(gateway
        .embeddings(vec!["hello".into()], &target.model_ref.model_id)
        .await
        .is_err());
    assert_eq!(
        gateway.runtime_lifecycle.read().await.runtime_instance_id,
        instance
    );
    assert_eq!(
        gateway
            .embeddings(vec!["hello".into()], &target_b.model_ref.model_id)
            .await
            .unwrap()[0]
            .vector
            .len(),
        12
    );
    assert_eq!(
        gateway.embeddings(vec!["hello".into()], "").await.unwrap()[0]
            .vector
            .len(),
        12
    );
    gateway.stop().await.unwrap();
}

#[tokio::test]
async fn failed_real_cross_backend_candidate_retains_resident_and_metadata() {
    let (_a, gateway, stops) = resident().await;
    let (b, request, target, decision) = crate::selected_embedding_execution::fixture(12);
    std::fs::write(b.path().join("model.safetensors"), b"invalid safetensors").unwrap();
    assert!(gateway
        .execute_selected_embedding_with_cancellation(
            request,
            target,
            decision,
            InferenceExecutionCancellationHandle::running()
        )
        .await
        .is_err());
    assert_resident(&gateway, &stops).await;
    gateway.stop().await.unwrap();
}

#[tokio::test]
async fn cancellation_before_publication_retains_resident_and_dropped_caller_retains_real_load_join(
) {
    for (drop_caller, stop_after_drop) in [(false, false), (true, true), (true, false)] {
        let (_a, mut gateway, stops) = resident().await;
        let (_b, request, target, decision) = crate::selected_embedding_execution::fixture(12);
        let entered = Arc::new(tokio::sync::Notify::new());
        let (release, receive) = std::sync::mpsc::channel();
        let receive = Arc::new(Mutex::new(receive));
        let first_load = AtomicBool::new(true);
        gateway.registry.register(
            "Candle",
            Box::new(Factory {
                hook: Arc::new({
                    let entered = entered.clone();
                    move || {
                        if first_load.swap(false, Ordering::AcqRel) {
                            entered.notify_one();
                            receive.lock().unwrap().recv().unwrap();
                        }
                    }
                }),
            }),
        );
        let gateway = Arc::new(gateway);
        let cancellation = Arc::new(Cancellation(AtomicBool::new(false)));
        let mut caller = tokio::spawn({
            let gateway = gateway.clone();
            let cancellation = cancellation.clone();
            async move {
                gateway
                    .execute_selected_embedding_with_cancellation(
                        request,
                        target,
                        decision,
                        InferenceExecutionCancellationHandle::with_signal(cancellation),
                    )
                    .await
            }
        });
        tokio::time::timeout(std::time::Duration::from_secs(5), entered.notified())
            .await
            .expect("native hook entry");
        if drop_caller {
            caller.abort();
            assert!((&mut caller).await.unwrap_err().is_cancelled());
        } else {
            cancellation.0.store(true, Ordering::Release);
        }
        assert_eq!(stops.load(Ordering::SeqCst), 0);
        let drained = tokio::spawn({
            let gateway = gateway.clone();
            async move {
                if drop_caller && stop_after_drop {
                    gateway.stop().await
                } else if drop_caller {
                    let (_directory, request, target, decision) =
                        crate::selected_embedding_execution::fixture(8);
                    gateway
                        .execute_selected_embedding_with_cancellation(
                            request,
                            target,
                            decision,
                            InferenceExecutionCancellationHandle::running(),
                        )
                        .await
                        .map(|_| ())
                } else {
                    gateway.embedding_replacement.drain().await
                }
            }
        });
        tokio::task::yield_now().await;
        assert!(
            !drained.is_finished(),
            "custody must await the actual held blocking worker"
        );
        release.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), drained)
            .await
            .expect("retained supervisor completion")
            .unwrap()
            .unwrap();
        if !drop_caller {
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(5), caller)
                    .await
                    .expect("caller completion")
                    .unwrap()
                    .is_err()
            );
        }
        if drop_caller && stop_after_drop {
            assert!(!gateway.is_ready().await);
            assert_eq!(stops.load(Ordering::SeqCst), 1);
        } else if drop_caller {
            assert_eq!(stops.load(Ordering::SeqCst), 1);
            assert_eq!(gateway.current_backend_name().await, "Candle");
            assert_eq!(
                gateway.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                    .vector
                    .len(),
                8
            );
            gateway.stop().await.unwrap();
        } else {
            assert_resident(&gateway, &stops).await;
            gateway.stop().await.unwrap();
        }
    }
}

#[tokio::test]
async fn late_cancel_or_caller_loss_completes_publication_before_stop() {
    for drop_caller in [false, true] {
        let (_a, gateway, stops) = resident().await;
        let (_b, request, target, decision) = crate::selected_embedding_execution::fixture(12);
        let published = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        *gateway
            .embedding_replacement
            .after_publication
            .lock()
            .unwrap() = Some(Arc::new({
            let published = published.clone();
            let release = release.clone();
            move || {
                let release = release.clone();
                published.notify_one();
                async move { release.notified().await }.boxed()
            }
        }));
        let gateway = Arc::new(gateway);
        let cancellation = Arc::new(Cancellation(AtomicBool::new(false)));
        let mut caller = tokio::spawn({
            let gateway = gateway.clone();
            let cancellation = cancellation.clone();
            async move {
                gateway
                    .execute_selected_embedding_with_cancellation(
                        request,
                        target,
                        decision,
                        InferenceExecutionCancellationHandle::with_signal(cancellation),
                    )
                    .await
            }
        });
        published.notified().await;
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        if drop_caller {
            caller.abort();
            assert!((&mut caller).await.unwrap_err().is_cancelled());
        } else {
            cancellation.0.store(true, Ordering::Release);
        }
        let drained = tokio::spawn({
            let gateway = gateway.clone();
            async move {
                if drop_caller {
                    gateway.stop().await
                } else {
                    gateway.embedding_replacement.drain().await
                }
            }
        });
        tokio::task::yield_now().await;
        assert!(
            !drained.is_finished(),
            "publication remains owned after caller loss"
        );
        release.notify_one();
        tokio::time::timeout(std::time::Duration::from_secs(5), drained)
            .await
            .expect("retained supervisor completion")
            .unwrap()
            .unwrap();
        if !drop_caller {
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(5), caller)
                    .await
                    .expect("caller completion")
                    .unwrap()
                    .is_err()
            );
        }
        assert_eq!(gateway.current_backend_name().await, "Candle");
        if drop_caller {
            assert!(!gateway.is_ready().await);
            assert!(!gateway.runtime_lifecycle.read().await.active);
            assert_eq!(stops.load(Ordering::SeqCst), 1);
            continue;
        }

        assert_eq!(
            gateway
                .current_runtime_config
                .read()
                .await
                .as_ref()
                .unwrap()
                .model_name
                .as_deref(),
            Some("embedding/test/synthetic-bert-12")
        );
        assert!(gateway.runtime_lifecycle.read().await.active);
        assert_eq!(
            gateway.embeddings(vec!["hello".into()], "").await.unwrap()[0]
                .vector
                .len(),
            12
        );
        gateway.stop().await.unwrap();
        assert_eq!(stops.load(Ordering::SeqCst), 1);
    }
}
