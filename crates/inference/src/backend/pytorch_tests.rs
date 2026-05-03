use super::pytorch_worker_contract::{
    PyTorchTransformersLoadRequest, PyTorchTransformersTrustPolicy, PyTorchWorkerEnvelope,
    PyTorchWorkerErrorKind, PyTorchWorkerFailure, PyTorchWorkerOperation, PyTorchWorkerResponse,
    PYTORCH_WORKER_CONTRACT_VERSION,
};
use super::*;
use crate::model_contracts::{ModelArtifactKind, PumasModelRef, ResolvedModelPackageFacts};

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
        envelope.payload.model_ref.model_id,
        "pumas://models/tiny-causal"
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(envelope.payload.device.as_deref(), Some("cuda:0"));
    assert!(!envelope.payload.trust_policy.allow_remote_code);
    assert!(envelope.payload.trust_policy.accepted_sources.is_empty());
}

#[test]
fn test_pytorch_worker_trust_policy_defaults_closed() {
    let default_policy = PyTorchBackend::default_transformers_trust_policy();
    assert!(!default_policy.allow_remote_code);
    assert!(default_policy.accepted_sources.is_empty());
    assert!(default_policy.decision_id.is_none());

    let request = PyTorchTransformersLoadRequest {
        model_ref: PumasModelRef {
            model_id: "pumas://models/no-custom-code".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
        artifact_kind: ModelArtifactKind::HfCompatibleDirectory,
        entry_path: "/models/no-custom-code".to_string(),
        task_id: InferenceTaskId::TextGeneration,
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
        envelope.payload.model_ref.model_id,
        "llm/example/tiny-transformers"
    );
    assert_eq!(
        envelope.payload.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(envelope.payload.entry_path, "llm/example/tiny-transformers");
    assert_eq!(envelope.payload.task_id, InferenceTaskId::TextGeneration);
    assert_eq!(envelope.payload.model_type_hint.as_deref(), Some("llama"));
    assert_eq!(envelope.payload.device.as_deref(), Some("cuda:0"));
    assert!(envelope.payload.trust_policy.allow_remote_code);
    assert_eq!(
        envelope.payload.trust_policy.decision_id.as_deref(),
        Some("trust-001")
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
