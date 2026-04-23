use super::*;
use std::sync::{Arc, Mutex};

use inference::kv_cache::{KvCacheRuntimeFingerprint, ModelFingerprint};
use inference::InferenceGateway;

#[path = "kv_cache_test_support.rs"]
mod test_support;
use test_support::{MockKvBackend, MockKvProcessSpawner};

#[tokio::test]
async fn capture_llamacpp_output_handle_saves_slot_into_store() {
    let restored = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockKvBackend {
            bytes: vec![1, 2, 3, 4],
            restored: restored.clone(),
        }),
        "mock-kv",
    ));
    gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

    let mut extensions = ExecutorExtensions::new();
    let store = Arc::new(KvCacheStore::memory_only());
    extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

    let handle_value =
        capture_llamacpp_output_handle("task-a", "exec-a", &gateway, &extensions, None)
            .await
            .expect("capture should succeed");
    let handle: KvCacheHandle =
        serde_json::from_value(handle_value).expect("capture should return a typed handle");
    let entry = store
        .load(
            &handle.cache_id,
            &ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
        )
        .await
        .expect("captured handle should resolve through the store");

    assert_eq!(entry.data, vec![1, 2, 3, 4]);
    assert_eq!(
        entry
            .metadata
            .runtime_fingerprint
            .as_ref()
            .map(|fp| fp.runtime_id.as_str()),
        Some("mock")
    );
}

#[tokio::test]
async fn execute_save_clones_handle_backed_entry() {
    let mut extensions = ExecutorExtensions::new();
    let store = Arc::new(KvCacheStore::memory_only());
    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some("source".to_string()),
            model_fingerprint: ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "mock".to_string(),
                backend_key: "mock".to_string(),
                tokenizer_fingerprint: "tok".to_string(),
                prompt_format_fingerprint: Some("prompt".to_string()),
                runtime_build_fingerprint: Some("build".to_string()),
            }),
            backend_hint: "mock".to_string(),
            token_count: 0,
            markers: Vec::new(),
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![4, 5, 6],
    };
    let source_cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .expect("fixture save should succeed");
    let source_handle = store
        .get_metadata(&source_cache_id)
        .await
        .expect("metadata should exist")
        .executable_handle()
        .expect("metadata should produce a handle");
    extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

    let mut inputs = HashMap::new();
    inputs.insert(
        "cache_data".to_string(),
        serde_json::to_value(source_handle).expect("handle should serialize"),
    );
    inputs.insert("label".to_string(), serde_json::json!("saved-copy"));

    let outputs = execute_save(&inputs, &extensions)
        .await
        .expect("save should clone the handle-backed entry");
    let saved_cache_id = outputs
        .get("cache_id")
        .and_then(|value| value.as_str())
        .expect("save should return a cache id");
    assert_ne!(saved_cache_id, source_cache_id);

    let saved_entry = store
        .load(
            saved_cache_id,
            &ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
        )
        .await
        .expect("saved entry should be readable");
    assert_eq!(saved_entry.data, vec![4, 5, 6]);
    assert_eq!(saved_entry.metadata.label.as_deref(), Some("saved-copy"));
}

#[tokio::test]
async fn execute_load_returns_typed_handle_when_runtime_matches() {
    let mut extensions = ExecutorExtensions::new();
    let store = Arc::new(KvCacheStore::memory_only());
    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some("saved".to_string()),
            model_fingerprint: ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "mock".to_string(),
                backend_key: "mock".to_string(),
                tokenizer_fingerprint: "tok".to_string(),
                prompt_format_fingerprint: Some("prompt".to_string()),
                runtime_build_fingerprint: Some("build".to_string()),
            }),
            backend_hint: "mock".to_string(),
            token_count: 0,
            markers: Vec::new(),
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![1, 2, 3],
    };
    let cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .expect("fixture save should succeed");
    extensions.set(crate::extension_keys::KV_CACHE_STORE, store);

    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockKvBackend {
            bytes: vec![9, 9, 9],
            restored: Arc::new(Mutex::new(Vec::new())),
        }),
        "mock-kv",
    ));
    gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

    let mut inputs = HashMap::new();
    inputs.insert("cache_id".to_string(), serde_json::json!(cache_id));
    inputs.insert(
        "model_fingerprint".to_string(),
        serde_json::json!({
            "modelId": "model",
            "configHash": "cfg",
        }),
    );

    let outputs = execute_load(&inputs, &extensions, Some(&gateway))
        .await
        .expect("load should succeed");
    assert_eq!(outputs.get("valid"), Some(&serde_json::json!(true)));
    let handle: KvCacheHandle = serde_json::from_value(
        outputs
            .get("cache_data")
            .cloned()
            .expect("load should return cache_data"),
    )
    .expect("load should return a typed handle");
    assert_eq!(handle.compatibility.runtime_fingerprint.runtime_id, "mock");
}

#[tokio::test]
async fn restore_llamacpp_input_handle_restores_saved_slot_bytes() {
    let restored = Arc::new(Mutex::new(Vec::new()));
    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockKvBackend {
            bytes: vec![9, 9, 9],
            restored: restored.clone(),
        }),
        "mock-kv",
    ));
    gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

    let mut extensions = ExecutorExtensions::new();
    let store = Arc::new(KvCacheStore::memory_only());
    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some("saved".to_string()),
            model_fingerprint: ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "mock".to_string(),
                backend_key: "mock".to_string(),
                tokenizer_fingerprint: "tok".to_string(),
                prompt_format_fingerprint: Some("prompt".to_string()),
                runtime_build_fingerprint: Some("build".to_string()),
            }),
            backend_hint: "mock".to_string(),
            token_count: 0,
            markers: Vec::new(),
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![7, 8, 9],
    };
    let cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .expect("fixture save should succeed");
    let metadata = store
        .get_metadata(&cache_id)
        .await
        .expect("metadata should be available");
    let handle = metadata
        .executable_handle()
        .expect("metadata should produce an executable handle");
    extensions.set(crate::extension_keys::KV_CACHE_STORE, store);

    let mut inputs = HashMap::new();
    inputs.insert(
        "kv_cache_in".to_string(),
        serde_json::to_value(handle).expect("handle should serialize"),
    );

    let restored_slot =
        restore_llamacpp_input_handle(&inputs, &gateway, &extensions, "task-a", "exec-a", None)
            .await
            .expect("restore should succeed");
    assert!(restored_slot);
    assert_eq!(
        restored.lock().expect("lock should succeed").as_slice(),
        [vec![7, 8, 9]]
    );
}

#[tokio::test]
async fn execute_truncate_delegates_to_backend_owned_codec() {
    let mut extensions = ExecutorExtensions::new();
    let store = Arc::new(KvCacheStore::memory_only());
    let entry = KvCacheEntry {
        metadata: KvCacheMetadata {
            cache_id: String::new(),
            label: Some("saved".to_string()),
            model_fingerprint: ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
            runtime_fingerprint: Some(KvCacheRuntimeFingerprint {
                runtime_id: "mock".to_string(),
                backend_key: "mock".to_string(),
                tokenizer_fingerprint: "tok".to_string(),
                prompt_format_fingerprint: Some("prompt".to_string()),
                runtime_build_fingerprint: Some("build".to_string()),
            }),
            backend_hint: "mock".to_string(),
            token_count: 4,
            markers: vec![CacheMarker {
                name: "prefix".to_string(),
                token_position: 2,
                description: None,
            }],
            created_at: 0,
            updated_at: 0,
            compressed: false,
            extra: serde_json::json!({}),
        },
        data: vec![1, 2, 3, 4],
    };
    let cache_id = store
        .save(entry, Some(StoragePolicy::MemoryOnly))
        .await
        .expect("fixture save should succeed");
    extensions.set(crate::extension_keys::KV_CACHE_STORE, store.clone());

    let gateway = Arc::new(InferenceGateway::with_backend(
        Box::new(MockKvBackend {
            bytes: vec![9, 9, 9],
            restored: Arc::new(Mutex::new(Vec::new())),
        }),
        "mock-kv",
    ));
    gateway.set_spawner(Arc::new(MockKvProcessSpawner)).await;

    let mut inputs = HashMap::new();
    inputs.insert("cache_id".to_string(), serde_json::json!(cache_id.clone()));
    inputs.insert("token_position".to_string(), serde_json::json!(2));

    let outputs = execute_truncate(
        &inputs,
        &extensions,
        Some(&gateway),
        "task-a",
        "exec-a",
        None,
    )
    .await
    .expect("truncate should succeed");
    let metadata: KvCacheMetadata = serde_json::from_value(
        outputs
            .get("metadata")
            .cloned()
            .expect("metadata output should exist"),
    )
    .expect("metadata output should deserialize");
    assert_eq!(metadata.token_count, 2);
    assert_eq!(metadata.markers.len(), 1);

    let truncated = store
        .load(
            &cache_id,
            &ModelFingerprint {
                model_id: "model".to_string(),
                config_hash: "cfg".to_string(),
            },
        )
        .await
        .expect("truncated entry should load");
    assert_eq!(truncated.data, vec![1, 2]);
}
