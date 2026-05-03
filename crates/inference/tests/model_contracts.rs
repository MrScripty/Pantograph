use inference::{
    BackendHintSource, InferenceLifecyclePhase, InferenceTaskId, ModelExecutionDescriptor,
    ModelExecutionStorageKind, ModelExecutionValidationState, ModelFactFamily,
    ModelLibraryChangeKind, ModelLibraryRefreshScope, ModelLibraryUpdateEvent,
    ModelLibraryUpdateFeed, ModelPackageFactsSummarySnapshot, ModelPackageFactsSummaryStatus,
    ModelValidationState, OptionCompatibilityDiagnostic, OptionSupportState,
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
        "stale_package_facts.json",
        include_str!("fixtures/inference_package_facts/stale_package_facts.json"),
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
    (
        "remote_search_mlx_vllm_hint.json",
        include_str!("fixtures/inference_package_facts/remote_search_mlx_vllm_hint.json"),
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
            decoded.contract_version,
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
    let facts: ResolvedModelPackageFacts =
        serde_json::from_str(PACKAGE_FACT_FIXTURES[9].1).expect("decode remote hint fixture");

    assert_eq!(facts.validation_state, ModelValidationState::Unknown);
    assert!(facts.feasible_execution_candidates.is_empty());
    assert!(facts.backend_hints.iter().all(|hint| {
        hint.source == BackendHintSource::RemoteSearchTag && !hint.executable_guarantee
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

    assert!(facts
        .task_evidence
        .iter()
        .any(|evidence| evidence.task_id == InferenceTaskId::Embedding));
    assert!(facts
        .feasible_execution_candidates
        .iter()
        .any(|candidate| candidate.task_id == InferenceTaskId::Embedding));
}
