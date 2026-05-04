use std::collections::{BTreeMap, BTreeSet};

use inference::{
    default_task_registry_entries, normalize_modality_label, normalize_task_label,
    resolve_task_registry_entry, resolve_task_registry_entry_from_evidence, BackendCapabilityFacts,
    BackendHintLabel, BackendTaskCapability, GenerationOptionSource, GenerationOptions,
    InferenceLifecyclePhase, InferenceModality, InferenceRequestLifecycleEvent,
    InferenceRequestLifecycleEventKind, InferenceTaskId, InferenceUsage, ModelArtifactKind,
    ModelExecutionDescriptor, ModelExecutionStorageKind, ModelExecutionValidationState,
    ModelFactFamily, ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibraryUpdateEvent,
    ModelLibraryUpdateFeed, ModelLoadCachePolicy, ModelLoadNetworkPolicy, ModelLoadSecurityPolicy,
    ModelPackageDiagnostic, ModelPackageFactsSummarySnapshot, ModelPackageFactsSummaryStatus,
    ModelRemoteCodePolicy, ModelStorageKind, ModelValidationState, OptionCompatibilityDiagnostic,
    OptionSupportState, PackageFactStatus, ProcessorComponentKind, PumasModelRef,
    ResolvedModelPackageFacts, ResolvedModelSource, ResolvedModelSourceKind,
    RuntimeLifecycleSnapshot, SupportTier, TaskEvidence, TaskExecutionBehavior, TaskFamily,
    TaskRegistryEntry, TaskRegistryResolutionDiagnosticKind, TaskStreamingSupport,
    MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
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
        "hf_transformers_text_generation_package_facts.json",
        include_str!(
            "fixtures/inference_package_facts/hf_transformers_text_generation_package_facts.json"
        ),
    ),
    (
        "hf_multimodal_processor_package_facts.json",
        include_str!("fixtures/inference_package_facts/hf_multimodal_processor_package_facts.json"),
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
    let event = InferenceRequestLifecycleEvent {
        request_id: Some("req-1".to_string()),
        phase: InferenceLifecyclePhase::BackendExecution,
        kind: InferenceRequestLifecycleEventKind::Completed,
        occurred_at_ms: 42,
        task_id: Some("text_generation".to_string()),
        backend_key: Some("llama_cpp".to_string()),
        runtime_id: Some("runtime.llama_cpp".to_string()),
        runtime_instance_id: Some("runtime.llama_cpp.1".to_string()),
        model_id: Some("pumas://models/tiny".to_string()),
        usage: Some(InferenceUsage {
            prompt_tokens: Some(2),
            completion_tokens: Some(3),
            total_tokens: Some(5),
        }),
        cache_handle_id: Some("kv-1".to_string()),
        detail: None,
        compatibility_report: None,
        compatibility_issues: Vec::new(),
        option_diagnostics: Vec::new(),
    };
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
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(PACKAGE_FACT_FIXTURES[2].1).expect("decode hf transformers fixture");

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
