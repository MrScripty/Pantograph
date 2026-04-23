use super::*;

fn make_entry(model_id: &str, config_hash: &str) -> KvCacheEntry {
    KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: None,
            model_fingerprint: ModelFingerprint {
                model_id: model_id.to_string(),
                config_hash: config_hash.to_string(),
            },
            runtime_fingerprint: None,
            backend_hint: "test".to_string(),
            token_count: 100,
            markers: vec![],
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100],
    }
}

fn matching_fingerprint() -> ModelFingerprint {
    ModelFingerprint {
        model_id: "llama-7b".to_string(),
        config_hash: "abc".to_string(),
    }
}

fn different_fingerprint() -> ModelFingerprint {
    ModelFingerprint {
        model_id: "mistral-7b".to_string(),
        config_hash: "xyz".to_string(),
    }
}

struct MockCodec;

#[async_trait::async_trait]
impl KvCacheCodec for MockCodec {
    async fn truncate(&self, data: &[u8], token_position: usize) -> Result<Vec<u8>, KvCacheError> {
        let end = token_position.min(data.len());
        Ok(data[..end].to_vec())
    }

    fn model_fingerprint(&self) -> Result<ModelFingerprint, KvCacheError> {
        Ok(ModelFingerprint {
            model_id: "mock".to_string(),
            config_hash: "mock".to_string(),
        })
    }

    fn backend_name(&self) -> &'static str {
        "mock"
    }
}

#[tokio::test]
async fn test_load_validates_model_fingerprint() {
    let store = KvCacheStore::memory_only();
    let entry = make_entry("llama-7b", "abc");

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    let loaded = store.load(&cache_id, &matching_fingerprint()).await;
    assert!(
        loaded.is_ok(),
        "load with matching fingerprint should succeed"
    );

    let loaded_entry = loaded.unwrap();
    assert_eq!(
        loaded_entry.data,
        vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
    );
}

#[tokio::test]
async fn test_load_model_mismatch_returns_error() {
    let store = KvCacheStore::memory_only();
    let entry = make_entry("llama-7b", "abc");

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    let result = store.load(&cache_id, &different_fingerprint()).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        KvCacheError::ModelMismatch {
            cache_model,
            requested_model,
        } => {
            assert_eq!(cache_model, "llama-7b");
            assert_eq!(requested_model, "mistral-7b");
        }
        other => panic!("expected ModelMismatch, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_add_and_remove_marker() {
    let store = KvCacheStore::memory_only();
    let entry = make_entry("llama-7b", "abc");

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    store
        .add_marker(
            &cache_id,
            CacheMarker {
                name: "system".to_string(),
                token_position: 50,
                description: Some("End of system prompt".to_string()),
            },
        )
        .await
        .expect("add_marker should succeed");

    store
        .add_marker(
            &cache_id,
            CacheMarker {
                name: "examples".to_string(),
                token_position: 80,
                description: None,
            },
        )
        .await
        .expect("add_marker should succeed");

    let meta = store
        .get_metadata(&cache_id)
        .await
        .expect("get_metadata should succeed");
    assert_eq!(meta.markers.len(), 2);

    store
        .remove_marker(&cache_id, "system")
        .await
        .expect("remove_marker should succeed");

    let meta = store
        .get_metadata(&cache_id)
        .await
        .expect("get_metadata should succeed");
    assert_eq!(meta.markers.len(), 1);
    assert_eq!(meta.markers[0].name, "examples");

    let result = store.remove_marker(&cache_id, "nonexistent").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        KvCacheError::MarkerNotFound { marker_name } => {
            assert_eq!(marker_name, "nonexistent");
        }
        other => panic!("expected MarkerNotFound, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_truncate_to_marker_with_mock_codec() {
    let store = KvCacheStore::memory_only();
    let entry = make_entry("llama-7b", "abc");

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    store
        .add_marker(
            &cache_id,
            CacheMarker {
                name: "system".to_string(),
                token_position: 5,
                description: None,
            },
        )
        .await
        .unwrap();

    store
        .add_marker(
            &cache_id,
            CacheMarker {
                name: "examples".to_string(),
                token_position: 8,
                description: None,
            },
        )
        .await
        .unwrap();

    let codec = MockCodec;
    store
        .truncate_to_marker(&cache_id, "system", &codec)
        .await
        .expect("truncate should succeed");

    let loaded = store
        .load(&cache_id, &matching_fingerprint())
        .await
        .expect("load should succeed after truncation");
    assert_eq!(loaded.data, vec![10, 20, 30, 40, 50]);
    assert_eq!(loaded.metadata.token_count, 5);
    assert_eq!(loaded.metadata.markers.len(), 1);
    assert_eq!(loaded.metadata.markers[0].name, "system");
}

#[tokio::test]
async fn test_update_label() {
    let store = KvCacheStore::memory_only();
    let entry = make_entry("llama-7b", "abc");

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    let meta = store.get_metadata(&cache_id).await.unwrap();
    assert!(meta.label.is_none());

    store
        .update_label(&cache_id, Some("My Cache".to_string()))
        .await
        .expect("update_label should succeed");

    let meta = store.get_metadata(&cache_id).await.unwrap();
    assert_eq!(meta.label, Some("My Cache".to_string()));

    store
        .update_label(&cache_id, None)
        .await
        .expect("update_label should succeed");

    let meta = store.get_metadata(&cache_id).await.unwrap();
    assert!(meta.label.is_none());
}

#[tokio::test]
async fn test_load_handle_requires_matching_runtime_fingerprint() {
    let store = KvCacheStore::memory_only();
    let mut entry = make_entry("llama-7b", "abc");
    entry.metadata.runtime_fingerprint = Some(KvCacheRuntimeFingerprint {
        runtime_id: "runtime-a".to_string(),
        backend_key: "llamacpp".to_string(),
        tokenizer_fingerprint: "tok-a".to_string(),
        prompt_format_fingerprint: Some("chatml".to_string()),
        runtime_build_fingerprint: Some("build-a".to_string()),
    });

    let cache_id = store.save(entry, None).await.expect("save should succeed");

    let error = store
        .load_handle(
            &cache_id,
            &matching_fingerprint(),
            &KvCacheRuntimeFingerprint {
                runtime_id: "runtime-b".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-a".to_string(),
                prompt_format_fingerprint: Some("chatml".to_string()),
                runtime_build_fingerprint: Some("build-a".to_string()),
            },
        )
        .await
        .expect_err("mismatched runtime should fail");

    match error {
        KvCacheError::RuntimeMismatch {
            cache_runtime,
            requested_runtime,
        } => {
            assert_eq!(cache_runtime, "runtime-a");
            assert_eq!(requested_runtime, "runtime-b");
        }
        other => panic!("expected RuntimeMismatch, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_load_for_execution_requires_runtime_fingerprint() {
    let store = KvCacheStore::memory_only();
    let cache_id = store
        .save(make_entry("llama-7b", "abc"), None)
        .await
        .expect("save should succeed");

    let error = store
        .load_for_execution(
            &cache_id,
            &matching_fingerprint(),
            &KvCacheRuntimeFingerprint {
                runtime_id: "runtime-a".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-a".to_string(),
                prompt_format_fingerprint: Some("chatml".to_string()),
                runtime_build_fingerprint: Some("build-a".to_string()),
            },
        )
        .await
        .expect_err("legacy metadata without runtime fingerprint should fail");

    match error {
        KvCacheError::MissingRuntimeFingerprint { cache_id } => {
            assert!(!cache_id.is_empty());
        }
        other => panic!("expected MissingRuntimeFingerprint, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_prune_to_max_entries_evicts_oldest_entries() {
    let store = KvCacheStore::memory_only();
    let oldest = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: "oldest".to_string(),
            label: Some("oldest".to_string()),
            model_fingerprint: matching_fingerprint(),
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "runtime-a".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-a".to_string(),
                prompt_format_fingerprint: Some("chatml".to_string()),
                runtime_build_fingerprint: Some("build-a".to_string()),
            }),
            backend_hint: "llamacpp".to_string(),
            token_count: 10,
            markers: Vec::new(),
            created_at: 1,
            updated_at: 1,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![1],
    };
    let middle = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: "middle".to_string(),
            label: Some("middle".to_string()),
            model_fingerprint: matching_fingerprint(),
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "runtime-a".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-a".to_string(),
                prompt_format_fingerprint: Some("chatml".to_string()),
                runtime_build_fingerprint: Some("build-a".to_string()),
            }),
            backend_hint: "llamacpp".to_string(),
            token_count: 20,
            markers: Vec::new(),
            created_at: 2,
            updated_at: 2,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![2],
    };
    let newest = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: "newest".to_string(),
            label: Some("newest".to_string()),
            model_fingerprint: matching_fingerprint(),
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "runtime-a".to_string(),
                backend_key: "llamacpp".to_string(),
                tokenizer_fingerprint: "tok-a".to_string(),
                prompt_format_fingerprint: Some("chatml".to_string()),
                runtime_build_fingerprint: Some("build-a".to_string()),
            }),
            backend_hint: "llamacpp".to_string(),
            token_count: 30,
            markers: Vec::new(),
            created_at: 3,
            updated_at: 3,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![3],
    };

    store
        .memory
        .save(&oldest)
        .await
        .expect("save should succeed");
    store
        .memory
        .save(&middle)
        .await
        .expect("save should succeed");
    store
        .memory
        .save(&newest)
        .await
        .expect("save should succeed");

    let evicted = store
        .prune_to_max_entries(2)
        .await
        .expect("prune should succeed");

    assert_eq!(evicted, vec!["oldest".to_string()]);
    assert!(
        matches!(
            store.get_metadata("oldest").await,
            Err(KvCacheError::NotFound { .. })
        ),
        "oldest entry should have been evicted"
    );
    assert!(store.get_metadata("middle").await.is_ok());
    assert!(store.get_metadata("newest").await.is_ok());
}
