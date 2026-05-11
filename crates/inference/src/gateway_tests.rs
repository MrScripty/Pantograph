use super::*;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures_util::{stream, StreamExt};
use tokio::sync::mpsc;

use crate::backend::BackendStartOutcome;
use crate::config::DeviceConfig;
use crate::model_contracts::{
    CacheGenerationOptions, GenerationOptions, InferenceLifecyclePhase, InferenceTaskId,
    LengthGenerationOptions, OptionSupportState, ResolvedModelPackageFacts,
    SamplingGenerationOptions, StoppingGenerationOptions,
};
use crate::runtime_load::{LlamaCppActiveRuntimeDescriptor, LlamaCppRuntimeMode};
use crate::types::{
    AudioTranscriptionRequest, AudioTranscriptionResult, DepthEstimationRequest, EncodedAudio,
    ImageGenerationRequest, ImageUnderstandingRequest, InferenceExecutionInput,
    InferenceExecutionRequest, InferenceExecutionResult, InferenceRequestLifecycleEvent,
    InferenceRequestLifecycleEventKind, InferenceRequestLifecycleEventSink,
    InferenceRequestLifecycleEventSinkError, InferenceUsage, MultimodalGenerationRequest,
    MultimodalInputPart, RuntimeFactReadiness, VideoUnderstandingRequest,
};
use crate::{InferenceDeviceClass, InferenceDeviceId};

#[path = "gateway_tests/start_config.rs"]
mod start_config;

struct MockImageBackend;
struct MockActiveLlamaBackend;
struct MockHttpBackend;
struct MockReusedBackend;
struct MockImplicitLifecycleBackend;
struct MockFailingBackend;
struct MockFailAfterFirstStartBackend {
    starts: usize,
    ready: bool,
}
#[derive(Default)]
struct MockLifecycleStreamBackend {
    fail_on_stream: bool,
    usage_on_terminal: Option<InferenceUsage>,
    cache_handle_on_terminal: Option<String>,
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
    fn record(
        &self,
        event: InferenceRequestLifecycleEvent,
    ) -> Result<(), InferenceRequestLifecycleEventSinkError> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
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
        Ok(Box::pin(stream::iter([
            Ok(ChatChunk {
                content: Some("mock text".to_string()),
                done: false,
                usage: None,
                cache_handle_id: None,
            }),
            Ok(ChatChunk {
                content: None,
                done: true,
                usage: Some(InferenceUsage {
                    prompt_tokens: Some(3),
                    completion_tokens: Some(2),
                    total_tokens: Some(5),
                }),
                cache_handle_id: Some("kv-mock-text".to_string()),
            }),
        ])))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(texts
            .into_iter()
            .map(|text| EmbeddingResult {
                vector: vec![text.len() as f32],
                token_count: text.split_whitespace().count().max(1),
            })
            .collect())
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

    async fn transcribe_audio(
        &self,
        request: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResult, BackendError> {
        Ok(AudioTranscriptionResult {
            text: format!("transcribed {}", request.model),
            language: request.language,
            duration_seconds: Some(1.5),
            segments: Vec::new(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[async_trait]
impl InferenceBackend for MockActiveLlamaBackend {
    fn name(&self) -> &'static str {
        "Mock llama.cpp"
    }

    fn description(&self) -> &'static str {
        "Mock llama.cpp backend with active runtime facts"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            embeddings: true,
            device_selection: true,
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
            lifecycle_decision_reason: Some("started_mock_llama_runtime".to_string()),
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

    fn active_llamacpp_runtime_descriptor(&self) -> Option<LlamaCppActiveRuntimeDescriptor> {
        Some(LlamaCppActiveRuntimeDescriptor {
            mode: LlamaCppRuntimeMode::Embedding,
            port: 11434,
            model_path: PathBuf::from("/models/embed.gguf"),
            mmproj_path: None,
            device: DeviceConfig {
                device: "CUDA0".to_string(),
                gpu_layers: 40,
            },
            selected_device_class: Some(InferenceDeviceClass::Cuda),
            selected_device_id: Some(InferenceDeviceId::parse("cuda:0").expect("valid cuda id")),
            context_size: None,
            cpu_threads: None,
            batch_size: None,
            ubatch_size: None,
        })
    }

    async fn chat_completion_stream(
        &self,
        _request_json: String,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatChunk, BackendError>> + Send>>, BackendError>
    {
        Ok(Box::pin(stream::iter([Ok(ChatChunk {
            content: None,
            done: true,
            usage: None,
            cache_handle_id: None,
        })])))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        _model: &str,
    ) -> Result<Vec<EmbeddingResult>, BackendError> {
        Ok(texts
            .into_iter()
            .map(|text| EmbeddingResult {
                vector: vec![text.len() as f32],
                token_count: text.split_whitespace().count().max(1),
            })
            .collect())
    }

    async fn rerank(&self, _request: RerankRequest) -> Result<RerankResponse, BackendError> {
        Ok(RerankResponse {
            results: Vec::new(),
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
                usage: None,
                cache_handle_id: None,
            }),
            Ok(ChatChunk {
                content: None,
                done: true,
                usage: self.usage_on_terminal.clone(),
                cache_handle_id: self.cache_handle_on_terminal.clone(),
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
async fn test_transcribe_audio_forwards_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let result = gateway
        .transcribe_audio(AudioTranscriptionRequest {
            model: "mock-asr".to_string(),
            audio: None,
            audio_ref: Some("artifact://audio.wav".to_string()),
            language: Some("en".to_string()),
            prompt: None,
            task: Some("transcribe".to_string()),
            chunk_length_s: None,
            extra_options: serde_json::Value::Null,
        })
        .await
        .unwrap();

    assert_eq!(result.text, "transcribed mock-asr");
    assert_eq!(result.language.as_deref(), Some("en"));
    assert_eq!(result.duration_seconds, Some(1.5));
}

#[tokio::test]
async fn test_execute_typed_forwards_image_generation_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-image-1".to_string()),
        task_id: InferenceTaskId::ImageGeneration,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::ImageGeneration {
            request: ImageGenerationRequest {
                model: "mock-image".to_string(),
                prompt: "typed prompt".to_string(),
                negative_prompt: None,
                width: Some(512),
                height: Some(512),
                num_inference_steps: None,
                guidance_scale: None,
                seed: Some(123),
                scheduler: Some("euler".to_string()),
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::json!({
                    "safety_checker": false,
                }),
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
        InferenceExecutionResult::ImageGeneration {
            result,
            option_diagnostics,
        } => {
            assert_eq!(result.images[0].data_base64, "typed prompt");
            assert_eq!(result.seed_used, Some(7));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "image.width"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "image.scheduler"
                    && diagnostic.state == OptionSupportState::Honored
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "image.extra_options.safety_checker"
                    && diagnostic.state == OptionSupportState::Mapped
            }));
        }
        other => panic!("expected image generation result, got {other:?}"),
    }
}

#[tokio::test]
async fn test_execute_typed_forwards_audio_transcription_to_active_backend() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-audio-1".to_string()),
        task_id: InferenceTaskId::AudioTranscription,
        model_ref: None,
        model_name: Some("mock-asr".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::AudioTranscription {
            request: AudioTranscriptionRequest {
                model: "mock-asr".to_string(),
                audio: None,
                audio_ref: Some("artifact://audio.wav".to_string()),
                language: Some("en".to_string()),
                prompt: Some("domain hint".to_string()),
                task: Some("transcribe".to_string()),
                chunk_length_s: Some(30.0),
                extra_options: serde_json::json!({
                    "return_timestamps": true,
                }),
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let result = gateway
        .execute_typed(request)
        .await
        .expect("typed audio request should execute");

    match result {
        InferenceExecutionResult::AudioTranscription {
            result,
            option_diagnostics,
        } => {
            assert_eq!(result.text, "transcribed mock-asr");
            assert_eq!(result.language.as_deref(), Some("en"));
            assert_eq!(result.duration_seconds, Some(1.5));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "audio_transcription.language"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "audio_transcription.prompt"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "audio_transcription.task"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "audio_transcription.chunk_length_s"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "audio_transcription.extra_options.return_timestamps"
                    && diagnostic.state == OptionSupportState::Mapped
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
        }
        other => panic!("expected audio transcription result, got {other:?}"),
    }
}

#[tokio::test]
async fn test_execute_typed_embedding_returns_task_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-embedding-result-options".to_string()),
        task_id: InferenceTaskId::Embedding,
        model_ref: None,
        model_name: Some("mock-embedding".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::Embedding {
            texts: vec!["alpha beta".to_string()],
        },
        generation_options: None,
        extra_options: serde_json::json!({
            "normalize": true
        }),
    };

    let result = gateway
        .execute_typed(request)
        .await
        .expect("typed embedding request should execute");

    match result {
        InferenceExecutionResult::Embedding {
            embeddings,
            usage,
            option_diagnostics,
        } => {
            assert_eq!(embeddings.len(), 1);
            assert_eq!(usage.and_then(|usage| usage.total_tokens), Some(2));
            assert!(option_diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.option_path == "extra_options.normalize"
                        && diagnostic.state == OptionSupportState::Mapped
                        && diagnostic.backend_key.as_deref() == Some("mock")
                ));
        }
        other => panic!("expected embedding result, got {other:?}"),
    }
}

#[tokio::test]
async fn test_execute_typed_rerank_returns_task_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-rerank-result-options".to_string()),
        task_id: InferenceTaskId::Rerank,
        model_ref: None,
        model_name: Some("mock-rerank".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::Rerank {
            query: "alpha".to_string(),
            documents: vec!["a".to_string(), "b".to_string()],
            top_n: Some(1),
            return_documents: false,
        },
        generation_options: None,
        extra_options: serde_json::json!({
            "score_threshold": 0.25,
        }),
    };

    let result = gateway
        .execute_typed(request)
        .await
        .expect("typed rerank request should execute");

    match result {
        InferenceExecutionResult::Rerank {
            option_diagnostics, ..
        } => {
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "rerank.top_n"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "rerank.return_documents"
                    && diagnostic.state == OptionSupportState::Honored
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics
                .iter()
                .any(
                    |diagnostic| diagnostic.option_path == "extra_options.score_threshold"
                        && diagnostic.state == OptionSupportState::Mapped
                        && diagnostic.backend_key.as_deref() == Some("mock")
                ));
        }
        other => panic!("expected rerank result, got {other:?}"),
    }
}

#[tokio::test]
async fn test_execute_typed_text_reports_generation_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-text-1".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: Some("mock-text".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: Some(GenerationOptions {
            length: LengthGenerationOptions {
                max_new_tokens: Some(16),
                ..LengthGenerationOptions::default()
            },
            sampling: SamplingGenerationOptions {
                temperature: Some(0.25),
                seed: Some(42),
                ..SamplingGenerationOptions::default()
            },
            cache: CacheGenerationOptions {
                use_cache: Some(true),
                kv_cache_checkpoint_requested: Some(true),
            },
            stopping: StoppingGenerationOptions {
                stop_strings: vec!["END".to_string()],
                ..StoppingGenerationOptions::default()
            },
            ..GenerationOptions::default()
        }),
        extra_options: serde_json::Value::Null,
    };

    let result = gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed text request should execute");

    match result {
        InferenceExecutionResult::TextGeneration {
            option_diagnostics, ..
        } => {
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "length.max_new_tokens"
                    && diagnostic.state == OptionSupportState::Mapped
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "sampling.seed"
                    && diagnostic.state == OptionSupportState::Unsupported
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "stopping.stop_strings"
                    && diagnostic.state == OptionSupportState::Unsupported
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "cache.use_cache"
                    && diagnostic.state == OptionSupportState::RequiresBackendSupport
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
            assert!(option_diagnostics.iter().any(|diagnostic| {
                diagnostic.option_path == "cache.kv_cache_checkpoint_requested"
                    && diagnostic.state == OptionSupportState::Mapped
                    && diagnostic.backend_key.as_deref() == Some("mock")
            }));
        }
        other => panic!("expected text generation result, got {other:?}"),
    }

    let events = sink.events();
    let phase_kinds: Vec<(InferenceLifecyclePhase, InferenceRequestLifecycleEventKind)> = events
        .iter()
        .map(|event| (event.phase.clone(), event.kind.clone()))
        .collect();
    assert_eq!(
        phase_kinds,
        vec![
            (
                InferenceLifecyclePhase::TaskValidation,
                InferenceRequestLifecycleEventKind::Started
            ),
            (
                InferenceLifecyclePhase::TaskValidation,
                InferenceRequestLifecycleEventKind::Completed
            ),
            (
                InferenceLifecyclePhase::TaskValidation,
                InferenceRequestLifecycleEventKind::CleanupCompleted
            ),
            (
                InferenceLifecyclePhase::Preprocessing,
                InferenceRequestLifecycleEventKind::Started
            ),
            (
                InferenceLifecyclePhase::Preprocessing,
                InferenceRequestLifecycleEventKind::Completed
            ),
            (
                InferenceLifecyclePhase::Preprocessing,
                InferenceRequestLifecycleEventKind::CleanupCompleted
            ),
            (
                InferenceLifecyclePhase::BackendExecution,
                InferenceRequestLifecycleEventKind::Started
            ),
            (
                InferenceLifecyclePhase::BackendExecution,
                InferenceRequestLifecycleEventKind::Completed
            ),
            (
                InferenceLifecyclePhase::BackendExecution,
                InferenceRequestLifecycleEventKind::CleanupCompleted
            ),
            (
                InferenceLifecyclePhase::Postprocessing,
                InferenceRequestLifecycleEventKind::Started
            ),
            (
                InferenceLifecyclePhase::Postprocessing,
                InferenceRequestLifecycleEventKind::Completed
            ),
            (
                InferenceLifecyclePhase::Postprocessing,
                InferenceRequestLifecycleEventKind::CleanupCompleted
            ),
            (
                InferenceLifecyclePhase::ResultProjection,
                InferenceRequestLifecycleEventKind::Started
            ),
            (
                InferenceLifecyclePhase::ResultProjection,
                InferenceRequestLifecycleEventKind::Completed
            ),
            (
                InferenceLifecyclePhase::ResultProjection,
                InferenceRequestLifecycleEventKind::CleanupCompleted
            ),
        ]
    );
    let completed_validation_event = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("completed validation event");
    assert!(completed_validation_event
        .option_diagnostics
        .iter()
        .any(
            |diagnostic| diagnostic.option_path == "length.max_new_tokens"
                && diagnostic.state == OptionSupportState::Mapped
        ));
    assert!(completed_validation_event
        .option_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "sampling.seed"
            && diagnostic.state == OptionSupportState::Unsupported));
    assert!(completed_validation_event
        .option_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "cache.use_cache"
            && diagnostic.state == OptionSupportState::RequiresBackendSupport));
    let completed_backend_event = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("completed backend event");
    assert_eq!(
        completed_backend_event.task_id.as_deref(),
        Some("text_generation")
    );
    assert!(completed_backend_event
        .option_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "sampling.seed"
            && diagnostic.state == OptionSupportState::Unsupported));
    assert!(completed_backend_event
        .option_diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "cache.use_cache"
            && diagnostic.state == OptionSupportState::RequiresBackendSupport));
    assert!(completed_backend_event
        .option_diagnostics
        .iter()
        .any(
            |diagnostic| diagnostic.option_path == "cache.kv_cache_checkpoint_requested"
                && diagnostic.state == OptionSupportState::Mapped
        ));
}

#[tokio::test]
async fn test_execute_typed_text_lifecycle_reports_usage_from_terminal_chunk() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-text-usage".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: Some("mock-text".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("SECRET_PROMPT should not reach lifecycle diagnostics".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let result = gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed text request should execute");

    match result {
        InferenceExecutionResult::TextGeneration {
            text,
            usage,
            cache_handle_id,
            ..
        } => {
            assert_eq!(text, "mock text");
            let usage = usage.expect("terminal chunk usage should become typed result usage");
            assert_eq!(usage.prompt_tokens, Some(3));
            assert_eq!(usage.completion_tokens, Some(2));
            assert_eq!(usage.total_tokens, Some(5));
            assert_eq!(cache_handle_id.as_deref(), Some("kv-mock-text"));
        }
        other => panic!("expected text generation result, got {other:?}"),
    }

    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("completed backend event");
    assert_eq!(
        backend_completed
            .usage
            .as_ref()
            .and_then(|usage| usage.prompt_tokens),
        Some(3)
    );
    assert_eq!(
        backend_completed
            .usage
            .as_ref()
            .and_then(|usage| usage.completion_tokens),
        Some(2)
    );
    assert_eq!(
        backend_completed
            .usage
            .as_ref()
            .and_then(|usage| usage.total_tokens),
        Some(5)
    );
    assert_eq!(
        backend_completed.cache_handle_id.as_deref(),
        Some("kv-mock-text")
    );
    let lifecycle_json =
        serde_json::to_string(backend_completed).expect("lifecycle event should serialize");
    assert!(!lifecycle_json.contains("SECRET_PROMPT"));
    assert!(!lifecycle_json.contains("mock text"));
}

#[tokio::test]
async fn test_execute_typed_text_filters_path_shaped_cache_handle() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            usage_on_terminal: None,
            cache_handle_on_terminal: Some("/tmp/private/kv-cache.bin".to_string()),
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-text-cache-path".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: Some("mock-text".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let result = gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed text request should execute");

    match result {
        InferenceExecutionResult::TextGeneration {
            cache_handle_id, ..
        } => {
            assert!(cache_handle_id.is_none());
        }
        other => panic!("expected text generation result, got {other:?}"),
    }

    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("completed backend event");
    assert!(backend_completed.cache_handle_id.is_none());
    let lifecycle_json =
        serde_json::to_string(backend_completed).expect("lifecycle event should serialize");
    assert!(!lifecycle_json.contains("/tmp/private/kv-cache.bin"));
}

#[tokio::test]
async fn test_execute_typed_with_lifecycle_reports_package_compatibility() {
    let fixture = include_str!(
        "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-text-compatibility".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package_facts.model_ref.clone()),
        model_name: Some("mock-text".to_string()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed request should execute");

    let events = sink.events();
    let package_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::ModelPackageResolution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("model package resolution completion event");
    assert_eq!(
        package_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert_eq!(
        package_completed.resolved_artifact_kind.as_deref(),
        Some("gguf")
    );
    assert!(package_completed.compatibility_report.is_none());
    assert!(package_completed.option_diagnostics.is_empty());

    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion event");
    let compatibility_report = validation_completed
        .compatibility_report
        .as_ref()
        .expect("compatibility report");
    assert_eq!(compatibility_report.status, "rejected");
    assert!(!compatibility_report.compatible);
    assert!(!validation_completed.compatibility_issues.is_empty());
    assert!(validation_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("llm/llama/tiny-gguf")));

    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion event");
    let backend_compatibility_report = backend_completed
        .compatibility_report
        .as_ref()
        .expect("backend execution compatibility report");
    assert_eq!(backend_compatibility_report.status, "rejected");
    assert!(!backend_compatibility_report.compatible);
    assert!(!backend_completed.compatibility_issues.is_empty());
    assert!(backend_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("llm/llama/tiny-gguf")));
}

#[tokio::test]
async fn test_execute_typed_rerank_lifecycle_reports_task_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-rerank-options".to_string()),
        task_id: InferenceTaskId::Rerank,
        model_ref: None,
        model_name: Some("mock-rerank".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::Rerank {
            query: "alpha".to_string(),
            documents: vec!["a".to_string(), "b".to_string()],
            top_n: Some(1),
            return_documents: false,
        },
        generation_options: None,
        extra_options: serde_json::json!({
            "score_threshold": 0.25,
            "trace": true
        }),
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed rerank request should execute");

    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend completion event");
    let option_paths = backend_completed
        .option_diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.option_path.as_str(),
                diagnostic.state,
                diagnostic.backend_key.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    assert!(option_paths.contains(&("rerank.top_n", OptionSupportState::Honored, Some("mock"))));
    assert!(option_paths.contains(&(
        "rerank.return_documents",
        OptionSupportState::Honored,
        Some("mock")
    )));
    assert!(option_paths.contains(&(
        "extra_options.score_threshold",
        OptionSupportState::Mapped,
        Some("mock")
    )));
    assert!(option_paths.contains(&(
        "extra_options.trace",
        OptionSupportState::Mapped,
        Some("mock")
    )));
}

#[tokio::test]
async fn test_execute_typed_embedding_lifecycle_reports_extra_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-embedding-options".to_string()),
        task_id: InferenceTaskId::Embedding,
        model_ref: None,
        model_name: Some("mock-embedding".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::Embedding {
            texts: vec!["alpha".to_string()],
        },
        generation_options: None,
        extra_options: serde_json::json!({
            "normalize": true
        }),
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed embedding request should execute");

    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend completion event");
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(
            |diagnostic| diagnostic.option_path == "extra_options.normalize"
                && diagnostic.state == OptionSupportState::Mapped
                && diagnostic.backend_key.as_deref() == Some("mock")
        ));
    assert_eq!(
        backend_completed
            .usage
            .as_ref()
            .and_then(|usage| usage.prompt_tokens),
        Some(1)
    );
    assert_eq!(
        backend_completed
            .usage
            .as_ref()
            .and_then(|usage| usage.total_tokens),
        Some(1)
    );
}

#[tokio::test]
async fn test_execute_typed_validates_before_backend_execution() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Mock");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-invalid-1".to_string()),
        task_id: InferenceTaskId::Embedding,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        resolved_model_package_facts: None,
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
async fn test_lifecycle_events_carry_active_runtime_selected_device() {
    let gateway = InferenceGateway::with_backend(Box::new(MockActiveLlamaBackend), "llama.cpp");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start with active llama runtime");
    let sink = Arc::new(RecordingLifecycleSink::default());

    gateway
        .embeddings_with_lifecycle(
            vec!["alpha".to_string()],
            "mock-embedding",
            Some("embedding-device".to_string()),
            sink.clone(),
        )
        .await
        .expect("embedding request should execute");

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|event| event.selected_device_class == Some(InferenceDeviceClass::Cuda)));
    assert!(events
        .iter()
        .all(|event| event.selected_device_id.as_deref() == Some("cuda:0")));
}

#[tokio::test]
async fn test_lifecycle_events_do_not_report_config_only_device_as_selected() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig {
            device: Some("cuda:0".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start with explicit device");
    let sink = Arc::new(RecordingLifecycleSink::default());

    gateway
        .embeddings_with_lifecycle(
            vec!["alpha".to_string()],
            "mock-embedding",
            Some("embedding-device".to_string()),
            sink.clone(),
        )
        .await
        .expect("embedding request should execute");

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|event| event.selected_device_class.is_none()));
    assert!(events
        .iter()
        .all(|event| event.selected_device_id.is_none()));
}

#[tokio::test]
async fn test_lifecycle_events_do_not_report_auto_as_selected_device() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig {
            device: Some("auto".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start with auto device");
    let sink = Arc::new(RecordingLifecycleSink::default());

    gateway
        .embeddings_with_lifecycle(
            vec!["alpha".to_string()],
            "mock-embedding",
            Some("embedding-device-auto".to_string()),
            sink.clone(),
        )
        .await
        .expect("embedding request should execute");

    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|event| event.selected_device_class.is_none()));
    assert!(events
        .iter()
        .all(|event| event.selected_device_id.is_none()));
}

#[tokio::test]
async fn test_embeddings_with_lifecycle_records_token_usage_without_payloads() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig::default())
        .await
        .expect("gateway should start");
    let sink = Arc::new(RecordingLifecycleSink::default());

    gateway
        .embeddings_with_lifecycle(
            vec!["alpha beta".to_string()],
            "mock-embedding",
            Some("embedding-usage".to_string()),
            sink.clone(),
        )
        .await
        .expect("embedding request should execute");

    let events = sink.events();
    let completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion event");
    let usage = completed.usage.as_ref().expect("embedding usage");
    assert_eq!(usage.prompt_tokens, Some(2));
    assert_eq!(usage.completion_tokens, None);
    assert_eq!(usage.total_tokens, Some(2));

    let serialized = serde_json::to_string(completed).expect("event should serialize");
    assert!(!serialized.contains("alpha beta"));
    assert!(!serialized.contains("[10.0]"));
    assert!(!serialized.contains("vector"));
}

#[tokio::test]
async fn test_mode_info_runtime_facts_report_explicit_resolved_device() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig {
            device: Some("cuda:0".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start with explicit device");

    let facts = gateway.mode_info().await.runtime_fact_snapshots();

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].resolved_device.as_deref(), Some("cuda:0"));
}

#[tokio::test]
async fn test_mode_info_runtime_facts_do_not_report_auto_as_resolved_device() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;
    gateway
        .start(&BackendConfig {
            device: Some("auto".to_string()),
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start with auto device");

    let facts = gateway.mode_info().await.runtime_fact_snapshots();

    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].resolved_device, None);
}

#[tokio::test]
async fn test_execute_typed_with_lifecycle_records_validation_and_backend_completion() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-image-lifecycle".to_string()),
        task_id: InferenceTaskId::ImageGeneration,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::ImageGeneration {
            request: ImageGenerationRequest {
                model: "mock-image".to_string(),
                prompt: "typed prompt".to_string(),
                negative_prompt: None,
                width: Some(768),
                height: None,
                num_inference_steps: None,
                guidance_scale: None,
                seed: Some(42),
                scheduler: None,
                num_images_per_prompt: None,
                init_image: None,
                mask_image: None,
                strength: None,
                extra_options: serde_json::json!({
                    "safety_checker": false,
                }),
            },
        },
        generation_options: None,
        extra_options: serde_json::json!({
            "audit": true,
        }),
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed request should execute");

    let events = sink.events();
    assert_eq!(events.len(), 15);
    assert_eq!(events[0].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[0].task_id.as_deref(), Some("image_generation"));
    assert_eq!(events[1].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[2].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[3].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(events[3].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[3].task_id.as_deref(), Some("image_generation"));
    assert_eq!(events[4].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(
        events[4].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[5].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(
        events[5].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[6].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(events[6].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[6].task_id.as_deref(), Some("image_generation"));
    assert_eq!(events[7].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[7].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert!(events[7].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "image.width"
            && diagnostic.state == OptionSupportState::Honored
            && diagnostic.backend_key.as_deref() == Some("mock")
    }));
    assert!(events[7].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "image.extra_options.safety_checker"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert!(events[7].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "extra_options.audit"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert_eq!(events[8].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[8].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[9].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(events[9].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[10].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(
        events[10].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[11].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(
        events[11].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[12].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(events[12].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[13].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(
        events[13].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[14].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(
        events[14].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events.iter().all(|event| {
        event.request_id.as_deref() == Some("typed-image-lifecycle")
            && event.backend_key.as_deref() == Some("mock")
            && event.model_id.as_deref() == Some("mock-image")
    }));
}

#[test]
fn test_typed_text_request_model_name_falls_back_to_package_facts() {
    let fixture = include_str!(
        "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");
    let request = InferenceExecutionRequest {
        request_id: Some("typed-text-package-model".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: None,
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: true,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    assert_eq!(typed_request_model_name(&request), "llm/llama/tiny-gguf");
    let request_json =
        typed_text_generation_stream_request_json(request).expect("stream request should encode");
    let request_value: serde_json::Value =
        serde_json::from_str(&request_json).expect("request json should decode");
    assert_eq!(request_value["model"], "llm/llama/tiny-gguf");
}

#[tokio::test]
async fn test_execute_typed_audio_lifecycle_reports_extra_option_diagnostics() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-audio-lifecycle".to_string()),
        task_id: InferenceTaskId::AudioTranscription,
        model_ref: None,
        model_name: Some("mock-asr".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::AudioTranscription {
            request: AudioTranscriptionRequest {
                model: "mock-asr".to_string(),
                audio: Some(EncodedAudio {
                    data_base64: "UklGRg==".to_string(),
                    mime_type: "audio/wav".to_string(),
                    sample_rate_hz: Some(16000),
                }),
                audio_ref: Some("artifact://audio.wav".to_string()),
                language: Some("en".to_string()),
                prompt: Some("domain hint".to_string()),
                task: Some("transcribe".to_string()),
                chunk_length_s: Some(30.0),
                extra_options: serde_json::json!({
                    "return_timestamps": true,
                }),
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed audio request should execute");

    let events = sink.events();
    assert_eq!(events.len(), 15);
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion event should be recorded");
    assert_eq!(
        validation_completed.artifact_refs,
        vec!["artifact://audio.wav".to_string()]
    );
    assert!(validation_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.language"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(validation_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.prompt"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(validation_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.extra_options.return_timestamps"
                && diagnostic.state == OptionSupportState::Mapped
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend completion event should be recorded");
    assert_eq!(
        backend_completed.task_id.as_deref(),
        Some("audio_transcription")
    );
    assert_eq!(backend_completed.model_id.as_deref(), Some("mock-asr"));
    assert_eq!(
        backend_completed.artifact_refs,
        vec!["artifact://audio.wav".to_string()]
    );
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.language"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.prompt"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.task"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.chunk_length_s"
                && diagnostic.state == OptionSupportState::Honored
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));
    assert!(backend_completed
        .option_diagnostics
        .iter()
        .any(|diagnostic| {
            diagnostic.option_path == "audio_transcription.extra_options.return_timestamps"
                && diagnostic.state == OptionSupportState::Mapped
                && diagnostic.backend_key.as_deref() == Some("mock")
        }));

    let serialized_events = serde_json::to_string(&events).unwrap();
    assert!(!serialized_events.contains("UklGRg=="));
    assert!(!serialized_events.contains("domain hint"));
}

#[tokio::test]
async fn test_execute_typed_audio_lifecycle_omits_local_path_artifact_refs() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-audio-local-path-lifecycle".to_string()),
        task_id: InferenceTaskId::AudioTranscription,
        model_ref: None,
        model_name: Some("mock-asr".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::AudioTranscription {
            request: AudioTranscriptionRequest {
                model: "mock-asr".to_string(),
                audio: Some(EncodedAudio {
                    data_base64: "UklGRg==".to_string(),
                    mime_type: "audio/wav".to_string(),
                    sample_rate_hz: Some(16000),
                }),
                audio_ref: Some("/tmp/SECRET_AUDIO_PATH.wav".to_string()),
                language: None,
                prompt: None,
                task: None,
                chunk_length_s: None,
                extra_options: serde_json::Value::Null,
            },
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect("typed audio request should execute");

    let events = sink.events();
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion event should be recorded");
    assert!(validation_completed.artifact_refs.is_empty());
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend completion event should be recorded");
    assert!(backend_completed.artifact_refs.is_empty());
    let serialized_events = serde_json::to_string(&events).expect("events serialize");
    assert!(!serialized_events.contains("SECRET_AUDIO_PATH"));
}

#[tokio::test]
async fn test_contract_only_typed_lifecycle_collects_bounded_artifact_refs() {
    let cases = vec![
        (
            InferenceTaskId::ImageUnderstanding,
            InferenceExecutionInput::ImageUnderstanding {
                request: ImageUnderstandingRequest {
                    prompt: "describe".to_string(),
                    images: Vec::new(),
                    image_refs: vec![
                        "artifact://image-a.png".to_string(),
                        "/tmp/private-image.png".to_string(),
                    ],
                    extra_options: serde_json::Value::Null,
                },
            },
            vec!["artifact://image-a.png".to_string()],
            "/tmp/private-image.png",
        ),
        (
            InferenceTaskId::DepthEstimation,
            InferenceExecutionInput::DepthEstimation {
                request: DepthEstimationRequest {
                    image: None,
                    image_ref: Some("artifact://depth-input.png".to_string()),
                    extra_options: serde_json::Value::Null,
                },
            },
            vec!["artifact://depth-input.png".to_string()],
            "/tmp/private-depth.png",
        ),
        (
            InferenceTaskId::VideoUnderstanding,
            InferenceExecutionInput::VideoUnderstanding {
                request: VideoUnderstandingRequest {
                    prompt: "summarize".to_string(),
                    video: None,
                    video_ref: Some("artifact://clip.mp4".to_string()),
                    extra_options: serde_json::Value::Null,
                },
            },
            vec!["artifact://clip.mp4".to_string()],
            "/tmp/private-video.mp4",
        ),
        (
            InferenceTaskId::MultimodalGeneration,
            InferenceExecutionInput::MultimodalGeneration {
                request: MultimodalGenerationRequest {
                    parts: vec![
                        MultimodalInputPart::Artifact {
                            modality: crate::model_contracts::InferenceModality::Image,
                            artifact_ref: "artifact://multi-image.png".to_string(),
                            mime_type: Some("image/png".to_string()),
                        },
                        MultimodalInputPart::Artifact {
                            modality: crate::model_contracts::InferenceModality::Audio,
                            artifact_ref: "/tmp/private-audio.wav".to_string(),
                            mime_type: Some("audio/wav".to_string()),
                        },
                    ],
                    extra_options: serde_json::Value::Null,
                },
            },
            vec!["artifact://multi-image.png".to_string()],
            "/tmp/private-audio.wav",
        ),
    ];

    for (task_id, input, expected_refs, forbidden_ref) in cases {
        let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
        let sink = Arc::new(RecordingLifecycleSink::default());
        let request = InferenceExecutionRequest {
            request_id: Some(format!("contract-only-{}", task_id.canonical_label())),
            task_id,
            model_ref: None,
            model_name: Some("mock-contract".to_string()),
            resolved_model_package_facts: None,
            input,
            generation_options: None,
            extra_options: serde_json::Value::Null,
        };

        gateway
            .execute_typed_with_lifecycle(request, sink.clone())
            .await
            .expect_err("contract-only task should not execute");

        let events = sink.events();
        let validation_failed = events
            .iter()
            .find(|event| {
                event.phase == InferenceLifecyclePhase::TaskValidation
                    && event.kind == InferenceRequestLifecycleEventKind::Failed
            })
            .expect("task validation failure should be recorded");
        assert_eq!(validation_failed.artifact_refs, expected_refs);
        let serialized_events = serde_json::to_string(&events).expect("events serialize");
        assert!(!serialized_events.contains(forbidden_ref));
    }
}

#[test]
fn test_bounded_artifact_ref_filters_local_path_shapes() {
    assert_eq!(
        crate::bounded_inference_artifact_ref("artifact://audio.wav"),
        Some("artifact://audio.wav".to_string())
    );
    assert_eq!(
        crate::bounded_inference_artifact_ref("/tmp/audio.wav"),
        None
    );
    assert_eq!(crate::bounded_inference_artifact_ref("./audio.wav"), None);
    assert_eq!(crate::bounded_inference_artifact_ref("../audio.wav"), None);
    assert_eq!(crate::bounded_inference_artifact_ref("~/audio.wav"), None);
    assert_eq!(
        crate::bounded_inference_artifact_ref("file:///tmp/audio.wav"),
        None
    );
    assert_eq!(
        crate::bounded_inference_artifact_ref("C:\\Users\\audio.wav"),
        None
    );
}

#[tokio::test]
async fn test_execute_typed_with_lifecycle_records_validation_failure_without_backend_phase() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-invalid".to_string()),
        task_id: InferenceTaskId::Embedding,
        model_ref: None,
        model_name: Some("mock-image".to_string()),
        resolved_model_package_facts: None,
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

    let error = gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect_err("typed validation should fail");

    assert!(matches!(
        error,
        GatewayError::Validation(
            crate::types::InferenceExecutionRequestValidationError::TaskInputMismatch { .. }
        )
    ));
    let events = sink.events();
    assert_eq!(events.len(), 3);
    assert!(events
        .iter()
        .all(|event| event.phase == InferenceLifecyclePhase::TaskValidation));
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events[1]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("does not match input type image_generation")));
}

#[tokio::test]
async fn test_execute_typed_with_lifecycle_rejects_package_task_mismatch_before_backend_phase() {
    let fixture =
        include_str!("../tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json");
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("embedding package facts fixture");
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "mock");
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("typed-package-task-mismatch".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package_facts.model_ref.clone()),
        model_name: Some("mock-text".to_string()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let error = gateway
        .execute_typed_with_lifecycle(request, sink.clone())
        .await
        .expect_err("typed validation should reject package task mismatch");

    match error {
        GatewayError::Validation(
            crate::types::InferenceExecutionRequestValidationError::PackageTaskMismatch {
                request_task_id,
                package_task_id,
                model_id,
            },
        ) => {
            assert_eq!(request_task_id, InferenceTaskId::TextGeneration);
            assert_eq!(package_task_id, InferenceTaskId::Embedding);
            assert_eq!(model_id, "embedding/qwen3/tiny-embedding-gguf");
        }
        other => panic!("unexpected gateway error: {other:?}"),
    }

    let events = sink.events();
    assert_eq!(events.len(), 6);
    assert!(events[..3].iter().all(|event| {
        event.phase == InferenceLifecyclePhase::ModelPackageResolution
            && event.model_id.as_deref() == Some("embedding/qwen3/tiny-embedding-gguf")
    }));
    assert!(events[3..].iter().all(|event| {
        event.phase == InferenceLifecyclePhase::TaskValidation
            && event.model_id.as_deref() == Some("embedding/qwen3/tiny-embedding-gguf")
    }));
    assert!(!events
        .iter()
        .any(|event| event.phase == InferenceLifecyclePhase::BackendExecution));
    assert_eq!(events[3].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[4].kind, InferenceRequestLifecycleEventKind::Failed);
    assert!(events[4].detail.as_deref().is_some_and(|detail| {
        detail.contains("TextGeneration")
            && detail.contains("Embedding")
            && detail.contains("embedding/qwen3/tiny-embedding-gguf")
    }));
    assert_eq!(
        events[5].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
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
            ..Default::default()
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let mut stream = gateway
        .chat_completion_stream_with_lifecycle(
            r#"{"model":"mock-chat"}"#.to_string(),
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
            && event.model_id.as_deref() == Some("mock-chat")
    }));
}

#[tokio::test]
async fn test_stream_typed_text_with_lifecycle_records_validation_and_backend_phases() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            ..Default::default()
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());
    let fixture = include_str!(
        "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");

    let request = InferenceExecutionRequest {
        request_id: Some("req-typed-stream".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package_facts.model_ref.clone()),
        model_name: Some("typed-model".to_string()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: true,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let mut stream = gateway
        .stream_typed_text_with_lifecycle(request, sink.clone())
        .await
        .expect("typed stream should start");
    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("stream chunk");
        if let Some(content) = chunk.content {
            response.push_str(&content);
        }
    }

    assert_eq!(response, "hello");
    let events = sink.events();
    assert_eq!(events.len(), 18);
    assert_eq!(
        events[0].phase,
        InferenceLifecyclePhase::ModelPackageResolution
    );
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(
        events[1].phase,
        InferenceLifecyclePhase::ModelPackageResolution
    );
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(
        events[2].phase,
        InferenceLifecyclePhase::ModelPackageResolution
    );
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[3].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(events[3].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[4].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[4].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    let validation_compatibility_report = events[4]
        .compatibility_report
        .as_ref()
        .expect("task validation compatibility report");
    assert_eq!(validation_compatibility_report.status, "rejected");
    assert!(!validation_compatibility_report.compatible);
    assert!(!events[4].compatibility_issues.is_empty());
    assert_eq!(events[5].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[5].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[6].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(events[6].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[7].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(
        events[7].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[8].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(
        events[8].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[9].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(events[9].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[10].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[10].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    let backend_compatibility_report = events[10]
        .compatibility_report
        .as_ref()
        .expect("backend execution compatibility report");
    assert_eq!(backend_compatibility_report.status, "rejected");
    assert!(!backend_compatibility_report.compatible);
    assert!(!events[10].compatibility_issues.is_empty());
    assert_eq!(events[11].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[11].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[12].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(events[12].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[13].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(
        events[13].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[14].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(
        events[14].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[15].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(events[15].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[16].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(
        events[16].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[17].phase, InferenceLifecyclePhase::ResultProjection);
    assert_eq!(
        events[17].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events.iter().all(|event| {
        event.request_id.as_deref() == Some("req-typed-stream")
            && event.task_id.as_deref() == Some("text_generation")
            && event.backend_key.as_deref() == Some("mock")
            && event.model_id.as_deref() == Some("llm/llama/tiny-gguf")
    }));
}

#[tokio::test]
async fn test_stream_typed_text_with_lifecycle_records_terminal_chunk_usage() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            usage_on_terminal: Some(InferenceUsage {
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            }),
            cache_handle_on_terminal: Some("kv-stream-checkpoint".to_string()),
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());
    let fixture = include_str!(
        "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");

    let request = InferenceExecutionRequest {
        request_id: Some("req-typed-stream-usage".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package_facts.model_ref.clone()),
        model_name: Some("typed-model".to_string()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("SECRET_STREAM_PROMPT should not reach lifecycle".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: true,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let mut stream = gateway
        .stream_typed_text_with_lifecycle(request, sink.clone())
        .await
        .expect("typed stream should start");
    let mut response = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("stream chunk");
        if let Some(content) = chunk.content {
            response.push_str(&content);
        }
    }

    assert_eq!(response, "hello");
    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    let usage = backend_completed
        .usage
        .as_ref()
        .expect("terminal stream chunk usage should be persisted on lifecycle completion");
    assert_eq!(usage.prompt_tokens, Some(8));
    assert_eq!(usage.completion_tokens, Some(5));
    assert_eq!(usage.total_tokens, Some(13));
    assert_eq!(
        backend_completed.cache_handle_id.as_deref(),
        Some("kv-stream-checkpoint")
    );
    let event_json = serde_json::to_string(backend_completed).expect("event serializes");
    assert!(!event_json.contains("SECRET_STREAM_PROMPT"));
    assert!(!event_json.contains("hello"));
}

#[tokio::test]
async fn test_stream_typed_text_with_lifecycle_filters_path_shaped_cache_handle() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            usage_on_terminal: None,
            cache_handle_on_terminal: Some("/tmp/private/kv-stream.bin".to_string()),
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());
    let request = InferenceExecutionRequest {
        request_id: Some("req-typed-stream-cache-path".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: Some("typed-model".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: true,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let mut stream = gateway
        .stream_typed_text_with_lifecycle(request, sink.clone())
        .await
        .expect("typed stream should start");
    let mut terminal_cache_handle = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("stream chunk");
        if chunk.done {
            terminal_cache_handle = chunk.cache_handle_id;
            break;
        }
    }

    assert_eq!(
        terminal_cache_handle.as_deref(),
        Some("/tmp/private/kv-stream.bin")
    );
    let events = sink.events();
    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    assert!(backend_completed.cache_handle_id.is_none());
    let event_json = serde_json::to_string(backend_completed).expect("event serializes");
    assert!(!event_json.contains("/tmp/private/kv-stream.bin"));
}

#[tokio::test]
async fn test_chat_completion_stream_with_lifecycle_records_stream_failure() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: true,
            ..Default::default()
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let mut stream = gateway
        .chat_completion_stream_with_lifecycle(
            r#"{"model":"mock-chat"}"#.to_string(),
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
    assert!(events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("mock-chat")));
}

#[tokio::test]
async fn test_stream_typed_text_with_lifecycle_records_failed_backend_compatibility() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: true,
            ..Default::default()
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());
    let fixture = include_str!(
        "../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");

    let request = InferenceExecutionRequest {
        request_id: Some("req-typed-stream-fail".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: Some(package_facts.model_ref.clone()),
        model_name: Some("typed-model".to_string()),
        resolved_model_package_facts: Some(package_facts),
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: true,
        },
        generation_options: None,
        extra_options: serde_json::Value::Null,
    };

    let mut stream = gateway
        .stream_typed_text_with_lifecycle(request, sink.clone())
        .await
        .expect("typed stream should start");
    let result = stream.next().await.expect("stream item");
    assert!(matches!(
        result,
        Err(BackendError::Inference(message)) if message.contains("mock stream failure")
    ));

    let events = sink.events();
    assert_eq!(events.len(), 12);
    assert_eq!(events[10].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(events[10].kind, InferenceRequestLifecycleEventKind::Failed);
    let backend_compatibility_report = events[10]
        .compatibility_report
        .as_ref()
        .expect("failed backend execution compatibility report");
    assert_eq!(backend_compatibility_report.status, "rejected");
    assert!(!backend_compatibility_report.compatible);
    assert!(!events[10].compatibility_issues.is_empty());
    assert_eq!(
        events[10].detail.as_deref(),
        Some("Inference error: mock stream failure")
    );
    assert!(events[11].compatibility_report.is_none());
    assert!(events[11].compatibility_issues.is_empty());
    assert!(events
        .iter()
        .all(|event| event.phase != InferenceLifecyclePhase::Postprocessing));
    assert!(events
        .iter()
        .all(|event| event.phase != InferenceLifecyclePhase::ResultProjection));
    assert!(events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("llm/llama/tiny-gguf")));
}

#[tokio::test]
async fn test_chat_completion_stream_with_lifecycle_records_drop_cancellation() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            ..Default::default()
        }),
        "mock",
    );
    let sink = Arc::new(RecordingLifecycleSink::default());

    let stream = gateway
        .chat_completion_stream_with_lifecycle(
            r#"{"model":"mock-chat"}"#.to_string(),
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
    assert!(events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("mock-chat")));
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
    assert!(events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("mock")));
}

#[tokio::test]
async fn test_generate_image_with_lifecycle_records_failure() {
    let gateway = InferenceGateway::with_backend(
        Box::new(MockLifecycleStreamBackend {
            fail_on_stream: false,
            ..Default::default()
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
    assert!(events
        .iter()
        .all(|event| event.model_id.as_deref() == Some("mock")));
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
