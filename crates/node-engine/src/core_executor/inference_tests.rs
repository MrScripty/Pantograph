use super::super::*;
#[cfg(feature = "inference-nodes")]
use crate::engine::TaskExecutor;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::extension_keys;
#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
use crate::model_dependencies::{
    DependencyState, DependencyValidationState, ModelDependencyInstallResult,
    ModelDependencyRequest, ModelDependencyRequirements, ModelDependencyResolver,
    ModelDependencyStatus, ModelRefV2,
};
#[cfg(feature = "inference-nodes")]
use async_trait::async_trait;
#[cfg(feature = "inference-nodes")]
use futures_util::stream;
#[cfg(feature = "inference-nodes")]
use inference::backend::BackendStartOutcome;
#[cfg(feature = "inference-nodes")]
use inference::{
    AudioTranscriptionRequest, AudioTranscriptionResult, AudioTranscriptionSegment,
    BackendCapabilities, BackendConfig, BackendError, CacheGenerationOptions, ChatChunk,
    EmbeddingResult, EncodedImage, GenerationOptions, ImageGenerationRequest,
    ImageGenerationResult, InferenceBackend, InferenceExecutionInput, InferenceLifecyclePhase,
    InferenceRequestLifecycleEvent, InferenceRequestLifecycleEventKind,
    InferenceRequestLifecycleEventSink, InferenceTaskId, InferenceUsage, LengthGenerationOptions,
    ProcessSpawner, PumasModelRef, RerankRequest, RerankResponse, RerankResult,
    ResolvedModelPackageFacts, SamplingGenerationOptions,
};
#[cfg(feature = "inference-nodes")]
use std::pin::Pin;
#[cfg(feature = "inference-nodes")]
use std::sync::{Arc, Mutex};

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
fn test_build_text_generation_execution_request_projects_kv_cache_options() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "kv_cache_in".to_string(),
        serde_json::json!({
            "cache_id": "cache-1",
            "backend_hint": "llama_cpp"
        }),
    );
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "kv_cache_checkpoint_requested": true
        }),
    );

    let request = build_text_generation_execution_request(&inputs)
        .expect("kv cache request should build typed generation options");

    assert_eq!(
        request.generation_options,
        Some(GenerationOptions {
            cache: CacheGenerationOptions {
                use_cache: Some(true),
                kv_cache_checkpoint_requested: Some(true),
            },
            ..GenerationOptions::default()
        })
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_accepts_legacy_flat_generation_options() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "generation_options".to_string(),
        serde_json::json!({
            "temperature": 0.5,
            "max_new_tokens": 24,
            "sampling": {"temperature": 0.25}
        }),
    );

    let request = build_text_generation_execution_request(&inputs)
        .expect("legacy flat generation options should normalize");

    assert_eq!(
        request.generation_options,
        Some(GenerationOptions {
            length: LengthGenerationOptions {
                max_new_tokens: Some(24),
                ..LengthGenerationOptions::default()
            },
            sampling: SamplingGenerationOptions {
                temperature: Some(0.25),
                ..SamplingGenerationOptions::default()
            },
            ..GenerationOptions::default()
        })
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_uses_resolved_model_source_ref() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_value("pumas://models/tiny-gguf", "/models/tiny/model.gguf"),
    );

    let request = build_text_generation_execution_request(&inputs)
        .expect("resolved model source should provide model identity");

    assert_eq!(
        request.model_ref,
        Some(PumasModelRef {
            model_id: "pumas://models/tiny-gguf".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        })
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_forwards_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_text_generation_execution_request(&inputs)
        .expect("package facts should be forwarded to typed request");

    assert_eq!(
        request
            .resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.model_id.as_str()),
        Some("llm/llama/tiny-gguf")
    );
    assert_eq!(request.model_ref, Some(package_facts.model_ref));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_rejects_malformed_package_facts() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::json!({
            "contract_version": "pantograph.inference.package-facts.v1"
        }),
    );

    let err = build_text_generation_execution_request(&inputs)
        .expect_err("malformed package facts should fail explicitly");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid resolved_model_package_facts input"));
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_defaults_missing_task_kind_to_text_generation() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let request = build_text_generation_execution_request(&inputs)
        .expect("missing task kind should preserve the existing text default");

    assert_eq!(request.task_id, InferenceTaskId::TextGeneration);
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
#[test]
fn test_build_text_generation_execution_request_rejects_unknown_task_kind() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("object-detection"),
    );

    let error = build_text_generation_execution_request(&inputs)
        .expect_err("unknown task kind should fail before backend execution");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Unsupported text generation task_kind"));
            assert!(message.contains("object-detection"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_rejects_non_string_task_kind() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("task_kind".to_string(), serde_json::json!({"task": "text"}));

    let error = build_text_generation_execution_request(&inputs)
        .expect_err("present non-string task kind should fail before backend execution");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid text generation task kind input"));
            assert!(message.contains("expected string"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_rejects_non_text_task_kind() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));

    let error = build_text_generation_execution_request(&inputs)
        .expect_err("non-text task kind should not silently become text generation");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("cannot be executed by the text generation node"));
            assert!(message.contains("embedding"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_text_generation_execution_request_rejects_image_generation_task_contract() {
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("draw a diagram"));
    inputs.insert("task_kind".to_string(), serde_json::json!("text-to-image"));

    let error = build_text_generation_execution_request(&inputs)
        .expect_err("image generation task contract should not become text generation");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("cannot be executed by the text generation node"));
            assert!(message.contains("image_generation"));
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
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));
    inputs.insert(
        "kv_cache_in".to_string(),
        serde_json::json!({
            "cache_id": "cache-1",
            "backend_hint": "mock"
        }),
    );
    inputs.insert(
        "generation_options".to_string(),
        serde_json::json!({
            "length": {"max_new_tokens": 16},
            "sampling": {
                "temperature": 0.2,
                "top_p": 0.8,
                "top_k": 40,
                "seed": 42
            },
            "stopping": {"stop_strings": ["END"]}
        }),
    );

    let extensions = ExecutorExtensions::new();
    let outputs = execute_llm_inference(
        Some(&gateway),
        &inputs,
        "llm-inference-1",
        None,
        "exec-a",
        &extensions,
    )
    .await
    .expect("typed non-streaming inference should execute");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert!(outputs.get("stream").is_some_and(|value| value.is_null()));
    assert_eq!(
        outputs.get("usage"),
        Some(&serde_json::json!({
            "prompt_tokens": 7,
            "completion_tokens": 2,
            "total_tokens": 9
        }))
    );
    assert_eq!(
        outputs.get("kv_cache_out"),
        Some(&serde_json::json!({
            "cache_id": "kv-typed-text"
        }))
    );

    let captured = requests.lock().expect("requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0]["model"], serde_json::json!("typed-model"));
    assert_eq!(captured[0]["max_tokens"], serde_json::json!(16));
    assert_eq!(captured[0]["temperature"], serde_json::json!(0.2));
    assert_eq!(captured[0]["top_p"], serde_json::json!(0.8));
    assert_eq!(captured[0]["top_k"], serde_json::json!(40));
    let diagnostics = outputs["diagnostics"]
        .as_array()
        .expect("diagnostics output should be an array");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["option_path"] == serde_json::json!("sampling.seed")
            && diagnostic["state"] == serde_json::json!("unsupported")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["option_path"] == serde_json::json!("length.max_new_tokens")
            && diagnostic["state"] == serde_json::json!("mapped")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic["option_path"] == serde_json::json!("cache.use_cache")
            && diagnostic["state"] == serde_json::json!("requires_backend_support")
    }));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_execute_llm_inference_streaming_uses_gateway_stream_boundary() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let event_sink = Arc::new(crate::events::VecEventSink::new());
    let event_sink_trait: Arc<dyn crate::events::EventSink> = event_sink.clone();
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));

    let outputs = execute_llm_inference(
        Some(&gateway),
        &inputs,
        "llm-inference-1",
        Some(&event_sink_trait),
        "exec-a",
        &extensions,
    )
    .await
    .expect("streaming inference should execute through gateway stream");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert_eq!(
        outputs.get("usage"),
        Some(&serde_json::json!({
            "prompt_tokens": 7,
            "completion_tokens": 2,
            "total_tokens": 9
        }))
    );
    assert_eq!(
        outputs.get("kv_cache_out"),
        Some(&serde_json::json!({
            "cache_id": "kv-typed-text"
        }))
    );
    let captured = requests.lock().expect("requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0]["model"], serde_json::json!("typed-model"));
    assert_eq!(captured[0]["stream"], serde_json::json!(true));
    let stream_events = event_sink.events();
    assert_eq!(stream_events.len(), 1);
    match &stream_events[0] {
        crate::WorkflowEvent::TaskStream {
            task_id,
            execution_id,
            port,
            data,
            ..
        } => {
            assert_eq!(task_id, "llm-inference-1");
            assert_eq!(execution_id, "exec-a");
            assert_eq!(port, "response");
            assert_eq!(data, &serde_json::json!("typed response"));
        }
        other => panic!("expected task stream event, got {other:?}"),
    }
    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 15);
    assert_eq!(events[0].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[6].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(events[6].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[7].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[7].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(
        events[0].request_id.as_deref(),
        Some("exec-a:llm-inference-1:text_generation")
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_execute_llm_inference_streaming_drops_path_shaped_cache_output() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("/tmp/private/kv-stream.bin".to_string()),
        }),
        "mock",
    ));
    let event_sink = Arc::new(crate::events::VecEventSink::new());
    let event_sink_trait: Arc<dyn crate::events::EventSink> = event_sink.clone();
    let extensions = ExecutorExtensions::new();
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));

    let outputs = execute_llm_inference(
        Some(&gateway),
        &inputs,
        "llm-inference-1",
        Some(&event_sink_trait),
        "exec-a",
        &extensions,
    )
    .await
    .expect("streaming inference should execute through gateway stream");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert!(!outputs.contains_key("kv_cache_out"));
    assert_eq!(requests.lock().expect("requests lock").len(), 1);
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_text_uses_typed_lifecycle_sink_extension() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("typed non-streaming inference should execute with lifecycle sink");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 15);
    assert_eq!(events[0].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].phase, InferenceLifecyclePhase::TaskValidation);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[3].phase, InferenceLifecyclePhase::Preprocessing);
    assert_eq!(events[3].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[6].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(events[6].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[7].phase, InferenceLifecyclePhase::BackendExecution);
    assert_eq!(
        events[7].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[9].phase, InferenceLifecyclePhase::Postprocessing);
    assert_eq!(events[12].phase, InferenceLifecyclePhase::ResultProjection);
    assert!(events.iter().all(|event| {
        event.request_id.as_deref() == Some("exec-a:llm-inference-1:text_generation")
            && event.backend_key.as_deref() == Some("mock")
            && event.model_id.as_deref() == Some("typed-model")
    }));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_text_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("text package facts should execute through typed lifecycle");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert_eq!(requests.lock().expect("requests lock").len(), 1);

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 18);
    let package_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::ModelPackageResolution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("package resolution completion");
    assert_eq!(
        package_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(package_completed.compatibility_report.is_none());

    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(validation_completed.compatibility_report.is_some());
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
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(backend_completed.compatibility_report.is_some());
    assert!(backend_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("llm/llama/tiny-gguf")));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_streaming_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let event_sink = Arc::new(crate::events::VecEventSink::new());
    let event_sink_trait: Arc<dyn crate::events::EventSink> = event_sink.clone();
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("model_name".to_string(), serde_json::json!("typed-model"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let outputs = execute_llm_inference(
        Some(&gateway),
        &inputs,
        "llm-inference-1",
        Some(&event_sink_trait),
        "exec-a",
        &extensions,
    )
    .await
    .expect("streaming text package facts should execute through typed lifecycle");

    assert_eq!(
        outputs.get("response").and_then(|value| value.as_str()),
        Some("typed response")
    );
    assert_eq!(requests.lock().expect("requests lock").len(), 1);
    let stream_events = event_sink.events();
    assert_eq!(stream_events.len(), 1);

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 18);
    let package_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::ModelPackageResolution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("package resolution completion");
    assert_eq!(
        package_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(package_completed.compatibility_report.is_none());

    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(validation_completed.compatibility_report.is_some());
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
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("llm/llama/tiny-gguf")
    );
    assert!(backend_completed.compatibility_report.is_some());
    assert!(backend_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("llm/llama/tiny-gguf")));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_text_rejects_package_task_mismatch_before_backend() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("embedding package facts fixture");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedTextBackend {
            requests: requests.clone(),
            cache_handle_on_terminal: Some("kv-typed-text".to_string()),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let err = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect_err("text node should reject embedding package facts");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Typed LLM inference failed"));
            assert!(message.contains("TextGeneration"));
            assert!(message.contains("Embedding"));
            assert!(message.contains("embedding/qwen3/tiny-embedding-gguf"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    assert!(
        requests.lock().expect("requests lock").is_empty(),
        "backend must not receive a request after package/task validation failure"
    );
    let events = lifecycle_events.lock().expect("lifecycle events lock");
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
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_embedding_execution_request_preserves_canonical_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    inputs.insert("model".to_string(), serde_json::json!("embed-model"));
    inputs.insert("runtime_hint".to_string(), serde_json::json!("llamacpp"));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/embed",
            "revision": "rev-1"
        }),
    );

    let request = build_embedding_execution_request(&inputs)
        .expect("canonical embedding request should build");

    assert_eq!(request.task_id, InferenceTaskId::Embedding);
    assert_eq!(request.model_name.as_deref(), Some("embed-model"));
    assert_eq!(request.runtime_hint.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.model_ref,
        Some(PumasModelRef {
            model_id: "pumas://models/embed".to_string(),
            revision: Some("rev-1".to_string()),
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        })
    );
    match request.input {
        InferenceExecutionInput::Embedding { texts } => {
            assert_eq!(texts, vec!["hello".to_string()]);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_embedding_execution_request_forwards_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("embedding package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_embedding_execution_request(&inputs)
        .expect("embedding package facts should be forwarded to typed request");

    assert_eq!(
        request
            .resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.model_id.as_str()),
        Some("embedding/qwen3/tiny-embedding-gguf")
    );
    assert_eq!(request.model_ref, Some(package_facts.model_ref));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_embedding_execution_request_rejects_empty_text() {
    let mut inputs = HashMap::new();
    inputs.insert("text".to_string(), serde_json::json!("  "));

    let error = build_embedding_execution_request(&inputs)
        .expect_err("empty embedding text should fail before backend execution");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Embedding input text cannot be empty"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_typed_result_projection_rejects_result_kind_mismatch() {
    let result = inference::InferenceExecutionResult::TextGeneration {
        text: "not an embedding".to_string(),
        usage: None,
        cache_handle_id: None,
        option_diagnostics: Vec::new(),
    };

    let error = ensure_typed_result_kind(
        &result,
        inference::InferenceExecutionResultKind::Embedding,
        "Typed embedding inference",
    )
    .expect_err("mismatched result kind should fail before output projection");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("text_generation"));
            assert!(message.contains("embedding"));
            assert!(message.contains("task contract expected"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_preserves_canonical_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert(
        "documents".to_string(),
        serde_json::json!([
            "first",
            {"text": "second"},
            {"content": "third"}
        ]),
    );
    inputs.insert("top_k".to_string(), serde_json::json!(2));
    inputs.insert("return_documents".to_string(), serde_json::json!(false));
    inputs.insert("runtime_hint".to_string(), serde_json::json!("llamacpp"));
    inputs.insert("gpu_layers".to_string(), serde_json::json!(12));
    inputs.insert("temperature".to_string(), serde_json::json!(0.2));
    inputs.insert(
        "inference_settings".to_string(),
        serde_json::json!([
            {"key": "temperature"},
            {"key": "gpu_layers"}
        ]),
    );
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/reranker",
            "selected_artifact_path": "/tmp/reranker.gguf"
        }),
    );

    let request =
        build_rerank_execution_request(&inputs).expect("canonical rerank request should build");

    assert_eq!(request.task_id, InferenceTaskId::Rerank);
    assert_eq!(request.model_name.as_deref(), Some("/tmp/reranker.gguf"));
    assert_eq!(request.runtime_hint.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.model_ref,
        Some(PumasModelRef {
            model_id: "pumas://models/reranker".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: Some("/tmp/reranker.gguf".to_string()),
            migration_diagnostics: Vec::new(),
        })
    );
    match request.input {
        InferenceExecutionInput::Rerank {
            query,
            documents,
            top_n,
            return_documents,
        } => {
            assert_eq!(query, "search");
            assert_eq!(
                documents,
                vec![
                    "first".to_string(),
                    "second".to_string(),
                    "third".to_string()
                ]
            );
            assert_eq!(top_n, Some(2));
            assert!(!return_documents);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
    assert_eq!(request.extra_options["temperature"], serde_json::json!(0.2));
    assert!(request.extra_options.get("gpu_layers").is_none());
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_forwards_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/rerank_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("rerank package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert(
        "documents".to_string(),
        serde_json::json!(["first", "second"]),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_rerank_execution_request(&inputs)
        .expect("rerank package facts should be forwarded to typed request");

    assert_eq!(
        request
            .resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.model_id.as_str()),
        Some("rerank/bge/tiny-reranker-gguf")
    );
    assert_eq!(request.model_ref, Some(package_facts.model_ref));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_reads_nested_task_options() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert(
        "documents".to_string(),
        serde_json::json!(["first", "second", "third"]),
    );
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "top_k": 2,
            "return_documents": false
        }),
    );

    let request =
        build_rerank_execution_request(&inputs).expect("nested task options should build");

    match request.input {
        InferenceExecutionInput::Rerank {
            top_n,
            return_documents,
            ..
        } => {
            assert_eq!(top_n, Some(2));
            assert!(!return_documents);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_prefers_connected_options_over_task_options() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["first"]));
    inputs.insert("top_k".to_string(), serde_json::json!(1));
    inputs.insert("return_documents".to_string(), serde_json::json!(true));
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "top_k": 3,
            "return_documents": false
        }),
    );

    let request = build_rerank_execution_request(&inputs)
        .expect("connected rerank options should override saved task options");

    match request.input {
        InferenceExecutionInput::Rerank {
            top_n,
            return_documents,
            ..
        } => {
            assert_eq!(top_n, Some(1));
            assert!(return_documents);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_rejects_empty_query() {
    let mut inputs = HashMap::new();
    inputs.insert("query".to_string(), serde_json::json!("  "));
    inputs.insert("documents".to_string(), serde_json::json!(["a"]));

    let error = build_rerank_execution_request(&inputs)
        .expect_err("empty rerank query should fail before backend execution");

    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Reranker query cannot be empty"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_rerank_execution_request_rejects_malformed_package_facts() {
    let mut inputs = HashMap::new();
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["first"]));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::json!({
            "contract_version": "pantograph.inference.package-facts.v1"
        }),
    );

    let err = build_rerank_execution_request(&inputs)
        .expect_err("malformed package facts should fail explicitly");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid resolved_model_package_facts input"));
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_image_generation_execution_request_preserves_canonical_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("image_generation"),
    );
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paint a quiet lake"),
    );
    inputs.insert("runtime_hint".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "negative_prompt": "blur",
            "width": 512,
            "height": 384,
            "num_inference_steps": 12,
            "guidance_scale": 7.5,
            "seed": 42,
            "scheduler": "euler",
            "num_images_per_prompt": 2
        }),
    );
    inputs.insert("strength".to_string(), serde_json::json!(0.35));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/tiny-diffusion",
            "selected_artifact_path": "/models/tiny-diffusion"
        }),
    );

    let request = build_image_generation_execution_request(&inputs)
        .expect("canonical image generation request should build");

    assert_eq!(request.task_id, InferenceTaskId::ImageGeneration);
    assert_eq!(
        request.model_name.as_deref(),
        Some("/models/tiny-diffusion")
    );
    assert_eq!(request.runtime_hint.as_deref(), Some("pytorch"));
    assert_eq!(
        request.model_ref,
        Some(PumasModelRef {
            model_id: "pumas://models/tiny-diffusion".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: Some("/models/tiny-diffusion".to_string()),
            migration_diagnostics: Vec::new(),
        })
    );
    match request.input {
        InferenceExecutionInput::ImageGeneration { request } => {
            assert_eq!(request.model, "/models/tiny-diffusion");
            assert_eq!(request.prompt, "paint a quiet lake");
            assert_eq!(request.negative_prompt.as_deref(), Some("blur"));
            assert_eq!(request.width, Some(512));
            assert_eq!(request.height, Some(384));
            assert_eq!(request.num_inference_steps, Some(12));
            assert_eq!(request.guidance_scale, Some(7.5));
            assert_eq!(request.seed, Some(42));
            assert_eq!(request.scheduler.as_deref(), Some("euler"));
            assert_eq!(request.num_images_per_prompt, Some(2));
            assert_eq!(request.strength, Some(0.35));
            assert_eq!(request.extra_options, serde_json::json!({}));
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_image_generation_execution_request_forwards_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/diffusers_bundle_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("image package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("prompt".to_string(), serde_json::json!("paint"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_image_generation_execution_request(&inputs)
        .expect("image package facts should be forwarded to typed request");

    assert_eq!(
        request
            .resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.model_id.as_str()),
        Some("image/example/tiny-diffusers")
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_image_generation_uses_typed_gateway_boundary() {
    let image_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedImageGenerationBackend {
            image_requests: image_requests.clone(),
        }),
        "mock",
    ));
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("image_generation"),
    );
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paint a quiet lake SECRET_PROMPT"),
    );
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "negative_prompt": "blur",
            "width": 512,
            "height": 384,
            "num_inference_steps": 12,
            "guidance_scale": 7.5,
            "seed": 42,
            "scheduler": "euler",
            "num_images_per_prompt": 1
        }),
    );
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/tiny-diffusion",
            "selected_artifact_path": "/models/tiny-diffusion"
        }),
    );

    let executor = CoreTaskExecutor::new().with_gateway(gateway);
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let outputs = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect("canonical image generation inference should use typed gateway");

    assert_eq!(outputs["results"]["images"][0]["mime_type"], "image/png");
    assert_eq!(outputs["results"]["images"][0]["width"], 512);
    assert_eq!(outputs["metadata"]["seed_used"], 42);
    assert_eq!(outputs["metadata"]["image_count"], 1);
    let bounded_outputs = serde_json::to_string(&serde_json::json!({
        "metadata": outputs.get("metadata"),
        "diagnostics": outputs.get("diagnostics"),
    }))
    .expect("bounded outputs serialize");
    assert!(!bounded_outputs.contains("SECRET_PROMPT"));
    assert!(!bounded_outputs.contains("aW1hZ2U="));

    let captured = image_requests.lock().expect("image requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].model, "/models/tiny-diffusion");
    assert_eq!(captured[0].negative_prompt.as_deref(), Some("blur"));
    assert_eq!(captured[0].width, Some(512));
    assert_eq!(captured[0].height, Some(384));
    assert_eq!(captured[0].num_inference_steps, Some(12));
    assert_eq!(captured[0].guidance_scale, Some(7.5));
    assert_eq!(captured[0].seed, Some(42));
    assert_eq!(captured[0].scheduler.as_deref(), Some("euler"));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_image_generation_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/diffusers_bundle_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("image package facts fixture");
    let image_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedImageGenerationBackend {
            image_requests: image_requests.clone(),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("image_generation"),
    );
    inputs.insert(
        "prompt".to_string(),
        serde_json::json!("paint a quiet lake SECRET_PROMPT"),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-image".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("image package facts should execute through typed lifecycle");

    assert_eq!(outputs["metadata"]["image_count"], 1);
    assert_eq!(image_requests.lock().expect("image requests lock").len(), 1);

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 18);
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("image/example/tiny-diffusers")
    );
    assert!(validation_completed.compatibility_report.is_some());

    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("image/example/tiny-diffusers")
    );
    assert!(backend_completed.compatibility_report.is_some());

    let bounded_events = serde_json::to_string(&*events).expect("events serialize");
    assert!(!bounded_events.contains("SECRET_PROMPT"));
    assert!(!bounded_events.contains("aW1hZ2U="));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_embedding_uses_typed_gateway_boundary() {
    let embedding_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedEmbeddingBackend {
            embedding_requests: embedding_requests.clone(),
        }),
        "mock",
    ));
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    inputs.insert("model".to_string(), serde_json::json!("embed-model"));

    let executor = CoreTaskExecutor::new().with_gateway(gateway);
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let outputs = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect("canonical embedding inference should use typed gateway");

    assert_eq!(
        outputs.get("embedding"),
        Some(&serde_json::json!([0.25, 0.5, 0.75]))
    );
    assert_eq!(
        outputs.get("usage"),
        Some(&serde_json::json!({
            "prompt_tokens": 3,
            "total_tokens": 3
        }))
    );
    assert_eq!(outputs.get("diagnostics"), Some(&serde_json::json!([])));
    let bounded_outputs = serde_json::to_string(&serde_json::json!({
        "usage": outputs.get("usage"),
        "diagnostics": outputs.get("diagnostics"),
    }))
    .expect("bounded outputs serialize");
    assert!(!bounded_outputs.contains("hello"));
    assert!(!bounded_outputs.contains("0.25"));
    assert!(!bounded_outputs.contains("0.5"));
    assert!(!bounded_outputs.contains("0.75"));
    let captured = embedding_requests.lock().expect("embedding requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].0, vec!["hello".to_string()]);
    assert_eq!(captured[0].1, "embed-model");
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_embedding_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("embedding package facts fixture");
    let embedding_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedEmbeddingBackend {
            embedding_requests: embedding_requests.clone(),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("embedding"));
    inputs.insert("text".to_string(), serde_json::json!("hello"));
    inputs.insert("model".to_string(), serde_json::json!("embed-model"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("embedding package facts should execute through typed lifecycle");

    assert_eq!(
        outputs.get("embedding"),
        Some(&serde_json::json!([0.25, 0.5, 0.75]))
    );
    assert_eq!(
        embedding_requests
            .lock()
            .expect("embedding requests lock")
            .len(),
        1
    );

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 18);
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("embedding/qwen3/tiny-embedding-gguf")
    );
    assert!(validation_completed.compatibility_report.is_some());

    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("embedding/qwen3/tiny-embedding-gguf")
    );
    assert!(backend_completed.compatibility_report.is_some());
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
#[test]
fn test_build_audio_transcription_execution_request_preserves_canonical_inputs() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("automatic-speech-recognition"),
    );
    inputs.insert(
        "audio".to_string(),
        serde_json::json!({
            "data_base64": "UklGRg==",
            "mime_type": "audio/flac",
            "sample_rate_hz": 16000
        }),
    );
    inputs.insert("model_name".to_string(), serde_json::json!("whisper-tiny"));
    inputs.insert("runtime_hint".to_string(), serde_json::json!("pytorch"));
    inputs.insert("language".to_string(), serde_json::json!("en"));
    inputs.insert("prompt".to_string(), serde_json::json!("domain terms"));
    inputs.insert("asr_task".to_string(), serde_json::json!("transcribe"));
    inputs.insert("chunk_length_s".to_string(), serde_json::json!(30.0));

    let request = build_audio_transcription_execution_request(&inputs)
        .expect("canonical audio transcription request should build");

    assert_eq!(request.task_id, InferenceTaskId::AudioTranscription);
    assert_eq!(request.model_name.as_deref(), Some("whisper-tiny"));
    assert_eq!(request.runtime_hint.as_deref(), Some("pytorch"));
    match request.input {
        InferenceExecutionInput::AudioTranscription { request } => {
            assert_eq!(request.model, "whisper-tiny");
            let audio = request.audio.expect("encoded audio should be present");
            assert_eq!(audio.data_base64, "UklGRg==");
            assert_eq!(audio.mime_type, "audio/flac");
            assert_eq!(audio.sample_rate_hz, Some(16000));
            assert_eq!(request.audio_ref, None);
            assert_eq!(request.language.as_deref(), Some("en"));
            assert_eq!(request.prompt.as_deref(), Some("domain terms"));
            assert_eq!(request.task.as_deref(), Some("transcribe"));
            assert_eq!(request.chunk_length_s, Some(30.0));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_audio_transcription_execution_request_forwards_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/hf_audio_transcription_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("audio package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("audio_transcription"),
    );
    inputs.insert(
        "audio".to_string(),
        serde_json::json!("artifact-read://audio.wav"),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_audio_transcription_execution_request(&inputs)
        .expect("audio package facts should be forwarded to typed request");

    assert_eq!(
        request.model_name.as_deref(),
        Some("audio/whisper/tiny-asr")
    );
    assert_eq!(
        request
            .model_ref
            .as_ref()
            .map(|model_ref| model_ref.model_id.as_str()),
        Some("audio/whisper/tiny-asr")
    );
    assert_eq!(
        request
            .resolved_model_package_facts
            .as_ref()
            .map(|facts| facts.model_ref.model_id.as_str()),
        Some("audio/whisper/tiny-asr")
    );
    match request.input {
        InferenceExecutionInput::AudioTranscription { request } => {
            assert_eq!(request.model, "audio/whisper/tiny-asr");
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_audio_transcription_execution_request_accepts_artifact_ref() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("audio_transcription"),
    );
    inputs.insert(
        "audio".to_string(),
        serde_json::json!("artifact-read://audio.wav"),
    );
    inputs.insert("model".to_string(), serde_json::json!("whisper-tiny"));

    let request = build_audio_transcription_execution_request(&inputs)
        .expect("artifact audio refs should build");

    match request.input {
        InferenceExecutionInput::AudioTranscription { request } => {
            assert_eq!(request.audio, None);
            assert_eq!(
                request.audio_ref.as_deref(),
                Some("artifact-read://audio.wav")
            );
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_audio_transcription_execution_request_rejects_malformed_package_facts() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "audio".to_string(),
        serde_json::json!("artifact-read://audio.wav"),
    );
    inputs.insert("model".to_string(), serde_json::json!("whisper-tiny"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::json!({
            "contract_version": "pantograph.inference.package-facts.v1"
        }),
    );

    let err = build_audio_transcription_execution_request(&inputs)
        .expect_err("malformed package facts should fail explicitly");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Invalid resolved_model_package_facts input"));
        }
        other => panic!("unexpected input variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_audio_transcription_uses_typed_gateway_boundary() {
    let audio_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedAudioTranscriptionBackend {
            audio_requests: audio_requests.clone(),
        }),
        "mock-audio",
    ));
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("audio_transcription"),
    );
    inputs.insert("audio".to_string(), serde_json::json!("UklGRg=="));
    inputs.insert("model_name".to_string(), serde_json::json!("whisper-tiny"));
    inputs.insert("language".to_string(), serde_json::json!("en"));

    let executor = CoreTaskExecutor::new().with_gateway(gateway);
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &ExecutorExtensions::new(),
        )
        .await
        .expect("canonical audio transcription should use typed gateway");

    assert_eq!(
        outputs.get("response"),
        Some(&serde_json::json!("hello audio"))
    );
    assert_eq!(outputs.get("text"), Some(&serde_json::json!("hello audio")));
    assert_eq!(outputs.get("language"), Some(&serde_json::json!("en")));
    assert_eq!(
        outputs.get("duration_seconds"),
        Some(&serde_json::json!(1.25_f32))
    );
    assert_eq!(outputs.get("stream"), Some(&serde_json::Value::Null));
    let diagnostics = serde_json::to_string(outputs.get("diagnostics").unwrap()).unwrap();
    assert!(!diagnostics.contains("UklGRg=="));

    let captured = audio_requests.lock().expect("audio requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].model, "whisper-tiny");
    assert_eq!(captured[0].language.as_deref(), Some("en"));
    assert_eq!(
        captured[0]
            .audio
            .as_ref()
            .map(|audio| audio.data_base64.as_str()),
        Some("UklGRg==")
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_audio_transcription_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/hf_audio_transcription_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("audio package facts fixture");
    let audio_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedAudioTranscriptionBackend {
            audio_requests: audio_requests.clone(),
        }),
        "mock-audio",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("audio_transcription"),
    );
    inputs.insert("audio".to_string(), serde_json::json!("UklGRg=="));
    inputs.insert("model_name".to_string(), serde_json::json!("whisper-tiny"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("audio package facts should execute through typed lifecycle");

    assert_eq!(outputs.get("text"), Some(&serde_json::json!("hello audio")));
    assert_eq!(audio_requests.lock().expect("audio requests lock").len(), 1);

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert!(
        events.iter().any(|event| {
            event.phase == InferenceLifecyclePhase::ModelPackageResolution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
                && event.model_id.as_deref() == Some("audio/whisper/tiny-asr")
        }),
        "audio execution should emit model-package resolution lifecycle for package facts"
    );
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("audio/whisper/tiny-asr")
    );
    assert!(validation_completed.compatibility_report.is_some());

    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("audio/whisper/tiny-asr")
    );
    assert!(backend_completed.compatibility_report.is_some());
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_rerank_uses_typed_gateway_boundary() {
    let rerank_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedRerankBackend {
            rerank_requests: rerank_requests.clone(),
        }),
        "mock",
    ));
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["a", "b"]));
    inputs.insert("top_k".to_string(), serde_json::json!(1));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_path": "/tmp/reranker.gguf",
            "recommended_backend": "llamacpp"
        }),
    );

    let executor = CoreTaskExecutor::new().with_gateway(gateway);
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let outputs = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect("canonical rerank inference should use typed gateway");

    assert_eq!(outputs.get("top_document"), Some(&serde_json::json!("b")));
    assert_eq!(outputs.get("top_score"), Some(&serde_json::json!(0.9_f32)));
    assert_eq!(outputs.get("scores"), Some(&serde_json::json!([0.9_f32])));
    let diagnostics = outputs
        .get("diagnostics")
        .and_then(|value| value.as_array())
        .expect("rerank diagnostics output should be an array");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic
                .get("option_path")
                .and_then(|value| value.as_str())
                == Some("rerank.top_n")
        }),
        "rerank top_n option diagnostic should be projected to graph outputs"
    );
    let diagnostics_json =
        serde_json::to_string(diagnostics).expect("diagnostics should serialize");
    assert!(!diagnostics_json.contains("search"));
    assert!(!diagnostics_json.contains("\"a\""));
    assert!(!diagnostics_json.contains("\"b\""));
    let captured = rerank_requests.lock().expect("rerank requests lock");
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].model, "/tmp/reranker.gguf");
    assert_eq!(captured[0].query, "search");
    assert_eq!(
        captured[0].documents,
        vec!["a".to_string(), "b".to_string()]
    );
    assert_eq!(captured[0].top_n, Some(1));
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_rerank_with_package_facts_emits_compatibility_lifecycle() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/rerank_package_facts.json"
    );
    let package_facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("rerank package facts fixture");
    let rerank_requests = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockTypedRerankBackend {
            rerank_requests: rerank_requests.clone(),
        }),
        "mock",
    ));
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(
        crate::extensions::extension_keys::INFERENCE_LIFECYCLE_SINK,
        lifecycle_sink,
    );
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("task_kind".to_string(), serde_json::json!("rerank"));
    inputs.insert("query".to_string(), serde_json::json!("search"));
    inputs.insert("documents".to_string(), serde_json::json!(["a", "b"]));
    inputs.insert("top_k".to_string(), serde_json::json!(1));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_path": "rerank/bge/tiny-reranker-gguf/tiny-reranker-Q8_0.gguf",
            "recommended_backend": "llamacpp"
        }),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let executor = CoreTaskExecutor::new()
        .with_gateway(gateway)
        .with_execution_id("exec-a".to_string());
    let outputs = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect("rerank package facts should execute through typed lifecycle");

    assert_eq!(outputs.get("top_document"), Some(&serde_json::json!("b")));
    assert_eq!(outputs.get("top_score"), Some(&serde_json::json!(0.9_f32)));
    assert_eq!(
        rerank_requests.lock().expect("rerank requests lock").len(),
        1
    );

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert!(
        events.iter().any(|event| {
            event.phase == InferenceLifecyclePhase::ModelPackageResolution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
                && event.model_id.as_deref() == Some("rerank/bge/tiny-reranker-gguf")
        }),
        "rerank execution should emit model-package resolution lifecycle for package facts"
    );
    let validation_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::TaskValidation
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("task validation completion");
    assert_eq!(
        validation_completed.model_id.as_deref(),
        Some("rerank/bge/tiny-reranker-gguf")
    );
    assert!(validation_completed.compatibility_report.is_some());
    assert!(validation_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("rerank/bge/tiny-reranker-gguf")));

    let backend_completed = events
        .iter()
        .find(|event| {
            event.phase == InferenceLifecyclePhase::BackendExecution
                && event.kind == InferenceRequestLifecycleEventKind::Completed
        })
        .expect("backend execution completion");
    assert_eq!(
        backend_completed.model_id.as_deref(),
        Some("rerank/bge/tiny-reranker-gguf")
    );
    assert!(backend_completed.compatibility_report.is_some());
    assert!(backend_completed
        .compatibility_issues
        .iter()
        .all(|issue| issue.model_id.as_deref() == Some("rerank/bge/tiny-reranker-gguf")));
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
async fn test_canonical_llm_rejects_unresolved_migration_model_reference_before_gateway() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));
    inputs.insert("runtime_hint".to_string(), serde_json::json!("llamacpp"));
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/legacy.gguf"),
    );
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "status": "unresolved",
            "source": "legacy_llamacpp",
            "legacy_model_path": "/tmp/legacy.gguf"
        }),
    );

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("unresolved migrated model evidence should block execution");
    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("model reference is unresolved"));
            assert!(message.contains("legacy_llamacpp"));
            assert!(!message.contains("InferenceGateway not configured"));
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

#[cfg(all(feature = "inference-nodes", feature = "pytorch-nodes"))]
#[tokio::test]
async fn test_canonical_llm_pytorch_package_facts_dispatches_to_dependency_preflight() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );
    inputs.insert("model_path".to_string(), serde_json::json!("/tmp/model"));
    inputs.insert("prompt".to_string(), serde_json::json!("hello"));

    let executor = CoreTaskExecutor::new();
    let context = graph_flow::Context::new();
    let extensions = ExecutorExtensions::new();
    let err = executor
        .execute_task("llm-inference-1", inputs, &context, &extensions)
        .await
        .expect_err("canonical Transformers package facts should require dependency preflight");
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

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_dependency_preflight_records_lifecycle_failure_without_resolver() {
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::INFERENCE_LIFECYCLE_SINK, lifecycle_sink);

    let mut inputs = HashMap::new();
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_artifact_kind(
            "pumas://models/tiny-hf",
            "/models/tiny-hf",
            "hf_compatible_directory",
        ),
    );

    let context = DependencyPreflightLifecycleContext {
        task_id: "llm-inference-1".to_string(),
        execution_id: "exec-a".to_string(),
        task_label: "text_generation".to_string(),
        backend_key: Some("pytorch".to_string()),
        model_id: Some("pumas://models/tiny-hf".to_string()),
        resolved_artifact_kind: Some("hf_compatible_directory".to_string()),
    };

    let err = enforce_dependency_preflight_with_lifecycle(
        "llm-inference",
        &inputs,
        &extensions,
        Some(&context),
    )
    .await
    .expect_err("missing dependency resolver should fail");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("dependency resolver is not configured"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| {
        event.phase == InferenceLifecyclePhase::ModelPackageResolution
            && event.request_id.as_deref() == Some("exec-a:llm-inference-1:text_generation")
            && event.backend_key.as_deref() == Some("pytorch")
            && event.runtime_id.as_deref() == Some("pytorch")
            && event.model_id.as_deref() == Some("pumas://models/tiny-hf")
            && event.resolved_artifact_kind.as_deref() == Some("hf_compatible_directory")
    }));
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert!(events[1]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("dependency resolver is not configured")));
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_dependency_preflight_records_lifecycle_success_with_resolver() {
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let captured_requests = Arc::new(Mutex::new(Vec::new()));
    let resolver: Arc<dyn ModelDependencyResolver> =
        Arc::new(CapturingDependencyResolver { captured_requests });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::INFERENCE_LIFECYCLE_SINK, lifecycle_sink);
    extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("text-generation"),
    );
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_artifact_kind(
            "pumas://models/tiny-hf",
            "/models/tiny-hf",
            "hf_compatible_directory",
        ),
    );

    let context = DependencyPreflightLifecycleContext {
        task_id: "llm-inference-1".to_string(),
        execution_id: "exec-a".to_string(),
        task_label: "text_generation".to_string(),
        backend_key: Some("pytorch".to_string()),
        model_id: Some("pumas://models/tiny-hf".to_string()),
        resolved_artifact_kind: Some("hf_compatible_directory".to_string()),
    };

    let resolved = enforce_dependency_preflight_with_lifecycle(
        "llm-inference",
        &inputs,
        &extensions,
        Some(&context),
    )
    .await
    .expect("dependency resolver should allow canonical PyTorch preflight")
    .expect("resolver should return a model_ref");

    assert_eq!(resolved.engine, "pytorch");
    assert_eq!(resolved.model_id, "pumas://models/tiny-hf");

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| {
        event.phase == InferenceLifecyclePhase::ModelPackageResolution
            && event.request_id.as_deref() == Some("exec-a:llm-inference-1:text_generation")
            && event.backend_key.as_deref() == Some("pytorch")
            && event.runtime_id.as_deref() == Some("pytorch")
            && event.model_id.as_deref() == Some("pumas://models/tiny-hf")
            && event.resolved_artifact_kind.as_deref() == Some("hf_compatible_directory")
            && event.usage.is_none()
            && event.cache_handle_id.is_none()
            && event.artifact_refs.is_empty()
            && event.compatibility_report.is_none()
            && event.compatibility_issues.is_empty()
            && event.option_diagnostics.is_empty()
    }));
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[0].detail, None);
    assert_eq!(
        events[1].kind,
        InferenceRequestLifecycleEventKind::Completed
    );
    assert_eq!(events[1].detail, None);
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert_eq!(events[2].detail, None);
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_dependency_preflight_lifecycle_context_reads_resolved_artifact_kind() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_artifact_kind(
            "pumas://models/tiny-hf",
            "/models/tiny-hf",
            "hf_compatible_directory",
        ),
    );

    let context = dependency_preflight_lifecycle_context(
        &inputs,
        "llm-inference-1",
        "exec-a",
        Some("pytorch"),
    );

    assert_eq!(context.model_id.as_deref(), Some("pumas://models/tiny-hf"));
    assert_eq!(
        context.resolved_artifact_kind.as_deref(),
        Some("hf_compatible_directory")
    );
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_pytorch_typed_generation_settings_accepts_empty_settings() {
    let settings = HashMap::new();
    assert_eq!(
        pytorch_typed_generation_settings(&settings).expect("empty settings should be accepted"),
        None
    );
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_pytorch_typed_generation_settings_accepts_single_top_k() {
    let mut settings = HashMap::new();
    settings.insert("top_k".to_string(), serde_json::json!(40));
    assert_eq!(
        pytorch_typed_generation_settings(&settings).expect("top_k should be accepted"),
        Some(40)
    );
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_pytorch_typed_generation_settings_accepts_typed_top_p() {
    let mut settings = HashMap::new();
    settings.insert("top_p".to_string(), serde_json::json!(0.9));
    assert_eq!(
        pytorch_typed_generation_settings(&settings).expect("typed top_p should be accepted"),
        None
    );
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_pytorch_typed_generation_settings_rejects_invalid_top_p() {
    let mut settings = HashMap::new();
    settings.insert("top_p".to_string(), serde_json::json!("high"));
    let error =
        pytorch_typed_generation_settings(&settings).expect_err("invalid top_p should be rejected");
    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("PyTorch top_p must be numeric"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_pytorch_typed_generation_settings_rejects_custom_kwargs() {
    let mut settings = HashMap::new();
    settings.insert("top_k".to_string(), serde_json::json!(40));
    settings.insert("block_length".to_string(), serde_json::json!(32));
    let error =
        pytorch_typed_generation_settings(&settings).expect_err("custom kwargs should be rejected");
    match error {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("Unsupported PyTorch generation setting 'block_length'"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "pytorch-nodes")]
#[test]
fn test_resolved_artifact_kind_prefers_package_facts_before_model_source() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_artifact_kind(
            "pumas://models/tiny-hf",
            "/models/tiny-hf",
            "hf_compatible_directory",
        ),
    );
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    assert_eq!(
        read_resolved_artifact_kind_from_inputs(&inputs).as_deref(),
        Some("gguf")
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_depth_estimation_rejects_contract_only_with_lifecycle() {
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::INFERENCE_LIFECYCLE_SINK, lifecycle_sink);

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("depth_estimation"),
    );
    inputs.insert("runtime_hint".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/depth-estimation"
        }),
    );
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "output_format": "depth_map",
            "include_point_cloud": true
        }),
    );

    let executor = CoreTaskExecutor::new().with_execution_id("exec-a".to_string());
    let err = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect_err("contract-only depth task should fail before backend execution");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("depth_estimation"));
            assert!(message.contains("execution_supported=false"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| {
        event.phase == InferenceLifecyclePhase::TaskValidation
            && event.request_id.as_deref() == Some("exec-a:llm-inference-1:depth_estimation")
            && event.task_id.as_deref() == Some("depth_estimation")
            && event.backend_key.as_deref() == Some("pytorch")
            && event.runtime_id.as_deref() == Some("pytorch")
            && event.model_id.as_deref() == Some("pumas://models/depth-estimation")
    }));
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert!(events[1]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("execution_supported=false")));
    assert_eq!(events[1].option_diagnostics.len(), 2);
    assert!(events[1].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "depth_estimation.output_format"
            && diagnostic.state == inference::OptionSupportState::BackendUnavailable
            && diagnostic.backend_key.as_deref() == Some("pytorch")
    }));
    assert!(events[1].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "depth_estimation.include_point_cloud"
            && diagnostic.state == inference::OptionSupportState::BackendUnavailable
            && diagnostic.backend_key.as_deref() == Some("pytorch")
    }));
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events[0].option_diagnostics.is_empty());
    assert!(events[2].option_diagnostics.is_empty());
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
struct CapturingDependencyResolver {
    captured_requests: Arc<Mutex<Vec<ModelDependencyRequest>>>,
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
#[async_trait]
impl ModelDependencyResolver for CapturingDependencyResolver {
    async fn resolve_model_dependency_requirements(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyRequirements, String> {
        self.captured_requests
            .lock()
            .expect("captured dependency requests lock")
            .push(request.clone());
        Ok(model_dependency_requirements_for_request(&request))
    }

    async fn check_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyStatus, String> {
        let requirements = model_dependency_requirements_for_request(&request);
        Ok(ModelDependencyStatus {
            state: DependencyState::Ready,
            code: None,
            message: None,
            requirements,
            bindings: Vec::new(),
            checked_at: None,
        })
    }

    async fn install_dependencies(
        &self,
        request: ModelDependencyRequest,
    ) -> std::result::Result<ModelDependencyInstallResult, String> {
        Ok(ModelDependencyInstallResult {
            state: DependencyState::Ready,
            code: None,
            message: None,
            requirements: model_dependency_requirements_for_request(&request),
            bindings: Vec::new(),
            installed_at: None,
        })
    }

    async fn resolve_model_ref(
        &self,
        request: ModelDependencyRequest,
        _requirements: Option<ModelDependencyRequirements>,
    ) -> std::result::Result<Option<ModelRefV2>, String> {
        Ok(Some(ModelRefV2 {
            contract_version: 2,
            engine: request.backend_key.unwrap_or_else(|| "pytorch".to_string()),
            model_id: request
                .model_id
                .unwrap_or_else(|| request.model_path.clone()),
            model_path: request.model_path,
            task_type_primary: request
                .task_type_primary
                .unwrap_or_else(|| "text-generation".to_string()),
            dependency_bindings: Vec::new(),
            dependency_requirements_id: Some("requirements.pytorch.hf".to_string()),
        }))
    }
}

#[cfg(any(feature = "inference-nodes", feature = "audio-nodes"))]
fn model_dependency_requirements_for_request(
    request: &ModelDependencyRequest,
) -> ModelDependencyRequirements {
    ModelDependencyRequirements {
        model_id: request
            .model_id
            .clone()
            .unwrap_or_else(|| request.model_path.clone()),
        platform_key: "linux-x86_64".to_string(),
        backend_key: request.backend_key.clone(),
        dependency_contract_version: 1,
        validation_state: DependencyValidationState::Resolved,
        validation_errors: Vec::new(),
        bindings: Vec::new(),
        selected_binding_ids: request.selected_binding_ids.clone(),
    }
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_dependency_preflight_maps_hf_transformers_source_to_pytorch_request() {
    let captured_requests = Arc::new(Mutex::new(Vec::new()));
    let resolver: Arc<dyn ModelDependencyResolver> = Arc::new(CapturingDependencyResolver {
        captured_requests: captured_requests.clone(),
    });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::MODEL_DEPENDENCY_RESOLVER, resolver);

    let mut inputs = HashMap::new();
    inputs.insert(
        "runtime_hint".to_string(),
        serde_json::json!("transformers_pytorch"),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("text-generation"),
    );
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_artifact_kind(
            "pumas://models/tiny-hf",
            "/models/tiny-hf",
            "hf_compatible_directory",
        ),
    );

    let resolved = enforce_dependency_preflight("llm-inference", &inputs, &extensions)
        .await
        .expect("HF-compatible Transformers/PyTorch preflight should resolve")
        .expect("resolver should return a model_ref");

    assert_eq!(resolved.engine, "pytorch");
    assert_eq!(resolved.model_id, "pumas://models/tiny-hf");
    assert_eq!(resolved.model_path, "/models/tiny-hf");
    assert_eq!(resolved.task_type_primary, "text-generation");

    let requests = captured_requests
        .lock()
        .expect("captured dependency requests lock");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.node_type, "llm-inference");
    assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
    assert_eq!(request.model_id.as_deref(), Some("pumas://models/tiny-hf"));
    assert_eq!(request.model_path, "/models/tiny-hf");
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("text-generation")
    );
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
fn test_inputs_with_model_path_preserves_resolved_mmproj_companion() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        resolved_model_source_with_companion_artifacts(
            "pumas://models/tiny-vlm-gguf",
            "/models/tiny-vlm/model.gguf",
            vec![
                "/models/tiny-vlm/readme.txt",
                "/models/tiny-vlm/mmproj-model-f16.mmproj",
            ],
        ),
    );

    let canonical =
        inputs_with_model_path_from_ref(&inputs).expect("resolved model source should parse");

    assert_eq!(
        canonical.get("model_path").and_then(|value| value.as_str()),
        Some("/models/tiny-vlm/model.gguf")
    );
    assert_eq!(
        canonical
            .get("mmproj_path")
            .and_then(|value| value.as_str()),
        Some("/models/tiny-vlm/mmproj-model-f16.mmproj")
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
fn test_inputs_with_model_path_rejects_unresolved_pumas_model_ref() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "model_path".to_string(),
        serde_json::json!("/tmp/legacy.gguf"),
    );
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "status": "unresolved",
            "source": "legacy_llamacpp",
            "legacy_model_path": "/tmp/legacy.gguf"
        }),
    );

    let err = inputs_with_model_path_from_ref(&inputs)
        .expect_err("unresolved Pumas model reference should fail explicitly");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("pumas_model_ref"));
            assert!(message.contains("legacy_llamacpp"));
            assert!(message.contains("Resolve this model through Pumas"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_inputs_with_model_path_rejects_unresolved_model_source() {
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_source".to_string(),
        serde_json::json!({
            "status": "unresolved",
            "source": "legacy_pytorch",
            "legacy_model_type": "causal_lm"
        }),
    );

    let err = inputs_with_model_path_from_ref(&inputs)
        .expect_err("unresolved model source should fail explicitly before serde parsing");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("resolved_model_source"));
            assert!(message.contains("legacy_pytorch"));
            assert!(message.contains("Resolve this model through Pumas"));
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

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_dependency_request_uses_package_facts_before_llm_heuristics() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);

    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("text_generation")
    );
    assert_eq!(request.model_id.as_deref(), Some("llm/llama/tiny-gguf"));
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_dependency_request_keeps_explicit_backend_before_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("text package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert("backend_key".to_string(), serde_json::json!("pytorch"));
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/model.gguf", &inputs);

    assert_eq!(request.backend_key.as_deref(), Some("pytorch"));
    assert_eq!(
        request.task_type_primary.as_deref(),
        Some("text_generation")
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_dependency_request_uses_embedding_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/gguf_embedding_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("embedding package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/embed.gguf", &inputs);

    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(request.task_type_primary.as_deref(), Some("embedding"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("embedding/qwen3/tiny-embedding-gguf")
    );
}

#[cfg(feature = "inference-nodes")]
#[test]
fn test_build_model_dependency_request_uses_rerank_package_facts() {
    let fixture = include_str!(
        "../../../inference/tests/fixtures/inference_package_facts/rerank_package_facts.json"
    );
    let package_facts: inference::ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("rerank package facts fixture");
    let mut inputs = HashMap::new();
    inputs.insert(
        "resolved_model_package_facts".to_string(),
        serde_json::to_value(&package_facts).expect("package facts json"),
    );

    let request = build_model_dependency_request("llm-inference", "/tmp/rerank.gguf", &inputs);

    assert_eq!(request.backend_key.as_deref(), Some("llamacpp"));
    assert_eq!(request.task_type_primary.as_deref(), Some("reranking"));
    assert_eq!(
        request.model_id.as_deref(),
        Some("rerank/bge/tiny-reranker-gguf")
    );
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

#[cfg(feature = "inference-nodes")]
#[test]
fn test_canonical_inference_input_kind_uses_task_request_contract() {
    let mut inputs = HashMap::new();
    inputs.insert("task_kind".to_string(), serde_json::json!("text-to-image"));

    assert_eq!(
        canonical_inference_input_kind(&inputs),
        Some(inference::InferenceExecutionInputKind::ImageGeneration)
    );
}

#[cfg(feature = "inference-nodes")]
#[tokio::test]
async fn test_canonical_llm_video_understanding_rejects_contract_only_with_lifecycle() {
    let lifecycle_events = Arc::new(Mutex::new(Vec::new()));
    let lifecycle_sink: Arc<dyn InferenceRequestLifecycleEventSink> =
        Arc::new(MockInferenceLifecycleSink {
            events: lifecycle_events.clone(),
        });
    let mut extensions = ExecutorExtensions::new();
    extensions.set(extension_keys::INFERENCE_LIFECYCLE_SINK, lifecycle_sink);

    let mut inputs = HashMap::new();
    inputs.insert(
        "_data".to_string(),
        serde_json::json!({"node_type": "llm-inference"}),
    );
    inputs.insert(
        "task_kind".to_string(),
        serde_json::json!("video_understanding"),
    );
    inputs.insert("runtime_hint".to_string(), serde_json::json!("vllm"));
    inputs.insert(
        "pumas_model_ref".to_string(),
        serde_json::json!({
            "model_id": "pumas://models/video-understanding"
        }),
    );
    inputs.insert(
        "task_options".to_string(),
        serde_json::json!({
            "max_frames": 16,
            "frame_sample_rate": 2
        }),
    );

    let executor = CoreTaskExecutor::new().with_execution_id("exec-a".to_string());
    let err = executor
        .execute_task(
            "llm-inference-1",
            inputs,
            &graph_flow::Context::new(),
            &extensions,
        )
        .await
        .expect_err("contract-only video task should fail before backend execution");

    match err {
        NodeEngineError::ExecutionFailed(message) => {
            assert!(message.contains("video_understanding"));
            assert!(message.contains("execution_supported=false"));
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    let events = lifecycle_events.lock().expect("lifecycle events lock");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| {
        event.phase == InferenceLifecyclePhase::TaskValidation
            && event.request_id.as_deref() == Some("exec-a:llm-inference-1:video_understanding")
            && event.task_id.as_deref() == Some("video_understanding")
            && event.backend_key.as_deref() == Some("vllm")
            && event.runtime_id.as_deref() == Some("vllm")
            && event.model_id.as_deref() == Some("pumas://models/video-understanding")
    }));
    assert_eq!(events[0].kind, InferenceRequestLifecycleEventKind::Started);
    assert_eq!(events[1].kind, InferenceRequestLifecycleEventKind::Failed);
    assert!(events[1]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("execution_supported=false")));
    assert_eq!(events[1].option_diagnostics.len(), 2);
    assert!(events[1].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "video_understanding.max_frames"
            && diagnostic.state == inference::OptionSupportState::BackendUnavailable
            && diagnostic.backend_key.as_deref() == Some("vllm")
    }));
    assert!(events[1].option_diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "video_understanding.frame_sample_rate"
            && diagnostic.state == inference::OptionSupportState::BackendUnavailable
            && diagnostic.backend_key.as_deref() == Some("vllm")
    }));
    assert_eq!(
        events[2].kind,
        InferenceRequestLifecycleEventKind::CleanupCompleted
    );
    assert!(events[0].option_diagnostics.is_empty());
    assert!(events[2].option_diagnostics.is_empty());
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
    resolved_model_source_with_artifact_kind(model_id, entry_path, "gguf")
}

#[cfg(feature = "inference-nodes")]
fn resolved_model_source_with_artifact_kind(
    model_id: &str,
    entry_path: &str,
    artifact_kind: &str,
) -> serde_json::Value {
    serde_json::json!({
        "source_contract_version": 1,
        "source_kind": "pumas_resolved",
        "artifact_kind": artifact_kind,
        "entry_path": entry_path,
        "storage_kind": "library_owned",
        "validation_state": "valid",
        "model_ref": {
            "model_id": model_id
        }
    })
}

#[cfg(feature = "inference-nodes")]
fn resolved_model_source_with_companion_artifacts(
    model_id: &str,
    entry_path: &str,
    companion_artifacts: Vec<&str>,
) -> serde_json::Value {
    let mut value = resolved_model_source_value(model_id, entry_path);
    value["companion_artifacts"] = serde_json::json!(companion_artifacts);
    value
}

#[cfg(feature = "inference-nodes")]
struct MockTypedTextBackend {
    requests: Arc<Mutex<Vec<serde_json::Value>>>,
    cache_handle_on_terminal: Option<String>,
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
                usage: None,
                cache_handle_id: None,
            }),
            Ok(ChatChunk {
                content: None,
                done: true,
                usage: Some(InferenceUsage {
                    prompt_tokens: Some(7),
                    completion_tokens: Some(2),
                    total_tokens: Some(9),
                }),
                cache_handle_id: self.cache_handle_on_terminal.clone(),
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

#[cfg(feature = "inference-nodes")]
struct MockTypedEmbeddingBackend {
    embedding_requests: Arc<Mutex<Vec<(Vec<String>, String)>>>,
}

#[cfg(feature = "inference-nodes")]
#[async_trait]
impl InferenceBackend for MockTypedEmbeddingBackend {
    fn name(&self) -> &'static str {
        "mock-embedding"
    }

    fn description(&self) -> &'static str {
        "Mock typed embedding backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            embeddings: true,
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
        _request_json: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        Err(BackendError::Inference(
            "chat not supported by mock".to_string(),
        ))
    }

    async fn embeddings(
        &self,
        texts: Vec<String>,
        model: &str,
    ) -> std::result::Result<Vec<EmbeddingResult>, BackendError> {
        self.embedding_requests
            .lock()
            .expect("embedding requests lock")
            .push((texts, model.to_string()));
        Ok(vec![EmbeddingResult {
            vector: vec![0.25, 0.5, 0.75],
            token_count: 3,
        }])
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

#[cfg(feature = "inference-nodes")]
struct MockTypedRerankBackend {
    rerank_requests: Arc<Mutex<Vec<RerankRequest>>>,
}

#[cfg(feature = "inference-nodes")]
#[async_trait]
impl InferenceBackend for MockTypedRerankBackend {
    fn name(&self) -> &'static str {
        "mock-rerank"
    }

    fn description(&self) -> &'static str {
        "Mock typed rerank backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            reranking: true,
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
        _request_json: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        Err(BackendError::Inference(
            "chat not supported by mock".to_string(),
        ))
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
        request: RerankRequest,
    ) -> std::result::Result<RerankResponse, BackendError> {
        self.rerank_requests
            .lock()
            .expect("rerank requests lock")
            .push(request.clone());
        let result = RerankResult {
            index: 1,
            score: 0.9,
            document: request.documents.get(1).cloned(),
        };
        Ok(RerankResponse {
            results: vec![result]
                .into_iter()
                .take(request.top_n.unwrap_or(usize::MAX))
                .collect(),
            metadata: serde_json::Value::Null,
        })
    }
}

#[cfg(feature = "inference-nodes")]
struct MockTypedImageGenerationBackend {
    image_requests: Arc<Mutex<Vec<ImageGenerationRequest>>>,
}

#[cfg(feature = "inference-nodes")]
#[async_trait]
impl InferenceBackend for MockTypedImageGenerationBackend {
    fn name(&self) -> &'static str {
        "mock-image"
    }

    fn description(&self) -> &'static str {
        "Mock typed image generation backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            image_generation: true,
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
        _request_json: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        Err(BackendError::Inference(
            "chat not supported by mock".to_string(),
        ))
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

    async fn generate_image(
        &self,
        request: ImageGenerationRequest,
    ) -> std::result::Result<ImageGenerationResult, BackendError> {
        self.image_requests
            .lock()
            .expect("image requests lock")
            .push(request.clone());
        Ok(ImageGenerationResult {
            images: vec![EncodedImage {
                data_base64: "aW1hZ2U=".to_string(),
                mime_type: "image/png".to_string(),
                width: request.width,
                height: request.height,
            }],
            seed_used: request.seed,
            metadata: serde_json::json!({"scheduler": request.scheduler}),
        })
    }
}

#[cfg(feature = "inference-nodes")]
struct MockTypedAudioTranscriptionBackend {
    audio_requests: Arc<Mutex<Vec<AudioTranscriptionRequest>>>,
}

#[cfg(feature = "inference-nodes")]
#[async_trait]
impl InferenceBackend for MockTypedAudioTranscriptionBackend {
    fn name(&self) -> &'static str {
        "mock-audio"
    }

    fn description(&self) -> &'static str {
        "Mock typed audio transcription backend"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::default()
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
        _request_json: String,
    ) -> std::result::Result<
        Pin<
            Box<
                dyn futures_util::Stream<Item = std::result::Result<ChatChunk, BackendError>>
                    + Send,
            >,
        >,
        BackendError,
    > {
        Err(BackendError::Inference(
            "chat not supported by mock".to_string(),
        ))
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

    async fn transcribe_audio(
        &self,
        request: AudioTranscriptionRequest,
    ) -> std::result::Result<AudioTranscriptionResult, BackendError> {
        self.audio_requests
            .lock()
            .expect("audio requests lock")
            .push(request);
        Ok(AudioTranscriptionResult {
            text: "hello audio".to_string(),
            language: Some("en".to_string()),
            duration_seconds: Some(1.25),
            segments: vec![AudioTranscriptionSegment {
                text: "hello audio".to_string(),
                start_seconds: Some(0.0),
                end_seconds: Some(1.25),
            }],
            metadata: serde_json::json!({"backend": "mock"}),
        })
    }
}

#[cfg(feature = "inference-nodes")]
struct MockInferenceLifecycleSink {
    events: Arc<Mutex<Vec<InferenceRequestLifecycleEvent>>>,
}

#[cfg(feature = "inference-nodes")]
impl InferenceRequestLifecycleEventSink for MockInferenceLifecycleSink {
    fn record(
        &self,
        event: InferenceRequestLifecycleEvent,
    ) -> std::result::Result<(), inference::InferenceRequestLifecycleEventSinkError> {
        self.events
            .lock()
            .expect("lifecycle events lock")
            .push(event);
        Ok(())
    }
}
