use std::collections::BTreeSet;

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
        model_ref: PumasModelRef {
            model_id: "pumas://models/no-custom-code".to_string(),
            revision: None,
            selected_artifact_id: None,
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        },
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
        envelope.payload.model_ref.model_id,
        "llm/example/tiny-transformers"
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
        Some(&envelope.payload.model_ref)
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
        backend_extensions: [(
            "transformers:renormalize_logits".to_string(),
            serde_json::json!(true),
        )]
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
