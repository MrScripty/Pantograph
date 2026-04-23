use std::path::PathBuf;
use std::sync::Arc;

use crate::backend::{BackendConfig, BackendError};

use super::super::{EmbeddingStartRequest, GatewayError, InferenceGateway, InferenceStartRequest};
use super::{MockHttpBackend, MockImageBackend, MockProcessSpawner, MockReusedBackend};

#[tokio::test]
async fn test_build_inference_start_config_for_ollama_uses_model_name() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Ollama");

    let config = gateway
        .build_inference_start_config(InferenceStartRequest {
            external_url: None,
            ollama_model_name: Some("llava:13b".to_string()),
            ..InferenceStartRequest::default()
        })
        .await
        .expect("config should build");

    assert_eq!(config.model_name.as_deref(), Some("llava:13b"));
    assert_eq!(config.model_path, None);
    assert!(!config.embedding_mode);
}

#[tokio::test]
async fn test_build_inference_start_config_for_external_llamacpp_uses_external_url() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "llama.cpp");

    let config = gateway
        .build_inference_start_config(InferenceStartRequest {
            external_url: Some("http://127.0.0.1:1234".to_string()),
            ..InferenceStartRequest::default()
        })
        .await
        .expect("config should build");

    assert_eq!(
        config.external_url.as_deref(),
        Some("http://127.0.0.1:1234")
    );
    assert_eq!(config.model_path, None);
    assert!(!config.embedding_mode);
}

#[tokio::test]
async fn test_build_inference_start_config_rejects_external_url_without_backend_support() {
    let gateway = InferenceGateway::with_backend(Box::new(MockReusedBackend), "mock");

    let error = gateway
        .build_inference_start_config(InferenceStartRequest {
            external_url: Some("http://127.0.0.1:1234".to_string()),
            ..InferenceStartRequest::default()
        })
        .await
        .expect_err("non-external backend should reject external attachment");

    assert!(matches!(
        error,
        GatewayError::Backend(BackendError::Config(message))
        if message.contains("not supported for active backend 'mock'")
    ));
}

#[tokio::test]
async fn test_build_inference_start_config_for_pytorch_uses_model_path_without_mmproj() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "PyTorch");

    let config = gateway
        .build_inference_start_config(InferenceStartRequest {
            file_model_path: Some(PathBuf::from("/models/qwen2.5-7b-instruct")),
            mmproj_path: None,
            device: Some("cuda:0".to_string()),
            ..InferenceStartRequest::default()
        })
        .await
        .expect("config should build");

    assert_eq!(
        config
            .model_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        Some("/models/qwen2.5-7b-instruct".to_string())
    );
    assert_eq!(config.mmproj_path, None);
    assert_eq!(config.device.as_deref(), Some("cuda:0"));
    assert!(!config.embedding_mode);
}

#[tokio::test]
async fn test_build_inference_start_config_for_candle_rejects_inference_mode() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Candle");

    let error = gateway
        .build_inference_start_config(InferenceStartRequest {
            file_model_path: Some(PathBuf::from("/models/unsupported")),
            ..InferenceStartRequest::default()
        })
        .await
        .expect_err("candle should reject inference mode");

    assert!(matches!(
        error,
        GatewayError::Backend(BackendError::Config(message))
        if message.contains("Candle does not support inference mode")
    ));
}

#[tokio::test]
async fn test_build_embedding_start_config_for_candle_uses_candle_model_path() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "Candle");

    let config = gateway
        .build_embedding_start_config(EmbeddingStartRequest {
            candle_model_path: Some(PathBuf::from("/models/bge-small-en-v1.5")),
            ..EmbeddingStartRequest::default()
        })
        .await
        .expect("config should build");

    assert_eq!(
        config
            .model_path
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        Some("/models/bge-small-en-v1.5".to_string())
    );
    assert!(config.embedding_mode);
}

#[tokio::test]
async fn test_build_embedding_start_config_for_pytorch_rejects_embedding_mode() {
    let gateway = InferenceGateway::with_backend(Box::new(MockImageBackend), "PyTorch");

    let error = gateway
        .build_embedding_start_config(EmbeddingStartRequest {
            gguf_model_path: Some(PathBuf::from("/models/embed.gguf")),
            ..EmbeddingStartRequest::default()
        })
        .await
        .expect_err("pytorch should reject embedding mode");

    assert!(matches!(
        error,
        GatewayError::Backend(BackendError::Config(message))
        if message.contains("PyTorch does not support embedding mode")
    ));
}

#[tokio::test]
async fn test_prepare_embedding_runtime_captures_restore_config_and_base_url() {
    let gateway = InferenceGateway::with_backend(Box::new(MockHttpBackend), "Ollama");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    let inference_config = BackendConfig {
        model_name: Some("llava:13b".to_string()),
        ..BackendConfig::default()
    };
    gateway
        .start(&inference_config)
        .await
        .expect("gateway should start in inference mode");

    let prepared = gateway
        .prepare_embedding_runtime(EmbeddingStartRequest::default())
        .await
        .expect("embedding preparation should succeed");

    assert_eq!(prepared.backend_name, "Ollama");
    assert_eq!(
        prepared
            .restore_config
            .as_ref()
            .and_then(|config| config.model_name.as_deref()),
        Some("llava:13b")
    );
    assert_eq!(prepared.base_url.as_deref(), Some("http://127.0.0.1:11434"));
    assert!(gateway.is_embedding_mode().await);
}

#[tokio::test]
async fn test_prepare_embedding_runtime_keeps_existing_embedding_runtime() {
    let gateway = InferenceGateway::with_backend(Box::new(MockHttpBackend), "Ollama");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    gateway
        .start(&BackendConfig {
            model_name: Some("nomic-embed-text".to_string()),
            embedding_mode: true,
            ..BackendConfig::default()
        })
        .await
        .expect("gateway should start in embedding mode");
    let before = gateway.runtime_lifecycle_snapshot().await;

    let prepared = gateway
        .prepare_embedding_runtime(EmbeddingStartRequest::default())
        .await
        .expect("existing embedding runtime should be reused");
    let after = gateway.runtime_lifecycle_snapshot().await;

    assert_eq!(prepared.backend_name, "Ollama");
    assert!(prepared.restore_config.is_none());
    assert_eq!(prepared.base_url.as_deref(), Some("http://127.0.0.1:11434"));
    assert_eq!(after.runtime_instance_id, before.runtime_instance_id);
}

#[tokio::test]
async fn test_restart_runtime_config_reads_active_runtime_config_until_stop() {
    let gateway = InferenceGateway::with_backend(Box::new(MockHttpBackend), "Ollama");
    gateway.set_spawner(Arc::new(MockProcessSpawner)).await;

    let config = BackendConfig {
        model_name: Some("llava:13b".to_string()),
        ..BackendConfig::default()
    };
    gateway
        .start(&config)
        .await
        .expect("gateway should start in inference mode");

    let restart_config = gateway
        .restart_runtime_config()
        .await
        .expect("active runtime config should be available");
    assert_eq!(restart_config.model_name.as_deref(), Some("llava:13b"));

    gateway.stop().await;
    assert!(gateway.restart_runtime_config().await.is_none());
}
