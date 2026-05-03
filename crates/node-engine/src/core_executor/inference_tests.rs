use super::super::*;
#[cfg(feature = "inference-nodes")]
use crate::engine::TaskExecutor;
#[cfg(feature = "inference-nodes")]
use async_trait::async_trait;
#[cfg(feature = "inference-nodes")]
use futures_util::stream;
#[cfg(feature = "inference-nodes")]
use inference::backend::BackendStartOutcome;
#[cfg(feature = "inference-nodes")]
use inference::{
    BackendCapabilities, BackendConfig, BackendError, ChatChunk, EmbeddingResult,
    GenerationOptions, InferenceBackend, InferenceExecutionInput, InferenceTaskId,
    LengthGenerationOptions, ProcessSpawner, PumasModelRef, RerankRequest, RerankResponse,
    SamplingGenerationOptions,
};
#[cfg(feature = "inference-nodes")]
use std::pin::Pin;
#[cfg(feature = "inference-nodes")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_execute_embedding_fails_when_gateway_missing() {
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    let err = execute_embedding(None, &inputs)
        .await
        .expect_err("embedding should fail fast without gateway");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_preserves_canonical_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("system_prompt".to_string(), serde_json::json!("system"));
    inputs.insert("context".to_string(), serde_json::json!("facts"));
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("chat-completion"),
    );
    inputs.insert("runtime_hint".to_string(), serde_json::json!("vllm"));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/tiny",
            "revision": "abc"
        }),
    );
    inputs.insert(
        "generation_options".to_string(),
        serde_json::json!({
            "length": {"max_new_tokens": 32},
            "sampling": {"temperature": 0.25}
        }),
    );

    let request = build_text_generation_execution_request(&inputs)
        .expect("canonical text generation request should build");

    assert_eq!(request.task_id, InferenceTaskId::ChatCompletion);
    assert_eq!(request.runtime_hint.as_deref(), Some("vllm"));
    assert_eq!(
        request.model_ref,
        Some(PumasModelRef {
            model_id: "pumas://models/tiny".to_string(),
            revision: Some("abc".to_string()),
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        })
    );
    assert_eq!(
        request.generation_options,
        Some(GenerationOptions {
            length: LengthGenerationOptions {
                max_new_tokens: Some(32),
                ..LengthGenerationOptions::default()
            },
            sampling: SamplingGenerationOptions {
                temperature: Some(0.25),
                ..SamplingGenerationOptions::default()
            },
            ..GenerationOptions::default()
        })
    );
    match request.input {
        InferenceExecutionInput::TextGeneration {
            prompt,
            system_prompt,
            stream,
            ..
        } => {
            assert_eq!(prompt.as_deref(), Some("hello\n\nContext:\nfacts"));
            assert_eq!(system_prompt.as_deref(), Some("system"));
            assert!(!stream);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_rejects_malformed_generation_options() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "generation_options".to_string(),
        serde_json::json!({"length": "not-an-object"}),
    );

    let error = build_text_generation_execution_request(&inputs)
        .expect_err("malformed generation options should fail");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid generation_options input"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_execute_llm_inference_non_streaming_uses_typed_gateway_boundary() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
        }),
        "mock",
    ));
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));
    inputs.insert(
        "generation_options".to_string(),
        serde_json::json!({
            "length": {"max_new_tokens": 16},
            "sampling": {
                "temperature": 0.2,
                "top_p": 0.8,
                "top_k": 40
            }
        }),
    );

    let outputs = execute_llm_inference(Some(&gateway), &inputs, "llm-inference-1", None, "exec-a")
        .await
        .expect("typed non-streaming inference should execute");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert!(outputs.get("stream").is_some_and(|value| value.is_null()));

    let captured = requests.lock().expect("requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0]["model"], serde_json::json!("typed-model"));
    assert_eq!(captured[0]["max_tokens"], serde_json::json!(16));
    assert_eq!(captured[0]["temperature"], serde_json::json!(0.2));
    assert_eq!(captured[0]["top_p"], serde_json::json!(0.8));
    assert_eq!(captured[0]["top_k"], serde_json::json!(40));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_embedding_dispatches_to_embedding_handler() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));
    inputs.insert("text".to_string(), serde_json::json!("hello"));

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("canonical embedding inference should route to embedding handler");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_feature_extraction_alias_dispatches_to_embedding_handler() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("feature-extraction"),
    );
    inputs.insert("text".to_string(), serde_json::json!("hello"));

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("feature-extraction alias should route to embedding handler");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_rerank_dispatches_to_reranker_handler() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["a", "b"]));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_path": "/tmp/reranker.gguf",
            "recommended_backend": "llamacpp"
        }),
    );

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("canonical rerank inference should route to reranker handler");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_pumas_text_ranking_alias_dispatches_to_reranker_handler() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["a", "b"]));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_path": "/tmp/reranker.gguf",
            "task_type_primary": "text-ranking"
        }),
    );

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("text-ranking alias should route to reranker handler");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("InferenceGateway not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_retired_llamacpp_node_type_is_not_executable() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llamacpp-inference"}),
    );

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llamacpp-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("retired llama.cpp inference should not execute");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Retired inference node type 'llamacpp-inference'"));
            assert!(message.contains("canonical llm-inference"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_unload_model_rejects_ollama_model_ref_without_network() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "model_ref".to_string(),
        serde_json::json!({
            "contractVersion": 2,
            "engine": "ollama",
            "modelId": "llama3:8b",
            "modelPath": "llama3:8b",
            "taskTypePrimary": "text-generation"
        }),
    );

    let error = execute_unload_model(None, &inputs)
        .await
        .expect_err("Ollama unload should be retired before network access");
    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Ollama model_ref"));
            assert!(message.contains("canonical llm-inference"));
            assert!(message.contains("Pumas model reference"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(all(feature = "inference-nodes", feature = "pytorch-nodes"))]
#[tokio::test]
async fn test_canonical_llm_pytorch_hint_dispatches_to_dependency_preflight() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );
    inputs.insert("model_path".to_string(), serde_json::json!("/tmp/model"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("canonical PyTorch inference should require dependency preflight");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency resolver is not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[tokio::test]
async fn test_dependency_preflight_skips_canonical_llamacpp() {
    let mut inputs = HashMap::new();
    inputs.insert("runtime_hint".to_string(), serde_json::json!("llamacpp"));
    let extensions = ExecutorExtensions::new();
    let resolved = enforce_dependency_preflight("llm-inference", &inputs, &extensions)
        .await
        .expect("canonical llama.cpp preflight should be skipped");
    assert!(resolved.is_none());
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[tokio::test]
async fn test_dependency_preflight_blocks_canonical_pytorch_without_resolver() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/model.gguf"),
    );
    inputs.insert("runtime_hint".to_string(), serde_json::json!("pytorch"));
    let extensions = ExecutorExtensions::new();
    let err = enforce_dependency_preflight("llm-inference", &inputs, &extensions)
        .await
        .expect_err("canonical PyTorch preflight should require resolver");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency resolver is not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_canonical_backend_key_normalizes_common_aliases() {
    assert_eq!(
        canonical_backend_key(Some("  onnx-runtime  ")),
        Some("onnx-runtime".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("llama.cpp")),
        Some("llamacpp".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("llama_cpp")),
        Some("llamacpp".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("torch")),
        Some("pytorch".to_string())
    );
    assert_eq!(
        canonical_backend_key(Some("stable-audio")),
        Some("stable_audio".to_string())
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_is_llamacpp_backend_name_accepts_aliases() {
    assert!(is_llamacpp_backend_name("llama.cpp"));
    assert!(is_llamacpp_backend_name("llama_cpp"));
    assert!(is_llamacpp_backend_name("llamacpp"));
    assert!(!is_llamacpp_backend_name("pytorch"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_uses_canonical_backend_key() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("onnx-runtime"));

    let request = build_model_dependency_request("llm-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("onnx-runtime"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_uses_canonical_llamacpp_hint() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("llama.cpp"));

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("text-generation")
    );
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_uses_canonical_pytorch_hint() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_does_not_infer_retired_backend_node() {
    let inputs = HashMap::new();

    let request = build_model_dependency_request("llamacpp-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key, None);
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_inputs_with_model_path_uses_resolved_model_source_entry_path() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_value("pumas://models/tiny-gguf", "/models/tiny/model.gguf"),
    );

    let canonical =
        inputs_with_model_path_from_ref(&inputs).expect("resolved model source should parse");

    assert_eq!(
        canonical.get("model_path").and_then(|value| value.as_str()),
        Some("/models/tiny/model.gguf")
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_inputs_with_model_path_rejects_malformed_resolved_model_source() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        serde_json::json!({"source_contract_version": 1}),
    );

    let err = inputs_with_model_path_from_ref(&inputs)
        .expect_err("malformed resolved model source should fail explicitly");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid resolved_model_source input"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_ref_v2_prefers_resolved_model_source_identity() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_value("pumas://models/tiny-gguf", "/models/tiny/model.gguf"),
    );

    let model_ref = build_model_ref_v2(
        None,
        "llamacpp",
        "/models/tiny/model.gguf",
        "/models/tiny/model.gguf",
        "text-generation",
        &inputs,
    );

    assert_eq!(model_ref.model_id, "pumas://models/tiny-gguf");
    assert_eq!(model_ref.model_path, "/models/tiny/model.gguf");
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_dependency_request_uses_resolved_model_source_identity() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_value("pumas://models/tiny-gguf", "/models/tiny/model.gguf"),
    );

    let request =
        build_model_dependency_request("llm-inference", "/models/tiny/model.gguf", &inputs);

    assert_eq!(
        request.model_id.as_deref(),
        Some("pumas://models/tiny-gguf")
    );
    assert_eq!(request.model_path, "/models/tiny/model.gguf");
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_maps_canonical_embedding_task() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("feature-extraction")
    );
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_maps_embedding_alias_task() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("sentence-similarity"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("feature-extraction")
    );
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_maps_canonical_rerank_task() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(request.task_type_primary.as_deref(), Some("reranking"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_maps_pumas_rerank_alias_task() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "pipeline_tag": "text-ranking"
        }),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(request.task_type_primary.as_deref(), Some("reranking"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_prefers_recommended_backend_for_diffusion() {
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "recommended_backend".to_string(),
        serde_json::json!("diffusers"),
    );

    let request = build_model_dependency_request("diffusion-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key.as_deref(), Some("diffusers"));
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_infer_task_type_primary_defaults_diffusion_node_to_text_to_image() {
    let inputs = HashMap::new();
    let task = infer_task_type_primary("diffusion-inference", &inputs);
    assert_eq!(task, "text-to-image");
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_build_model_dependency_request_defaults_diffusion_backend_to_pytorch() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("diffusion"));

    let request = build_model_dependency_request("diffusion-inference", "/tmp/model", &inputs);
    assert_eq!(request.backend_key, None);
    assert_eq!(request.task_type_primary.as_deref(), Some("text-to-image"));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_accepts_strings_and_objects() {
    let value = serde_json::json!([
        "first",
        {"text": "second"},
        {"content": "third"},
        {"document": "fourth"}
    ]);
    let documents = parse_reranker_documents(&value).expect("documents should parse");
    assert_eq!(documents, vec!["first", "second", "third", "fourth"]);
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_rejects_invalid_item() {
    let value = serde_json::json!([{"id": 1}]);
    let error = parse_reranker_documents(&value).expect_err("invalid item should fail");
    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("strings or objects"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[test]
fn test_infer_task_type_primary_detects_reranker() {
    let mut inputs = HashMap::new();
    inputs.insert("model_type".to_string(), serde_json::json!("reranker"));
    assert_eq!(infer_task_type_primary("reranker", &inputs), "reranking");
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_parse_reranker_documents_input_accepts_json_string_alias() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "documents_json".to_string(),
        serde_json::json!("[\"alpha\", {\"text\": \"beta\"}]"),
    );
    let documents = parse_reranker_documents_input(&inputs).expect("documents_json should parse");
    assert_eq!(documents, vec!["alpha", "beta"]);
}

#[cfg(feature = "inference-nodes")]
fn resolved_model_source_value(model_id: &str, entry_path: &str) -> serde_json::Value {
    serde_json::json!({
        "source_contract_version": 1,
        "source_kind": "pumas_resolved",
        "artifact_kind": "gguf",
        "entry_path": entry_path,
        "storage_kind": "library_owned",
        "validation_state": "valid",
        "model_ref": {
            "model_id": model_id
        }
    })
}

#[cfg(feature = "inference-nodes")]
struct MockTypedTextBackend {
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
}

#[cfg(feature = "inference-nodes")]
#[async_trait]
impl InferenceBackend for MockTypedTextBackend {
    fn name(&self) -> &'static str {
        "mock-typed"
    }

    fn description(&self) -> &'static str {
        "Mock typed text backend"
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
    ) -> std::result::Result<BackendStartOutcome, BackendError> {
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
        request_json: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        let request: serde_json::Value = serde_json::from_str(&request_json)
            .map_err(|error| BackendError::Inference(error.to_string()))?;
        self.requests.lock().expect("requests lock").push(request);
        Ok(Box::pin(stream::iter([
            Ok(ChatChunk {
                content: Some("typed response".to_string()),
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
    ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
        Err(BackendError::Inference(
            "embeddings not supported by mock".to_string(),
        ))
    }

    async fn rerank(
        &self,
        _request: RerankRequest,
    ) -> std::result::Result<RerankResponse, BackendError> {
        Err(BackendError::Inference(
            "rerank not supported by mock".to_string(),
        ))
    }
}
