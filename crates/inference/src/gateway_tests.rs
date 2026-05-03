use super::*;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{stream, StreamExt};
use tokio::sync::mpsc;

use crate::backend::BackendStartOutcome;
use crate::model_contracts::InferenceTaskId;
use crate::types::{
    ImageGenerationRequest, InferenceExecutionInput, InferenceExecutionRequest,
    InferenceExecutionResult, InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, RuntimeFactReadiness,
};

#[path = "gateway_tests/start_config.rs"]
mod start_config;

struct MockImageBackend;
struct MockHttpBackend;
struct MockReusedBackend;
struct MockImplicitLifecycleBackend;
struct MockFailingBackend;
struct MockFailAfterFirstStartBackend {
    starts: usize,
    ready: bool,
}
struct MockLifecycleStreamBackend {
    fail_on_stream: bool,
}
struct MockKvBackend;

#[derive(Default)]
struct RecordingLifecycleSink {
    events: Mutex<Vec<InferenceRequestLifecycleEvent>>,
}

impl RecordingLifecycleSink {
    fn events(&self) -> Vec<InferenceRequestLifecycleEvent> {
        self.events.lock().expect("events lock").clone()
    }
}

impl InferenceRequestLifecycleEventSink for RecordingLifecycleSink {
    fn record(&self, event: InferenceRequestLifecycleEvent) {
        self.events.lock().expect("events lock").push(event);
    }
}

struct MockProcessHandle;

impl crate::process::ProcessHandle for MockProcessHandle {
    fn pid(&self) -> u32 {
        1
    }

    fn kill(&self) -> Result<(), String> {
        Ok(())
    }
}

struct MockProcessSpawner;

#[async_trait]
impl ProcessSpawner for MockProcessSpawner {
    async fn spawn_sidecar(
        &self,
        _sidecar_name: &str,
        _args: &[&str],
    ) -> Result<
        (
            mpsc::Receiver<crate::process::ProcessEvent>,
            Box<dyn crate::process::ProcessHandle>,
        ),
        String,
    > {
        let (_tx, rx) = mpsc::channel(1);
        Ok((rx, Box::new(MockProcessHandle)))
    }

    fn app_data_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }

    fn binaries_dir(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/tmp"))
    }
}

#[async_trait]
impl InferenceBackend for MockImageBackend {
    fn name(&self) -> &'static str {
        "Mock"
    }

    fn description(&self) -> &'static str {
        "Mock image backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            image_generation: true,
            external_connection: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Ok(ImageGenerationResult {
            images: vec![crate::types::EncodedImage {
                data_base64: request.prompt,
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(7),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockHttpBackend {
    fn name(&self) -> &'static str {
        "MockHttp"
    }

    fn description(&self) -> &'static str {
        "Mock HTTP backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            external_connection: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: Some("started_http_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        Some("http://127.0.0.1:11434".to_string())
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, BackendError> {
        Ok(ImageGenerationResult {
            images: vec![crate::types::EncodedImage {
                data_base64: request.prompt,
                mime_type: "image/png".to_string(),
                width: Some(512),
                height: Some(512),
            }],
            seed_used: Some(11),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockReusedBackend {
    fn name(&self) -> &'static str {
        "MockReused"
    }

    fn description(&self) -> &'static str {
        "Mock reused backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(true),
            lifecycle_decision_reason: Some("reused_mock_runtime".to_string()),
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockImplicitLifecycleBackend {
    fn name(&self) -> &'static str {
        "MockImplicitLifecycle"
    }

    fn description(&self) -> &'static str {
        "Mock backend without explicit lifecycle reasons"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome {
            runtime_reused: Some(false),
            lifecycle_decision_reason: None,
        })
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockFailingBackend {
    fn name(&self) -> &'static str {
        "MockFailing"
    }

    fn description(&self) -> &'static str {
        "Mock backend that fails to start"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Err(BackendError::StartupFailed(
            "mock start failure".to_string(),
        ))
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        false
    }

    async fn health_check(&self) -> bool {
        false
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockFailAfterFirstStartBackend {
    fn name(&self) -> &'static str {
        "MockFailAfterFirstStart"
    }

    fn description(&self) -> &'static str {
        "Mock backend that fails after the first successful start"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            external_connection: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        self.starts += 1;
        if self.starts == 1 {
            self.ready = true;
            return Ok(BackendStartOutcome {
                runtime_reused: Some(false),
                lifecycle_decision_reason: Some("started_flaky_runtime".to_string()),
            });
        }

        self.ready = false;
        Err(BackendError::StartupFailed(
            "mock restart failure".to_string(),
        ))
    }

    fn stop(&mut self) {
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }

    async fn health_check(&self) -> bool {
        self.ready
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockLifecycleStreamBackend {
    fn name(&self) -> &'static str {
        "MockLifecycleStream"
    }

    fn description(&self) -> &'static str {
        "Mock backend for lifecycle stream tests"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            streaming: true,
            ..BackendCapabilities::default()
        }
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome::default())
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        None
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        if self.fail_on_stream {
            return Ok(Box::pin(stream::iter(vec![Err(BackendError::Inference(
                "mock stream failure".to_string(),
            ))])));
        }

        Ok(Box::pin(stream::iter(vec![
            Ok(ChatChunk {
                content: Some("hello".to_string()),
                done: false,
            }),
            Ok(ChatChunk {
                content: None,
                done: true,
            }),
        ])))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockKvBackend {
    fn name(&self) -> &'static str {
        "MockKv"
    }

    fn description(&self) -> &'static str {
        "Mock backend with KV support"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
    }

    async fn start(
        &mut self,
        _config: &BackendConfig,
        _spawner: Arc<dyn ProcessSpawner>,
    ) -> Result<BackendStartOutcome, BackendError> {
        Ok(BackendStartOutcome::default())
    }

    fn stop(&mut self) {}

    fn is_ready(&self) -> bool {
        true
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn base_url(&self) -> Option<String> {
        Some("http://127.0.0.1:11434".to_string())
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::empty()))
    }

    async fn embeddings(
        &self,
        _texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(Vec::new())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }

    async fn kv_cache_runtime_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<KvCacheRuntimeFingerprint, BackendError> {
        Ok(KvCacheRuntimeFingerprint {
            runtime_id: "mock".to_string(),
            backend_key: "mock".to_string(),
            tokenizer_fingerprint: "tok".to_string(),
            prompt_format_fingerprint: Some("prompt".to_string()),
            runtime_build_fingerprint: Some("build".to_string()),
        })
    }

    async fn kv_cache_model_fingerprint(
        &self,
        _active_config: Option<&BackendConfig>,
    ) -> Result<ModelFingerprint, BackendError> {
        Ok(ModelFingerprint {
            model_id: "model".to_string(),
            config_hash: "cfg".to_string(),
        })
    }

    async fn save_kv_cache_slot(
        &self,
        _slot_id: u32,
        _path: &std::path::Path,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    async fn restore_kv_cache_slot(
        &self,
        _slot_id: u32,
        _path: &std::path::Path,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    async fn clear_kv_cache_slot(&self, _slot_id: u32) -> Result<(), BackendError> {
        Ok(())
    }

    async fn truncate_kv_cache_data(
        &self,
        data: &[u8],
        token_position: usize,
        _active_config: Option<&BackendConfig>,
    ) -> Result<Vec<u8>, BackendError> {
        Ok(data[..token_position.min(data.len())].to_vec())
    }
}

#[cfg(feature = "backend-llamacpp")]
#[test]
fn test_gateway_creation() {
    let gateway = InferenceGateway::new();
    // Registry should have at least llama.cpp
    assert!(!gateway.registry.list().is_empty());
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_initial_backend_is_llamacpp() {
    let gateway = InferenceGateway::new();
    let name = gateway.current_backend_name().await;
    assert_eq!(name, "llama.cpp");
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_switch_backend_normalizes_llamacpp_alias() {
    let gateway = InferenceGateway::new();

    gateway
        .switch_backend("llama_cpp")
        .await
        .expect("llama_cpp alias should resolve");

    assert_eq!(gateway.current_backend_name().await, "llama.cpp");
    assert_eq!(
        gateway
            .runtime_lifecycle_snapshot()
            .await
            .runtime_id
            .as_deref(),
        Some("llama_cpp")
    );
}

#[cfg(feature = "backend-pytorch")]
#[tokio::test]
async fn test_switch_backend_normalizes_pytorch_alias() {
    let gateway = InferenceGateway::new();

    gateway
        .switch_backend("pytorch")
        .await
        .expect("pytorch alias should resolve");

    assert_eq!(gateway.current_backend_name().await, "PyTorch");
    assert_eq!(
        gateway
            .runtime_lifecycle_snapshot()
            .await
            .runtime_id
            .as_deref(),
        Some("pytorch")
    );
}

#[cfg(feature = "backend-llamacpp")]
#[tokio::test]
async fn test_not_ready_initially() {
    let gateway = InferenceGateway::new();
    assert!(!gateway.is_ready().await);
}

#[tokio::test]
async fn test_generate_image_forwards_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let result = gateway
        .generate_image(ImageGenerationRequest {
            model: "mock".to_string(),
            prompt: "paper lantern".to_string(),
            negative_prompt: None,
            width: Some(512),
            height: Some(512),
            num_inference_steps: Some(20),
            guidance_scale: Some(4.0),
            seed: Some(7),
            scheduler: None,
            num_images_per_prompt: Some(1),
            init_image: None,
            mask_image: None,
            strength: None,
            extra_options: serde_json::Value::Null,
        })
        .await
        .unwrap();

    assert_eq!(result.seed_used, Some(7));
    assert_eq!(result.images.len(), 1);
    assert_eq!(result.images[0].data_base64, "paper lantern");
}

#[tokio::test]
async fn test_execute_typed_forwards_image_generation_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-image-1".to_string()),
        task_id: InferenceTaskId::ImageGeneration,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        runtime_hint: Some("mock".to_string()),
        input: InferenceExecutionInput::ImageGeneration {
            request: ImageGenerationRequest {
                model: "mock-image".to_string(),
                prompt: "typed prompt".to_string(),
                negative_prompt: None,
                width: None,
                height: None,
                num_inference_steps: None,
                guidance_scale: None,
                seed: None,
                scheduler: None,
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::Value::Null,
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let result = gateway
        .execute_typed(request)
        .await
        .expect("typed image request should execute");

    match result {
        InferenceExecutionResult::ImageGeneration { result } => {
            assert_eq!(result.images[0].data_base64, "typed prompt");
            assert_eq!(result.seed_used, Some(7));
        }
        other => panic!("expected image generation result, got {other:?}"),
    }
}

#[tokio::test]
async fn test_execute_typed_validates_before_backend_execution() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-invalid-1".to_string()),
        task_id: InferenceTaskId::Embedding,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        runtime_hint: Some("mock".to_string()),
        input: InferenceExecutionInput::ImageGeneration {
            request: ImageGenerationRequest {
                model: "mock-image".to_string(),
                prompt: "typed prompt".to_string(),
                negative_prompt: None,
                width: None,
                height: None,
                num_inference_steps: None,
                guidance_scale: None,
                seed: None,
                scheduler: None,
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::Value::Null,
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    match gateway.execute_typed(request).await {
        Err(GatewayError::Validation(_)) => {}
        other => panic!("expected validation error, got {other:?}"),
    }
}

#[tokio::test]
async fn test_rerank_forwards_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let result = gateway
        .rerank(RerankRequest {
            model: "mock".to_string(),
            query: "alpha".to_string(),
            documents: vec!["a".to_string()],
            top_n: Some(1),
            return_documents: true,
            extra_options: serde_json::Value::Null,
        })
        .await
        .expect("rerank should forward");
    assert!(result.results.is_empty());
}

#[tokio::test]
async fn test_chat_completion_stream_with_lifecycle_records_completion() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let mut stream = gateway
        .chat_completion_stream_with_lifecycle(
            "{}".to_string(),
            Some("req-complete".to_string()),
            sink.clone(),
        )
        .await
        .expect("stream should start");

    while stream.next().await.is_some() {}

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events.iter().all(|event| {
        event.request_id.as_deref() == Some("req-complete")
            && event.backend_key.as_deref() == Some("mock")
    }));
}

#[tokio::test]
async fn test_chat_completion_stream_with_lifecycle_records_stream_failure() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: true,
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let mut stream = gateway
        .chat_completion_stream_with_lifecycle(
            "{}".to_string(),
            Some("req-fail".to_string()),
            sink.clone(),
        )
        .await
        .expect("stream should start");

    let result = stream.next().await.expect("stream item");
    assert!(matches!(
        result,
        Err(BackendError::Inference(message)) if message.contains("mock stream failure")
    ));

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(
        events[1].detail.as_deref(),
        Some("Inference error: mock stream failure")
    );
}

#[tokio::test]
async fn test_chat_completion_stream_with_lifecycle_records_drop_cancellation() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let stream = gateway
        .chat_completion_stream_with_lifecycle(
            "{}".to_string(),
            Some("req-cancel".to_string()),
            sink.clone(),
        )
        .await
        .expect("stream should start");
    drop(stream);

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Cancelled
    );
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(
        events[1].detail.as_deref(),
        Some("stream dropped before completion")
    );
}

#[tokio::test]
async fn test_rerank_with_lifecycle_records_completion() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());

    gateway
        .rerank_with_lifecycle(
            RerankRequest {
                model: "mock".to_string(),
                query: "alpha".to_string(),
                documents: vec!["a".to_string()],
                top_n: Some(1),
                return_documents: true,
                extra_options: serde_json::Value::Null,
            },
            Some("req-rerank".to_string()),
            sink.clone(),
        )
        .await
        .expect("rerank should complete");

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
}

#[tokio::test]
async fn test_generate_image_with_lifecycle_records_failure() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let error = gateway
        .generate_image_with_lifecycle(
            ImageGenerationRequest {
                model: "mock".to_string(),
                prompt: "prompt".to_string(),
                negative_prompt: None,
                width: None,
                height: None,
                num_inference_steps: None,
                guidance_scale: None,
                seed: None,
                scheduler: None,
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::Value::Null,
            },
            Some("req-image".to_string()),
            sink.clone(),
        )
        .await
        .expect_err("image generation should be unsupported");

    assert!(matches!(
        error,
        GatewayError::Backend(BackendError::Inference(message))
            if message.contains("Image generation not supported")
    ));
    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events[1]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("Image generation not supported")));
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_tracks_start_and_stop() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let started = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(started.runtime_id.as_deref(), Some("mock"));
    assert!(started.runtime_instance_id.is_some());
    assert!(started.warmup_started_at_ms.is_some());
    assert!(started.warmup_completed_at_ms.is_some());
    assert!(started.warmup_duration_ms.is_some());
    assert_eq!(started.runtime_reused, Some(false));
    assert_eq!(
        started.lifecycle_decision_reason.as_deref(),
        Some("started_mock_runtime")
    );
    assert!(started.active);
    assert!(started.last_error.is_none());

    gateway.stop().await;

    let stopped = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(stopped.runtime_id.as_deref(), Some("mock"));
    assert!(!stopped.active);
    assert_eq!(
        stopped.lifecycle_decision_reason.as_deref(),
        Some("runtime_stopped")
    );
    assert_eq!(
        stopped.runtime_fact_readiness(),
        RuntimeFactReadiness::Stopped
    );

    let facts = gateway.mode_info().await.runtime_fact_snapshots();
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].readiness, RuntimeFactReadiness::Stopped);
    assert_eq!(
        facts[0].lifecycle_decision_reason.as_deref(),
        Some("runtime_stopped")
    );
    assert_eq!(facts[0].absence_reason, None);
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_preserves_instance_id_for_reused_runtime() {
    let gateway = InferenceGateway::with_backend(Box::new(MockReusedBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");
    let first = gateway.runtime_lifecycle_snapshot().await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should reuse");
    let second = gateway.runtime_lifecycle_snapshot().await;

    assert_eq!(first.runtime_id.as_deref(), Some("mock"));
    assert_eq!(second.runtime_id.as_deref(), Some("mock"));
    assert_eq!(second.runtime_reused, Some(true));
    assert_eq!(second.runtime_instance_id, first.runtime_instance_id);
    assert_eq!(
        second.lifecycle_decision_reason.as_deref(),
        Some("reused_mock_runtime")
    );
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_normalizes_missing_start_reason() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImplicitLifecycleBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");

    let snapshot = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(snapshot.runtime_reused, Some(false));
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_ready")
    );
}

#[tokio::test]
async fn test_runtime_lifecycle_snapshot_normalizes_start_failure_reason() {
    let gateway = InferenceGateway::with_backend(Box::new(MockFailingBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    let error = gateway.start(&BackendConfig::default()).await;
    assert!(error.is_err());

    let snapshot = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(snapshot.runtime_reused, None);
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
    assert_eq!(
        snapshot.last_error.as_deref(),
        Some("Startup failed: mock start failure")
    );

    gateway.stop().await;

    let stopped = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(
        stopped.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
    assert_eq!(
        stopped.last_error.as_deref(),
        Some("Startup failed: mock start failure")
    );
}

#[tokio::test]
async fn test_failed_restart_clears_active_runtime_config_and_attempted_modes() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockFailAfterFirstStartBackend {
            starts: 0,
            ready: false,
        }),
        "mock",
    );
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    let successful_config = BackendConfig {
        model_path: Some(PathBuf::from("/models/previous.gguf")),
        ..BackendConfig::default()
    };
    gateway
        .start(&successful_config)
        .await
        .expect("initial start should succeed");
    assert_eq!(
        gateway
            .restart_runtime_config()
            .await
            .and_then(|config| config.model_path),
        Some(PathBuf::from("/models/previous.gguf"))
    );

    let failed_restart = gateway
        .start(&BackendConfig {
            external_url: Some("http://127.0.0.1:9999".to_string()),
            ..BackendConfig::default()
        })
        .await;

    assert!(failed_restart.is_err());
    assert!(gateway.restart_runtime_config().await.is_none());
    assert!(!gateway.is_embedding_mode().await);
    assert!(!gateway.is_reranking_mode().await);
    assert!(!gateway.is_external_mode().await);
    assert_eq!(
        gateway
            .last_inference_config()
            .await
            .and_then(|config| config.model_path),
        Some(PathBuf::from("/models/previous.gguf"))
    );

    let snapshot = gateway.runtime_lifecycle_snapshot().await;
    assert_eq!(
        snapshot.lifecycle_decision_reason.as_deref(),
        Some("runtime_start_failed")
    );
    assert_eq!(
        snapshot.last_error.as_deref(),
        Some("Startup failed: mock restart failure")
    );

    let facts = gateway.mode_info().await.runtime_fact_snapshots();
    assert_eq!(facts[0].readiness, RuntimeFactReadiness::Failed);
    assert_eq!(
        facts[0].last_backend_error.as_deref(),
        Some("Startup failed: mock restart failure")
    );
}

#[tokio::test]
async fn test_mode_info_reports_external_runtime_from_start_config() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig {
            external_url: Some("http://127.0.0.1:1234".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start");

    let mode = gateway.mode_info().await;
    assert_eq!(mode.backend_name.as_deref(), Some("mock"));
    assert_eq!(mode.backend_key.as_deref(), Some("mock"));
    assert_eq!(mode.mode, "external");
    assert!(!mode.is_embedding_mode);
    assert_eq!(
        mode.active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_id.as_deref()),
        Some("mock")
    );
    assert_eq!(
        mode.active_runtime
            .as_ref()
            .and_then(|snapshot| snapshot.runtime_reused),
        Some(false)
    );
    assert_eq!(mode.embedding_runtime, None);
}

#[tokio::test]
async fn test_mode_info_preserves_selected_backend_after_stop() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");
    gateway.stop().await;

    let mode = gateway.mode_info().await;

    assert_eq!(mode.backend_name.as_deref(), Some("mock"));
    assert_eq!(mode.backend_key.as_deref(), Some("mock"));
    assert!(mode.active_runtime.is_some());
}

#[tokio::test]
async fn test_kv_gateway_methods_delegate_to_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockKvBackend), "mock-kv");

    let runtime = gateway
        .kv_cache_runtime_fingerprint()
        .await
        .expect("runtime fingerprint should be available");
    assert_eq!(runtime.runtime_id, "mock");

    let model = gateway
        .kv_cache_model_fingerprint()
        .await
        .expect("model fingerprint should be available");
    assert_eq!(model.model_id, "model");

    let path = std::path::Path::new("/tmp/mock-slot.bin");
    gateway
        .save_kv_cache_slot(0, path)
        .await
        .expect("save should delegate to backend");
    gateway
        .restore_kv_cache_slot(0, path)
        .await
        .expect("restore should delegate to backend");
    gateway
        .clear_kv_cache_slot(0)
        .await
        .expect("clear should delegate to backend");
    let truncated = gateway
        .truncate_kv_cache_data(&[1, 2, 3, 4], 2)
        .await
        .expect("truncate should delegate to backend");
    assert_eq!(truncated, vec![1, 2]);
}

#[tokio::test]
async fn test_mode_info_reports_active_model_target() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "llama.cpp");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig {
            model_path: Some(PathBuf::from("/models/vision.gguf")),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start");

    let mode = gateway.mode_info().await;

    assert_eq!(mode.backend_name.as_deref(), Some("llama.cpp"));
    assert_eq!(mode.backend_key.as_deref(), Some("llama_cpp"));
    assert_eq!(
        mode.active_model_target.as_deref(),
        Some("/models/vision.gguf")
    );
    assert_eq!(mode.embedding_model_target, None);
}
