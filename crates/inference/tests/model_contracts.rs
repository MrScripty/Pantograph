use std::collections::{BTreeMap, BTreeSet};

use inference::{
    default_task_registry_entries, normalize_modality_label, normalize_task_label,
    resolve_task_registry_entry, resolve_task_registry_entry_from_evidence, BackendCapabilityFacts,
    BackendCompatibilityIssue, BackendCompatibilityIssueKind, BackendCompatibilityReport,
    BackendCompatibilityStatus, BackendHintLabel, BackendTaskCapability, DiffusersComponentRole,
    GenerationOptionSource, GenerationOptions, ImageGenerationFamilyLabel,
    InferenceEmbeddingResult, InferenceExecutionInput, InferenceExecutionInputKind,
    InferenceExecutionRequest, InferenceExecutionResult, InferenceExecutionResultKind,
    InferenceLifecyclePhase, InferenceModality, InferenceRequestLifecycleEvent,
    InferenceRequestLifecycleEventKind, InferenceTaskId, InferenceUsage, ModelArtifactKind,
    ModelExecutionDescriptor, ModelExecutionStorageKind, ModelExecutionValidationState,
    ModelFactFamily, ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibraryUpdateEvent,
    ModelLibraryUpdateFeed, ModelLoadCachePolicy, ModelLoadNetworkPolicy, ModelLoadSecurityPolicy,
    ModelPackageDiagnostic, ModelPackageFactsSummarySnapshot, ModelPackageFactsSummarySnapshotItem,
    ModelPackageFactsSummaryStatus, ModelRemoteCodePolicy, ModelStorageKind, ModelValidationState,
    OptionCompatibilityDiagnostic, OptionSupportState, PackageFactStatus, PackageFactValueSource,
    PackageSizeRole, ProcessorComponentKind, PumasArtifactLoadPathKind, PumasArtifactLoadTarget,
    PumasModelRef, ResolvedModelPackageFacts, ResolvedModelSource, ResolvedModelSourceKind,
    RuntimeLifecycleSnapshot, SupportTier, TaskEvidence, TaskExecutionBehavior, TaskFamily,
    TaskRegistryEntry, TaskRegistryResolutionDiagnosticKind, TaskRequestContract,
    TaskStreamingSupport, MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
};

const PACKAGE_FACT_FIXTURES: &[(&str, &str)] = &[
    (
        "gguf_text_generation_package_facts.json",
        include_str!("fixtures/inference_package_facts/gguf_text_generation_package_facts.json"),
    ),
    (
        "gguf_embedding_package_facts.json",
        include_str!("fixtures/inference_package_facts/gguf_embedding_package_facts.json"),
    ),
    (
        "rerank_package_facts.json",
        include_str!("fixtures/inference_package_facts/rerank_package_facts.json"),
    ),
    (
        "hf_transformers_text_generation_package_facts.json",
        include_str!(
            "fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
        ),
    ),
    (
        "hf_candle_embedding_package_facts.json",
        include_str!("fixtures/inference_package_facts/hf_candle_embedding_package_facts.json"),
    ),
    (
        "hf_multimodal_processor_package_facts.json",
        include_str!("fixtures/inference_package_facts/hf_multimodal_processor_package_facts.json"),
    ),
    (
        "hf_audio_transcription_package_facts.json",
        include_str!("fixtures/inference_package_facts/hf_audio_transcription_package_facts.json"),
    ),
    (
        "safetensors_package_facts.json",
        include_str!("fixtures/inference_package_facts/safetensors_package_facts.json"),
    ),
    (
        "diffusers_bundle_package_facts.json",
        include_str!("fixtures/inference_package_facts/diffusers_bundle_package_facts.json"),
    ),
    (
        "onnx_package_facts.json",
        include_str!("fixtures/inference_package_facts/onnx_package_facts.json"),
    ),
    (
        "custom_code_required_package_facts.json",
        include_str!("fixtures/inference_package_facts/custom_code_required_package_facts.json"),
    ),
    (
        "unsupported_ollama_hint_package_facts.json",
        include_str!("fixtures/inference_package_facts/unsupported_ollama_hint_package_facts.json"),
    ),
    (
        "invalid_generation_config_package_facts.json",
        include_str!(
            "fixtures/inference_package_facts/invalid_generation_config_package_facts.json"
        ),
    ),
    (
        "missing_tokenizer_package_facts.json",
        include_str!("fixtures/inference_package_facts/missing_tokenizer_package_facts.json"),
    ),
];

const PUMAS_IMAGE_PACKAGE_FACT_FIXTURES: &[(&str, &str, &str)] = &[(
    "diffusers_sd_text_to_image_package_facts.json",
    "f87c3da8",
    include_str!("fixtures/inference_package_facts/diffusers_sd_text_to_image_package_facts.json"),
)];

fn package_fact_fixture(name: &str) -> &'static str {
    PACKAGE_FACT_FIXTURES
        .iter()
        .find_map(|(fixture_name, raw)| (*fixture_name == name).then_some(*raw))
        .unwrap_or_else(|| panic!("missing package fact fixture {name}"))
}

#[test]
fn package_fact_fixtures_decode_through_public_contracts() {
    for (fixture_name, raw) in PACKAGE_FACT_FIXTURES {
        let facts: ResolvedModelPackageFacts = serde_json::from_str(raw).unwrap_or_else(|error| {
            panic!("fixture {fixture_name} should decode: {error}");
        });

        assert!(
            facts.uses_current_contract(),
            "fixture {fixture_name} should use current contract version"
        );

        let encoded = serde_json::to_string(&facts).expect("encode facts");
        let decoded: ResolvedModelPackageFacts =
            serde_json::from_str(&encoded).expect("decode encoded facts");
        assert_eq!(
            decoded.package_facts_contract_version,
            MODEL_PACKAGE_FACTS_CONTRACT_VERSION
        );
        assert_eq!(decoded.model_ref.model_id, facts.model_ref.model_id);
    }
}

#[test]
fn pumas_image_generation_fixture_decodes_with_structured_diffusers_facts() {
    for (fixture_name, source_commit, raw) in PUMAS_IMAGE_PACKAGE_FACT_FIXTURES {
        let raw_value: serde_json::Value =
            serde_json::from_str(raw).expect("Pumas fixture should be valid JSON");
        let facts: ResolvedModelPackageFacts = serde_json::from_str(raw).unwrap_or_else(|error| {
            panic!("Pumas fixture {fixture_name} from {source_commit} should decode: {error}");
        });
        let diffusers = facts.diffusers.as_ref().unwrap_or_else(|| {
            panic!("Pumas fixture {fixture_name} should carry structured diffusers facts")
        });
        let source = ResolvedModelSource::from_package_facts(&facts);
        let entry =
            resolve_task_registry_entry_from_evidence(&facts.task).unwrap_or_else(|error| {
                panic!("Pumas fixture {fixture_name} should resolve: {error:?}")
            });

        assert_eq!(
            facts.package_facts_contract_version,
            MODEL_PACKAGE_FACTS_CONTRACT_VERSION
        );
        assert!(facts.uses_current_contract());
        assert_eq!(
            facts.artifact.artifact_kind,
            ModelArtifactKind::DiffusersBundle
        );
        let logical_size = facts.artifact.logical_size.as_ref().unwrap_or_else(|| {
            panic!("Pumas fixture {fixture_name} should carry logical size facts")
        });
        assert_eq!(logical_size.total_size_bytes, Some(7952));
        assert_eq!(
            logical_size.value_source,
            PackageFactValueSource::ComponentLayout
        );
        assert!(logical_size.files.iter().any(|file| {
            file.relative_path == "unet/diffusion_pytorch_model.safetensors"
                && file.size_bytes == Some(4096)
                && file.value_source == PackageFactValueSource::FilesystemMetadata
                && file.role == Some(PackageSizeRole::Weight)
        }));
        assert_eq!(
            facts.task.task_type_primary.as_deref(),
            Some("image_generation")
        );
        assert_eq!(facts.task.pipeline_tag.as_deref(), Some("text-to-image"));
        assert_eq!(entry.task_id, InferenceTaskId::ImageGeneration);
        assert_eq!(
            diffusers.pipeline_class.as_deref(),
            Some("StableDiffusionPipeline")
        );
        assert!(diffusers.family_evidence.iter().any(|evidence| {
            evidence.family == ImageGenerationFamilyLabel::StableDiffusion
                && evidence.source_path.as_deref() == Some("model_index.json")
        }));
        assert!(diffusers.components.iter().any(|component| {
            component.role == DiffusersComponentRole::Scheduler
                && component.config_path.as_deref() == Some("scheduler/scheduler_config.json")
        }));
        assert!(facts
            .backend_hints
            .accepted
            .contains(&BackendHintLabel::Diffusers));
        assert!(
            source.validate_for_backend_load().is_ok(),
            "Pumas fixture {fixture_name} should project into a backend-loadable source"
        );
        for forbidden in [
            "pantograph",
            "workflow",
            "runtime_registry",
            "diagnostics_ledger",
            "scheduler_policy",
        ] {
            assert!(
                raw_value.get(forbidden).is_none(),
                "Pumas fixture {fixture_name} should not expose consumer field {forbidden}"
            );
        }
    }
}

#[test]
fn pumas_artifact_load_target_decodes_existing_pumas_wire_shape() {
    let target: PumasArtifactLoadTarget = serde_json::from_value(serde_json::json!({
        "model_ref": {
            "model_ref_contract_version": 1,
            "model_id": "image/stable-diffusion/tiny-sd",
            "selected_artifact_path": "image/stable-diffusion/tiny-sd"
        },
        "artifact_kind": "diffusers_bundle",
        "local_load_path": "/pumas/models/image/stable-diffusion/tiny-sd",
        "load_path_kind": "directory",
        "library_root_id": "library-root",
        "storage_kind": "library_owned",
        "validation_state": "valid",
        "package_facts_contract_version": MODEL_PACKAGE_FACTS_CONTRACT_VERSION
    }))
    .expect("Pumas load-target response target should decode");

    assert_eq!(target.model_ref.model_id, "image/stable-diffusion/tiny-sd");
    assert_eq!(target.artifact_kind, ModelArtifactKind::DiffusersBundle);
    assert_eq!(target.load_path_kind, PumasArtifactLoadPathKind::Directory);
    assert_eq!(target.storage_kind, ModelStorageKind::LibraryOwned);
    assert_eq!(target.validation_state, ModelValidationState::Valid);
    assert_eq!(
        target.local_load_path,
        "/pumas/models/image/stable-diffusion/tiny-sd"
    );
}

#[test]
fn package_fact_fixtures_cover_safetensors_diffusers_and_onnx_artifact_kinds() {
    for (fixture_name, expected_kind, expected_backend_hint) in [
        (
            "safetensors_package_facts.json",
            ModelArtifactKind::Safetensors,
            BackendHintLabel::Candle,
        ),
        (
            "diffusers_bundle_package_facts.json",
            ModelArtifactKind::DiffusersBundle,
            BackendHintLabel::Diffusers,
        ),
        (
            "onnx_package_facts.json",
            ModelArtifactKind::Onnx,
            BackendHintLabel::OnnxRuntime,
        ),
    ] {
        let raw = PACKAGE_FACT_FIXTURES
            .iter()
            .find_map(|(name, raw)| (*name == fixture_name).then_some(*raw))
            .unwrap_or_else(|| panic!("fixture {fixture_name} should be registered"));
        let facts: ResolvedModelPackageFacts =
            serde_json::from_str(raw).expect("decode package facts fixture");
        let source = ResolvedModelSource::from_package_facts(&facts);

        assert!(facts.uses_current_contract());
        assert_eq!(
            facts.package_facts_contract_version,
            MODEL_PACKAGE_FACTS_CONTRACT_VERSION
        );
        assert_eq!(facts.artifact.artifact_kind, expected_kind);
        assert!(
            !facts.artifact.entry_path.trim().is_empty(),
            "fixture {fixture_name} should expose a backend-loadable entry path"
        );
        assert!(
            facts
                .backend_hints
                .accepted
                .contains(&expected_backend_hint),
            "fixture {fixture_name} should preserve its Pumas backend hint"
        );
        assert!(
            source.validate_for_backend_load().is_ok(),
            "fixture {fixture_name} should project into a backend-loadable Pumas source"
        );
    }
}

#[test]
fn hf_candle_embedding_fixture_projects_to_backend_load_source() {
    let raw = PACKAGE_FACT_FIXTURES
        .iter()
        .find_map(|(name, raw)| (*name == "hf_candle_embedding_package_facts.json").then_some(*raw))
        .expect("HF Candle embedding fixture should be registered");
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(raw).expect("decode package facts fixture");
    let source = ResolvedModelSource::from_package_facts(&facts);

    assert!(facts.uses_current_contract());
    assert_eq!(
        facts.package_facts_contract_version,
        MODEL_PACKAGE_FACTS_CONTRACT_VERSION
    );
    assert_eq!(
        facts.artifact.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(facts.task.task_type_primary.as_deref(), Some("embedding"));
    assert!(facts
        .backend_hints
        .accepted
        .contains(&BackendHintLabel::Candle));
    assert_eq!(
        source.artifact_kind,
        ModelArtifactKind::HfCompatibleDirectory
    );
    assert_eq!(source.model_ref.as_ref(), Some(&facts.model_ref));
    assert!(
        source.validate_for_backend_load().is_ok(),
        "HF Candle embedding fixture should project into backend-loadable package facts"
    );
}

#[test]
fn package_fact_fixtures_resolve_task_evidence_through_registry() {
    for (fixture_name, expected_task_id) in [
        (
            "gguf_text_generation_package_facts.json",
            InferenceTaskId::TextGeneration,
        ),
        (
            "gguf_embedding_package_facts.json",
            InferenceTaskId::Embedding,
        ),
        ("rerank_package_facts.json", InferenceTaskId::Rerank),
        (
            "hf_transformers_text_generation_package_facts.json",
            InferenceTaskId::TextGeneration,
        ),
        (
            "hf_candle_embedding_package_facts.json",
            InferenceTaskId::Embedding,
        ),
        (
            "hf_multimodal_processor_package_facts.json",
            InferenceTaskId::ImageUnderstanding,
        ),
        (
            "hf_audio_transcription_package_facts.json",
            InferenceTaskId::AudioTranscription,
        ),
        ("safetensors_package_facts.json", InferenceTaskId::Embedding),
        (
            "diffusers_bundle_package_facts.json",
            InferenceTaskId::ImageGeneration,
        ),
        ("onnx_package_facts.json", InferenceTaskId::Embedding),
    ] {
        let raw = PACKAGE_FACT_FIXTURES
            .iter()
            .find_map(|(name, raw)| (*name == fixture_name).then_some(*raw))
            .unwrap_or_else(|| panic!("fixture {fixture_name} should be registered"));
        let facts: ResolvedModelPackageFacts =
            serde_json::from_str(raw).expect("decode package facts fixture");
        let entry = resolve_task_registry_entry_from_evidence(&facts.task)
            .unwrap_or_else(|error| panic!("fixture {fixture_name} should resolve: {error:?}"));

        assert_eq!(
            entry.task_id, expected_task_id,
            "fixture {fixture_name} should resolve to the expected canonical task"
        );
    }
}

#[test]
fn inference_execution_request_wire_contract_preserves_tags_defaults_and_unknown_fields() {
    let raw = serde_json::json!({
        "request_id": "req-rerank-1",
        "task_id": "rerank",
        "model_name": "reranker-model",
        "input": {
            "input_type": "rerank",
            "query": "search term",
            "documents": ["alpha", "beta"],
            "top_n": 1,
            "return_documents": true,
            "future_input_field": "ignored"
        },
        "future_request_field": {"ignored": true}
    });

    let request: InferenceExecutionRequest =
        serde_json::from_value(raw).expect("canonical request should decode");

    assert_eq!(request.request_id.as_deref(), Some("req-rerank-1"));
    assert_eq!(request.task_id, InferenceTaskId::Rerank);
    assert_eq!(request.model_name.as_deref(), Some("reranker-model"));
    assert!(request.model_ref.is_none());
    assert!(request.resolved_model_package_facts.is_none());
    assert!(request.generation_options.is_none());
    assert!(request.extra_options.is_null());
    match &request.input {
        InferenceExecutionInput::Rerank {
            query,
            documents,
            top_n,
            return_documents,
        } => {
            assert_eq!(query, "search term");
            assert_eq!(documents, &vec!["alpha".to_string(), "beta".to_string()]);
            assert_eq!(*top_n, Some(1));
            assert!(*return_documents);
        }
        other => panic!("unexpected input variant: {other:?}"),
    }

    let encoded = serde_json::to_value(&request).expect("request should encode");
    assert_eq!(encoded["task_id"], serde_json::json!("rerank"));
    assert_eq!(encoded["input"]["input_type"], serde_json::json!("rerank"));
    assert!(
        encoded.get("model_ref").is_none(),
        "none-valued optional fields should stay absent on the wire"
    );
    assert!(
        encoded.get("extra_options").is_none(),
        "null extra options should stay absent on the wire"
    );
}

#[test]
fn compact_model_execution_descriptor_stays_smaller_than_package_facts() {
    let raw = serde_json::json!({
        "execution_contract_version": 1,
        "model_id": "pumas://models/llama-3.1-8b-q4",
        "entry_path": "/models/llama-3.1-8b-q4/model.gguf",
        "model_type": "llm",
        "task_type_primary": "text-generation",
        "recommended_backend": "llama_cpp",
        "runtime_engine_hints": ["llama_cpp"],
        "storage_kind": "library_owned",
        "validation_state": "valid",
        "dependency_resolution": null
    });

    let descriptor: ModelExecutionDescriptor =
        serde_json::from_value(raw).expect("decode descriptor");

    assert_eq!(descriptor.execution_contract_version, 1);
    assert_eq!(
        descriptor.storage_kind,
        ModelExecutionStorageKind::LibraryOwned
    );
    assert_eq!(
        descriptor.validation_state,
        ModelExecutionValidationState::Valid
    );
    assert_eq!(descriptor.recommended_backend.as_deref(), Some("llama_cpp"));
}

#[test]
fn remote_search_hints_are_not_executable_guarantees() {
    let remote_search: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/inference_package_facts/remote_search_mlx_vllm_hint.json"
    ))
    .expect("decode remote search fixture");

    assert!(remote_search
        .get("package_facts_contract_version")
        .is_none());
    let compatible_engines = remote_search["compatibleEngines"]
        .as_array()
        .expect("compatibleEngines should be an array");
    assert!(compatible_engines
        .iter()
        .any(|engine| engine.as_str() == Some("mlx")));
    assert!(compatible_engines
        .iter()
        .any(|engine| engine.as_str() == Some("vllm")));
}

#[test]
fn public_inference_contract_json_keys_avoid_scheduler_policy_language() {
    let lifecycle = RuntimeLifecycleSnapshot {
        runtime_id: Some("runtime.llama_cpp".to_string()),
        runtime_instance_id: Some("runtime.llama_cpp.1".to_string()),
        runtime_reused: Some(true),
        active: true,
        ..RuntimeLifecycleSnapshot::default()
    };
    let event = InferenceRequestLifecycleEvent::builder(
        InferenceLifecyclePhase::BackendExecution,
        InferenceRequestLifecycleEventKind::Completed,
        42,
    )
    .with_request_id(Some("req-1".to_string()))
    .with_task_id(Some("text_generation".to_string()))
    .with_backend_key(Some("llama_cpp".to_string()))
    .with_runtime_id(Some("runtime.llama_cpp".to_string()))
    .with_runtime_instance_id(Some("runtime.llama_cpp.1".to_string()))
    .with_model_id(Some("pumas://models/tiny".to_string()))
    .with_resolved_artifact_kind(Some("gguf".to_string()))
    .with_usage(Some(InferenceUsage {
        prompt_tokens: Some(2),
        completion_tokens: Some(3),
        total_tokens: Some(5),
    }))
    .with_cache_handle_id(Some("kv-1".to_string()))
    .with_artifact_refs(vec!["artifact://audio.wav".to_string()])
    .build();
    let capability_facts = BackendCapabilityFacts::from_tasks(vec![BackendTaskCapability::stable(
        InferenceTaskId::TextGeneration,
        vec![InferenceModality::Text],
        vec![InferenceModality::Text],
    )]);
    let source = ResolvedModelSource {
        source_contract_version: 1,
        source_kind: ResolvedModelSourceKind::PumasResolved,
        artifact_kind: ModelArtifactKind::Gguf,
        entry_path: "models/tiny.gguf".to_string(),
        storage_kind: ModelStorageKind::LibraryOwned,
        validation_state: ModelValidationState::Valid,
        model_ref: Some(PumasModelRef {
            model_id: "pumas://models/tiny".to_string(),
            revision: None,
            selected_artifact_id: Some("gguf".to_string()),
            selected_artifact_path: None,
            migration_diagnostics: Vec::new(),
        }),
        repo_id: None,
        revision: None,
        selected_files: Vec::new(),
        companion_artifacts: Vec::new(),
        diagnostics: Vec::new(),
    };
    let typed_request = InferenceExecutionRequest {
        request_id: Some("req-typed".to_string()),
        task_id: InferenceTaskId::TextGeneration,
        model_ref: None,
        model_name: Some("tiny".to_string()),
        resolved_model_package_facts: None,
        input: InferenceExecutionInput::TextGeneration {
            prompt: Some("hello".to_string()),
            system_prompt: None,
            messages: Vec::new(),
            stream: false,
        },
        generation_options: Some(GenerationOptions::default()),
        extra_options: serde_json::Value::Null,
    };
    let typed_text_result = InferenceExecutionResult::TextGeneration {
        text: "hello".to_string(),
        usage: Some(InferenceUsage {
            prompt_tokens: Some(1),
            completion_tokens: Some(1),
            total_tokens: Some(2),
        }),
        cache_handle_id: Some("kv-typed".to_string()),
        option_diagnostics: Vec::new(),
    };
    let typed_embedding_result = InferenceExecutionResult::Embedding {
        embeddings: vec![InferenceEmbeddingResult {
            vector: vec![0.25, 0.75],
            token_count: Some(2),
            index: Some(0),
        }],
        usage: Some(InferenceUsage {
            prompt_tokens: Some(2),
            completion_tokens: None,
            total_tokens: Some(2),
        }),
        option_diagnostics: Vec::new(),
    };
    let compatibility_report = BackendCompatibilityReport {
        compatible: false,
        task: BackendCompatibilityStatus::Supported,
        model_source: BackendCompatibilityStatus::Supported,
        preprocessing: BackendCompatibilityStatus::Unsupported,
        postprocessing: BackendCompatibilityStatus::Supported,
        option_diagnostics: vec![OptionCompatibilityDiagnostic {
            option_path: "cache.use_cache".to_string(),
            state: OptionSupportState::RequiresBackendSupport,
            backend_key: Some("llama_cpp".to_string()),
            message: Some("cache reuse requires backend/runtime support".to_string()),
        }],
        issues: vec![BackendCompatibilityIssue {
            kind: BackendCompatibilityIssueKind::MissingPreprocessingComponent,
            phase: InferenceLifecyclePhase::Preprocessing,
            message: "tokenizer component is missing".to_string(),
            model_id: Some("pumas://models/tiny".to_string()),
            path: Some("tokenizer.json".to_string()),
        }],
    };
    let update_feed = ModelLibraryUpdateFeed {
        cursor: "model-library-updates:43".to_string(),
        stale_cursor: false,
        snapshot_required: false,
        events: vec![ModelLibraryUpdateEvent {
            cursor: "model-library-updates:43".to_string(),
            model_id: "pumas://models/tiny".to_string(),
            change_kind: ModelLibraryChangeKind::PackageFactsModified,
            fact_family: ModelFactFamily::PackageFacts,
            refresh_scope: ModelLibraryRefreshScope::SummaryAndDetail,
            selected_artifact_id: Some("gguf".to_string()),
            producer_revision: Some("rev-42".to_string()),
        }],
    };
    let summary_snapshot = ModelPackageFactsSummarySnapshot {
        cursor: "model-library-updates:43".to_string(),
        items: vec![ModelPackageFactsSummarySnapshotItem {
            model_id: "pumas://models/tiny".to_string(),
            status: ModelPackageFactsSummaryStatus::Missing,
            summary: None,
        }],
    };
    let load_security_policy = ModelLoadSecurityPolicy::default();

    for (name, value) in [
        (
            "runtime_lifecycle_snapshot",
            serde_json::to_value(lifecycle).expect("encode lifecycle"),
        ),
        (
            "inference_request_lifecycle_event",
            serde_json::to_value(event).expect("encode lifecycle event"),
        ),
        (
            "backend_capability_facts",
            serde_json::to_value(capability_facts).expect("encode capabilities"),
        ),
        (
            "resolved_model_source",
            serde_json::to_value(source).expect("encode source"),
        ),
        (
            "inference_execution_request",
            serde_json::to_value(typed_request).expect("encode typed request"),
        ),
        (
            "inference_execution_text_result",
            serde_json::to_value(typed_text_result).expect("encode typed text result"),
        ),
        (
            "inference_execution_embedding_result",
            serde_json::to_value(typed_embedding_result).expect("encode typed embedding result"),
        ),
        (
            "backend_compatibility_report",
            serde_json::to_value(compatibility_report).expect("encode compatibility report"),
        ),
        (
            "model_library_update_feed",
            serde_json::to_value(update_feed).expect("encode update feed"),
        ),
        (
            "model_package_facts_summary_snapshot",
            serde_json::to_value(summary_snapshot).expect("encode summary snapshot"),
        ),
        (
            "model_load_security_policy",
            serde_json::to_value(load_security_policy).expect("encode load security policy"),
        ),
    ] {
        assert_json_keys_avoid_scheduler_policy_language(name, &value);
    }
}

fn assert_json_keys_avoid_scheduler_policy_language(context: &str, value: &serde_json::Value) {
    const FORBIDDEN_KEY_PARTS: &[&str] = &[
        "admission",
        "eviction",
        "priority",
        "reservation",
        "scheduler_policy",
        "selected_best_backend",
    ];

    match value {
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                let normalized = key.to_ascii_lowercase();
                for forbidden in FORBIDDEN_KEY_PARTS {
                    assert!(
                        !normalized.contains(forbidden),
                        "{context} key '{key}' must stay factual and avoid scheduler policy term '{forbidden}'"
                    );
                }
                assert_json_keys_avoid_scheduler_policy_language(context, nested);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                assert_json_keys_avoid_scheduler_policy_language(context, item);
            }
        }
        _ => {}
    }
}

#[test]
fn stale_package_facts_cache_rows_wrap_canonical_facts_json() {
    let stale_cache_row: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/inference_package_facts/stale_package_facts.json"
    ))
    .expect("decode stale cache row fixture");
    let facts_json = stale_cache_row["facts_json"]
        .as_str()
        .expect("stale cache row should carry facts_json");
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(facts_json).expect("decode stale cache row facts_json");

    assert!(facts.uses_current_contract());
    assert_eq!(
        facts.task.task_type_primary.as_deref(),
        Some("text_generation")
    );
}

#[test]
fn package_facts_use_nested_pumas_artifact_task_and_backend_hint_shape() {
    let facts: ResolvedModelPackageFacts = serde_json::from_str(PACKAGE_FACT_FIXTURES[0].1)
        .expect("decode gguf text generation fixture");

    assert_eq!(
        facts.artifact.entry_path,
        "llm/llama/tiny-gguf/tiny-Q4_K_M.gguf"
    );
    assert_eq!(
        facts.task.task_type_primary.as_deref(),
        Some("text_generation")
    );
    assert_eq!(facts.task.input_modalities, vec!["text"]);
    assert_eq!(facts.task.output_modalities, vec!["text"]);
    assert_eq!(
        facts.backend_hints.accepted,
        vec![BackendHintLabel::LlamaCpp]
    );
    assert!(facts.components.iter().any(|component| {
        component.status == PackageFactStatus::Present
            && component.relative_path.as_deref() == Some("tiny-Q4_K_M.gguf")
    }));
}

#[test]
fn default_task_registry_seeds_transformers_aligned_vertical_slices() {
    let entries = default_task_registry_entries();

    let text_generation = entries
        .iter()
        .find(|entry| entry.task_id == InferenceTaskId::TextGeneration)
        .expect("text generation task");
    assert_eq!(text_generation.canonical_label(), "text_generation");
    assert_eq!(text_generation.task_family, TaskFamily::Generative);
    assert_eq!(text_generation.support_tier, SupportTier::Stable);
    assert_eq!(
        text_generation.execution_behavior,
        TaskExecutionBehavior::Generates
    );
    assert_eq!(
        text_generation.streaming_support,
        TaskStreamingSupport::BackendDependent
    );
    assert!(text_generation
        .required_components
        .contains(&ProcessorComponentKind::Tokenizer));

    let embedding =
        resolve_task_registry_entry("feature-extraction").expect("embedding alias should resolve");
    assert_eq!(embedding.task_id, InferenceTaskId::Embedding);
    assert_eq!(embedding.task_family, TaskFamily::Embedding);
    assert_eq!(
        embedding.modality_signature.outputs,
        vec![InferenceModality::Embedding]
    );

    let rerank =
        resolve_task_registry_entry("text-reranking").expect("rerank alias should resolve");
    assert_eq!(rerank.task_id, InferenceTaskId::Rerank);
    assert_eq!(rerank.execution_behavior, TaskExecutionBehavior::Scores);
}

#[test]
fn default_task_registry_entries_have_complete_public_contract_shape() {
    let mut canonical_task_ids = BTreeSet::new();

    for entry in default_task_registry_entries() {
        assert!(
            canonical_task_ids.insert(entry.canonical_label().to_string()),
            "duplicate task registry entry for {}",
            entry.canonical_label()
        );
        assert_ne!(
            entry.task_id,
            InferenceTaskId::Unknown,
            "registry entries must expose typed task ids"
        );
        assert_ne!(
            entry.task_family,
            TaskFamily::Unknown,
            "task {} should classify its task family",
            entry.canonical_label()
        );
        assert_ne!(
            entry.execution_behavior,
            TaskExecutionBehavior::Unknown,
            "task {} should classify its execution behavior",
            entry.canonical_label()
        );
        assert_ne!(
            entry.streaming_support,
            TaskStreamingSupport::Unknown,
            "task {} should define streaming support",
            entry.canonical_label()
        );
        assert_ne!(
            entry.support_tier,
            SupportTier::Unknown,
            "task {} should define support tier",
            entry.canonical_label()
        );
        assert!(
            !entry.modality_signature.inputs.is_empty(),
            "task {} should declare input modalities",
            entry.canonical_label()
        );
        assert!(
            !entry.modality_signature.outputs.is_empty(),
            "task {} should declare output modalities",
            entry.canonical_label()
        );
        assert!(
            !entry.result_family.trim().is_empty(),
            "task {} should declare a result family",
            entry.canonical_label()
        );

        for label in entry
            .aliases
            .iter()
            .map(String::as_str)
            .chain(entry.upstream_task_ids.iter().map(String::as_str))
            .chain(std::iter::once(entry.canonical_label()))
        {
            let normalized = normalize_task_label(label);
            assert!(
                !normalized.is_empty(),
                "task {} should not expose blank labels",
                entry.canonical_label()
            );
            for forbidden in [
                "admission",
                "reservation",
                "priority",
                "eviction",
                "scheduler",
                "selected_best_backend",
            ] {
                assert!(
                    !normalized.contains(forbidden),
                    "task {} label '{}' leaks scheduler/runtime policy language",
                    entry.canonical_label(),
                    label
                );
            }
        }

        let contract = entry.request_contract().unwrap_or_else(|| {
            panic!(
                "task {} should publish a request contract",
                entry.canonical_label()
            )
        });
        assert_eq!(contract.task_id, entry.task_id);
        assert_eq!(contract.streaming_support, entry.streaming_support);
        assert_eq!(
            contract.required_input_modalities,
            entry.modality_signature.inputs
        );
        assert_eq!(contract.output_modalities, entry.modality_signature.outputs);
    }
}

#[test]
fn task_request_contracts_publish_transformers_aligned_execution_matrix() {
    for (
        task_id,
        input_kind,
        result_kind,
        execution_supported,
        streaming_support,
        input_modalities,
        output_modalities,
    ) in [
        (
            InferenceTaskId::TextGeneration,
            InferenceExecutionInputKind::TextGeneration,
            InferenceExecutionResultKind::TextGeneration,
            true,
            TaskStreamingSupport::BackendDependent,
            vec![InferenceModality::Text],
            vec![InferenceModality::Text],
        ),
        (
            InferenceTaskId::ChatCompletion,
            InferenceExecutionInputKind::TextGeneration,
            InferenceExecutionResultKind::TextGeneration,
            true,
            TaskStreamingSupport::BackendDependent,
            vec![InferenceModality::Text],
            vec![InferenceModality::Text],
        ),
        (
            InferenceTaskId::Embedding,
            InferenceExecutionInputKind::Embedding,
            InferenceExecutionResultKind::Embedding,
            true,
            TaskStreamingSupport::Unsupported,
            vec![InferenceModality::Text],
            vec![InferenceModality::Embedding],
        ),
        (
            InferenceTaskId::Rerank,
            InferenceExecutionInputKind::Rerank,
            InferenceExecutionResultKind::Rerank,
            true,
            TaskStreamingSupport::Unsupported,
            vec![InferenceModality::Text, InferenceModality::Json],
            vec![InferenceModality::Json],
        ),
        (
            InferenceTaskId::ImageGeneration,
            InferenceExecutionInputKind::ImageGeneration,
            InferenceExecutionResultKind::ImageGeneration,
            true,
            TaskStreamingSupport::Unsupported,
            vec![InferenceModality::Text],
            vec![InferenceModality::Image],
        ),
        (
            InferenceTaskId::ImageUnderstanding,
            InferenceExecutionInputKind::ImageUnderstanding,
            InferenceExecutionResultKind::ImageUnderstanding,
            false,
            TaskStreamingSupport::BackendDependent,
            vec![InferenceModality::Image, InferenceModality::Text],
            vec![InferenceModality::Text],
        ),
        (
            InferenceTaskId::DepthEstimation,
            InferenceExecutionInputKind::DepthEstimation,
            InferenceExecutionResultKind::DepthEstimation,
            false,
            TaskStreamingSupport::Unsupported,
            vec![InferenceModality::Image],
            vec![InferenceModality::Image, InferenceModality::PointCloud],
        ),
        (
            InferenceTaskId::AudioTranscription,
            InferenceExecutionInputKind::AudioTranscription,
            InferenceExecutionResultKind::AudioTranscription,
            true,
            TaskStreamingSupport::BackendDependent,
            vec![InferenceModality::Audio],
            vec![InferenceModality::Text],
        ),
        (
            InferenceTaskId::VideoUnderstanding,
            InferenceExecutionInputKind::VideoUnderstanding,
            InferenceExecutionResultKind::VideoUnderstanding,
            false,
            TaskStreamingSupport::Unsupported,
            vec![InferenceModality::Video, InferenceModality::Text],
            vec![InferenceModality::Text],
        ),
        (
            InferenceTaskId::MultimodalGeneration,
            InferenceExecutionInputKind::MultimodalGeneration,
            InferenceExecutionResultKind::MultimodalGeneration,
            false,
            TaskStreamingSupport::BackendDependent,
            vec![
                InferenceModality::Text,
                InferenceModality::Image,
                InferenceModality::Audio,
            ],
            vec![InferenceModality::Text],
        ),
    ] {
        let entry = resolve_task_registry_entry(task_id.canonical_label())
            .unwrap_or_else(|| panic!("task {task_id:?} should resolve"));
        let contract = entry
            .request_contract()
            .unwrap_or_else(|| panic!("task {task_id:?} should publish a request contract"));

        assert_eq!(contract.task_id, task_id);
        assert_eq!(contract.input_kind, input_kind);
        assert_eq!(contract.result_kind, result_kind);
        assert_eq!(contract.execution_supported, execution_supported);
        assert_eq!(contract.streaming_support, streaming_support);
        assert_eq!(contract.required_input_modalities, input_modalities);
        assert_eq!(contract.output_modalities, output_modalities);
    }
}

#[test]
fn task_request_contract_wire_shape_preserves_snake_case_defaults_and_unknown_fields() {
    let encoded = serde_json::json!({
        "task_id": "audio_transcription",
        "input_kind": "audio_transcription",
        "result_kind": "audio_transcription",
        "execution_supported": true,
        "streaming_support": "unsupported",
        "required_input_modalities": ["audio"],
        "output_modalities": ["text"],
        "future_transformers_task_field": {
            "ignored": true
        }
    });

    let decoded: TaskRequestContract =
        serde_json::from_value(encoded).expect("task request contract decodes");
    let round_tripped = serde_json::to_value(&decoded).expect("task request contract encodes");

    assert_eq!(decoded.task_id, InferenceTaskId::AudioTranscription);
    assert_eq!(
        decoded.input_kind,
        InferenceExecutionInputKind::AudioTranscription
    );
    assert_eq!(
        decoded.result_kind,
        InferenceExecutionResultKind::AudioTranscription
    );
    assert!(decoded.execution_supported);
    assert_eq!(decoded.streaming_support, TaskStreamingSupport::Unsupported);
    assert_eq!(
        decoded.required_input_modalities,
        vec![InferenceModality::Audio]
    );
    assert_eq!(decoded.output_modalities, vec![InferenceModality::Text]);
    assert_eq!(
        round_tripped["task_id"],
        serde_json::json!("audio_transcription")
    );
    assert_eq!(
        round_tripped["input_kind"],
        serde_json::json!("audio_transcription")
    );
    assert_eq!(
        round_tripped["result_kind"],
        serde_json::json!("audio_transcription")
    );
    assert_eq!(
        round_tripped["streaming_support"],
        serde_json::json!("unsupported")
    );
    assert_eq!(
        round_tripped["required_input_modalities"][0],
        serde_json::json!("audio")
    );
    assert_eq!(
        round_tripped["output_modalities"][0],
        serde_json::json!("text")
    );
    assert!(round_tripped
        .get("future_transformers_task_field")
        .is_none());

    let minimal = TaskRequestContract {
        task_id: InferenceTaskId::TextGeneration,
        input_kind: InferenceExecutionInputKind::TextGeneration,
        result_kind: InferenceExecutionResultKind::TextGeneration,
        execution_supported: true,
        streaming_support: TaskStreamingSupport::BackendDependent,
        required_input_modalities: Vec::new(),
        output_modalities: Vec::new(),
    };
    let minimal_encoded = serde_json::to_value(&minimal).expect("minimal contract encodes");

    assert_eq!(
        minimal_encoded["task_id"],
        serde_json::json!("text_generation")
    );
    assert_eq!(
        minimal_encoded["streaming_support"],
        serde_json::json!("backend_dependent")
    );
    assert!(minimal_encoded.get("required_input_modalities").is_none());
    assert!(minimal_encoded.get("output_modalities").is_none());
}

#[test]
fn task_registry_labels_normalize_without_leaking_backend_policy() {
    assert_eq!(normalize_task_label(" text-generation "), "text_generation");
    assert_eq!(
        normalize_task_label("Automatic Speech Recognition"),
        "automatic_speech_recognition"
    );
    assert_eq!(normalize_modality_label("Point Cloud"), "point_cloud");

    let audio = resolve_task_registry_entry("automatic-speech-recognition")
        .expect("audio transcription alias");
    assert_eq!(audio.task_id, InferenceTaskId::AudioTranscription);
    assert_eq!(audio.task_family, TaskFamily::Perception);
    assert_eq!(
        audio.required_components,
        vec![ProcessorComponentKind::AudioFeatureExtractor]
    );

    let video = resolve_task_registry_entry("video-text-to-text").expect("video roadmap task");
    assert_eq!(video.task_id, InferenceTaskId::VideoUnderstanding);
    assert_eq!(video.support_tier, SupportTier::Roadmap);

    let depth = resolve_task_registry_entry("depth-estimation").expect("depth roadmap task");
    assert_eq!(depth.task_id, InferenceTaskId::DepthEstimation);
    assert_eq!(depth.support_tier, SupportTier::Roadmap);
    assert_eq!(
        depth.modality_signature.inputs,
        vec![InferenceModality::Image]
    );
    assert_eq!(
        depth.modality_signature.outputs,
        vec![InferenceModality::Image, InferenceModality::PointCloud]
    );
}

#[test]
fn task_registry_matches_package_task_and_modality_evidence() {
    let text_generation = resolve_task_registry_entry("text-generation")
        .expect("text generation task should be seeded");

    assert!(text_generation.matches_task_evidence(&TaskEvidence {
        pipeline_tag: Some("causal-lm".to_string()),
        task_type_primary: Some("text-generation".to_string()),
        input_modalities: vec!["text".to_string()],
        output_modalities: vec!["text".to_string()],
    }));
    assert!(text_generation.matches_modality_evidence(&TaskEvidence {
        input_modalities: vec!["Text".to_string()],
        output_modalities: vec!["text".to_string()],
        ..TaskEvidence::default()
    }));
    assert!(!text_generation.matches_task_evidence(&TaskEvidence {
        pipeline_tag: Some("feature-extraction".to_string()),
        ..TaskEvidence::default()
    }));
    assert!(!text_generation.matches_modality_evidence(&TaskEvidence {
        input_modalities: vec!["image".to_string()],
        ..TaskEvidence::default()
    }));
}

#[test]
fn task_registry_resolution_returns_validated_entry_from_package_evidence() {
    let entry = resolve_task_registry_entry_from_evidence(&TaskEvidence {
        pipeline_tag: Some("causal-lm".to_string()),
        task_type_primary: Some("text-generation".to_string()),
        input_modalities: vec!["Text".to_string()],
        output_modalities: vec!["text".to_string()],
    })
    .expect("text generation evidence should resolve");

    assert_eq!(entry.task_id, InferenceTaskId::TextGeneration);
}

#[test]
fn task_registry_resolution_reports_unsupported_task_evidence() {
    let diagnostic = resolve_task_registry_entry_from_evidence(&TaskEvidence {
        pipeline_tag: Some("object-detection".to_string()),
        task_type_primary: Some("object_detection".to_string()),
        input_modalities: vec!["image".to_string()],
        output_modalities: vec!["json".to_string()],
    })
    .expect_err("unsupported task should report diagnostic");

    assert_eq!(
        diagnostic.kind,
        TaskRegistryResolutionDiagnosticKind::UnsupportedTaskLabel
    );
    assert_eq!(
        diagnostic.labels,
        vec![
            "object_detection".to_string(),
            "object-detection".to_string()
        ]
    );

    let encoded = serde_json::to_value(&diagnostic).expect("encode diagnostic");
    assert_eq!(encoded["kind"], serde_json::json!("unsupported_task_label"));
}

#[test]
fn task_registry_resolution_reports_missing_task_evidence() {
    let diagnostic = resolve_task_registry_entry_from_evidence(&TaskEvidence {
        input_modalities: vec!["text".to_string()],
        output_modalities: vec!["text".to_string()],
        ..TaskEvidence::default()
    })
    .expect_err("missing task labels should report diagnostic");

    assert_eq!(
        diagnostic.kind,
        TaskRegistryResolutionDiagnosticKind::MissingTaskEvidence
    );
    assert!(diagnostic.labels.is_empty());
}

#[test]
fn task_registry_resolution_reports_conflicting_task_evidence() {
    let diagnostic = resolve_task_registry_entry_from_evidence(&TaskEvidence {
        pipeline_tag: Some("feature-extraction".to_string()),
        task_type_primary: Some("text-generation".to_string()),
        input_modalities: vec!["text".to_string()],
        output_modalities: vec!["text".to_string()],
    })
    .expect_err("conflicting task labels should report diagnostic");

    assert_eq!(
        diagnostic.kind,
        TaskRegistryResolutionDiagnosticKind::ConflictingTaskEvidence
    );
    assert_eq!(
        diagnostic.canonical_task_ids,
        vec!["text_generation".to_string(), "embedding".to_string()]
    );
}

#[test]
fn task_registry_resolution_reports_modality_mismatch() {
    let diagnostic = resolve_task_registry_entry_from_evidence(&TaskEvidence {
        task_type_primary: Some("text-generation".to_string()),
        input_modalities: vec!["image".to_string()],
        output_modalities: vec!["text".to_string()],
        ..TaskEvidence::default()
    })
    .expect_err("modality mismatch should report diagnostic");

    assert_eq!(
        diagnostic.kind,
        TaskRegistryResolutionDiagnosticKind::ModalityMismatch
    );
    assert_eq!(
        diagnostic.canonical_task_ids,
        vec!["text_generation".to_string()]
    );
    assert_eq!(diagnostic.input_modalities, vec!["image".to_string()]);
}

#[test]
fn task_registry_entry_decodes_append_only_defaults() {
    let raw = serde_json::json!({
        "task_id": "embedding",
        "aliases": ["feature-extraction"],
        "modality_signature": {
            "inputs": ["text"],
            "outputs": ["embedding"]
        },
        "result_family": "embedding_vector",
        "support_tier": "stable"
    });

    let entry: TaskRegistryEntry = serde_json::from_value(raw).expect("decode task entry");
    assert_eq!(entry.task_id, InferenceTaskId::Embedding);
    assert_eq!(entry.task_family, TaskFamily::Unknown);
    assert_eq!(entry.execution_behavior, TaskExecutionBehavior::Unknown);
    assert_eq!(entry.streaming_support, TaskStreamingSupport::Unknown);
    assert!(entry.required_components.is_empty());
    assert!(entry.matches_label("feature-extraction"));
}

#[test]
fn resolved_model_source_projects_from_pumas_package_facts() {
    let facts: ResolvedModelPackageFacts = serde_json::from_str(PACKAGE_FACT_FIXTURES[0].1)
        .expect("decode gguf text generation fixture");

    let source = ResolvedModelSource::from_package_facts(&facts);

    assert!(source.is_pumas_resolved());
    assert!(source.validate_for_backend_load().is_ok());
    assert_eq!(
        source.source_contract_version,
        MODEL_PACKAGE_FACTS_CONTRACT_VERSION
    );
    assert_eq!(source.source_kind, ResolvedModelSourceKind::PumasResolved);
    assert_eq!(source.artifact_kind, facts.artifact.artifact_kind);
    assert_eq!(source.entry_path, facts.artifact.entry_path);
    assert_eq!(source.model_ref.as_ref(), Some(&facts.model_ref));
    assert_eq!(source.storage_kind, facts.artifact.storage_kind);
    assert_eq!(source.validation_state, facts.artifact.validation_state);
}

#[test]
fn resolved_model_source_distinguishes_direct_sources_from_pumas_refs() {
    let direct = ResolvedModelSource::direct_local(
        ResolvedModelSourceKind::DirectGgufPath,
        ModelArtifactKind::Gguf,
        "/tmp/model.gguf",
    );
    assert!(!direct.is_pumas_resolved());
    assert_eq!(direct.model_ref, None);
    assert_eq!(direct.entry_path, "/tmp/model.gguf");

    let repo = ResolvedModelSource::hugging_face_repo(
        "org/model",
        Some("main".to_string()),
        ModelArtifactKind::HfCompatibleDirectory,
    );
    assert_eq!(repo.source_kind, ResolvedModelSourceKind::HuggingFaceRepo);
    assert_eq!(repo.entry_path, "org/model");
    assert_eq!(repo.repo_id.as_deref(), Some("org/model"));
    assert_eq!(repo.revision.as_deref(), Some("main"));
    assert!(repo.validate_for_backend_load().is_ok());
}

#[test]
fn resolved_model_source_rejects_invalid_backend_load_states() {
    let mut source = ResolvedModelSource {
        source_contract_version: MODEL_PACKAGE_FACTS_CONTRACT_VERSION + 1,
        source_kind: ResolvedModelSourceKind::PumasResolved,
        artifact_kind: ModelArtifactKind::Unknown,
        entry_path: "   ".to_string(),
        storage_kind: ModelStorageKind::LibraryOwned,
        validation_state: ModelValidationState::Invalid,
        model_ref: None,
        repo_id: None,
        revision: None,
        selected_files: Vec::new(),
        companion_artifacts: Vec::new(),
        diagnostics: Vec::new(),
    };

    let diagnostics = source
        .validate_for_backend_load()
        .expect_err("invalid Pumas source should report diagnostics");
    assert_diagnostic_code(&diagnostics, "model_source_contract_version_mismatch");
    assert_diagnostic_code(&diagnostics, "model_source_missing_entry_path");
    assert_diagnostic_code(&diagnostics, "pumas_resolved_source_missing_model_ref");
    assert_diagnostic_code(&diagnostics, "model_source_artifact_kind_unknown");
    assert_diagnostic_code(&diagnostics, "model_source_artifact_invalid");

    source.source_contract_version = MODEL_PACKAGE_FACTS_CONTRACT_VERSION;
    source.source_kind = ResolvedModelSourceKind::DirectGgufPath;
    source.artifact_kind = ModelArtifactKind::Gguf;
    source.entry_path = "/models/model.gguf".to_string();
    source.validation_state = ModelValidationState::Valid;
    source.model_ref = Some(PumasModelRef {
        model_id: "library-model".to_string(),
        revision: None,
        selected_artifact_id: None,
        selected_artifact_path: None,
        migration_diagnostics: Vec::new(),
    });

    let diagnostics = source
        .validate_for_backend_load()
        .expect_err("direct source with Pumas ref should report diagnostic");
    assert_diagnostic_code(&diagnostics, "direct_source_has_pumas_model_ref");
}

#[test]
fn resolved_model_source_wire_shape_defaults_optional_collections() {
    let raw = serde_json::json!({
        "source_contract_version": MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
        "source_kind": "hugging_face_repo",
        "artifact_kind": "hf_compatible_directory",
        "entry_path": "org/model",
        "storage_kind": "external_reference",
        "validation_state": "unknown",
        "repo_id": "org/model",
        "future_additive_field": {
            "ignored": true
        }
    });

    let source: ResolvedModelSource = serde_json::from_value(raw).expect("decode source");

    assert_eq!(source.source_kind, ResolvedModelSourceKind::HuggingFaceRepo);
    assert_eq!(source.repo_id.as_deref(), Some("org/model"));
    assert!(source.model_ref.is_none());
    assert!(source.selected_files.is_empty());
    assert!(source.companion_artifacts.is_empty());
    assert!(source.diagnostics.is_empty());
    assert!(source.validate_for_backend_load().is_ok());
}

fn assert_diagnostic_code(diagnostics: &[ModelPackageDiagnostic], expected: &str) {
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == expected),
        "expected diagnostic code {expected}, got {diagnostics:?}"
    );
}

#[test]
fn generation_defaults_preserve_raw_pumas_defaults() {
    let facts: ResolvedModelPackageFacts = serde_json::from_str(package_fact_fixture(
        "hf_transformers_text_generation_package_facts.json",
    ))
    .expect("decode hf transformers fixture");

    assert_eq!(facts.generation_defaults.status, PackageFactStatus::Present);
    assert_eq!(
        facts
            .generation_defaults
            .defaults
            .as_ref()
            .and_then(|defaults| defaults.get("max_new_tokens"))
            .and_then(serde_json::Value::as_u64),
        Some(128)
    );
}

#[test]
fn model_load_security_policy_defaults_closed_and_local() {
    let policy = ModelLoadSecurityPolicy::default();

    assert_eq!(policy.trust_remote_code, ModelRemoteCodePolicy::Deny);
    assert_eq!(policy.network, ModelLoadNetworkPolicy::LocalOnly);
    assert_eq!(policy.cache, ModelLoadCachePolicy::BackendDefault);
    assert!(!policy.allow_remote_code());
    assert!(policy.local_files_only());

    let encoded = serde_json::to_value(&policy).expect("encode policy");
    assert_eq!(encoded["trust_remote_code"], serde_json::json!("deny"));
    assert_eq!(encoded["network"], serde_json::json!("local_only"));
    assert!(encoded.get("token").is_none());
}

#[test]
fn model_load_security_policy_preserves_revision_without_secret_tokens() {
    let raw = serde_json::json!({
        "trust_remote_code": "allow",
        "network": "allow_network",
        "cache": "bypass_cache",
        "auth_token_source": "environment",
        "revision": "weights-rev",
        "code_revision": "code-rev",
        "decision_id": "trust-001",
        "accepted_code_sources": ["configuration_tiny.py"]
    });

    let policy: ModelLoadSecurityPolicy = serde_json::from_value(raw).expect("decode policy");

    assert!(policy.allow_remote_code());
    assert!(!policy.local_files_only());
    assert_eq!(policy.cache, ModelLoadCachePolicy::BypassCache);
    assert_eq!(policy.revision.as_deref(), Some("weights-rev"));
    assert_eq!(policy.code_revision.as_deref(), Some("code-rev"));
}

#[test]
fn generation_options_group_transformers_aligned_request_fields() {
    let mut backend_extensions = BTreeMap::new();
    backend_extensions.insert(
        "transformers:watermarking_config".to_string(),
        serde_json::json!({"greenlist_ratio": 0.25}),
    );
    let options = GenerationOptions {
        length: inference::LengthGenerationOptions {
            max_new_tokens: Some(128),
            ..Default::default()
        },
        sampling: inference::SamplingGenerationOptions {
            temperature: Some(0.7),
            top_p: Some(0.95),
            seed: Some(42),
            ..Default::default()
        },
        stopping: inference::StoppingGenerationOptions {
            stop_strings: vec!["</s>".to_string()],
            eos_token_ids: vec![2],
        },
        cache: inference::CacheGenerationOptions {
            use_cache: Some(true),
            kv_cache_checkpoint_requested: Some(false),
        },
        backend_extensions,
        ..Default::default()
    };

    let json = serde_json::to_string(&options).expect("encode generation options");
    assert!(json.contains("max_new_tokens"));
    assert!(json.contains("transformers:watermarking_config"));

    let decoded: GenerationOptions = serde_json::from_str(&json).expect("decode options");
    assert_eq!(decoded.length.max_new_tokens, Some(128));
    assert_eq!(decoded.sampling.top_p, Some(0.95));
    assert_eq!(decoded.sampling.seed, Some(42));
    assert_eq!(decoded.stopping.stop_strings, vec!["</s>"]);
    assert_eq!(decoded.cache.use_cache, Some(true));
    assert!(decoded
        .backend_extensions
        .contains_key("transformers:watermarking_config"));
    assert_eq!(
        decoded.requested_option_paths(),
        vec![
            "length.max_new_tokens",
            "sampling.temperature",
            "sampling.top_p",
            "sampling.seed",
            "stopping.stop_strings",
            "stopping.eos_token_ids",
            "cache.use_cache",
            "cache.kv_cache_checkpoint_requested",
            "backend_extensions.transformers:watermarking_config",
        ]
    );
}

#[test]
fn generation_options_reject_unscoped_backend_extension_keys() {
    let options = GenerationOptions {
        backend_extensions: BTreeMap::from([
            (
                "transformers:renormalize_logits".to_string(),
                serde_json::json!(true),
            ),
            ("top_k".to_string(), serde_json::json!(40)),
            ("llama.cpp:mirostat".to_string(), serde_json::json!(1)),
            (":missing_scope".to_string(), serde_json::json!(true)),
            ("missing_option:".to_string(), serde_json::json!(true)),
        ]),
        ..Default::default()
    };

    let diagnostics = options.backend_extension_scope_diagnostics();

    assert_eq!(diagnostics.len(), 3);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.state == OptionSupportState::Rejected && diagnostic.backend_key.is_none()
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "backend_extensions.top_k"
            && diagnostic.message.as_deref()
                == Some("backend extension keys must be scoped as <backend-or-adapter>:<option>")
    }));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "backend_extensions.:missing_scope"));
    assert!(diagnostics
        .iter()
        .any(|diagnostic| diagnostic.option_path == "backend_extensions.missing_option:"));
}

#[test]
fn generation_option_precedence_resolves_model_workflow_runtime_and_request_layers() {
    let model_defaults = serde_json::json!({
        "max_new_tokens": 128,
        "temperature": 0.7,
        "top_p": 0.95,
        "eos_token_ids": [2],
        "use_cache": true
    });
    let workflow_defaults = GenerationOptions {
        length: inference::LengthGenerationOptions {
            max_new_tokens: Some(256),
            ..Default::default()
        },
        sampling: inference::SamplingGenerationOptions {
            top_p: Some(0.9),
            ..Default::default()
        },
        ..Default::default()
    };
    let runtime_preset = GenerationOptions {
        sampling: inference::SamplingGenerationOptions {
            temperature: Some(0.5),
            ..Default::default()
        },
        stopping: inference::StoppingGenerationOptions {
            stop_strings: vec!["STOP".to_string()],
            ..Default::default()
        },
        ..Default::default()
    };
    let request_overrides = GenerationOptions {
        sampling: inference::SamplingGenerationOptions {
            temperature: Some(0.2),
            seed: Some(42),
            ..Default::default()
        },
        cache: inference::CacheGenerationOptions {
            use_cache: Some(false),
            ..Default::default()
        },
        ..Default::default()
    };

    let report = GenerationOptions::resolve_precedence(
        Some(&model_defaults),
        Some(&workflow_defaults),
        Some(&runtime_preset),
        Some(&request_overrides),
    );

    assert_eq!(report.options.length.max_new_tokens, Some(256));
    assert_eq!(report.options.sampling.top_p, Some(0.9));
    assert_eq!(report.options.sampling.temperature, Some(0.2));
    assert_eq!(report.options.sampling.seed, Some(42));
    assert_eq!(report.options.cache.use_cache, Some(false));
    assert_eq!(report.options.stopping.eos_token_ids, vec![2]);
    assert_eq!(report.options.stopping.stop_strings, vec!["STOP"]);
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "sampling.temperature"
            && diagnostic.source == GenerationOptionSource::RequestOverride
            && diagnostic.state == OptionSupportState::Honored
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.option_path == "length.max_new_tokens"
            && diagnostic.source == GenerationOptionSource::WorkflowDefaults
            && diagnostic.state == OptionSupportState::Defaulted
    }));
}

#[test]
fn option_support_diagnostics_round_trip_state_without_policy() {
    let diagnostic = OptionCompatibilityDiagnostic {
        option_path: "sampling.temperature".to_string(),
        state: OptionSupportState::Mapped,
        backend_key: Some("llama_cpp".to_string()),
        message: Some("mapped to llama.cpp temperature option".to_string()),
    };

    let json = serde_json::to_string(&diagnostic).expect("encode diagnostic");
    assert!(json.contains("sampling.temperature"));

    let decoded: OptionCompatibilityDiagnostic =
        serde_json::from_str(&json).expect("decode diagnostic");
    assert_eq!(decoded.state, OptionSupportState::Mapped);
}

#[test]
fn model_library_change_events_carry_cache_invalidation_scope() {
    let event = ModelLibraryUpdateEvent {
        cursor: "cursor-42".to_string(),
        change_kind: ModelLibraryChangeKind::PackageFactsModified,
        model_id: "pumas://models/llama-3.1-8b-q4".to_string(),
        selected_artifact_id: Some("main-gguf".to_string()),
        fact_family: ModelFactFamily::PackageFacts,
        refresh_scope: ModelLibraryRefreshScope::SummaryAndDetail,
        producer_revision: Some("revision-42".to_string()),
    };

    let json = serde_json::to_string(&event).expect("encode event");
    let decoded: ModelLibraryUpdateEvent = serde_json::from_str(&json).expect("decode event");

    assert_eq!(decoded.cursor, "cursor-42");
    assert_eq!(decoded.fact_family, ModelFactFamily::PackageFacts);
    assert!(decoded.refreshes_summary());
    assert!(decoded.refreshes_details());
}

#[test]
fn model_library_update_feed_matches_pumas_cursor_contract() {
    let raw = serde_json::json!({
        "cursor": "model-library-updates:43",
        "stale_cursor": false,
        "snapshot_required": false,
        "events": [
            {
                "cursor": "model-library-updates:43",
                "model_id": "pumas://models/llama-3.1-8b-q4",
                "change_kind": "package_facts_modified",
                "fact_family": "package_facts",
                "refresh_scope": "summary_and_detail",
                "selected_artifact_id": "main-gguf",
                "producer_revision": "revision-42"
            }
        ]
    });

    let feed: ModelLibraryUpdateFeed = serde_json::from_value(raw).expect("decode update feed");

    assert_eq!(feed.cursor, "model-library-updates:43");
    assert!(!feed.stale_cursor);
    assert!(!feed.snapshot_required);
    assert_eq!(feed.events.len(), 1);
    assert_eq!(
        feed.events[0].change_kind,
        ModelLibraryChangeKind::PackageFactsModified
    );
    assert_eq!(
        feed.events[0].selected_artifact_id.as_deref(),
        Some("main-gguf")
    );
}

#[test]
fn model_package_summary_snapshot_matches_pumas_startup_shape() {
    let raw = serde_json::json!({
        "cursor": "model-library-updates:43",
        "items": [
            {
                "model_id": "pumas://models/llama-3.1-8b-q4",
                "status": "cached",
                "summary": null
            },
            {
                "model_id": "pumas://models/missing-summary",
                "status": "missing"
            }
        ]
    });

    let snapshot: ModelPackageFactsSummarySnapshot =
        serde_json::from_value(raw).expect("decode summary snapshot");

    assert_eq!(snapshot.cursor, "model-library-updates:43");
    assert_eq!(snapshot.items.len(), 2);
    assert_eq!(
        snapshot.items[0].status,
        ModelPackageFactsSummaryStatus::Cached
    );
    assert_eq!(
        snapshot.items[1].status,
        ModelPackageFactsSummaryStatus::Missing
    );
    assert!(snapshot.items.iter().all(|item| item.summary.is_none()));
}

#[test]
fn lifecycle_phases_use_transformers_aligned_boundary_names() {
    let phases = [
        InferenceLifecyclePhase::ModelPackageResolution,
        InferenceLifecyclePhase::TaskValidation,
        InferenceLifecyclePhase::Preprocessing,
        InferenceLifecyclePhase::BackendExecution,
        InferenceLifecyclePhase::Postprocessing,
        InferenceLifecyclePhase::ResultProjection,
    ];

    let json = serde_json::to_string(&phases).expect("encode phases");
    assert!(json.contains("model_package_resolution"));
    assert!(json.contains("preprocessing"));
    assert!(json.contains("postprocessing"));
}

#[test]
fn embeddings_are_normal_task_evidence() {
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(PACKAGE_FACT_FIXTURES[1].1).expect("decode embedding fixture");

    assert_eq!(facts.task.task_type_primary.as_deref(), Some("embedding"));
    assert_eq!(facts.task.output_modalities, vec!["embedding"]);
}
