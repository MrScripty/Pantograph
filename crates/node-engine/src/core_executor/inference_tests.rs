use super::super::*;
#[cfg(feature = "inference-nodes")]
use crate::engine::TaskExecutor;

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
fn test_build_model_dependency_request_maps_canonical_rerank_task() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));

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
