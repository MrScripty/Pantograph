use std::collections::BTreeSet;
use std::ffi::CString;

use super::pytorch_worker_contract::{
    PyTorchGenerateTextRequest, PyTorchGenerateTextResult, PyTorchTransformersLoadRequest,
    PyTorchTransformersModelLoader, PyTorchTransformersTrustPolicy, PyTorchWorkerEnvelope,
    PyTorchWorkerErrorKind, PyTorchWorkerFailure, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::*;
use crate::model_contracts::{
    CacheGenerationOptions, GenerationOptions, LengthGenerationOptions, ModelArtifactKind,
    ModelAuthTokenSource, ModelLoadCachePolicy, ModelLoadNetworkPolicy, ModelLoadSecurityPolicy,
    ModelRemoteCodePolicy, OptionSupportState, OutputGenerationOptions, ProcessorComponentKind,
    PumasModelRef, ResolvedModelPackageFacts, ResolvedModelSourceKind, SamplingGenerationOptions,
    SpecialTokenGenerationOptions, StoppingGenerationOptions, TaskEvidence,
};
use crate::types::{AudioTranscriptionRequest, EncodedAudio};

fn load_worker_contract_module<'py>(py: Python<'py>) -> Bound<'py, pyo3::types::PyModule> {
    let source = CString::new(include_str!("../../torch/worker_contract.py"))
        .expect("worker contract source should not contain nul bytes");
    pyo3::types::PyModule::from_code(py, &source, c"worker_contract.py", c"worker_contract")
        .expect("worker_contract module should load")
}

#[test]
fn test_backend_name() {
    let backend = PyTorchBackend::new();
    assert_eq!(backend.name(), "PyTorch");
}

#[test]
fn test_capabilities() {
    let caps = PyTorchBackend::static_capabilities();
    assert!(!caps.vision);
    assert!(!caps.embeddings);
    assert!(caps.gpu);
    assert!(caps.device_selection);
    assert!(caps.streaming);
    assert!(!caps.tool_calling);
    assert!(caps.supports_task(InferenceTaskId::TextGeneration));
    assert!(caps.supports_task(InferenceTaskId::AudioTranscription));
    assert!(!caps.supports_task(InferenceTaskId::Embedding));
    assert_eq!(
        caps.facts.features.kv_cache,
        BackendFeatureSupport::Supported
    );
}

#[test]
fn test_not_ready_initially() {
    let backend = PyTorchBackend::new();
    assert!(!backend.is_ready());
    assert!(backend.base_url().is_none());
}

#[test]
fn test_no_loaded_model_initially() {
    let backend = PyTorchBackend::new();
    assert!(backend.loaded_model.is_none());
}

#[test]
fn test_can_reuse_loaded_model_requires_matching_request() {
    let mut backend = PyTorchBackend::new();
    backend.loaded_model = Some(LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "text-generation".to_string(),
        device: "cuda".to_string(),
    });

    assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", None));
    assert!(backend.can_reuse_loaded_model("/models/demo", "cuda", Some("text-generation")));
    assert!(!backend.can_reuse_loaded_model("/models/other", "cuda", None));
    assert!(!backend.can_reuse_loaded_model("/models/demo", "cpu", None));
    assert!(!backend.can_reuse_loaded_model("/models/demo", "cuda", Some("dllm")));
}

#[test]
fn test_kv_runtime_fingerprint_for_loaded_model_is_stable() {
    let loaded = LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };

    let fingerprint = PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded);
    assert_eq!(fingerprint.backend_key, "pytorch");
    assert_eq!(fingerprint.runtime_id, "pytorch");
    assert!(fingerprint.tokenizer_fingerprint.contains("/models/demo"));
    assert_eq!(
        fingerprint.prompt_format_fingerprint.as_deref(),
        Some("pytorch_dllm")
    );
    assert_eq!(
        fingerprint.runtime_build_fingerprint.as_deref(),
        Some("cuda")
    );
}

#[test]
fn test_kv_model_fingerprint_for_loaded_model_tracks_model_identity() {
    let loaded = LoadedModelInfo {
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };

    let fingerprint = PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded);
    assert_eq!(fingerprint.model_id, "/models/demo");
    assert_eq!(fingerprint.config_hash, "pytorch:dllm");
}

#[test]
fn test_require_live_kv_slot_rejects_nonzero_slots() {
    assert!(PyTorchBackend::require_live_kv_slot(0).is_ok());
    match PyTorchBackend::require_live_kv_slot(1) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("slot_id 0"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_live_kv_fingerprint_helpers_match_loaded_model_helpers() {
    let info = PyTorchLiveKvInfo {
        token_count: 42,
        model_path: "/models/demo".to_string(),
        model_type: "dllm".to_string(),
        device: "cuda".to_string(),
    };
    let loaded = LoadedModelInfo {
        model_path: info.model_path.clone(),
        model_type: info.model_type.clone(),
        device: info.device.clone(),
    };

    assert_eq!(
        kv_cache_runtime_fingerprint_for_live_kv(&info),
        PyTorchBackend::kv_cache_runtime_fingerprint_for_loaded_model(&loaded)
    );
    assert_eq!(
        kv_cache_model_fingerprint_for_live_kv(&info),
        PyTorchBackend::kv_cache_model_fingerprint_for_loaded_model(&loaded)
    );
}

#[test]
fn test_in_process_no_base_url() {
    let backend = PyTorchBackend::new();
    assert!(backend.base_url().is_none());
}

#[test]
fn test_extract_prompt() {
    let req = serde_json::json!({
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello!"}
        ]
    });
    assert_eq!(extract_prompt_from_messages(&req).unwrap(), "Hello!");
}

#[test]
fn test_extract_system_prompt() {
    let req = serde_json::json!({
        "messages": [
            {"role": "system", "content": "Be concise."},
            {"role": "user", "content": "Hi"}
        ]
    });
    assert_eq!(extract_system_prompt(&req), Some("Be concise.".to_string()));
}

#[test]
fn test_extract_system_prompt_missing() {
    let req = serde_json::json!({
        "messages": [{"role": "user", "content": "Hi"}]
    });
    assert_eq!(extract_system_prompt(&req), None);
}

#[test]
fn test_pytorch_worker_load_envelope_decodes_fixture() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest> =
        serde_json::from_str(fixture).expect("decode worker load fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-load-001");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert_eq!(
        envelope
            .payload
            .model_ref
            .as_ref()
            .map(|value| value.model_id.as_str()),
        Some("pumas://models/tiny-causal")
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("load fixture should carry resolved model source");
    assert!(model_source.validate_for_backend_load().is_ok());
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::PumasResolved
    );
    assert_eq!(
        model_source.model_ref.as_ref(),
        envelope.payload.model_ref.as_ref()
    );
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("load fixture should carry a canonical task profile");
    assert_eq!(task_profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(task_profile.canonical_task_label, "text_generation");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
    assert!(task_profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
    assert_eq!(envelope.payload.device.as_deref(), Some("cuda:0"));
    assert!(!envelope.payload.trust_policy.allow_remote_code);
    assert!(envelope.payload.trust_policy.local_files_only);
    assert_eq!(
        envelope.payload.trust_policy.cache_policy,
        ModelLoadCachePolicy::BackendDefault
    );
    assert_eq!(
        envelope.payload.trust_policy.auth_token_source,
        ModelAuthTokenSource::None
    );
    assert_eq!(
        envelope.payload.trust_policy.revision.as_deref(),
        Some("rev-1")
    );
    assert!(envelope.payload.trust_policy.code_revision.is_none());
    assert!(envelope.payload.trust_policy.accepted_sources.is_empty());

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("load fixture should validate");
}

#[test]
fn test_python_worker_contract_projects_task_profile_loader() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-asr-load",
            "operation": "load_transformers_model",
            "payload": {
                "model_ref": {"model_id": "asr/example/tiny"},
                "artifact_kind": "hf_compatible_directory",
                "entry_path": "/models/asr",
                "task_id": "audio_transcription",
                "task_profile": {
                    "task_id": "audio_transcription",
                    "canonical_task_label": "audio_transcription",
                    "loader": "automatic_speech_recognition",
                    "required_components": ["audio_feature_extractor", "tokenizer"]
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true,
                    "cache_policy": "backend_default",
                    "auth_token_source": "none"
                }
            }
        });

        let kwargs = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect("worker contract should project load kwargs");
        let loader = kwargs
            .get_item("loader")
            .expect("loader key should be readable")
            .extract::<String>()
            .expect("loader should be a string");

        assert_eq!(loader, "automatic_speech_recognition");
    });
}

#[test]
fn test_python_worker_contract_rejects_unsupported_task_profile_loader() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let envelope = serde_json::json!({
            "contract_version": PYTORCH_WORKER_CONTRACT_VERSION,
            "request_id": "req-invalid-load",
            "operation": "load_transformers_model",
            "payload": {
                "model_ref": {"model_id": "vision/example/tiny"},
                "artifact_kind": "hf_compatible_directory",
                "entry_path": "/models/vision",
                "task_id": "image_understanding",
                "task_profile": {
                    "task_id": "image_understanding",
                    "canonical_task_label": "image_understanding",
                    "loader": "image_to_text",
                    "required_components": ["image_processor", "tokenizer"]
                },
                "trust_policy": {
                    "allow_remote_code": false,
                    "local_files_only": true,
                    "cache_policy": "backend_default",
                    "auth_token_source": "none"
                }
            }
        });

        let error = module
            .call_method1(
                "load_transformers_model_kwargs_from_envelope",
                (envelope.to_string(),),
            )
            .expect_err("unsupported loaders should fail validation");

        assert!(error
            .to_string()
            .contains("Unsupported PyTorch worker Transformers loader: image_to_text"));
    });
}

#[test]
fn test_pytorch_worker_generate_text_envelope_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GenerateText);
    assert_eq!(
        envelope.payload.prompt,
        "Explain why bounded diagnostics matter."
    );
    assert_eq!(
        envelope.payload.system_prompt.as_deref(),
        Some("Be concise.")
    );
    assert_eq!(envelope.payload.max_tokens, 64);
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(40)
    );

    PyTorchBackend::validate_generate_text_envelope(&envelope)
        .expect("generate_text fixture should validate");
}

#[test]
fn test_pytorch_worker_generate_text_dllm_envelope_decodes_backend_local_controls() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_dllm_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker dLLM generate fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-dllm-001");
    assert_eq!(envelope.operation, PyTorchWorkerOperation::GenerateText);
    assert_eq!(
        envelope.payload.masked_prompt_json.as_deref(),
        Some("{\"segments\":[{\"kind\":\"known\",\"text\":\"Plan:\"},{\"kind\":\"mask\",\"token_count\":8}]}")
    );
    assert_eq!(envelope.payload.denoising_steps, Some(8));
    assert_eq!(envelope.payload.block_length, Some(64));
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(10)
    );

    PyTorchBackend::validate_generate_text_envelope(&envelope)
        .expect("dLLM generate_text fixture should validate");
}

#[test]
fn test_python_worker_contract_projects_dllm_generation_controls() {
    Python::with_gil(|py| {
        let module = load_worker_contract_module(py);
        let fixture = include_str!(
            "../../tests/fixtures/pytorch_worker_contract/generate_text_dllm_request.json"
        );

        let kwargs = module
            .call_method1("generate_text_kwargs_from_envelope", (fixture,))
            .expect("dLLM worker envelope should project to kwargs");

        let masked_prompt_json = kwargs
            .get_item("masked_prompt_json")
            .expect("masked prompt key should exist")
            .extract::<String>()
            .expect("masked prompt should be a string");
        let denoising_steps = kwargs
            .get_item("denoising_steps")
            .expect("denoising steps key should exist")
            .extract::<i64>()
            .expect("denoising steps should be an integer");
        let block_length = kwargs
            .get_item("block_length")
            .expect("block length key should exist")
            .extract::<i64>()
            .expect("block length should be an integer");

        assert!(masked_prompt_json.contains("\"mask\""));
        assert_eq!(denoising_steps, 8);
        assert_eq!(block_length, 64);
    });
}

#[test]
fn test_pytorch_worker_generate_text_stream_envelope_decodes_fixture() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_stream_request.json"
    );
    let envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker stream fixture");

    assert_eq!(envelope.contract_version, PYTORCH_WORKER_CONTRACT_VERSION);
    assert_eq!(envelope.request_id, "req-generate-stream-001");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::GenerateTextStream
    );
    assert!(envelope.cancellation.drop_stream_cancels);
    assert_eq!(
        envelope.payload.prompt,
        "Stream a short answer about adapters."
    );
    assert_eq!(
        envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(20)
    );

    PyTorchBackend::validate_generate_text_stream_envelope(&envelope)
        .expect("generate_text_stream fixture should validate");
}

#[test]
fn test_pytorch_worker_generate_text_response_decodes_fixture() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_response.json");
    let response: PyTorchWorkerResponse<PyTorchGenerateTextResult> =
        serde_json::from_str(fixture).expect("decode worker generate response fixture");

    match response {
        PyTorchWorkerResponse::Ok(success) => {
            assert_eq!(success.request_id, "req-generate-001");
            assert!(success.result.text.contains("Bounded diagnostics"));
            assert!(success.option_diagnostics.is_empty());
        }
        other => panic!("expected worker success response, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_success_response_returns_text() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-generate-ok",
        "result": {
            "text": "Generated through Transformers."
        }
    });

    let text = PyTorchBackend::generate_text_from_worker_response(&response.to_string())
        .expect("generate_text response decodes");

    assert_eq!(text, "Generated through Transformers.");
}

#[test]
fn test_pytorch_worker_generate_text_runtime_unavailable_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-generate-no-model",
        "error": {
            "kind": "runtime_unavailable",
            "message": "No model loaded. Call load_model() first.",
            "canonical_code": "pytorch_worker_generate_text_failed"
        }
    });

    match PyTorchBackend::generate_text_from_worker_response(&response.to_string()) {
        Err(BackendError::NotRunning(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_failure_normalizes_to_inference_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-generate-failed",
        "error": {
            "kind": "generation_failed",
            "message": "Transformers generation failed.",
            "canonical_code": "pytorch_worker_generation_failed"
        }
    });

    match PyTorchBackend::generate_text_from_worker_response(&response.to_string()) {
        Err(BackendError::Inference(message)) => {
            assert!(message.contains("pytorch_worker_generation_failed"));
            assert!(message.contains("req-generate-failed"));
            assert!(message.contains("Transformers generation failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_transport_error_normalizes_to_backend_error() {
    match PyTorchBackend::generate_text_worker_failure_from_message(
        "req-generate-transport",
        "PyTorch worker generate_text envelope failed: Python bridge failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-transport"));
            assert!(message.contains("Python bridge failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_transport_no_model_normalizes_to_not_running() {
    match PyTorchBackend::generate_text_worker_failure_from_message(
        "req-generate-no-model",
        "PyTorch worker generate_text envelope failed: No model loaded.".to_string(),
    ) {
        BackendError::NotRunning(message) => {
            assert!(message.contains("pytorch_worker_generate_text_failed"));
            assert!(message.contains("req-generate-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generate_text_request_threads_top_k_as_transformers_kwarg() {
    let request = PyTorchBackend::generate_text_request(
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(20),
        None,
    );

    assert_eq!(request.transformers_kwargs["top_k"], serde_json::json!(20));
    assert_eq!(request.prompt, "Explain adapters.");
    assert_eq!(request.system_prompt.as_deref(), Some("Be precise."));
}

#[test]
fn test_pytorch_generate_text_envelopes_thread_top_k_for_generate_and_stream() {
    let generate_envelope = PyTorchBackend::generate_text_envelope(
        "req-generate-top-k",
        PyTorchWorkerOperation::GenerateText,
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(33),
        None,
    );
    let stream_envelope = PyTorchBackend::generate_text_envelope(
        "req-stream-top-k",
        PyTorchWorkerOperation::GenerateTextStream,
        "Explain adapters.".to_string(),
        Some("Be precise.".to_string()),
        48,
        0.3,
        0.9,
        Some(33),
        None,
    );

    PyTorchBackend::validate_generate_text_envelope(&generate_envelope)
        .expect("generate envelope validates");
    PyTorchBackend::validate_generate_text_stream_envelope(&stream_envelope)
        .expect("stream envelope validates");
    assert_eq!(
        generate_envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(33)
    );
    assert_eq!(
        stream_envelope.payload.transformers_kwargs["top_k"],
        serde_json::json!(33)
    );
}

#[test]
fn test_pytorch_generate_text_envelope_rejects_unscoped_transformers_kwargs() {
    let mut generate_envelope = PyTorchBackend::generate_text_envelope(
        "req-generate-raw-kwarg",
        PyTorchWorkerOperation::GenerateText,
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );
    generate_envelope
        .payload
        .transformers_kwargs
        .insert("raw_max_batch_size".to_string(), serde_json::json!(8));

    match PyTorchBackend::validate_generate_text_envelope(&generate_envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("unsupported transformers_kwargs key"));
            assert!(message.contains("raw_max_batch_size"));
        }
        other => panic!("expected unsupported kwargs config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generate_text_stream_envelope_rejects_policy_transformers_kwargs() {
    let mut stream_envelope = PyTorchBackend::generate_text_envelope(
        "req-stream-policy-kwarg",
        PyTorchWorkerOperation::GenerateTextStream,
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );
    stream_envelope
        .payload
        .transformers_kwargs
        .insert("priority".to_string(), serde_json::json!("high"));

    match PyTorchBackend::validate_generate_text_stream_envelope(&stream_envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("unsupported transformers_kwargs key"));
            assert!(message.contains("priority"));
        }
        other => panic!("expected unsupported stream kwargs config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generate_text_request_omits_absent_top_k_kwarg() {
    let request = PyTorchBackend::generate_text_request(
        "Explain adapters.".to_string(),
        None,
        48,
        0.3,
        0.9,
        None,
        None,
    );

    assert!(request.transformers_kwargs.is_empty());
}

#[test]
fn test_pytorch_worker_generate_text_stream_envelope_rejects_wrong_operation() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/generate_text_stream_request.json"
    );
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker stream fixture");
    envelope.operation = PyTorchWorkerOperation::GenerateText;

    match PyTorchBackend::validate_generate_text_stream_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("GenerateText"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_envelope_rejects_wrong_operation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");
    envelope.operation = PyTorchWorkerOperation::LoadTransformersModel;

    match PyTorchBackend::validate_generate_text_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("LoadTransformersModel"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_generate_text_envelope_rejects_wrong_contract_version() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/generate_text_request.json");
    let mut envelope: PyTorchWorkerEnvelope<PyTorchGenerateTextRequest> =
        serde_json::from_str(fixture).expect("decode worker generate fixture");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_generate_text_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("generate_text envelope contract version"));
        }
        other => panic!("expected wrong-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_trust_policy_defaults_closed() {
    let default_policy = PyTorchBackend::default_transformers_trust_policy();
    assert!(!default_policy.allow_remote_code);
    assert!(default_policy.accepted_sources.is_empty());
    assert!(default_policy.decision_id.is_none());
    assert!(default_policy.local_files_only);
    assert_eq!(default_policy.auth_token_source, ModelAuthTokenSource::None);
    assert_eq!(
        default_policy.cache_policy,
        ModelLoadCachePolicy::BackendDefault
    );

    let request = PyTorchTransformersLoadRequest {
        model_ref: Some(PumasModelRef {
            model_id: "pumas://models/no-custom-code".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
        artifact_kind: ModelArtifactKind::HfCompatibleDirectory,
        entry_path: "/models/no-custom-code".to_string(),
        model_source: None,
        task_id: InferenceTaskId::TextGeneration,
        task_profile: None,
        model_type_hint: None,
        device: None,
        trust_policy: PyTorchTransformersTrustPolicy::default(),
        generation_defaults: None,
    };
    let envelope = PyTorchWorkerEnvelope::new(
        "req-default-trust",
        PyTorchWorkerOperation::LoadTransformersModel,
        request,
    );
    let encoded = serde_json::to_value(&envelope).expect("encode envelope");

    assert_eq!(
        encoded["payload"]["trust_policy"]["allow_remote_code"],
        false
    );
    assert!(encoded["payload"]["trust_policy"]
        .get("accepted_sources")
        .is_none());
    assert_eq!(
        encoded["payload"]["trust_policy"]["local_files_only"],
        serde_json::json!(true)
    );
}

#[test]
fn test_pytorch_worker_trust_policy_maps_public_load_security_policy() {
    let policy = ModelLoadSecurityPolicy {
        trust_remote_code: ModelRemoteCodePolicy::Allow,
        network: ModelLoadNetworkPolicy::AllowNetwork,
        cache: ModelLoadCachePolicy::BypassCache,
        auth_token_source: ModelAuthTokenSource::Environment,
        revision: Some("weights-rev".to_string()),
        code_revision: Some("code-rev".to_string()),
        decision_id: Some("trust-001".to_string()),
        accepted_code_sources: vec!["configuration_tiny.py".to_string()],
    };

    let trust_policy = PyTorchTransformersTrustPolicy::from(policy);

    assert!(trust_policy.allow_remote_code);
    assert!(!trust_policy.local_files_only);
    assert_eq!(trust_policy.cache_policy, ModelLoadCachePolicy::BypassCache);
    assert_eq!(
        trust_policy.auth_token_source,
        ModelAuthTokenSource::Environment
    );
    assert_eq!(trust_policy.revision.as_deref(), Some("weights-rev"));
    assert_eq!(trust_policy.code_revision.as_deref(), Some("code-rev"));
    assert_eq!(
        trust_policy.accepted_sources,
        vec!["configuration_tiny.py".to_string()]
    );
}

#[test]
fn test_pytorch_worker_error_response_preserves_request_correlation() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/worker_error_response.json");
    let response: PyTorchWorkerResponse<serde_json::Value> =
        serde_json::from_str(fixture).expect("decode worker error fixture");

    match response {
        PyTorchWorkerResponse::Error(PyTorchWorkerFailure { request_id, error }) => {
            assert_eq!(request_id, "req-load-001");
            assert_eq!(error.kind, PyTorchWorkerErrorKind::TrustPolicyRejected);
            assert_eq!(
                error.canonical_code.as_deref(),
                Some("pytorch_transformers_trust_policy_rejected")
            );
        }
        other => panic!("expected worker error response, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_failure_normalizes_to_backend_error() {
    let fixture =
        include_str!("../../tests/fixtures/pytorch_worker_contract/worker_error_response.json");
    let response: PyTorchWorkerResponse<serde_json::Value> =
        serde_json::from_str(fixture).expect("decode worker error fixture");
    let PyTorchWorkerResponse::Error(failure) = response else {
        panic!("expected worker error response");
    };

    match failure.into_backend_error() {
        BackendError::Config(message) => {
            assert!(message.contains("pytorch_transformers_trust_policy_rejected"));
            assert!(message.contains("req-load-001"));
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_load_response_decodes_loaded_model_info() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-load-ok",
        "result": {
            "model_path": "/models/tiny",
            "model_type": "text-generation",
            "device": "cpu"
        }
    });

    let info = PyTorchBackend::load_info_from_worker_response(&response.to_string())
        .expect("load response decodes");

    assert_eq!(info.model_path, "/models/tiny");
    assert_eq!(info.model_type, "text-generation");
    assert_eq!(info.device, "cpu");
}

#[test]
fn test_pytorch_worker_load_error_response_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-load-rejected",
        "error": {
            "kind": "trust_policy_rejected",
            "message": "Model package requires custom Transformers code but trust policy is closed.",
            "canonical_code": "pytorch_transformers_trust_policy_rejected"
        }
    });

    match PyTorchBackend::load_info_from_worker_response(&response.to_string()) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("pytorch_transformers_trust_policy_rejected"));
            assert!(message.contains("req-load-rejected"));
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected Config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_setup_error_response_normalizes_to_backend_error() {
    let response = serde_json::json!({
        "status": "error",
        "request_id": "req-stream-no-model",
        "error": {
            "kind": "runtime_unavailable",
            "message": "No model loaded. Call load_model() first.",
            "canonical_code": "pytorch_worker_generate_text_stream_failed"
        }
    });

    match PyTorchBackend::stream_setup_from_worker_response(&response.to_string()) {
        Err(BackendError::NotRunning(message)) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_generator_error_normalizes_to_backend_error() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-generator",
        "Generator error: Transformers stream failed.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-generator"));
            assert!(message.contains("Transformers stream failed"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_token_extraction_error_normalizes_to_backend_error() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-token",
        "Token extraction failed: expected string token.".to_string(),
    ) {
        BackendError::Inference(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-token"));
            assert!(message.contains("expected string token"));
        }
        other => panic!("expected Inference error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_runtime_unavailable_normalizes_to_not_running() {
    match PyTorchBackend::stream_worker_failure_from_message(
        "req-stream-no-model",
        "Generator error: No model loaded. Call load_model() first.".to_string(),
    ) {
        BackendError::NotRunning(message) => {
            assert!(message.contains("pytorch_worker_generate_text_stream_failed"));
            assert!(message.contains("req-stream-no-model"));
            assert!(message.contains("No model loaded"));
        }
        other => panic!("expected NotRunning error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_worker_stream_setup_success_response_decodes() {
    let response = serde_json::json!({
        "status": "ok",
        "request_id": "req-stream-ready",
        "result": {"ready": true}
    });

    PyTorchBackend::stream_setup_from_worker_response(&response.to_string())
        .expect("stream setup success should decode");
}

#[test]
fn test_pytorch_audio_transcription_requires_encoded_audio() {
    let request = AudioTranscriptionRequest {
        model: "openai/whisper-tiny".to_string(),
        audio: None,
        audio_ref: Some("artifact://audio.wav".to_string()),
        language: None,
        prompt: None,
        task: None,
        chunk_length_s: None,
        extra_options: serde_json::Value::Null,
    };

    match PyTorchBackend::audio_base64_from_request(&request) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("audio_ref resolution is owned by the host adapter"));
        }
        other => panic!("expected config error for unresolved audio_ref, got {other:?}"),
    }
}

#[test]
fn test_pytorch_audio_transcription_accepts_encoded_audio() {
    let request = AudioTranscriptionRequest {
        model: "openai/whisper-tiny".to_string(),
        audio: Some(EncodedAudio {
            data_base64: " UklGRg== ".to_string(),
            mime_type: "audio/wav".to_string(),
            sample_rate_hz: Some(16_000),
        }),
        audio_ref: None,
        language: Some("en".to_string()),
        prompt: None,
        task: Some("transcribe".to_string()),
        chunk_length_s: None,
        extra_options: serde_json::Value::Null,
    };

    let audio = PyTorchBackend::audio_base64_from_request(&request)
        .expect("encoded audio should be accepted");
    assert_eq!(audio, "UklGRg==");
}

#[test]
fn test_pytorch_worker_envelope_rejects_missing_required_fields() {
    let fixture = include_str!(
        "../../tests/fixtures/pytorch_worker_contract/load_transformers_model_request.json"
    );
    let mut invalid: serde_json::Value =
        serde_json::from_str(fixture).expect("decode worker load fixture");
    invalid
        .as_object_mut()
        .expect("fixture is object")
        .remove("request_id");
    let error =
        serde_json::from_value::<PyTorchWorkerEnvelope<PyTorchTransformersLoadRequest>>(invalid)
            .expect_err("missing request_id and payload fields should reject");

    assert!(error.to_string().contains("request_id"));
}

#[test]
fn test_pytorch_load_envelope_maps_pumas_package_facts() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let trust_policy = PyTorchTransformersTrustPolicy {
        allow_remote_code: true,
        accepted_sources: vec!["configuration_tiny.py".to_string()],
        decision_id: Some("trust-001".to_string()),
        local_files_only: true,
        cache_policy: ModelLoadCachePolicy::BackendDefault,
        auth_token_source: ModelAuthTokenSource::None,
        revision: Some("abc123".to_string()),
        code_revision: None,
    };

    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cuda:0"),
        trust_policy,
    )
    .expect("map package facts to worker envelope");

    assert_eq!(envelope.request_id, "req-pumas-load");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert_eq!(
        envelope
            .payload
            .model_ref
            .as_ref()
            .map(|value| value.model_id.as_str()),
        Some("llm/example/tiny-transformers")
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.entry_path, "llm/example/tiny-transformers");
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("model source should project from package facts");
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::PumasResolved
    );
    assert_eq!(model_source.entry_path, envelope.payload.entry_path);
    assert_eq!(
        model_source.model_ref.as_ref(),
        envelope.payload.model_ref.as_ref()
    );
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("task profile should project from registry entry");
    assert_eq!(task_profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(task_profile.canonical_task_label, "text_generation");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
    assert!(task_profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
    assert_eq!(envelope.payload.model_type_hint.as_deref(), Some("llama"));
    assert_eq!(envelope.payload.device.as_deref(), Some("cuda:0"));
    assert!(envelope.payload.trust_policy.allow_remote_code);
    assert_eq!(
        envelope.payload.trust_policy.decision_id.as_deref(),
        Some("trust-001")
    );
    assert!(envelope.payload.trust_policy.local_files_only);
    assert_eq!(
        envelope.payload.trust_policy.revision.as_deref(),
        Some("abc123")
    );
    assert_eq!(
        envelope
            .payload
            .generation_defaults
            .as_ref()
            .and_then(|defaults| {
                defaults
                    .get("max_new_tokens")
                    .and_then(serde_json::Value::as_u64)
            }),
        Some(128)
    );
}

#[test]
fn test_pytorch_transformers_load_args_use_worker_envelope_payload() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cuda:0"),
        PyTorchTransformersTrustPolicy::from(ModelLoadSecurityPolicy {
            trust_remote_code: ModelRemoteCodePolicy::Allow,
            network: ModelLoadNetworkPolicy::AllowNetwork,
            cache: ModelLoadCachePolicy::BypassCache,
            auth_token_source: ModelAuthTokenSource::Environment,
            revision: Some("weights-rev".to_string()),
            code_revision: Some("code-rev".to_string()),
            decision_id: Some("trust-001".to_string()),
            accepted_code_sources: vec!["configuration_tiny.py".to_string()],
        }),
    )
    .expect("map package facts to worker envelope");

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("envelope should validate");
    let args = PyTorchBackend::transformers_load_args_from_request(&envelope.payload);

    assert_eq!(args.model_path, "llm/example/tiny-transformers");
    assert_eq!(args.device, "cuda:0");
    assert_eq!(args.model_type.as_deref(), Some("llama"));
    assert!(args.trust_policy.allow_remote_code);
    assert!(!args.trust_policy.local_files_only);
    assert_eq!(
        args.trust_policy.cache_policy,
        ModelLoadCachePolicy::BypassCache
    );
    assert_eq!(
        args.trust_policy.auth_token_source,
        ModelAuthTokenSource::Environment
    );
    assert_eq!(args.trust_policy.revision.as_deref(), Some("weights-rev"));
    assert_eq!(args.trust_policy.code_revision.as_deref(), Some("code-rev"));
}

#[test]
fn test_pytorch_direct_load_envelope_uses_transformers_contract() {
    let envelope = PyTorchBackend::transformers_load_envelope_from_direct_path(
        "req-direct-load",
        "/models/direct-hf",
        Some("cpu"),
        Some("dllm"),
        PyTorchTransformersTrustPolicy::default(),
    );

    PyTorchBackend::validate_transformers_load_envelope(&envelope)
        .expect("direct load envelope should validate");
    assert_eq!(envelope.request_id, "req-direct-load");
    assert_eq!(
        envelope.operation,
        PyTorchWorkerOperation::LoadTransformersModel
    );
    assert!(envelope.payload.model_ref.is_none());
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.entry_path, "/models/direct-hf");
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(envelope.payload.model_type_hint.as_deref(), Some("dllm"));
    assert_eq!(envelope.payload.device.as_deref(), Some("cpu"));
    let model_source = envelope
        .payload
        .model_source
        .as_ref()
        .expect("direct load envelope should carry a model source");
    assert_eq!(
        model_source.source_kind,
        ResolvedModelSourceKind::DirectHfCompatibleDirectory
    );
    assert!(model_source.model_ref.is_none());
    assert!(model_source.validate_for_backend_load().is_ok());
    let task_profile = envelope
        .payload
        .task_profile
        .as_ref()
        .expect("direct load envelope should carry a task profile");
    assert_eq!(
        task_profile.loader,
        PyTorchTransformersModelLoader::CausalLm
    );
}

#[test]
fn test_pytorch_transformers_load_args_default_device_auto() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        None,
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");

    let args = PyTorchBackend::transformers_load_args_from_request(&envelope.payload);

    assert_eq!(args.device, "auto");
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_contract_version() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope.contract_version = PYTORCH_WORKER_CONTRACT_VERSION + 1;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unsupported PyTorch worker load envelope"));
        }
        other => panic!("expected contract-version config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_wrong_operation() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope.operation = PyTorchWorkerOperation::InitWorker;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Unexpected PyTorch worker operation"));
            assert!(message.contains("InitWorker"));
        }
        other => panic!("expected wrong-operation config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_transformers_load_envelope_validation_rejects_invalid_model_source() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");
    let mut envelope = PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy {
            allow_remote_code: true,
            accepted_sources: vec!["configuration_tiny.py".to_string()],
            decision_id: None,
            local_files_only: true,
            cache_policy: ModelLoadCachePolicy::BackendDefault,
            auth_token_source: ModelAuthTokenSource::None,
            revision: None,
            code_revision: None,
        },
    )
    .expect("map package facts to worker envelope");
    envelope
        .payload
        .model_source
        .as_mut()
        .expect("model source")
        .model_ref = None;

    match PyTorchBackend::validate_transformers_load_envelope(&envelope) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("Invalid PyTorch worker resolved model source"));
            assert!(message.contains("pumas_resolved_source_missing_model_ref"));
        }
        other => panic!("expected invalid model source config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_task_profile_uses_canonical_registry_aliases() {
    let profile = PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        pipeline_tag: Some("causal-lm".to_string()),
        ..TaskEvidence::default()
    })
    .expect("causal-lm alias should resolve through task registry");

    assert_eq!(profile.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(profile.canonical_task_label, "text_generation");
    assert_eq!(profile.loader, PyTorchTransformersModelLoader::CausalLm);
    assert!(profile
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));
}

#[test]
fn test_pytorch_task_profile_maps_audio_transcription_aliases() {
    let profile = PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        pipeline_tag: Some("automatic-speech-recognition".to_string()),
        task_type_primary: Some("audio_transcription".to_string()),
        input_modalities: vec!["audio".to_string()],
        output_modalities: vec!["text".to_string()],
    })
    .expect("audio transcription aliases should resolve through task registry");

    assert_eq!(profile.task_id, InferenceTaskId::AudioTranscription);
    assert_eq!(profile.canonical_task_label, "audio_transcription");
    assert_eq!(
        profile.loader,
        PyTorchTransformersModelLoader::AutomaticSpeechRecognition
    );
    assert!(profile
        .required_components
        .contains(&ProcessorComponentKind::AudioFeatureExtractor));
}

#[test]
fn test_pytorch_task_profile_rejects_registry_tasks_without_loader() {
    match PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        task_type_primary: Some("feature-extraction".to_string()),
        ..TaskEvidence::default()
    }) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("does not support canonical task embedding yet"));
        }
        other => panic!("expected unsupported task config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_task_profile_rejects_unregistered_task_evidence() {
    match PyTorchBackend::transformers_task_profile_from_evidence(&TaskEvidence {
        task_type_primary: Some("object-detection".to_string()),
        pipeline_tag: Some("object-detection".to_string()),
        input_modalities: vec!["image".to_string()],
        output_modalities: vec!["json".to_string()],
    }) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("UnsupportedTaskLabel"));
            assert!(message.contains("task evidence did not resolve"));
        }
        other => panic!("expected unsupported task evidence config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_load_envelope_rejects_custom_code_without_trust_opt_in() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");

    match PyTorchBackend::transformers_load_envelope_from_package(
        "req-pumas-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy::default(),
    ) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("trust policy is closed"));
        }
        other => panic!("expected closed trust policy config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_load_envelope_rejects_unsupported_artifact_kind() {
    let fixture = include_str!(
        "../../tests/fixtures/inference_package_facts/gguf_text_generation_package_facts.json"
    );
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(fixture).expect("decode package facts fixture");

    match PyTorchBackend::transformers_load_envelope_from_package(
        "req-gguf-load",
        &facts,
        Some("cpu"),
        PyTorchTransformersTrustPolicy::default(),
    ) {
        Err(BackendError::Config(message)) => {
            assert!(message.contains("cannot load Gguf artifacts"));
        }
        other => panic!("expected unsupported artifact config error, got {other:?}"),
    }
}

#[test]
fn test_pytorch_generation_options_map_to_transformers_kwargs_and_diagnostics() {
    let options = GenerationOptions {
        length: LengthGenerationOptions {
            max_new_tokens: Some(128),
            ..Default::default()
        },
        sampling: SamplingGenerationOptions {
            temperature: Some(0.6),
            top_p: Some(0.9),
            top_k: Some(40),
            seed: Some(42),
            ..Default::default()
        },
        stopping: StoppingGenerationOptions {
            stop_strings: vec!["END".to_string()],
            eos_token_ids: vec![2, 32000],
        },
        cache: CacheGenerationOptions {
            use_cache: Some(true),
            kv_cache_checkpoint_requested: Some(true),
        },
        output: OutputGenerationOptions {
            return_logprobs: Some(true),
            ..Default::default()
        },
        special_tokens: SpecialTokenGenerationOptions {
            pad_token_id: Some(0),
            ..Default::default()
        },
        backend_extensions: [
            (
                "transformers:renormalize_logits".to_string(),
                serde_json::json!(true),
            ),
            ("raw_top_k".to_string(), serde_json::json!(40)),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    let mapping = PyTorchBackend::transformers_generation_option_mapping(&options);

    assert_eq!(mapping.kwargs["max_new_tokens"], serde_json::json!(128));
    let temperature = mapping.kwargs["temperature"]
        .as_f64()
        .expect("temperature is numeric");
    assert!((temperature - 0.6).abs() < 0.000_001);
    assert_eq!(mapping.kwargs["use_cache"], serde_json::json!(true));
    assert_eq!(mapping.kwargs["pad_token_id"], serde_json::json!(0));
    assert_eq!(
        mapping.kwargs["eos_token_id"],
        serde_json::json!([2, 32000])
    );
    assert_eq!(
        mapping.kwargs["renormalize_logits"],
        serde_json::json!(true)
    );
    assert_eq!(mapping.kwargs["top_k"], serde_json::json!(40));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "sampling.top_k"
            && diagnostic.state == OptionSupportState::Honored
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "sampling.seed"
            && diagnostic.state == OptionSupportState::Unsupported
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "stopping.eos_token_ids"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "cache.kv_cache_checkpoint_requested"
            && diagnostic.state == OptionSupportState::Mapped
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "output.return_logprobs"
            && diagnostic.state == OptionSupportState::Unsupported
    }));
    assert!(mapping.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "backend_extensions.raw_top_k"
            && diagnostic.state == OptionSupportState::Rejected
    }));

    let requested_paths: BTreeSet<_> = options.requested_option_paths().into_iter().collect();
    let diagnostic_paths: BTreeSet<_> = mapping
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.option_path.clone())
        .collect();
    assert!(
        requested_paths.is_subset(&diagnostic_paths),
        "missing diagnostics for requested options: {:?}",
        requested_paths
            .difference(&diagnostic_paths)
            .collect::<Vec<_>>()
    );
}
