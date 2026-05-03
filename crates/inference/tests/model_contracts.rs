use std::collections::BTreeMap;

use inference::{
    BackendHintLabel, GenerationOptions, InferenceLifecyclePhase, ModelExecutionDescriptor,
    ModelExecutionStorageKind, ModelExecutionValidationState, ModelFactFamily,
    ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibraryUpdateEvent,
    ModelLibraryUpdateFeed, ModelPackageFactsSummarySnapshot, ModelPackageFactsSummaryStatus,
    OptionCompatibilityDiagnostic, OptionSupportState, PackageFactStatus,
    ResolvedModelPackageFacts, MODEL_PACKAGE_FACTS_CONTRACT_VERSION,
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
